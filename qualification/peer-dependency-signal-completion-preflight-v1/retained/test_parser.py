"""Synthetic checks for the two observed abort-child libtest output blocks."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('native_cpu_parser_v4', Path(__file__).with_name('run_cpu.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)

NAMES = (
    'queue_linux::tests::payload_release_failure_after_event_destroy_is_process_terminal',
    'queue_linux::tests::unpublished_custody_cleanup_failure_is_process_terminal',
)


def split(name):
    return 'test ' + name + ' ... \nrunning 1 test\nok\n'


def summary(passed, ignored=0, failed=0):
    status = 'FAILED' if failed else 'ok'
    return (f'test result: {status}. {passed} passed; {failed} failed; {ignored} ignored; '
            '0 measured; 0 filtered out; finished in 0.01s\n')


class ParserTests(unittest.TestCase):
    def parse(self, text):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'stdout'
            path.write_text(text)
            before = path.read_bytes()
            result = R.test_outcomes(path)
            self.assertEqual(path.read_bytes(), before)
            return result

    def test_ordinary_output_remains_unchanged(self):
        text = 'running 2 tests\ntest ordinary::pass ... ok\ntest ordinary::ignore ... ignored, retained artifact\n' + summary(1, 1)
        self.assertEqual(R.normalized_libtest(text), text)
        result = self.parse(text)
        self.assertEqual((result['passed'], result['failed'], result['ignored']), (1, 0, 1))

    def test_both_actual_split_blocks_become_exact_parent_ok(self):
        for name in NAMES:
            with self.subTest(name=name):
                self.assertEqual(R.normalized_libtest(split(name)), 'test ' + name + ' ... ok\n')
                self.assertEqual(R.normalized_libtest(split(name).rstrip('\n')), 'test ' + name + ' ... ok')

    def test_multiple_suite_summaries_and_raw_body_are_preserved(self):
        text = (split(NAMES[0]) + split(NAMES[1])
                + 'test retained::ignored ... ignored\n' + summary(2, 1)
                + 'test integration::pass ... ok\n' + summary(1)
                + 'test second_integration::pass ... ok\n' + summary(1))
        result = self.parse(text)
        self.assertEqual((result['passed'], result['failed'], result['ignored']), (4, 0, 1))
        self.assertEqual([row['passed'] for row in result['summaries']], [2, 1, 1])
        self.assertEqual([row['name'] for row in result['named'][:2]], list(NAMES))

    def test_unknown_parent_block_is_not_normalized_or_counted(self):
        text = split('queue_linux::tests::unknown_child_test') + summary(1)
        self.assertEqual(R.normalized_libtest(text), text)
        with self.assertRaisesRegex(RuntimeError, 'named libtest census'):
            self.parse(text)

    def test_malformed_known_blocks_are_not_repaired(self):
        original = split(NAMES[0])
        variants = (
            original.replace('running 1 test', 'running 2 tests'),
            original.replace('running 1 test\n', ''),
            original.replace('running 1 test\n', 'running 1 test\nextra output\n'),
            original.replace(' ... \n', ' ...\n'),
            original.replace('\nok\n', '\nFAILED\n'),
            original.replace('\nok\n', '\nok extra\n'),
        )
        for block in variants:
            with self.subTest(block=block):
                self.assertEqual(R.normalized_libtest(block), block)
                with self.assertRaisesRegex(RuntimeError, 'named libtest census'):
                    self.parse(block + summary(1))

    def test_duplicate_special_parent_block_is_refused(self):
        for name in NAMES:
            with self.subTest(name=name):
                with self.assertRaisesRegex(RuntimeError, 'duplicate known abort-child'):
                    R.normalized_libtest(split(name) + split(name))

    def test_already_inline_known_parents_remain_valid(self):
        text = ''.join('test ' + name + ' ... ok\n' for name in NAMES) + summary(2)
        self.assertEqual(R.normalized_libtest(text), text)
        self.assertEqual(self.parse(text)['passed'], 2)

    def test_missing_summary_or_mismatched_count_still_refuses(self):
        with self.assertRaisesRegex(RuntimeError, 'no actual libtest summaries'):
            self.parse(split(NAMES[0]))
        with self.assertRaisesRegex(RuntimeError, 'named libtest census'):
            self.parse(split(NAMES[0]) + summary(2))
        with self.assertRaisesRegex(RuntimeError, 'named libtest census'):
            self.parse('prefix ' + split(NAMES[0]) + summary(1))


if __name__ == '__main__':
    unittest.main()
