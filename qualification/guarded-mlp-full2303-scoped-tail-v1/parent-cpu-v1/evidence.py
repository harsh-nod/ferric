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

ROOT = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-scoped-tail-parent-cpu-v228-v1")
OLD_ROOT = "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v228-v2"
WORKER_ROOT = "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-scoped-tail-cpu-v228-v1"
WORKER_COMPLETE = {"bytes":2498847,"sha256":"f25bd6d9f582a4e632c05e385961c260c0c95d2a119bac5af4927af6dc3b61ed"}
WORKER_SOURCES = {"bytes":435848,"sha256":"eda7f5f696a5b93d048923cd5f8bd83b51a5ca9cb77efab8322da97f4286e928"}
BASE_WORKER_COMPLETE = {"bytes":2448697,"sha256":"3c4fe3a6c4942b20ec61fd7ea5c3409526e2167c43d24a62fb8660c34b0e713e"}
BASE_WORKER_SOURCES = {"bytes":424416,"sha256":"5de4929b2832eda55a53f99945fc4b64a9fbda28b4d4f8bef07f2e8d5e000d6a"}
PARENT = 'ferric/adapters/m1-engineering-execution-v1/'
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
OLD_FEATURE = 'guarded-mlp-readiness-engineering'
FULL_FEATURE = 'guarded-mlp-full2303-engineering'
FEATURE = OLD_FEATURE + ',' + FULL_FEATURE
ADDED_BINARY = 'ferric-qwen3-guarded-mlp-full2303-engineering'
INPUT = {"bytes":273923,"sha256":"1fca1a72e1647a039a63cd1c9607cd43f366e33e713e2fd63994bec6a122abc5"}
SOURCE_ARCHIVE = {"bytes":1570164,"sha256":"066d3975ba4ea67a08d040d4d2336dc864205b17dbe5e98d09655d4b925caec2"}
CACHE = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
HELPERS = {
    'run_cpu.py': {"bytes":47115,"sha256":"52e5d03a2f31ab6944e58fffd3ae92943ef7162f2e8529e170c4e7e89723dd92"},
    'qualification_support.py': dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'),
    'supervisor.py': dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
    'parent_transport.py': {"bytes":28613,"sha256":"9760f86c72f605d12267f431c482c82a00393ff4e5e0f929c4382b6a4597d9dd"},
    'stage_parent_cache.py': dict(bytes=14343, sha256='d6db632d18c6bb3f85fa0c5b7fe312ae09b91fb934d260477225125680780ecb'),
}
BASE_COMPLETE = {"bytes":4163125,"sha256":"a59c39cfa2be0ec11d0ffa6a352576814917243627ce9a73aca1d4f6501266e9"}
BASE_SOURCES = {"bytes":538263,"sha256":"f8b92a7b220c9733e75aa7745498d915217241369ea2a003edfbdda55f52d4b2"}
PROPOSAL_PINS = {"parent-proposal.json":{"bytes":18956,"sha256":"721425e289ae2e26fc9ed178cfa8f5d6f84f645ea9094cfedfbf3a232e169f4e"},"worker-proposal.json":{"bytes":23469,"sha256":"e53136db6e443f768f92d374395cce3867a160a90994d768555573606e8e8e9a"},"interface.md":{"bytes":4970,"sha256":"0f83da4e18dce6beacadf565035f5e6b5ed7c32d5f44303826db93ff06fbd589"}}
PARENT_REL = PARENT.removeprefix('ferric/').removesuffix('/')
WORKER_PREFIX = WORKER
BASE_PINS = dict(PROPOSAL_PINS, **{
    'parent-complete.json': BASE_COMPLETE,
    'parent-sources.json': BASE_SOURCES,
    'worker-complete.json': WORKER_COMPLETE,
    'worker-sources.json': WORKER_SOURCES,
    'baseline-worker-sources.json': BASE_WORKER_SOURCES,
    'baseline-worker-complete.json': BASE_WORKER_COMPLETE,
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
            if old['label'] == 'parent-full2303-bank-census-policy':
                index = argv.index('--lib') + 1
                require(argv[index] == 'finite_guarded_mlp_full2303_bank_scoped_census_v1::tests::',
                        'exact existing Full bank-census policy selector')
                argv[index] = 'finite_guarded_mlp_full2303_bank_scoped_census'
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
            and result['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-parent-cpu-v1', 'clean actual outcome')
    require(result['source_generation'] == 'full2303-scoped-tail-parent-v1'
            and result['shared_test_fixture_repaired'] is True
            and result['readiness40_bank_scoped_census_tail_parent_source_added'] is True
            and result['readiness40_bank_scoped_census_tail_policy_prepublication_checked'] is True
            and result['full2303_scoped_tail_parent_source_added'] is True
            and result['full2303_scoped_tail_policy_prepublication_checked'] is True
            and result['inherited_readiness40_tail_parent_preserved'] is True,
            'explicit Full Tail source generation and inherited Readiness40 route')
    for key in ('readiness40_bank_scoped_warm_native_execution', 'readiness40_bank_scoped_census_native_execution',
                'full2303_scoped_warm_native_execution', 'full2303_launch_admitted',
                'full2303_bank_scoped_census_native_execution', 'full2303_deadline_changed',
                'readiness40_bank_scoped_census_tail_native_execution', 'full2303_scoped_tail_native_execution',
                'readiness40_scoped_warm_native_execution', 'currentness_temporal_equivalence_claim',
                'shared_full_host_timing_native_execution', 'readiness40_shared_full_native_execution', 'parent_host_timing_native_execution', 'ordinary_wire_schema_changed',
                'ordinary_observation_schema_changed', 'full2303_native_execution', 'full2303_launch_feasibility', 'gpu_execution', 'causal_layer0_native_execution', 'position5_native_execution', 'readiness_native_execution', 'full_long_workload', 'runtime_suite_rerun',
                'worker_suite_rerun', 'full_model_acceptance', 'numerical_acceptance', 'performance_claim',
                'production_authority', 'performance_policy_changed', 'default_policy_changed',
                'warm_paired_terminal_native_execution', 'global_currentness_policy_changed',
                'hidden_read_policy_changed', 'read_ns_scope_changed', 'full_parent_library_suite_executed'):
        require(result[key] is False, 'false authority/scope: ' + key)
    require(result['readiness40_bank_scoped_census_parent_source_added'] is True
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
    require(compact(result['input_manifest']) == INPUT and len(inputs['files']) == 1298
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
    baseline_worker = value('inputs/baseline-worker-complete.json')
    base_worker_map = value('inputs/baseline-worker-sources.json')
    require(base['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v1'
            and base['source_generation'] == 'scoped-tail-parent-v2'
            and base['shared_test_fixture_repaired'] is True
            and len(old_map) == 1293 and len(base['phases']) == 68 and len(base['tests']) == 58
            and len(base['inventory']) == 1032 and sum(v['passed'] for v in base['tests'].values()) == 568
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['full2303_scoped_warm_parent_source_added'] is True
            and base['full2303_scoped_policy_prepublication_checked'] is True
            and base['readiness40_scoped_warm_parent_source_added'] is True
            and base['scoped_warm_host_timing_parent_source_added'] is True
            and base['readiness40_bank_scoped_warm_parent_source_added'] is True
            and base['allocation_preflights_changed'] is True
            and base['readiness40_bank_scoped_census_parent_source_added'] is True
            and base['readiness40_bank_scoped_census_tail_parent_source_added'] is True
            and base['readiness40_bank_scoped_census_tail_policy_prepublication_checked'] is True
            and base['full2303_bank_scoped_census_parent_source_added'] is True
            and base['full2303_bank_scoped_census_policy_prepublication_checked'] is True
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual568 selected Tail V2 parent baseline')
    for receipt, source_map, expected_pin, phase_count in (
            (base, old_map, BASE_SOURCES, 68),
            (baseline_worker, base_worker_map, BASE_WORKER_SOURCES, 27),
            (worker, worker_map, WORKER_SOURCES, 27)):
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
    require(worker['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-cpu-v1'
            and worker['source_generation'] == 'full2303-scoped-tail-coupled-v1'
            and len(worker_map) == 1063 and len(worker['artifacts']) == 11
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (838, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1180, 0, 8)
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['runtime_source_changed'] is False
            and worker['scoped_tail_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_census_tail_source_added'] is True
            and worker['full2303_scoped_tail_source_added'] is True
            and worker['inherited_readiness40_tail_preserved'] is True
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
                'full2303_scoped_tail_native_execution', 'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual Full Tail worker and unchanged runtime with tested real executable')
    require(baseline_worker['schema'] == 'ferric-guarded-mlp-scoped-tail-cpu-v1'
            and baseline_worker['source_generation'] == 'scoped-tail-coupled-v2'
            and len(base_worker_map) == 1059
            and (baseline_worker['tests']['worker-tests']['passed'], baseline_worker['tests']['worker-tests']['ignored']) == (820, 4)
            and (baseline_worker['tests']['kfd-tests']['passed'], baseline_worker['tests']['kfd-tests']['ignored']) == (1180, 8),
            'actual direct Tail V2 coupled baseline')
    require(compact(worker['readset']['worker-proposal.json']) == fixed['worker-proposal.json']
            and compact(worker['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual coupled direct proposal and predecessor readset')
    require(proposal['schema'] == 'ferric-full2303-bank-scoped-census-tail-parent-source-v1'
            and compact(proposal['base']['parent_receipt']) == BASE_COMPLETE
            and compact(proposal['base']['parent_sources']) == BASE_SOURCES
            and compact(proposal['requires']['worker_source_manifest']) == fixed['worker-proposal.json']
            and compact(proposal['requires']['interface']) == fixed['interface.md']
            and compact(proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and all(proposal[k] is False for k in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'default_routes_changed', 'default_group_policy_changed',
                'runtime_api_changed', 'deadline_changed', 'reference_feedback',
                'temporal_equivalent_to_full', 'performance_claim', 'numerical_acceptance',
                'full2303_launch_admitted')), 'exact Full bank-census parent proposal')
    require(worker_proposal['schema'] == 'ferric-full2303-bank-scoped-census-tail-worker-source-v1'
            and len(worker_proposal['files']) == 12
            and worker_proposal['source_generation'] == 'full2303-bank-scoped-census-tail-worker-v1'
            and worker_proposal['new_explicit_full2303_route'] is True
            and worker_proposal['first_two_forwards_unchanged'] is True
            and worker_proposal['full_child_abort_ms'] == 3600000
            and worker_proposal['per_dispatch_timeout_max_ms'] == 10000
            and compact(worker_proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(worker_proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and compact(worker_proposal['interface']) == fixed['interface.md']
            and worker_proposal['conditional_qualification'] == dict(worker_source_files=232,
                passed=838, ignored=4, inventory=842, new_library_methods=17,
                new_readiness_cli_methods=1, library_passed=810, library_ignored=4,
                binary_passed=0, readiness_cli_passed=12, wire_passed=16,
                parent_imported_new_methods=9, shared_sequence_new_methods=0,
                existing_targets_and_old_named_statuses_must_be_preserved=True)
            and all(worker_proposal[k] is False for k in ('compiled', 'tested', 'canonical_changed',
                'native_execution', 'runtime_changed', 'default_group_policy_changed',
                'existing_selectors_changed', 'currentness_temporal_equivalence_claim',
                'performance_claim', 'numerical_acceptance', 'full_launch_admitted')),
            'qualified Full bank-census worker source lineage')
    old_names = {r['name']: r['outcome'] for r in baseline_worker['tests']['worker-tests']['named']}
    new_names = worker_proposal['new_tests']['library'] + worker_proposal['new_tests']['readiness_cli']
    require(len(old_names) == 824 and len(new_names) == len(set(new_names)) == 18
            and not set(old_names) & set(new_names)
            and {r['name']: r['outcome'] for r in worker['tests']['worker-tests']['named']}
                == dict(old_names, **{n: 'ok' for n in new_names})
            and all(worker['tests'][name] == result for name, result in baseline_worker['tests'].items()
                    if name != 'worker-tests'),
            'all old runtime, focused and worker named outcomes preserved with exactly18 additions')
    old_runtime = {n: compact(r) for n, r in base_worker_map.items() if n.startswith('fe2o3/')}
    require(len(old_runtime) == 827
            and {n: compact(r) for n, r in worker_map.items() if n.startswith('fe2o3/')} == old_runtime
            and {n: compact(r) for n, r in worker['preformat_sources'].items()
                 if n.startswith('fe2o3/')} == old_runtime,
            'all827 actual Tail runtime bodies unchanged through worker qualification')
    worker_rows = {'ferric/' + r['path']: r for r in worker_proposal['files']}
    require(len(worker_rows) == 12 and sum(r['before'] is None for r in worker_rows.values()) == 4
            and all(n.startswith(WORKER_PREFIX) and n.endswith('.rs') and r['repository'] == 'ferric'
                    for n, r in worker_rows.items()), 'twelve closed worker paths with four additions')
    require(all(worker_rows.get('ferric/' + r['path']) == {k: r[k] for k in ('repository', 'path', 'before', 'after')}
                for r in proposal['requires']['imported_policy_rows'])
            and len(proposal['requires']['imported_policy_rows']) == 2,
            'exact imported worker policy and test source rows')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1290 and sum(n.startswith(WORKER_PREFIX) for n in old) == 228
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual1290 parent source closure with228 worker bodies')
    pre_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    require(pre_worker == {n: compact(r) for n, r in base_worker_map.items()
                           if n.startswith(WORKER_PREFIX)},
            'all228 actual parent worker bodies equal the coupled baseline')
    for n, row in worker_rows.items():
        require(pre_worker.get(n) == row['before'], 'actual worker preimage or absence ' + n)
        pre_worker[n] = row['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 232 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in worker_map) == 827,
            'only twelve qualified worker format paths; all232 final worker bodies preserved')
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(parent_rows) == len(proposal['files']) == 6
            and sum(r['before'] is None for r in parent_rows.values()) == 1
            and all(n.startswith('ferric/' + PARENT_REL + '/') and n.endswith('.rs')
                    and r['repository'] == 'ferric' for n, r in parent_rows.items()),
            'exact six parent paths with one addition')
    expected = {n: r for n, r in old.items() if not n.startswith(WORKER_PREFIX)}
    expected.update(qualified_worker)
    for n, row in parent_rows.items():
        require(expected.get(n) == row['before'], 'actual parent preimage or absence ' + n)
        expected[n] = row['after']
    require(len(expected) == 1295 and not set(parent_rows) & set(worker_rows),
            '1295 Ferric bodies with disjoint ownership')
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
            and set(pre) == set(after) and len(after) == 1298, 'all source maps')
    if before is not None:
        require(result['source_unchanged'] is True and before == after, 'clean post-format sources')
    else:
        require(not success and result['source_unchanged'] is False, 'failed preformat-only attempt')
    require(all(row['path'] == str(ROOT / relative(n)) for n, row in after.items()), 'source path mapping')
    overlay = parent_rows
    expected.update({n: HELPERS[n] for n in ('run_cpu.py', 'qualification_support.py', 'supervisor.py')})
    require(inputs['files'] == expected, 'entire1298 composed source map')
    formatted = sorted(n for n in after if compact(after[n]) != compact(pre[n]))
    allowed = sorted(overlay)
    require(inputs['parent_overlay'] == allowed and len(allowed) == 6
            and set(formatted) <= set(allowed)
            and len(result['format_changed_paths']) == len(set(result['format_changed_paths']))
            and sorted(result['format_changed_paths']) == (formatted if before is not None else []),
            'only six parent Rust sources may format; all232 qualified worker bodies preserved')
    parent_sources = set(overlay) | {PARENT + 'Cargo.lock'}
    worker_sources = set(worker_rows)
    require(len(parent_sources) == 7 and len(worker_sources) == 12,
            'six parent postimages plus lock and twelve qualified worker postimages')
    for n in worker_sources:
        body(n, after[n])
        require(compact(after[n]) == qualified_worker[n], 'retained actual worker unchanged')
    for n in parent_sources:
        body(n, after[n])
    stage = value('stage.json')
    require(stage['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-parent-source-stage-v1'
            and stage['passed'] is True and stage['archive'] == SOURCE_ARCHIVE and stage['input'] == INPUT
            and stage['controller'] == HELPERS['parent_transport.py'] and stage['root'] == str(ROOT)
            and [stage[k] for k in ('source_files', 'ferric_files', 'overlay_files', 'parent_format_paths',
                                   'qualified_worker_files', 'helpers', 'lineage_files')]
                == [1298, 1295, 18, 6, 232, 3, 127]
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
        (PARENT + 'src/tp_finite_client/long/full2303_tests.rs',
         'tp_finite_client::long::full2303::tests::', 23, 6),
        (WORKER + 'src/finite_guarded_mlp_full2303_bank_scoped_census_tail_v1_tests.rs',
         'finite_guarded_mlp_full2303_bank_scoped_census_tail_v1::tests::', 9, 9))
    additions = []
    for path, prefix, total, added in test_groups:
        declared = [prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', body(path).decode())]
        new = [n for n in declared if n not in base['inventory']]
        require(len(declared) == len(set(declared)) == total and len(new) == added,
                'exact source-declared new and inherited methods')
        additions.extend(new)
    additions.sort()
    declared_additions = proposal['qualified_new_tests']
    require(len(additions) == len(set(additions)) == 15
            and additions == sorted(declared_additions['parent-client']
                                    + declared_additions['full-bank-census-tail-policy']),
            'exact fifteen additive library names')
    bin_names = sorted(declared_additions['full2303-bin'])
    source = body(PARENT + 'src/bin/ferric-qwen3-guarded-mlp-full2303-engineering.rs').decode()
    declared_bin = sorted('tests::' + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source))
    old_bin = {row['name'] for row in base['tests']['full2303-bin-tests']['named']}
    require(len(bin_names) == 1 and not set(bin_names) & old_bin
            and declared_bin == sorted(old_bin | set(bin_names)), 'exact one additive Full2303 CLI test')
    expected_tests = {}
    selected_library = set()
    for label, old_test in base['tests'].items():
        require(outcomes(body('inputs/' + label + '.stdout')) == old_test, 'prior actual selected outcomes')
        old_phase = next(p for p in base['phases'] if p['label'] == label)
        require(value('inputs/' + label + '.command.json')['argv'] == old_phase['argv'], 'prior command ancestry')
        names = {r['name'] for r in old_test['named']}
        if label == 'full2303-bin-tests':
            names |= set(bin_names)
        argv = next(argv for name, argv, _ in planned if name == label)
        if '--lib' in argv:
            selector = argv[argv.index('--lib') + 1]
            names |= {n for n in additions if n == selector or ('--exact' not in argv and selector in n)}
            require(not selected_library & names, 'inherited library scopes remain disjoint')
            selected_library |= names
        expected_tests[label] = names
    full_policy = expected_tests['parent-full2303-bank-census-policy']
    require(len(full_policy) == 16
            and len(base['tests']['parent-full2303-bank-census-policy']['named']) == 7
            and set(declared_additions['full-bank-census-tail-policy']) <= full_policy,
            'exact seven old plus nine new Full policy scope')
    require(set(additions) <= selected_library
            and len(expected_tests) == 58 and sum(map(len, expected_tests.values())) == 584,
            'all568 original outcomes plus sixteen declared additions')
    require(set(result['tests']) <= set(expected_tests), 'only declared selections')
    for label, observed in result['tests'].items():
        require(outcomes(body('evidence/' + label + '.stdout')) == observed
                and {r['name']: r['outcome'] for r in observed['named']} == {n: 'ok' for n in expected_tests[label]}
                and observed['failed'] == observed['ignored'] == 0, 'exact actual named selection')
    if result['inventory']:
        require(inventory(body('evidence/parent-lib-list.stdout')) == result['inventory']
                == sorted(base['inventory'] + additions) and len(result['inventory']) == 1047, '1047 library inventory')
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
                and sum(v['passed'] for v in result['tests'].values()) == 584
                and set(products) == set(base['artifacts'])
                and result['all_selected_parent_tests_executed'] is True and len(result['raw']) == 344,
                'complete68/58/584/seven-product success')
    expected_bodies = {'evidence/' + n for n in raw_names | {terminal_name}}
    expected_bodies |= {'inputs/' + n for n in lineage} | set(HELPERS) | parent_sources | worker_sources
    expected_bodies |= {'input-manifest.json', 'stage.json', 'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json', 'evidence.py'}
    require(set(bodies) == expected_bodies and (not success or len(bodies) == 501), 'closed selected evidence bodies')
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
        manifest = dict(schema='ferric-full2303-scoped-tail-parent-cpu-selected-evidence-v1', terminal_name=terminal_name,
            terminal=pin(original), files={n: pin(b) for n, b in sorted(bodies.items())},
            passed=result['passed'], live_rehashed=live, source_body_scope='six parent overlays plus unchanged Cargo.lock and twelve qualified Full Tail worker overlays; all232 worker bodies preserved',
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
        require(manifest['schema'] == 'ferric-full2303-scoped-tail-parent-cpu-selected-evidence-v1'
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
        report = dict(schema='ferric-full2303-scoped-tail-parent-cpu-retention-v1', archive=pin(raw), terminal=manifest['terminal'],
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
