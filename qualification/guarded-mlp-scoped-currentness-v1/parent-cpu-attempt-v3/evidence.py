"""Bounded data-only export/retention; never imports or executes project code."""
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
import time
import tomllib

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-scoped-warm-parent-cpu-v228-v3')
OLD_ROOT = str(ROOT.parent / 'guarded-mlp-readiness40-shared-full-parent-cpu-v228-v3')
WORKER_ROOT = str(ROOT.parent / 'guarded-mlp-scoped-currentness-cpu-v228-v3')
WORKER_COMPLETE = dict(bytes=2364691, sha256='335cf93cc109390f3b7590f7dbe35ed852adca223a042894dcc18418a738ffbd')
WORKER_SOURCES = dict(bytes=420283, sha256='e5ca47a688908d4c78d1e3157211aee4c29bf12f626c01258f06a8ca8eefbda3')
BASE_WORKER_COMPLETE = dict(bytes=1746409, sha256='4a016b7e09b0cc6f9b4bd32c98a5713f24709bd6509564a589c29b0478f7e337')
BASE_WORKER_SOURCES = dict(bytes=428240, sha256='2566997ffb73fb40812688178f4494e48c1db4c8e7522717d54b9176994fd3e1')
PARENT = 'ferric/adapters/m1-engineering-execution-v1/'
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
OLD_FEATURE = 'guarded-mlp-readiness-engineering'
FULL_FEATURE = 'guarded-mlp-full2303-engineering'
FEATURE = OLD_FEATURE + ',' + FULL_FEATURE
ADDED_BINARY = 'ferric-qwen3-guarded-mlp-full2303-engineering'
INPUT = dict(bytes=265721, sha256='30d16317a92cdd1477a10c87921a19196d6a109894ad6729f994353feaecf372')
SOURCE_ARCHIVE = dict(bytes=1140009, sha256='175227c39899290a1a1725c5c1c4305aae652e23830b2017f8a5bf43cd537ed8')
CACHE = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
HELPERS = {
    'run_cpu.py': dict(bytes=45154, sha256='39fd25f0feecf5204363a0942615f5f6cd543df7a52075597210a35e53df00cf'),
    'qualification_support.py': dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'),
    'supervisor.py': dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
    'parent_transport.py': dict(bytes=27700, sha256='4fe4fdfde04446bb2642da59c593d0ffc783b06036fe725789596104d51a80f7'),
    'stage_parent_cache.py': dict(bytes=14346, sha256='03329d5b1e3df897c849b0383bb25995e4b2329879a5c6c36e783088142e81a9'),
}
BASE_COMPLETE = dict(bytes=3983845, sha256='f7134c5f22d75c8295b8ab99ad8625fac40aa615ac1682340c0b8e6ffde1f71e')
BASE_SOURCES = dict(bytes=507332, sha256='9e70e32da52ebba96a8354ecd5fe60bbb3add9a7d1b93ba0d218572349f129c4')
PROPOSAL_PINS = {
    "selector-repair.json": {
        "bytes": 16911,
        "sha256": "2659a6e7d2dd71a22fa5579be3a70be948b81c806cb511c79633e8739eb375a0"
    },
    "consumer-repair.json": {
        "bytes": 4565,
        "sha256": "c2be6fe83fba6c4bfef971bdf1e1535b88167ec8685f6faf6aa163abbdaa93e0"
    },
    "parent-proposal.json": {
        "bytes": 11484,
        "sha256": "687fa059a6eb2744ad55618fe7b59830ec36a4f096e6cf5ee456bf4ee5056db8"
    },
    "consumer-proposal.json": {
        "bytes": 1677,
        "sha256": "6b4dffbe4032e52f6dddb80f255553c420e95a3fb7af84f2b1da311daf1168bb"
    },
    "selector-proposal.json": {
        "bytes": 15169,
        "sha256": "cfec625935042c416a329cfb0e02c1fc8836f12820967e1d3c0257ed2d6a8235"
    },
    "runtime-proposal.json": {
        "bytes": 13373,
        "sha256": "437948cd334c9f95dcf77de4e3217c92f4d8e6d4cabbcf5010aff182f53ae5b2"
    },
    "primitive-proposal.json": {
        "bytes": 3820,
        "sha256": "9abcd59d4c3fd23d532bd1b9e86592a191e62960d8a901fbd31d8e61974bdec7"
    }
}
PARENT_REL = PARENT.removeprefix('ferric/').removesuffix('/')
WORKER_PREFIX = WORKER
BASE_PINS = dict(PROPOSAL_PINS, **{
    'parent-complete.json': BASE_COMPLETE,
    'parent-sources.json': BASE_SOURCES,
    'worker-complete.json': WORKER_COMPLETE,
    'worker-sources.json': WORKER_SOURCES,
})
MAX_FILE, MAX_TOTAL, MAX_MEMBERS = 16 << 20, 96 << 20, 512
DEADLINE = None


def require(ok, message):
    if not ok:
        raise ValueError(message)


def guard():
    require(DEADLINE is not None and time.monotonic() < DEADLINE, 'whole evidence deadline')


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def parse(body):
    def pairs(rows):
        out = {}
        for key, value in rows:
            require(key not in out, 'duplicate JSON field')
            out[key] = value
        return out
    return json.loads(body, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()


def relative(name):
    path = PurePosixPath(name)
    require(type(name) is str and name and str(path) == name and not path.is_absolute()
            and all(p not in ('.', '..') for p in path.parts) and '\\' not in name, 'ordinary relative path')
    return name


def read(path, maximum=MAX_FILE, keep=True):
    guard()
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input path')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    digest, chunks, size = hashlib.sha256(), [], 0
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= maximum, 'bounded regular body')
        while True:
            guard()
            chunk = stream.read(min(1 << 20, maximum + 1 - size))
            if not chunk:
                break
            size += len(chunk)
            require(size <= maximum, 'body grew past bound')
            digest.update(chunk)
            if keep:
                chunks.append(chunk)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and size == before.st_size, 'body changed during read')
    return b''.join(chunks) if keep else dict(bytes=size, sha256=digest.hexdigest())


def tree(root, omit_git=False):
    root = Path(root)
    require(root.resolve(strict=True) == root and root.is_dir(), 'ordinary tree root')
    names = set()
    for directory, dirs, files in os.walk(root, followlinks=False,
            onerror=lambda e: (_ for _ in ()).throw(e)):
        guard()
        require(not any((Path(directory) / n).is_symlink() for n in dirs), 'directory alias')
        if omit_git:
            dirs[:] = [n for n in dirs if n != '.git']
        for name in files:
            rel = str((Path(directory) / name).relative_to(root))
            if omit_git and '.git' in Path(rel).parts:
                continue
            names.add(relative(rel))
            require(len(names) <= 150000, 'bounded tree roster')
    return names


def outcomes(raw):
    summaries, named, active, progress = [], [], [], set()
    for line in raw.decode().splitlines():
        notice = re.fullmatch(r'test ([A-Za-z0-9_:]+) has been running for over 60 seconds', line)
        if notice:
            name = notice.group(1)
            require(name not in progress and name not in {r['name'] for r in named}, 'test progress')
            progress.add(name)
            continue
        match = re.fullmatch(r'test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?', line)
        if match:
            name, status = match.groups()
            require(name not in {r['name'] for r in named}, 'duplicate named result')
            row = dict(name=name, outcome=status)
            named.append(row); active.append(row)
            continue
        match = re.fullmatch(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                            r'(\d+) measured; (\d+) filtered out; finished in [0-9.]+s', line)
        if match:
            state, *numbers = match.groups()
            row = dict(zip(('passed', 'failed', 'ignored', 'measured', 'filtered_out'), map(int, numbers)), status=state)
            require(all(sum(r['outcome'] == status for r in active) == row[key]
                for key, status in [('passed', 'ok'), ('failed', 'FAILED'), ('ignored', 'ignored')]), 'named summary mismatch')
            summaries.append(row); active = []
        else:
            require(not line.startswith('test '), 'malformed test output')
    require(summaries and not active and progress <= {r['name'] for r in named}, 'incomplete named output')
    return dict(summaries=summaries, named=named, **{k: sum(r[k] for r in summaries) for k in ('passed', 'failed', 'ignored')})


def inventory(raw):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', raw.decode(), re.M)
    require(len(names) == len(set(names)) and ': benchmark' not in raw.decode(), 'test inventory')
    return sorted(names)


def relocate(value):
    if isinstance(value, str):
        return value.replace(OLD_ROOT, str(ROOT))
    if isinstance(value, list):
        return [relocate(v) for v in value]
    if isinstance(value, dict):
        return {k: relocate(v) for k, v in value.items()}
    return value


def recipes(base, inputs):
    tools = base['parent_toolchain_observation']; tc = tools['toolchain']
    cargo = tc + '/cargo'; manifest = str(ROOT / PARENT / 'Cargo.toml')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', manifest]
    feature = ['--features', FEATURE]
    fmt = [tc + '/rustfmt', '--edition', '2024', '--config', 'skip_children=true']
    paths = [str(ROOT / n) for n in inputs['parent_overlay']]
    rows = [('rustfmt', fmt + paths, 120), ('rustfmt-check', fmt + ['--check'] + paths, 120),
        ('rustc-version', [tc + '/rustc', '--version', '--verbose'], 60),
        ('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path', manifest,
                      *feature, '--format-version', '1'], 120),
        ('parent-lib-list', [cargo, 'test', *common, *feature, '--lib', '--', '--list', '--format=terse'], 1200)]
    for old in base['phases']:
        if old['label'] in base['tests'] or old['label'] in ('guarded-bin-list', 'readiness-bin-list', 'full2303-bin-list'):
            argv = relocate(old['argv'][6:])
            require(argv[argv.index('--features') + 1] == FEATURE, 'unchanged selected features')
            rows.append((old['label'], argv, 1200))
    rows.append(('parent-readiness-scoped-policy', [cargo, 'test', *common, *feature, '--lib',
        'finite_guarded_mlp_readiness_scoped_v1::tests', '--', '--test-threads=1'], 1200))
    bins = sorted(base['artifacts'])
    rows += [('parent-builds', [cargo, 'build', '--profile', 'test', *common, *feature,
                              *[v for n in bins for v in ('--bin', n)], '--message-format=json'], 1200),
             ('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], 1200)]
    require(len(rows) == 65 and len({r[0] for r in rows}) == 65, 'exact sixty-five recipes')
    return rows


def verify(bodies, terminal_name, terminal_sha):
    guard()
    require(INPUT is not None and SOURCE_ARCHIVE is not None and HELPERS['parent_transport.py'] is not None
            and HELPERS['run_cpu.py'] is not None and WORKER_COMPLETE is not None and WORKER_SOURCES is not None,
            'actual package/input/bound transport pins are pending')
    def body(name, expected=None):
        value = bodies[relative(name)]
        require(expected is None or pin(value) == compact(expected), 'body pin: ' + name)
        return value
    def value(name, expected=None):
        return parse(body(name, expected))
    require(terminal_name in ('complete.json', 'failed.json'), 'terminal basename')
    result = value('evidence/' + terminal_name)
    require(pin(body('evidence/' + terminal_name))['sha256'] == terminal_sha, 'observed terminal SHA')
    success = result['passed'] is True
    require(type(result['passed']) is bool and (terminal_name == 'complete.json') == success
            and (result['failure'] is None) == success and result['postcheck_errors'] == []
            and result['schema'] == 'ferric-guarded-mlp-readiness40-scoped-warm-parent-cpu-v1', 'clean actual outcome')
    for key in ('readiness40_scoped_warm_native_execution', 'currentness_temporal_equivalence_claim',
                'shared_full_host_timing_native_execution', 'readiness40_shared_full_native_execution', 'parent_host_timing_native_execution', 'ordinary_wire_schema_changed',
                'ordinary_observation_schema_changed', 'full2303_native_execution', 'full2303_launch_feasibility', 'gpu_execution', 'causal_layer0_native_execution', 'position5_native_execution', 'readiness_native_execution', 'full_long_workload', 'runtime_suite_rerun',
                'worker_suite_rerun', 'full_model_acceptance', 'numerical_acceptance', 'performance_claim',
                'production_authority', 'performance_policy_changed', 'default_policy_changed',
                'warm_paired_terminal_native_execution', 'global_currentness_policy_changed',
                'hidden_read_policy_changed', 'read_ns_scope_changed', 'full_parent_library_suite_executed'):
        require(result[key] is False, 'false authority/scope: ' + key)
    require(result['readiness40_scoped_warm_parent_source_added'] is True
            and result['scoped_warm_host_timing_parent_source_added'] is True
            and result['shared_full_host_timing_parent_source_added'] is True
            and result['readiness40_shared_full_parent_route_added'] is True
            and result['selected_readiness_currentness_policy_changed'] is True
            and result['qualified_worker_sources_preserved'] is True
            and result['worker_qualification'] == WORKER_COMPLETE and result['worker_source_manifest'] == WORKER_SOURCES
            and result['parent_host_timing_source_added'] is True
            and result['parent_host_timing_rows'] == 40 and result['parent_host_timing_disjoint_spans'] == 124
            and result['parent_causal_file_reads_retained'] is True
            and result['full2303_parent_route_added'] is True
            and result['full2303_source_abort_ms'] == 3600000
            and result['inherited_causal_layer0_parent_route_preserved'] is True
            and result['causal_layer0_parent_route_added'] is True
            and result['inherited_warm_paired_terminal_parent_route_preserved'] is True
            and result['causal_capture_positions'] == list(range(6)) and result['causal_layer'] == 0
            and result['warm_paired_terminal_parent_route_added'] is True
            and result['method_local_currentness_cadence_changed'] is True
            and result['selected_terminal_cadence_changed'] is True
            and result['position5_diagnostic_parent_route_added'] is True
            and result['inherited_readiness_parent_route_preserved'] is True,
            'supported inherited parent source scope')
    for n, p in HELPERS.items():
        body(n, p)
    require(compact(result['controller']) == HELPERS['run_cpu.py']
            and compact(result['supervisor']) == HELPERS['supervisor.py'], 'reviewed executed controller')
    inputs = value('input-manifest.json', INPUT)
    require(compact(result['input_manifest']) == INPUT and len(inputs['files']) == 1266
            and len(inputs['lineage']) == 121, 'actual input map')
    for n, p in inputs['lineage'].items():
        body('inputs/' + n, p)
        require(compact(result['readset'][n]) == p
                and result['readset'][n]['path'] == str(ROOT / 'inputs' / n), 'lineage readset join')
    require(set(result['readset']) == set(inputs['lineage']), 'closed lineage readset')
    for n, p in BASE_PINS.items():
        body('inputs/' + n, p)
    fixed = BASE_PINS
    base = value('inputs/parent-complete.json')
    old_map = value('inputs/parent-sources.json')
    proposal = value('inputs/parent-proposal.json')
    worker = value('inputs/worker-complete.json')
    worker_map = value('inputs/worker-sources.json')
    selector = value('inputs/selector-proposal.json')
    selector_repaired = value('inputs/selector-repair.json')
    consumer = value('inputs/consumer-proposal.json')
    repaired = value('inputs/consumer-repair.json')
    runtime = value('inputs/runtime-proposal.json')
    primitive = value('inputs/primitive-proposal.json')
    require(base['schema'] == 'ferric-guarded-mlp-readiness40-shared-full-host-timing-parent-cpu-v1'
            and len(old_map) == 1260 and len(base['phases']) == 64 and len(base['tests']) == 54
            and len(base['inventory']) == 948 and sum(v['passed'] for v in base['tests'].values()) == 478
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['shared_full_host_timing_parent_source_added'] is True
            and base['readiness40_shared_full_parent_route_added'] is True
            and base['readiness40_shared_full_native_execution'] is False
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual478 selected timed Shared parent baseline')
    for receipt, source_map, expected_pin, phase_count in (
            (base, old_map, BASE_SOURCES, 64), (worker, worker_map, WORKER_SOURCES, 26)):
        require(receipt['passed'] is True and receipt['failure'] is None
                and receipt['postcheck_errors'] == [] and receipt['source_unchanged'] is True
                and receipt['gpu_execution'] is False
                and receipt['input_sources'] == receipt['final_sources'] == source_map
                and compact(receipt['raw']['sources-after.json']) == expected_pin
                and len(receipt['phases']) == phase_count
                and all(p['exit_code'] == 0 and p['natural_exit'] is True and p['reaped'] is True
                    and p['process_group_absent'] is True and p['forced_cleanup'] is False
                    and p['timed_out'] is False and p['exception'] is None and p['storage_failure'] is None
                    for p in receipt['phases']), 'actual clean source/CPU lifecycle')
    require(worker['schema'] == 'ferric-guarded-mlp-scoped-currentness-cpu-v1'
            and worker['source_generation'] == 'scoped-currentness-coupled-v3'
            and len(worker_map) == 1033 and len(worker['artifacts']) == 11
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (726, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1143, 0, 8)
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['scoped_currentness_runtime_source_added'] is True
            and worker['scoped_currentness_worker_source_added'] is True
            and worker['explicit_scoped_readiness_selector_source_added'] is True
            and all(worker[k] is False for k in ('scoped_currentness_native_execution',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual coupled routed runtime and real worker executable qualification')
    for name in ('runtime-proposal.json', 'primitive-proposal.json', 'consumer-proposal.json',
                 'consumer-repair.json', 'selector-proposal.json', 'selector-repair.json'):
        require(compact(worker['readset'][name]) == fixed[name], 'actual coupled proposal readset join')
    require(proposal['schema'] == 'ferric-scoped-warm-readiness40-parent-source-v1'
            and compact(proposal['base']['parent_receipt']) == BASE_COMPLETE
            and compact(proposal['base']['parent_sources']) == BASE_SOURCES
            and compact(proposal['requires']['worker_selector']) == fixed['selector-proposal.json']
            and compact(proposal['requires']['consumer']) == fixed['consumer-proposal.json']
            and compact(proposal['requires']['runtime']) == fixed['runtime-proposal.json']
            and all(proposal[k] is False for k in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'default_routes_changed', 'default_group_policy_changed',
                'temporal_equivalent_to_full', 'performance_claim', 'numerical_acceptance')),
            'exact source-only explicit scoped parent proposal')
    require(selector['schema'] == 'ferric-scoped-warm-readiness40-worker-selector-source-v1'
            and len(selector['files']) == 9 and len(consumer['files']) == 4
            and compact(selector['requires']['consumer']) == fixed['consumer-proposal.json']
            and compact(selector['requires']['runtime']) == fixed['runtime-proposal.json']
            and compact(selector['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(selector['base']['worker_sources']) == BASE_WORKER_SOURCES
            and runtime['primitive_manifest'] == fixed['primitive-proposal.json']
            and len(runtime['files']) == 22 and len(primitive['files']) == 6,
            'coupled worker and routed runtime source lineage')
    require(repaired['schema'] == consumer['schema']
            and compact(repaired['predecessor_manifest']) == fixed['consumer-proposal.json']
            and repaired['source_generation'] == 'scoped-consumer-v2-module-path-repair'
            and repaired['declared_tests'] == consumer['declared_tests'] == 7
            and all(repaired[k] is False for k in ('compiled', 'tested', 'native_execution',
                'canonical_changed', 'profile_enabled'))
            and len(repaired['files']) == len(consumer['files']) == 4,
            'explicit consumer repair preserves original parent and selector dependencies')
    repair = repaired['repair']
    require(repair == dict(
        path='adapters/tp-peer-finite-engineering-worker-v1/src/state_roster/guarded_mlp_decode_v1/scoped_currentness_v1.rs',
        before_call='crate::native_forward::checked_layer_hidden_pair',
        after_call='crate::native_catalog::forward::checked_layer_hidden_pair',
        changed_occurrences=1, other_three_postimages_unchanged=True, tests_unchanged=True),
        'one registered-module call repair, not a route or test change')
    require(sorted(repaired['new_tests']['worker_library']) == sorted(
        name for names in selector['consumer_tests'].values() for name in names),
        'all seven original consumer test names')
    for old_row, new_row in zip(consumer['files'], repaired['files']):
        require(old_row['path'] == new_row['path'] and old_row['before'] == new_row['before']
                and (new_row['after'] != old_row['after'] if old_row['path'] == repair['path']
                     else new_row == old_row), 'one exact cumulative consumer postimage transition')
    require(selector_repaired['schema'] == selector['schema']
            and compact(selector_repaired['predecessor_manifest']) == fixed['selector-proposal.json']
            and selector_repaired['source_generation'] == 'scoped-selector-v2-owned-reader-signature-repair'
            and len(selector_repaired['files']) == len(selector['files']) == 9
            and all(selector_repaired[k] == selector[k] for k in (
                'base', 'requires', 'new_tests', 'consumer_tests', 'composed_new_tests',
                'conditional_composition', 'route', 'canonical_changed', 'compiled', 'tested',
                'native_execution', 'profile_enabled', 'old_routes_preserved',
                'default_group_policy_changed', 'full2303_profile_enabled',
                'performance_claim', 'numerical_acceptance')),
            'explicit selector test repair preserves original source and policy dependencies')
    selector_repair = selector_repaired['repair']
    require(selector_repair == dict(
        path='adapters/tp-peer-finite-engineering-worker-v1/src/native_guarded_mlp_readiness_cli_v1_tests.rs',
        before_reader='&mut &[u8]', after_reader='&mut io::Cursor<Vec<u8>>',
        changed_occurrences=2,
        changed_test='scoped_warm_publication_deadline_requires_both_pre_and_post_write_to_be_inside',
        other_eight_postimages_unchanged=True, production_bodies_unchanged=True,
        test_names_unchanged=True, original_consumer_requirement_unchanged=True),
        'two concrete owned-reader test types only, no production or assertion change')
    for old_row, new_row in zip(selector['files'], selector_repaired['files']):
        require(old_row['path'] == new_row['path'] and old_row['before'] == new_row['before']
                and ({k: v for k, v in new_row.items() if k != 'after'}
                     == {k: v for k, v in old_row.items() if k != 'after'})
                and (new_row['after'] != old_row['after'] if old_row['path'] == selector_repair['path']
                     else new_row == old_row), 'one exact cumulative selector test postimage transition')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1257 and sum(n.startswith(WORKER_PREFIX) for n in old) == 205
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual1257 parent source closure with205 worker bodies')
    worker_rows = {'ferric/' + r['path']: r for source in (repaired, selector_repaired) for r in source['files']}
    require(len(worker_rows) == 13 and all(n.startswith(WORKER_PREFIX) and n.endswith('.rs')
            for n in worker_rows), 'thirteen closed worker overlay paths')
    old_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    pre_worker = dict(old_worker)
    for n, row in worker_rows.items():
        require(pre_worker.get(n) == row['before'], 'actual worker preimage or absence ' + n)
        pre_worker[n] = row['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 209 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in worker_map) == 820,
            'only thirteen actually qualified worker formatting paths; all209 final bodies retained')
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(parent_rows) == len(proposal['files']) == 7
            and sum(r['before'] is None for r in parent_rows.values()) == 2
            and all(n.startswith('ferric/' + PARENT_REL + '/') and n.endswith('.rs')
                    and r['repository'] == 'ferric' for n, r in parent_rows.items()),
            'exact seven parent paths with two additions')
    expected = {n: r for n, r in old.items() if not n.startswith(WORKER_PREFIX)}
    expected.update(qualified_worker)
    for n, row in parent_rows.items():
        require(expected.get(n) == row['before'], 'actual parent preimage or absence ' + n)
        expected[n] = row['after']
    require(len(expected) == 1263 and not set(parent_rows) & set(worker_rows),
            '1263 Ferric bodies with disjoint parent and worker ownership')
    rows = dict(worker_rows, **parent_rows)

    lineage = set(BASE_PINS) | {'parent-lib-list.stdout', 'metadata.stdout'}
    lineage |= {n + suffix for n in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(inputs['lineage']) == lineage and len(lineage) == 121, 'exact121 direct lineage')
    for n in lineage - set(BASE_PINS):
        require(inputs['lineage'][n] == compact(base['raw'][n]), 'baseline raw ancestry')
    require(inventory(body('inputs/parent-lib-list.stdout')) == base['inventory'], 'prior raw inventory')
    pre = value('evidence/sources-preformat.json')
    before = value('evidence/sources-before.json') if result['input_sources'] is not None else None
    after = value('evidence/sources-after.json')
    require(result['preformat_sources'] == pre and result['input_sources'] == before
            and after == result['final_sources']
            and inputs['files'] == {n: compact(v) for n, v in pre.items()}
            and set(pre) == set(after) and len(after) == 1266, 'all source maps')
    if before is not None:
        require(result['source_unchanged'] is True and before == after, 'clean post-format sources')
    else:
        require(not success and result['source_unchanged'] is False, 'failed preformat-only attempt')
    require(all(row['path'] == str(ROOT / relative(n)) for n, row in after.items()), 'source path mapping')
    overlay = parent_rows
    expected.update({n: HELPERS[n] for n in ('run_cpu.py', 'qualification_support.py', 'supervisor.py')})
    require(inputs['files'] == expected, 'entire1266 composed source map')
    formatted = sorted(n for n in after if compact(after[n]) != compact(pre[n]))
    allowed = sorted(overlay)
    require(inputs['parent_overlay'] == allowed and len(allowed) == 7
            and set(formatted) <= set(allowed)
            and len(result['format_changed_paths']) == len(set(result['format_changed_paths']))
            and sorted(result['format_changed_paths']) == (formatted if before is not None else []),
            'only seven parent Rust sources may format; all209 qualified worker bodies preserved')
    parent_sources = set(overlay) | {PARENT + 'Cargo.lock'}
    worker_sources = set(worker_rows)
    require(len(parent_sources) == 8 and len(worker_sources) == 13,
            'seven parent postimages plus lock and thirteen qualified worker postimages')
    for n in worker_sources:
        body(n, after[n])
        require(compact(after[n]) == qualified_worker[n], 'retained actual worker unchanged')
    for n in parent_sources:
        body(n, after[n])
    stage = value('stage.json')
    require(stage['schema'] == 'ferric-guarded-mlp-readiness40-scoped-warm-parent-source-stage-v1'
            and stage['passed'] is True and stage['archive'] == SOURCE_ARCHIVE and stage['input'] == INPUT
            and stage['controller'] == HELPERS['parent_transport.py'] and stage['root'] == str(ROOT)
            and [stage[k] for k in ('source_files', 'ferric_files', 'overlay_files', 'parent_format_paths',
                                   'qualified_worker_files', 'helpers', 'lineage_files')]
                == [1266, 1263, 20, 7, 209, 3, 121]
            and stage['qualified_worker_sources_preserved'] is True
            and all(stage[k] is False for k in ('project_execution', 'canonical_changed', 'shared_cache_changed', 'lockfiles_changed')),
            'actual source staging')
    cache = value('cargo-cache-manifest.json', CACHE); cs = value('cargo-cache-stage-complete.json')
    provenance = result['cache_provenance']
    require(compact(provenance['manifest']) == CACHE
            and compact(provenance['stage']) == pin(body('cargo-cache-stage-complete.json'))
            and len(cache['files']) == 247 and len(cache['packages']) == 118 and len(cache['git_commits']) == 3
            and {n: compact(p) for n, p in provenance['files'].items()} == cache['files']
            and cs['passed'] is True and cs['manifest'] == CACHE and cs['files'] == cache['files']
            and cs['git_commits'] == cache['git_commits'] and cs['cargo_home'] == str(ROOT / 'cargo-home')
            and cs['lock'] == cache['lock'] == pin(body(PARENT + 'Cargo.lock')),
            'immutable private247-file cache')
    require(all(cs[k] is False for k in ('shared_cache_changed', 'lock_changed', 'project_code_executed',
                                        'crate_sources_extracted', 'git_sources_checked_out')), 'cache staging is data only')
    lock = tomllib.loads(body(PARENT + 'Cargo.lock').decode())['package']
    require(sorted((p['name'], p['version'], p['checksum']) for p in lock if p.get('source', '').startswith('registry+'))
            == sorted((p['name'], p['version'], p['checksum']) for p in cache['packages']), 'exact locked registry roster')
    require({p['source'] for p in lock if p.get('source', '').startswith('git+')}
            == {p['source'] for p in cache['git_commits']}, 'exact locked Git revisions')
    planned = recipes(base, inputs); phases = result['phases']
    require(1 <= len(phases) <= 65 and [p['label'] for p in phases] == [p[0] for p in planned[:len(phases)]], 'serial recipe prefix')
    raw_names = {name + suffix for name, _, _ in planned[:len(phases)]
                 for suffix in ('.command.json', '.started.json', '.stdout', '.stderr', '.result.json')}
    raw_names |= {'sources-preformat.json', 'sources-after.json'}
    if before is not None:
        raw_names.add('sources-before.json')
    if 'dependencies-before.json' in result['raw']:
        raw_names.add('dependencies-before.json')
    require(set(result['raw']) == raw_names, 'exact raw phase/map roster')
    for n, p in result['raw'].items():
        require(p['path'] == str(ROOT / 'evidence' / n), 'raw identity')
        body('evidence/' + n, p)
    first_old = value('inputs/' + next(iter(base['tests'])) + '.command.json')
    environment = relocate(first_old['env'])
    prefix = ['/usr/bin/prlimit', '--as=12884901888', '--cpu=1200', '--fsize=1073741824', '--core=0', '--']
    for index, (phase, (label, argv, maximum)) in enumerate(zip(phases, planned)):
        require(phase['argv'] == prefix + argv and phase['natural_exit'] is True and phase['reaped'] is True
                and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
                and phase['timed_out'] is False and phase['exception'] is None and phase['storage_failure'] is None
                and phase['observed_signals'] == [] and (phase['exit_code'] == 0 or (not success and index == len(phases)-1)),
                'natural bounded phase')
        require(value('evidence/' + label + '.result.json') == phase, 'exact original phase result')
        command = value('evidence/' + label + '.command.json'); started = value('evidence/' + label + '.started.json')
        require(command['argv'] == phase['argv'] == started['argv'] and command['env'] == environment
                and command['cwd'] == str(ROOT / PARENT) and 0 < command['wall_timeout_seconds'] <= maximum
                and started['pid'] == started['pgid'] == phase['pid'] == phase['pgid'], 'command/start/resource joins')
        for k in ('command', 'stdout', 'stderr'):
            require(result['raw'][label + '.' + ('command.json' if k == 'command' else k)] == phase[k], 'phase raw pin')
    test_groups = (
        (PARENT + 'src/tp_finite_client/long/readiness_host_timing_tests.rs',
         'tp_finite_client::long::readiness::host_timing::tests::', 14, 2),
        (PARENT + 'src/tp_finite_client/long/readiness_scoped_tests.rs',
         'tp_finite_client::long::readiness::scoped::tests::', 4, 4),
        (WORKER + 'src/finite_guarded_mlp_readiness_scoped_v1_tests.rs',
         'finite_guarded_mlp_readiness_scoped_v1::tests::', 7, 7))
    additions = []
    for path, prefix, total, added in test_groups:
        declared = [prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', body(path).decode())]
        new = [n for n in declared if n not in base['inventory']]
        require(len(declared) == len(set(declared)) == total and len(new) == added,
                'exact source-declared new and inherited parent methods')
        additions.extend(new)
    additions.sort()
    require(len(additions) == len(set(additions)) == 13
            and sorted(n.rsplit('::', 1)[-1] for n in additions)
                == sorted(proposal['new_tests']['parent_library']), 'thirteen additive library names')
    bin_names = sorted('tests::' + n for n in proposal['new_tests']['parent_readiness_binary'])
    declared_bin = re.findall(r'#\[test\]\s*fn\s+(\w+)\(',
        body(PARENT + 'src/bin/ferric-qwen3-guarded-mlp-readiness-engineering.rs').decode())
    require(len(bin_names) == len(set(bin_names)) == 1, 'one additive readiness binary method')
    old_bin = {r['name'] for r in base['tests']['readiness-bin-tests']['named']}
    require(not old_bin & set(bin_names) and sorted('tests::' + n for n in declared_bin) == sorted(old_bin | set(bin_names)),
            'exact one additive readiness CLI regression')
    expected_tests = {}
    selected_library = set()
    for label, old_test in base['tests'].items():
        require(outcomes(body('inputs/' + label + '.stdout')) == old_test, 'prior actual selected outcomes')
        old_phase = next(p for p in base['phases'] if p['label'] == label)
        require(value('inputs/' + label + '.command.json')['argv'] == old_phase['argv'], 'prior command ancestry')
        names = {r['name'] for r in old_test['named']}
        if label == 'readiness-bin-tests':
            names |= set(bin_names)
        argv = next(argv for name, argv, _ in planned if name == label)
        if '--lib' in argv:
            selector = argv[argv.index('--lib') + 1]
            names |= {n for n in additions if n == selector or ('--exact' not in argv and selector in n)}
            require(not selected_library & names, 'inherited library scopes remain disjoint')
            selected_library |= names
        expected_tests[label] = names
    policy_names = {n for n in additions if n.startswith('finite_guarded_mlp_readiness_scoped_v1::tests::')}
    require(len(policy_names) == 7 and not policy_names & selected_library,
            'seven new disjoint scoped policy names')
    expected_tests['parent-readiness-scoped-policy'] = policy_names
    selected_library |= policy_names
    require(set(additions) <= selected_library
            and len(expected_tests) == 55 and sum(map(len, expected_tests.values())) == 492,
            'all478 original outcomes plus fourteen declared additions')
    require(set(result['tests']) <= set(expected_tests), 'only declared selections')
    for label, observed in result['tests'].items():
        require(outcomes(body('evidence/' + label + '.stdout')) == observed
                and {r['name']: r['outcome'] for r in observed['named']} == {n: 'ok' for n in expected_tests[label]}
                and observed['failed'] == observed['ignored'] == 0, 'exact actual named selection')
    if result['inventory']:
        require(inventory(body('evidence/parent-lib-list.stdout')) == result['inventory']
                == sorted(base['inventory'] + additions) and len(result['inventory']) == 961, '961 library inventory')
    for label, names in [('guarded-bin-list', expected_tests['guarded-bin-tests']),
                         ('readiness-bin-list', expected_tests['readiness-bin-tests']),
                         ('full2303-bin-list', expected_tests['full2303-bin-tests'])]:
        completed = next((p for p in phases if p['label'] == label), None)
        if completed is not None and completed['exit_code'] == 0:
            require(inventory(body('evidence/' + label + '.stdout')) == sorted(names), 'named binary inventory')
    if result['metadata'] is not None:
        current = value('evidence/metadata.stdout')
        require(current == result['metadata'], 'retained current metadata')
        require(current == relocate(value('inputs/metadata.stdout'))
                and current['workspace_root'] == str(ROOT / PARENT)
                and current['target_directory'] == str(ROOT / 'target')
                and len(current['packages']) == 209, 'unchanged dependency/feature/target graph')
    products = result['artifacts']
    records = [parse(line) for line in body('evidence/parent-builds.stdout').splitlines() if line.startswith(b'{')] \
        if 'parent-builds.stdout' in result['raw'] else []
    for name, product in products.items():
        row = product['cargo_artifact']; p = product['pin']
        candidates = [r for r in records if r.get('reason') == 'compiler-artifact'
            and r.get('manifest_path') == str(ROOT / PARENT / 'Cargo.toml')
            and r.get('target', {}).get('name') == name and r['target']['kind'] == ['bin']
            and r.get('profile', {}).get('test') is False]
        require(candidates == [row] and p['path'] == row['executable']
                and Path(p['path']).is_relative_to(ROOT / 'target')
                and {OLD_FEATURE, FULL_FEATURE} <= set(row['features'])
                and row['filenames'].count(p['path']) == 1, 'actual Cargo-selected parent product')
    require(result['tool_pins'] == base['tool_pins'] == result['parent_toolchain_observation']['tool_pins']
            and result['parent_toolchain_observation'] == base['parent_toolchain_observation']
            and all(v is None for v in result['configurations'].values()), 'actual tool/configuration closure')
    if success:
        require(len(phases) == 65 and len(result['tests']) == 55 and set(result['tests']) == set(expected_tests)
                and sum(v['passed'] for v in result['tests'].values()) == 492
                and set(products) == set(base['artifacts'])
                and result['all_selected_parent_tests_executed'] is True and len(result['raw']) == 329,
                'complete65/55/492/seven-product success')
    expected_bodies = {'evidence/' + n for n in raw_names | {terminal_name}}
    expected_bodies |= {'inputs/' + n for n in lineage} | set(HELPERS) | parent_sources | worker_sources
    expected_bodies |= {'input-manifest.json', 'stage.json', 'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json', 'evidence.py'}
    require(set(bodies) == expected_bodies and (not success or len(bodies) == 482), 'closed selected evidence bodies')
    return result


def live_rehash(result):
    sources = result['final_sources']
    require(tree(ROOT / 'ferric') == {n[len('ferric/'):] for n in sources if n.startswith('ferric/')}, 'live full source roster')
    for n, row in sources.items():
        require(read(ROOT / n, keep=False) == compact(row), 'live source drift')
    for row in result['tool_pins'].values():
        require(read(Path(row['path']), 1 << 30, False) == compact(row), 'live tool drift')
    for row in result['cache_provenance']['files'].values():
        require(Path(row['path']).is_relative_to(ROOT / 'cargo-home')
                and read(Path(row['path']), 64 << 20, False) == compact(row), 'live immutable cache drift')
    dep_path = ROOT / 'evidence/dependencies-before.json'
    roots = parse(read(dep_path)) if dep_path.exists() else {}
    require(not result['passed'] or len(roots) == 121, 'complete dependency roots')
    for name, rows in roots.items():
        root = Path(name)
        require(root.is_relative_to(ROOT / 'cargo-home') and tree(root, True) == set(rows), 'live dependency roster')
        for rel, row in rows.items():
            require(row['path'] == str(root / relative(rel))
                    and read(root / rel, 64 << 20, False) == compact(row), 'live dependency drift')
    for value in result['artifacts'].values():
        row = value['pin']; path = Path(row['path'])
        require(read(path, 1 << 30, False) == compact(row), 'live product drift')
        with path.open('rb') as stream:
            require(stream.read(4) == b'\x7fELF', 'actual selected ELF')
    require(all(not os.path.lexists(p) for p in result['configurations']), 'live Cargo configuration drift')
    return dict(sources=len(sources), dependency_roots=len(roots), cache_files=len(result['cache_provenance']['files']),
                tools=len(result['tool_pins']), products=len(result['artifacts']))


def unpack(raw):
    bodies, seen, total = {}, set(), 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for member in archive:
            guard(); relative(member.name)
            require(member.isfile() and member.name not in seen and len(seen) < MAX_MEMBERS
                    and 0 <= member.size <= MAX_FILE and not member.pax_headers, 'closed regular bounded archive')
            seen.add(member.name); total += member.size
            require(total <= MAX_TOTAL, 'expanded evidence bound')
            data = archive.extractfile(member).read(MAX_FILE + 1)
            require(len(data) == member.size, 'complete archive member')
            bodies[member.name] = data
    return bodies


def main():
    global DEADLINE
    DEADLINE = time.monotonic() + 300
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError('whole evidence deadline')))
    signal.setitimer(signal.ITIMER_REAL, 300)
    resource.setrlimit(resource.RLIMIT_AS, (768 << 20, 768 << 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_CPU, (240, 240))
    require(__debug__ and sys.dont_write_bytecode, 'nonoptimized python3 -I -S -B')
    require(len(sys.argv) == 7 and sys.argv[1] in ('export', 'retain'),
        'export TERMINAL_SHA STAGER CACHE_STAGER ARCHIVE TERMINAL_NAME | retain ARCHIVE ARCHIVE_SHA TERMINAL_SHA DEST TERMINAL_NAME')
    mode = sys.argv[1]; terminal_name = sys.argv[6]
    own = read(Path(__file__).resolve())
    if mode == 'export':
        terminal_sha, stage_path, cache_path, destination = sys.argv[2:6]
        require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'export host/UID')
        require(re.fullmatch('[0-9a-f]{64}', terminal_sha), 'observed terminal SHA')
        original = read(ROOT / 'evidence' / terminal_name); result = parse(original)
        inputs = parse(read(ROOT / 'input-manifest.json')); proposal = parse(read(ROOT / 'inputs/parent-proposal.json'))
        paths = {n: ROOT / n for n in ('input-manifest.json', 'stage.json', 'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json',
                                      'run_cpu.py', 'qualification_support.py', 'supervisor.py')}
        paths.update({'parent_transport.py': Path(stage_path), 'stage_parent_cache.py': Path(cache_path)})
        paths.update({'evidence/' + n: ROOT / 'evidence' / n for n in set(result['raw']) | {terminal_name}})
        paths.update({'inputs/' + n: ROOT / 'inputs' / n for n in inputs['lineage']})
        names = {'ferric/' + r['path'] for r in proposal['files'] if ('ferric/' + r['path']).startswith(PARENT)}
        names.add(PARENT + 'Cargo.lock')
        for source in ('consumer-repair.json', 'selector-repair.json'):
            worker_proposal = parse(read(ROOT / 'inputs' / source))
            names.update('ferric/' + r['path'] for r in worker_proposal['files'])
        paths.update({n: ROOT / n for n in names})
        bodies = {n: read(p) for n, p in paths.items()}; bodies['evidence.py'] = own
        require(bodies['evidence/' + terminal_name] == original and tree(ROOT / 'evidence') == set(result['raw']) | {terminal_name},
                'complete original evidence directory')
        result = verify(bodies, terminal_name, terminal_sha)
        live = live_rehash(result)
        require(all(read(p) == bodies[n] for n, p in paths.items()) and read(Path(__file__).resolve()) == own, 'selected final posthashes')
        manifest = dict(schema='ferric-readiness40-scoped-warm-parent-cpu-selected-evidence-v1', terminal_name=terminal_name,
            terminal=pin(original), files={n: pin(b) for n, b in sorted(bodies.items())},
            passed=result['passed'], live_rehashed=live, source_body_scope='seven parent overlays plus unchanged Cargo.lock and thirteen coupled-V3 qualified worker overlays; all209 worker bodies preserved',
            all_raw_and_lineage_retained=True, binary_bodies_retained=False, dependency_bodies_retained=False,
            original_receipts_unchanged=True, project_execution=False, gpu_execution=False,
            full_model_acceptance=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
        bodies['manifest.json'] = encoded(manifest)
        require(len(bodies) <= MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_TOTAL, 'selected archive bounds')
        destination = Path(destination)
        require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
                and not os.path.lexists(destination), 'fresh archive destination')
        guard()
        with destination.open('xb') as stream:
            with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive:
                for n, data in sorted(bodies.items()):
                    guard(); info = tarfile.TarInfo(n); info.size = len(data); info.mode = 0o600; info.mtime = 0
                    archive.addfile(info, io.BytesIO(data))
            stream.flush(); os.fsync(stream.fileno())
        raw = read(destination, MAX_TOTAL)
        require(unpack(raw) == bodies, 'roundtrip exact evidence archive')
        report = dict(archive=pin(raw), members=len(bodies), expanded_bytes=sum(map(len, bodies.values())),
                      terminal=pin(original), passed=result['passed'], live_rehashed=live)
    else:
        archive_path, archive_sha, terminal_sha, destination = sys.argv[2:6]
        require(all(re.fullmatch('[0-9a-f]{64}', s) for s in (archive_sha, terminal_sha)), 'observed archive/terminal SHA')
        raw = read(Path(archive_path), MAX_TOTAL)
        require(pin(raw)['sha256'] == archive_sha, 'actual archive SHA')
        bodies = unpack(raw); manifest_raw = bodies.pop('manifest.json'); manifest = parse(manifest_raw)
        require(manifest['schema'] == 'ferric-readiness40-scoped-warm-parent-cpu-selected-evidence-v1'
                and manifest['terminal_name'] == terminal_name
                and manifest['files'] == {n: pin(b) for n, b in bodies.items()}
                and bodies['evidence.py'] == own, 'closed original selected pin manifest and exact reviewed helper')
        result = verify(bodies, terminal_name, terminal_sha)
        require(manifest['passed'] is result['passed'] and manifest['terminal'] == pin(bodies['evidence/' + terminal_name]), 'manifest actual outcome')
        bodies['manifest.json'] = manifest_raw
        require(read(Path(archive_path), MAX_TOTAL) == raw, 'archive input posthash')
        destination = Path(destination)
        require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
                and not os.path.lexists(destination), 'fresh retention destination')
        destination.mkdir(mode=0o755)
        for n, data in sorted(bodies.items()):
            guard(); path = destination / relative(n); path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('xb') as stream:
                stream.write(data); stream.flush(); os.fsync(stream.fileno())
            require(read(path) == data, 'retained body mismatch')
        require(read(Path(archive_path), MAX_TOTAL) == raw and read(Path(__file__).resolve()) == own
                and all(read(destination / n) == v for n, v in bodies.items()), 'final retained/input posthashes')
        report = dict(schema='ferric-readiness40-scoped-warm-parent-cpu-retention-v1', archive=pin(raw), terminal=manifest['terminal'],
            files={n: pin(v) for n, v in sorted(bodies.items())}, passed=result['passed'], original_receipts_unchanged=True,
            external_bodies_rehashed_locally=False, project_execution=False, gpu_execution=False,
            full_model_acceptance=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
        guard()
        with (destination / 'retention.json').open('xb') as stream:
            stream.write(encoded(report)); stream.flush(); os.fsync(stream.fileno())
    guard(); print(json.dumps(report if mode == 'export' else dict(destination=str(destination), passed=result['passed'], files=len(bodies))))
    signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()
