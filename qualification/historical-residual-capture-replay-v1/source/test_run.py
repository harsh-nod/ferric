"""Pure policy/layout tests; no historical GPU or model result is manufactured."""
import copy
import hashlib
import io
from pathlib import Path
import tempfile
import unittest

import run as R


class Layout:
    STAGES = R.STAGES
    CAPTURE_BYTES = R.CAPTURE_BYTES

    @staticmethod
    def capture(raw, _input):
        if len(raw) != R.CAPTURE_BYTES:
            raise ValueError('test capture extent')
        rows, offset = [], 0
        for _ in range(2):
            for _name, size, _width in R.STAGES:
                rows.append(raw[offset:offset + size])
                offset += size
        return rows


def capture():
    return b''.join(bytes([index + 1]) * size
                    for index, (_name, size, _width) in enumerate(R.STAGES * 2))


def upload_records():
    registration = dict(globals=[dict(kind='token_embedding', buffer=dict(
        rank=0, id=185, elements=151936 * 4096, element_bytes=2))])
    record = dict(key=dict(kind='source', rank=0, id=185), bytes=R.TENSOR_BYTES,
                  sha256=list(bytes.fromhex(R.TENSOR_SHA)))
    return registration, dict(version=1, uploads=[record], tail={})


class ReplayPolicyTests(unittest.TestCase):
    def test_reader_records_original_and_transport_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'payload'
            path.write_bytes(b'payload')
            reader = R.Reader(directory)
            pin = dict(path=str(R.E / 'payload'), bytes=7, sha256=R.digest(b'payload'))
            self.assertEqual(reader.original(pin), b'payload')
            record = reader.records[str(path)]
            self.assertEqual(record['original_path'], str(R.E / 'payload'))
            self.assertEqual(record['path'], str(path))
            reader.recheck()

    def test_reader_changed_or_wrong_sized_bytes_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'payload'
            path.write_bytes(b'original')
            reader = R.Reader(directory)
            reader.read(path, (8, R.digest(b'original')))
            path.write_bytes(b'changed!')
            with self.assertRaisesRegex(ValueError, 'pinned bytes'):
                reader.recheck()
            with self.assertRaises(ValueError):
                reader.read(path, (7, R.digest(b'changed!')))

    def test_reader_escape_symlink_and_relative_paths_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            path, link = Path(directory) / 'payload', Path(directory) / 'link'
            path.write_bytes(b'x')
            link.symlink_to(path)
            reader = R.Reader(directory)
            for name in (str(R.E / '../payload'), '/tmp/payload'):
                with self.assertRaises(ValueError):
                    reader.original(dict(path=name, bytes=1, sha256=R.digest(b'x')))
            for bad in (link, Path('payload')):
                with self.assertRaises(ValueError):
                    reader.read(bad, (1, R.digest(b'x')))

    def test_reader_conflicting_origin_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'payload'
            path.write_bytes(b'x')
            reader = R.Reader(directory)
            reader.read(path, (1, R.digest(b'x')), 'first')
            with self.assertRaisesRegex(ValueError, 'conflicting'):
                reader.read(path, (1, R.digest(b'x')), 'second')

    def test_rust_pin_digest_is_exact_unsigned_bytes(self):
        good = dict(path='/recorded', bytes=4, sha256=list(range(32)))
        self.assertEqual(R.rust_pin(good)['sha256'], bytes(range(32)).hex())
        for wrong in ([True] + list(range(1, 32)), [256] + list(range(1, 32)), [0] * 31):
            with self.assertRaises(ValueError):
                R.rust_pin(dict(good, sha256=wrong))
        with self.assertRaises(ValueError):
            R.rust_pin(dict(good, extra=True))

    def test_embedding_upload_binds_registered_source_and_whole_tensor(self):
        registration, uploads = upload_records()
        self.assertEqual(R.embedding_upload(registration, uploads), uploads['uploads'][0])
        for mutate in (
            lambda r, u: r['globals'][0]['buffer'].update(rank=1),
            lambda r, u: r['globals'][0]['buffer'].update(rank=False),
            lambda r, u: r['globals'][0]['buffer'].update(id=186),
            lambda r, u: u['uploads'][0].update(bytes=8192),
            lambda r, u: u['uploads'][0].update(sha256=[0] * 32),
        ):
            r, u = copy.deepcopy(registration), copy.deepcopy(uploads)
            mutate(r, u)
            with self.assertRaises(ValueError):
                R.embedding_upload(r, u)

    def test_duplicate_embedding_registration_or_upload_refuses(self):
        registration, uploads = upload_records()
        for target in ('globals', 'uploads'):
            r, u = copy.deepcopy(registration), copy.deepcopy(uploads)
            rows = r['globals'] if target == 'globals' else u['uploads']
            rows.append(copy.deepcopy(rows[0]))
            with self.assertRaises(ValueError):
                R.embedding_upload(r, u)

    def test_tensor_header_authenticates_dtype_shape_and_bounded_offsets(self):
        tensor = dict(dtype='BF16', shape=[151936, 4096], data_offsets=[0, R.TENSOR_BYTES])
        header = {'model.embed_tokens.weight': tensor}
        self.assertEqual(R.tensor_span(header, 9328, R.MODEL[0]), (9336, R.TENSOR_BYTES))
        for changed in (dict(tensor, dtype='F16'), dict(tensor, shape=[4096, 151936]),
                        dict(tensor, data_offsets=[True, R.TENSOR_BYTES]),
                        dict(tensor, data_offsets=[0, R.TENSOR_BYTES - 1]),
                        dict(tensor, data_offsets=[R.MODEL[0], R.MODEL[0] + R.TENSOR_BYTES])):
            with self.assertRaises(ValueError):
                R.tensor_span({'model.embed_tokens.weight': changed}, 9328, R.MODEL[0])

    def test_streaming_span_hashes_complete_shard_tensor_and_selected_row(self):
        raw = bytes(range(97))
        for chunk in (1, 3, 8, 31, 128):
            size, whole, tensor, row = R.stream_span(io.BytesIO(raw), 11, 71, 29, 17, chunk)
            self.assertEqual((size, whole, tensor, row),
                (len(raw), R.digest(raw), R.digest(raw[11:82]), raw[29:46]))

    def test_truncated_tensor_or_row_escape_refuses(self):
        for first, size, row_first, row_size in ((0, 12, 0, 4), (0, 8, 7, 2), (0, 8, -1, 2)):
            with self.assertRaises(ValueError):
                R.stream_span(io.BytesIO(b'12345678'), first, size, row_first, row_size)

    def test_all28_capture_offsets_and_selected_rank_pairs(self):
        raw = capture()
        pairs, pins = R.residual_rows(raw, Layout, {})
        self.assertEqual(len(pins), 28)
        expected = {'output-partial': 4741120, 'first-residual': 4757504,
                    'down-partial': 4810752, 'final-hidden': 4827136}
        for name, offset in expected.items():
            rows = [pin for pin in pins if pin['stage'] == name]
            self.assertEqual([pin['offset'] for pin in rows], [offset, offset + 4835328])
            for rank, pin in enumerate(rows):
                self.assertEqual(pairs[name][rank], raw[pin['offset']:pin['offset'] + pin['bytes']])
                self.assertEqual(pin['sha256'], R.digest(pairs[name][rank]))

    def test_wrong_capture_extent_or_stage_contract_refuses(self):
        with self.assertRaises(ValueError):
            R.residual_rows(capture()[:-1], Layout, {})
        class Wrong(Layout):
            STAGES = Layout.STAGES[::-1]
        with self.assertRaisesRegex(ValueError, 'capture contract'):
            R.residual_rows(capture(), Wrong, {})

    def test_changed_slice_order_cannot_be_claimed_as_original_offset(self):
        class Swapped(Layout):
            @staticmethod
            def capture(raw, input_):
                rows = Layout.capture(raw, input_)
                rows[6], rows[12] = rows[12], rows[6]
                return rows
        with self.assertRaisesRegex(ValueError, 'ordered capture offset'):
            R.residual_rows(capture(), Swapped, {})

    def test_changed_executable_helper_refuses_before_import(self):
        with tempfile.TemporaryDirectory() as directory:
            path, marker = Path(directory) / 'helper.py', Path(directory) / 'executed'
            path.write_text(f'open({str(marker)!r}, "w").write("bad")\n')
            with self.assertRaises(ValueError):
                R.module(R.Reader(directory), path, R.VALIDATOR, 'must_not_execute')
            self.assertFalse(marker.exists())


if __name__ == '__main__':
    unittest.main(verbosity=2)
