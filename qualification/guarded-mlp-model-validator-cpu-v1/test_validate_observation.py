"""Synthetic data checks only; no GPU, native executable, or model import."""
import hashlib
import json
from pathlib import Path
import struct
import unittest

import validate_observation as V


def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


def state_control(generation, first=0):
    p = [1, 0, 0, 31] + [1, 48, 1, 16, 64] * 2 + ([0xffffffff] * 4 + [3]) * 2 + [64] * 260
    m = [1, 0, 0, 31] + [1, 96, 96, 1, 64] * 2 + ([0xffffffff] * 8 + [3]) * 2 + [64] * 516
    body = bytearray(struct.pack('<QQ', 1, 2))
    for layer in range(36):
        body.extend(struct.pack('<568I', *(p * 2)))
        body.extend(struct.pack('<1096I', *(m * 2)))
        body.extend(struct.pack('<8I', *(([(generation - 1) // 2 + 1, 0, 1, 0]) * 2)))
        body.extend(struct.pack('<QQQ', 3, 4, 5))
        for rank in range(2):
            write = first + layer * 5 + rank + 5
            body.extend(struct.pack('<QQ', write, write - 2))
    body.extend(struct.pack('<QQQ', 6, 7, 8))
    return bytes(body)


def fixture(ar=False):
    directory = Path('/synthetic/native')
    image = dict(path='/synthetic/image', bytes=4, sha256=[4] * 32)
    config = dict(mode='autoregressive' if ar else 'teacher_forced', device_ids=V.IDS,
                  expected_bundle_id=[1] * 32, expected_model_id=[2] * 32, session=[3] * 32,
                  dispatch_timeout_ms=10000, prefix_image=image, tiles_image=image,
                  images={name: image for name in ('prefix', 'mlp', 'residual', 'tail')})
    request = dict(schema='FerricFiniteGuardedMlpDecodeRequestV1', decode=config,
                   projection_image=image, guarded_image=image)
    scope = dict(bundle_id=[1] * 32, model_id=[2] * 32, session=[3] * 32,
                 pool_identity=8, group_id=9, child_identity=100)
    begin = dict(source_program=dict(sha256=[5] * 32), uploads=dict(sha256=[6] * 32),
                 **{name + '_image': {k: image[k] for k in ('bytes', 'sha256')}
                    for name in ('prefix', 'mlp', 'residual', 'tail')})
    b = dict(schema='FerricGuardedMlpDecodeBootstrapV1', decode=dict(scope=scope, begin=begin,
             mode=config['mode'], device_ids=V.IDS, timeout_ms=10000, registration=[7] * 32,
             prefix_image={k: image[k] for k in ('bytes', 'sha256')},
             tiles_image={k: image[k] for k in ('bytes', 'sha256')}, input_tokens=V.SEED[:1] if ar else list(V.SEED)),
             projection_image={k: image[k] for k in ('bytes', 'sha256')},
             guarded_image={k: image[k] for k in ('bytes', 'sha256')})
    profile = list(V.profile(b)); bodies = {}; frames = []
    chain = hashlib.sha256(b'ferric-prefix284-combined2208-four-transcript-v1\0' + bytes([7] * 32) + bytes(profile)).digest()
    def put(name, raw):
        path = str(directory / name); bodies[path] = raw
        return dict(path=path, **V.part(raw))
    def response(ident, event):
        return dict(schema='FerricGuardedMlpDecodeObservationV1', protocol=1, id=ident,
            device_ids=V.IDS, session=[3] * 32, registration=[7] * 32, profile_sha256=profile,
            event=event, native_closed=ident == 5, gpu_execution=True, **{k: False for k in V.FALSE})
    payload = bytes(V.PAYLOAD_BYTES)
    for i in range(4):
        control = state_control(i + 1, i * 1000)
        token = V.SEED[i] if not ar or i == 0 else 0
        c = dict(status='completed', generation=i + 1, position=i, input_token=token, output_token=0,
                 control=V.part(control), observation=V.part(payload), capture=V.payload(payload)[1])
        chain = hashlib.sha256(chain + struct.pack('<QIIII', i + 1, i, token, 0, V.CONTROL_BYTES)
            + hashlib.sha256(control).digest() + struct.pack('<I', V.PAYLOAD_BYTES)
            + hashlib.sha256(payload).digest()).digest()
        c['chain'] = list(chain)
        r = dict(protocol=1, id=i + 1, device_ids=V.IDS, session=[3] * 32, registration=[7] * 32,
                 profile_sha256=profile, command=dict(op='forward', generation=i + 1, token=token,
                 cache_metadata=[i] + list(range(144)), rotary_bits=[0] * 128))
        frames.append(dict(response=response(i + 1, c), control=put(f'control-{i}.bin', control),
                           observation=put(f'observation-{i}.bin', payload), request=put(f'request-{i}.json', encoded(r))))
    stderr = put('child-stderr.bin', b'')
    total = sum(map(len, bodies.values()))
    o = dict(schema='FerricFiniteGuardedMlpDecodeObservationV1', request=request, child_pid=100,
        registration_sha256=[7] * 32, source_program_sha256=[5] * 32, upload_manifest_sha256=[6] * 32,
        bootstrap=b, profile_sha256=profile, setup_commands=1, completed_forwards=4,
        input_tokens=list(V.SEED) if not ar else [V.SEED[0], 0, 0, 0], observed_output_tokens=[0] * 4,
        page_permutation=list(range(144)), transcript_sha256=list(chain), request_stream_bytes=123,
        response_stream_bytes=456, files=dict(frames=frames, child_stderr=stderr,
            bytes_before_summary=total, summary_bytes=0, total_bytes=total),
        close=response(5, dict(status='closed', completed_forwards=4, transcript_sha256=list(chain))),
        child_exit_zero=True, process_group_absent=True, native_closed=True, gpu_execution=True,
        full_long_workload=False, native_attempts=1, retries=0, **{k: False for k in V.FALSE})
    def serialize():
        for _ in range(10):
            raw = encoded(o)
            if len(raw) == o['files']['summary_bytes']:
                return raw
            o['files']['summary_bytes'] = len(raw)
            o['files']['total_bytes'] = total + len(raw)
        raise AssertionError('summary did not converge')
    return o, bodies, request, directory, serialize


class GuardedObservationTests(unittest.TestCase):
    def test_complete_tf4(self):
        o, bodies, request, directory, serialize = fixture()
        result = V.validate(serialize(), request, directory, lambda pin: bodies[pin['path']])
        self.assertEqual(result['input_tokens'], V.SEED)
        self.assertEqual(result['layer_observations'], 144)
        self.assertEqual(result['rank_guard_observations'], 288)

    def test_complete_own_output_ar4(self):
        o, bodies, request, directory, serialize = fixture(True)
        result = V.validate(serialize(), request, directory, lambda pin: bodies[pin['path']])
        self.assertEqual(result['input_tokens'], [9112, 0, 0, 0])
        self.assertFalse(result['independent_full_model_reference'])

    def test_control_generation_and_extent_refusals(self):
        for generation in range(1, 5):
            raw = state_control(generation)
            V.control(raw, generation, [(0, 0)] * 2)
            for bad in (raw[:-1], raw + b'\0'):
                with self.assertRaises(ValueError): V.control(bad, generation, [(0, 0)] * 2)
            changed = bytearray(raw); struct.pack_into('<I', changed, 16 + (568 + 1096) * 4, 77)
            with self.assertRaises(ValueError): V.control(changed, generation, [(0, 0)] * 2)

    def test_each_terminal_region_and_frontier_refuses(self):
        raw = state_control(1)
        for offset in (16, 16 + 284 * 4, 16 + 568 * 4, 16 + (568 + 548) * 4,
                       16 + (568 + 1096 + 8) * 4 + 24):
            changed = bytearray(raw); struct.pack_into('<I', changed, offset, 0)
            with self.subTest(offset=offset), self.assertRaises(ValueError):
                V.control(changed, 1, [(0, 0)] * 2)
        with self.assertRaises(ValueError): V.control(raw, 1, [(10000, 0)] * 2)

    def test_bf16_lowest_argmax_and_signed_zero(self):
        raw = bytearray(V.PAYLOAD_BYTES)
        struct.pack_into('<H', raw, 37 * 8192, 0x8000)
        self.assertEqual(V.payload(raw)[0], 0)
        for i in (7, 11): struct.pack_into('<H', raw, 37 * 8192 + i * 2, 0x3f80)
        self.assertEqual(V.payload(raw)[0], 7)
        struct.pack_into('<H', raw, 37 * 8192 + 7 * 2, 0xbf80)
        self.assertEqual(V.payload(raw)[0], 11)

    def test_nonfinite_every_payload_partition_refuses(self):
        for index in (0, 36 * 4096, 37 * 4096, 303487):
            for bits in (0x7f80, 0xff80, 0x7fc0):
                raw = bytearray(V.PAYLOAD_BYTES); struct.pack_into('<H', raw, index * 2, bits)
                with self.assertRaises(ValueError): V.payload(raw)

    def test_closed_claims_counters_and_metadata_refuse(self):
        mutations = []
        for key in (*V.FALSE, 'full_long_workload'):
            mutations.append(lambda o, key=key: o.__setitem__(key, True))
        mutations += [lambda o: o.__setitem__('completed_forwards', 3),
            lambda o: o.__setitem__('native_attempts', 2), lambda o: o.__setitem__('retries', 1),
            lambda o: o.__setitem__('child_exit_zero', False), lambda o: o.__setitem__('extra', 1),
            lambda o: o.__setitem__('setup_commands', True), lambda o: o['profile_sha256'].__setitem__(0, 99),
            lambda o: o['page_permutation'].__setitem__(0, 1)]
        for mutate in mutations:
            o, bodies, request, directory, serialize = fixture(); mutate(o)
            with self.assertRaises(ValueError): V.validate(serialize(), request, directory, lambda pin: bodies[pin['path']])

    def test_raw_pin_and_close_chain_refuse(self):
        mutations = [lambda o: o['files']['frames'][0]['control'].__setitem__('path', '/elsewhere/control.bin'),
            lambda o: o['files']['frames'][0]['control']['sha256'].__setitem__(0, 99),
            lambda o: o['files']['frames'][0]['response']['event']['chain'].__setitem__(0, 99),
            lambda o: o['close']['event'].__setitem__('completed_forwards', 3),
            lambda o: o['close'].__setitem__('native_closed', False),
            lambda o: o['observed_output_tokens'].__setitem__(2, 1),
            lambda o: o['input_tokens'].__setitem__(2, 1)]
        for mutate in mutations:
            o, bodies, request, directory, serialize = fixture(); mutate(o)
            with self.assertRaises(ValueError): V.validate(serialize(), request, directory, lambda pin: bodies[pin['path']])

    def test_json_and_integer_types_refuse(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(ValueError): V.parse(raw)
        for value in (True, -1, 1 << 64, 1.0):
            with self.assertRaises(ValueError): V.uint(value)


if __name__ == '__main__':
    unittest.main(verbosity=2)
