"""Pure adapter regressions; runner supplies authenticated immutable helpers."""
import copy
import hashlib
import math
import struct
import unittest

C = L = D = A = None


def fixture():
    framework = {}
    for ordinal, (name, shape) in enumerate(L.SHAPES.items()):
        count = math.prod(shape)
        framework[name] = b''.join(struct.pack('<H', 0x3f00 + (ordinal * 3 + i) % 100)
                                   for i in range(count))
    parts = []
    for rank in (0, 1):
        q = framework['q-projection'][rank * 4096:(rank + 1) * 4096]
        k = framework['k-projection'][rank * 1024:(rank + 1) * 1024]
        v = framework['v-projection'][rank * 1024:(rank + 1) * 1024]
        parts.append(dict(input=framework['embedding'], input_normalized=framework['input-norm'],
            raw_qkv=q + k + v, query=framework['rotary-q'][rank * 4096:(rank + 1) * 4096],
            current_key=framework['cache-key'][rank * 1024:(rank + 1) * 1024],
            current_value=framework['cache-value'][rank * 1024:(rank + 1) * 1024],
            attention=framework['attention-output'][rank * 4096:(rank + 1) * 4096],
            first_residual=framework['first-residual'], post_normalized=framework['post-norm'],
            gate=framework['gate'][rank * 12288:(rank + 1) * 12288],
            up=framework['up'][rank * 12288:(rank + 1) * 12288],
            activation=framework['product'][rank * 12288:(rank + 1) * 12288],
            final_hidden=framework['layer0-hidden'], cache_metadata=b'\0' * 580, rotary=b'\0' * 512,
            output_partial=struct.pack('<f', 3.0) * 4096,
            down_partial=struct.pack('<f', 7.0 + rank) * 4096))
    payload = bytearray()
    rows = []
    for boundary, specs in A.PARTS:
        for rank in (0, 1):
            for role, scalar, size in specs:
                raw = parts[rank][role]
                assert len(raw) == size
                rows.append(dict(boundary=boundary, rank=rank, role=role, scalar=scalar,
                    elements=size // (2 if scalar == 'bf16' else 4), offset=len(payload), bytes=size,
                    source_byte_offset=0, sha256=list(hashlib.sha256(raw).digest())))
                payload.extend(raw)
    envelope = dict(capture=dict(payload=list(payload), payload_bytes=len(payload),
        payload_sha256=list(hashlib.sha256(payload).digest()), parts=rows))
    return framework, parts, envelope


class AdapterTests(unittest.TestCase):
    def test_complete_partition_preserves_dedicated_down_and_raw_pins(self):
        _framework, expected, envelope = fixture()
        parts, excluded = A.split_capture(C, envelope)
        self.assertEqual(parts, expected)
        self.assertEqual(len(envelope['capture']['parts']), 34)
        self.assertEqual(len(excluded), 4)
        self.assertNotEqual(parts[0]['down_partial'], parts[0]['output_partial'])
        self.assertEqual(excluded[-1]['sha256'], C.sha(parts[1]['down_partial']))

    def test_embedding_and_all_rank_shards_use_unchanged_metrics(self):
        framework, parts, _envelope = fixture()
        rows, earliest = A.comparisons(C, L, D, framework, parts)
        self.assertEqual(len(rows), 26)
        self.assertIsNone(earliest)
        self.assertTrue(all(row['byte_equal'] for row in rows))
        detail = [item for row in rows for item in row.get('components', [])]
        self.assertEqual(len(detail), 6)
        self.assertTrue(all(row['byte_equal'] for row in detail))

    def test_first_divergence_is_observation_order_not_causal_claim(self):
        framework, parts, _envelope = fixture()
        parts[1]['input_normalized'] = b'\0\0' + parts[1]['input_normalized'][2:]
        parts[0]['gate'] = b'\0\0' + parts[0]['gate'][2:]
        _rows, earliest = A.comparisons(C, L, D, framework, parts)
        self.assertEqual(earliest, dict(stage='input_normalized', observable_order=1, ranks=[1]))

    def test_fp32_partials_are_never_cast_or_compared_to_full_bf16(self):
        framework, parts, _envelope = fixture()
        rows, first = A.comparisons(C, L, D, framework, parts)
        parts[0]['output_partial'] = struct.pack('<f', 1000.0) * 4096
        parts[1]['down_partial'] = struct.pack('<f', -1000.0) * 4096
        self.assertEqual((rows, first), A.comparisons(C, L, D, framework, parts))
        self.assertNotIn('output_partial', A.ORDER)
        self.assertNotIn('down_partial', A.ORDER)

    def test_missing_reordered_extent_and_digest_mutations_refuse(self):
        _framework, _parts, original = fixture()
        for mutate in (
            lambda c: c['parts'].pop(),
            lambda c: c['parts'].__setitem__(0, c['parts'][1]),
            lambda c: c['parts'][0].__setitem__('bytes', 8190),
            lambda c: c['parts'][0].__setitem__('sha256', [0] * 32),
            lambda c: c['payload'].__setitem__(0, c['payload'][0] ^ 1),
        ):
            value = copy.deepcopy(original)
            mutate(value['capture'])
            with self.assertRaises((ValueError, RuntimeError)):
                A.split_capture(C, value)

    def test_failed_unverified_and_acceptance_receipts_refuse(self):
        value = dict(schema='ferric-guarded-mlp-model-stage-capture-gpu-v1', mode='ar4',
            passed=True, errors=[], capture_requested=True, capture_verified=True,
            native_attempts=1, retries=0, gpu_execution=True, gpu_execution_confirmed=True,
            native_spawn_observed=True, **{key: False for key in A.FALSE})
        A.terminal_contract(C, value)
        for key, changed in [('passed', False), ('capture_verified', False), ('mode', 'tf4'),
                             ('native_attempts', 2), ('numerical_acceptance', True)]:
            mutated = dict(value, **{key: changed})
            with self.assertRaises((ValueError, RuntimeError)):
                A.terminal_contract(C, mutated)
