"""Closed captured-operand tests; no project, GPU or framework work."""
import copy
import hashlib
import json
import struct
import unittest

import r2 as R

STAGES = (
    ('before_prefix', (('input', 'bf16', 8192), ('cache_metadata', 'u32', 580), ('rotary', 'f32', 512))),
    ('after_prefix', (('input_normalized', 'bf16', 8192), ('raw_qkv', 'bf16', 6144),
                     ('query', 'bf16', 4096), ('current_key', 'bf16', 1024), ('current_value', 'bf16', 1024),
                     ('attention', 'bf16', 4096), ('output_partial', 'f32', 16384))),
    ('after_first_residual', (('first_residual', 'bf16', 8192),)),
    ('after_mlp', (('post_normalized', 'bf16', 8192), ('gate', 'bf16', 12288), ('up', 'bf16', 12288),
                   ('activation', 'bf16', 12288), ('down_partial', 'f32', 16384))),
    ('after_final_residual', (('final_hidden', 'bf16', 8192),)),
)


def encoded(value):
    return json.dumps(value, sort_keys=True, allow_nan=False).encode()


def wire(path, body):
    return dict(path=path, bytes=len(body), sha256=list(hashlib.sha256(body).digest()))


def fixture():
    metadata, rotary = [0, 3] + [0] * 143, [0] * 128
    payload, parts = bytearray(), []
    for boundary, specs in STAGES:
        for rank in (0, 1):
            for role, scalar, size in specs:
                data = struct.pack('<145I', *metadata) if role == 'cache_metadata' else bytes(size)
                parts.append(dict(boundary=boundary, rank=rank, role=role, scalar=scalar,
                    bytes=size, elements=size // (2 if scalar == 'bf16' else 4), offset=len(payload),
                    source_byte_offset=49152 if role in ('current_key', 'current_value') else 0,
                    sha256=list(hashlib.sha256(data).digest())))
                payload.extend(data)
    envelope = dict(schema='FerricFiniteGuardedMlpLayerZeroCaptureV1', profile_sha256=[1] * 32,
        registration_sha256=[2] * 32, session=[3] * 32, device_ids=R.capture.IDS,
        completed_forwards=4, native_closed=True, sampling='prefix-boundaries-and-post-paired-retained',
        numerical_acceptance=False, performance_claim=False, production_authority=False,
        capture=dict(schema='FerricFiniteLayerZeroCaptureV1', generation=1, position=0, layer=0,
            payload=list(payload), payload_bytes=len(payload), payload_sha256=list(hashlib.sha256(payload).digest()),
            parts=parts, full_cache_capture=False, native_close_confirmed=True,
            numerical_acceptance=False, performance_claim=False, production_authority=False))
    bodies = {'stderr': encoded(envelope), 'request': encoded(dict(id=1, command=dict(
        generation=1, cache_metadata=metadata, rotary_bits=rotary))), 'observation': bytes(606976)}
    summary = dict(schema='FerricFiniteGuardedMlpDecodeObservationV1',
        profile_sha256=[1] * 32, registration_sha256=[2] * 32, request=dict(decode=dict(session=[3] * 32)),
        native_closed=True, child_exit_zero=True, process_group_absent=True, completed_forwards=4,
        input_tokens=[9112, 67, 25, 576], files=dict(child_stderr=wire('stderr', bodies['stderr']),
        frames=[dict(request=wire('request', bodies['request']), observation=wire('observation', bodies['observation']))]))
    read = lambda row: bodies[row['path']]
    checked = R.capture.capture_admission(encoded(summary), read, R.Fields)
    terminal = dict(schema='ferric-guarded-mlp-model-stage-capture-gpu-v1', passed=True, errors=[],
        capture_requested=True, capture_verified=True, native_attempts=1, retries=0,
        capture_observation=checked, observation={'fixture': 1}, numerical_acceptance=False,
        full_model_acceptance=False, performance_claim=False, production_authority=False)
    numerical = dict(schema='ferric-guarded-mlp-model-stage-numerical-report-v1', passed=True,
        error=None, postcheck_errors=[], input_posthashes_complete=True,
        diagnostic=dict(candidate=dict(capture_observation=checked, observation={'fixture': 1}),
            captured_dedicated_guarded_down=True, position=0, layer=0, input_token=9112),
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False, production_authority=False)
    return terminal, numerical, summary, envelope, bodies, checked


def operands(p0=0x3f000000, p1=0x3f000000, r0=0x3f80, r1=0xbf80, o0=0x4000, o1=0):
    return {(rank, role): struct.pack('<4096' + code, *([value] * 4096))
            for rank, values in enumerate(((p0, r0, o0), (p1, r1, o1)))
            for role, code, value in zip(('down_partial', 'first_residual', 'final_hidden'), ('I', 'H', 'H'), values)}


class R2Tests(unittest.TestCase):
    def test_all_rows_and_independent_rank_residuals(self):
        result, outputs = R.replay(operands())
        self.assertEqual(result['matched_words_per_rank'], [4096, 4096])
        self.assertEqual(len(result['rows']), 4096)
        self.assertFalse(result['rank_residual_inputs_byte_equal'])
        self.assertEqual(outputs['derived-down.bf16'], b'\x80\x3f' * 4096)
        self.assertEqual(outputs['replayed-final-rank0.bf16'], b'\x00\x40' * 4096)
        self.assertEqual(outputs['replayed-final-rank1.bf16'], bytes(8192))
        for key in ('numerical_acceptance', 'full_model_acceptance', 'performance_claim', 'gpu_execution',
                    'dedicated_down_dot_products_replayed', 'materialized_down_directly_captured'):
            self.assertFalse(result[key])

    def test_one_mutated_output_remains_an_exact_mismatch(self):
        selected = operands(); raw = bytearray(selected[(1, 'final_hidden')]); raw[8190] = 1
        selected[(1, 'final_hidden')] = bytes(raw)
        result, _ = R.replay(selected)
        self.assertFalse(result['all_final_encodings_match'])
        self.assertEqual(result['matched_words_per_rank'], [4096, 4095])
        self.assertEqual(result['mismatch_rows'], [4095])

    def test_nonfinite_all_operand_and_output_roles_refuse(self):
        for rank in (0, 1):
            for role in ('down_partial', 'first_residual', 'final_hidden'):
                selected = operands(); code, word = ('I', 0x7fc00001) if role == 'down_partial' else ('H', 0x7f80)
                width = struct.calcsize(code)
                selected[(rank, role)] = struct.pack('<' + code, word) + selected[(rank, role)][width:]
                with self.assertRaises(ValueError): R.replay(selected)

    def test_role_extent_and_bytes_type_are_closed(self):
        selected = operands(); del selected[(0, 'down_partial')]
        with self.assertRaises(ValueError): R.replay(selected)
        for value in (bytes(8192), bytes(16385), bytearray(16384)):
            selected = operands(); selected[(0, 'down_partial')] = value
            with self.assertRaises(ValueError): R.replay(selected)

    def test_capture_closes_all_parts_and_actual_output_join(self):
        terminal, numerical, summary, envelope, bodies, checked = fixture()
        selected, custody = R.admit(terminal, numerical, encoded(summary), lambda p: bodies[p['path']], checked)
        self.assertEqual(len(selected), 6); self.assertEqual(len(custody['selected_parts']), 6)
        mutations = (
            lambda e: e['capture']['parts'].__setitem__(29, copy.deepcopy(e['capture']['parts'][28])),
            lambda e: e['capture']['parts'][29].__setitem__('offset', 0),
            lambda e: e['capture']['parts'][29].__setitem__('rank', True),
            lambda e: e['capture']['parts'][29].__setitem__('scalar', 'bf16'),
            lambda e: e['capture']['parts'][29].__setitem__('sha256', [0] * 32),
            lambda e: e['capture']['parts'].pop(),
        )
        for change in mutations:
            value = copy.deepcopy(envelope); change(value); altered = dict(bodies, stderr=encoded(value))
            owner = copy.deepcopy(summary); owner['files']['child_stderr'] = wire('stderr', altered['stderr'])
            with self.assertRaises(ValueError):
                R.capture.capture_admission(encoded(owner), lambda p: altered[p['path']], R.Fields)
        altered = dict(bodies, observation=b'\x01\x00' + bodies['observation'][2:])
        owner = copy.deepcopy(summary); owner['files']['frames'][0]['observation'] = wire('observation', altered['observation'])
        with self.assertRaises(ValueError):
            R.capture.capture_admission(encoded(owner), lambda p: altered[p['path']], R.Fields)

    def test_failed_admission_identity_and_saved_observations_refuse(self):
        terminal, numerical, summary, envelope, bodies, checked = fixture()
        for change in (lambda t, n: t.__setitem__('passed', False),
                       lambda t, n: n.__setitem__('postcheck_errors', ['changed']),
                       lambda t, n: n['diagnostic'].__setitem__('captured_dedicated_guarded_down', False),
                       lambda t, n: t['capture_observation'].__setitem__('generation', 2)):
            t, n = copy.deepcopy(terminal), copy.deepcopy(numerical); change(t, n)
            with self.assertRaises(ValueError): R.admit(t, n, encoded(summary), lambda p: bodies[p['path']], checked)
        for key, value in (('native_closed', False), ('profile_sha256', [4] * 32), ('session', [4] * 32)):
            altered = copy.deepcopy(envelope); altered[key] = value
            raw = encoded(altered); owner = copy.deepcopy(summary); owner['files']['child_stderr'] = wire('stderr', raw)
            with self.assertRaises(ValueError):
                R.capture.capture_admission(encoded(owner), lambda p: raw if p['path'] == 'stderr' else bodies[p['path']], R.Fields)

    def test_framework_control_is_separate_and_reports_mismatch(self):
        p, r, f = b'\x80\x3f' * 4096, b'\x80\xbf' * 4096, bytes(8192)
        self.assertTrue(R.framework_control(p, r, f)['all_encodings_match'])
        value = R.framework_control(p, r, b'\x01\x00' + f[2:])
        self.assertFalse(value['all_encodings_match']); self.assertEqual(value['matched_words'], 4095)

    def test_materialization_zero_and_overflow_boundaries(self):
        self.assertEqual(R.F.combined(0x3f800000, 0x3b800000, 0xbf80)['residual'], 0)
        self.assertEqual(R.F.combined(0x3f800000, 0x3c400000, 0xbf80)['residual'], 0x3c80)
        self.assertEqual(R.F.combined(0x80000000, 0x80000000, 0x8000)['residual'], 0)
        self.assertEqual(R.F.combined(0x8001, 0, 0)['residual'], 1)
        self.assertEqual(R.F.combined(0x18000, 0, 0)['residual'], 2)
        for values in ((0x7f7fffff, 0x7f7fffff, 0), (0x7f7fffff, 0, 0), (0, 0, 0x7f80)):
            with self.assertRaises(ValueError): R.F.combined(*values)


if __name__ == '__main__':
    unittest.main()
