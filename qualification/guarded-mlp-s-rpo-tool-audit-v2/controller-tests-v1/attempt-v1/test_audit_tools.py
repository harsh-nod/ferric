"""Synthetic admission tests only; none invokes readelf, ldd or a producer."""
import copy
import unittest
from unittest.mock import patch

import audit_tools as A


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
             patch.object(A.sys, 'argv', ['audit.py', '1' * 64]), \
             patch.object(A.os, 'getuid', side_effect=AssertionError('host/effect gate reached')):
            with self.assertRaisesRegex(RuntimeError, 'driver producer controller binding is pending'):
                A.main()


if __name__ == '__main__':
    unittest.main()

