"""Focused named-libtest parser fixtures; root owns execution."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('provider_cpu_v3', Path(__file__).with_name('run_cpu.py'))
M = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(M)

FIRST = 'memory::tests::safe_copy_rejects_an_out_of_bounds_witness'
SECOND = 'memory::tests::safe_volatile_load_rejects_out_of_bounds_access'
NAMES = [FIRST, SECOND]
SUMMARY = 'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s\n'


class OutcomesTests(unittest.TestCase):
    def parse(self, body):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'stdout'
            path.write_text(body)
            return M.outcomes(path, NAMES, 5)

    def body(self, annotation=''):
        return ''.join('test ' + name + annotation + ' ... ok\n' for name in NAMES) + '\n' + SUMMARY

    def test_ordinary_named_passes(self):
        result = self.parse(self.body())
        self.assertEqual(result, dict(names=sorted(NAMES), passed=2, failed=0, ignored=0, filtered_out=3))

    def test_standard_should_panic_annotation_passes(self):
        self.assertEqual(self.parse(self.body(' - should panic'))['names'], sorted(NAMES))

    def test_failed_status_rejected(self):
        with self.assertRaisesRegex(RuntimeError, 'selected named tests'):
            self.parse(self.body(' - should panic').replace(' ... ok', ' ... FAILED', 1))

    def test_ignored_status_rejected(self):
        with self.assertRaisesRegex(RuntimeError, 'selected named tests'):
            self.parse(self.body().replace(' ... ok', ' ... ignored, historical', 1))

    def test_duplicate_name_rejected(self):
        with self.assertRaisesRegex(RuntimeError, 'selected named tests'):
            self.parse(self.body().replace(SECOND, FIRST))

    def test_missing_named_line_rejected(self):
        with self.assertRaisesRegex(RuntimeError, 'selected named tests'):
            self.parse(self.body().replace('test ' + FIRST + ' ... ok\n', ''))

    def test_forged_summary_counts_rejected(self):
        for before, after in [('2 passed', '3 passed'), ('3 filtered out', '0 filtered out'),
                              ('0 ignored', '1 ignored'), ('0 failed', '1 failed')]:
            with self.subTest(field=before), self.assertRaisesRegex(RuntimeError, 'selected named tests'):
                self.parse(self.body().replace(before, after))

    def test_unrecognized_annotation_rejected(self):
        with self.assertRaisesRegex(RuntimeError, 'selected named tests'):
            self.parse(self.body(' - expected panic'))


if __name__ == '__main__':
    unittest.main(verbosity=2)
