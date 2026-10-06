"""Synthetic admission tests only; none invokes readelf, ldd or a producer."""
import copy
import hashlib
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import audit_tools as A

# Adjacent bodies are mechanically copied from retained evidence by the fixture packager.
_FIXTURE_ROOT = Path(__file__).resolve().parent
BASELINE_FAILURE_BODY = (_FIXTURE_ROOT / 'baseline-failed.json').read_bytes()
BASELINE_STDERR = (_FIXTURE_ROOT / 'baseline-compile.stderr').read_bytes()
if ((len(BASELINE_FAILURE_BODY), hashlib.sha256(BASELINE_FAILURE_BODY).hexdigest())
        != (17737, '8d4cc1a0cfe0c9104282c3d20349638a616bd98e7e28f92598dcaf0110057ca2')
        or (len(BASELINE_STDERR), hashlib.sha256(BASELINE_STDERR).hexdigest())
        != (123245, 'c60dc2edc62bfe11ba7a8f79ce1465322d2c887aeb3fa5661dae8002a33c5095')):
    raise RuntimeError('packaged baseline fixture pins differ')
BASELINE_FAILURE = json.loads(BASELINE_FAILURE_BODY)


def fixture():
    def pin(path, digest='1' * 64):
        return dict(path=str(path), bytes=10, sha256=digest)

    root = A.CPU_ROOT
    sources = {'fe2o3/file-' + str(i): pin(root / ('fe2o3/file-' + str(i)))
               for i in range(5793)}
    sources['run_cpu.py'] = pin(root / 'run_cpu.py', A.CPU_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = pin(root / 'qualification_helpers.py', A.CPU_HELPER_SHA)
    lineage = dict(proposal=pin('/synthetic/proposal.json', A.PROPOSAL_SHA))
    inputs = dict(schema='ferric-guarded-mlp-s-rpo-qualification-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()})
    package = root / 'fe2o3/crates/rustc-codegen-fe2o3'
    records, artifacts = [], {}
    for role, name, kinds, filenames, src in (
            ('backend', 'rustc_codegen_fe2o3', ['rlib', 'dylib'],
             ['librustc_codegen_fe2o3.so', 'librustc_codegen_fe2o3.rlib'], 'src/lib.rs'),
            ('extractor', 'fe2o3-rustc-extract', ['bin'],
             ['fe2o3-rustc-extract'], 'src/bin/fe2o3-rustc-extract.rs')):
        paths = [str(root / 'target/debug/deps' / name) for name in filenames]
        row = dict(reason='compiler-artifact', manifest_path=str(package / 'Cargo.toml'),
                   target=dict(name=name, kind=kinds, crate_types=kinds, src_path=str(package / src)),
                   profile=dict(test=False), filenames=paths,
                   executable=paths[0] if role == 'extractor' else None)
        records.append(row)
        artifacts[role] = dict(pin=pin(paths[0]), cargo_artifact=row)
        if role == 'backend':
            artifacts['backend-rlib'] = dict(pin=pin(paths[1]), cargo_artifact=row)
    records.append(dict(reason='build-finished', success=True))
    for role in ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction'):
        artifacts[role] = dict(pin=pin(root / 'target/debug/deps' / role), cargo_artifact={})
    tests = {'scope-' + str(i): dict(passed=1, failed=0, ignored=0) for i in range(17)}
    for index, count in enumerate((15, 6, 6, 10, 6)):
        tests['scope-' + str(index)]['passed'] = count
    tests.update(compiler=dict(passed=1239, failed=0, ignored=24),
                 pliron=dict(passed=1504, failed=0, ignored=1))
    value = dict(schema='ferric-guarded-mlp-s-rpo-qualification-cpu-v1',
                 passed=True, failure=None, postcheck_errors=[], source_unchanged=True,
                 diagnostic_build=False, controller=sources['run_cpu.py'],
                 helper=sources['qualification_helpers.py'], source_lineage=lineage,
                 final_compiler_product_phase='compiler-tests-build',
                 phases=[dict(label='phase-' + str(i), exit_code=0, natural_exit=True,
                              reaped=True, process_group_absent=True, forced_cleanup=False,
                              timed_out=False, exception=None, storage_failure=None)
                         for i in range(32)],
                 tests=tests, tests_passed=2798, tests_ignored=25, artifacts=artifacts)
    old = {'bin/' + name: dict(source='/synthetic/old/' + name, bytes=20, sha256='2' * 64)
           for name in A.NAMES}
    manifest = copy.deepcopy(old)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = artifacts[role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, records, manifest, old


class ProducerAdmissionTests(unittest.TestCase):
    def test_final_two_products_only_and_rlib_metadata(self):
        args = fixture()
        proof = A.producer_contract(*args)
        self.assertEqual(proof['rlib_cpu_provenance'], args[0]['artifacts']['backend-rlib'])
        self.assertEqual(len(proof['test_elf_cpu_provenance']), 4)
        self.assertIs(proof['omitted_artifact_bodies_rehashed'], False)

    def test_rejects_failed_incomplete_or_diagnostic_producer(self):
        for key, bad in (('passed', False), ('failure', 'failed'), ('diagnostic_build', True),
                         ('postcheck_errors', ['drift']), ('source_unchanged', False)):
            with self.subTest(key=key):
                args = fixture()
                args[0][key] = bad
                with self.assertRaises(RuntimeError):
                    A.producer_contract(*args)

    def test_rejects_controller_helper_or_proposal_generation_drift(self):
        for key in ('controller', 'helper', 'proposal'):
            with self.subTest(key=key):
                args = fixture()
                row = args[0][key] if key != 'proposal' else args[1]['lineage']['proposal']
                row['sha256'] = '9' * 64
                with self.assertRaises(RuntimeError):
                    A.producer_contract(*args)

    def test_rejects_missing_or_changed_source_map(self):
        for change in ('missing', 'changed'):
            args = fixture()
            if change == 'missing':
                del args[2]['fe2o3/file-0']
            else:
                args[2]['fe2o3/file-0']['sha256'] = '3' * 64
            with self.assertRaises(RuntimeError):
                A.producer_contract(*args)

    def test_rejects_phase_or_test_census_failure(self):
        for change in ('phase-count', 'reap', 'cleanup', 'scope-count', 'passed', 'ignored', 'failed'):
            args = fixture()
            value = args[0]
            if change == 'phase-count':
                value['phases'].pop()
            elif change == 'reap':
                value['phases'][0]['reaped'] = False
            elif change == 'cleanup':
                value['phases'][0]['forced_cleanup'] = True
            elif change == 'scope-count':
                value['tests'].pop('scope-0')
            elif change == 'passed':
                value['tests_passed'] -= 1
            elif change == 'ignored':
                value['tests_ignored'] -= 1
            else:
                value['tests']['scope-0']['failed'] = 1
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.producer_contract(*args)

    def test_rejects_old_or_failed_final_cargo_observation(self):
        for change in ('phase', 'finished', 'record'):
            args = fixture()
            if change == 'phase':
                args[0]['final_compiler_product_phase'] = 'compiler-products'
            elif change == 'finished':
                args[3][-1]['success'] = False
            else:
                args[0]['artifacts']['extractor']['cargo_artifact'] = {}
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.producer_contract(*args)

    def test_rejects_product_outside_target_and_wrong_role(self):
        for change in ('outside', 'wrong-name', 'duplicate', 'test-profile'):
            args = fixture()
            product = args[0]['artifacts']['extractor']
            if change in ('outside', 'wrong-name'):
                path = '/synthetic/fe2o3-rustc-extract' if change == 'outside' else str(A.CPU_ROOT / 'target/wrong')
                product['pin']['path'] = path
                product['cargo_artifact']['filenames'] = [path]
                product['cargo_artifact']['executable'] = path
            elif change == 'duplicate':
                args[3].insert(0, copy.deepcopy(args[3][1]))
            else:
                product['cargo_artifact']['profile']['test'] = True
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.producer_contract(*args)

    def test_rejects_extra_tools_or_old_backend_replacement(self):
        for change in ('extra', 'old-backend', 'body', 'source'):
            args = fixture()
            if change == 'extra':
                args[4]['bin/extra'] = dict(args[4]['bin/lld'])
            elif change == 'old-backend':
                args[4]['bin/librustc_codegen_fe2o3.so'] = args[5]['bin/librustc_codegen_fe2o3.so']
            else:
                args[4]['bin/fe2o3-rustc-extract']['sha256' if change == 'body' else 'source'] = 'wrong'
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.producer_contract(*args)

    def test_rejects_any_of_five_legacy_tool_changes(self):
        for name in set(A.NAMES) - {'fe2o3-rustc-extract', 'librustc_codegen_fe2o3.so'}:
            args = fixture()
            args[4]['bin/' + name]['sha256'] = '3' * 64
            with self.subTest(name=name), self.assertRaises(RuntimeError):
                A.producer_contract(*args)

    def test_rejects_missing_rlib_or_test_product(self):
        for role in ('backend-rlib', 'pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction'):
            args = fixture()
            del args[0]['artifacts'][role]
            with self.subTest(role=role), self.assertRaises(RuntimeError):
                A.producer_contract(*args)


def driver_fixture():
    def pin(path, digest='4' * 64):
        return dict(path=str(path), bytes=20, sha256=digest)
    sources = {'fe2o3/file-' + str(i): pin(A.DRIVER_ROOT / ('fe2o3/file-' + str(i)))
               for i in range(5294)}
    sources.update({name: pin(A.DRIVER_ROOT / name, digest)
                    for name, digest in A.DRIVER_SOURCE_PINS.items()})
    sources['run_cpu.py'] = pin(A.DRIVER_ROOT / 'run_cpu.py', A.DRIVER_CONTROLLER_SHA)
    sources['supervisor.py'] = pin(A.DRIVER_ROOT / 'supervisor.py', A.DRIVER_SUPERVISOR_SHA)
    inputs = dict(schema='ferric-guarded-mlp-driver-cpu-input-v1',
                  source_generation=A.DRIVER_GENERATION, tool_pins={},
                  files={name: A.compact(row) for name, row in sources.items()})
    package = A.DRIVER_ROOT / 'fe2o3/crates/cargo-fe2o3'
    artifacts, streams = {}, []
    for role, test in (('driver', False), ('test', True)):
        path = A.DRIVER_ROOT / ('target/debug/deps/cargo_fe2o3-fixture' if test
                                else 'target/debug/cargo-fe2o3')
        row = dict(reason='compiler-artifact', manifest_path=str(package / 'Cargo.toml'),
                   target=dict(name='cargo-fe2o3', kind=['bin'], crate_types=['bin'],
                               src_path=str(package / 'src/main.rs')),
                   profile=dict(test=test), executable=str(path), filenames=[str(path)])
        artifacts[role] = dict(pin=pin(path), cargo_artifact=row)
        streams.append([row, dict(reason='build-finished', success=True)])
    names = sorted([*A.DRIVER_ENGINEERING, *A.DRIVER_IGNORED, 'other::ordinary'])
    value = dict(schema='ferric-guarded-mlp-driver-cpu-v1', passed=True, failure=None,
                 postcheck_errors=[], source_unchanged=True,
                 source_generation=A.DRIVER_GENERATION, controller=sources['run_cpu.py'],
                 supervisor=sources['supervisor.py'], tool_pins={},
                 base_cpu_complete=pin('/synthetic/base-complete.json', A.DRIVER_BASE_CPU_SHA),
                 base_cpu_sources=pin('/synthetic/base-sources.json', A.DRIVER_BASE_SOURCES_SHA),
                 final_artifact_phase='driver-build', test_artifact_phase='driver-tests-build',
                 phases=[dict(label=label, exit_code=0, natural_exit=True, reaped=True,
                              process_group_absent=True, forced_cleanup=False, timed_out=False,
                              exception=None, storage_failure=None) for label in A.DRIVER_PHASES],
                 artifacts=artifacts, inventory=names, ignored=list(A.DRIVER_IGNORED),
                 engineering_tests=list(A.DRIVER_ENGINEERING),
                 test_scope='engineering_hsaco::tests::', full_bin_tests_executed=False,
                 tests={'driver': dict(names=list(A.DRIVER_ENGINEERING), passed=18,
                                       failed=0, ignored=0, filtered_out=len(names)-18)})
    previous = fixture()[4]
    manifest = copy.deepcopy(previous)
    product = artifacts['driver']['pin']
    manifest['bin/cargo-fe2o3'] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, *streams, manifest, previous


class DriverAdmissionTests(unittest.TestCase):
    def test_only_new_driver_and_test_metadata_are_admitted(self):
        args = driver_fixture()
        proof = A.driver_contract(*args)
        self.assertEqual(proof['driver'], args[0]['artifacts']['driver'])
        self.assertEqual(proof['test_elf_cpu_provenance'], args[0]['artifacts']['test'])
        self.assertIs(proof['omitted_test_elf_body_rehashed'], False)

    def test_failed_unreviewed_or_wrong_source_generation_refused(self):
        for key, bad in (('passed', False), ('failure', 'failed'), ('postcheck_errors', ['drift']),
                         ('source_unchanged', False), ('source_generation', '0' * 40)):
            args = driver_fixture()
            args[0][key] = bad
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                A.driver_contract(*args)
        for key in ('controller', 'supervisor', 'base_cpu_complete', 'base_cpu_sources'):
            args = driver_fixture()
            args[0][key]['sha256'] = '9' * 64
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                A.driver_contract(*args)

    def test_missing_changed_or_relocated_source_map_refused(self):
        for change in ('missing', 'body', 'path', 'input'):
            args = driver_fixture()
            name = next(iter(A.DRIVER_SOURCE_PINS))
            if change == 'missing':
                del args[2][name]
            elif change == 'body':
                args[2][name]['sha256'] = '9' * 64
                args[1]['files'][name]['sha256'] = '9' * 64
            elif change == 'path':
                args[2][name]['path'] = '/synthetic/outside.rs'
            else:
                args[1]['source_generation'] = '0' * 40
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.driver_contract(*args)

    def test_phase_cleanup_and_wrong_final_build_refused(self):
        for change in ('missing', 'reaped', 'cleanup', 'phase', 'build', 'test-build'):
            args = driver_fixture()
            if change == 'missing':
                args[0]['phases'].pop()
            elif change == 'reaped':
                args[0]['phases'][0]['reaped'] = False
            elif change == 'cleanup':
                args[0]['phases'][0]['forced_cleanup'] = True
            elif change == 'phase':
                args[0]['final_artifact_phase'] = 'driver-tests-build'
            else:
                args[3 if change == 'build' else 4][-1]['success'] = False
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.driver_contract(*args)

    def test_exact_focused_census_and_no_full_suite_claim(self):
        for change in ('full', 'name', 'filter', 'ignore', 'failure', 'extra-engineering'):
            args = driver_fixture()
            value = args[0]
            if change == 'full':
                value['full_bin_tests_executed'] = True
            elif change == 'name':
                value['tests']['driver']['names'].pop()
            elif change == 'filter':
                value['tests']['driver']['filtered_out'] = 0
            elif change == 'ignore':
                value['ignored'].pop()
            elif change == 'failure':
                value['tests']['driver']['failed'] = 1
            else:
                value['inventory'] = sorted([*value['inventory'], 'engineering_hsaco::tests::extra'])
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.driver_contract(*args)

    def test_wrong_driver_or_test_cargo_role_refused(self):
        for role in ('driver', 'test'):
            for change in ('profile', 'path', 'source', 'duplicate'):
                args = driver_fixture()
                artifact = args[0]['artifacts'][role]
                record = artifact['cargo_artifact']
                if change == 'profile':
                    record['profile']['test'] = role != 'test'
                elif change == 'path':
                    artifact['pin']['path'] = '/synthetic/wrong-ELF'
                    record['executable'] = artifact['pin']['path']
                    record['filenames'] = [artifact['pin']['path']]
                elif change == 'source':
                    record['target']['src_path'] = '/synthetic/wrong.rs'
                else:
                    args[3 if role == 'driver' else 4].insert(0, copy.deepcopy(record))
                with self.subTest(role=role, change=change), self.assertRaises(RuntimeError):
                    A.driver_contract(*args)

    def test_each_unchanged_tool_and_old_driver_are_refused(self):
        for name in A.NAMES:
            args = driver_fixture()
            if name == 'cargo-fe2o3':
                args[5]['bin/' + name] = args[6]['bin/' + name]
            else:
                args[5]['bin/' + name]['sha256'] = '9' * 64
            with self.subTest(name=name), self.assertRaises(RuntimeError):
                A.driver_contract(*args)

    def test_pending_driver_binding_refuses_before_host_or_output_effects(self):
        with patch.object(A, 'DRIVER_CONTROLLER_SHA', None), \
                patch.object(A, 'MEMORY_BOUNDS_DAG_COMPLETE_SHA', '1' * 64), \
             patch.object(A.sys, 'argv', ['audit.py', '1' * 64]), \
             patch.object(A.os, 'getuid', side_effect=AssertionError('host/effect gate reached')):
            with self.assertRaisesRegex(RuntimeError, 'driver producer controller binding is pending'):
                A.main()



def cap_baseline_fixture():
    base, _, base_sources, _, _, previous = fixture()
    existing = (A.CFG_PATH + 'production_ranked_projection_v1.rs',
                A.CFG_PATH + 'production_ranked_projection_v1/loop_switch_domain_v1.rs',
                A.CFG_PATH + 'production_ranked_projection_v1/analysis_multi_split_v1_tests.rs',
                A.CFG_PATH + 'production_ranked_projection_v1/projection_08_tests.rs',
                'fe2o3/crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds.rs',
                'fe2o3/crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds/resource_tests.rs')
    for index, name in enumerate(existing):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.CPU_ROOT / name))
    base['test_inventories'] = {}
    base['ignored_inventories'] = {}
    for short, count, ignored in (('compiler', 1263, 24), ('pliron', 1505, 1)):
        names = [short + '-name-' + str(i).zfill(4) for i in range(count)]
        ignored_names = names[-ignored:]
        base['test_inventories'][short + '-lib'] = names
        base['ignored_inventories'][short + '-lib'] = ignored_names
        base['tests'][short].update(
            names=names, ignored_names=ignored_names, filtered_out=0,
            named_outcomes={name: 'ignored' if name in ignored_names else 'ok'
                            for name in names})
    base['compiler_cohorts'] = {'scope-' + str(i): {} for i in range(5)}
    for label, row in base['tests'].items():
        if label not in ('compiler', 'pliron'):
            row.update(names=[label + '-name-' + str(i) for i in range(row['passed'])],
                       filtered_out=1263-row['passed'] if label in base['compiler_cohorts'] else 0)
    base['phases'][20]['label'] = 'atomic-extraction-0'
    overlay = {}
    for name in sorted((*existing,
            A.CFG_PATH + 'production_ranked_projection_v1/cfg_block_limit_diagnostic_v1.rs',
            A.CFG_PATH + 'production_ranked_projection_v1/cfg_block_limit_diagnostic_v1_tests.rs')):
        before = A.compact(base_sources[name]) if name in base_sources else None
        overlay[name] = dict(before=before, after=dict(bytes=11, sha256='7' * 64))
    cohorts = {'cfg-block-limit-diagnostic': dict(role='compiler-lib', prefix=A.CFG_PREFIX,
                                               names=list(A.CFG_NAMES)), **A.CAPACITY_COHORTS}
    proposal = dict(
        schema='ferric-guarded-mlp-ranked-cfg-cap-source-v1',
        base_complete=dict(bytes=10, sha256=A.BASE_COMPLETE_SHA),
        base_sources=dict(bytes=10, sha256=A.BASE_SOURCES_SHA),
        admission_changed=True, structural_capacity_expansion=True,
        cfg_diagnostics_retained=True, runtime_or_kernel_source_changes=False,
        block_limit=dict(before=1024, after=2048),
        preserved_limits=dict(facts=1024, edges=2048, operations=65536, findings=4096,
                              projector_graph_work=3145728, bounds_work=8388608,
                              bounds_storage_items=131072),
        cohorts={label: dict(filter=row['prefix'], names=row['names'])
                 for label, row in cohorts.items()},
        files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items()
                if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.DAG_BASE_ROOT / name), **row)
               for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.DAG_BASE_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.DAG_BASE_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(
        path=str(A.DAG_BASE_ROOT / 'qualification_helpers.py'), bytes=10,
        sha256=A.CPU_HELPER_SHA)
    lineage = {key: {} for key in ('base_input', 'base_metadata',
                                  'base_dependencies', 'base_streams')}
    lineage.update(
        base_complete=dict(path='/synthetic/base-complete', **proposal['base_complete']),
        base_sources=dict(path='/synthetic/base-sources', **proposal['base_sources']),
        capacity_proposal=dict(path='/synthetic/proposal', bytes=10,
                               sha256='71915fa85ce1ff46c5dd4c1bb96e75f6b27505616923bfec4d657782b46a5d72'),
        capacity_overlay=overlay)
    inputs = dict(schema='ferric-guarded-mlp-ranked-cfg-cap-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()})
    with patch.object(A, 'CPU_ROOT', A.DAG_BASE_ROOT):
        value, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-ranked-cfg-cap-cpu-v1',
                 diagnostic_build=False, admission_changed=True,
                 structural_capacity_expansion=True, cfg_diagnostics_retained=True,
                 capacity_limits=copy.deepcopy(A.CAPACITY_LIMITS),
                 capacity_cohorts=copy.deepcopy(A.CAPACITY_COHORTS),
                 cfg_diagnostic_tests=list(A.CFG_NAMES), source_lineage=lineage,
                 compiler_cohorts=copy.deepcopy(base['compiler_cohorts']),
                 controller=sources['run_cpu.py'], helper=sources['qualification_helpers.py'],
                 phases=copy.deepcopy(base['phases']), tests=copy.deepcopy(base['tests']),
                 test_inventories=copy.deepcopy(base['test_inventories']),
                 ignored_inventories=copy.deepcopy(base['ignored_inventories']),
                 tests_passed=2822)
    value['phases'][20:20] = [dict(value['phases'][0], label=label) for label in cohorts]
    for role in ('compiler-lib', 'pliron-lib'):
        additions = [name for row in cohorts.values() if row['role'] == role for name in row['names']]
        inventory = sorted(base['test_inventories'][role] + additions)
        value['test_inventories'][role] = inventory
        short = role.removesuffix('-lib')
        value['tests'][short]['names'] = inventory
        value['tests'][short]['named_outcomes'].update({name: 'ok' for name in additions})
        value['tests'][short]['passed'] += len(additions)
    for label in base['compiler_cohorts']:
        value['tests'][label]['filtered_out'] += 9
    for label, row in cohorts.items():
        value['tests'][label] = dict(names=row['names'], passed=len(row['names']), failed=0,
                                    ignored=0, filtered_out=len(value['test_inventories'][row['role']])-len(row['names']))
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, records, manifest, previous, base, base_sources, proposal


def dag_fixture():
    base, _, base_sources, _, _, previous, *_ = cap_baseline_fixture()
    atomic = A.CFG_PATH + 'production_ranked_projection_v1/indexed_atomic_v1.rs'
    row = base_sources.pop('fe2o3/file-6')
    base_sources[atomic] = dict(row, path=str(A.DAG_BASE_ROOT / atomic))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=12, sha256='8' * 64)) for name in A.DAG_FILES}
    proposal = dict(schema='ferric-guarded-mlp-indexed-atomic-dag-source-v1', source_only=True,
                    semantic_predicates_changed=False, resource_admission_may_change=True,
                    actual_failure_caller_identified=False, block_limit=2048, edge_limit=2048,
                    graph_work_limit=3145728, filter=A.DAG_PREFIX, test_names=list(A.DAG_NAMES),
                    base_complete=dict(bytes=10, sha256=A.DAG_BASE_COMPLETE_SHA),
                    base_sources=dict(bytes=10, sha256=A.DAG_BASE_SOURCES_SHA),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.DAG_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.DAG_ROOT / 'run_cpu.py'), bytes=10, sha256=A.DAG_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.DAG_ROOT / 'qualification_helpers.py'),
                                               bytes=10, sha256=A.CPU_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/cap-complete', **proposal['base_complete']),
                   base_sources=dict(path='/synthetic/cap-sources', **proposal['base_sources']),
                   base_input=dict(path='/synthetic/cap-input', bytes=10, sha256=A.DAG_BASE_INPUT_SHA),
                   dag_proposal=dict(path='/synthetic/dag-proposal', bytes=10, sha256=A.DAG_PROPOSAL_SHA),
                   dag_overlay=overlay)
    inputs = dict(schema='ferric-guarded-mlp-indexed-atomic-dag-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    with patch.object(A, 'CPU_ROOT', A.DAG_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-indexed-atomic-dag-cpu-v1',
                 structural_capacity_expansion=False, inherited_capacity_expansion=True,
                 structural_limits_changed=False, graph_analysis_optimization=True,
                 resource_admission_may_change=True, semantic_predicates_changed=False,
                 actual_failure_caller_identified=False, dag_test_filter=A.DAG_PREFIX,
                 dag_tests=list(A.DAG_NAMES), controller=sources['run_cpu.py'],
                 helper=sources['qualification_helpers.py'], source_lineage=lineage,
                 artifacts=products['artifacts'], tests_passed=2840)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='indexed-atomic-dag-certificate'))
    inventory = sorted(value['test_inventories']['compiler-lib'] + list(A.DAG_NAMES))
    value['test_inventories']['compiler-lib'] = inventory
    value['tests']['compiler']['names'] = inventory
    value['tests']['compiler']['named_outcomes'].update({name: 'ok' for name in A.DAG_NAMES})
    value['tests']['compiler']['passed'] += 9
    for label in (*base['compiler_cohorts'], 'cfg-block-limit-diagnostic', 'cfg-block-cap-compiler'):
        value['tests'][label]['filtered_out'] += 9
    value['tests']['indexed-atomic-dag-certificate'] = dict(
        names=list(A.DAG_NAMES), passed=9, failed=0, ignored=0, filtered_out=len(inventory)-9)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, records, manifest, previous, base, base_sources, proposal


def diagnostic_fixture():
    base, _, base_sources, _, _, previous, *_ = dag_fixture()
    for index, name in enumerate(sorted(name for name, new in A.GRAPH_WORK_FILES.items()
                                        if not new and name not in base_sources), start=7):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.DAG_ROOT / name))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=13, sha256='9' * 64)) for name in A.GRAPH_WORK_FILES}
    proposal = dict(schema='ferric-guarded-mlp-graph-work-diagnostic-source-v1', source_only=True,
                    diagnostic_only=True, semantic_predicates_changed=False,
                    resource_admission_changed=False, graph_work_limit=3145728,
                    raw_path_bytes=128, escaped_path_bytes=512,
                    dag_proposal=A.compact(base['source_lineage']['dag_proposal']),
                    test_filter=A.GRAPH_WORK_PREFIX, test_names=list(A.GRAPH_WORK_NAMES),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.DIAGNOSTIC_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.DIAGNOSTIC_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.DIAGNOSTIC_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.DIAGNOSTIC_ROOT / 'qualification_helpers.py'),
                                               bytes=10, sha256=A.CPU_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/dag-complete', bytes=10, sha256=A.DAG_COMPLETE_SHA),
                   base_sources=dict(path='/synthetic/dag-sources', bytes=10, sha256=A.DAG_SOURCES_SHA),
                   base_input=dict(path='/synthetic/dag-input', bytes=10, sha256=A.DAG_INPUT_SHA),
                   graph_work_proposal=dict(path='/synthetic/diagnostic-proposal', bytes=10,
                                            sha256=A.GRAPH_WORK_PROPOSAL_SHA),
                   graph_work_overlay=overlay)
    inputs = dict(schema='ferric-guarded-mlp-ranked-graph-work-diagnostic-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    with patch.object(A, 'CPU_ROOT', A.DIAGNOSTIC_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-ranked-graph-work-diagnostic-cpu-v1',
                 diagnostic_build=True, diagnostic_only=True, admission_changed=False,
                 graph_analysis_optimization=False, inherited_graph_analysis_optimization=True,
                 resource_admission_may_change=False, resource_admission_changed=False,
                 graph_work_diagnostics=True, graph_work_test_filter=A.GRAPH_WORK_PREFIX,
                 graph_work_tests=list(A.GRAPH_WORK_NAMES), controller=sources['run_cpu.py'],
                 helper=sources['qualification_helpers.py'], source_lineage=lineage,
                 artifacts=products['artifacts'], tests_passed=2858)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='graph-work-diagnostic'))
    inventory = sorted(value['test_inventories']['compiler-lib'] + list(A.GRAPH_WORK_NAMES))
    value['test_inventories']['compiler-lib'] = inventory
    value['tests']['compiler']['names'] = inventory
    value['tests']['compiler']['named_outcomes'].update({name: 'ok' for name in A.GRAPH_WORK_NAMES})
    value['tests']['compiler']['passed'] += 9
    for label in (*base['compiler_cohorts'], 'cfg-block-limit-diagnostic',
                  'cfg-block-cap-compiler', 'indexed-atomic-dag-certificate'):
        value['tests'][label]['filtered_out'] += 9
    value['tests']['graph-work-diagnostic'] = dict(
        names=list(A.GRAPH_WORK_NAMES), passed=9, failed=0, ignored=0, filtered_out=len(inventory)-9)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, records, manifest, previous, base, base_sources, proposal


def membership_fixture():
    base, _, base_sources, _, _, previous, *_ = diagnostic_fixture()
    for index, name in enumerate(sorted(name for name, new in A.MEMBERSHIP_FILES.items()
                                        if not new and name not in base_sources), start=19):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.DIAGNOSTIC_ROOT / name))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=14, sha256='a' * 64)) for name in A.MEMBERSHIP_FILES}
    proposal = dict(schema='ferric-guarded-mlp-indexed-atomic-membership-source-v1', source_only=True,
                    semantic_predicates_changed=False, structural_limits_changed=False,
                    resource_admission_may_change=True, graph_analysis_optimization=True,
                    membership_lookup_optimization=True, inherited_graph_work_diagnostics=True,
                    actual_failure_caller_identified=True,
                    actual_failure=dict(bytes=9723, sha256=
                        '7727462e3e8f3744e22926546da24ec9eec2e55a6f7b9c3fcf6036168ae0a52e'),
                    actual_failure_stderr=dict(bytes=8533, sha256=
                        'a52d57b5b340b8b7223d35bfaa441dcf88c0febc74bc3924925647cf1d91e99c'),
                    base_complete=dict(bytes=10, sha256=A.DIAGNOSTIC_COMPLETE_SHA),
                    base_sources=dict(bytes=10, sha256=A.DIAGNOSTIC_SOURCES_SHA),
                    base_input=dict(bytes=10, sha256=A.DIAGNOSTIC_INPUT_SHA),
                    base_controller=A.compact(base['controller']),
                    graph_work_limit=3145728, block_limit=2048, edge_limit=2048,
                    filter=A.MEMBERSHIP_PREFIX, test_names=list(A.MEMBERSHIP_NAMES),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.MEMBERSHIP_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.MEMBERSHIP_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.MEMBERSHIP_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.MEMBERSHIP_ROOT / 'qualification_helpers.py'),
                                               bytes=10, sha256=A.CPU_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/diagnostic-complete', **proposal['base_complete']),
                   base_sources=dict(path='/synthetic/diagnostic-sources', **proposal['base_sources']),
                   base_input=dict(path='/synthetic/diagnostic-input', **proposal['base_input']),
                   membership_proposal=dict(path='/synthetic/membership-proposal', bytes=10,
                                            sha256=A.MEMBERSHIP_PROPOSAL_SHA),
                   membership_overlay=overlay)
    inputs = dict(schema='ferric-guarded-mlp-indexed-atomic-membership-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    value.pop('resource_admission_changed', None)
    with patch.object(A, 'CPU_ROOT', A.MEMBERSHIP_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-indexed-atomic-membership-cpu-v1',
                 diagnostic_build=False, diagnostic_only=False, admission_changed=True,
                 graph_analysis_optimization=True, inherited_graph_analysis_optimization=True,
                 resource_admission_may_change=True, membership_lookup_optimization=True,
                 baseline_failure_caller_identified=True, inherited_graph_work_diagnostics=True,
                 membership_test_filter=A.MEMBERSHIP_PREFIX, membership_tests=list(A.MEMBERSHIP_NAMES),
                 controller=sources['run_cpu.py'], helper=sources['qualification_helpers.py'],
                 source_lineage=lineage, artifacts=products['artifacts'], tests_passed=2876)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='indexed-atomic-membership'))
    inventory = sorted(value['test_inventories']['compiler-lib'] + list(A.MEMBERSHIP_NAMES))
    value['test_inventories']['compiler-lib'] = inventory
    value['tests']['compiler']['names'] = inventory
    value['tests']['compiler']['named_outcomes'].update({name: 'ok' for name in A.MEMBERSHIP_NAMES})
    value['tests']['compiler']['passed'] += 9
    for label in (*base['compiler_cohorts'], 'cfg-block-limit-diagnostic',
                  'cfg-block-cap-compiler', 'indexed-atomic-dag-certificate', 'graph-work-diagnostic'):
        value['tests'][label]['filtered_out'] += 9
    value['tests']['indexed-atomic-membership'] = dict(
        names=list(A.MEMBERSHIP_NAMES), passed=9, failed=0, ignored=0, filtered_out=len(inventory)-9)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, records, manifest, previous, base, base_sources, proposal


def census_fixture():
    base, _, base_sources, _, _, previous, *_ = membership_fixture()
    for index, name in enumerate(sorted(name for name, new in A.CENSUS_FILES.items()
                                        if not new and name not in base_sources), start=20):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.MEMBERSHIP_ROOT / name))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=15, sha256='b' * 64)) for name in A.CENSUS_FILES}
    assert all(row['before'] != row['after'] for row in overlay.values())
    proposal = dict(schema='ferric-guarded-mlp-indexed-atomic-dead-cast-census-source-v1', source_only=True,
                    semantic_predicates_changed=False, structural_limits_changed=False,
                    resource_admission_may_change=True, graph_analysis_optimization=True,
                    dead_cast_census_optimization=True, compiler_scratch_added=True,
                    scratch_entry_limit=262144, inherited_membership_lookup_optimization=True,
                    inherited_graph_work_diagnostics=True,
                    actual_failure_caller_identified=True,
                    actual_failure=dict(bytes=10856, sha256=
                        '914ba56097474530279206963c2a08268026cfc3ea389cce30a81157e653dfde'),
                    actual_failure_stderr=dict(bytes=8519, sha256=
                        '7dbc29c02010d6d2a34a20992381de9faccff678d5540aa0c1a2f5609e88cb6e'),
                    base_complete=dict(bytes=10, sha256=A.MEMBERSHIP_COMPLETE_SHA),
                    base_sources=dict(bytes=10, sha256=A.MEMBERSHIP_SOURCES_SHA),
                    base_input=dict(bytes=10, sha256=A.MEMBERSHIP_INPUT_SHA),
                    base_controller=A.compact(base['controller']),
                    graph_work_limit=3145728, block_limit=2048, edge_limit=2048,
                    filter=A.CENSUS_PREFIX, test_names=list(A.CENSUS_NAMES),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.CENSUS_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.CENSUS_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.CENSUS_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.CENSUS_ROOT / 'qualification_helpers.py'),
                                               bytes=10, sha256=A.CPU_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/membership-complete', **proposal['base_complete']),
                   base_sources=dict(path='/synthetic/membership-sources', **proposal['base_sources']),
                   base_input=dict(path='/synthetic/membership-input', **proposal['base_input']),
                   census_proposal=dict(path='/synthetic/census-proposal', bytes=10,
                                            sha256=A.CENSUS_PROPOSAL_SHA),
                   census_overlay=overlay)
    inputs = dict(schema='ferric-guarded-mlp-indexed-atomic-dead-cast-census-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    value.pop('resource_admission_changed', None)
    with patch.object(A, 'CPU_ROOT', A.CENSUS_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-indexed-atomic-dead-cast-census-cpu-v1',
                 diagnostic_build=False, diagnostic_only=False, admission_changed=True,
                 graph_analysis_optimization=True, inherited_graph_analysis_optimization=True,
                 resource_admission_may_change=True, membership_lookup_optimization=True,
                 inherited_membership_lookup_optimization=True,
                 dead_cast_census_optimization=True, compiler_scratch_added=True,
                 baseline_failure_caller_identified=True, inherited_graph_work_diagnostics=True,
                 census_test_filter=A.CENSUS_PREFIX, census_tests=list(A.CENSUS_NAMES),
                 controller=sources['run_cpu.py'], helper=sources['qualification_helpers.py'],
                 source_lineage=lineage, artifacts=products['artifacts'], tests_passed=2894)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='indexed-atomic-dead-cast-census'))
    inventory = sorted(value['test_inventories']['compiler-lib'] + list(A.CENSUS_NAMES))
    value['test_inventories']['compiler-lib'] = inventory
    value['tests']['compiler']['names'] = inventory
    value['tests']['compiler']['named_outcomes'].update({name: 'ok' for name in A.CENSUS_NAMES})
    value['tests']['compiler']['passed'] += 9
    for label in (*base['compiler_cohorts'], 'cfg-block-limit-diagnostic',
                  'cfg-block-cap-compiler', 'indexed-atomic-dag-certificate', 'graph-work-diagnostic',
                  'indexed-atomic-membership'):
        value['tests'][label]['filtered_out'] += 9
    value['tests']['indexed-atomic-dead-cast-census'] = dict(
        names=list(A.CENSUS_NAMES), passed=9, failed=0, ignored=0, filtered_out=len(inventory)-9)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, records, manifest, previous, base, base_sources, proposal


def use_lookup_fixture():
    base, _, base_sources, _, _, previous, *_ = census_fixture()
    for index, name in enumerate(sorted(name for name, new in A.USE_LOOKUP_FILES.items()
                                        if not new and name not in base_sources), start=22):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.CENSUS_ROOT / name))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=16, sha256='c' * 64)) for name in A.USE_LOOKUP_FILES}
    assert all(row['before'] != row['after'] for row in overlay.values())
    proposal = dict(schema='ferric-guarded-mlp-indexed-atomic-use-lookup-source-v1', source_only=True,
                    semantic_predicates_changed=False, structural_limits_changed=False,
                    resource_admission_may_change=True, graph_analysis_optimization=True,
                    use_lookup_optimization=True, compiler_scratch_added=False,
                    inherited_dead_cast_census_optimization=True, inherited_membership_lookup_optimization=True,
                    inherited_graph_work_diagnostics=True,
                    actual_failure_caller_identified=True,
                    actual_failure=dict(bytes=12321, sha256=
                        '34699054a75a7300bd308d99e22635c25237c3d85ecb8f15a1ca9de00c5831fd'),
                    actual_failure_stderr=dict(bytes=8554, sha256=
                        'ba24844fd07b9b9a7bc3d5934ff29b297a85ed965a3871a21b917fda5da0c044'),
                    base_complete=dict(bytes=10, sha256=A.CENSUS_COMPLETE_SHA),
                    base_sources=dict(bytes=10, sha256=A.CENSUS_SOURCES_SHA),
                    base_input=dict(bytes=10, sha256=A.CENSUS_INPUT_SHA),
                    base_controller=A.compact(base['controller']),
                    graph_work_limit=3145728, block_limit=2048, edge_limit=2048,
                    filter=A.USE_LOOKUP_PREFIX, test_names=list(A.USE_LOOKUP_NAMES),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.USE_LOOKUP_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.USE_LOOKUP_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.USE_LOOKUP_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.USE_LOOKUP_ROOT / 'qualification_helpers.py'),
                                               bytes=10, sha256=A.CPU_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/census-complete', **proposal['base_complete']),
                   base_sources=dict(path='/synthetic/census-sources', **proposal['base_sources']),
                   base_input=dict(path='/synthetic/census-input', **proposal['base_input']),
                   use_lookup_proposal=dict(path='/synthetic/use-lookup-proposal', bytes=10,
                                            sha256=A.USE_LOOKUP_PROPOSAL_SHA),
                   use_lookup_overlay=overlay)
    inputs = dict(schema='ferric-guarded-mlp-indexed-atomic-use-lookup-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    value.pop('resource_admission_changed', None)
    with patch.object(A, 'CPU_ROOT', A.USE_LOOKUP_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-indexed-atomic-use-lookup-cpu-v1',
                 diagnostic_build=False, diagnostic_only=False, admission_changed=True,
                 graph_analysis_optimization=True, inherited_graph_analysis_optimization=True,
                 resource_admission_may_change=True, membership_lookup_optimization=True,
                 inherited_membership_lookup_optimization=True,
                 dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                 use_lookup_optimization=True, compiler_scratch_added=False,
                 baseline_failure_caller_identified=True, inherited_graph_work_diagnostics=True,
                 use_lookup_test_filter=A.USE_LOOKUP_PREFIX, use_lookup_tests=list(A.USE_LOOKUP_NAMES),
                 controller=sources['run_cpu.py'], helper=sources['qualification_helpers.py'],
                 source_lineage=lineage, artifacts=products['artifacts'], tests_passed=2912)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='indexed-atomic-use-lookup'))
    inventory = sorted(value['test_inventories']['compiler-lib'] + list(A.USE_LOOKUP_NAMES))
    value['test_inventories']['compiler-lib'] = inventory
    value['tests']['compiler']['names'] = inventory
    value['tests']['compiler']['named_outcomes'].update({name: 'ok' for name in A.USE_LOOKUP_NAMES})
    value['tests']['compiler']['passed'] += 9
    for label in (*base['compiler_cohorts'], 'cfg-block-limit-diagnostic',
                  'cfg-block-cap-compiler', 'indexed-atomic-dag-certificate', 'graph-work-diagnostic',
                  'indexed-atomic-membership', 'indexed-atomic-dead-cast-census'):
        value['tests'][label]['filtered_out'] += 9
    value['tests']['indexed-atomic-use-lookup'] = dict(
        names=list(A.USE_LOOKUP_NAMES), passed=9, failed=0, ignored=0, filtered_out=len(inventory)-9)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, records, manifest, previous, base, base_sources, proposal


def borrow_lookup_fixture():
    base, _, base_sources, _, _, previous, *_ = use_lookup_fixture()
    for index, name in enumerate(sorted(name for name, new in A.BORROW_LOOKUP_FILES.items()
                                        if not new and name not in base_sources), start=24):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.USE_LOOKUP_ROOT / name))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=17, sha256='d' * 64)) for name in A.BORROW_LOOKUP_FILES}
    assert all(row['before'] != row['after'] for row in overlay.values())
    proposal = dict(schema='ferric-guarded-mlp-indexed-atomic-borrow-lookup-source-v1', source_only=True,
                    semantic_predicates_changed=False, structural_limits_changed=False,
                    resource_admission_may_change=True, graph_analysis_optimization=True,
                    borrow_lookup_optimization=True, compiler_scratch_added=False,
                    inherited_use_lookup_optimization=True,
                    inherited_dead_cast_census_optimization=True, inherited_membership_lookup_optimization=True,
                    inherited_graph_work_diagnostics=True,
                    actual_failure_caller_identified=True,
                    actual_failure=dict(bytes=13412, sha256=
                        '5a9b97db2af7e9e6f152aa84f632112abb587a27f3ffb6dca14920b0657a53df'),
                    actual_failure_stderr=dict(bytes=8512, sha256=
                        'f47c6cd5b7bbd4d9238212cd6b570dfd6e7b2898d9518e5af435f110f4db1160'),
                    base_complete=dict(bytes=10, sha256=A.USE_LOOKUP_COMPLETE_SHA),
                    base_sources=dict(bytes=10, sha256=A.USE_LOOKUP_SOURCES_SHA),
                    base_input=dict(bytes=10, sha256=A.USE_LOOKUP_INPUT_SHA),
                    base_controller=A.compact(base['controller']),
                    graph_work_limit=3145728, block_limit=2048, edge_limit=2048,
                    filter=A.BORROW_LOOKUP_PREFIX, test_names=list(A.BORROW_LOOKUP_NAMES),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.BORROW_LOOKUP_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.BORROW_LOOKUP_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.BORROW_LOOKUP_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.BORROW_LOOKUP_ROOT / 'qualification_helpers.py'),
                                               bytes=10, sha256=A.CPU_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/use-lookup-complete', **proposal['base_complete']),
                   base_sources=dict(path='/synthetic/use-lookup-sources', **proposal['base_sources']),
                   base_input=dict(path='/synthetic/use-lookup-input', **proposal['base_input']),
                   borrow_lookup_proposal=dict(path='/synthetic/borrow-lookup-proposal', bytes=10,
                                            sha256=A.BORROW_LOOKUP_PROPOSAL_SHA),
                   borrow_lookup_overlay=overlay)
    inputs = dict(schema='ferric-guarded-mlp-indexed-atomic-borrow-lookup-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    value.pop('resource_admission_changed', None)
    with patch.object(A, 'CPU_ROOT', A.BORROW_LOOKUP_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-indexed-atomic-borrow-lookup-cpu-v1',
                 diagnostic_build=False, diagnostic_only=False, admission_changed=True,
                 graph_analysis_optimization=True, inherited_graph_analysis_optimization=True,
                 resource_admission_may_change=True, membership_lookup_optimization=True,
                 inherited_membership_lookup_optimization=True,
                 dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                 borrow_lookup_optimization=True, compiler_scratch_added=False,
                 inherited_use_lookup_optimization=True,
                 baseline_failure_caller_identified=True, inherited_graph_work_diagnostics=True,
                 borrow_lookup_test_filter=A.BORROW_LOOKUP_PREFIX, borrow_lookup_tests=list(A.BORROW_LOOKUP_NAMES),
                 controller=sources['run_cpu.py'], helper=sources['qualification_helpers.py'],
                 source_lineage=lineage, artifacts=products['artifacts'], tests_passed=2930)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='indexed-atomic-borrow-lookup'))
    inventory = sorted(value['test_inventories']['compiler-lib'] + list(A.BORROW_LOOKUP_NAMES))
    value['test_inventories']['compiler-lib'] = inventory
    value['tests']['compiler']['names'] = inventory
    value['tests']['compiler']['named_outcomes'].update({name: 'ok' for name in A.BORROW_LOOKUP_NAMES})
    value['tests']['compiler']['passed'] += 9
    for label in (*base['compiler_cohorts'], 'cfg-block-limit-diagnostic',
                  'cfg-block-cap-compiler', 'indexed-atomic-dag-certificate', 'graph-work-diagnostic',
                  'indexed-atomic-membership', 'indexed-atomic-dead-cast-census', 'indexed-atomic-use-lookup'):
        value['tests'][label]['filtered_out'] += 9
    value['tests']['indexed-atomic-borrow-lookup'] = dict(
        names=list(A.BORROW_LOOKUP_NAMES), passed=9, failed=0, ignored=0, filtered_out=len(inventory)-9)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, records, manifest, previous, base, base_sources, proposal


def cfg_expansion_fixture():
    base, _, base_sources, _, _, previous, *_ = borrow_lookup_fixture()
    for index, name in enumerate(sorted(name for name, new in A.CFG_EXPANSION_FILES.items()
                                        if not new and name not in base_sources), start=24):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.BORROW_LOOKUP_ROOT / name))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=18, sha256='e' * 64)) for name in A.CFG_EXPANSION_FILES}
    assert all(row['before'] != row['after'] for row in overlay.values())
    proposal = dict(schema='ferric-guarded-mlp-ranked-cfg-expansion-diagnostic-source-v1', source_only=True,
                    diagnostic_only=True, cfg_expansion_diagnostics=True, diagnostic_changes_admission=False,
                    semantic_predicates_changed=False, structural_limits_changed=False,
                    resource_admission_may_change=False, static_failure_site_identified=True,
                    actual_failure_caller_identified=False, actual_failure_block_count_observed=False,
                    base_source_count=5803, source_count=5803, source_file_additions=0, new_test_count=3,
                    actual_failure=dict(bytes=14686, sha256=
                        'cb1def314b36717465478dbf88d521f094749b34d805b9f0e07ed5f3c5421778'),
                    actual_failure_stderr=dict(bytes=8018, sha256=
                        '646b7e35b6dbf8cd8b00e43f52008824ccc06634d636238fbc23fc33d9ccdb83'),
                    base_complete=dict(bytes=10, sha256=A.BORROW_LOOKUP_COMPLETE_SHA),
                    base_sources=dict(bytes=10, sha256=A.BORROW_LOOKUP_SOURCES_SHA),
                    base_input=dict(bytes=10, sha256=A.BORROW_LOOKUP_INPUT_SHA),
                    base_controller=A.compact(base['controller']),
                    graph_work_limit=3145728, block_limit=2048, edge_limit=2048,
                    filter=A.CFG_EXPANSION_PREFIX, test_names=list(A.CFG_EXPANSION_NAMES),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.CFG_EXPANSION_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.CFG_EXPANSION_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.CFG_EXPANSION_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.CFG_EXPANSION_ROOT / 'qualification_helpers.py'),
                                               bytes=10, sha256=A.CPU_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/borrow-lookup-complete', **proposal['base_complete']),
                   base_sources=dict(path='/synthetic/borrow-lookup-sources', **proposal['base_sources']),
                   base_input=dict(path='/synthetic/borrow-lookup-input', **proposal['base_input']),
                   cfg_expansion_proposal=dict(path='/synthetic/cfg-expansion-proposal', bytes=10,
                                            sha256=A.CFG_EXPANSION_PROPOSAL_SHA),
                   cfg_expansion_overlay=overlay)
    inputs = dict(schema='ferric-guarded-mlp-ranked-cfg-expansion-diagnostic-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    value.pop('resource_admission_changed', None)
    with patch.object(A, 'CPU_ROOT', A.CFG_EXPANSION_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-ranked-cfg-expansion-diagnostic-cpu-v1',
                 diagnostic_build=True, diagnostic_only=True, admission_changed=False,
                 cfg_expansion_diagnostics=True, diagnostic_changes_admission=False,
                 static_failure_site_identified=True, actual_failure_block_count_observed=False,
                 graph_analysis_optimization=False, inherited_graph_analysis_optimization=True,
                 resource_admission_may_change=False, membership_lookup_optimization=True,
                 inherited_membership_lookup_optimization=True,
                 dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                 borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                 compiler_scratch_added=False,
                 inherited_use_lookup_optimization=True,
                 baseline_failure_caller_identified=False, inherited_graph_work_diagnostics=True,
                 cfg_expansion_test_filter=A.CFG_EXPANSION_PREFIX, cfg_expansion_tests=list(A.CFG_EXPANSION_NAMES),
                 controller=sources['run_cpu.py'], helper=sources['qualification_helpers.py'],
                 source_lineage=lineage, artifacts=products['artifacts'], tests_passed=2936)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='cfg-expansion-diagnostic'))
    inventory = sorted(value['test_inventories']['compiler-lib'] + list(A.CFG_EXPANSION_NAMES))
    value['test_inventories']['compiler-lib'] = inventory
    value['tests']['compiler']['names'] = inventory
    value['tests']['compiler']['named_outcomes'].update({name: 'ok' for name in A.CFG_EXPANSION_NAMES})
    value['tests']['compiler']['passed'] += 3
    for label in (*base['compiler_cohorts'], 'cfg-block-limit-diagnostic',
                  'cfg-block-cap-compiler', 'indexed-atomic-dag-certificate', 'graph-work-diagnostic',
                  'indexed-atomic-membership', 'indexed-atomic-dead-cast-census', 'indexed-atomic-use-lookup',
                  'indexed-atomic-borrow-lookup'):
        value['tests'][label]['filtered_out'] += 3
    value['tests']['cfg-expansion-diagnostic'] = dict(
        names=list(A.CFG_EXPANSION_NAMES), passed=3, failed=0, ignored=0, filtered_out=len(inventory)-3)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    return value, inputs, sources, records, manifest, previous, base, base_sources, proposal


HISTORICAL_FAILURE_ROOT = A.E / 'guarded-mlp-ranked-cfg-expansion-diagnostic-lowering-v228-v1'
HISTORICAL_FAILURE_SHA = 'ba277bc9fbf3593b0b797a6bba3e7f8ee4fd073b8e28dc33d245b465d0c09a59'
HISTORICAL_FAILURE_STDERR_SHA = 'f806e763a124eb460d47de32531749744f9d24a03cd240115d9f744326ee374a'
HISTORICAL_FAILURE_DIAGNOSTIC = (
    b'fe2o3 rustc extraction: production compilation general kernel verification failed: '
    b'semantic-to-ranked projection rejected semantic CFG projection exceeds the ranked block limit; '
    b'cfg-block-diagnostic-v1 blocks=2778 limit=2048 reason=above-limit '
    b'site=projected-cfg-expansion semantic_blocks=1121 '
    b'function_sha256=72f186b0c49aafc62f29a7b77a5bbca4fc458c507019297c8bea173b272fe70c '
    b'role=KernelRoot; source=Rust source edb8c73e35fa:17:1; '
    b'kernel_export=ferric_qwen3_mlp_state_guard_v1 phase=root-projection; '
    b'root ferric_qwen3_mlp_state_guard_v1 (semantic root 1, body 1)')

def cfg_compaction_fixture():
    base, _, base_sources, _, _, previous, *_ = cfg_expansion_fixture()
    for index, name in enumerate(sorted(name for name, new in A.CFG_COMPACTION_FILES.items()
                                        if not new and name not in base_sources), start=30):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.CFG_EXPANSION_ROOT / name))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=19, sha256='f' * 64)) for name in A.CFG_COMPACTION_FILES}
    assert all(row['before'] != row['after'] for row in overlay.values())
    proposal = dict(schema='ferric-guarded-mlp-ranked-cfg-compaction-source-v1', source_only=True,
                    cfg_compaction_optimization=True, admission_changed=True,
                    compiler_scratch_added=False, graph_analysis_optimization=False,
                    semantic_predicates_changed=False, structural_limits_changed=False,
                    resource_admission_may_change=True, static_failure_site_identified=True,
                    actual_failure_caller_identified=False, actual_failure_block_count_observed=False,
                    base_source_count=5803, source_count=5804, source_file_additions=1, new_test_count=9,
                    baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                    baseline_projected_blocks=2778, baseline_semantic_blocks=1121, baseline_block_limit=2048,
                    actual_failure=dict(bytes=15422, sha256=HISTORICAL_FAILURE_SHA),
                    actual_failure_stderr=dict(bytes=8424, sha256=HISTORICAL_FAILURE_STDERR_SHA),
                    base_complete=dict(bytes=10, sha256=A.CFG_EXPANSION_COMPLETE_SHA),
                    base_sources=dict(bytes=10, sha256=A.CFG_EXPANSION_SOURCES_SHA),
                    base_input=dict(bytes=10, sha256=A.CFG_EXPANSION_INPUT_SHA),
                    base_controller=A.compact(base['controller']),
                    graph_work_limit=3145728, block_limit=2048, edge_limit=2048,
                    filter=A.CFG_COMPACTION_PREFIX, test_names=list(A.CFG_COMPACTION_NAMES),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.CFG_COMPACTION_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.CFG_COMPACTION_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.CFG_COMPACTION_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.CFG_COMPACTION_ROOT / 'qualification_helpers.py'),
                                               bytes=10, sha256=A.CPU_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/cfg-expansion-complete', **proposal['base_complete']),
                   base_sources=dict(path='/synthetic/cfg-expansion-sources', **proposal['base_sources']),
                   base_input=dict(path='/synthetic/cfg-expansion-input', **proposal['base_input']),
                   cfg_compaction_proposal=dict(path='/synthetic/cfg-compaction-proposal', bytes=10,
                                            sha256=A.CFG_COMPACTION_PROPOSAL_SHA),
                   cfg_compaction_overlay=overlay,
                   baseline_failure=dict(path=str(HISTORICAL_FAILURE_ROOT / 'failed.json'),
                                         bytes=15422, sha256=HISTORICAL_FAILURE_SHA),
                   baseline_failure_stderr=dict(path=str(HISTORICAL_FAILURE_ROOT / 'compile.stderr'),
                                                bytes=8424, sha256=HISTORICAL_FAILURE_STDERR_SHA))
    inputs = dict(schema='ferric-guarded-mlp-ranked-cfg-compaction-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    value.pop('resource_admission_changed', None)
    with patch.object(A, 'CPU_ROOT', A.CFG_COMPACTION_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-ranked-cfg-compaction-cpu-v1',
                 diagnostic_build=False, diagnostic_only=False, admission_changed=True,
                 cfg_compaction_optimization=True, inherited_cfg_expansion_diagnostics=True,
                 static_failure_site_identified=True, actual_failure_block_count_observed=False,
                 graph_analysis_optimization=False, inherited_graph_analysis_optimization=True,
                 resource_admission_may_change=True, membership_lookup_optimization=True,
                 inherited_membership_lookup_optimization=True,
                 dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                 borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                 compiler_scratch_added=False,
                 inherited_use_lookup_optimization=True,
                 baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                 baseline_projected_blocks=2778, baseline_semantic_blocks=1121, baseline_block_limit=2048,
                 inherited_graph_work_diagnostics=True,
                 cfg_compaction_test_filter=A.CFG_COMPACTION_PREFIX, cfg_compaction_tests=list(A.CFG_COMPACTION_NAMES),
                 controller=sources['run_cpu.py'], helper=sources['qualification_helpers.py'],
                 source_lineage=lineage, artifacts=products['artifacts'], tests_passed=2954)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='cfg-compaction'))
    inventory = sorted(value['test_inventories']['compiler-lib'] + list(A.CFG_COMPACTION_NAMES))
    value['test_inventories']['compiler-lib'] = inventory
    value['tests']['compiler']['names'] = inventory
    value['tests']['compiler']['named_outcomes'].update({name: 'ok' for name in A.CFG_COMPACTION_NAMES})
    value['tests']['compiler']['passed'] += 9
    for label in (*base['compiler_cohorts'], 'cfg-block-limit-diagnostic',
                  'cfg-block-cap-compiler', 'indexed-atomic-dag-certificate', 'graph-work-diagnostic',
                  'indexed-atomic-membership', 'indexed-atomic-dead-cast-census', 'indexed-atomic-use-lookup',
                  'indexed-atomic-borrow-lookup', 'cfg-expansion-diagnostic'):
        value['tests'][label]['filtered_out'] += 9
    value['tests']['cfg-compaction'] = dict(
        names=list(A.CFG_COMPACTION_NAMES), passed=9, failed=0, ignored=0, filtered_out=len(inventory)-9)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    stdout = dict(path=str(HISTORICAL_FAILURE_ROOT / 'compile.stdout'), bytes=0,
                  sha256='e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855')
    source = dict(path='/synthetic/source-map', bytes=10, sha256='1' * 64)
    baseline_failure = dict(
        schema='ferric-guarded-mlp-ranked-cfg-expansion-diagnostic-lowering-result-v1',
        passed=False, failure="RuntimeError('natural successful/reaped leaf required')",
        postcheck_errors=[], source_unchanged=True, input_byte_maps_rechecked=True,
        automatic_retries=0, artifact=None, retained_handoff_or_llvm=False,
        capacity_limits=copy.deepcopy(A.CAPACITY_LIMITS),
        raw={'compile.stderr': lineage['baseline_failure_stderr'], 'compile.stdout': stdout,
             'source-before.json': source, 'source-after.json': dict(source)},
        phases=[dict(label='compile', exit_code=1, natural_exit=True, reaped=True,
                     process_group_absent=True, forced_cleanup=False, timed_out=False,
                     exception=None, observed_signals=[], stdout=stdout,
                     stderr=lineage['baseline_failure_stderr'])],
        production_authority=False, load_authority=False, launch_authority=False,
        gpu_execution=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False)
    baseline_stderr = HISTORICAL_FAILURE_DIAGNOSTIC + b'\n'
    return (value, inputs, sources, records, manifest, previous, base, base_sources,
            proposal, baseline_failure, baseline_stderr)


def cfg_linear_fusion_fixture():
    base, _, base_sources, _, _, previous, *_ = cfg_compaction_fixture()
    for index, name in enumerate(sorted(name for name, new in A.CFG_LINEAR_FUSION_FILES.items()
                                        if not new and name not in base_sources), start=40):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.CFG_COMPACTION_ROOT / name))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=20, sha256='6' * 64)) for name in A.CFG_LINEAR_FUSION_FILES}
    assert all(row['before'] != row['after'] for row in overlay.values())
    proposal = dict(schema='ferric-guarded-mlp-ranked-cfg-linear-fusion-source-v1', source_only=True,
                    cfg_linear_fusion_optimization=True, cfg_compaction_optimization=True,
                    inherited_cfg_compaction_optimization=True, admission_changed=True,
                    compiler_scratch_added=True, graph_analysis_optimization=False,
                    semantic_predicates_changed=False, structural_limits_changed=False,
                    resource_admission_may_change=True, static_failure_site_identified=True,
                    actual_failure_caller_identified=False, actual_failure_block_count_observed=False,
                    base_source_count=5804, source_count=5806, source_file_additions=2, new_test_count=9,
                    baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                    baseline_identity_first_refusal_blocks=1025, baseline_identity_block_limit=1024,
                    baseline_rendered_blocks=1675, baseline_rendered_edges=2230,
                    baseline_edge_verdict_observed=False,
                    actual_failure=dict(bytes=16503, sha256=A.BASE_FAILURE_SHA),
                    actual_failure_stderr=dict(bytes=164209, sha256=A.BASE_FAILURE_STDERR_SHA),
                    base_complete=dict(bytes=10, sha256=A.CFG_COMPACTION_COMPLETE_SHA),
                    base_sources=dict(bytes=10, sha256=A.CFG_COMPACTION_SOURCES_SHA),
                    base_input=dict(bytes=10, sha256=A.CFG_COMPACTION_INPUT_SHA),
                    base_controller=A.compact(base['controller']),
                    graph_work_limit=3145728, block_limit=2048, edge_limit=2048,
                    filter=A.CFG_LINEAR_FUSION_PREFIX, test_names=list(A.CFG_LINEAR_FUSION_NAMES),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.CFG_LINEAR_FUSION_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.CFG_LINEAR_FUSION_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.CFG_LINEAR_FUSION_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.CFG_LINEAR_FUSION_ROOT / 'qualification_helpers.py'),
                                               bytes=A.CFG_LINEAR_FUSION_HELPER_BYTES,
                                               sha256=A.CFG_LINEAR_FUSION_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/cfg-compaction-complete', **proposal['base_complete']),
                   base_sources=dict(path='/synthetic/cfg-compaction-sources', **proposal['base_sources']),
                   base_input=dict(path='/synthetic/cfg-compaction-input', **proposal['base_input']),
                   cfg_linear_fusion_proposal=dict(path='/synthetic/cfg-linear-fusion-proposal', bytes=10,
                                            sha256=A.CFG_LINEAR_FUSION_PROPOSAL_SHA),
                   cfg_linear_fusion_overlay=overlay,
                   baseline_failure=dict(path=str(A.BASE_FAILURE_ROOT / 'failed.json'),
                                         bytes=16503, sha256=A.BASE_FAILURE_SHA),
                   baseline_failure_stderr=dict(path=str(A.BASE_FAILURE_ROOT / 'compile.stderr'),
                                                bytes=164209, sha256=A.BASE_FAILURE_STDERR_SHA))
    inputs = dict(schema='ferric-guarded-mlp-ranked-cfg-linear-fusion-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    value.pop('resource_admission_changed', None)
    with patch.object(A, 'CPU_ROOT', A.CFG_LINEAR_FUSION_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-ranked-cfg-linear-fusion-cpu-v1',
                 diagnostic_build=False, diagnostic_only=False, admission_changed=True,
                 cfg_linear_fusion_optimization=True, cfg_compaction_optimization=True,
                    inherited_cfg_compaction_optimization=True, inherited_cfg_expansion_diagnostics=True,
                 static_failure_site_identified=True, actual_failure_block_count_observed=False,
                 graph_analysis_optimization=False, inherited_graph_analysis_optimization=True,
                 resource_admission_may_change=True, membership_lookup_optimization=True,
                 inherited_membership_lookup_optimization=True,
                 dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                 borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                 compiler_scratch_added=True,
                 inherited_use_lookup_optimization=True,
                 baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                 baseline_identity_first_refusal_blocks=1025, baseline_identity_block_limit=1024,
                    baseline_rendered_blocks=1675, baseline_rendered_edges=2230,
                    baseline_edge_verdict_observed=False,
                 inherited_graph_work_diagnostics=True,
                 cfg_linear_fusion_test_filter=A.CFG_LINEAR_FUSION_PREFIX, cfg_linear_fusion_tests=list(A.CFG_LINEAR_FUSION_NAMES),
                 controller=sources['run_cpu.py'], helper=sources['qualification_helpers.py'],
                 source_lineage=lineage, artifacts=products['artifacts'], tests_passed=2972)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='cfg-linear-fusion'))
    inventory = sorted(value['test_inventories']['compiler-lib'] + list(A.CFG_LINEAR_FUSION_NAMES))
    value['test_inventories']['compiler-lib'] = inventory
    value['tests']['compiler']['names'] = inventory
    value['tests']['compiler']['named_outcomes'].update({name: 'ok' for name in A.CFG_LINEAR_FUSION_NAMES})
    value['tests']['compiler']['passed'] += 9
    for label in (*base['compiler_cohorts'], 'cfg-block-limit-diagnostic',
                  'cfg-block-cap-compiler', 'indexed-atomic-dag-certificate', 'graph-work-diagnostic',
                  'indexed-atomic-membership', 'indexed-atomic-dead-cast-census', 'indexed-atomic-use-lookup',
                  'indexed-atomic-borrow-lookup', 'cfg-expansion-diagnostic', 'cfg-compaction'):
        value['tests'][label]['filtered_out'] += 9
    value['tests']['cfg-linear-fusion'] = dict(
        names=list(A.CFG_LINEAR_FUSION_NAMES), passed=9, failed=0, ignored=0, filtered_out=len(inventory)-9)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    baseline_failure = copy.deepcopy(BASELINE_FAILURE)
    baseline_stderr = BASELINE_STDERR
    return (value, inputs, sources, records, manifest, previous, base, base_sources,
            proposal, baseline_failure, baseline_stderr)


def memory_bounds_dag_fixture():
    base, _, base_sources, _, _, previous, *_ = cfg_linear_fusion_fixture()
    for index, name in enumerate(sorted(name for name, new in A.MEMORY_BOUNDS_DAG_FILES.items()
                                        if not new and name not in base_sources), start=60):
        row = base_sources.pop('fe2o3/file-' + str(index))
        base_sources[name] = dict(row, path=str(A.CFG_LINEAR_FUSION_ROOT / name))
    overlay = {name: dict(before=A.compact(base_sources[name]) if name in base_sources else None,
                          after=dict(bytes=20, sha256='6' * 64)) for name in A.MEMORY_BOUNDS_DAG_FILES}
    assert all(row['before'] != row['after'] for row in overlay.values())
    proposal = dict(schema='ferric-guarded-mlp-memory-bounds-dag-source-v1', source_only=True,
                    memory_bounds_dag_optimization=True, inherited_cfg_linear_fusion_optimization=True,
                    cfg_linear_fusion_optimization=True, cfg_compaction_optimization=True,
                    inherited_cfg_compaction_optimization=True, admission_changed=True,
                    compiler_scratch_added=True, graph_analysis_optimization=True,
                    semantic_predicates_changed=False, structural_limits_changed=False,
                    resource_admission_may_change=True, static_failure_site_identified=True,
                    actual_failure_caller_identified=False, actual_failure_block_count_observed=False,
                    base_source_count=5806, source_count=5808, source_file_additions=2, new_test_count=9,
                    baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                    baseline_memory_bounds_preflight_refusal=True,
                    baseline_rendered_blocks=567, baseline_rendered_edges=1122,
                    baseline_rendered_operations=2240, baseline_guard_candidates=552,
                    baseline_intersection_work_upper_bound=9340170,
                    baseline_runtime_work_exhaustion_observed=False,
                    baseline_edge_verdict_observed=False,
                    actual_failure=dict(bytes=17737, sha256=A.BASE_FAILURE_SHA),
                    actual_failure_stderr=dict(bytes=123245, sha256=A.BASE_FAILURE_STDERR_SHA),
                    base_complete=dict(bytes=10, sha256=A.CFG_LINEAR_FUSION_COMPLETE_SHA),
                    base_sources=dict(bytes=10, sha256=A.CFG_LINEAR_FUSION_SOURCES_SHA),
                    base_input=dict(bytes=10, sha256=A.CFG_LINEAR_FUSION_INPUT_SHA),
                    base_controller=A.compact(base['controller']),
                    graph_work_limit=3145728, block_limit=2048, edge_limit=2048,
                    work_limit=8388608, storage_limit=131072, fact_limit=1024,
                    production_requires_zero_ownership_contracts=True,
                    filter=A.MEMORY_BOUNDS_DAG_PREFIX, test_names=list(A.MEMORY_BOUNDS_DAG_NAMES),
                    files={name.removeprefix('fe2o3/'): row for name, row in overlay.items()})
    expected = {name: A.compact(row) for name, row in base_sources.items() if name.startswith('fe2o3/')}
    expected.update({name: row['after'] for name, row in overlay.items()})
    sources = {name: dict(path=str(A.MEMORY_BOUNDS_DAG_ROOT / name), **row) for name, row in expected.items()}
    sources['run_cpu.py'] = dict(path=str(A.MEMORY_BOUNDS_DAG_ROOT / 'run_cpu.py'), bytes=10,
                                sha256=A.MEMORY_BOUNDS_DAG_CONTROLLER_SHA)
    sources['qualification_helpers.py'] = dict(path=str(A.MEMORY_BOUNDS_DAG_ROOT / 'qualification_helpers.py'),
                                               bytes=A.MEMORY_BOUNDS_DAG_HELPER_BYTES,
                                               sha256=A.MEMORY_BOUNDS_DAG_HELPER_SHA)
    lineage = {key: {} for key in ('base_metadata', 'base_dependencies', 'base_streams')}
    lineage.update(base_complete=dict(path='/synthetic/cfg-linear-fusion-complete', **proposal['base_complete']),
                   base_sources=dict(path='/synthetic/cfg-linear-fusion-sources', **proposal['base_sources']),
                   base_input=dict(path='/synthetic/cfg-linear-fusion-input', **proposal['base_input']),
                   memory_bounds_dag_proposal=dict(path='/synthetic/memory-bounds-dag-proposal', bytes=10,
                                            sha256=A.MEMORY_BOUNDS_DAG_PROPOSAL_SHA),
                   memory_bounds_dag_overlay=overlay,
                   baseline_failure=dict(path=str(A.BASE_FAILURE_ROOT / 'failed.json'),
                                         bytes=17737, sha256=A.BASE_FAILURE_SHA),
                   baseline_failure_stderr=dict(path=str(A.BASE_FAILURE_ROOT / 'compile.stderr'),
                                                bytes=123245, sha256=A.BASE_FAILURE_STDERR_SHA))
    inputs = dict(schema='ferric-guarded-mlp-memory-bounds-dag-cpu-input-v1',
                  lineage=lineage, files={name: A.compact(row) for name, row in sources.items()},
                  tool_pins={}, metadata_relocations={}, rust_src={})
    value = copy.deepcopy(base)
    value.pop('resource_admission_changed', None)
    with patch.object(A, 'CPU_ROOT', A.MEMORY_BOUNDS_DAG_ROOT):
        products, _, _, records, _, _ = fixture()
    value.update(schema='ferric-guarded-mlp-memory-bounds-dag-cpu-v1',
                 diagnostic_build=False, diagnostic_only=False, admission_changed=True,
                 memory_bounds_dag_optimization=True, inherited_cfg_linear_fusion_optimization=True,
                    cfg_linear_fusion_optimization=True, cfg_compaction_optimization=True,
                    inherited_cfg_compaction_optimization=True, inherited_cfg_expansion_diagnostics=True,
                 static_failure_site_identified=True, actual_failure_block_count_observed=False,
                 graph_analysis_optimization=True, inherited_graph_analysis_optimization=True,
                 resource_admission_may_change=True, membership_lookup_optimization=True,
                 inherited_membership_lookup_optimization=True,
                 dead_cast_census_optimization=True, inherited_dead_cast_census_optimization=True,
                 borrow_lookup_optimization=True, inherited_borrow_lookup_optimization=True,
                 compiler_scratch_added=True,
                 inherited_use_lookup_optimization=True,
                 baseline_failure_caller_identified=True, baseline_failure_block_count_observed=True,
                 baseline_memory_bounds_preflight_refusal=True,
                    baseline_rendered_blocks=567, baseline_rendered_edges=1122,
                    baseline_rendered_operations=2240, baseline_guard_candidates=552,
                    baseline_intersection_work_upper_bound=9340170,
                    baseline_runtime_work_exhaustion_observed=False,
                    baseline_edge_verdict_observed=False,
                 inherited_graph_work_diagnostics=True,
                 memory_bounds_dag_test_filter=A.MEMORY_BOUNDS_DAG_PREFIX, memory_bounds_dag_tests=list(A.MEMORY_BOUNDS_DAG_NAMES),
                 baseline_memory_bounds_work_limit=8388608,
                 controller=sources['run_cpu.py'], helper=sources['qualification_helpers.py'],
                 source_lineage=lineage, artifacts=products['artifacts'], tests_passed=2990)
    position = next(i for i, row in enumerate(value['phases']) if row['label']=='atomic-extraction-0')
    value['phases'].insert(position, dict(value['phases'][0], label='memory-bounds-dag'))
    inventory = sorted(value['test_inventories']['pliron-lib'] + list(A.MEMORY_BOUNDS_DAG_NAMES))
    value['test_inventories']['pliron-lib'] = inventory
    value['tests']['pliron']['names'] = inventory
    value['tests']['pliron']['named_outcomes'].update({name: 'ok' for name in A.MEMORY_BOUNDS_DAG_NAMES})
    value['tests']['pliron']['passed'] += 9
    value['tests']['cfg-block-cap-pliron']['filtered_out'] += 9
    value['tests']['memory-bounds-dag'] = dict(
        names=list(A.MEMORY_BOUNDS_DAG_NAMES), passed=9, failed=0, ignored=0, filtered_out=len(inventory)-9)
    manifest = copy.deepcopy(previous)
    for name, role in (('fe2o3-rustc-extract', 'extractor'), ('librustc_codegen_fe2o3.so', 'backend')):
        product = value['artifacts'][role]['pin']
        manifest['bin/' + name] = dict(source=product['path'], **A.compact(product))
    baseline_failure = copy.deepcopy(BASELINE_FAILURE)
    baseline_stderr = BASELINE_STDERR
    return (value, inputs, sources, records, manifest, previous, base, base_sources,
            proposal, baseline_failure, baseline_stderr)


class MemoryBoundsDagAdmissionTests(unittest.TestCase):
    def test_exact_memory_bounds_dag_generation_preserves_non_deployed_provenance(self):
        args = memory_bounds_dag_fixture()
        self.assertEqual(args[0]['helper']['sha256'], A.MEMORY_BOUNDS_DAG_HELPER_SHA)
        self.assertEqual(args[6]['helper']['sha256'], A.CFG_LINEAR_FUSION_HELPER_SHA)
        self.assertEqual(args[0]['helper']['sha256'], args[6]['helper']['sha256'])
        proof = A.memory_bounds_dag_contract(*args)
        self.assertIs(proof['omitted_artifact_bodies_rehashed'], False)
        self.assertEqual(proof['rlib_cpu_provenance'], args[0]['artifacts']['backend-rlib'])

    def test_refuses_failed_admitting_or_wrong_generation(self):
        for key, bad in (('passed', False), ('failure', 'failed'),
                         ('diagnostic_build', True), ('diagnostic_only', True), ('admission_changed', False),
                         ('diagnostic_changes_admission', True), ('cfg_expansion_diagnostics', False),
                         ('static_failure_site_identified', False), ('actual_failure_block_count_observed', True),
                         ('structural_capacity_expansion', True), ('inherited_capacity_expansion', False),
                         ('structural_limits_changed', True), ('graph_analysis_optimization', False), ('inherited_graph_analysis_optimization', False),
                         ('resource_admission_may_change', False), ('membership_lookup_optimization', False),
                         ('inherited_membership_lookup_optimization', False),
                         ('dead_cast_census_optimization', False), ('compiler_scratch_added', False),
                         ('inherited_dead_cast_census_optimization', False), ('borrow_lookup_optimization', False), ('inherited_borrow_lookup_optimization', False),
                         ('use_lookup_optimization', False), ('inherited_use_lookup_optimization', False),
                         ('use_lookup_test_filter', 'wrong'), ('use_lookup_tests', []),
                         ('census_test_filter', 'wrong'), ('census_tests', []),
                         ('graph_work_diagnostics', False), ('semantic_predicates_changed', True),
                         ('actual_failure_caller_identified', True), ('baseline_failure_caller_identified', False),
                         ('baseline_failure_block_count_observed', False), ('baseline_memory_bounds_preflight_refusal', False),
                         ('baseline_rendered_blocks', 566), ('baseline_guard_candidates', 551),
                         ('baseline_rendered_operations', 2239), ('baseline_intersection_work_upper_bound', 9340169),
                         ('baseline_memory_bounds_work_limit', 8388607), ('baseline_runtime_work_exhaustion_observed', True),
                         ('baseline_rendered_edges', 1121), ('baseline_edge_verdict_observed', True),
                         ('cfg_compaction_optimization', False), ('inherited_cfg_compaction_optimization', False),
                         ('cfg_compaction_test_filter', 'wrong'), ('cfg_compaction_tests', []),
                         ('memory_bounds_dag_optimization', False), ('cfg_linear_fusion_optimization', False),
                         ('inherited_cfg_linear_fusion_optimization', False),
                         ('cfg_linear_fusion_test_filter', 'wrong'), ('cfg_linear_fusion_tests', []), ('inherited_cfg_expansion_diagnostics', False),
                         ('memory_bounds_dag_test_filter', 'wrong'), ('memory_bounds_dag_tests', []),
                         ('inherited_graph_work_diagnostics', False), ('cfg_diagnostics_retained', False),
                         ('postcheck_errors', ['drift']), ('source_unchanged', False)):
            args = memory_bounds_dag_fixture()
            args[0][key] = bad
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)
        for index, field, bad in (
                (0, 'sha256', A.CPU_HELPER_SHA),
                (0, 'sha256', '0' * 64),
                (0, 'bytes', A.MEMORY_BOUNDS_DAG_HELPER_BYTES - 1),
                (0, 'path', str(A.CFG_COMPACTION_ROOT / 'qualification_helpers.py')),
                (6, 'sha256', A.CPU_HELPER_SHA)):
            args = memory_bounds_dag_fixture()
            self.assertNotEqual(args[index]['helper'][field], bad)
            args[index]['helper'][field] = bad
            with self.subTest(helper_index=index, field=field), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)
        for key in ('controller', 'memory_bounds_dag_proposal'):
            args = memory_bounds_dag_fixture()
            row = args[0][key] if key == 'controller' else args[1]['lineage'][key]
            row['sha256'] = '0' * 64
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)
        for key, bad in (('passed', False), ('source_unchanged', False),
                         ('schema', 'ferric-guarded-mlp-s-rpo-qualification-cpu-v1'),
                         ('tests_passed', 2972-1), ('structural_capacity_expansion', True)):
            args = memory_bounds_dag_fixture()
            args[6][key] = bad
            with self.subTest(base_key=key), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)

        for key in ('baseline_failure', 'baseline_failure_stderr'):
            args = memory_bounds_dag_fixture()
            args[1]['lineage'][key]['sha256'] = '0' * 64
            with self.subTest(baseline_pin=key), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)
        for key, bad in (('passed', True), ('artifact', {}), ('source_unchanged', False),
                         ('postcheck_errors', ['drift']), ('automatic_retries', 1),
                         ('input_byte_maps_rechecked', False), ('gpu_execution', True)):
            args = memory_bounds_dag_fixture()
            args[9][key] = bad
            with self.subTest(baseline_receipt=key), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)
        for key, bad in (('exit_code', 0), ('natural_exit', False), ('reaped', False),
                         ('process_group_absent', False), ('forced_cleanup', True),
                         ('timed_out', True), ('exception', 'failure'), ('observed_signals', [15])):
            args = memory_bounds_dag_fixture()
            args[9]['phases'][0][key] = bad
            with self.subTest(baseline_phase=key), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)
        for body in (b'', A.BASE_FAILURE_DIAGNOSTIC.replace(b'work hard limit', b'wrong failure'),
                     A.BASE_FAILURE_DIAGNOSTIC + b'\n' + A.BASE_FAILURE_DIAGNOSTIC,
                     BASELINE_STDERR.replace(b'^bb566:', b'^bb565:'),
                     BASELINE_STDERR.replace(b'kernel.br ^bb225', b'kernel.br ^bb9999'),
                     BASELINE_STDERR.replace(b'@ferric_qwen3_mlp_state_guard_v1', b'@other_root'),
                     BASELINE_STDERR.replace(b'  = lowering stopped before target IR or artifact emission', b'')):
            args = list(memory_bounds_dag_fixture())
            args[10] = body
            with self.subTest(baseline_stderr=body), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)

    def test_refuses_changed_non_overlay_body_or_preimage(self):
        for change in ('other-body', 'preimage', 'extra-overlay', 'missing-source', 'missing-memory-bounds-dag-overlay'):
            args = memory_bounds_dag_fixture()
            if change == 'other-body':
                args[2]['fe2o3/file-26']['sha256'] = '0' * 64
                args[1]['files']['fe2o3/file-26']['sha256'] = '0' * 64
            elif change == 'preimage':
                args[8]['files']['crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds.rs']['before']['sha256'] = '0' * 64
            elif change == 'extra-overlay':
                args[8]['files']['extra.rs'] = dict(before=None, after=dict(bytes=1, sha256='0'*64))
            elif change == 'missing-memory-bounds-dag-overlay':
                args[8]['files'].pop('crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds/dag_schedule_v1_tests.rs')
            else:
                args[2].pop('fe2o3/file-26')
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)

    def test_refuses_lost_old_name_changed_ignore_or_memory_bounds_dag_status(self):
        for change in ('old-name', 'old-pliron-name', 'ignored', 'named-status', 'new-pliron-status', 'pliron-cap-filtered', 'new-name', 'filtered', 'cfg-filtered', 'cap-filtered', 'dag-filtered', 'graph-filtered', 'membership-filtered', 'census-filtered', 'use-lookup-filtered', 'borrow-lookup-filtered', 'expansion-filtered'):
            args = memory_bounds_dag_fixture()
            value = args[0]
            if change == 'old-name':
                value['test_inventories']['compiler-lib'][0] = 'substituted'
            elif change == 'old-pliron-name':
                value['test_inventories']['pliron-lib'][0] = 'substituted'
            elif change == 'new-pliron-status':
                value['tests']['pliron']['named_outcomes'][A.MEMORY_BOUNDS_DAG_NAMES[0]] = 'ignored'
            elif change == 'pliron-cap-filtered':
                value['tests']['cfg-block-cap-pliron']['filtered_out'] -= 9
            elif change == 'ignored':
                value['ignored_inventories']['compiler-lib'].pop()
            elif change == 'named-status':
                name = next(iter(value['tests']['compiler']['named_outcomes']))
                value['tests']['compiler']['named_outcomes'][name] = 'ignored'
            elif change == 'new-name':
                value['memory_bounds_dag_tests'][0] = 'substituted'
            elif change == 'cfg-filtered':
                value['tests']['cfg-block-limit-diagnostic']['filtered_out'] -= 9
            elif change == 'cap-filtered':
                value['tests']['cfg-block-cap-compiler']['filtered_out'] -= 9
            elif change == 'dag-filtered':
                value['tests']['indexed-atomic-dag-certificate']['filtered_out'] -= 9
            elif change == 'expansion-filtered':
                value['tests']['cfg-expansion-diagnostic']['filtered_out'] -= 9
            elif change == 'borrow-lookup-filtered':
                value['tests']['indexed-atomic-borrow-lookup']['filtered_out'] -= 9
            elif change == 'use-lookup-filtered':
                value['tests']['indexed-atomic-use-lookup']['filtered_out'] -= 9
            elif change == 'census-filtered':
                value['tests']['indexed-atomic-dead-cast-census']['filtered_out'] -= 9
            elif change == 'membership-filtered':
                value['tests']['indexed-atomic-membership']['filtered_out'] -= 9
            elif change == 'graph-filtered':
                value['tests']['graph-work-diagnostic']['filtered_out'] -= 9
            else:
                value['tests']['scope-0']['filtered_out'] -= 9
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)

    def test_refuses_phase_omission_reordering_and_cleanup(self):
        for change in ('missing', 'order', 'forced', 'memory-bounds-dag-position'):
            args = memory_bounds_dag_fixture()
            phases = args[0]['phases']
            if change == 'missing':
                phases.pop()
            elif change == 'order':
                phases[0], phases[1] = phases[1], phases[0]
            elif change == 'forced':
                phases[0]['forced_cleanup'] = True
            else:
                index = next(i for i, row in enumerate(phases) if row['label']=='memory-bounds-dag')
                phases[index], phases[index+1] = phases[index+1], phases[index]
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)

    def test_refuses_old_product_and_any_unchanged_tool_substitution(self):
        for change in ('extractor', 'cargo-fe2o3', 'clang-22', 'lld',
                       'fe2o3-engineering-lld-proxy', 'fe2o3-llvm-link-worker'):
            args = memory_bounds_dag_fixture()
            name = 'fe2o3-rustc-extract' if change == 'extractor' else change
            args[4]['bin/' + name]['sha256'] = '0' * 64
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)


    def test_refuses_changed_independent_limit_or_inherited_cohort(self):
        for key in A.CAPACITY_LIMITS:
            args = memory_bounds_dag_fixture()
            args[0]['capacity_limits'][key] += 1
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)
        for key in ('graph_work_limit', 'block_limit', 'edge_limit', 'fact_limit', 'storage_limit', 'work_limit'):
            args = memory_bounds_dag_fixture()
            args[8][key] += 1
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)
        args = memory_bounds_dag_fixture()
        args[8]['production_requires_zero_ownership_contracts'] = False
        with self.assertRaises(RuntimeError):
            A.memory_bounds_dag_contract(*args)
        for key in ('base_complete', 'base_sources', 'base_input'):
            args = memory_bounds_dag_fixture()
            args[1]['lineage'][key]['sha256'] = '0' * 64
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                A.memory_bounds_dag_contract(*args)
        args = memory_bounds_dag_fixture()
        args[0]['capacity_cohorts']['cfg-block-cap-pliron']['names'].pop()
        with self.assertRaises(RuntimeError):
            A.memory_bounds_dag_contract(*args)

    def test_pending_memory_bounds_dag_receipt_refuses_before_effects(self):
        for field in ('MEMORY_BOUNDS_DAG_COMPLETE_SHA', 'MEMORY_BOUNDS_DAG_HELPER_SHA'):
            with patch.object(A.sys, 'argv', ['audit_tools.py', '1' * 64]), \
                    patch.object(A.sys, 'dont_write_bytecode', True), \
                    patch.object(A, 'MEMORY_BOUNDS_DAG_CONTROLLER_SHA', '2' * 64), \
                    patch.object(A, 'MEMORY_BOUNDS_DAG_INPUT_SHA', '3' * 64), \
                    patch.object(A, 'MEMORY_BOUNDS_DAG_COMPLETE_SHA', '4' * 64), \
                    patch.object(A, field, None), patch.object(A.os, 'umask') as umask:
                with self.subTest(field=field), self.assertRaisesRegex(
                        RuntimeError, 'memory-bounds DAG producer bindings remain pending'):
                    A.main()
                umask.assert_not_called()


if __name__ == '__main__':
    unittest.main()
