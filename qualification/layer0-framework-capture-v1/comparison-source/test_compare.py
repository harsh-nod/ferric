"""Synthetic layout/identity tests; no framework, device, or numerical policy."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import unittest
from unittest.mock import patch

import compare as C
import diagnostics as D


def pin(name, raw):
    return dict(path='/synthetic/' + name, bytes=len(raw), sha256=C.sha(raw))


def framework():
    rows, files = {}, {}
    for name, shape in C.SHAPES.items():
        count = 1
        for d in shape: count *= d
        raw = b'\0\0' * count
        if name == 'rotary-cos': raw = b'\x80\x3f' * count
        record = pin(name, raw); files[record['path']] = raw
        rows[name] = dict(dtype='bfloat16', shape=list(shape), pin=record)
    passes = [dict(ordinal=i, position=0, input_token=9112, fresh_cache=True, stages=copy.deepcopy(rows)) for i in (1, 2)]
    report = dict(schema='ferric-p228-layer0-framework-capture-v1', status='PASS', position=0, input_token=9112,
        model_id=C.MODEL, bundle_id=C.BUNDLE, genuine_framework_chain=True, conditional_replay_performed=False,
        candidate_intermediate_inputs=False, candidate_gpu_execution=False, repeat_passes_byte_equal=True,
        captured_stages_per_pass=33, numerical_acceptance=False, acceptance_threshold=None, passes=passes)
    return report, files


def historical(values, page=7):
    chunks = []
    for rank in range(2):
        parts = {name: b'\0' * size for name, size in C.HISTORICAL_LAYOUT}
        parts['qkv'] = (values['q-projection'][rank * 4096:(rank + 1) * 4096]
                        + values['k-projection'][rank * 1024:(rank + 1) * 1024]
                        + values['v-projection'][rank * 1024:(rank + 1) * 1024])
        for stage, field in (('key-cache', 'cache-key'), ('value-cache', 'cache-value')):
            raw = bytearray(parts[stage]); start = page * 16 * 1024
            raw[start:start + 1024] = values[field][rank * 1024:(rank + 1) * 1024]
            parts[stage] = bytes(raw)
        chunks.extend(parts[name] for name, _ in C.HISTORICAL_LAYOUT)
    return b''.join(chunks)


def native_fixture(change=None):
    payload = b'\0' * D.PAYLOAD_BYTES
    tokens = C.TOKENS + [0] * 2044
    if change == 'prompt-token': tokens[0] = 785
    token_raw = struct.pack('<2048I', *tokens)
    manifest = dict(schema='FerricQwen3LongPromptV1', input_token_ids=tokens, input_tokens=2048,
                    add_special_tokens=False, chat_template=None)
    if change == 'manifest-token': manifest['input_token_ids'] = [785] + tokens[1:]
    manifest_raw = json.dumps(manifest).encode()
    token_pin, manifest_pin = pin('prompt.u32le', token_raw), pin('prompt.json', manifest_raw)
    def wire_pin(p): return dict(p, sha256=list(bytes.fromhex(p['sha256'])))
    request = dict(schema='FerricFinitePrefixDecodeDeviceClockRequestV2', decode=dict(
        mode='teacher_forced', prompt=dict(tokens=wire_pin(token_pin), manifest=wire_pin(manifest_pin), text={}),
        expected_model_id=list(bytes.fromhex(C.MODEL)), expected_bundle_id=list(bytes.fromhex(C.BUNDLE)),
        prefix_image=dict(bytes=53560, sha256=list(bytes.fromhex(C.V7))),
        tiles_image=dict(bytes=33112, sha256=list(bytes.fromhex(C.DOWN2)))))
    if change == 'image': request['decode']['prefix_image']['sha256'] = [0] * 32
    body = json.dumps(request).encode(); req_pin = pin('request', body); payload_pin = pin('observation', payload)
    files = {req_pin['path']: body, payload_pin['path']: payload if change != 'payload' else b'bad',
             token_pin['path']: token_raw, manifest_pin['path']: manifest_raw}
    observed = dict(mode='teacher_forced', input_tokens=C.TOKENS, positions=[0, 1, 2, 3],
                    logical_generations=[1, 2, 3, 4], output_tokens=[0] * 4)
    if change == 'observed-token': observed['input_tokens'] = [785, 2190, 3772, 220]
    if change == 'observed-output': observed['output_tokens'] = [1] * 4
    complete = dict(schema='ferric-p228-down2-clock-gpu-v1', passed=True, failures=[], native_attempts=1, retries=0,
        checked=dict(all_payloads_tokens_and_tensors_equal=True, recorded_close_and_owner_reap_checked=True,
                     structural=observed), request=req_pin, retained_native={'observation-0.bin': payload_pin})
    return complete, request, files


class Comparison(unittest.TestCase):
    def test_unchanged_diagnostics_source_is_exact(self):
        self.assertEqual(hashlib.sha256(Path(D.__file__).read_bytes()).hexdigest(), C.DIAGNOSTICS_SHA)

    def test_full_framework_shape_roster_and_repeat(self):
        report, files = framework()
        with patch.object(C, 'EMBEDDING_SHA', C.sha(b'\0' * 8192)):
            values = C.read_framework(report, lambda p: files[p['path']], D)
        self.assertEqual(len(values), 33)
        self.assertEqual(len(values['q-norm']), 8192)

    def test_wrong_token_and_conditional_inputs_are_refused(self):
        for key, value in (('input_token', 785), ('conditional_replay_performed', True), ('candidate_intermediate_inputs', True)):
            report, files = framework(); report[key] = value
            with self.assertRaises(ValueError): C.read_framework(report, lambda p: files[p['path']], D)

    def test_wrong_observed_dtype_or_shape_is_refused(self):
        for key, value in (('dtype', 'float32'), ('shape', [4096])):
            report, files = framework(); report['passes'][0]['stages']['embedding'][key] = value
            with self.assertRaises(ValueError): C.read_framework(report, lambda p: files[p['path']], D)

    def test_changed_stage_pin_and_nonfinite_values_are_refused(self):
        for nonfinite in (False, True):
            report, files = framework(); stage = report['passes'][0]['stages']['q-norm']
            raw = b'\x80\x7f' * 4096 if nonfinite else b'\x80\x3f' * 4096
            files[stage['pin']['path']] = raw
            if nonfinite: stage['pin'] = pin('q-norm', raw)
            with patch.object(C, 'EMBEDDING_SHA', C.sha(b'\0' * 8192)):
                with self.assertRaises(ValueError): C.read_framework(report, lambda p: files[p['path']], D)

    def test_producer_join_and_real_repeat_mismatch_are_refused(self):
        for name in ('q-input', 'q-norm'):
            report, files = framework(); raw = b'\x80\x3f' * 4096
            record = pin('changed', raw); files[record['path']] = raw
            report['passes'][1]['stages'][name]['pin'] = record
            with patch.object(C, 'EMBEDDING_SHA', C.sha(b'\0' * 8192)):
                with self.assertRaises(ValueError): C.read_framework(report, lambda p: files[p['path']], D)

    def test_original_framework_slice_comes_from_actual_complete_payload(self):
        raw = b'\0' * D.PAYLOAD_BYTES; record = pin('old', raw)
        case = dict(record=dict(generation=1, position=0, input_token=9112, output_token=0), payload=record,
                    tensors={'layer0-hidden': dict(bytes=8192, sha256=C.sha(raw[:8192]))})
        report = dict(schema='ferric-p224-rearm-four-framework-reference-v1', status='PASS', model_id=C.MODEL,
            bundle_id=C.BUNDLE, mode='teacher_forced', selected_input_tokens=C.TOKENS,
            repeat_passes_byte_equal=True, candidate_intermediate_inputs=False,
            passes=[dict(ordinal=i, fresh_cache=True, cases=[case, {}, {}, {}]) for i in (1, 2)])
        self.assertEqual(C.original_framework_hidden(report, lambda _p: raw, D), raw[:8192])
        case['tensors']['layer0-hidden']['sha256'] = '0' * 64
        with self.assertRaises(ValueError): C.original_framework_hidden(report, lambda _p: raw, D)

    def test_primary_mismatches_are_diagnostics_not_acceptance(self):
        zeros, ones = b'\0' * 8192, b'\x80\x3f' * 4096
        value = C.compare_primary(zeros, ones, ones, D)
        self.assertFalse(value['original_framework_matches_new'])
        self.assertFalse(value['numerical_acceptance'])
        self.assertIsNone(value['acceptance_threshold'])
        self.assertEqual(value['current_native']['exact_words'], 0)
        self.assertFalse(value['conditional_replay_performed'])

    def test_current_native_uses_actual_v7_down2_payload_and_request(self):
        complete, request, files = native_fixture()
        self.assertNotIn('input_tokens', request['decode'])
        self.assertEqual(C.current_hidden(complete, request, lambda p: files[p['path']], D), b'\0' * 8192)
        request['decode']['mode'] = 'changed'
        with self.assertRaises(ValueError): C.current_hidden(complete, request, lambda p: files[p['path']], D)

    def test_current_image_change_or_wrong_capture_pin_is_rejected(self):
        for changed in ('image', 'payload', 'prompt-token', 'manifest-token', 'observed-token', 'observed-output'):
            complete, request, files = native_fixture(changed)
            with self.assertRaises(ValueError): C.current_hidden(complete, request, lambda p: files[p['path']], D)

    def test_checkpoint_identity_ignores_only_host_stat_fields(self):
        a = {'model_sources': {'shard': dict(bytes=1, sha256='a', stat=[1])}}
        b = copy.deepcopy(a); b['model_sources']['shard']['stat'] = [2]
        C.same_model_sources(a, b)
        b['model_sources']['shard']['sha256'] = 'b'
        with self.assertRaises(ValueError): C.same_model_sources(a, b)

    def test_historical_rank_qkv_and_current_page_mapping(self):
        report, files = framework()
        values = {name: files[row['pin']['path']] for name, row in report['passes'][0]['stages'].items()}
        for name, count in (('q-projection', 2048), ('k-projection', 512), ('v-projection', 512), ('cache-key', 512), ('cache-value', 512)):
            values[name] = b'\x80\x3f' * count + b'\0\x40' * count
        result = C.historical_views(values, historical(values), 7, D)
        for rank in result['rows']:
            selected = {r['stage']: r for r in rank['comparisons']}
            for stage in ('qkv', 'key-current', 'value-current'): self.assertTrue(selected[stage]['byte_equal'])
        wrong = C.historical_views(values, historical(values), 0, D)
        self.assertFalse(wrong['rows'][0]['comparisons'][3]['byte_equal'])
        self.assertTrue(result['historical_V227_only'])
        self.assertFalse(result['current_V7_intermediate_evidence'])

    def test_historical_activation_maps_product_not_silu_and_excludes_partials(self):
        report, files = framework()
        values = {name: files[row['pin']['path']] for name, row in report['passes'][0]['stages'].items()}
        values['silu'] = b'\x80\x3f' * 12288
        result = C.historical_views(values, historical(values), 7, D)
        for rank in result['rows']:
            rows = {r['stage']: r for r in rank['comparisons']}
            self.assertEqual(len(rows), 12)
            self.assertTrue(rows['activation-product']['byte_equal'])
            self.assertNotIn('output-partial', rows)
            self.assertNotIn('down-partial', rows)

    def test_bounds_page_and_strict_json_fail_closed(self):
        for page in (True, -1, 144):
            with self.assertRaises(ValueError): C.historical_views({}, b'\0' * 9670656, page, D)
        with self.assertRaises(ValueError): C.checked_read(lambda _p: b'a', pin('x', b'ab'), 8)
        with self.assertRaises(ValueError): C.document('{"x":1,"x":2}')
        with self.assertRaises(ValueError): C.document('{"x":NaN}')


if __name__ == '__main__':
    unittest.main(verbosity=2)
