"""Small restoration policy fixtures; no Cargo, downloads, or GPU calls."""
import copy
import hashlib
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('registry_restore_fixture', Path(__file__).with_name('restore_registry.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)


def expected(name, body):
    return dict(path=str(R.REGISTRY / 'src/index/pkg-1' / name),
                bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


class RestoreTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        # The production entry requires UID9661; pure fixtures use this user's temp files.
        def fixture_pin(path, retain=False):
            path = Path(path)
            body = path.read_bytes()
            return dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest()), body if retain else b''
        self.patch = mock.patch.object(R, 'pin', side_effect=fixture_pin)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        self.rows = {'src/lib.rs': expected('src/lib.rs', b'body'), '.cargo-ok': expected('.cargo-ok', b'marker')}

    def archive(self, entries):
        path = self.root / 'archive.crate'
        with tarfile.open(path, 'w:gz', format=tarfile.GNU_FORMAT) as archive:
            for name, body, kind in entries:
                entry = tarfile.TarInfo(name)
                entry.type = kind
                entry.size = len(body) if kind == tarfile.REGTYPE else 0
                entry.linkname = '../escape' if kind in (tarfile.SYMTYPE, tarfile.LNKTYPE) else ''
                archive.addfile(entry, io.BytesIO(body) if entry.isfile() else None)
        return path

    def stage(self, entries, rows=None, marker=b'marker', target='stage'):
        archive = self.archive(entries)
        R.stage_package(archive, R.pin(archive)[0], 'pkg-1', rows or self.rows, self.root / target, marker)

    def test_exact_archive_and_independent_marker_stage_without_touching_destination(self):
        self.stage([('pkg-1/src/lib.rs', b'body', tarfile.REGTYPE)])
        self.assertEqual((self.root / 'stage/src/lib.rs').read_bytes(), b'body')
        self.assertEqual((self.root / 'stage/.cargo-ok').read_bytes(), b'marker')
        self.assertFalse((self.root / 'installed').exists())

    def test_path_traversal_links_special_members_and_wrong_root_refused(self):
        for index, (name, kind) in enumerate([('pkg-1/../escape', tarfile.REGTYPE),
                ('/pkg-1/file', tarfile.REGTYPE), ('other/src/lib.rs', tarfile.REGTYPE),
                ('pkg-1/link', tarfile.SYMTYPE), ('pkg-1/hard', tarfile.LNKTYPE),
                ('pkg-1/fifo', tarfile.FIFOTYPE)]):
            with self.subTest(name=name, kind=kind), self.assertRaises(RuntimeError):
                self.stage([(name, b'body', kind)], target='bad' + str(index))

    def test_duplicate_extra_missing_and_wrong_body_refused(self):
        valid = ('pkg-1/src/lib.rs', b'body', tarfile.REGTYPE)
        cases = [[valid, valid], [valid, ('pkg-1/extra', b'x', tarfile.REGTYPE)], [],
                 [('pkg-1/src/lib.rs', b'bad!', tarfile.REGTYPE)]]
        for index, entries in enumerate(cases):
            with self.subTest(index=index), self.assertRaises(RuntimeError):
                self.stage(entries, target='bad' + str(index))

    def test_marker_and_archive_pin_mismatch_refused(self):
        entry = [('pkg-1/src/lib.rs', b'body', tarfile.REGTYPE)]
        with self.assertRaises(RuntimeError):
            self.stage(entry, marker=b'wrong')
        archive = self.archive(entry)
        wrong = dict(R.pin(archive)[0], sha256='0' * 64)
        with self.assertRaises(RuntimeError):
            R.stage_package(archive, wrong, 'pkg-1', self.rows, self.root / 'unused', b'marker')
        self.assertFalse((self.root / 'unused').exists())

    def test_staged_extra_body_is_refused_before_commit(self):
        self.stage([('pkg-1/src/lib.rs', b'body', tarfile.REGTYPE)])
        (self.root / 'stage/extra').write_bytes(b'not-in-before')
        with self.assertRaises(RuntimeError):
            R.verify_stage(self.root / 'stage', self.rows)

    def test_no_replace_never_overwrites_existing_package(self):
        source, destination = self.root / 'source', self.root / 'destination'
        source.mkdir()
        destination.mkdir()
        (destination / 'existing').write_bytes(b'preserve')
        with self.assertRaises(FileExistsError):
            R.rename_noreplace(source, destination)
        self.assertTrue(source.is_dir())
        self.assertEqual((destination / 'existing').read_bytes(), b'preserve')
        fresh = self.root / 'fresh'
        R.rename_noreplace(source, fresh)
        self.assertFalse(source.exists())
        self.assertTrue(fresh.is_dir())

    def test_whole_missing_package_only_and_unchanged_archives(self):
        before = {s: {} for s in R.SECTIONS}
        before['src'] = {'index/pkg-1/' + n: v for n, v in self.rows.items()}
        before['cache']['index/pkg-1.crate'] = dict(path=str(R.REGISTRY / 'cache/index/pkg-1.crate'), bytes=1, sha256='a' * 64)
        after = copy.deepcopy(before)
        after['src'] = {}
        self.assertEqual(R.missing_packages(before, after), {'index/pkg-1': self.rows})
        partial = copy.deepcopy(after)
        partial['src']['index/pkg-1/.cargo-ok'] = self.rows['.cargo-ok']
        with self.assertRaises(RuntimeError):
            R.missing_packages(before, partial)
        del after['cache']['index/pkg-1.crate']
        with self.assertRaises(RuntimeError):
            R.missing_packages(before, after)

    def test_relative_path_grammar_rejects_normalization_and_empty_paths(self):
        for value in ('', '.', '..', '../x', '/x', 'x//y', 'x/./y', 'x\\y', 'x\x00y'):
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                R.relative(value)
        self.assertEqual(R.relative('src/lib.rs'), ('src', 'lib.rs'))


if __name__ == '__main__':
    unittest.main()
