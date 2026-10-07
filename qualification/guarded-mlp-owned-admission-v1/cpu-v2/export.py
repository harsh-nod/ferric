"""Bounded original-only exporter for the two closed pure-loader CPU attempts."""
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import resource
import signal
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
CONTROLLERS = {
    'v1': (23844, '2eca93aff71855eb1a48b1150dd235c9956dbe55b46e008b606f56b842e9ca2a'),
    'v2': (26434, '87d20407c459d0cc7dc3a1bc335f88fd83fc9460e20e3e181c6a35f403368f6d'),
}
PACKETS = {
    'v1': (430536, '043ae0da1ad819b0fc60a5e7d23376d91c65bb8563ad8ca45cef166fc7079747'),
    'v2': (431409, '5b72ddac27c8257aae9e986ce63c83e7a665293f6bdb290f7c887f03bbd164ba'),
}
BASE = (2448697, '3c4fe3a6c4942b20ec61fd7ea5c3409526e2167c43d24a62fb8660c34b0e713e')
SOURCE_MAP = (424416, '5de4929b2832eda55a53f99945fc4b64a9fbda28b4d4f8bef07f2e8d5e000d6a')
PROPOSAL = (14692, '600f4cc39c0bdfcc969ad2a3cdc52da4fdea91413ab001a7e0b8a833dc8a4d6c')
SUPERVISOR = (41485, '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
V1_FAILED = (1052453, 'dd9b6b534ae61a927e468be3a37fbe9281adc696b6b066dc6820915805ac252a')
V1_STAGE = (694712, 'f524ee70c4c0aa1c1023f1dfe8fbcb86ffc395a7ee87ac41525938c8602516c2')
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'generate-lock', 'metadata',
          'tests-build', 'tests-list', 'tests', 'release-build', 'benchmark')
RUST = {'bench/src/lib.rs', 'bench/src/main.rs', 'bench/src/tests.rs'}
MAPS = {'sources-before.json', 'sources-formatted.json', 'sources-tested.json',
        'sources-after.json', 'dependencies-before.json', 'dependencies-after.json'}
SUFFIXES = ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')
FILE_CAP, TOTAL_CAP, MEMBER_CAP = 16 << 20, 128 << 20, 1024
LIMITS = dict(whole_seconds=900, build_leaf_seconds=600, cleanup_seconds=50,
    build_address_space_bytes=12 << 30, scratch_bytes=2 << 30, build_stream_bytes=4 << 20,
    benchmark_wall_seconds=60, benchmark_cpu_seconds=45, benchmark_address_space_bytes=256 << 20,
    benchmark_each_stream_bytes=64 << 10, benchmark_internal_stdout_bytes=128 << 10,
    affinity=[8, 9], nice=10, initial_free_bytes=40 << 30, live_free_bytes=38 << 30)


def require(ok, why):
    if not ok:
        raise ValueError(why)


def compact(row):
    require(type(row) is dict and type(row.get('bytes')) is int and row['bytes'] >= 0
            and type(row.get('sha256')) is str and re.fullmatch('[0-9a-f]{64}', row['sha256']),
            'ordinary original pin')
    return {key: row[key] for key in ('bytes', 'sha256')}


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def relative(name):
    require(type(name) is str and name and str(PurePosixPath(name)) == name
            and not PurePosixPath(name).is_absolute() and '..' not in PurePosixPath(name).parts,
            'canonical relative member')
    return name


def read(path, cap=FILE_CAP, keep=True, single_link=True):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical read path')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink,
                       s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink >= 1
                and (not single_link or before.st_nlink == 1)
                and 0 <= before.st_size <= cap, 'bounded regular read and retained-body link policy')
        digest = hashlib.sha256(); chunks = []; total = 0
        while chunk := stream.read(1 << 20):
            total += len(chunk)
            require(total <= cap, 'read growth')
            digest.update(chunk)
            if keep:
                chunks.append(chunk)
        require(total == before.st_size and stamp(before) == stamp(os.fstat(stream.fileno()))
                == stamp(path.lstat()), 'read identity drift')
    result = dict(bytes=total, sha256=digest.hexdigest())
    return (b''.join(chunks) if keep else None), result


def tree(path, maximum):
    require(path.resolve(strict=True) == path and path.is_dir(), 'ordinary tree root')
    names = set()
    for directory, dirs, files in os.walk(path, followlinks=False):
        for name in dirs:
            node = Path(directory) / name
            require(not node.is_symlink() and node.resolve(strict=True) == node, 'directory alias')
        for name in files:
            node = Path(directory) / name
            require(not node.is_symlink() and stat.S_ISREG(node.lstat().st_mode), 'ordinary tree file')
            names.add(relative(node.relative_to(path).as_posix()))
            require(len(names) <= maximum, 'bounded tree inventory')
    return names


def export(attempt, terminal_sha, stage_sha):
    require(attempt in CONTROLLERS and CONTROLLERS[attempt] is not None, 'reviewed attempt controller binding required')
    require(E.resolve(strict=True) == E and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'exact original host/UID')
    root = E / ('owned-kernel-admission-bench-v228-' + attempt)
    stage_path = E / ('owned-kernel-admission-bench-stage-v228-' + attempt + '.json')
    output = E / ('owned-kernel-admission-bench-evidence-v228-' + attempt + '.tar.gz')
    partial = output.with_name(output.name + '.partial')
    require(root.resolve(strict=True) == root and not os.path.lexists(output)
            and not os.path.lexists(partial), 'fresh fixed export namespace')
    bodies = {}; paths = {}; checked = {}; total = 0
    def take(name, path, expected=None):
        nonlocal total
        relative(name)
        require(name not in bodies, 'one original member')
        raw, actual = read(path)
        require(expected is None or actual == compact(expected), 'original body pin: ' + name)
        total += len(raw)
        require(total <= TOTAL_CAP and len(bodies) < MEMBER_CAP - 1, 'bounded retained originals')
        bodies[name] = raw; paths[name] = path
        checked[str(path)] = (actual, FILE_CAP, True)
        return raw
    def live(path, expected, cap=1 << 30):
        _, actual = read(path, cap, False, False)
        require(actual == compact(expected), 'live external body drift: ' + str(path))
        checked[str(path)] = (actual, cap, False)
    outcomes = tree(root / 'evidence', 58) & {'complete.json', 'failed.json'}
    require(len(outcomes) == 1, 'one original terminal, never both')
    terminal = next(iter(outcomes))
    terminal_raw = take('evidence/' + terminal, root / 'evidence' / terminal)
    require(pin(terminal_raw)['sha256'] == terminal_sha, 'caller-observed terminal SHA')
    c = parse(terminal_raw)
    require(c['schema'] == 'ferric-owned-kernel-admission-cpu-v1' and type(c['passed']) is bool
            and c['passed'] == (terminal == 'complete.json')
            and (c['failure'] is None) == c['passed']
            and (c['passed'] or type(c['failure']) is str)
            and type(c['postcheck_errors']) is list
            and all(type(x) is str for x in c['postcheck_errors']), 'original outcome, not a manufactured success')
    require(c['limits'] == LIMITS and all(c[k] is False for k in
            ('gpu_execution', 'native_execution', 'model_execution', 'runtime_source_changed',
             'end_to_end_gain', 'numerical_acceptance', 'full2303_feasibility', 'production_authority')),
            'original bounded CPU-only scope')
    require(c['qualified_reduced_runtime_workspace_preserved'] is True
            and c['pure_inherited_workspace_fields_equal_to_original'] is True,
            'unchanged reduced-runtime and pure-workspace admission')
    require(type(c['sources_after']) is dict and 842 <= len(c['sources_after']) <= 843,
            'available bounded original source postcheck required')
    for name, row in sorted(c['sources_after'].items()):
        require(row['path'] == str(root / relative(name)), 'source path stays in exact root')
        take(name, root / name, row)
    def obj(name):
        return parse(bodies[name])
    for name, expected in [('run_cpu.py', CONTROLLERS[attempt]), ('supervisor.py', SUPERVISOR),
                           ('benchmark-source-manifest.json', PROPOSAL),
                           ('baseline-complete.json', BASE), ('baseline-sources.json', SOURCE_MAP)]:
        require((len(bodies[name]), pin(bodies[name])['sha256']) == expected, 'fixed source ancestry')
    base, source_map = obj('baseline-complete.json'), obj('baseline-sources.json')
    proposal, inputs = obj('benchmark-source-manifest.json'), obj('input-manifest.json')
    require(base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['final_sources'] == source_map, 'actual qualified original ancestor')
    require(set(inputs) == {'schema', 'files'} and inputs['schema'] == 'ferric-owned-kernel-admission-stage-v1'
            and pin(bodies['input-manifest.json']) == compact(c['input_manifest']), 'original stage input')
    cache = base['cache_provenance']['files']
    require(len(cache) == 73, 'original73 immutable cache inputs')
    expected = {'runtime/' + name.removeprefix('fe2o3/'): compact(row)
                for name, row in source_map.items() if name.startswith('fe2o3/')}
    require(len(expected) == 827, '825 canonical runtime bodies plus two qualified Cargo aliases')
    original_root = Path(proposal['sources'][0]['path']).parents[1]
    for row in proposal['sources']:
        name = Path(row['path']).relative_to(original_root).as_posix()
        name = 'proposal-README.md' if name == 'README.md' else relative(name)
        expected[name] = compact(row)
    for name, row in cache.items():
        expected['cargo-home/' + relative(name)] = compact(row)
    images = {'images/' + row['copy_name']: compact(row) for row in proposal['image_inputs']}
    require(set(images) == {'images/' + n + '.hsaco' for n in ('prefix', 'projection', 'mlp', 'guarded')},
            'four original image roles')
    expected.update(images)
    for name in ('run_cpu.py', 'supervisor.py', 'benchmark-source-manifest.json',
                 'baseline-complete.json', 'baseline-sources.json'):
        expected[name] = pin(bodies[name])
    require(len(expected) == 914 and inputs['files'] == expected, 'closed original staged914-file contract')
    before = c['sources_before']; after = c['sources_after']
    source_names = set(expected) - {n for n in expected if n.startswith('cargo-home/')}
    require(set(before) == source_names | {'input-manifest.json'}
            and set(after) == set(before) | ({'bench/Cargo.lock'} if 'bench/Cargo.lock' in after else set()),
            'source prefix plus optional actual generated lock only')
    for name, row in before.items():
        require(row['path'] == str(root / name)
                and compact(row) == (pin(bodies[name]) if name == 'input-manifest.json' else expected[name]),
                'original preformat source/input join')
    for name in set(before) - RUST:
        require(after[name] == before[name], 'immutable staged source drift cannot be recovered')
    for subdir in ('runtime', 'bench', 'images'):
        require(tree(root / subdir, 830) == {n[len(subdir) + 1:] for n in after if n.startswith(subdir + '/')},
                'closed original source tree')
    require(c['controller'] == before['run_cpu.py'] and c['supervisor'] == before['supervisor.py']
            and c['proposal'] == before['benchmark-source-manifest.json'], 'controller/source identity')
    raw_names = set(c['raw'])
    allowed = MAPS | {label + suffix for label in PHASES for suffix in SUFFIXES}
    require(raw_names <= allowed and tree(root / 'evidence', 58) == raw_names | {terminal}, 'exact original raw prefix')
    for name, row in sorted(c['raw'].items()):
        require(row['path'] == str(root / 'evidence' / relative(name)), 'raw path')
        take('evidence/' + name, root / 'evidence' / name, row)
    for name, field in [('sources-before.json', 'sources_before'), ('sources-after.json', 'sources_after'),
                        ('sources-tested.json', 'sources_tested'), ('dependencies-before.json', 'dependencies_before'),
                        ('dependencies-after.json', 'dependencies_after')]:
        if name in raw_names:
            require(obj('evidence/' + name) == c[field], 'original source/dependency map join')
    if c['sources_tested'] is not None:
        require(c['sources_tested'] == after, 'stable tested sources after original run')
    if 'sources-formatted.json' in raw_names:
        formatted = obj('evidence/sources-formatted.json')
        require(set(formatted) == set(before) and all(formatted[n] == before[n] for n in set(before) - RUST)
                and all(after[n] == formatted[n] for n in formatted), 'only three Rust formatter postimages')
    stage_raw = take('stage-ledger.json', stage_path)
    require(pin(stage_raw)['sha256'] == stage_sha, 'caller-observed external stage SHA')
    stage = parse(stage_raw)
    require(stage['schema'] == 'ferric-owned-kernel-admission-data-stage-receipt-v1'
            and stage['passed'] is True and stage['root'] == str(root)
            and stage['files'] == expected and stage['input'] == pin(bodies['input-manifest.json'])
            and all(stage[k] is False for k in ('project_execution', 'gpu_execution', 'shared_cache_changed')),
            'original external stage ledger')
    origins = stage['original_copy_ledger']
    require(set(origins) == {n for n in expected if n.startswith(('runtime/', 'cargo-home/', 'images/'))},
            'closed904-copy stage ledger')
    for name, row in origins.items():
        require(compact(row) == expected[name] and Path(row['path']).is_relative_to(E)
                and '..' not in Path(row['path']).parts, 'original copy identity/pin metadata')
    packet = E / ('owned-kernel-admission-bench-input-v228-' + attempt + '.tar.gz')
    require(stage['archive']['path'] == str(packet)
            and (stage['archive']['bytes'], stage['archive']['sha256']) == PACKETS[attempt], 'fixed original packet')
    live(packet, stage['archive'], 16 << 20)
    archive = E / 'guarded-mlp-reusable-arena-cache-v228-v1.tar.gz'
    require(stage['immutable_cache_archive']['path'] == str(archive)
            and compact(stage['immutable_cache_archive']) == compact(base['cache_provenance']['archive']),
            'frozen cache archive metadata')
    live(archive, stage['immutable_cache_archive'], 16 << 20)
    for name, row in cache.items():
        live(root / 'cargo-home' / relative(name), row, 16 << 20)
    require(c['tool_pins'] == base['tool_pins'] and len(c['tool_pins']) == 8, 'original eight tool bindings')
    for row in c['tool_pins'].values():
        live(Path(row['path']), row)
    phases = c['phases']
    require(type(phases) is list and len(phases) <= 10
            and [row['label'] for row in phases] == list(PHASES[:len(phases)]), 'original ordered phase prefix')
    require(raw_names - MAPS == {row['label'] + suffix for row in phases for suffix in SUFFIXES},
            'no unjoined future-phase raw bodies')
    env = c['environment']
    require(all(env[k] == '' for k in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'))
            and env['CARGO_HOME'] == str(root / 'cargo-home') and env['CARGO_TARGET_DIR'] == str(root / 'target')
            and env['CARGO_NET_OFFLINE'] == 'true' and env['CARGO_BUILD_JOBS'] == '2', 'GPU-hidden private offline build')
    tools = c['tool_pins']; cargo = tools['cargo']['path']; manifest = str(root / 'bench/Cargo.toml')
    common = ['--manifest-path', manifest, '--offline', '--locked']
    fmt = [tools['rustfmt']['path'], '--edition', '2024', *[str(root / n) for n in ('bench/src/lib.rs', 'bench/src/main.rs', 'bench/src/tests.rs')]]
    commands = {
        'rustfmt': (fmt, 60), 'rustfmt-check': ([fmt[0], '--check', *fmt[1:]], 60),
        'rustc-version': ([tools['rustc']['path'], '-Vv'], 30),
        'generate-lock': ([cargo, 'generate-lockfile', '--manifest-path', manifest, '--offline'], 60),
        'metadata': ([cargo, 'metadata', *common, '--format-version', '1'], 60),
        'tests-build': ([cargo, 'test', *common, '--release', '--lib', '--no-run', '--message-format=json'], 600),
        'release-build': ([cargo, 'build', *common, '--release', '--bin',
                          'ferric-owned-kernel-admission-bench-v1', '--message-format=json'], 600),
    }
    for field, label, testing, kind in [('test_artifact', 'tests-build', True, 'lib'),
                                      ('benchmark_artifact', 'release-build', False, 'bin')]:
        product = c[field]
        if product is None:
            continue
        row, identity = product['cargo_artifact'], product['pin']
        records = [parse(line) for line in bodies['evidence/' + label + '.stdout'].splitlines() if line.startswith(b'{')]
        candidates = [r for r in records if r.get('reason') == 'compiler-artifact'
            and r.get('manifest_path') == manifest and kind in r.get('target', {}).get('kind', [])
            and r.get('profile', {}).get('test') is testing and r.get('executable')]
        require(candidates == [row] and row['executable'] == identity['path']
                and Path(identity['path']).is_relative_to(root / 'target/release')
                and row['profile']['opt_level'] == '2' and row['profile']['debug_assertions'] is True,
                'exact own original Cargo-selected product')
        live(Path(identity['path']), identity)
        if testing:
            commands.update({'tests-list': ([identity['path'], '--list'], 30),
                             'tests': ([identity['path'], '--test-threads=1'], 60)})
        else:
            commands['benchmark'] = ([identity['path'], str(root / 'images')], 60)
    for index, phase in enumerate(phases):
        label = phase['label']
        require(phase['reaped'] is True and phase['process_group_absent'] is True, 'every started phase retired')
        require(all(type(phase[k]) is bool for k in ('natural_exit', 'reaped', 'process_group_absent', 'forced_cleanup', 'timed_out'))
                and type(phase['exit_code']) in (int, type(None)), 'original phase state types')
        if c['passed'] or index < len(phases) - 1:
            require(phase['exit_code'] == 0 and phase['natural_exit'] is True
                    and phase['forced_cleanup'] is False and phase['timed_out'] is False
                    and phase['exception'] is None and phase['storage_failure'] is None
                    and phase['observed_signals'] == [], 'clean original completed phase')
        require({label + suffix for suffix in SUFFIXES} <= raw_names
                and obj('evidence/' + label + '.result.json') == phase, 'complete original phase receipt set')
        command, started = obj('evidence/' + label + '.command.json'), obj('evidence/' + label + '.started.json')
        require(started == dict(pid=phase['pid'], pgid=phase['pgid'], argv=phase['argv'])
                and started['pid'] == started['pgid'] > 0 and command['argv'] == phase['argv']
                and command['cwd'] == str(root / 'bench') and command['env'] == env, 'owned original command/start')
        prefix = ['/usr/bin/prlimit', '--as=' + str((256 << 20) if label == 'benchmark' else (12 << 30)),
                  '--cpu=' + ('45' if label == 'benchmark' else '600'), '--fsize=1073741824', '--core=0', '--']
        argv, maximum = commands[label]
        require(phase['argv'] == prefix + argv and 0 < command['wall_timeout_seconds'] <= maximum, 'exact bounded leaf recipe')
        for key, suffix in [('command', 'command.json'), ('stdout', 'stdout'), ('stderr', 'stderr')]:
            require(phase[key] == c['raw'][label + '.' + suffix], 'original raw phase pin')
    require(c['dependencies_before'] == c['dependencies_after'], 'available admitted dependency snapshots stable')
    if c['dependencies_after']:
        metadata = obj('evidence/metadata.stdout')
        registry = {str(Path(p['manifest_path']).parent): p for p in metadata['packages'] if p['source'] is not None}
        require(set(c['dependencies_after']) <= set(registry), 'only original resolved registry dependencies')
        if c['passed']:
            require(set(c['dependencies_after']) == set(registry), 'successful original dependency inventory is complete')
    for dep, rows in c['dependencies_after'].items():
        path = Path(dep)
        require(path.is_relative_to(root / 'cargo-home/registry/src') and type(rows) is dict
                and len(rows) <= 2048 and tree(path, 2048) == set(rows), 'bounded original extracted dependency roster')
        require(sum(compact(row)['bytes'] for row in rows.values()) <= 48 << 20, 'dependency byte bound')
        for name, row in rows.items():
            require(row['path'] == str(path / relative(name)), 'dependency member path')
            live(path / name, row, 4 << 20)
    if c['tests'] is not None:
        require(any(p['label'] == 'tests' and p['exit_code'] == 0 and p['natural_exit'] is True for p in phases),
                'admitted tests require original clean test phase')
        expected_names = sorted(proposal['new_test_names'])
        stdout = bodies['evidence/tests.stdout'].decode()
        named = re.findall(r'^test ([A-Za-z0-9_:]+)(?: - should panic)? \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', stdout, re.M)
        summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                               r'(\d+) ignored; (\d+) measured; (\d+) filtered out;', stdout, re.M)
        inventory = bodies['evidence/tests-list.stdout'].decode()
        require(len(expected_names) == len(set(expected_names)) == 10
                and len(named) == 10 and sorted(name for name, _ in named) == expected_names
                and all(outcome == 'ok' for _, outcome in named)
                and summaries == [('ok', '10', '0', '0', '0', '0')]
                and sorted(re.findall(r'^([A-Za-z0-9_:]+): test$', inventory, re.M)) == expected_names
                and ': benchmark' not in inventory
                and c['tests'] == dict(names=expected_names, passed=10, failed=0, ignored=0, filtered_out=0),
                'original exact ten-test inventory, named outcomes and summary')
    if c['benchmark_report'] is not None:
        require(obj('evidence/benchmark.stdout') == c['benchmark_report'], 'original benchmark JSON join')
    if c['passed']:
        report = c['benchmark_report']
        require(len(phases) == 10 and raw_names == allowed and c['postcheck_errors'] == []
                and c['tests'] is not None and c['test_artifact'] is not None and c['benchmark_artifact'] is not None
                and c['sources_tested'] == after and c['dependencies_after'], 'complete original successful run')
        require(report['schema'] == 'ferric-owned-kernel-admission-opportunity-v1' and report['passed'] is True
                and report['layers_per_sample'] == 36 and report['preparations_per_sample'] == 360
                and len(report['warmup_pairs']) == 2 and len(report['sample_pairs']) == 12
                and report['pre_and_post_equivalence_checked'] is True
                and report['original_images_rehashed_after_samples'] is True
                and bodies['evidence/benchmark.stderr'] == b'', 'original complete pure opportunity report')
        require(all(report[k] is False for k in ('external_resource_limits_verified_by_program',
            'dynamic_dispatch_validation_measured', 'native_or_model_execution', 'end_to_end_gain',
            'full2303_feasibility', 'numerical_acceptance')), 'no benchmark authority transfer')
    if attempt == 'v1':
        require((len(terminal_raw), pin(terminal_raw)['sha256']) == V1_FAILED
                and (len(stage_raw), pin(stage_raw)['sha256']) == V1_STAGE and not c['passed']
                and len(phases) == 5 and len(raw_names) == 30 and len(after) == 843
                and c['tests'] is None and c['benchmark_report'] is None and c['dependencies_after'] == {},
                'fixed original v1 failure, no measurement or invented dependency inventory')
    take('export.py', Path(__file__).resolve())
    manifest_value = dict(schema='ferric-owned-kernel-admission-original-export-v1',
        attempt=attempt, root=str(root), passed=c['passed'], terminal_name=terminal,
        terminal=pin(terminal_raw), stage_original_path=str(stage_path), stage=pin(stage_raw),
        original_failure=c['failure'], original_postcheck_errors=c['postcheck_errors'],
        original_tests=c['tests'], benchmark_measurement_admitted=c['passed'],
        files={name: pin(raw) for name, raw in sorted(bodies.items())},
        live_rehashed=dict(sources=len(after), raw=len(raw_names), tools=len(c['tool_pins']),
            immutable_cache_inputs=73, original_archives=2, dependency_roots=len(c['dependencies_after']),
            dependency_files=sum(len(rows) for rows in c['dependencies_after'].values()),
            products=sum(c[key] is not None for key in ('test_artifact', 'benchmark_artifact'))),
        external_readset={name: value[0] for name, value in sorted(checked.items()) if name not in {str(p) for p in paths.values()}},
        original_sources_after_retained=True, original_phase_prefix_unchanged=True,
        uninventoried_extracted_cache_qualified=False, elf_bodies_retained=False,
        cache_archive_bodies_retained=False, project_execution=False, gpu_execution=False,
        native_execution=False, model_execution=False, end_to_end_gain=False,
        full2303_feasibility=False, numerical_acceptance=False, production_authority=False,
        local_external_rehash_claim=False)
    raw_manifest = (json.dumps(manifest_value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    require(len(raw_manifest) <= FILE_CAP and total + len(raw_manifest) <= TOTAL_CAP, 'bounded export manifest')
    bodies['manifest.json'] = raw_manifest
    with partial.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, raw in sorted(bodies.items()):
                member = tarfile.TarInfo(name); member.size = len(raw)
                member.mode, member.mtime = 0o644, 0
                tar.addfile(member, io.BytesIO(raw))
        stream.flush(); os.fsync(stream.fileno())
    for path, (expected_pin, maximum, single_link) in checked.items():
        require(read(Path(path), maximum, False, single_link)[1] == expected_pin, 'complete export readset posthash')
    for subdir in ('runtime', 'bench', 'images'):
        require(tree(root / subdir, 830) == {n[len(subdir) + 1:] for n in after if n.startswith(subdir + '/')},
                'source tree postinventory')
    for dep, rows in c['dependencies_after'].items():
        require(tree(Path(dep), 2048) == set(rows), 'admitted dependency postinventory')
    require(tree(root / 'evidence', 58) == raw_names | {terminal}, 'original evidence roster unchanged')
    os.link(partial, output, follow_symlinks=False)
    partial.unlink()
    directory = os.open(E, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)
    archive_pin = read(output, TOTAL_CAP, False)[1]
    print(json.dumps(dict(archive=dict(path=str(output), **archive_pin), members=len(bodies),
        original_sources=len(after), original_raw=len(raw_names), passed=c['passed'],
        original_failure=c['failure'], benchmark_measurement_admitted=c['passed']), sort_keys=True))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 4
            and sys.argv[1] in CONTROLLERS
            and all(re.fullmatch('[0-9a-f]{64}', value) for value in sys.argv[2:]),
            'python3 -B export.py v1|v2 OBSERVED_TERMINAL_SHA OBSERVED_STAGE_SHA')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected exporter nice level')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_AS, 512 << 20), (resource.RLIMIT_FSIZE, TOTAL_CAP),
                      (resource.RLIMIT_CPU, 180), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [value for value in (soft, hard) if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    def interrupted(number, _frame):
        raise RuntimeError('export interrupted by signal ' + str(number))
    for number in (signal.SIGALRM, signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, interrupted)
    signal.alarm(240)
    try:
        export(*sys.argv[1:])
    finally:
        signal.alarm(0)
