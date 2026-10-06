"""Synthetic capture-boundary tests; no model arithmetic or native invocation."""
import copy
import hashlib
import json
import struct
import unittest

import run_model_gpu as R
import validate_observation as V


def encode(value):
    return json.dumps(value, separators=(',', ':'), allow_nan=False).encode()


def pin(path, raw):
    return dict(path=path, bytes=len(raw), sha256=list(hashlib.sha256(raw).digest()))


def fixture():
    metadata = [0, 3] + [page for page in range(144) if page != 3]
    rotary = [0x3f800000] * 128
    request = dict(id=1, command=dict(generation=1, cache_metadata=metadata, rotary_bits=rotary))
    request_raw = encode(request)
    observation = bytes(V.PAYLOAD_BYTES)
    parts = []
    payload = bytearray()
    layout = (
        ('before_prefix', [('input', 'bf16', 4096), ('cache_metadata', 'u32', 145), ('rotary', 'f32', 128)]),
        ('after_prefix', [('input_normalized', 'bf16', 4096), ('raw_qkv', 'bf16', 3072),
                          ('query', 'bf16', 2048), ('current_key', 'bf16', 512), ('current_value', 'bf16', 512),
                          ('attention', 'bf16', 2048), ('output_partial', 'f32', 4096)]),
        ('after_first_residual', [('first_residual', 'bf16', 4096)]),
        ('after_mlp', [('post_normalized', 'bf16', 4096), ('gate', 'bf16', 6144),
                       ('up', 'bf16', 6144), ('activation', 'bf16', 6144), ('down_partial', 'f32', 4096)]),
        ('after_final_residual', [('final_hidden', 'bf16', 4096)]),
    )
    for boundary, roles in layout:
        for rank in (0, 1):
            for role, scalar, elements in roles:
                size = elements * (2 if scalar == 'bf16' else 4)
                data = (struct.pack('<145I', *metadata) if role == 'cache_metadata' else
                        struct.pack('<128I', *rotary) if role == 'rotary' else bytes(size))
                parts.append(dict(boundary=boundary, rank=rank, role=role, scalar=scalar,
                    elements=elements, offset=len(payload), bytes=size,
                    source_byte_offset=49152 if role in ('current_key', 'current_value') else 0,
                    sha256=list(hashlib.sha256(data).digest())))
                payload.extend(data)
    assert len(parts) == 34 and len(payload) == 256136
    capture = dict(schema='FerricFiniteLayerZeroCaptureV1', generation=1, position=0, layer=0,
        parts=parts, payload_bytes=len(payload), payload_sha256=list(hashlib.sha256(payload).digest()),
        payload=list(payload), full_cache_capture=False, native_close_confirmed=True,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    envelope = dict(schema='FerricFiniteGuardedMlpLayerZeroCaptureV1', profile_sha256=[7] * 32,
        registration_sha256=[8] * 32, session=[9] * 32, device_ids=list(R.IDS), completed_forwards=4,
        native_closed=True, sampling='prefix-boundaries-and-post-paired-retained', capture=capture,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    summary = dict(profile_sha256=[7] * 32, registration_sha256=[8] * 32,
        request=dict(decode=dict(session=[9] * 32)), files=dict(frames=[dict(
            request=pin('/synthetic/request.json', request_raw),
            observation=pin('/synthetic/observation.bin', observation))]))
    return envelope, summary, {'/synthetic/request.json': request_raw, '/synthetic/observation.bin': observation}


def check(data):
    envelope, summary, bodies = copy.deepcopy(data)
    raw = encode(envelope)
    summary['files']['child_stderr'] = pin('/synthetic/child.stderr', raw)
    bodies['/synthetic/child.stderr'] = raw
    def read(expected):
        body = bodies[expected['path']]
        assert len(body) == expected['bytes'] and hashlib.sha256(body).hexdigest() == expected['sha256']
        return body
    return R.capture_admission(encode(summary), read, V)


def replace_part(data, role, raw):
    capture = data[0]['capture']
    part = next(row for row in capture['parts'] if row['role'] == role)
    assert len(raw) == part['bytes']
    capture['payload'][part['offset']:part['offset'] + part['bytes']] = list(raw)
    part['sha256'] = list(hashlib.sha256(raw).digest())
    capture['payload_sha256'] = list(hashlib.sha256(bytes(capture['payload'])).digest())


class CaptureTests(unittest.TestCase):
    def refuse(self, data):
        with self.assertRaises((RuntimeError, ValueError)):
            check(data)

    def test_valid_closed_envelope(self):
        result = check(fixture())
        self.assertEqual((result['parts'], result['payload_bytes']), (34, 256136))
        self.assertTrue(result['native_close_confirmed'])
        self.assertTrue(result['same_run_request_and_layer_output_joined'])
        for key in ('independent_framework_comparison', 'numerical_acceptance', 'performance_claim', 'production_authority'):
            self.assertIs(result[key], False)

    def test_stage_order_extent_and_source_offset_refuse(self):
        data = fixture(); parts = data[0]['capture']['parts']; parts[0], parts[3] = parts[3], parts[0]
        self.refuse(data)
        for key, value in [('bytes', 8190), ('offset', 2), ('elements', 4095), ('rank', True)]:
            data = fixture(); data[0]['capture']['parts'][0][key] = value; self.refuse(data)
        data = fixture()
        next(p for p in data[0]['capture']['parts'] if p['role'] == 'current_key')['source_byte_offset'] = 0
        self.refuse(data)

    def test_whole_and_part_hashes_refuse(self):
        for target in ('payload_sha256', 'part'):
            data = fixture(); capture = data[0]['capture']
            digest = capture['payload_sha256'] if target != 'part' else capture['parts'][0]['sha256']
            digest[0] ^= 1
            self.refuse(data)

    def test_nonfinite_bf16_and_f32_refuse(self):
        for role, bits, width, size in [('input', 0x7f80, 2, 8192), ('down_partial', 0x7fc00000, 4, 16384)]:
            data = fixture(); replace_part(data, role, bits.to_bytes(width, 'little') + bytes(size - width))
            self.refuse(data)

    def test_device_profile_registration_and_session_refuse(self):
        for key, value in [('device_ids', list(reversed(R.IDS))), ('device_ids', [39903, 22482]),
                           ('profile_sha256', [0] * 32), ('registration_sha256', [0] * 32),
                           ('session', [0] * 32), ('session', [9] * 16), ('profile_sha256', [True] * 32)]:
            data = fixture(); data[0][key] = value; self.refuse(data)

    def test_metadata_and_rotary_request_joins_refuse(self):
        for role, size in [('cache_metadata', 580), ('rotary', 512)]:
            data = fixture(); replace_part(data, role, bytes(size)); self.refuse(data)

    def test_final_hidden_observation_join_refuses(self):
        data = fixture(); replace_part(data, 'final_hidden', b'\x80\x3f' + bytes(8190)); self.refuse(data)

    def test_unknown_claims_and_unclosed_scope_refuse(self):
        for inner, key, value in [(False, 'extra', 0), (True, 'extra', 0),
                (False, 'native_closed', False), (True, 'native_close_confirmed', False),
                (False, 'completed_forwards', True), (True, 'generation', True), (True, 'position', 1),
                (True, 'layer', 1), (True, 'full_cache_capture', True),
                (False, 'performance_claim', True), (True, 'numerical_acceptance', True)]:
            data = fixture(); (data[0]['capture'] if inner else data[0])[key] = value; self.refuse(data)


if __name__ == '__main__':
    unittest.main(verbosity=2)
