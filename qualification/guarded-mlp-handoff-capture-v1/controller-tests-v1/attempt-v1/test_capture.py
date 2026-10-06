"""Synthetic capture fixtures; execute only on MI350, never invoke the compiler."""
import ast
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys
import tempfile
import unittest
from unittest import mock


assert len(sys.argv) == 3
CURRENT_PATH, PREVIOUS_PATH = map(Path, sys.argv[1:])
CURRENT_BYTES, PREVIOUS_BYTES = CURRENT_PATH.read_bytes(), PREVIOUS_PATH.read_bytes()
TEST_BYTES = Path(__file__).read_bytes()
assert hashlib.sha256(PREVIOUS_BYTES).hexdigest() == '637857bc4a5178bae425135271a3c537f14269f368d62d3dc82107fe5c13f33e'
CURRENT_AST, PREVIOUS_AST = ast.parse(CURRENT_BYTES), ast.parse(PREVIOUS_BYTES)
SELECTED = {'HandoffCapture', 'require', 'PRIOR_HANDOFF', 'PRIOR_IMAGE', 'PRIOR_DESCRIPTOR_SHA'}


def name(node):
    if isinstance(node, (ast.FunctionDef, ast.ClassDef)):
        return node.name
    if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
        return node.targets[0].id
    return None


scope = dict(Path=Path, os=os, stat=stat, re=re, hashlib=hashlib)
selected = [node for node in CURRENT_AST.body if name(node) in SELECTED]
assert {name(node) for node in selected} == SELECTED
exec(compile(ast.Module(body=selected, type_ignores=[]), str(CURRENT_PATH), 'exec'), scope)
Capture = scope['HandoffCapture']
sys.argv[:] = [sys.argv[0]]


def identity(body):
    return dict(byte_len=len(body), sha256=hashlib.sha256(body).hexdigest())


class CaptureTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='capture-fixture-', dir=Path.cwd())
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.pid = os.getpid()
        self.child = self.root / ('fe2o3-engineering-hsaco-' + str(self.pid) + '-123-0')
        self.body = b'complete synthetic handoff bytes for fixture only'
        self.expected = identity(self.body)
        self.capture = Capture(self.root, self.expected)
        self.addCleanup(self.capture.close)

    def file(self, body=None):
        self.child.mkdir(mode=0o700, exist_ok=True)
        result = self.child / 'compiler-handoff-v2'
        result.write_bytes(self.body if body is None else body)
        return result

    def artifact(self):
        return {'value': {'compiler_handoff': self.expected,
                          'hsaco': {'identity': scope['PRIOR_IMAGE'],
                                    'canonical_descriptor_sha256': scope['PRIOR_DESCRIPTOR_SHA']}}}

    def test_absent_then_complete(self):
        self.capture.poll(self.pid)
        self.assertIsNone(self.capture.snapshot)
        self.file()
        self.capture.poll(self.pid)
        self.assertTrue(self.capture.matched)
        self.assertEqual(self.capture.snapshot, self.body)
        self.assertTrue(all(self.capture.joins(self.artifact()).values()))

    def test_partial_stable_read_does_not_stop_polling(self):
        path = self.file(self.body[:5])
        self.capture.poll(self.pid)
        self.assertEqual(self.capture.snapshot, self.body[:5])
        self.assertFalse(self.capture.matched)
        path.write_bytes(self.body)
        self.capture.poll(self.pid)
        self.assertTrue(self.capture.matched)

    def test_empty_stable_read_is_not_success(self):
        self.file(b'')
        self.capture.poll(self.pid)
        self.assertFalse(self.capture.matched)
        self.assertFalse(self.capture.joins(self.artifact())['handoff_matches_current'])

    def test_same_size_mutation_during_read_is_pending(self):
        path = self.file()
        read = os.read
        changed = False

        def mutate(fd, maximum):
            nonlocal changed
            part = read(fd, maximum)
            if not changed:
                changed = True
                path.write_bytes(b'X' * len(self.body))
            return part

        with mock.patch.object(os, 'read', side_effect=mutate):
            self.capture.poll(self.pid)
        self.assertIsNone(self.capture.snapshot)
        self.capture.poll(self.pid)
        self.assertFalse(self.capture.matched)

    def test_inode_replacement_is_rejected(self):
        path = self.file(b'partial')
        self.capture.poll(self.pid)
        replacement = self.child / 'replacement'
        replacement.write_bytes(self.body)
        replacement.replace(path)
        with self.assertRaisesRegex(RuntimeError, 'inode replaced'):
            self.capture.poll(self.pid)

    def test_parent_replacement_is_rejected(self):
        self.file(b'partial')
        self.capture.poll(self.pid)
        self.child.rename(self.root / 'old')
        self.file()
        with self.assertRaisesRegex(RuntimeError, 'child replaced'):
            self.capture.poll(self.pid)

    def test_root_replacement_is_rejected(self):
        moved = self.root.with_name(self.root.name + '-moved')
        self.root.rename(moved)
        self.root.mkdir()
        self.addCleanup(lambda: moved.rmdir())
        with self.assertRaisesRegex(RuntimeError, 'root substituted'):
            self.capture.poll(self.pid)

    def test_leaf_symlink_is_rejected(self):
        target = self.root / 'outside'
        target.write_bytes(self.body)
        self.child.mkdir(mode=0o700)
        (self.child / 'compiler-handoff-v2').symlink_to(target)
        with self.assertRaises(OSError):
            self.capture.poll(self.pid)

    def test_parent_symlink_is_rejected(self):
        target = self.root / 'outside'
        target.mkdir()
        (target / 'compiler-handoff-v2').write_bytes(self.body)
        self.child.symlink_to(target, target_is_directory=True)
        with self.assertRaises(OSError):
            self.capture.poll(self.pid)

    def test_root_symlink_is_rejected(self):
        alias = self.root / 'alias'
        alias.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises((OSError, RuntimeError)):
            Capture(alias, self.expected)

    def test_fifo_is_nonblocking_and_rejected(self):
        self.child.mkdir(mode=0o700)
        os.mkfifo(self.child / 'compiler-handoff-v2')
        with self.assertRaisesRegex(RuntimeError, 'owner/kind'):
            self.capture.poll(self.pid)

    def test_directory_in_place_of_file_is_rejected(self):
        self.child.mkdir(mode=0o700)
        (self.child / 'compiler-handoff-v2').mkdir()
        with self.assertRaisesRegex(RuntimeError, 'owner/kind'):
            self.capture.poll(self.pid)

    def test_oversize_file_is_rejected(self):
        self.file(b'X' * (Capture.MAX_BYTES + 1))
        with self.assertRaisesRegex(RuntimeError, 'byte bound'):
            self.capture.poll(self.pid)

    def test_wrong_pid_is_rejected(self):
        self.file()
        with self.assertRaisesRegex(RuntimeError, 'PID/name'):
            self.capture.poll(self.pid + 1)

    def test_noncanonical_ancestor_is_rejected(self):
        alias = self.root / 'alias'
        alias.symlink_to(self.root, target_is_directory=True)
        target = self.root / 'target'
        target.mkdir(mode=0o700)
        with self.assertRaisesRegex(RuntimeError, 'canonical absolute ancestry'):
            Capture(alias / 'target', self.expected)

    def test_attempt_outside_producer_bound_is_rejected(self):
        (self.root / ('fe2o3-engineering-hsaco-' + str(self.pid) + '-123-32')).mkdir(mode=0o700)
        with self.assertRaisesRegex(RuntimeError, 'attempt bound'):
            self.capture.poll(self.pid)

    def test_nonprivate_parent_is_rejected(self):
        self.file()
        self.child.chmod(0o755)
        with self.assertRaisesRegex(RuntimeError, 'private mode'):
            self.capture.poll(self.pid)

    def test_hard_link_is_rejected(self):
        path = self.file()
        os.link(path, self.root / 'alias')
        with self.assertRaisesRegex(RuntimeError, 'link count'):
            self.capture.poll(self.pid)

    def test_duplicate_matching_child_is_rejected(self):
        self.file()
        (self.root / ('fe2o3-engineering-hsaco-' + str(self.pid) + '-456-0')).mkdir()
        with self.assertRaisesRegex(RuntimeError, 'multiple owned'):
            self.capture.poll(self.pid)

    def test_scan_bound_is_enforced(self):
        for number in range(Capture.MAX_ENTRIES + 1):
            (self.root / ('unrelated-' + str(number))).touch()
        with self.assertRaisesRegex(RuntimeError, 'entry bound'):
            self.capture.poll(self.pid)

    def test_poll_bound_and_closed_reader_are_rejected(self):
        self.capture.MAX_POLLS = 1
        self.capture.poll(self.pid)
        with self.assertRaisesRegex(RuntimeError, 'poll bound'):
            self.capture.poll(self.pid)
        self.capture.close()
        with self.assertRaisesRegex(RuntimeError, 'closed'):
            self.capture.poll(self.pid)

    def test_disappearance_during_read_is_pending(self):
        path = self.file()
        read = os.read

        def disappear(fd, maximum):
            part = read(fd, maximum)
            path.unlink(missing_ok=True)
            return part

        with mock.patch.object(os, 'read', side_effect=disappear):
            self.capture.poll(self.pid)
        self.assertIsNone(self.capture.snapshot)

    def test_capture_survives_producer_cleanup(self):
        path = self.file()
        self.capture.poll(self.pid)
        path.unlink()
        self.child.rmdir()
        self.capture.poll(self.pid)
        self.assertEqual(self.capture.snapshot, self.body)
        self.assertTrue(all(self.capture.joins(self.artifact()).values()))

    def test_read_error_closes_transient_descriptors(self):
        self.file()
        before = len(os.listdir('/proc/self/fd'))
        with mock.patch.object(os, 'read', side_effect=OSError('synthetic read failure')):
            with self.assertRaises(OSError):
                self.capture.poll(self.pid)
        self.assertEqual(len(os.listdir('/proc/self/fd')), before)
        self.capture.close()
        self.assertEqual(len(os.listdir('/proc/self/fd')), before - 1)

    def test_signal_exception_closes_transient_descriptors(self):
        self.file()
        before = len(os.listdir('/proc/self/fd'))
        previous = signal.getsignal(signal.SIGALRM)

        def interrupt(_number, _frame):
            raise TimeoutError('synthetic deadline')

        def read(_fd, _maximum):
            signal.raise_signal(signal.SIGALRM)

        try:
            signal.signal(signal.SIGALRM, interrupt)
            with mock.patch.object(os, 'read', side_effect=read):
                with self.assertRaises(TimeoutError):
                    self.capture.poll(self.pid)
        finally:
            signal.signal(signal.SIGALRM, previous)
        self.assertEqual(len(os.listdir('/proc/self/fd')), before)

    def test_missing_capture_and_changed_joins_are_independent(self):
        self.assertFalse(self.capture.joins(self.artifact())['handoff_matches_prior'])
        self.file()
        self.capture.poll(self.pid)
        baseline = self.artifact()
        for key in ('handoff', 'image', 'descriptor'):
            artifact = copy.deepcopy(baseline)
            if key == 'handoff':
                artifact['value']['compiler_handoff']['sha256'] = '0' * 64
                changed_key = 'handoff_matches_current'
            elif key == 'image':
                artifact['value']['hsaco']['identity']['sha256'] = '0' * 64
                changed_key = 'image_matches_prior'
            else:
                artifact['value']['hsaco']['canonical_descriptor_sha256'] = '0' * 64
                changed_key = 'descriptor_matches_prior'
            joins = self.capture.joins(artifact)
            self.assertFalse(joins[changed_key])
            self.assertTrue(all(value for name, value in joins.items() if name != changed_key))


class ControllerShapeTests(unittest.TestCase):
    def test_old_functions_except_owned_and_main_are_ast_identical(self):
        before = {node.name: node for node in PREVIOUS_AST.body if isinstance(node, ast.FunctionDef)}
        after = {node.name: node for node in CURRENT_AST.body if isinstance(node, ast.FunctionDef)}
        self.assertEqual(set(before), set(after))
        for key in before.keys() - {'owned', 'main'}:
            self.assertEqual(ast.dump(before[key]), ast.dump(after[key]), key)

    def test_owned_only_adds_capture_parameter_and_poll(self):
        before = next(node for node in PREVIOUS_AST.body if isinstance(node, ast.FunctionDef) and node.name == 'owned')
        after = copy.deepcopy(next(node for node in CURRENT_AST.body if isinstance(node, ast.FunctionDef) and node.name == 'owned'))
        self.assertEqual(after.args.args[-1].arg, 'capture')
        after.args.args.pop()
        removed = []

        class RemovePoll(ast.NodeTransformer):
            def visit_Expr(self, node):
                if ast.dump(node) == ast.dump(ast.parse('capture.poll(child.pid)').body[0]):
                    removed.append(node)
                    return None
                return self.generic_visit(node)

        after = RemovePoll().visit(after)
        self.assertEqual(len(removed), 1)
        self.assertEqual(ast.dump(before), ast.dump(after))

    def test_prior_pins_and_resource_limits_are_exact(self):
        self.assertEqual(scope['PRIOR_HANDOFF'], {'byte_len': 288742, 'sha256': '5f52c141f577162cbc3eda8704b173df7035e8c336b8b436c23558f101fe844c'})
        before = {name(node): node for node in PREVIOUS_AST.body if isinstance(node, ast.Assign)}
        after = {name(node): node for node in CURRENT_AST.body if isinstance(node, ast.Assign)}
        for key in before.keys() - {'OUT', 'SCRIPT'}:
            self.assertEqual(ast.dump(before[key]), ast.dump(after[key]), key)
        self.assertEqual(Capture.MAX_BYTES, 1 << 20)
        self.assertEqual(Capture.MAX_POLLS, 7000)
        self.assertEqual(Capture.MAX_ENTRIES, 32)


if __name__ == '__main__':
    class Result(unittest.TextTestResult):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            self.passing_names = []

        def addSuccess(self, test):
            self.passing_names.append(test.id())
            super().addSuccess(test)

    suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
    names = sorted(test.id() for group in suite for test in group)
    result = unittest.TextTestRunner(verbosity=2, resultclass=Result).run(suite)
    unchanged = CURRENT_PATH.read_bytes() == CURRENT_BYTES and PREVIOUS_PATH.read_bytes() == PREVIOUS_BYTES and Path(__file__).read_bytes() == TEST_BYTES
    passed = result.wasSuccessful() and unchanged and len(names) == len(set(names)) == 29
    print(json.dumps(dict(schema='ferric-handoff-capture-tests-v1', passed=passed,
                          names=names, passing_names=sorted(result.passing_names), tests_run=result.testsRun,
                          failures=len(result.failures), errors=len(result.errors), skipped=len(result.skipped),
                          expected_failures=len(result.expectedFailures), unexpected_successes=len(result.unexpectedSuccesses),
                          source_unchanged=unchanged, synthetic_capture_only=True, compiler_qualification=False,
                          gpu_execution=False)))
    sys.exit(0 if passed else 1)
