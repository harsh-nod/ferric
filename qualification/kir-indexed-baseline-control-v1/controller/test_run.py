"""Synthetic negative-outcome observer tests; no subprocesses or qualified imports."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('unchanged_rpo_baseline', Path(__file__).with_name('run.py'))
M = importlib.util.module_from_spec(spec)
spec.loader.exec_module(M)


class BaselineTests(unittest.TestCase):
    def result(self, code=101, **changes):
        return dict(dict(exit_code=code, reason=None, group_absent=True), **changes)

    def output(self, names, failed=True, filtered=0):
        rows = ['test ' + name + ' ... ' + ('FAILED' if failed and name == M.FAILURE else 'ok') for name in names]
        if failed:
            rows += ['assertion `left == right` failed', '  left: (8, Some(7), 5)', ' right: (7, Some(4), 5)']
        rows += [f'test result: {"FAILED" if failed else "ok"}. {len(names) - int(failed)} passed; {int(failed)} failed; 0 ignored; 0 measured; {filtered} filtered out; finished in 0.01s']
        return '\n'.join(rows) + '\n'

    def test_focused_exact_prior_assertion_is_negative_observation(self):
        value = M.observed_tests(self.output([M.FAILURE], filtered=764), '', [M.FAILURE], 764, self.result())
        self.assertEqual(value['failed_names'], [M.FAILURE])
        self.assertEqual(value['passed'], 0)
        self.assertFalse(value['test_suite_passed'])

    def test_full765_named_result_preserves_single_failure(self):
        names = sorted([M.FAILURE] + ['other_' + str(i) for i in range(764)])
        value = M.observed_tests(self.output(names), '', names, 0, self.result())
        self.assertEqual((value['passed'], value['failed'], value['ignored']), (764, 1, 0))

    def test_natural_success_is_measured_not_assumed(self):
        names = [M.FAILURE]
        value = M.observed_tests(self.output(names, failed=False), '', names, 0, self.result(0))
        self.assertTrue(value['test_suite_passed'])
        self.assertEqual(value['failed_names'], [])

    def test_changed_failure_tuple_is_refused(self):
        good = self.output([M.FAILURE])
        for text in (good.replace('(8, Some(7), 5)', '(9, Some(7), 5)'),
                     good.replace('(7, Some(4), 5)', '(7, Some(3), 5)'), good + 'left: (8, Some(7), 5)\nright: (7, Some(4), 5)\n'):
            with self.assertRaises(RuntimeError): M.observed_tests(text, '', [M.FAILURE], 0, self.result())

    def test_unrelated_failed_test_is_refused(self):
        text = self.output([M.FAILURE]).replace(M.FAILURE, 'unrelated')
        with self.assertRaises(RuntimeError): M.observed_tests(text, '', ['unrelated'], 0, self.result())

    def test_missing_duplicate_and_ignored_results_refused(self):
        good = self.output([M.FAILURE])
        for text in (good.replace('test ' + M.FAILURE, 'not-a-test ' + M.FAILURE),
                     good + f'test {M.FAILURE} ... FAILED\n', good.replace(' ... FAILED', ' ... ignored')):
            with self.assertRaises(RuntimeError): M.observed_tests(text, '', [M.FAILURE], 0, self.result())

    def test_summary_filtered_count_and_exit_are_exact(self):
        good = self.output([M.FAILURE])
        for text in (good.replace('0 passed', '1 passed'), good.replace('0 filtered', '1 filtered'),
                     good.replace('0 measured', '1 measured')):
            with self.assertRaises(RuntimeError): M.observed_tests(text, '', [M.FAILURE], 0, self.result())
        with self.assertRaises(RuntimeError): M.observed_tests(good, '', [M.FAILURE], 0, self.result(0))

    def test_timeout_signal_boolean_or_surviving_group_refused(self):
        for result in (self.result(-9), self.result(True), self.result(reason='deadline'), self.result(group_absent=False)):
            with self.assertRaises(RuntimeError): M.observed_tests(self.output([M.FAILURE]), '', [M.FAILURE], 0, result)

    def test_only_bounded_final101_assertion_can_be_observed(self):
        M.allow_observed_assertion(AssertionError('phase'), 'phase', self.result())
        for error in (RuntimeError('phase'), AssertionError('another'), AssertionError()):
            with self.assertRaises(RuntimeError): M.allow_observed_assertion(error, 'phase', self.result())

    def test_observer_never_catches_resource_or_group_failures(self):
        for result in (self.result(0), self.result(-15), self.result(reason='free-space-floor'), self.result(group_absent=False)):
            with self.assertRaises(RuntimeError): M.allow_observed_assertion(AssertionError('phase'), 'phase', result)

    def inventory(self):
        additions = ['indexed_' + str(i) for i in range(20)]
        baseline = sorted([M.FAILURE] + ['old_' + str(i) for i in range(764)])
        return baseline, sorted(baseline + additions), additions

    def test_inventory_exactly_candidate_minus_twenty(self):
        baseline, candidate, additions = self.inventory()
        self.assertEqual(M.baseline_inventory(baseline, [], candidate, additions), baseline)

    def test_inventory_no_substitution_or_hidden_skips(self):
        baseline, candidate, additions = self.inventory()
        for names, ignored in ((baseline[:-1], []), (sorted(baseline[:-1] + ['substitute']), []),
                               (baseline, [M.FAILURE])):
            with self.assertRaises(RuntimeError): M.baseline_inventory(names, ignored, candidate, additions)
        with self.assertRaises(RuntimeError): M.baseline_inventory(baseline, [], candidate, additions[:-1])

    def dependency_fixture(self):
        original = Path('/qualified/source')
        previous = {str(original / 'crates/file.rs'): {'old': True}, '/external/file': {'pin': 1}}
        before = {str(M.COPY / 'crates/file.rs'): {'new-stamp': True}, str(M.COPY / 'Cargo.toml'): {'whole-source-only': True}}
        current = {str(M.COPY / 'crates/file.rs'): {'new-stamp': True}, '/external/file': {'pin': 1}}
        return previous, current, before, original

    def test_dependency_subset_has_no_indexed_additions(self):
        old, now, before, original = self.dependency_fixture()
        M.dependency_transition(old, now, before, original)
        with self.assertRaises(RuntimeError): M.dependency_transition(old, {**now, str(M.COPY / 'new.rs'): {}}, before, original)

    def test_dependency_bodies_and_external_sources_stay_exact(self):
        old, now, before, original = self.dependency_fixture()
        for name in ('/external/file', str(M.COPY / 'crates/file.rs')):
            with self.assertRaises(RuntimeError): M.dependency_transition(old, {**now, name: {}}, before, original)

    def test_six_phase_control_has_no_overlay_formatter_or_finalizer(self):
        self.assertEqual(M.PHASES, {'metadata', 'lower-build-tests', 'lower-list', 'lower-ignored-list', 'lower-focused-test', 'lower-tests'})
        self.assertNotEqual(M.OUT, M.CANDIDATE)
        self.assertFalse(M.TARGET.is_relative_to(M.CANDIDATE))

    def test_selected_failure_cannot_be_one_of_twenty_additions(self):
        baseline, candidate, additions = self.inventory()
        additions[0] = M.FAILURE
        with self.assertRaises(RuntimeError): M.baseline_inventory(baseline, [], candidate, additions)


if __name__ == '__main__':
    unittest.main(verbosity=2)
