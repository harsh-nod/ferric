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

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v228-v1')
OLD_ROOT = str(ROOT.parent / 'guarded-mlp-full2303-bank-scoped-census-parent-cpu-v228-v1')
WORKER_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-scoped-tail-cpu-v228-v1'
WORKER_COMPLETE = {"bytes":2449865,"sha256":"a2e3d99f94b5b01223f42f3d8917c2cf6a1fc25f9227d4c341bc2b9a7cc9285c"}
WORKER_SOURCES = {"bytes":424416,"sha256":"a2a74ed023e9b2aac224621d37f328d097838c317c901a361c1496064e162ec4"}
BASE_WORKER_COMPLETE = {"bytes":2475794,"sha256":"a7fcea3f09cfcda84ba556f60b5f1578ed378a45fc84bb81fbfa6ed31273cd64"}
BASE_WORKER_SOURCES = {"bytes":438675,"sha256":"5372e185504d11ed89d45cfb3d3e32672f93e8d1bbf3d04e44932b280300528c"}
PARENT = 'ferric/adapters/m1-engineering-execution-v1/'
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
OLD_FEATURE = 'guarded-mlp-readiness-engineering'
FULL_FEATURE = 'guarded-mlp-full2303-engineering'
FEATURE = OLD_FEATURE + ',' + FULL_FEATURE
ADDED_BINARY = 'ferric-qwen3-guarded-mlp-full2303-engineering'
INPUT = {"bytes":272903,"sha256":"4d17d38cceb6e2607ffa89f747ac6c5422f772b3dded645a03c77823579fb4eb"}
SOURCE_ARCHIVE = {"bytes":1247091,"sha256":"e7119687b35845bb87ad0db3fa1f41c4c2880fd271085180aeda6ebcc11698df"}
CACHE = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
HELPERS = {
    'run_cpu.py': {"bytes":46865,"sha256":"db1f0c44ba309b99068dc42275f640c23004c3435e1fdf9e0440d2d82bdd9313"},
    'qualification_support.py': dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'),
    'supervisor.py': dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
    'parent_transport.py': {"bytes":28648,"sha256":"24e6b1cea88b8a2de01f8b19064296d6e2b4b788d4719ce285b37577a792b861"},
    'stage_parent_cache.py': dict(bytes=14358, sha256='0fe8b95236b68bd614160984501b8b7949f925abac2c0887f49b4d3ffa16ebc9'),
}
BASE_COMPLETE = {"bytes":4092678,"sha256":"6580942cb6b39d69b6fdac91a48ac6e0a25cd4e7648c4175152f3620ff7bacb6"}
BASE_SOURCES = {"bytes":525041,"sha256":"cf69d8bb33a1c9bf2fa0238e18da01ef70bba182a488388de378d1f611bc35fb"}
PROPOSAL_PINS = {"parent-proposal.json":{"bytes":23473,"sha256":"840417429d3357155672cc5d096d592329461dd742d6d2476580759495dd116a"},"worker-proposal.json":{"bytes":27229,"sha256":"96463bf9a70d6020e28c2d44260f589c38e7f61229f8c8e77f28242f1893ca71"},"runtime-proposal.json":{"bytes":10858,"sha256":"71da4afd43eb9f7de8fa1cac47cf8e9b2f0ececc86cfb98877b94678d500df83"},"interface.md":{"bytes":24881,"sha256":"749fd470a27cf7630f292b04c5464567e4e7d398ee7855b1fbd152824421a8e4"}}
PARENT_REL = PARENT.removeprefix('ferric/').removesuffix('/')
WORKER_PREFIX = WORKER
BASE_PINS = dict(PROPOSAL_PINS, **{
    'parent-complete.json': BASE_COMPLETE,
    'parent-sources.json': BASE_SOURCES,
    'worker-complete.json': WORKER_COMPLETE,
    'worker-sources.json': WORKER_SOURCES,
    'baseline-worker-sources.json': BASE_WORKER_SOURCES,
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
    bins = sorted(base['artifacts'])
    rows += [('parent-builds', [cargo, 'build', '--profile', 'test', *common, *feature,
                              *[v for n in bins for v in ('--bin', n)], '--message-format=json'], 1200),
             ('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], 1200)]
    require(len(rows) == 68 and len({r[0] for r in rows}) == 68, 'exact sixty-eight recipes')
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
            and result['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v1', 'clean actual outcome')
    for key in ('readiness40_bank_scoped_census_tail_native_execution', 'readiness40_bank_scoped_warm_native_execution', 'readiness40_bank_scoped_census_native_execution',
                'full2303_scoped_warm_native_execution', 'full2303_launch_admitted',
                'full2303_bank_scoped_census_native_execution', 'full2303_deadline_changed',
                'readiness40_scoped_warm_native_execution', 'currentness_temporal_equivalence_claim',
                'shared_full_host_timing_native_execution', 'readiness40_shared_full_native_execution', 'parent_host_timing_native_execution', 'ordinary_wire_schema_changed',
                'ordinary_observation_schema_changed', 'full2303_native_execution', 'full2303_launch_feasibility', 'gpu_execution', 'causal_layer0_native_execution', 'position5_native_execution', 'readiness_native_execution', 'full_long_workload', 'runtime_suite_rerun',
                'worker_suite_rerun', 'full_model_acceptance', 'numerical_acceptance', 'performance_claim',
                'production_authority', 'performance_policy_changed', 'default_policy_changed',
                'warm_paired_terminal_native_execution', 'global_currentness_policy_changed',
                'hidden_read_policy_changed', 'read_ns_scope_changed', 'full_parent_library_suite_executed'):
        require(result[key] is False, 'false authority/scope: ' + key)
    require(result['readiness40_bank_scoped_census_tail_parent_source_added'] is True
            and result['readiness40_bank_scoped_census_tail_policy_prepublication_checked'] is True
            and result['readiness40_bank_scoped_census_parent_source_added'] is True
            and result['allocation_preflights_changed'] is True
            and result['allocation_preflights_changed_only_for_explicit_warm_position5'] is False
            and result['allocation_preflights_changed_only_for_explicit_warm_position5_or_full'] is True
            and result['full2303_bank_scoped_census_parent_source_added'] is True
            and result['full2303_bank_scoped_census_policy_prepublication_checked'] is True
            and result['readiness40_bank_scoped_warm_parent_source_added'] is True
            and result['full2303_scoped_warm_parent_source_added'] is True
            and result['full2303_scoped_policy_prepublication_checked'] is True
            and result['readiness40_scoped_warm_parent_source_added'] is True
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
    require(compact(result['input_manifest']) == INPUT and len(inputs['files']) == 1293
            and len(inputs['lineage']) == 127, 'actual input map')
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
    worker_proposal = value('inputs/worker-proposal.json')
    runtime_proposal = value('inputs/runtime-proposal.json')
    base_worker_map = value('inputs/baseline-worker-sources.json')
    require(base['schema'] == 'ferric-guarded-mlp-full2303-bank-scoped-census-parent-cpu-v1'
            and len(old_map) == 1287 and len(base['phases']) == 68 and len(base['tests']) == 58
            and len(base['inventory']) == 1014 and sum(v['passed'] for v in base['tests'].values()) == 549
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['full2303_bank_scoped_census_parent_source_added'] is True
            and base['full2303_bank_scoped_census_policy_prepublication_checked'] is True
            and base['full2303_scoped_warm_parent_source_added'] is True
            and base['full2303_scoped_policy_prepublication_checked'] is True
            and base['readiness40_scoped_warm_parent_source_added'] is True
            and base['scoped_warm_host_timing_parent_source_added'] is True
            and base['readiness40_bank_scoped_warm_parent_source_added'] is True
            and base['allocation_preflights_changed'] is True
            and base['readiness40_bank_scoped_census_parent_source_added'] is True
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual549 selected Full bank-census parent baseline')
    for receipt, source_map, expected_pin, phase_count in (
            (base, old_map, BASE_SOURCES, 68), (worker, worker_map, WORKER_SOURCES, 27)):
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
    require(worker['schema'] == 'ferric-guarded-mlp-scoped-tail-cpu-v1'
            and worker['source_generation'] == 'scoped-tail-coupled-v1'
            and len(worker_map) == 1059 and len(worker['artifacts']) == 11
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (820, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1180, 0, 8)
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['runtime_source_changed'] is True
            and worker['scoped_tail_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_census_tail_source_added'] is True
            and worker['scoped_bank_rearm_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_warm_source_added'] is True
            and worker['full2303_scoped_warm_source_added'] is True
            and worker['inherited_scoped_currentness_runtime_preserved'] is True
            and worker['scoped_capacity_census_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_census_source_added'] is True
            and worker['allocation_preflights_changed'] is True
            and worker['full2303_bank_scoped_census_source_added'] is True
            and worker['inherited_readiness40_bank_scoped_census_preserved'] is True
            and worker['full2303_deadline_changed'] is False
            and all(worker[k] is False for k in ('full2303_scoped_warm_native_execution',
                'readiness40_bank_scoped_warm_native_execution', 'readiness40_bank_scoped_census_native_execution',
                'full2303_bank_scoped_census_native_execution',
                'readiness40_bank_scoped_census_tail_native_execution',
                'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual Tail runtime and worker qualification with tested real executable')
    require(compact(worker['readset']['worker-proposal.json']) == fixed['worker-proposal.json']
            and compact(worker['readset']['runtime-proposal.json']) == fixed['runtime-proposal.json']
            and compact(worker['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual coupled direct proposal and predecessor readset')
    require(proposal['schema'] == 'ferric-bank-scoped-census-tail-readiness40-parent-source-v1'
            and proposal['source_generation'] == 'bank-scoped-census-tail-parent-v1'
            and compact(proposal['base']['parent_receipt']) == BASE_COMPLETE
            and compact(proposal['base']['parent_sources']) == BASE_SOURCES
            and compact(proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and all(compact(proposal['requires'][key]) == fixed[name] for key, name in
                (('worker_proposal', 'worker-proposal.json'), ('runtime_proposal', 'runtime-proposal.json'),
                 ('interface', 'interface.md')))
            and all(proposal[k] is False for k in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'default_routes_changed', 'default_group_policy_changed',
                'temporal_equivalent_to_full', 'performance_claim', 'numerical_acceptance')),
            'exact timed-only Readiness40 Tail parent proposal')
    require(worker_proposal['schema'] == 'ferric-readiness40-bank-scoped-census-tail-worker-source-v4'
            and len(worker_proposal['files']) == 13
            and worker_proposal['source_generation'] == 'readiness40-bank-scoped-census-tail-worker-v4'
            and worker_proposal['new_explicit_readiness40_tail_route'] is True
            and worker_proposal['first_two_forwards_unchanged'] is True
            and compact(worker_proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(worker_proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and compact(worker_proposal['requires']['runtime']) == fixed['runtime-proposal.json']
            and compact(worker_proposal['requires']['interface']) == fixed['interface.md']
            and worker_proposal['conditional_qualification'] == {"worker_files":228,"passed":820,"ignored":4,"inventory":824,"new_library_tests":20,"new_readiness_cli_tests":1,"targets":{"library":{"passed":793,"ignored":4},"binary":{"passed":0,"ignored":0},"readiness_cli":{"passed":11,"ignored":0},"wire":{"passed":16,"ignored":0}},"coupled_sources":1059,"runtime_sources":827,"runtime_passed":1180,"runtime_ignored":8,"helper_sources":4,"phases":27,"elf_products":11,"qualified":False}
            and all(worker_proposal[k] is False for k in ('compiled', 'tested', 'canonical_changed',
                'native_execution', 'runtime_changed', 'default_group_policy_changed',
                'existing_selectors_changed', 'full2303_route_changed', 'allocation_preflights_changed',
                'currentness_temporal_equivalence_claim', 'production_authority',
                'performance_claim', 'numerical_acceptance', 'full_launch_admitted')),
            'qualified Tail worker source lineage')
    require(runtime_proposal['schema'] == 'ferric-scoped-tail-runtime-source-v1'
            and runtime_proposal['status'] == 'source-only-uncompiled-unexecuted'
            and compact(runtime_proposal['interface']) == fixed['interface.md']
            and runtime_proposal['baseline']['runtime_map_rows'] == 825
            and runtime_proposal['conditional']['runtime_map_rows'] == 827
            and runtime_proposal['conditional']['runtime_passed'] == 1180
            and runtime_proposal['conditional']['runtime_ignored'] == 8
            and all(value is False for value in runtime_proposal['claims'].values()),
            'exact separately qualified closed tail runtime source')
    old_runtime = {n: compact(r) for n, r in base_worker_map.items() if n.startswith('fe2o3/')}
    runtime_rows = {'fe2o3/' + r['path']: r for r in runtime_proposal['files']}
    require(len(old_runtime) == 825 and len(runtime_rows) == len(runtime_proposal['files']) == 6
            and sum(r['before'] is None for r in runtime_rows.values()) == 2
            and all(n.startswith('fe2o3/crates/fe2o3-kfd/src/') and n.endswith('.rs')
                    and r['repository'] == 'fe2o3' for n, r in runtime_rows.items()),
            'six closed runtime paths with two additions')
    pre_runtime = dict(old_runtime)
    for n, row in runtime_rows.items():
        require(pre_runtime.get(n) == row['before'], 'actual runtime preimage or absence ' + n)
        pre_runtime[n] = row['after']
    qualified_runtime = {n: compact(r) for n, r in worker_map.items() if n.startswith('fe2o3/')}
    require(len(qualified_runtime) == 827 and set(qualified_runtime) == set(pre_runtime)
            and {n: compact(r) for n, r in worker['preformat_sources'].items()
                 if n.startswith('fe2o3/')} == pre_runtime
            and all(qualified_runtime[n] == r for n, r in pre_runtime.items() if n not in runtime_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_runtime),
            'only six declared runtime format paths within actual coupled qualification')
    worker_rows = {'ferric/' + r['path']: r for r in worker_proposal['files']}
    require(len(worker_rows) == 13 and sum(r['before'] is None for r in worker_rows.values()) == 4
            and all(n.startswith(WORKER_PREFIX) and n.endswith('.rs') and r['repository'] == 'ferric'
                    for n, r in worker_rows.items()), 'thirteen closed worker paths with four additions')
    require(all(worker_rows.get('ferric/' + r['path']) == {k: r[k] for k in ('repository', 'path', 'before', 'after')}
                for r in proposal['requires']['imported_policy_rows'])
            and len(proposal['requires']['imported_policy_rows']) == 2,
            'exact imported worker policy and test source rows')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1284 and sum(n.startswith(WORKER_PREFIX) for n in old) == 224
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual1284 parent source closure with224 worker bodies')
    pre_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    require(pre_worker == {n: compact(r) for n, r in base_worker_map.items()
                           if n.startswith(WORKER_PREFIX)},
            'all224 actual parent worker bodies equal the coupled baseline')
    for n, row in worker_rows.items():
        require(pre_worker.get(n) == row['before'], 'actual worker preimage or absence ' + n)
        pre_worker[n] = row['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 228 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in worker_map) == 827,
            'only thirteen qualified worker format paths; all228 final worker bodies preserved')
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
    require(len(expected) == 1290 and not set(parent_rows) & set(worker_rows),
            '1290 Ferric bodies with disjoint ownership')
    rows = dict(worker_rows, **parent_rows)

    lineage = set(BASE_PINS) | {'parent-lib-list.stdout', 'metadata.stdout'}
    lineage |= {n + suffix for n in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(inputs['lineage']) == lineage and len(lineage) == 127, 'exact127 direct lineage')
    for n in lineage - set(BASE_PINS):
        require(inputs['lineage'][n] == compact(base['raw'][n]), 'baseline raw ancestry')
    require(inventory(body('inputs/parent-lib-list.stdout')) == base['inventory'], 'prior raw inventory')
    pre = value('evidence/sources-preformat.json')
    before = value('evidence/sources-before.json') if result['input_sources'] is not None else None
    after = value('evidence/sources-after.json')
    require(result['preformat_sources'] == pre and result['input_sources'] == before
            and after == result['final_sources']
            and inputs['files'] == {n: compact(v) for n, v in pre.items()}
            and set(pre) == set(after) and len(after) == 1293, 'all source maps')
    if before is not None:
        require(result['source_unchanged'] is True and before == after, 'clean post-format sources')
    else:
        require(not success and result['source_unchanged'] is False, 'failed preformat-only attempt')
    require(all(row['path'] == str(ROOT / relative(n)) for n, row in after.items()), 'source path mapping')
    overlay = parent_rows
    expected.update({n: HELPERS[n] for n in ('run_cpu.py', 'qualification_support.py', 'supervisor.py')})
    require(inputs['files'] == expected, 'entire1293 composed source map')
    formatted = sorted(n for n in after if compact(after[n]) != compact(pre[n]))
    allowed = sorted(overlay)
    require(inputs['parent_overlay'] == allowed and len(allowed) == 7
            and set(formatted) <= set(allowed)
            and len(result['format_changed_paths']) == len(set(result['format_changed_paths']))
            and sorted(result['format_changed_paths']) == (formatted if before is not None else []),
            'only seven parent Rust sources may format; all228 qualified worker bodies preserved')
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
    require(stage['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-parent-source-stage-v1'
            and stage['passed'] is True and stage['archive'] == SOURCE_ARCHIVE and stage['input'] == INPUT
            and stage['controller'] == HELPERS['parent_transport.py'] and stage['root'] == str(ROOT)
            and [stage[k] for k in ('source_files', 'ferric_files', 'overlay_files', 'parent_format_paths',
                                   'qualified_worker_files', 'helpers', 'lineage_files')]
                == [1293, 1290, 20, 7, 228, 3, 127]
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
    require(1 <= len(phases) <= 68 and [p['label'] for p in phases] == [p[0] for p in planned[:len(phases)]], 'serial recipe prefix')
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
        (PARENT + 'src/tp_finite_client/long/readiness_bank_scoped_census_tail_tests.rs',
         'tp_finite_client::long::readiness::bank_scoped_census_tail::tests::', 4, 4),
        (PARENT + 'src/tp_finite_client/long/readiness_host_timing_tests.rs',
         'tp_finite_client::long::readiness::host_timing::tests::', 20, 2),
        (WORKER + 'src/finite_guarded_mlp_readiness_bank_scoped_census_tail_v4_tests.rs',
         'finite_guarded_mlp_readiness_bank_scoped_census_tail_v4::tests::', 11, 11))
    additions = []
    for path, prefix, total, added in test_groups:
        declared = [prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', body(path).decode())]
        new = [n for n in declared if n not in base['inventory']]
        require(len(declared) == len(set(declared)) == total and len(new) == added,
                'exact source-declared new and inherited methods')
        additions.extend(new)
    additions.sort()
    declared_additions = proposal['test_roster']
    library_names = sorted(r['qualified_name'] for r in declared_additions if r['target'] == 'parent_library')
    bin_names = sorted(r['qualified_name'] for r in declared_additions if r['target'] == 'parent_readiness_binary')
    require(len(declared_additions) == 18 and len(additions) == len(set(additions)) == 17
            and additions == library_names and len(set(library_names + bin_names)) == 18,
            'exact seventeen additive library names and one readiness binary name')
    source = body(PARENT + 'src/bin/ferric-qwen3-guarded-mlp-readiness-engineering.rs').decode()
    declared_bin = sorted('tests::' + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source))
    old_bin = {row['name'] for row in base['tests']['readiness-bin-tests']['named']}
    require(len(bin_names) == 1 and not set(bin_names) & old_bin
            and declared_bin == sorted(old_bin | set(bin_names)), 'exact one additive Readiness40 CLI test')
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
    require(set(additions) <= selected_library
            and len(expected_tests) == 58 and sum(map(len, expected_tests.values())) == 567,
            'all549 original outcomes plus eighteen declared additions')
    require(set(result['tests']) <= set(expected_tests), 'only declared selections')
    for label, observed in result['tests'].items():
        require(outcomes(body('evidence/' + label + '.stdout')) == observed
                and {r['name']: r['outcome'] for r in observed['named']} == {n: 'ok' for n in expected_tests[label]}
                and observed['failed'] == observed['ignored'] == 0, 'exact actual named selection')
    if result['inventory']:
        require(inventory(body('evidence/parent-lib-list.stdout')) == result['inventory']
                == sorted(base['inventory'] + additions) and len(result['inventory']) == 1031, '1031 library inventory')
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
        require(len(phases) == 68 and len(result['tests']) == 58 and set(result['tests']) == set(expected_tests)
                and sum(v['passed'] for v in result['tests'].values()) == 567
                and set(products) == set(base['artifacts'])
                and result['all_selected_parent_tests_executed'] is True and len(result['raw']) == 344,
                'complete68/58/567/seven-product success')
    expected_bodies = {'evidence/' + n for n in raw_names | {terminal_name}}
    expected_bodies |= {'inputs/' + n for n in lineage} | set(HELPERS) | parent_sources | worker_sources
    expected_bodies |= {'input-manifest.json', 'stage.json', 'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json', 'evidence.py'}
    require(set(bodies) == expected_bodies and (not success or len(bodies) == 503), 'closed selected evidence bodies')
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
        for source in ('worker-proposal.json',):
            worker_proposal = parse(read(ROOT / 'inputs' / source))
            names.update('ferric/' + r['path'] for r in worker_proposal['files'])
        paths.update({n: ROOT / n for n in names})
        bodies = {n: read(p) for n, p in paths.items()}; bodies['evidence.py'] = own
        require(bodies['evidence/' + terminal_name] == original and tree(ROOT / 'evidence') == set(result['raw']) | {terminal_name},
                'complete original evidence directory')
        result = verify(bodies, terminal_name, terminal_sha)
        live = live_rehash(result)
        require(all(read(p) == bodies[n] for n, p in paths.items()) and read(Path(__file__).resolve()) == own, 'selected final posthashes')
        manifest = dict(schema='ferric-readiness40-bank-scoped-census-tail-parent-cpu-selected-evidence-v1', terminal_name=terminal_name,
            terminal=pin(original), files={n: pin(b) for n, b in sorted(bodies.items())},
            passed=result['passed'], live_rehashed=live, source_body_scope='seven parent overlays plus unchanged Cargo.lock and thirteen qualified Tail worker overlays; all228 worker bodies preserved',
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
        require(manifest['schema'] == 'ferric-readiness40-bank-scoped-census-tail-parent-cpu-selected-evidence-v1'
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
        report = dict(schema='ferric-readiness40-bank-scoped-census-tail-parent-cpu-retention-v1', archive=pin(raw), terminal=manifest['terminal'],
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
