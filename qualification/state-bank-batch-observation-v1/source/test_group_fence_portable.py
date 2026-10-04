"""Regression tests for distinct-cohort intake; no process, compiler, or GPU launch."""
import copy
from pathlib import Path
import unittest
from unittest.mock import patch

import group_fence_portable as G
import policy_portable as P
import export_group_fence as X


class GroupFenceTests(unittest.TestCase):
    def test_twelve_worker_only_recipes(self):
        rows, worker = G.recipes(Path('/owned/cpu'), {'worker': {'root': '/nightly'}})
        self.assertEqual(len(rows), 12)
        self.assertEqual(len({r[0] for r in rows}), 12)
        self.assertEqual(worker, Path('/owned/cpu/sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'))
        for name, argv, deadline in rows:
            self.assertEqual(argv[0], '/nightly/bin/cargo')
            self.assertIn('--offline', argv)
            self.assertIn('--locked', argv)
            self.assertNotIn('--features', argv)
            self.assertNotIn('--no-default-features', argv)
            self.assertEqual(deadline, 120 if name == 'metadata' else 1200)

    def test_runtime_and_worker_routes_stay_separate(self):
        rows, _ = G.recipes(Path('/owned/cpu'), {'worker': {'root': '/nightly'}})
        for name, argv, _ in rows:
            if name == 'runtime-list' or name in {r[0] for r in G.FILTERS}:
                self.assertEqual(argv[argv.index('-p') + 1], 'fe2o3-kfd')
            else:
                self.assertNotIn('-p', argv)

    def test_selection_is_seventy_seven_and_disjoint(self):
        self.assertEqual(sum(row[2] for row in G.FILTERS), 77)
        self.assertEqual(len({r[0] for r in G.FILTERS}), 7)
        self.assertEqual(len({r[1] for r in G.FILTERS}), 7)

    def test_cpu633_cannot_be_relabelled_as_new_worker(self):
        class Store:
            def doc(self, _): return {'schema': P.CPU_SCHEMA, 'passed': True}
        with self.assertRaises(RuntimeError):
            G.cpu_evidence(Store(), {}, None, None)

    def test_failed_or_authoritative_receipt_refuses_before_raw_reads(self):
        value = dict(schema='ferric-p228-group-fence-cpu-result-v1', passed=True, error=None,
            postcheck_errors=[], inputs=[], metadata={}, phases={}, tests={}, binaries={}, tests_passed=475,
            tests_ignored=4, empty_initial_target=True, external_cargo_cache_reused=True,
            gpu_execution=False, numerical_acceptance=False, performance_claim=False,
            production_authority=False, raw={})
        class Store:
            def doc(self, _): return changed
        for key, bad in (('passed', False), ('error', 'failure'), ('postcheck_errors', ['changed']),
                         ('tests_passed', 633), ('tests_ignored', 0), ('gpu_execution', True),
                         ('production_authority', True), ('performance_claim', True)):
            changed = dict(value, **{key: bad})
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                G.cpu_evidence(Store(), {}, None, None)

    def test_duplicate_input_census_refuses(self):
        row = dict(path='/input', bytes=1, sha256='a' * 64)
        with self.assertRaises(RuntimeError):
            G.source_delta(None, {'inputs': [row] * 25}, None, {})

    def test_unknown_or_wrong_cohort_refuses_before_file_reads(self):
        value = dict(schema=G.SCHEMA, base_deployment={'sha256': G.BASE_SHA},
                     worker_cpu={'sha256': G.CPU_SHA}, worker_cpu_review={}, aliases={}, runtime={})
        for key, bad in (('schema', P.SCHEMA), ('base_deployment', {'sha256': 'a' * 64}),
                         ('worker_cpu', {'sha256': 'b' * 64})):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                G.verify(None, None, dict(value, **{key: bad}), Path('/unused'), None)

    def test_courier_rejects_alias_conflicts_and_missing_inputs(self):
        row = dict(path='/input', bytes=1, sha256='a' * 64)
        cpu = dict(inputs=[], raw={}, binaries={})
        for changed in (dict(row, bytes=2), row):
            with self.assertRaises(RuntimeError):
                X.records(row, cpu, changed)

    def test_export_does_not_call_new_intake_to_validate_old_base(self):
        source = Path(X.__file__).read_text()
        self.assertIn('P.verify(I.D, pins, base,', source)
        self.assertNotIn('I.deployment(', source)

    def test_actual_artifact_identity_is_not_old_worker(self):
        self.assertNotEqual(G.WORKER_SHA, 'b0de0feedcdf85e4ad71aa418c0df3da72830a2b1050f1aba8dca3257cbd490f')
        self.assertNotEqual(G.CPU_SHA, '1fc4d17534161e6e7f96e6d0eba0a2455227ba2166623823f6a74f845b93deae')


if __name__ == '__main__':
    unittest.main()
