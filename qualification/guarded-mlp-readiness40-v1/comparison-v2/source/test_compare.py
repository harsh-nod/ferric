import copy
import json
import unittest

from common import compact, encoded
import compare as C
import reference as R


def wire(raw):
    row = compact(raw)
    return dict(bytes=row['bytes'], sha256=list(bytes.fromhex(row['sha256'])))


def fixture():
    tokens = [17 + i % 41 for i in range(2048)]
    contract = dict(model_id='11' * 32, bundle_id='22' * 32)
    payload = b'\x80\x3f' * (R.PAYLOAD_BYTES // 2)
    tensors = R.D.split_payload(payload)
    cases, frames, native = [], [], {}
    for p in range(40):
        chosen = p in R.SELECTED
        cases.append(dict(position=p, generation=p + 1, input_token=tokens[p],
            predicted_token=0, cache_length=p + 1, logits=compact(tensors['logits']),
            payload=compact(payload) if chosen else None,
            tensors={n: compact(b) for n, b in tensors.items()} if chosen else None,
            cache_hashes=[{k: dict(bytes=2 * 8 * (p + 1) * 128, sha256='33' * 32)
                           for k in ('key', 'value')} for _ in range(36)] if chosen else None))
        control = bytes(C.CONTROL_BYTES)
        frames.append(dict(schema='FerricGuardedMlpLongResponseV2', profile='readiness40',
            request=dict(command=dict(op='forward', token=tokens[p], generation=p + 1)),
            completion=dict(position=p, generation=p + 1, input_token=tokens[p], output_token=0,
                captured=chosen, control=wire(control), observation=wire(payload), logits=wire(tensors['logits']))))
        if chosen:
            native['readiness/native/capture-%d.bin' % p] = control + payload
    native['readiness/native/frames.ndjson'] = b''.join(encoded(row).replace(b'\n', b'') + b'\n' for row in frames)
    native['readiness/native/complete.json'] = encoded(dict(bootstrap=dict(sequence=dict(
        profile='readiness40', prompt_tokens=tokens,
        scope={k: list(bytes.fromhex(v)) for k, v in contract.items()}))))
    report = dict(model_id=contract['model_id'], bundle_id=contract['bundle_id'],
        full_prompt_tokens=tokens[:], input_tokens=tokens[:40], generated_tokens=0,
        native_intermediates_used=False, candidate_receipt=None,
        passes=[dict(ordinal=n, fresh_cache=True, cases=copy.deepcopy(cases)) for n in (1, 2)])
    refs = [{p: payload for p in R.SELECTED} for _ in range(2)]
    return native, report, refs, contract, tokens


def frames(value):
    return [json.loads(row) for row in value['readiness/native/frames.ndjson'].splitlines()]


def save_frames(value, rows):
    value['readiness/native/frames.ndjson'] = b''.join(encoded(row).replace(b'\n', b'') + b'\n' for row in rows)


class ComparisonTests(unittest.TestCase):
    def test_full152_metrics_and40_argmax_records_never_accept(self):
        result = C.compare(*fixture())
        self.assertEqual(result['tensor_rows'], 152)
        self.assertEqual(sum(len(r['tensors']) for r in result['selected']['comparisons']), 152)
        self.assertEqual(result['argmax_matches'], 40)
        self.assertEqual(sum(r['candidate_argmax_recomputed'] for r in result['argmax_diagnostics']), 4)
        self.assertFalse(result['numerical_acceptance'])
        self.assertIsNone(result['acceptance_threshold'])
        self.assertFalse(result['selected']['receipt_authentication'])

    def test_unselected_argmax_difference_is_diagnostic_not_refusal(self):
        args = fixture(); rows = frames(args[0]); rows[7]['completion']['output_token'] = 3
        save_frames(args[0], rows)
        result = C.compare(*args)
        self.assertEqual(result['argmax_matches'], 39)
        self.assertFalse(result['argmax_diagnostics'][7]['candidate_argmax_recomputed'])
        self.assertFalse(result['numerical_acceptance'])

    def test_all40_input_ids_including_unselected_are_required(self):
        for position in (0, 7, 15, 16, 38, 39):
            args = fixture(); rows = frames(args[0]); rows[position]['completion']['input_token'] += 1
            save_frames(args[0], rows)
            with self.subTest(position=position), self.assertRaises(ValueError): C.compare(*args)

    def test_full_prompt_and_both_model_identities_are_required(self):
        for key in ('prompt', 'model_id', 'bundle_id'):
            args = fixture(); s = json.loads(args[0]['readiness/native/complete.json'])
            if key == 'prompt': s['bootstrap']['sequence']['prompt_tokens'][2047] += 1
            else: s['bootstrap']['sequence']['scope'][key][0] ^= 1
            args[0]['readiness/native/complete.json'] = encoded(s)
            with self.subTest(key=key), self.assertRaises(ValueError): C.compare(*args)

    def test_capture_boundary_hash_and_finite_words_are_required(self):
        for mode in ('short', 'hash', 'nan'):
            args = fixture(); key = 'readiness/native/capture-16.bin'; raw = args[0][key]
            if mode == 'short': args[0][key] = raw[:-1]
            elif mode == 'hash': args[0][key] = bytes([raw[0] ^ 1]) + raw[1:]
            else:
                payload = b'\xc0\x7f' + raw[C.CONTROL_BYTES + 2:]
                args[0][key] = raw[:C.CONTROL_BYTES] + payload
                rows = frames(args[0]); rows[16]['completion']['observation'] = wire(payload); save_frames(args[0], rows)
            with self.subTest(mode=mode), self.assertRaises(ValueError): C.compare(*args)

    def test_selected_argmax_and_repeat_gate_are_required(self):
        for mode in ('argmax', 'repeat'):
            args = fixture()
            if mode == 'argmax':
                rows = frames(args[0]); rows[15]['completion']['output_token'] = 1; save_frames(args[0], rows)
            else: args[1]['passes'][1]['cases'][6]['predicted_token'] = 1
            with self.subTest(mode=mode), self.assertRaises(ValueError): C.compare(*args)

    def test_native_order_selection_and_boolean_scalars_refused(self):
        for mode in ('order', 'selection', 'bool'):
            args = fixture(); rows = frames(args[0])
            if mode == 'order': rows[7], rows[8] = rows[8], rows[7]
            elif mode == 'selection': rows[7]['completion']['captured'] = True
            else: rows[0]['completion']['generation'] = True
            save_frames(args[0], rows)
            with self.subTest(mode=mode), self.assertRaises(ValueError): C.compare(*args)

    def test_native_reference_ancestry_and_wire_hash_types_refused(self):
        args = fixture(); args[1]['native_intermediates_used'] = True
        with self.assertRaises(ValueError): C.compare(*args)
        for field, value in (('bytes', True), ('sha256', [False] * 32), ('sha256', '00' * 32)):
            row = wire(b'x'); row[field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError): C.compact_wire(row, 1)
