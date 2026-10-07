"""Synthetic causal envelope and same-side parity tests; no model or process execution."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import unittest

import validate_causal as V
import validate_readiness as OLD
import test_readiness as T


def fresh_session(value):
    summary, request, bodies, _ = value
    session = [9] * 32
    request['base']['session'] = session
    seq = summary['bootstrap']['sequence']
    seq['scope']['session'] = session
    seq['begin']['scope']['session'] = session
    profile = hashlib.sha256(b'ferric-prefix284-combined2208-closed-long-v2\0' + T.wire(seq)).digest()
    chain = hashlib.sha256(b'ferric-guarded-mlp-long-transcript-v2\0' + profile).digest()
    summary['profile_sha256'] = list(profile)
    path = str(T.DIRECTORY / 'frames.ndjson')
    frames = [json.loads(line) for line in bodies[path].splitlines()]
    for row in frames:
        old_request = T.wire(row['request']); old_frame = T.wire(row)
        row['request']['session'] = session
        row['request']['profile_sha256'] = list(profile)
        row['completion']['chain'] = [0] * 32
        chain = hashlib.sha256(chain + T.wire(row['request']) + T.wire(row['completion'])).digest()
        row['completion']['chain'] = list(chain)
        summary['request_stream_bytes'] += len(T.wire(row['request'])) - len(old_request)
        summary['response_stream_bytes'] += len(T.wire(row)) - len(old_frame)
    old_close = T.wire(summary['close']); old_request = T.wire(summary['close']['request'])
    summary['close']['request']['session'] = session
    summary['close']['request']['profile_sha256'] = list(profile)
    summary['close']['transcript_sha256'] = list(chain)
    summary['transcript_sha256'] = list(chain)
    summary['request_stream_bytes'] += len(T.wire(summary['close']['request'])) - len(old_request)
    summary['response_stream_bytes'] += len(T.wire(summary['close'])) - len(old_close)
    raw = b''.join(T.wire(row) + b'\n' for row in frames)
    summary['files']['bytes_before_summary'] += len(raw) - len(bodies[path])
    bodies[path] = raw; summary['files']['frames'] = T.pin(path, raw)
    return value


def specs(position):
    return [
        ('before_prefix', [('input', 'bf16', 8192), ('cache_metadata', 'u32', 580), ('rotary', 'f32', 512)]),
        ('after_prefix', [('input_normalized', 'bf16', 8192), ('raw_qkv', 'bf16', 6144), ('query', 'bf16', 4096),
            ('used_key', 'bf16', 1024 * (position + 1)), ('used_value', 'bf16', 1024 * (position + 1)),
            ('attention', 'bf16', 4096), ('output_partial', 'f32', 16384)]),
        ('after_first_residual', [('first_residual', 'bf16', 8192)]),
        ('after_mlp', [('post_normalized', 'bf16', 8192), ('gate', 'bf16', 12288),
            ('up', 'bf16', 12288), ('activation', 'bf16', 12288), ('down_partial', 'f32', 16384)]),
        ('after_final_residual', [('final_hidden', 'bf16', 8192)]),
    ]


def fixture():
    value = fresh_session(T.fixture())
    summary, request, bodies, _ = value
    request['schema'] = 'FerricGuardedMlpReadiness40CausalLayerZeroRequestV1'
    summary['schema'] = 'FerricGuardedMlpReadiness40CausalLayerZeroObservationV1'
    pages = summary['page_permutation']; captures = []; payload = bytearray()
    for position in range(6):
        parts = []; raw = bytearray()
        for boundary, roles in specs(position):
            for rank in range(2):
                for role, scalar, count in roles:
                    data = bytes(count)
                    if role == 'cache_metadata': data = struct.pack('<145I', position, *pages)
                    if role in ('used_key', 'used_value'):
                        data = struct.pack('<H', 0x3f80 + rank * 128 + (64 if role == 'used_value' else 0)) * (count // 2)
                    parts.append(dict(boundary=boundary, rank=rank, role=role, scalar=scalar,
                        elements=count // (2 if scalar == 'bf16' else 4), offset=len(raw), bytes=count,
                        source_byte_offset=pages[0] * 16384 if role in ('used_key', 'used_value') else 0,
                        sha256=list(hashlib.sha256(data).digest())))
                    raw.extend(data)
        captures.append(dict(generation=position + 1, position=position, layer=0, parts=parts,
            payload_bytes=len(raw), payload_sha256=list(hashlib.sha256(raw).digest())))
        payload.extend(raw)
    header = dict(schema='FerricReadiness40CausalLayerZeroV1', bootstrap=summary['bootstrap'],
        transcript_sha256=summary['transcript_sha256'], captures=captures,
        payload_bytes=len(payload), payload_sha256=list(hashlib.sha256(payload).digest()),
        cache_layout='used-prefix-token-head-channel-bf16', native_close_confirmed=True,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    install(value, header, payload)
    return value


def unpack(value):
    raw = value[2][str(T.DIRECTORY / 'child-stderr.bin')]
    count = struct.unpack_from('<I', raw, 8)[0]
    return json.loads(raw[16:16 + count]), bytearray(raw[16 + count:])


def install(value, header, payload):
    summary, _, bodies, _ = value
    head = T.wire(header); raw = b'FCAP061\0' + struct.pack('<II', len(head), len(payload)) + head + payload
    path = str(T.DIRECTORY / 'child-stderr.bin')
    summary['files']['bytes_before_summary'] += len(raw) - len(bodies[path])
    bodies[path] = raw; summary['files']['child_stderr'] = T.pin(path, raw)
    summary['causal_layer_zero'] = dict(schema='FerricReadiness40CausalLayerZeroCheckedV1',
        positions=list(range(6)), parts=204, payload_bytes=len(payload),
        payload_sha256=list(hashlib.sha256(payload).digest()), sidecar_bytes=len(raw),
        sidecar_sha256=list(hashlib.sha256(raw).digest()), native_close_confirmed=True,
        numerical_acceptance=False, performance_claim=False)


def repin(header, payload):
    base = 0
    for capture in header['captures']:
        size = capture['payload_bytes']
        for part in capture['parts']:
            data = payload[base + part['offset']:base + part['offset'] + part['bytes']]
            part['sha256'] = list(hashlib.sha256(data).digest())
        capture['payload_sha256'] = list(hashlib.sha256(payload[base:base + size]).digest())
        base += size
    header['payload_sha256'] = list(hashlib.sha256(payload).digest())


def check(value):
    return V.validate(T.summary_bytes(value[0]), value[1], T.DIRECTORY, lambda pin: value[2][pin['path']], value[3])


class CausalTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = fixture()

    def rejected(self, edit):
        value = copy.deepcopy(self.base)
        header, payload = unpack(value)
        edit(header, payload)
        install(value, header, payload)
        with self.assertRaises((ValueError, KeyError, TypeError)):
            check(value)

    def test_complete_six_prefixes_and_all40_chain(self):
        got = check(copy.deepcopy(self.base))
        self.assertEqual(got['causal_layer_zero']['payload_bytes'], 1598256)
        self.assertEqual(got['causal_layer_zero']['parts'], 204)
        self.assertEqual(got['causal_layer_zero']['positions'], list(range(6)))
        self.assertEqual(got['completed_forwards'], 40)
        self.assertFalse(got['numerical_acceptance'])

    def test_old_summaries_and_missing_sidecar_are_refused(self):
        old = T.fixture()
        with self.assertRaises(ValueError): check(old)
        with self.assertRaises(ValueError):
            OLD.validate(T.summary_bytes(self.base[0]), self.base[1], T.DIRECTORY,
                lambda pin: self.base[2][pin['path']], self.base[3])
        for field in ('causal_layer_zero',):
            value = copy.deepcopy(self.base); del value[0][field]
            with self.assertRaises(ValueError): check(value)

    def test_envelope_magic_lengths_truncation_and_extra_bytes(self):
        for mutate in (lambda raw: b'BADMAGIC' + raw[8:], lambda raw: raw[:-1],
                       lambda raw: raw + b'x', lambda raw: raw[:8] + struct.pack('<II', 131073, 1598256) + raw[16:]):
            value = copy.deepcopy(self.base); path = str(T.DIRECTORY / 'child-stderr.bin')
            raw = mutate(value[2][path]); value[2][path] = raw; value[0]['files']['child_stderr'] = T.pin(path, raw)
            with self.assertRaises(ValueError): check(value)

    def test_exact_schema_identity_transcript_and_false_claims(self):
        for edit in (lambda h: h.__setitem__('schema', 'wrong'), lambda h: h.__setitem__('extra', False),
                     lambda h: h['bootstrap']['sequence']['scope'].__setitem__('child_identity', 102),
                     lambda h: h['transcript_sha256'].__setitem__(0, h['transcript_sha256'][0] ^ 1),
                     lambda h: h.__setitem__('native_close_confirmed', False),
                     lambda h: h.__setitem__('numerical_acceptance', True)):
            self.rejected(lambda h, p: edit(h))

    def test_order_rank_role_elements_and_offsets_are_closed(self):
        for field, value in [('boundary', 'after_mlp'), ('rank', True), ('role', 'down_partial'),
                             ('elements', 0), ('offset', 1), ('bytes', 8190), ('source_byte_offset', 1)]:
            self.rejected(lambda h, p: h['captures'][0]['parts'][0].__setitem__(field, value))
        self.rejected(lambda h, p: h['captures'].reverse())
        self.rejected(lambda h, p: h['captures'][0].__setitem__('generation', True))

    def test_payload_snapshot_and_part_hashes_are_checked(self):
        self.rejected(lambda h, p: h['payload_sha256'].__setitem__(0, h['payload_sha256'][0] ^ 1))
        self.rejected(lambda h, p: h['captures'][0]['payload_sha256'].__setitem__(0,
            h['captures'][0]['payload_sha256'][0] ^ 1))
        self.rejected(lambda h, p: h['captures'][0]['parts'][0]['sha256'].__setitem__(0,
            h['captures'][0]['parts'][0]['sha256'][0] ^ 1))
        self.rejected(lambda h, p: p.__setitem__(0, 1))

    def test_nonfinite_scalars_reject_even_with_correct_hashes(self):
        for role, word in [('input', struct.pack('<H', 0x7f80)), ('output_partial', struct.pack('<I', 0x7f800001))]:
            def edit(h, p):
                part = next(r for r in h['captures'][0]['parts'] if r['role'] == role)
                p[part['offset']:part['offset'] + len(word)] = word
                repin(h, p)
            self.rejected(edit)

    def test_metadata_pages_and_physical_cache_offsets_are_checked(self):
        def edit(h, p):
            part = next(r for r in h['captures'][0]['parts'] if r['role'] == 'cache_metadata')
            struct.pack_into('<I', p, part['offset'] + 4, 0); repin(h, p)
        self.rejected(edit)
        self.rejected(lambda h, p: next(r for r in h['captures'][0]['parts'] if r['role'] == 'used_key')
            .__setitem__('source_byte_offset', 0))

    def test_all_prior_kv_prefixes_are_immutable(self):
        for role in ('used_key', 'used_value'):
            def edit(h, p):
                part = next(r for r in h['captures'][5]['parts'] if r['role'] == role and r['rank'] == 1)
                offset = sum(c['payload_bytes'] for c in h['captures'][:5]) + part['offset']
                p[offset] ^= 1; repin(h, p)
            self.rejected(edit)

    def test_final_hidden_rank_selected_and_summary_joins(self):
        for both in (False, True):
            def edit(h, p):
                for part in h['captures'][0]['parts']:
                    if part['role'] == 'final_hidden' and (both or part['rank'] == 1):
                        p[part['offset']] = 1
                repin(h, p)
            self.rejected(edit)
        value = copy.deepcopy(self.base); value[0]['causal_layer_zero']['parts'] = 203
        with self.assertRaises(ValueError): check(value)

    def test_same_side_parity_requires_every_record_and_payload(self):
        old = T.fixture(); current = copy.deepcopy(self.base)
        checked = check(current); old_checked = T.check(old)
        # Paths differ in actual runs. Prefix keys isolate these two complete synthetic readsets.
        old_bodies = {}
        for name, raw in old[2].items():
            old_bodies[name.replace('/synthetic/', '/baseline/')] = raw
        for entry in [old[0]['files']['frames'], old[0]['files']['child_stderr'],
                      *[row['file'] for row in old[0]['files']['captures']]]:
            entry['path'] = entry['path'].replace('/synthetic/', '/baseline/')
        bodies = dict(current[2], **old_bodies)
        read = lambda pin: bodies[pin['path']]
        baseline_raw = T.summary_bytes(old[0]); current_raw = T.summary_bytes(current[0])
        got = V.compare_same_side(current_raw, baseline_raw, checked, old_checked, read)
        self.assertTrue(got['all40_records_equal']); self.assertTrue(got['all4_payloads_byte_equal'])
        self.assertEqual(len(got['records']), 40); self.assertFalse(got['numerical_acceptance'])
        for position in (0, 5, 16, 39, 31):
            bad = copy.deepcopy(current)
            T.change_frame(bad, position, lambda f: f['completion'].__setitem__('output_token', 4))
            with self.assertRaises(ValueError):
                V.compare_same_side(T.summary_bytes(bad[0]), baseline_raw, checked, old_checked,
                    lambda pin: bad[2].get(pin['path'], bodies.get(pin['path'])))
        bad = copy.deepcopy(current)
        path = str(T.DIRECTORY / 'capture-5.bin'); data = bytearray(bad[2][path]); data[242824] = 1
        bad[2][path] = bytes(data); bad[0]['files']['captures'][1]['file'] = T.pin(path, data)
        with self.assertRaises(ValueError):
            V.compare_same_side(T.summary_bytes(bad[0]), baseline_raw, checked, old_checked,
                lambda pin: bad[2].get(pin['path'], bodies.get(pin['path'])))

    def test_parity_rejects_same_session_model_drift_and_authority(self):
        old = T.fixture(); current = copy.deepcopy(self.base)
        checked = check(current); baseline_checked = T.check(old)
        for edit in (lambda x: x[0]['request']['base'].__setitem__('session', [3] * 32),
                     lambda x: x[0]['request']['base'].__setitem__('source', '/substituted'),
                     lambda x: x[1].__setitem__('numerical_acceptance', True)):
            value = [copy.deepcopy(current[0]), copy.deepcopy(checked)]; edit(value)
            with self.assertRaises(ValueError):
                V.compare_same_side(T.summary_bytes(value[0]), T.summary_bytes(old[0]), value[1],
                    baseline_checked, lambda pin: current[2][pin['path']])


if __name__ == '__main__':
    unittest.main()
