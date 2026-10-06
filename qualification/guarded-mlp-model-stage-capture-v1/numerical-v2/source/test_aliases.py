"""Small source-injected reader regressions; no historical body is modified."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest import mock

C = R = OUT = None


def fixture(root):
    raw = b'unchanged original framework bytes\n'
    original = dict(path='/unavailable-original-framework/copy.bin', bytes=len(raw),
                    sha256=hashlib.sha256(raw).hexdigest())
    (root / 'copy.bin').write_bytes(raw)
    (root / 'framework-aliases.json').write_bytes(b'fixture-only\n')
    reader = R()
    reader.c = C
    reader.aliases = {original['path']: dict(original=original, relative='copy.bin')}
    return reader, original, raw


class AliasTests(unittest.TestCase):
    def test_original_pin_and_physical_path_remain_separate_without_fallback(self):
        with tempfile.TemporaryDirectory(dir=OUT) as directory:
            root = Path(directory)
            with mock.patch.dict(R.read_path.__globals__, {'ALIAS_ROOT': root}):
                reader, original, raw = fixture(root)
                self.assertEqual(reader.read(original), raw)
                self.assertEqual(reader.pins, {original['path']: original})
                self.assertEqual(reader.physical_pins[original['path']], dict(original, path=str(root / 'copy.bin')))
                reader.recheck()
                with self.assertRaises(RuntimeError):
                    reader.read(dict(original, sha256='0' * 64))
                with self.assertRaises(FileNotFoundError):
                    reader.read(dict(original, path='/unavailable-original-framework/unknown.bin'))
                (root / 'extra').write_bytes(b'extra')
                with self.assertRaisesRegex(RuntimeError, 'exact original case members'):
                    reader.recheck()

    def test_alias_posthash_and_symlink_mutations_refuse(self):
        with tempfile.TemporaryDirectory(dir=OUT) as directory:
            root = Path(directory)
            with mock.patch.dict(R.read_path.__globals__, {'ALIAS_ROOT': root}):
                reader, original, raw = fixture(root)
                reader.read(original)
                (root / 'copy.bin').write_bytes(raw.replace(b'unchanged', b'corrupted'))
                with self.assertRaisesRegex(RuntimeError, 'actual input pin mismatch'):
                    reader.recheck()
                (root / 'copy.bin').unlink()
                (root / 'elsewhere').write_bytes(raw)
                (root / 'copy.bin').symlink_to(root / 'elsewhere')
                with self.assertRaisesRegex(RuntimeError, 'canonical regular input path'):
                    reader.read(original)
