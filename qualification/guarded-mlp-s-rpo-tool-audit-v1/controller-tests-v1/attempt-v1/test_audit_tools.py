"""Synthetic admission tests only; none invokes readelf, ldd or a producer."""
import copy
import unittest

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


if __name__ == '__main__':
    unittest.main()
