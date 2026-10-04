"""V2 CPU custody from retained bytes only; never open build-host source paths."""
import hashlib
import io
import json
from pathlib import Path
import re
import tarfile

import layer_validation as V

SCHEMA = 'ferric-p228-prefix-decode-host-policy-deployment-v2'
CPU_SCHEMA = 'ferric-p228-host-policy-cpu-result-v1'
REVIEW_SCHEMA = 'ferric-p228-host-policy-cpu-review-v1'
CPU_RUNNER_SHA = '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494'
OVERLAY_SHA = 'e69a6206e86c61de07c9643d8684c6b7a084ca9bf053ac7ced0f36ce407e8dc9'
SOURCE_INPUTS_SHA = 'd992cf95dd2acbb150b2fb70b28efd24ea1dba39e2542e7060a61ed61adc295b'
BOUNDS_SHA = 'e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1'
EXTRACTOR_SHA = '2f3ef5c80e4483ac1c1a1ebb2bbacf18af7e7b263d13d965fa991525d85e6d2a'
STABLE_SHA = '5b5efdaf64053b49c3acb78a82a1ffeaa948a4095b689d2f24e0dff8778ee816'
TOOL_PINS = {
    'worker': dict(rustc='08dfef109ad22d90556dbd2f964543cd93843dcd75a2e9792c173667392a1950',
        cargo='c9ad606cb1dbb4a65aa27c80be88ed61eb2b811b6450eeec6794f60ed78b94a3',
        rustdoc='475ad4924e815034ed7ec5f85edfc8a99b2df99eab8695a76debb1eb64a9bd44'),
    'parent': dict(cargo='828980723df339d62434390e9fb8ef8831036583343ae2316b7ab5646b5c1953',
        rustc='d3a664c970a9fd8361b64194861bebc1ae37b9054e5ee3400dc1c9e691797eea',
        rustdoc='3b0c7aef20e0dfdfe2dd19ddb0b40d3b744f76318a44031813f501e1912934df'),
}
FERRIC_COMMIT = '9eca257697069f10c87ee9f624016391f6a2e479'
PARENT = 'ferric-qwen3-finite-prefix-decode-host-policy-engineering'
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
RUNTIME_CRATES = frozenset(('fe2o3-amd-target', 'fe2o3-amdhsa-loader', 'fe2o3-aql',
    'fe2o3-drm-uapi', 'fe2o3-hsaco', 'fe2o3-kfd', 'fe2o3-kfd-uapi',
    'fe2o3-runtime-model', 'fe2o3-target-spec'))
PARENT_BINS = tuple('ferric-qwen3-finite-' + name + '-engineering' for name in (
    'two-forward', 'long', 'rearm-smoke', 'queued-mlp-comparison',
    'queued-projection-comparison', 'mlp-tiles-comparison', 'tiles-decode',
    'prefix-layer', 'prefix-decode', 'prefix-decode-host', 'prefix-decode-host-policy'))
FILTERS = (
    ('parent-client', 'tp_finite_client::', 138),
    ('parent-old-wire', 'finite_forward_wire_v1::', 5),
    ('parent-long-wire', 'finite_long_wire_v1::', 8),
    ('parent-smoke-wire', 'finite_rearm_smoke_wire_v1::', 8),
    ('parent-queued-mlp-wire', 'finite_queued_mlp_comparison_wire_v1::', 6),
    ('parent-projection-wire', 'finite_queued_projection_comparison_wire_v1::', 7),
    ('parent-tiles-wire', 'finite_mlp_tiles_comparison_wire_v1::', 9),
    ('parent-decode-wire', 'finite_tiles_decode_wire_v1::', 10),
    ('parent-prefix-layer-wire', 'finite_prefix_layer_wire_v1::', 8),
    ('parent-prefix-decode-wire', 'finite_prefix_decode_wire_v1::', 9),
    ('parent-host-data-v1', 'prefix_decode_host_observation_v1::', 7),
    ('parent-host-data-v2', 'prefix_decode_host_observation_v2::', 9),
)
MAX_FILE = 64 << 20
MAX_TOTAL = 256 << 20
MAX_RECORDS = 512


def pin(value):
    V.keys(value, 'path bytes sha256')
    V.uint(value['bytes'], MAX_TOTAL)
    V.require(type(value['path']) is str and Path(value['path']).is_absolute()
        and str(Path(value['path'])) == value['path'] and '..' not in Path(value['path']).parts
        and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']),
        'closed portable FilePin')
    return value


class Store:
    """Original identities map to verified relocated objects, never original files."""
    def __init__(self, D, pins, aliases, directory):
        V.require(type(aliases) is dict and 0 < len(aliases) <= MAX_RECORDS, 'bounded portable aliases')
        self.D, self.pins, self.aliases, self.used = D, pins, aliases, set()
        unique = {}
        for name, row in aliases.items():
            V.keys(row, 'original relocated')
            original, relocated = pin(row['original']), pin(row['relocated'])
            V.require(name == original['path'] and original['bytes'] == relocated['bytes']
                and original['sha256'] == relocated['sha256']
                and relocated['path'] == str(directory / 'objects' / original['sha256']),
                'exact content-addressed relocation')
            V.require(unique.setdefault(original['sha256'], original['bytes']) == original['bytes'],
                      'same object extent')
        V.require(sum(unique.values()) <= MAX_TOTAL, 'bounded unique portable bytes')

    def get(self, record, maximum=MAX_FILE):
        pin(record)
        V.require(record['bytes'] <= maximum and record['path'] in self.aliases,
                  'retained actual input missing or oversized')
        row = self.aliases[record['path']]
        V.require(row['original'] == record, 'retained original identity mismatch')
        actual, raw = self.pins.read(Path(row['relocated']['path']), record['sha256'], True, maximum)
        V.require(actual == row['relocated'], 'actual relocated extent')
        self.used.add(record['path'])
        return raw

    def doc(self, record, maximum=MAX_FILE):
        return V.parse(self.get(record, maximum))


def test_inventory(raw):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', raw.decode(), re.MULTILINE)
    V.require(names and len(names) == len(set(names)) and b': benchmark' not in raw,
              'actual nonduplicate test inventory')
    return set(names)


def test_results(raw, names, expected):
    text = raw.decode()
    rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored(?:, [^\n]*)?)$', text, re.MULTILINE)
    V.require(len(rows) == len({name for name, _ in rows}) and {name for name, _ in rows} == names,
              'actual named tests match compiled inventory')
    counts = [list(map(int, row)) for row in re.findall(
        r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)]
    V.require(counts == expected, 'exact observed test outcome counts')
    passed, ignored = sum(s == 'ok' for _, s in rows), sum(s.startswith('ignored') for _, s in rows)
    V.require(passed == sum(row[0] for row in counts) and ignored == sum(row[2] for row in counts),
              'raw outcomes and summaries agree')
    return dict(passed=passed, ignored=ignored, names=sorted(names), summaries=counts)


def git_source_map(store, archives):
    result = {}
    total, count = 0, 0
    for prefix in ('ferric', 'fe2o3'):
        row = archives[prefix]
        V.keys(row, 'path bytes sha256 commit tree')
        V.require(re.fullmatch('[0-9a-f]{40}', row['commit']) and re.fullmatch('[0-9a-f]{40}', row['tree']),
                  'source archive commit/tree identities')
        raw = store.get({key: row[key] for key in ('path', 'bytes', 'sha256')})
        seen = set()
        with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
            for member in archive:
                path = Path(member.name)
                V.require(path.parts and path.parts[0] == prefix and not path.is_absolute()
                    and '..' not in path.parts and member.name not in seen
                    and (member.isdir() or member.isfile()) and not member.linkname,
                    'closed Git archive member')
                seen.add(member.name)
                count += 1
                V.require(count <= 24000, 'combined two-archive node bound')
                if member.isdir():
                    continue
                V.require(0 <= member.size <= 16 << 20, 'source file cap')
                total += member.size
                V.require(total <= 256 << 20, 'combined source bytes cap')
                body = archive.extractfile(member).read((16 << 20) + 1)
                V.require(len(body) == member.size, 'source archive file extent')
                result[str(path)] = dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())
    return result


def recipes(directory, tools):
    source, target = directory / 'sources', directory / 'target'
    worker = source / 'ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'
    parent = source / 'ferric/adapters/m1-engineering-execution-v1/Cargo.toml'
    common = lambda manifest: ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(manifest)]
    wc, pc = str(Path(tools['worker']['root']) / 'bin/cargo'), str(Path(tools['parent']['root']) / 'bin/cargo')
    wa = [wc, 'test', *common(worker), '--lib', '--test', 'shared_wire']
    features = [*common(parent), '--features', 'tp-batch-engineering']
    rows = [
        ('worker-metadata', 'worker', [wc, 'metadata', '--offline', '--locked', '--manifest-path', str(worker), '--format-version', '1'], 120),
        ('worker-list', 'worker', [*wa, '--', '--list', '--format', 'terse'], 1200),
        ('worker-tests', 'worker', [*wa, '--', '--test-threads=2'], 1200),
        ('worker-build', 'worker', [wc, 'build', '--profile', 'test', *common(worker), '--bin', WORKER, '--message-format=json'], 1200),
        ('parent-metadata', 'parent', [pc, 'metadata', '--offline', '--locked', '--manifest-path', str(parent), '--features', 'tp-batch-engineering', '--format-version', '1'], 120),
        ('parent-lib-list', 'parent', [pc, 'test', *features, '--lib', '--', '--list', '--format', 'terse'], 1200),
    ]
    rows += [(name, 'parent', [pc, 'test', *features, '--lib', selector, '--', '--test-threads=2'], 1200)
             for name, selector, _ in FILTERS]
    for binary in PARENT_BINS:
        args = [pc, 'test', *features, '--bin', binary]
        rows += [(binary + '-list', 'parent', [*args, '--', '--list', '--format', 'terse'], 1200),
                 (binary + '-tests', 'parent', [*args, '--', '--test-threads=2'], 1200)]
    bins = [part for name in PARENT_BINS for part in ('--bin', name)]
    rows += [('parent-builds', 'parent', [pc, 'build', '--profile', 'test', *features, *bins, '--message-format=json'], 1200),
             ('parent-default-check', 'parent', [pc, 'check', *common(parent), '--lib', '--message-format=json'], 1200)]
    return rows, worker, parent, target


def cpu_environment(directory, tools, role):
    root, target = directory.parents[2], directory / 'target'
    compiler = Path(tools[role]['root'])
    env = dict(PATH=str(compiler / 'bin') + ':/usr/bin:/bin', HOME='/home/harmenon',
        LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC', CARGO_HOME=str(root / 'toolchain/cargo'),
        RUSTUP_HOME=str(root / 'toolchain/rustup'), RUSTC=str(compiler / 'bin/rustc'),
        RUSTDOC=str(compiler / 'bin/rustdoc'), CARGO_TARGET_DIR=str(target),
        CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_PROFILE_DEV_DEBUG='0',
        CARGO_PROFILE_TEST_DEBUG='0', CARGO_PROFILE_TEST_OPT_LEVEL='2',
        CARGO_PROFILE_DEV_OPT_LEVEL='2', TMPDIR=str(directory / 'tmp'),
        OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1', MKL_NUM_THREADS='1',
        HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    if role == 'worker':
        env.update(RUSTUP_TOOLCHAIN='nightly-2026-04-03',
            LD_LIBRARY_PATH=str(target / 'debug/deps') + ':' + str(compiler / 'lib'))
    else:
        V.require(role == 'parent', 'exact CPU compiler role')
        env.update(RUSTUP_TOOLCHAIN='1.97.1-x86_64-unknown-linux-gnu',
            RUSTC_BOOTSTRAP='fe2o3_device,fe2o3_macros', RUST_TEST_THREADS='2')
    return env


def cpu_evidence(store, complete):
    cpu = store.doc(complete)
    V.keys(cpu, 'schema passed runner helper extractor stable_environment source_inputs overlay archives extraction '
        'source_unchanged toolchains metadata binaries tests phases raw tests_passed tests_ignored '
        'empty_initial_target target_cap_bytes external_cargo_cache_reused gpu_execution numerical_acceptance '
        'performance_claim compiler_hsaco_reproduced production_authority')
    V.require(CPU_RUNNER_SHA is not None, 'freeze the reviewed V2 CPU runner before any runtime/GPU attempt')
    V.require(cpu['schema'] == CPU_SCHEMA and cpu['passed'] is True and cpu['source_unchanged'] is True
        and cpu['empty_initial_target'] is True and cpu['external_cargo_cache_reused'] is True
        and cpu['target_cap_bytes'] == 6 << 30 and cpu['tests_passed'] == 633 and cpu['tests_ignored'] == 4
        and all(cpu[k] is False for k in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
                                        'compiler_hsaco_reproduced', 'production_authority')),
        'actual successful distinct V2 CPU cohort, not historical605')
    for key, expected in (('runner', CPU_RUNNER_SHA), ('helper', BOUNDS_SHA), ('extractor', EXTRACTOR_SHA),
                          ('stable_environment', STABLE_SHA), ('overlay', OVERLAY_SHA), ('source_inputs', SOURCE_INPUTS_SHA)):
        V.require(cpu[key]['sha256'] == expected, 'frozen CPU source/controller identity: ' + key)
        store.get(cpu[key])
    source_inputs = store.doc(cpu['source_inputs'])
    V.require(source_inputs['schema'] == 'ferric-p228-clean-worker-sources-v1'
        and source_inputs['archives'] == cpu['archives'] and set(cpu['archives']) == {'ferric', 'fe2o3'}
        and cpu['archives']['ferric']['commit'] == FERRIC_COMMIT, 'exact two source archive inputs')
    base = git_source_map(store, cpu['archives'])
    directory = Path(complete['path']).parent
    V.require(re.fullmatch(r'host-policy-cpu-v228-v[1-9][0-9]*', directory.name), 'original CPU output namespace')
    raw = cpu['raw']
    V.require(type(raw) is dict and len(raw) < 256, 'bounded CPU raw file census')
    for name, record in raw.items():
        V.require(record['path'] == str(directory / name) and Path(name).name == name, 'actual CPU raw path')
        store.get(record)
    V.require(store.doc(raw['sources-base.json']) == base, 'source archives reproduce exact base census')
    overlay = store.doc(cpu['overlay'])
    V.require(overlay['schema'] == 'FerricPrefixHostPolicySourceProposalV1'
        and overlay['baseline_head'] == FERRIC_COMMIT and len(overlay['files']) == 13,
        'exact reviewed thirteen-source overlay')
    installed = dict(base)
    for row in overlay['files']:
        name = 'ferric/' + row['path']
        V.require((name not in base) if row['before'] is None else base.get(name) == row['before'],
                  'overlay exact before-body or absence')
        original = dict(row['after'], path=str(Path(cpu['overlay']['path']).parent / 'draft' / row['path']))
        store.get(original)
        installed[name] = row['after']
    V.require(store.doc(raw['sources-before.json']) == installed == store.doc(raw['sources-after.json']),
              'actual compiled source and lock maps unchanged after exact overlay')
    V.keys(cpu['toolchains'], 'worker parent')
    for role in ('worker', 'parent'):
        V.keys(cpu['toolchains'][role], 'root pins')
        V.require(cpu['toolchains'][role]['pins'] == TOOL_PINS[role], 'exact reviewed tool binaries')
    root = directory.parents[2]
    V.require(cpu['toolchains']['worker']['root'] == str(root / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu')
        and cpu['toolchains']['parent']['root'] == str(root / 'toolchain/rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu'),
        'reviewed compiler generations')
    expected, worker, parent, target = recipes(directory, cpu['toolchains'])
    V.require(set(cpu['phases']) == {r[0] for r in expected}, 'all exact CPU leaf phases')
    outputs = {}
    for name, role, argv, deadline in expected:
        command = store.doc(raw[name + '-command.json'])
        started = store.doc(raw[name + '-started.json'])
        result = store.doc(raw[name + '-result.json'])
        V.keys(command, 'argv env tools deadline_seconds cache_cap_bytes affinity nice gpu_execution expected_exit')
        V.keys(started, 'pid pgid')
        V.keys(result, 'exit_code reason elapsed_seconds group_absent cache_bytes stdout_sha256 stderr_sha256')
        V.require(command['argv'] == argv and command['deadline_seconds'] == deadline
            and command['cache_cap_bytes'] == 6 << 30 and command['affinity'] == [8, 9]
            and command['nice'] == 10 and command['gpu_execution'] is False
            and type(command['expected_exit']) is int and command['expected_exit'] == 0
            and command['tools'] == cpu['toolchains'][role]['pins']
            and command['env'] == cpu_environment(directory, cpu['toolchains'], role),
            'actual bounded exact CPU command/profile')
        V.require(type(started['pid']) is int and started['pid'] > 0 and started['pgid'] == started['pid']
            and type(result['exit_code']) is int and result['exit_code'] == 0 and result['reason'] is None
            and result['group_absent'] is True and V.uint(result['cache_bytes'], 6 << 30) <= 6 << 30
            and result['stdout_sha256'] == raw[name + '-stdout']['sha256']
            and result['stderr_sha256'] == raw[name + '-stderr']['sha256']
            and result == cpu['phases'][name], 'actual natural CPU leaf exit, group absence and raw streams')
        outputs[name] = store.get(raw[name + '-stdout'])
    expected_raw = {'sources-base.json', 'sources-before.json', 'sources-after.json'}
    expected_raw.update(name + suffix for name, *_ in expected
                        for suffix in ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json'))
    V.require(set(raw) == expected_raw, 'closed original CPU raw output set')
    tests = {}
    names = test_inventory(outputs['worker-list'])
    V.require(len(names) == 402 and sum(n.startswith('prefix_decode_host_observation_v2::tests::') for n in names) == 9
        and sum(n.startswith('native_prefix_decode_host_v2::tests::') for n in names) == 2, 'V2 worker inventory')
    tests['worker'] = test_results(outputs['worker-tests'], names, [[385, 0, 4], [13, 0, 0]])
    parent_names = test_inventory(outputs['parent-lib-list'])
    selected = set()
    for name, selector, count in FILTERS:
        names = {n for n in parent_names if selector in n}
        V.require(len(names) == count and not names.intersection(selected), 'closed disjoint parent test selection')
        selected.update(names)
        tests[name] = test_results(outputs[name], names, [[count, 0, 0]])
    V.require(len(selected) == 224 and sum(n.startswith('tp_finite_client::prefix_decode::host_policy_v2::tests::')
              for n in selected) == 7, 'exact parent tests and V2 subset')
    for binary in PARENT_BINS:
        names = test_inventory(outputs[binary + '-list'])
        V.require(len(names) == 1, 'actual binary CLI inventory')
        tests[binary] = test_results(outputs[binary + '-tests'], names, [[1, 0, 0]])
    V.require(tests == cpu['tests'] and sum(t['passed'] for t in tests.values()) == 633
        and sum(t['ignored'] for t in tests.values()) == 4, 'recomputed complete633 +4ignored, not synthetic counters')
    artifacts = {}
    for phase, manifest, names in (('worker-build', worker, [WORKER]), ('parent-builds', parent, PARENT_BINS)):
        rows = [V.parse(line) for line in outputs[phase].splitlines() if line.startswith(b'{')]
        V.require([r['success'] for r in rows if r.get('reason') == 'build-finished'] == [True], 'actual Cargo finish')
        executable = [r for r in rows if r.get('reason') == 'compiler-artifact' and r.get('executable')]
        V.require(len(executable) == len(names) and {r['target']['name'] for r in executable} == set(names),
                  'exact selected compiler artifacts')
        for row in executable:
            name = row['target']['name']
            V.require(row['manifest_path'] == str(manifest) and row['target']['kind'] == ['bin']
                and row['profile']['test'] is False and row['profile']['opt_level'] == '2'
                and row['executable'] == str(target / 'debug' / name), 'actual non-test opt2 binary identity')
            emitted = cpu['binaries'][name]
            V.require(emitted['artifact'] == row and emitted['binary']['path'] == row['executable'], 'actual binary pin joins Cargo')
            # All twelve actual artifacts, not only the two selected GPU executables.
            store.get(emitted['binary'])
            artifacts[name] = emitted
    V.require(artifacts == cpu['binaries'], 'closed compiled executable set')
    for role in ('worker', 'parent'):
        metadata = V.parse(outputs[role + '-metadata'])
        V.require(metadata['target_directory'] == str(target), 'actual fresh target metadata')
        V.require(len(metadata['packages']) == len({p['id'] for p in metadata['packages']})
            == cpu['metadata'][role]['package_count'], 'actual metadata package cardinality')
        local = {p['name']: p['manifest_path'] for p in metadata['packages'] if p['source'] is None}
        V.require(local == cpu['metadata'][role]['local'], 'actual metadata local graph')
        external = [dict(name=p['name'], version=p['version'], source=p['source'], manifest=p['manifest_path'])
                    for p in metadata['packages'] if p['source'] is not None]
        recorded = cpu['metadata'][role]['external']
        V.require(external == [{k: row[k] for k in ('name', 'version', 'source', 'manifest')} for row in recorded]
            and all(re.fullmatch('[0-9a-f]{64}', row['manifest_sha256'])
                    and Path(row['manifest']).is_relative_to(root / 'toolchain/cargo') for row in recorded),
            'actual external metadata graph; historical cache bodies are not silently opened')
        if role == 'worker':
            expected_local = {n: str(directory / 'sources/fe2o3/crates' / n / 'Cargo.toml') for n in RUNTIME_CRATES}
            expected_local[WORKER] = str(worker)
            V.require(local == expected_local and len(metadata['packages']) == 39
                and all(p['source'] is None or p['source'].startswith('registry+') for p in metadata['packages']),
                'worker exact nine runtime crates and locked registry graph')
        else:
            V.require('ferric-m1-engineering-execution-v1' in local
                and all(Path(p).is_relative_to(directory / 'sources/ferric') for p in local.values()),
                'parent separate locked Git graph; fresh local Ferric sources only')
    return cpu


def verify(D, pins, value, directory, historical):
    V.keys(value, 'schema historical_deployment cpu cpu_review aliases runtime')
    V.require(value['schema'] == SCHEMA, 'distinct V2 portable deployment')
    V.keys(value['cpu'], 'complete')
    _, old_runtime = historical(pins, value['historical_deployment'])
    store = Store(D, pins, value['aliases'], directory)
    cpu = cpu_evidence(store, value['cpu']['complete'])
    review = store.doc(value['cpu_review'], 65536)
    V.keys(review, 'schema reviewed authority complete runner overlay source_inputs notes '
        'owned_leaf_results_reviewed source_and_toolchain_reviewed production_authority performance_claim')
    V.require(review['schema'] == REVIEW_SCHEMA and review['reviewed'] is True and review['authority'] == 'none'
        and review['complete'] == value['cpu']['complete'] and review['runner'] == cpu['runner']
        and review['overlay'] == cpu['overlay'] and review['source_inputs'] == cpu['source_inputs']
        and type(review['notes']) is str and 0 < len(review['notes'].strip()) <= 16384
        and review['owned_leaf_results_reviewed'] is True and review['source_and_toolchain_reviewed'] is True
        and review['production_authority'] is False and review['performance_claim'] is False,
        'actual scoped root review, not an exporter-generated success declaration')
    V.keys(value['runtime'], 'parent worker image')
    for role, name in (('parent', PARENT), ('worker', WORKER)):
        deployed = pin(value['runtime'][role])
        original = cpu['binaries'][name]['binary']
        V.require(deployed['path'] == str(directory / 'bin' / name)
            and deployed['bytes'] == original['bytes'] and deployed['sha256'] == original['sha256'],
            'distinct selected actual V2 executable')
        actual, raw = pins.read(Path(deployed['path']), deployed['sha256'], True, 128 << 20)
        V.require(actual == deployed and raw[:6] == b'\x7fELF\x02\x01', 'actual relocated ELF64 executable')
    V.require(value['runtime']['image'] == old_runtime['image'], 'unchanged preserved SourceV5 image')
    V.require(store.used == set(store.aliases), 'no unreviewed extra archived CPU inputs')
    V.require({p.name for p in directory.iterdir()} <= {'objects', 'bin', 'complete.json'}
        and {p.name for p in (directory / 'objects').iterdir()} == {r['original']['sha256'] for r in store.aliases.values()}
        and {p.name for p in (directory / 'bin').iterdir()} == {PARENT, WORKER},
        'closed deployed CPU object and executable directories')
    pins.recheck()
    return dict(value['runtime'])
