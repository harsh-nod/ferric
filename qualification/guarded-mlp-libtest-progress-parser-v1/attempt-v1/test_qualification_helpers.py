"""Standalone parser tests; no compiler, project, or GPU execution."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent
HELPER_SHA = 'ba127f1057546c0ce6e57fb78832c3e774c1108a4f7c5a2421f4aad7f51108be'
DATA_PINS = {
    'baseline-complete.json': (1344193, '439d4b5bf10a1fe9f44d02fa1cbd28b03eec9601bb161806909b2f073beb9018'),
    'actual-failed.json': (900004, '67f0792e4d90b33f939fa1df7af45024d27d0cdb184ff82b7762b89f0910a9dd'),
    'actual-compiler-tests.stdout': (164031, '77d8c1c15022e2addd6b1e2411ea1f6464b4e151528cd619941eea64f7a7d3d1'),
}
NEW_NAMES = tuple('production_ranked_projection_v1::tests::cfg_linear_fusion_' + name for name in (
    'chain_and_identity_boundaries',
    'conditional_effect_traces',
    'cycles_and_argument_fallback',
    'duplicate_edges_and_entry_are_preserved',
    'malformed_references_fail_closed',
    'measured_guard_pattern',
    'production_projection_route',
    'source_wave_generated_coordinates',
    'work_storage_and_failure_boundaries',
))
helper_path = ROOT / 'qualification_helpers.py'
assert hashlib.sha256(helper_path.read_bytes()).hexdigest() == HELPER_SHA
spec = importlib.util.spec_from_file_location('qualified_progress_helper', helper_path)
H = importlib.util.module_from_spec(spec)
spec.loader.exec_module(H)


class FullSuiteProgressTests(unittest.TestCase):
    def parse(self, text, names=('a', 'b'), ignored=('b',)):
        with tempfile.TemporaryDirectory(prefix='ferric-libtest-parser-') as directory:
            path = Path(directory) / 'stdout'
            path.write_text(text)
            return H.full_suite_outcomes(path, names, ignored)

    @staticmethod
    def summary(passed=1, ignored=1):
        return ('test result: ok. ' + str(passed) + ' passed; 0 failed; '
                + str(ignored) + ' ignored; 0 measured; 0 filtered out; finished in 0.01s\n')

    def valid(self, prefix=''):
        return prefix + 'test a ... ok\ntest b ... ignored\n' + self.summary()

    def test_no_progress_preserves_exact_result_shape(self):
        self.assertEqual(self.parse(self.valid()), dict(
            names=['a', 'b'], passed=1, failed=0, ignored=1, filtered_out=0,
            ignored_names=['b'], named_outcomes={'a': 'ok', 'b': 'ignored'}))

    def test_one_progress_notice_does_not_add_an_outcome(self):
        plain = self.parse(self.valid())
        self.assertEqual(self.parse(self.valid('test a has been running for over 60 seconds\n')), plain)

    def test_independent_active_names_can_report_once(self):
        text = ('test a has been running for over 60 seconds\n'
                'test b has been running for over 60 seconds\n'
                'test b ... ok\ntest a ... ok\n' + self.summary(2, 0))
        self.assertEqual(self.parse(text, ignored=())['named_outcomes'], {'a': 'ok', 'b': 'ok'})

    def test_should_panic_terminal_form_is_preserved(self):
        text = self.valid('test a has been running for over 60 seconds\n')
        text = text.replace('test a ... ok', 'test a - should panic ... ok')
        self.assertEqual(self.parse(text)['passed'], 1)

    def test_unknown_progress_name_is_refused(self):
        with self.assertRaisesRegex(RuntimeError, 'invalid named libtest progress notice'):
            self.parse(self.valid('test unknown has been running for over 60 seconds\n'))

    def test_ignored_progress_name_is_refused(self):
        with self.assertRaisesRegex(RuntimeError, 'invalid named libtest progress notice'):
            self.parse(self.valid('test b has been running for over 60 seconds\n'))

    def test_duplicate_progress_notice_is_refused(self):
        with self.assertRaisesRegex(RuntimeError, 'invalid named libtest progress notice'):
            self.parse(self.valid('test a has been running for over 60 seconds\n' * 2))

    def test_progress_after_own_terminal_is_refused(self):
        text = ('test a ... ok\ntest a has been running for over 60 seconds\n'
                'test b ... ignored\n' + self.summary())
        with self.assertRaisesRegex(RuntimeError, 'invalid named libtest progress notice'):
            self.parse(text)

    def test_progress_after_summary_is_refused_even_before_own_terminal(self):
        text = (self.summary() + 'test a has been running for over 60 seconds\n'
                'test a ... ok\ntest b ... ignored\n')
        with self.assertRaisesRegex(RuntimeError, 'invalid named libtest progress notice'):
            self.parse(text)

    def test_malformed_progress_forms_are_refused(self):
        for line in (
            'test a has been running for over 0 seconds',
            'test a has been running for over 59 seconds',
            'test a has been running for over 61 seconds',
            'test a has been running for over 060 seconds',
            'test a has been running for over 60.0 seconds',
            'test a has been running for over 60 second',
            'test a has been running for over 60 seconds trailing',
            'test a has been running for over 60 seconds ',
            'test a has been running for over 60 seconds\t',
            'test a - should panic has been running for over 60 seconds',
            'test a/b has been running for over 60 seconds',
        ):
            with self.subTest(line=line), self.assertRaisesRegex(RuntimeError, 'malformed named libtest result'):
                self.parse(self.valid(line + '\n'))

    def test_progress_does_not_replace_missing_terminal(self):
        text = ('test a has been running for over 60 seconds\ntest b ... ignored\n'
                + self.summary())
        with self.assertRaisesRegex(RuntimeError, 'full inventory must appear exactly once'):
            self.parse(text)

    def test_duplicate_and_unknown_terminals_remain_refused(self):
        for text in (self.valid().replace('test b ... ignored', 'test a ... ok'),
                     self.valid().replace('test b ... ignored', 'test unknown ... ignored'),
                     'test a ... ok\n' + self.valid()):
            with self.subTest(text=text), self.assertRaisesRegex(RuntimeError, 'full inventory must appear exactly once'):
                self.parse(text)

    def test_failed_and_changed_ignored_statuses_remain_refused(self):
        for text in (self.valid().replace('test a ... ok', 'test a ... FAILED'),
                     self.valid().replace('test a ... ok', 'test a ... ignored'),
                     self.valid().replace('test b ... ignored', 'test b ... ok')):
            with self.subTest(text=text), self.assertRaisesRegex(RuntimeError, 'full-suite status differs'):
                self.parse(text)

    def test_malformed_final_status_remains_refused(self):
        with self.assertRaisesRegex(RuntimeError, 'malformed named libtest result'):
            self.parse(self.valid().replace('test a ... ok', 'test a ... passed'))

    def test_summary_count_filter_and_duplicates_remain_refused(self):
        for text in (self.valid().replace('1 passed;', '2 passed;'),
                     self.valid().replace('0 filtered out;', '1 filtered out;'),
                     self.valid() + self.summary(),
                     self.valid().replace('test result: ok.', 'test result: FAILED.')):
            with self.subTest(text=text), self.assertRaisesRegex(RuntimeError, 'full-suite summary must match'):
                self.parse(text)

    def test_invalid_expected_inventories_remain_refused(self):
        for names, ignored in ((('a', 'a'), ()), (('a', 'b'), ('b', 'b')),
                               (('a',), ('b',)), (('bad/name',), ()), ((), ())):
            with self.subTest(names=names, ignored=ignored), self.assertRaises(RuntimeError):
                self.parse(self.valid(), names, ignored)

    def test_actual_v2_stream_replays_without_relabeling_failed_qualification(self):
        bodies = {name: (ROOT / name).read_bytes() for name in DATA_PINS}
        for name, body in bodies.items():
            self.assertEqual((len(body), hashlib.sha256(body).hexdigest()), DATA_PINS[name])
        baseline = json.loads(bodies['baseline-complete.json'])
        failed = json.loads(bodies['actual-failed.json'])
        self.assertIs(baseline['passed'], True)
        self.assertIs(failed['passed'], False)
        self.assertEqual(failed['failure'], "RuntimeError('malformed named libtest result')")
        names = sorted(baseline['test_inventories']['compiler-lib'] + list(NEW_NAMES))
        ignored = baseline['ignored_inventories']['compiler-lib']
        self.assertEqual(len(names), len(set(names)))
        self.assertEqual((len(names), len(ignored)), (1347, 24))
        self.assertEqual(failed['test_inventories']['compiler-lib'], names)
        self.assertEqual(failed['ignored_inventories']['compiler-lib'], ignored)
        stream_pin = failed['raw']['compiler-tests.stdout']
        self.assertEqual((stream_pin['bytes'], stream_pin['sha256']), DATA_PINS['actual-compiler-tests.stdout'])
        self.assertEqual(failed['controller']['sha256'],
                         '696f03d895777bf8d96a5d9e883049adf740f1ebedc39cabb90c5894fe392289')
        phase = [row for row in failed['phases'] if row['label'] == 'compiler-tests']
        self.assertEqual(len(phase), 1)
        self.assertEqual(phase[0]['exit_code'], 0)
        self.assertIs(phase[0]['natural_exit'], True)
        self.assertIs(phase[0]['reaped'], True)
        self.assertIs(phase[0]['process_group_absent'], True)
        result = H.full_suite_outcomes(ROOT / 'actual-compiler-tests.stdout', names, ignored)
        self.assertEqual(result['names'], names)
        self.assertEqual((result['passed'], result['failed'], result['ignored'], result['filtered_out']),
                         (1323, 0, 24, 0))
        self.assertEqual(result['ignored_names'], sorted(ignored))
        self.assertEqual(result['named_outcomes'], {
            name: 'ignored' if name in ignored else 'ok' for name in names})


if __name__ == '__main__':
    unittest.main(verbosity=2)
