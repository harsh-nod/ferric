"""Focused source-inventory regressions; execution is root-owned."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('provider_cpu_v2', Path(__file__).with_name('run_cpu.py'))
M = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(M)


class SourceScanTests(unittest.TestCase):
    def scan(self, root, packed):
        return {str(path.relative_to(root)): M.pin(path) for path in M.files_below(root, packed)}

    def test_external_cc_target_module_is_included_and_hashed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'cc-1.4.4'
            target = root / 'src/target'
            target.mkdir(parents=True)
            expected = {}
            for name in ('apple.rs', 'generated.rs', 'llvm.rs', 'parser.rs'):
                body = ('// synthetic ' + name + '\n').encode()
                (target / name).write_bytes(body)
                expected['src/target/' + name] = (len(body), hashlib.sha256(body).hexdigest())
            rows = self.scan(root, False)
            self.assertEqual({name: (pin['bytes'], pin['sha256']) for name, pin in rows.items()}, expected)
            self.assertEqual({pin['path'] for pin in rows.values()}, {str(target / name) for name in
                ('apple.rs', 'generated.rs', 'llvm.rs', 'parser.rs')})

    def test_packed_target_and_git_directories_still_refused(self):
        for name in ('target', '.git'):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                (root / name).mkdir()
                (root / name / 'body').write_bytes(b'unchanged packed refusal')
                with self.assertRaisesRegex(RuntimeError, 'source pack contains Git/cache directory'):
                    self.scan(root, True)

    def test_external_git_metadata_remains_included(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / '.git').mkdir()
            (root / '.git/HEAD').write_bytes(b'ref: refs/heads/test\n')
            self.assertEqual(set(self.scan(root, False)), {'.git/HEAD'})

    def test_external_directory_alias_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'source'
            root.mkdir()
            outside = Path(tmp) / 'outside'
            outside.mkdir()
            (outside / 'body.rs').write_bytes(b'not ordinary source custody')
            (root / 'target').symlink_to(outside, target_is_directory=True)
            with self.assertRaisesRegex(RuntimeError, 'source directory alias'):
                self.scan(root, False)

    def test_external_file_alias_refused_during_hashing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'source'
            (root / 'src/target').mkdir(parents=True)
            outside = Path(tmp) / 'outside.rs'
            outside.write_bytes(b'not ordinary source custody')
            (root / 'src/target/apple.rs').symlink_to(outside)
            with self.assertRaisesRegex(RuntimeError, 'not a bounded ordinary file'):
                self.scan(root, False)


if __name__ == '__main__':
    unittest.main(verbosity=2)
