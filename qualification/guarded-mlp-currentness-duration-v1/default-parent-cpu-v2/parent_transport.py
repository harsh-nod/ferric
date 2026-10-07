"""Data-only scoped parent clone using actually qualified worker postimages."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import sys
import tarfile

P = Path(__file__).resolve().parent
PROPOSAL_ROOT = Path("/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216/proposals/guarded-mlp-model-interface-v228-v1/model-currentness-duration-v1/worker-v2")
FULL = PROPOSAL_ROOT.parent
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/guarded-mlp-full2303-scoped-tail-v1/parent-cpu-v1'
QWORKER = F / 'qualification/guarded-mlp-currentness-duration-v1/default-cpu-attempt-v2'
QBASEWORKER = F / 'qualification/guarded-mlp-full2303-scoped-tail-v1/cpu-v1'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
BASE = E / 'guarded-mlp-full2303-scoped-tail-parent-cpu-v228-v1'
ROOT = E / 'guarded-mlp-currentness-duration-default-parent-cpu-v228-v2'
OLD_ROOT = str(BASE)
WORKER_ROOT = "/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-currentness-duration-default-cpu-v228-v2"
BASE_WORKER_COMPLETE = {"bytes":2498847,"sha256":"f25bd6d9f582a4e632c05e385961c260c0c95d2a119bac5af4927af6dc3b61ed"}
WORKER_COMPLETE = {"bytes":2544682,"sha256":"c2371fe9155ddc2046c1e1cddae12447ec48a77b6c18df17d4ece2071f81ccbe"}
BASE_WORKER_SOURCES = {"bytes":435848,"sha256":"eda7f5f696a5b93d048923cd5f8bd83b51a5ca9cb77efab8322da97f4286e928"}
WORKER_SOURCES = {"bytes":447045,"sha256":"1c61d9240a1bd44c0b7cd9f7799c1e7752497d135b003c1e0036684694061b07"}
W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
BASENAME = 'guarded-mlp-currentness-duration-default-parent-input-v228-v2.tar.gz'
CONTROLLER_PIN = {"bytes":51334,"sha256":"584942b6c65efd5b4b410d84cea65ac24d38be8273c3f9d0a84e0712520f1ccc"}
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
SUPPORT_PIN = dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13')
BASE_COMPLETE = {"bytes":4077579,"sha256":"6e61fbbd1b1e82d97102e311272a947744858a7123ca7c73062b656bc8bdaea2"}
BASE_SOURCES = {"bytes":521263,"sha256":"3f451da9914922109ffc0b232fe276f99aca98ecce3723fc7285faeb9fa6feea"}
PROPOSAL_PINS = {"worker-proposal.json":{"bytes":25694,"sha256":"c6224930fef73622d36b2634f5ef88846db8c1b55e9a7e3fdd550247e5024e52"},"worker-proposal-v1.json":{"bytes":22283,"sha256":"e7a0c23826acc56b83b53fb254b7f17fef9e6f245b04d80ea98b66d71f6d7e9f"},"worker-failed-v1.json":{"bytes":2267201,"sha256":"a3f48e67451287afca3ee91000209a2c3d9c65b693659a4afa73882ab2615d8b"},"runtime-proposal.json":{"bytes":14821,"sha256":"85cd4a641b0e7429f3eeb8aebe310e56a64487b467ca5e81431002c4a7ffc8aa"},"interface.md":{"bytes":3488,"sha256":"9c724086a502043303601f7fe32036f67abf84ebb4ff76c4e524b1544e315303"}}
CACHE_PIN = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
MODE = "default"
DIAGNOSTIC = MODE == 'diagnostic'
DIAGNOSTIC_FEATURE = 'engineering-currentness-duration-diagnostics'
OLD_FEATURE = 'guarded-mlp-readiness-engineering'
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
PARENT_REL = 'adapters/m1-engineering-execution-v1'
HELPERS = {'run_cpu.py', 'supervisor.py', 'qualification_support.py'}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def parse(raw):
    def pairs(rows):
        result = {}
        for name, value in rows:
            require(name not in result, 'duplicate JSON key')
            result[name] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical source')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 <= before.st_size <= 64 << 20,
                'bounded ordinary source')
        body = stream.read((64 << 20) + 1)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and len(body) == before.st_size, 'source read drift')
    return body


def ordinary(name):
    return type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute() \
        and '..' not in Path(name).parts and name not in ('', '.')


def contract(packed):
    require(CONTROLLER_PIN is not None and WORKER_COMPLETE is not None and WORKER_SOURCES is not None,
            'reviewed controller and actual coupled worker pins must be bound')
    require(pin(packed['run_cpu.py']) == CONTROLLER_PIN
            and pin(packed['supervisor.py']) == SUPERVISOR_PIN
            and pin(packed['qualification_support.py']) == SUPPORT_PIN, 'three exact reviewed harness inputs')
    fixed = dict(PROPOSAL_PINS, **{'parent-complete.json': BASE_COMPLETE,
        'parent-sources.json': BASE_SOURCES, 'worker-complete.json': WORKER_COMPLETE,
        'worker-sources.json': WORKER_SOURCES,
        'baseline-worker-sources.json': BASE_WORKER_SOURCES,
        'baseline-worker-complete.json': BASE_WORKER_COMPLETE})
    bodies = {n.removeprefix('inputs/'): raw for n, raw in packed.items() if n.startswith('inputs/')}
    require(all(pin(bodies[n]) == v for n, v in fixed.items()), 'literal actual parent/worker/source identities')
    base = parse(bodies['parent-complete.json'])
    old_map = parse(bodies['parent-sources.json'])
    proposal = parse(bodies['worker-proposal.json'])
    runtime_proposal = parse(bodies['runtime-proposal.json'])
    worker = parse(bodies['worker-complete.json'])
    worker_map = parse(bodies['worker-sources.json'])
    baseline_worker = parse(bodies['baseline-worker-complete.json'])
    base_worker_map = parse(bodies['baseline-worker-sources.json'])
    require(base['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-parent-cpu-v1'
            and base['source_generation'] == 'full2303-scoped-tail-parent-v1'
            and len(old_map) == 1298 and len(base['phases']) == 68 and len(base['tests']) == 58
            and len(base['inventory']) == 1047 and sum(v['passed'] for v in base['tests'].values()) == 584
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['full2303_scoped_tail_parent_source_added'] is True
            and base['full2303_scoped_tail_policy_prepublication_checked'] is True
            and base['readiness40_bank_scoped_census_tail_parent_source_added'] is True
            and base['readiness40_bank_scoped_census_tail_policy_prepublication_checked'] is True
            and base['qualified_worker_sources_preserved'] is True
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual584 selected Full Tail parent baseline')
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
                    and p['observed_signals'] == [] for p in receipt['phases']),
                'actual clean source/CPU lifecycle')
    require(baseline_worker['schema'] == 'ferric-guarded-mlp-full2303-scoped-tail-cpu-v1'
            and baseline_worker['source_generation'] == 'full2303-scoped-tail-coupled-v1'
            and len(base_worker_map) == 1063
            and (baseline_worker['tests']['worker-tests']['passed'], baseline_worker['tests']['worker-tests']['ignored']) == (838, 4)
            and (baseline_worker['tests']['kfd-tests']['passed'], baseline_worker['tests']['kfd-tests']['ignored']) == (1180, 8),
            'actual direct Full Tail coupled baseline')
    require(proposal['schema'] == 'ferric-readiness40-tail-currentness-duration-worker-parent-source-v1'
            and proposal['source_only'] is True and len(proposal['files']) == 16
            and proposal['features']['worker'] == dict(name=DIAGNOSTIC_FEATURE,
                dependencies=['fe2o3-kfd/' + DIAGNOSTIC_FEATURE])
            and proposal['features']['parent'] == dict(name=DIAGNOSTIC_FEATURE,
                dependencies=[OLD_FEATURE])
            and proposal['features']['default_features_changed'] is False
            and proposal['features']['locks_changed'] is False
            and all(proposal['guarantees'][k] is False for k in ('new_execution_authority',
                'currentness_policy_changed', 'native_execution', 'numerical_acceptance',
                'performance_claim', 'canonical_changed', 'compiled', 'tests_executed',
                'formatted', 'ambient_environment_flag', 'new_selector', 'new_runtime_facade')),
            'fixed duration source and feature-only nonauthority')
    require(any(compact(p) == BASE_COMPLETE for p in proposal['requires'])
            and any(compact(p) == BASE_SOURCES for p in proposal['requires'])
            and any(compact(p) == BASE_WORKER_COMPLETE for p in proposal['requires'])
            and any(compact(p) == BASE_WORKER_SOURCES for p in proposal['requires'])
            and any(compact(p) == PROPOSAL_PINS['runtime-proposal.json'] for p in proposal['requires'])
            and compact(proposal['interface']) == PROPOSAL_PINS['interface.md'],
            'original parent/worker/runtime/interface dependency joins')
    prior_proposal = parse(bodies['worker-proposal-v1.json'])
    failed_v1 = parse(bodies['worker-failed-v1.json'])
    repair = proposal['repair']
    repair_path = 'adapters/tp-peer-finite-engineering-worker-v1/src/native_guarded_mlp_readiness_cli_v1.rs'
    require(proposal['source_generation'] == 'currentness-duration-worker-parent-v2-cfg-assignment-block'
            and compact(proposal['predecessor_manifest']) == PROPOSAL_PINS['worker-proposal-v1.json']
            and compact(repair['failed_attempt']) == PROPOSAL_PINS['worker-failed-v1.json']
            and prior_proposal['schema'] == proposal['schema']
            and prior_proposal['source_only'] is True
            and all(proposal[k] == prior_proposal[k] for k in
                ('features', 'source_census', 'test_contract', 'guarantees', 'unchanged_contracts'))
            and compact(prior_proposal['interface']) == PROPOSAL_PINS['interface.md'],
            'immutable original source and exact cumulative syntax repair')
    old_rows = {r['path']: r for r in prior_proposal['files']}
    new_rows = {r['path']: r for r in proposal['files']}
    require(len(old_rows) == len(prior_proposal['files']) == len(new_rows) == len(proposal['files']) == 16
            and set(old_rows) == set(new_rows)
            and all(new_rows[n]['before'] == old_rows[n]['before']
                    and new_rows[n]['repository'] == old_rows[n]['repository'] for n in old_rows)
            and [n for n in old_rows if old_rows[n]['after'] != new_rows[n]['after']] == [repair_path]
            and repair['path'] == repair_path
            and compact(repair['v1_after']) == old_rows[repair_path]['after']
            and compact(repair['v2_after']) == new_rows[repair_path]['after']
            and repair['before_text'] == "            #[cfg(not(feature = \"engineering-currentness-duration-diagnostics\"))]\n            self.tail_counts = Some(owner.close_with_tail_scoped(request, digest)?);"
            and repair['after_text'] == "            #[cfg(not(feature = \"engineering-currentness-duration-diagnostics\"))]\n            {\n                self.tail_counts = Some(owner.close_with_tail_scoped(request, digest)?);\n            }"
            and repair['test_contract_identical_to_predecessor'] is True
            and repair['runtime_changed_from_predecessor'] is False
            and repair['unstable_language_feature_added'] is False,
            'only the attributed assignment gains a block; other fifteen rows and all tests unchanged')
    failed_phases = failed_v1['phases']
    require(failed_v1['schema'] == 'ferric-guarded-mlp-currentness-duration-cpu-v1'
            and failed_v1['qualification_mode'] == 'default'
            and failed_v1['source_generation'] == 'currentness-duration-default-coupled-v1'
            and failed_v1['passed'] is False and type(failed_v1['failure']) is str
            and failed_v1['postcheck_errors'] == []
            and len(failed_phases) == 23
            and [p['exit_code'] for p in failed_phases] == [0] * 22 + [101]
            and failed_phases[-1]['label'] == 'worker-tests-build'
            and all(p['natural_exit'] is True and p['reaped'] is True
                    and p['process_group_absent'] is True and p['forced_cleanup'] is False
                    and p['timed_out'] is False and p['exception'] is None
                    and p['storage_failure'] is None and p['observed_signals'] == [] for p in failed_phases)
            and compact(failed_v1['readset']['worker-proposal.json']) == PROPOSAL_PINS['worker-proposal-v1.json']
            and repair['compiler_stderr_from_terminal'] == failed_phases[-1]['stderr']
            and 'worker-tests' not in failed_v1['tests'],
            'original default failure retained as ancestry, never successful qualification')
    require(runtime_proposal['schema'] == 'ferric-currentness-duration-runtime-source-v1'
            and runtime_proposal['feature'] == DIAGNOSTIC_FEATURE
            and len(runtime_proposal['files']) == 10
            and runtime_proposal['test_contract']['feature_off_new_tests'] == []
            and len(runtime_proposal['test_contract']['feature_on_new_tests']) == 12,
            'fixed duration runtime source and two-mode test contract')
    require(worker['schema'] == 'ferric-guarded-mlp-currentness-duration-cpu-v1'
            and worker['qualification_mode'] == MODE
            and worker['source_generation'] == 'currentness-duration-' + MODE + '-coupled-v2'
            and len(worker_map) == 1069 and len(worker['artifacts']) == 11
            and worker['selected_runtime_features'] == sorted(['default', 'engineering-gfx950']
                + ([DIAGNOSTIC_FEATURE] if DIAGNOSTIC else []))
            and worker['selected_worker_features'] == ([DIAGNOSTIC_FEATURE] if DIAGNOSTIC else [])
            and worker['currentness_duration_diagnostic_build'] is DIAGNOSTIC
            and worker['default_check_no_default_features'] is True
            and worker['default_check_diagnostic_feature_requested'] is DIAGNOSTIC
            and worker['currentness_duration_runtime_source_added'] is True
            and worker['currentness_duration_worker_source_added'] is True
            and worker['currentness_duration_native_execution'] is False
            and worker['runtime_source_changed'] is True
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['artifacts']['worker']['cargo_artifact']['features']
                == ([DIAGNOSTIC_FEATURE] if DIAGNOSTIC else [])
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (853 if DIAGNOSTIC else 838, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1192 if DIAGNOSTIC else 1180, 0, 8)
            and all(worker[k] is False for k in ('full2303_scoped_tail_native_execution',
                'readiness40_bank_scoped_census_tail_native_execution', 'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')), 'matched actual mode and tested executable')
    require(compact(worker['readset']['worker-proposal.json']) == PROPOSAL_PINS['worker-proposal.json']
            and compact(worker['readset']['runtime-proposal.json']) == PROPOSAL_PINS['runtime-proposal.json']
            and compact(worker['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual duration coupled direct proposal and predecessor readset')
    for label, added in (('worker-tests', proposal['test_contract']['feature_on_new_tests']),
                        ('kfd-tests', runtime_proposal['test_contract']['feature_on_new_tests'])):
        old_names = {r['name']: r['outcome'] for r in baseline_worker['tests'][label]['named']}
        new_names = added if DIAGNOSTIC else []
        require(len(new_names) == len(set(new_names)) and not set(old_names) & set(new_names)
                and {r['name']: r['outcome'] for r in worker['tests'][label]['named']}
                    == dict(old_names, **{n: 'ok' for n in new_names}),
                'all actual old runtime/worker names and exact diagnostic additions')
    runtime_rows = {'fe2o3/' + r['path']: r for r in runtime_proposal['files']}
    prior_runtime = {n: compact(r) for n, r in base_worker_map.items() if n.startswith('fe2o3/')}
    require(len(prior_runtime) == 827 and len(runtime_rows) == 10
            and sum(r['before'] is None for r in runtime_rows.values()) == 2, 'runtime source extent')
    for n, r in runtime_rows.items():
        require(prior_runtime.get(n) == r['before'] and r['repository'] == 'fe2o3', 'runtime preimage')
        prior_runtime[n] = r['after']
    require({n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith('fe2o3/')}
                == prior_runtime and len(prior_runtime) == 829
            and {n for n in worker_map if n.startswith('fe2o3/')} == set(prior_runtime)
            and all(compact(worker_map[n]) == r for n, r in prior_runtime.items()
                    if n not in runtime_rows or not n.endswith('.rs')),
            'only reviewed runtime Rust formatting, exact Cargo features')
    worker_rows = {'ferric/' + r['path']: r for r in proposal['files']
                   if ('ferric/' + r['path']).startswith(WORKER_PREFIX)}
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']
                   if r['path'].startswith(PARENT_REL + '/')}
    require(len(worker_rows) == 12 and len(parent_rows) == 4
            and set(worker_rows) | set(parent_rows) == {'ferric/' + r['path'] for r in proposal['files']}
            and sum(r['before'] is None for r in worker_rows.values()) == 4
            and all(r['before'] is not None for r in parent_rows.values())
            and all(r['repository'] == 'ferric' for r in proposal['files'])
            and all(n.endswith('.rs') or n.endswith('/Cargo.toml') for n in (*worker_rows, *parent_rows)),
            'exact twelve worker and four parent rows including two Cargo features')
    old = {n: compact(r) for n, r in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1295 and sum(n.startswith(WORKER_PREFIX) for n in old) == 232
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'actual1295 parent baseline with232 worker bodies')
    pre_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    require(pre_worker == {n: compact(r) for n, r in base_worker_map.items() if n.startswith(WORKER_PREFIX)},
            'entire actual parent and coupled worker source agreement')
    for n, r in worker_rows.items():
        require(pre_worker.get(n) == r['before'], 'worker preimage or absence')
        pre_worker[n] = r['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(pre_worker) == len(qualified_worker) == 236 and set(pre_worker) == set(qualified_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)} == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items()
                    if n not in worker_rows or not n.endswith('.rs'))
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker),
            'all236 actually qualified worker bodies, only eleven reviewed Rust formatting paths')
    expected = {n: r for n, r in old.items() if not n.startswith(WORKER_PREFIX)}
    expected.update(qualified_worker)
    for n, r in parent_rows.items():
        require(expected.get(n) == r['before'], 'parent preimage')
        expected[n] = r['after']
    require(len(expected) == 1299 and not set(worker_rows) & set(parent_rows), '1299 closed Ferric composition')
    rows = dict(worker_rows, **parent_rows)
    require(all(ordinary(n) for n in old) and all(ordinary(n) for n in rows), 'ordinary closed source paths')
    names = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    names |= {label + suffix for label in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == names and len(names) == 129, 'closed129 direct lineage bodies')
    require(all(pin(bodies[n]) == compact(base['raw'][n]) for n in names - set(fixed)),
            'actual current recipes and raw named outcomes')
    require(set(packed) == set(rows) | HELPERS | {'inputs/' + n for n in names},
            'closed146 original package bodies')
    for n in rows:
        require(pin(packed[n]) == expected[n], 'exact authored parent or actually qualified worker overlay ' + n)
    for n in HELPERS:
        expected[n] = pin(packed[n])
    require(len(expected) == 1302 and sum(v['bytes'] for v in expected.values()) <= 64 << 20,
            'bounded1302 source map')
    inputs = dict(schema='ferric-guarded-mlp-currentness-duration-parent-cpu-input-v1',
        qualification_mode=MODE, files=dict(sorted(expected.items())), lineage={n: pin(bodies[n]) for n in sorted(names)},
        parent_overlay=sorted(n for n in parent_rows if n.endswith('.rs')), cache_manifest=CACHE_PIN, git_revision=base['git_revision'])
    return inputs, old


def pack():
    archive = W / BASENAME
    require(not os.path.lexists(archive) and not os.path.lexists(P / 'parent-input-manifest.json'),
            'fresh package outputs')
    paths = {'run_cpu.py': P / 'parent_cpu.py', 'supervisor.py': Q / 'supervisor.py',
             'qualification_support.py': Q / 'qualification_support.py',
             'inputs/parent-complete.json': Q / 'evidence/complete.json',
             'inputs/parent-sources.json': Q / 'evidence/sources-after.json',
             'inputs/worker-complete.json': QWORKER / 'evidence/complete.json',
             'inputs/worker-sources.json': QWORKER / 'evidence/sources-after.json',
             'inputs/worker-proposal.json': PROPOSAL_ROOT / 'source-manifest.json',
             'inputs/runtime-proposal.json': FULL / 'runtime-v1/source-manifest.json',
             'inputs/interface.md': FULL / 'worker-v2/INTERFACE.md',
             'inputs/worker-proposal-v1.json': FULL / 'worker-v1/source-manifest.json',
             'inputs/worker-failed-v1.json': W / 'currentness-duration-default-failed-v228-v1.json',
             'inputs/baseline-worker-complete.json': QBASEWORKER / 'evidence/complete.json',
             'inputs/baseline-worker-sources.json': QBASEWORKER / 'evidence/sources-after.json'}
    base = parse(read(paths['inputs/parent-complete.json']))
    proposal = parse(read(paths['inputs/worker-proposal.json']))
    for n in {'parent-lib-list.stdout', 'metadata.stdout'} | {label + suffix for label in base['tests']
            for suffix in ('.stdout', '.command.json')}:
        paths['inputs/' + n] = Q / 'evidence' / n
    for row in proposal['files']:
        if row['path'].startswith(PARENT_REL + '/'):
            n = 'ferric/' + row['path']
            paths[n] = PROPOSAL_ROOT / n
    for name in ('worker-proposal.json',):
        for row in parse(read(paths['inputs/' + name]))['files']:
            n = 'ferric/' + row['path']
            if not n.startswith(WORKER_PREFIX):
                continue
            require(n not in paths, 'disjoint parent and worker overlays')
            paths[n] = QWORKER / n
    bodies = {n: read(path) for n, path in paths.items()}
    inputs, old = contract(bodies)
    canonical = {n: row for n, row in old.items() if not n.startswith(WORKER_PREFIX)}
    canonical.update({n: row for n, row in inputs['files'].items() if n.startswith(WORKER_PREFIX)})
    require(len(canonical) == 1299 and sum(n.startswith(WORKER_PREFIX) for n in canonical) == 236,
            'old1063 nonworker plus236 actually qualified worker preparation closure')
    for n, row in canonical.items():
        require(pin(read(F / n.removeprefix('ferric/'))) == row,
                'canonical predecessor and actual qualified worker body ' + n)
    for row in proposal['files']:
        if row['path'].startswith(PARENT_REL + '/') and row['before'] is None:
            require(not os.path.lexists(F / row['path']), 'new parent source is absent')
    require(all(read(path) == bodies[n] for n, path in paths.items()), 'all packaging inputs unchanged')
    raw_input = encoded(inputs)
    bodies['input-manifest.json'] = raw_input
    require(len(bodies) == 149 and sum(map(len, bodies.values())) <= 32 << 20, 'bounded149-member package')
    with archive.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive_stream:
            for n, raw in sorted(bodies.items()):
                item = tarfile.TarInfo(n); item.size = len(raw); item.mode = 0o600
                archive_stream.addfile(item, io.BytesIO(raw))
    with (P / 'parent-input-manifest.json').open('xb') as stream:
        stream.write(raw_input)
    print(json.dumps(dict(archive=dict(path=str(archive), **pin(read(archive))),
                         input=pin(raw_input), members=149, source_files=1302), sort_keys=True))


def stage(archive_sha, input_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha) and re.fullmatch('[0-9a-f]{64}', input_sha),
            'observed two SHA arguments')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact staging host/UID')
    require(E.resolve(strict=True) == E and not os.path.lexists(ROOT), 'fresh exact parent root')
    require(shutil.disk_usage(E).free >= 40 << 30, '40GiB initial free floor')
    archive_raw = read(E / BASENAME)
    require(pin(archive_raw)['sha256'] == archive_sha and len(archive_raw) <= 32 << 20,
            'actual bounded source archive')
    with tarfile.open(fileobj=io.BytesIO(archive_raw), mode='r:gz') as archive_stream:
        members = archive_stream.getmembers()
        require(len(members) == len({m.name for m in members}) == 149
                and all(m.isfile() and ordinary(m.name) and not m.pax_headers
                        and 0 <= m.size <= 8 << 20 for m in members)
                and sum(m.size for m in members) <= 32 << 20, 'closed regular USTAR package')
        bodies = {m.name: archive_stream.extractfile(m).read(m.size + 1) for m in members}
        require(all(len(bodies[m.name]) == m.size for m in members), 'complete member reads')
    raw_input = bodies.pop('input-manifest.json')
    require(pin(raw_input)['sha256'] == input_sha, 'actual source input')
    expected, old = contract(bodies)
    require(parse(raw_input) == expected, 'entire reconstructed input contract')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/parent-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/parent-sources.json'],
            'immutable actual parent baseline receipts')
    actual_paths = set()
    for directory, dirs, names in os.walk(BASE / 'ferric', followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(not any((Path(directory) / n).is_symlink() for n in dirs), 'no baseline directory aliases')
        actual_paths.update(str((Path(directory) / n).relative_to(BASE)) for n in names)
    require(actual_paths == set(old), 'exact original source subtree roster')
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'all1295 actual parent prehashes')
    os.umask(0o077); ROOT.mkdir(mode=0o700)
    def write(n, raw):
        require(ordinary(n), 'ordinary destination')
        path = ROOT / n; path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
    for n in old:
        if n not in bodies:
            write(n, read(BASE / n))
    for n, raw in bodies.items():
        write(n, raw)
    write('input-manifest.json', raw_input)
    require(all(pin(read(ROOT / n)) == row for n, row in expected['files'].items()),
            'all1302 staged source rows')
    require(all(pin(read(BASE / n)) == row for n, row in old.items()), 'immutable baseline body posthashes')
    require(read(E / BASENAME) == archive_raw, 'archive unchanged during staging')
    require(read(BASE / 'evidence/complete.json') == bodies['inputs/parent-complete.json']
            and read(BASE / 'evidence/sources-after.json') == bodies['inputs/parent-sources.json'],
            'immutable baseline receipt posthashes')
    receipt = dict(schema='ferric-guarded-mlp-currentness-duration-parent-source-stage-v1', passed=True,
        qualification_mode=MODE, archive=pin(archive_raw), input=pin(raw_input), source_files=1302, ferric_files=1299,
        overlay_files=16, parent_format_paths=3, qualified_worker_files=236, qualified_worker_sources_preserved=True,
        helpers=3, lineage_files=129,
        project_execution=False, canonical_changed=False, shared_cache_changed=False, lockfiles_changed=False,
        root=str(ROOT), controller=pin(read(Path(__file__).resolve())))
    write('stage.json', encoded(receipt))
    print(json.dumps(receipt, sort_keys=True))


def main():
    require(__debug__ and sys.dont_write_bytecode, 'nonoptimized python3 -B')
    def timeout(_number, _frame):
        raise RuntimeError('bounded parent source transport deadline')
    signal.signal(signal.SIGALRM, timeout)
    signal.alarm(180)
    if len(sys.argv) == 2 and sys.argv[1] == 'pack':
        pack()
    elif len(sys.argv) == 4 and sys.argv[1] == 'stage':
        stage(sys.argv[2], sys.argv[3])
    else:
        raise ValueError('parent_transport.py pack | stage ARCHIVE_SHA INPUT_SHA')
    signal.alarm(0)


if __name__ == '__main__':
    main()
