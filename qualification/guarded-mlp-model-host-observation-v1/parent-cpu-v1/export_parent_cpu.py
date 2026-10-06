"""Retain a bound successful host-observation parent CPU run; execute no project code."""
import hashlib
import io
import json
import os
from pathlib import Path
import resource
import re
import signal
import stat
import sys
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-host-observation-parent-cpu-v228-v1'
OUT = ROOT / 'evidence'
ARCHIVE = E / 'guarded-mlp-host-observation-parent-cpu-evidence-v228-v1.tar.gz'
TERMINAL = dict(bytes=3815416, sha256='ab72e2316315ec6b0766284e0b4c26e834b2502cd4a6fc29c304a875950e3c8f')
INPUT = dict(bytes=253729, sha256='c89ef5c5dea1d976562fdc2544fde28a1ee8ee47e1195e54f45e6f74671caf88')
SOURCE_ARCHIVE = dict(bytes=7704637, sha256='7eb6567cea9f195a6e66876dcc86e7cd3147c41718f860d711a2862b4261e66a')
CACHE = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
HELPERS = {
    'run_cpu.py': dict(bytes=38261, sha256='312a5b5a9d9367b5bb43480885bff0ad187a65757ae6a08695d48ea22b499de3'),
    'qualification_support.py': dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'),
    'supervisor.py': dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
}
STAGERS = {
    'stage_parent_cpu.py': (E / 'host-observation-stage-parent-v1.py',
        'f4ef4f7ead6d34eae9206211687ebc880eba0ba91d41503755feac78c6ee0a01'),
    'stage_parent_cache.py': (E / 'host-observation-stage-parent-cache-v1.py',
        '562f0c5a1f0b78e4510f4644fc04d488cdc4c61577a548a1f41a17c0756b25f6'),
}
PARENT = ROOT / 'ferric/adapters/m1-engineering-execution-v1'
TARGET = ROOT / 'target'
PACKAGE = 'ferric-m1-engineering-execution-v1'
BINARY = 'ferric-qwen3-finite-guarded-mlp-decode-engineering'
FEATURE = 'guarded-mlp-model-engineering'
OLD_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/projection-ordered-segment-cpu-v228-v2'
OLD_CARGO = '/home/harmenon/ferric-asrock-42/toolchain/cargo'
TOOLS_PIN = dict(bytes=2586, sha256='c38e83e58d01e60b7311efa1ff53cb94e69dbbcd227ee30bde6a04c7928590d9')
PRLIMIT = ['/usr/bin/prlimit', '--as=12884901888', '--cpu=1200', '--fsize=1073741824', '--core=0', '--']
FALSE_FIELDS = ('full_parent_library_suite_executed',
    'runtime_suite_rerun', 'worker_suite_rerun', 'gpu_execution', 'full_model_acceptance',
    'numerical_acceptance', 'performance_claim', 'production_authority', 'host_observation_native_execution', 'performance_policy_changed')
MAX_BODY, MAX_TOTAL, MAX_MEMBERS = 16 << 20, 64 << 20, 512


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def stamp(row):
    return (row.st_dev, row.st_ino, row.st_mode, row.st_nlink,
            row.st_size, row.st_mtime_ns, row.st_ctime_ns)


def file_pin(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 0 <= before.st_size <= 1 << 30, 'bounded ordinary file: ' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed before pin')
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed during pin')
    require(stamp(path.lstat()) == stamp(before), 'file changed after pin')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())


def ordinary(name):
    return (type(name) is str and Path(name).as_posix() == name and name not in ('', '.')
            and not Path(name).is_absolute() and '..' not in Path(name).parts)


def paths_below(root, limit=3000):
    require(root.resolve(strict=True) == root and root.is_dir(), 'ordinary source root')
    paths = []
    for directory, dirs, names in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'directory alias')
        paths.extend(Path(directory) / name for name in names)
        require(len(paths) <= limit, 'bounded source roster')
    return sorted(paths)


def inventory(text):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.M)
    require(len(names) == len(set(names)) and ': benchmark' not in text,
            'closed unique worker test inventory')
    return sorted(names)

def outcomes(text):
    summaries, named, active, progress = [], [], [], set()
    for line in text.splitlines():
        notice = re.fullmatch(r'test ([A-Za-z0-9_:]+) has been running for over 60 seconds', line)
        if notice:
            name = notice.group(1)
            require(name not in progress and name not in {row['name'] for row in named},
                    'duplicate or completed worker progress notice')
            progress.add(name)
            continue
        result = re.fullmatch(r'test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?', line)
        if result:
            name, status = result.groups()
            row = dict(name=name, outcome=status)
            require(name not in {value['name'] for value in named}, 'duplicate named worker outcome')
            require(status != 'ignored' or name not in progress, 'ignored worker progress notice')
            named.append(row)
            active.append(row)
            continue
        summary = re.fullmatch(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
            r'(\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s', line)
        if summary:
            status, passed, failed, ignored, measured, filtered = summary.groups()
            row = dict(status=status, passed=int(passed), failed=int(failed), ignored=int(ignored),
                       measured=int(measured), filtered_out=int(filtered))
            require(all(sum(item['outcome'] == outcome for item in active) == row[key]
                        for outcome, key in (('ok', 'passed'), ('FAILED', 'failed'), ('ignored', 'ignored'))),
                    'worker named outcome and target summary differ')
            summaries.append(row)
            active = []
        else:
            require(not line.startswith('test '), 'malformed worker libtest result')
    require(summaries and not active and progress <= {row['name'] for row in named},
            'incomplete worker libtest outcome')
    return dict(summaries=summaries, named=named,
                passed=sum(row['passed'] for row in summaries),
                failed=sum(row['failed'] for row in summaries),
                ignored=sum(row['ignored'] for row in summaries))

def statuses(value):
    return {row['name']: row['outcome'] for row in value['named']}


def environment(toolchain):
    library = toolchain.parent / 'lib'
    return dict(HOME='/home/harmenon', PATH=str(toolchain) + ':/usr/bin:/bin',
        CARGO_HOME=str(ROOT / 'cargo-home'), CARGO_TARGET_DIR=str(TARGET), TMPDIR=str(ROOT / 'tmp'),
        LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(library),
        RUSTC=str(toolchain / 'rustc'), RUSTDOC=str(toolchain / 'rustdoc'),
        CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
        CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never', MALLOC_ARENA_MAX='2', RUST_BACKTRACE='1',
        RUSTC_BOOTSTRAP='fe2o3_device,fe2o3_macros',
        CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0',
        CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
        CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
        CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
        ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')


def recipes(inputs, base, proposal, bodies, toolchain):
    cargo = str(toolchain / 'cargo')
    fmt = [str(toolchain / 'rustfmt'), '--edition', '2024', '--config', 'skip_children=true']
    paths = [str(ROOT / name) for name in inputs['parent_overlay']]
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(PARENT / 'Cargo.toml')]
    feature = ['--features', FEATURE]
    plan = [
        ('rustfmt', fmt + paths, 120),
        ('rustfmt-check', fmt + ['--check'] + paths, 120),
        ('rustc-version', [str(toolchain / 'rustc'), '--version', '--verbose'], 60),
        ('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path', str(PARENT / 'Cargo.toml'),
                      *feature, '--format-version', '1'], 120),
        ('parent-lib-list', [cargo, 'test', *common, *feature, '--lib', '--', '--list', '--format=terse'], 1200),
    ]
    scopes = {name: row for name, row in base['tests'].items() if name.startswith(('parent-', 'ferric-'))}
    require(len(scopes) == 44 and sum(row['passed'] for row in scopes.values()) == 364,
            '44 inherited selected scopes and364 passes')
    additions = proposal['direct_new_lib_tests'] + proposal['shared_new_lib_tests']
    require(len(proposal['direct_new_lib_tests']) == 9 and len(proposal['shared_new_lib_tests']) == 5
            and len(set(additions)) == 14 and len(proposal['new_binary_tests']) == 3
            and len(proposal['host_lib_tests']) == 6
            and proposal['binary'] == BINARY and proposal['feature'] == FEATURE, 'twenty-three reviewed parent additions including capture and host observations')
    tests, selected = {}, set()
    for label, previous in sorted(scopes.items()):
        raw = label + '-stdout'
        command = label + '-command.json'
        require(pin(bodies['inputs/' + raw]) == compact(base['raw'][raw])
                and pin(bodies['inputs/' + command]) == compact(base['raw'][command]), 'inherited raw scope pins')
        old = outcomes(bodies['inputs/' + raw].decode())
        require(statuses(old) == {name: 'ok' for name in previous['names']}
                and old['passed'] == previous['passed'] and old['failed'] == old['ignored'] == 0,
                'inherited raw named outcomes')
        argv = json.loads(bodies['inputs/' + command])['argv']
        tail = argv[argv.index('test') + 1:]
        tail[tail.index('--manifest-path') + 1] = str(PARENT / 'Cargo.toml')
        tail[tail.index('--features') + 1] = FEATURE
        names = set(previous['names'])
        if label.startswith('parent-'):
            selector = tail[tail.index('--lib') + 1]
            names |= {name for name in additions if name == selector or ('--exact' not in tail and selector in name)}
            require(not selected & names, 'overlapping selected parent library scopes')
            selected |= names
        plan.append((label, [cargo, 'test', *tail], 1200))
        tests[label] = names
    shared = proposal['shared_new_lib_tests']
    require(set(proposal['direct_new_lib_tests']) <= selected and not set(shared) & selected,
            'direct additions included; shared additions remain separate')
    plan.append(('parent-guarded-wire', [cargo, 'test', *common, *feature, '--lib',
                 'finite_guarded_mlp_decode_wire_v1::', '--', '--test-threads=2'], 1200))
    tests['parent-guarded-wire'] = set(shared)
    host_names = proposal['host_lib_tests']
    require(not set(host_names) & (selected | set(shared)), 'host scope does not overlap prior scopes')
    plan.append(('parent-guarded-host', [cargo, 'test', *common, *feature, '--lib',
                 'guarded_mlp_host_observation_v1::', '--', '--test-threads=2'], 1200))
    tests['parent-guarded-host'] = set(host_names)
    for label, args in [('guarded-bin-list', ['--list', '--format=terse']),
                        ('guarded-bin-tests', ['--test-threads=2'])]:
        plan.append((label, [cargo, 'test', *common, *feature, '--bin', BINARY, '--', *args], 1200))
    tests['guarded-bin-tests'] = set(proposal['new_binary_tests'])
    binaries = sorted(name for name in base['binaries'] if name != 'ferric-tp-peer-finite-engineering-worker-v1')
    require(len(binaries) == 4, 'four inherited parent products')
    binaries.append(BINARY)
    args = [item for name in binaries for item in ('--bin', name)]
    plan.append(('parent-builds', [cargo, 'build', '--profile', 'test', *common, *feature, *args,
                                 '--message-format=json'], 1200))
    plan.append(('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], 1200))
    require(len(plan) == 55 and len({row[0] for row in plan}) == 55
            and len(tests) == 47 and sum(map(len, tests.values())) == 387, 'closed55 recipes/47 scopes/387 names')
    return plan, tests, binaries


def relocate(value):
    if isinstance(value, str):
        return value.replace(OLD_ROOT + '/sources/ferric', str(ROOT / 'ferric')).replace(
            OLD_ROOT + '/target/parent', str(TARGET)).replace(OLD_CARGO, str(ROOT / 'cargo-home'))
    if isinstance(value, list):
        return [relocate(item) for item in value]
    if isinstance(value, dict):
        return {key: relocate(item) for key, item in value.items()}
    return value


def metadata_contract(current, historical):
    normalized = json.loads(json.dumps(current))
    expected = relocate(historical)
    root = next(row for row in normalized['packages'] if row['name'] == PACKAGE)
    prior = next(row for row in expected['packages'] if row['name'] == PACKAGE)
    target = dict(next(row for row in prior['targets'] if row['name'] ==
        'ferric-qwen3-finite-projection-residual-decode-ordered-host-engineering'))
    target.update(name=BINARY, src_path=str(PARENT / 'src/bin' / (BINARY + '.rs')))
    target['required-features'] = [FEATURE]
    require(root['targets'].count(target) == 1 and root['features'].pop(FEATURE) == ['tp-batch-engineering'],
            'exact additive feature/binary metadata')
    root['targets'].remove(target)
    node = next(row for row in normalized['resolve']['nodes'] if row['id'] == root['id'])
    require(node['features'].count(FEATURE) == 1, 'exact selected metadata feature')
    node['features'].remove(FEATURE)
    require(normalized == expected and current['workspace_root'] == str(PARENT)
            and current['target_directory'] == str(TARGET) and len(current['packages']) == 209,
            'complete historical metadata preserved apart from reviewed additions')


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and TERMINAL is not None and sys.argv[1] == TERMINAL['sha256'], 'python3 -B export_parent_cpu.py TERMINAL_SHA')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'actual qualification host/UID')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(ARCHIVE), 'fresh exact export')
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (128 << 20, 128 << 20))
    os.umask(0o077)
    bodies, observed = {}, {}

    def add(name, path, expected=None):
        require(ordinary(name) and name not in bodies, 'unique retained member')
        before = file_pin(path)
        require(before['bytes'] <= MAX_BODY and (expected is None or compact(before) == compact(expected)),
                'retained input pin: ' + name)
        body = path.read_bytes()
        require(pin(body) == compact(before) and file_pin(path) == before, 'retained body drift')
        bodies[name], observed[str(path)] = body, before
        require(len(bodies) < MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_TOTAL,
                'retention bounds')
        return body

    def verify(path, expected):
        row = file_pin(path)
        require(row == expected, 'full observed pin drift: ' + str(path))
        observed[str(path)] = row

    result = json.loads(add('evidence/complete.json', OUT / 'complete.json', TERMINAL))
    require(not os.path.lexists(OUT / 'failed.json')
            and result['schema'] == 'ferric-guarded-mlp-host-observation-parent-cpu-v1'
            and result['passed'] is True and result['postcheck_errors'] == [] and result['failure'] is None
            and result['source_unchanged'] is True and len(result['tests']) == 47
            and len(result['inventory']) == 868 and len(result['artifacts']) == 5
            and result['all_selected_parent_tests_executed'] is True and result['host_observation_source_added'] is True
            and result['inherited_capture_source_added'] is True
            and type(result['metadata']) is dict and len(result['metadata']['packages']) == 209
            and all(result[key] is False for key in FALSE_FIELDS), 'successful selected parent CPU attempt only')
    require(result['limits'] == dict(whole_seconds=3600, leaf_seconds=1200, cleanup_reserve_seconds=50,
        address_space_bytes=12 << 30, cache_bytes=6 << 30, stream_bytes=64 << 20,
        initial_free_bytes=40 << 30, live_free_bytes=38 << 30, affinity=[8, 9], cargo_jobs=2)
        and 0 < result['elapsed_seconds'] <= 3600, 'unchanged finite CPU resource contract')
    inputs = json.loads(add('input-manifest.json', ROOT / 'input-manifest.json', INPUT))
    require(set(inputs) == {'schema', 'files', 'lineage', 'parent_overlay', 'cache_manifest', 'git_revision'}
            and inputs['schema'] == 'ferric-guarded-mlp-host-observation-parent-cpu-input-v1'
            and len(inputs['files']) == 1225 and len(inputs['lineage']) == 103
            and inputs['cache_manifest'] == CACHE and compact(result['input_manifest']) == INPUT,
            'closed actual input')
    for name, expected in HELPERS.items():
        add(name, ROOT / name, expected)
    require(compact(result['controller']) == HELPERS['run_cpu.py']
            and compact(result['supervisor']) == HELPERS['supervisor.py'], 'actual controller identity')
    for name, (path, digest) in STAGERS.items():
        require(pin(add(name, path))['sha256'] == digest, 'reviewed stager identity')
    stage = json.loads(add('stage.json', ROOT / 'stage.json'))
    require(stage['schema'] == 'ferric-guarded-mlp-host-observation-parent-source-stage-v1' and stage['passed'] is True
            and stage['archive'] == SOURCE_ARCHIVE and stage['input_manifest'] == INPUT
            and stage['members'] == 1329 and stage['source_files'] == 1225
            and stage['root'] == str(ROOT) and all(stage[k] is False for k in
                ('project_code_executed', 'cargo_execution', 'gpu_execution', 'qualification_passed')),
            'actual source stage joins')
    require(set(result['readset']) == set(inputs['lineage']), 'closed103 readset')
    for name, expected in inputs['lineage'].items():
        require(ordinary(name) and '/' not in name, 'ordinary lineage basename')
        add('inputs/' + name, ROOT / 'inputs' / name, expected)
        verify(ROOT / 'inputs' / name, result['readset'][name])
    proposal = json.loads(bodies['inputs/parent-source-manifest.json'])
    capture = json.loads(bodies['inputs/capture-source-manifest.json'])
    require(pin(bodies['inputs/capture-source-manifest.json']) == dict(bytes=7739,
                sha256='2dc554b35f227180b1367f926d38e3332305f99666d9f9bd52b941240d1d899e')
            and capture['schema'] == 'ferric-guarded-mlp-model-stage-capture-source-v1'
            and capture['runtime_sources_changed'] is False and capture['default_wire_changed'] is False
            and capture['kernel_images_changed'] is False, 'exact capture source proposal')
    require(len(proposal['direct_new_lib_tests']) == len(proposal['shared_new_lib_tests']) == 5
            and len(proposal['new_binary_tests']) == 1
            and len(capture['new_tests']['parent_library']) == 4
            and len(capture['new_tests']['parent_guarded_binary']) == 1, 'closed old and capture additions')
    host = json.loads(bodies['inputs/host-source-manifest.json'])
    require(pin(bodies['inputs/host-source-manifest.json']) == dict(bytes=11067,
                sha256='8c36b978a705f6ce126ac0d3cd7f892f15e52aab752e020d84bd3e1f722021d7')
            and host['schema'] == 'ferric-guarded-mlp-model-host-observation-source-v1'
            and host['runtime_sources_changed'] is False and host['default_wire_changed'] is False
            and host['kernel_images_changed'] is False and host['performance_policy_changed'] is False
            and host['semantic_changes_relative_to_host_v1'] is False
            and len(host['new_tests']['parent_library']) == 6
            and len(host['new_tests']['parent_guarded_binary']) == 1,
            'exact thirteen-row mechanically rebased host source proposal')
    proposal = dict(proposal,
        direct_new_lib_tests=proposal['direct_new_lib_tests'] + capture['new_tests']['parent_library'],
        host_lib_tests=host['new_tests']['parent_library'],
        new_binary_tests=sorted(proposal['new_binary_tests'] + capture['new_tests']['parent_guarded_binary']
                                + host['new_tests']['parent_guarded_binary']))
    committed = json.loads(bodies['inputs/committed-source.json'])
    base = json.loads(bodies['inputs/base-complete.json'])
    require(base['passed'] is True and base['postcheck_errors'] == [] and base['error'] is None,
            'successful historical parent CPU baseline')
    parent_tools = json.loads(bodies['inputs/parent-tools.json'])
    require(pin(bodies['inputs/parent-tools.json']) == TOOLS_PIN
            and parent_tools == result['parent_toolchain_observation']
            and parent_tools['schema'] == 'ferric-guarded-mlp-parent-toolchain-v1'
            and parent_tools['build_execution'] is False and parent_tools['gpu_execution'] is False
            and parent_tools['tool_pins'] == result['tool_pins'], 'actual1.97.1 parent tool observation')
    toolchain = Path(parent_tools['toolchain'])
    require(str(toolchain) == '/home/harmenon/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin'
            and len(parent_tools['tool_pins']) == 8, 'fixed parent toolchain and eight tool pins')
    plan, test_names, binary_names = recipes(inputs, base, proposal, bodies, toolchain)
    phase_names = [row[0] for row in plan]
    lineage_names = {'base-complete.json', 'tool-complete.json', 'parent-source-manifest.json',
        'worker-source.json', 'worker-proposal.json', 'parent-lib-list-stdout',
        'parent-metadata-stdout', 'committed-source.json', 'parent-tools.json'}
    lineage_names |= {'capture-source-manifest.json', 'host-source-manifest.json', 'qualified-worker-complete.json',
                      'qualified-worker-sources.json', 'qualified-parent-complete.json',
                      'qualified-parent-sources.json'}
    lineage_names |= {name + suffix for name in base['tests'] if name.startswith(('parent-', 'ferric-'))
                     for suffix in ('-stdout', '-command.json')}
    require(set(inputs['lineage']) == lineage_names, 'closed103 semantic lineage names')
    old_inventory = inventory(bodies['inputs/parent-lib-list-stdout'].decode())
    require(len(old_inventory) == 848 and pin(bodies['inputs/parent-lib-list-stdout'])
            == compact(base['raw']['parent-lib-list-stdout']), 'authenticated historical848-name inventory')
    require(pin(bodies['inputs/parent-metadata-stdout']) == compact(base['raw']['parent-metadata-stdout']),
            'historical metadata raw pin')
    qualified = {}
    for kind, count, phases in (('worker', 990, 9), ('parent', 1222, 54)):
        previous = json.loads(bodies['inputs/qualified-' + kind + '-complete.json'])
        source_map = json.loads(bodies['inputs/qualified-' + kind + '-sources.json'])
        for suffix in ('complete', 'sources'):
            require(pin(bodies['inputs/qualified-' + kind + '-' + suffix + '.json'])
                    == compact(host['base'][kind + '_' + suffix]), 'actual host baseline pin')
        require(previous['schema'] == 'ferric-guarded-mlp-stage-capture-' + kind + '-cpu-v1'
                and previous['passed'] is True and previous['failure'] is None
                and previous['postcheck_errors'] == [] and previous['source_unchanged'] is True
                and previous['gpu_execution'] is False
                and previous['input_sources'] == previous['final_sources'] == source_map
                and compact(previous['raw']['sources-after.json'])
                    == pin(bodies['inputs/qualified-' + kind + '-sources.json'])
                and len(source_map) == count, 'actual qualified capture baseline source closure')
        require(len(previous['phases']) == phases and all(row['exit_code'] == 0
                and row['natural_exit'] is True and row['reaped'] is True
                and row['process_group_absent'] is True and row['forced_cleanup'] is False
                and row['timed_out'] is False and row['exception'] is None for row in previous['phases']),
                'actual qualified capture baseline lifecycle')
        qualified[kind] = (previous, source_map)
    prior_parent = qualified['parent'][0]
    prior_additions = proposal['direct_new_lib_tests'] + proposal['shared_new_lib_tests']
    require(prior_parent['inventory'] == sorted(old_inventory + prior_additions)
            and len(prior_parent['tests']) == 46
            and sum(row['passed'] for row in prior_parent['tests'].values()) == 380
            and all(row['failed'] == row['ignored'] == 0 for row in prior_parent['tests'].values())
            and not set(proposal['host_lib_tests']) & set(prior_parent['inventory']),
            'qualified capture selected outcomes and independent host additions')
    for label, names in test_names.items():
        if label == 'parent-guarded-host':
            continue
        expected = names - set(host['new_tests']['parent_guarded_binary']) if label == 'guarded-bin-tests' else names
        require(statuses(prior_parent['tests'][label]) == {name: 'ok' for name in expected},
                'every qualified capture selected outcome preserved')
    canonical = {n: compact(row) for n, row in qualified['parent'][1].items() if n.startswith('ferric/')}
    worker = {n: compact(row) for n, row in qualified['worker'][1].items()
              if n.startswith('ferric/adapters/tp-peer-finite-engineering-worker-v1/')}
    require(len(canonical) == 1219 and len(worker) == 181 and set(worker) <= set(canonical)
            and sum(canonical[n] != row for n, row in worker.items()) == 7,
            'exact seven inherited capture worker formatting transitions')
    canonical.update(worker)
    overlay = {'ferric/' + row['path']: {k: row[k] for k in ('before', 'after')} for row in host['files']}
    require(len(overlay) == 13 and sum(row['before'] is None for row in overlay.values()) == 3
            and committed['overlay'] == overlay and committed['files'] == canonical
            and committed['schema'] == 'ferric-guarded-mlp-parent-committed-source-v1'
            and set(committed['git_blobs']) == set(committed['files'])
            and committed['revision'] == inputs['git_revision'] == result['git_revision']
                == host['canonical_revision'] == '7f0d73ce594ee721576dc10e9d95dad973646bb0',
            'committed7f0d plus thirteen host source overlays')
    reconstructed = dict(committed['files'])
    for name, row in overlay.items():
        require(ordinary(name) and name.startswith('ferric/') and reconstructed.get(name) == row['before'],
                'reviewed source preimage')
        reconstructed[name] = row['after']
    require(reconstructed == {n: p for n, p in inputs['files'].items() if n.startswith('ferric/')},
            'complete preformat source reconstruction')
    before, formatted, final = (result[key] for key in ('preformat_sources', 'input_sources', 'final_sources'))
    require({n: compact(p) for n, p in before.items()} == inputs['files']
            and len(final) == 1225 and formatted == final and set(before) == set(final), 'source map joins')
    changed = sorted(n for n in before if before[n] != formatted[n])
    require(len(result['format_changed_paths']) == len(changed)
            and set(changed) == set(result['format_changed_paths']) and set(changed) <= set(inputs['parent_overlay'])
            and inputs['parent_overlay'] == sorted(n for n in overlay
                if n.startswith('ferric/adapters/m1-engineering-execution-v1/')),
            'only three selected parent sources may format')
    source_paths = paths_below(ROOT / 'ferric') + [ROOT / n for n in HELPERS]
    require({str(p.relative_to(ROOT)) for p in source_paths} == set(final), 'full current source roster')
    for name, expected in final.items():
        require(ordinary(name), 'ordinary source key')
        verify(ROOT / name, expected)
    for name in overlay:
        add(name, ROOT / name, final[name])
    cache = json.loads(add('cargo-cache-manifest.json', ROOT / 'cargo-cache-manifest.json', CACHE))
    cache_stage = json.loads(add('cargo-cache-stage-complete.json', ROOT / 'cargo-cache-stage-complete.json'))
    provenance = result['cache_provenance']
    verify(ROOT / 'cargo-cache-manifest.json', provenance['manifest'])
    verify(ROOT / 'cargo-cache-stage-complete.json', provenance['stage'])
    require(cache['schema'] == 'ferric-guarded-mlp-parent-cache-v1' and cache['cargo_execution'] is False
            and len(cache['files']) == 247 and len(cache['packages']) == 118 and len(cache['git_commits']) == 3
            and compact(final['ferric/adapters/m1-engineering-execution-v1/Cargo.lock']) == cache['lock']
            and {n: compact(p) for n, p in provenance['files'].items()} == cache['files']
            and cache_stage['schema'] == 'ferric-guarded-mlp-parent-cache-stage-v1'
            and cache_stage['passed'] is True and cache_stage['manifest'] == CACHE
            and cache_stage['cargo_home'] == str(ROOT / 'cargo-home')
            and cache_stage['lock'] == cache['lock'] and cache_stage['files'] == cache['files']
            and cache_stage['git_commits'] == cache['git_commits'] and cache_stage['packages'] == 118
            and cache_stage['cache_files'] == 247
            and cache_stage['baseline_cache'] == cache['baseline_cache'] == dict(
                archive=dict(bytes=39383327, sha256='18b1320fa9b4cc5c1e1a6db22b6da546f35edb00ba37ac5668f774e71eed69d2'),
                manifest=dict(bytes=96016, sha256='e8c533b038db432d45a31d461651a50998d0b94f42ecfc678c5823d00ffa8c81'),
                change='refs-heads-main')
            and all(cache_stage[k] is False for k in ('shared_cache_changed', 'lock_changed',
                'project_code_executed', 'crate_sources_extracted', 'git_sources_checked_out')),
            'exact private-cache provenance')
    for name, row in provenance['files'].items():
        require(ordinary(name), 'ordinary immutable cache key')
        verify(ROOT / 'cargo-home' / name, row)
    for row in result['tool_pins'].values():
        verify(Path(row['path']), row)
    require(result['configurations'] and all(p is None for p in result['configurations'].values())
            and all(not os.path.lexists(p) for p in result['configurations']), 'no Cargo configuration drift')
    raw_names = {name + suffix for name in phase_names for suffix in
                 ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
    raw_names |= {'sources-preformat.json', 'sources-before.json', 'sources-after.json', 'dependencies-before.json'}
    require(len(raw_names) == 279 and set(result['raw']) == raw_names
            and {p.name for p in OUT.iterdir()} == raw_names | {'complete.json'}, 'closed279 raw evidence files')
    for name, expected in result['raw'].items():
        add('evidence/' + name, OUT / name, expected)
    for name, rows in (('sources-preformat.json', before), ('sources-before.json', formatted),
                       ('sources-after.json', final)):
        require(json.loads(bodies['evidence/' + name]) == rows, 'raw source snapshot join')
    require([p['label'] for p in result['phases']] == phase_names, 'exact55 successful phases')
    env = environment(toolchain)
    for index, phase in enumerate(result['phases']):
        label = phase['label']
        require(phase['exit_code'] == 0
                and all(phase[k] is True for k in ('natural_exit', 'reaped', 'process_group_absent'))
                and all(phase[k] is False for k in ('timed_out', 'forced_cleanup'))
                and all(phase[k] is None for k in ('exception', 'storage_failure'))
                and phase['adopted_reaped'] == [] and phase['observed_signals'] == []
                and phase['argv'] == PRLIMIT + plan[index][1], 'clean natural exact phase recipe')
        command = json.loads(bodies['evidence/' + label + '.command.json'])
        started = json.loads(bodies['evidence/' + label + '.started.json'])
        require(json.loads(bodies['evidence/' + label + '.result.json']) == phase
                and command['argv'] == started['argv'] == phase['argv']
                and command['cwd'] == str(PARENT) and command['env'] == env
                and 0 < command['wall_timeout_seconds'] <= plan[index][2]
                and started['pid'] == started['pgid'] == phase['pid'] == phase['pgid']
                and type(phase['pid']) is int and phase['pid'] > 0, 'raw process recipe/lifecycle joins')
        for key in ('command', 'stdout', 'stderr'):
            require(phase[key] == result['raw'][label + ('.command.json' if key == 'command' else '.' + key)],
                    'phase stream join')
    require(bodies['evidence/rustc-version.stdout'].decode() == parent_tools['rustc_version'],
            'actual parent rustc version observation repeated')
    additions = proposal['direct_new_lib_tests'] + proposal['shared_new_lib_tests'] + proposal['host_lib_tests']
    current_inventory = inventory(bodies['evidence/parent-lib-list.stdout'].decode())
    require(current_inventory == result['inventory'] == sorted(old_inventory + additions)
            and len(current_inventory) == 868, 'exact old848 plus20 library additions')
    require(inventory(bodies['evidence/guarded-bin-list.stdout'].decode()) == proposal['new_binary_tests'],
            'exact new parent binary inventory')
    require(set(result['tests']) == set(test_names), 'exact selected47 test scopes')
    for label, names in test_names.items():
        actual = outcomes(bodies['evidence/' + label + '.stdout'].decode())
        require(actual == result['tests'][label] and statuses(actual) == {name: 'ok' for name in names}
                and len(actual['summaries']) == 1 and actual['summaries'][0]['status'] == 'ok'
                and actual['summaries'][0]['measured'] == 0 and actual['failed'] == actual['ignored'] == 0,
                'complete exact selected named outcomes: ' + label)
    require(sum(row['passed'] for row in result['tests'].values()) == 387,
            '387 selected passes; not a full library suite')
    require(json.loads(bodies['evidence/metadata.stdout']) == result['metadata'],
            'actual successful metadata body join')
    metadata_contract(result['metadata'], json.loads(bodies['inputs/parent-metadata-stdout']))
    dependencies = json.loads(bodies['evidence/dependencies-before.json'])
    require(len(dependencies) == 121, '118 registry and3 Git dependency roots')
    expected_roots = set()
    external_packages = 0
    cargo_home = ROOT / 'cargo-home'
    for package in result['metadata']['packages']:
        path = Path(package['manifest_path'])
        if package['source'] is None:
            require(path.is_relative_to(ROOT / 'ferric'), 'local metadata path')
            continue
        external_packages += 1
        require(path.is_relative_to(cargo_home), 'private dependency path')
        directory = path.parent
        if package['source'].startswith('git+'):
            parts = path.relative_to(cargo_home).parts
            require(parts[:2] == ('git', 'checkouts') and len(parts) >= 5, 'Git dependency path')
            directory = cargo_home.joinpath(*parts[:4])
        expected_roots.add(str(directory))
    require(external_packages == 181 and set(dependencies) == expected_roots, 'metadata dependency root census')
    for directory, rows in dependencies.items():
        root = Path(directory)
        paths = [path for path in paths_below(root, 20000) if '.git' not in path.relative_to(root).parts]
        require({str(path.relative_to(root)) for path in paths} == set(rows), 'dependency file roster')
        for name, row in rows.items():
            require(ordinary(name), 'ordinary dependency member')
            verify(root / name, row)
    require(set(result['artifacts']) == set(binary_names), 'exact five parent host products')
    build_records = [json.loads(line) for line in bodies['evidence/parent-builds.stdout'].decode().splitlines()
                     if line.startswith('{')]
    artifact_paths = set()
    for name, artifact in result['artifacts'].items():
        row = artifact['cargo_artifact']
        require(sum(item == row for item in build_records) == 1 and row['reason'] == 'compiler-artifact'
                and row['manifest_path'] == str(PARENT / 'Cargo.toml') and row['target']['name'] == name
                and row['target']['kind'] == ['bin'] and row['profile']['test'] is False
                and FEATURE in row['features'] and row['executable'] == artifact['pin']['path']
                and row['filenames'].count(row['executable']) == 1, 'actual Cargo parent product metadata')
        path = Path(row['executable'])
        require(path.is_relative_to(TARGET) and path not in artifact_paths, 'distinct fresh target artifact')
        artifact_paths.add(path)
        verify(path, artifact['pin'])
        with path.open('rb') as stream:
            require(stream.read(4) == b'\x7fELF', 'actual host artifact ELF')
    add('export_parent_cpu.py', Path(__file__).resolve())
    require(len(bodies) == 406, 'closed selected evidence capsule')
    manifest = dict(schema='ferric-guarded-mlp-host-observation-parent-cpu-retained-v1',
        files={name: pin(body) for name, body in sorted(bodies.items())}, terminal=TERMINAL,
        input=INPUT, source_archive=SOURCE_ARCHIVE, cache_manifest=CACHE,
        passed=True, failure=None, phases=55, raw_count=279, selected_tests_passed=387,
        selected_test_scopes=47, library_inventory=868, artifact_metadata=result['artifacts'],
        source_rows=1225, overlay_bodies=13, lineage_bodies=103, immutable_cache_rows=247,
        external_dependency_roots=121, all_selected_parent_tests_executed=True,
        full_parent_library_suite_executed=False,
        full_source_bodies_retained=False, cache_package_bodies_retained=False,
        host_executable_bodies_retained=False, parent_cpu_qualified=True,
        host_observation_source_added=True, inherited_capture_source_added=True,
        host_observation_native_execution=False, performance_policy_changed=False,
        gpu_execution=False, full_model_acceptance=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)
    bodies['manifest.json'] = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()
    require(len(bodies) <= MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_TOTAL, 'final capsule bounds')
    for path, expected in observed.items():
        require(file_pin(Path(path)) == expected, 'final source/input/cache/tool/retained-body posthash')
    created = False
    try:
        with ARCHIVE.open('xb') as stream:
            created = True
            with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
                for name, body in sorted(bodies.items()):
                    row = tarfile.TarInfo(name)
                    row.size, row.mode, row.mtime = len(body), 0o600, 0
                    tar.addfile(row, io.BytesIO(body))
        for path, expected in observed.items():
            require(file_pin(Path(path)) == expected, 'post-export input drift')
    except BaseException:
        if created:
            ARCHIVE.unlink(missing_ok=True)
        raise
    print(json.dumps(dict(archive=file_pin(ARCHIVE), terminal=TERMINAL, passed=True,
        members=len(bodies), pinned_members=len(manifest['files']), raw_count=279,
        expanded_bytes=sum(map(len, bodies.values())), selected_parent_cpu_qualified=True,
        full_parent_library_suite_executed=False, gpu_execution=False), sort_keys=True))


if __name__ == '__main__':
    def interrupted(signum, _frame):
        raise RuntimeError('parent failure export interrupted: ' + str(signum))
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(signum, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 180)
    try:
        main()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
