import copy
import json
import struct
import unittest

import validation as V


def pin(path, raw=b'x'):
    return dict(path=path, bytes=len(raw), sha256=V.sha(raw))


def state(profile):
    if profile == 0:
        return [1, 0, 65535, 65535, 0x55555555, 0] + [64] * 16
    return ([1, 0, 0, 31] + [1, 48, 1, 16, 64] * 2
            + ([4294967295] * 4 + [3]) * 2 + [1] * 130 + [64] * 130)


def fixture(case='genuine-pos0'):
    history, position = V.case_scope(case)
    baseline = dict(schema='fe2o3-qwen-resident-prefix-tp2-request-v1',
        devices=[dict(rank=r, unique_id=u) for r, u in enumerate(V.DEVICES)],
        producer=pin('/e/v5.hsaco'), consumer=pin('/e/consumer.hsaco'),
        pair_reference=pin('/e/pair.json'), timeout_ms=10000,
        history_kind=history, position=position,
        prefix_requests=[pin('/e/rank0.json'), pin('/e/rank1.json')])
    requested = dict(schema='fe2o3-qwen-prefix-tiles-comparison-request-v6',
        baseline_request=pin('/e/baseline.json'),
        tiles=dict(object=pin('/e/v6.hsaco'), descriptor_sha256='a' * 64,
            canonical_code_object_digest='b' * 64, entry_symbol=V.SYMBOL,
            descriptor_symbol=V.SYMBOL + '.kd'),
        reviews=[pin('/e/' + name + '.json') for name in V.KINDS],
        timeout_ms=10000, capture_directory='/e/case/captures')
    requested_pin = pin('/e/request.json', json.dumps(requested).encode())
    inspected = dict(schema='fe2o3-qwen-prefix-tiles-comparison-inspection-v6', authority='none',
        request_sha256=requested_pin['sha256'], baseline_request=requested['baseline_request'],
        baseline_image_sha256=baseline['producer']['sha256'], tiles=requested['tiles'],
        devices=V.DEVICES, history_kind=history, position=position, reviews=requested['reviews'],
        opened_device=False, completed_and_closed=False, bitwise_match=False,
        input_sha256=[['c' * 64] * 7 for _ in range(2)],
        initial_output_sha256=[['d' * 64] * 7 for _ in range(2)],
        data_root_bytes=V.EXTENTS, v5_grid=[128, 1, 1], v6_grid=[4096, 1, 1],
        workgroup=[64, 1, 1], stage_order=list(V.STAGES), capture_bytes_per_rank=V.CAPTURE_BYTES,
        capture_format=V.FORMAT, **{name: False for name in V.FALSE_FIELDS})
    for rank in range(2):
        inspected['input_sha256'][rank][5] = V.sha(struct.pack('<145I', position,
            *[(page * 5 + 7) % 144 for page in range(144)]))
    stages = [bytes(n) for n in V.EXTENTS[7:]]
    capture = b''.join(stages)
    captured, capture_pins = {}, []
    for profile in ('baseline-v5', 'tiles-v6'):
        ranks = []
        for rank in range(2):
            path = '/e/case/captures/' + profile + '-rank' + str(rank) + '.bin'
            captured[path] = capture; ranks.append(pin(path, capture))
        capture_pins.append(ranks)
    observed = dict(inspected, schema='fe2o3-qwen-prefix-tiles-comparison-observation-v6',
        opened_device=True, completed_and_closed=True, bitwise_match=True,
        profiles=[dict(profile=name, states=[state(index), state(index)],
                       host_dispatch_elapsed_ns=[1, 2], closed=True)
                  for index, name in enumerate(('baseline_v5', 'tiles_v6'))],
        captures=capture_pins, immutable_input_readbacks_match=True, timing_boundary=V.TIMING,
        stages=[dict(rank=rank, stage=name, elements=len(stages[i]) // (4 if i == 6 else 2),
                     word_bytes=4 if i == 6 else 2, mismatches=0,
                     baseline_sha256=V.sha(stages[i]), tiles_sha256=V.sha(stages[i]))
                for rank in range(2) for i, name in enumerate(V.STAGES)])
    return requested, requested_pin, baseline, inspected, observed, captured


def checked(values, case='genuine-pos0'):
    requested, requested_pin, baseline, inspected, observed, captured = values
    return V.observation(observed, inspected, requested, requested_pin, baseline, case, captured)


class Tests(unittest.TestCase):
    def test_exact_capture_census_and_closed_success(self):
        values = fixture()
        result = checked(values)
        self.assertEqual((V.CAPTURE_BYTES, V.CASE_CAPTURE_BYTES), (4757504, 19030016))
        self.assertEqual(len(result['compared_rows']), 14)
        self.assertEqual(result['observed_owners'][1][0]['useful_workgroups'], 1)
        self.assertFalse(result['independent_numerical_acceptance'])

    def test_six_cases_and_request_scope_are_closed(self):
        for case in V.CASES:
            requested, _, baseline, *_ = fixture(case)
            V.request(requested, baseline, case)
            baseline['position'] += 1
            with self.assertRaises(RuntimeError): V.request(requested, baseline, case)
        for case in ('patterned-pos0', 'genuine-pos1', 'patterned-pos2303'):
            with self.assertRaises(RuntimeError): V.case_scope(case)

    def test_json_duplicate_nonfinite_empty_or_large_refused(self):
        for raw in (b'', b'{"x":1,"x":2}', b'{"x":NaN}', b' ' * (65536 + 1)):
            with self.assertRaises((RuntimeError, ValueError)): V.parse(raw)

    def test_exact_review_scope_and_authority(self):
        requested, _, baseline, *_ = fixture()
        value = dict(schema='fe2o3-qwen-prefix-tiles-comparison-review-v6', authority='none',
            kind='isa', baseline_request_sha256=requested['baseline_request']['sha256'],
            baseline_image_sha256=baseline['producer']['sha256'],
            tiles_image_sha256=requested['tiles']['object']['sha256'], descriptor_sha256='a' * 64,
            canonical_code_object_digest='b' * 64, devices=V.DEVICES, history_kind='genuine',
            position=0, workgroup=[64, 1, 1], grid=[4096, 1, 1], reviewed=True,
            runtime_premises_discharged=False, production_authority=False,
            notes='Synthetic test-only review, not an operator record.')
        V.review(value, 'isa', requested, baseline, V.CASES[0])
        for key, replacement in [('position', 4), ('reviewed', 1), ('notes', ''),
                ('production_authority', True), ('tiles_image_sha256', 'f' * 64),
                ('devices', list(reversed(V.DEVICES))), ('grid', [128, 1, 1])]:
            changed = dict(value, **{key: replacement})
            with self.assertRaises(RuntimeError): V.review(changed, 'isa', requested, baseline, V.CASES[0])

    def test_request_timeout_aliases_and_old_profile_refused(self):
        requested, _, baseline, *_ = fixture()
        for key, replacement in [('timeout_ms', True), ('timeout_ms', 0), ('timeout_ms', 10001),
                ('capture_directory', '/e/../captures'), ('schema', 'old'),
                ('reviews', [requested['reviews'][0]] * 6)]:
            with self.assertRaises(RuntimeError): V.request(dict(requested, **{key: replacement}), baseline, V.CASES[0])

    def test_inspection_never_claims_open_or_close(self):
        requested, requested_pin, baseline, inspected, *_ = fixture()
        V.inspection(inspected, requested, requested_pin, baseline, V.CASES[0])
        for key in ('opened_device', 'completed_and_closed', 'bitwise_match', *V.FALSE_FIELDS):
            with self.assertRaises(RuntimeError):
                V.inspection(dict(inspected, **{key: True}), requested, requested_pin, baseline, V.CASES[0])

    def test_all_284_state_words_and_both_profiles(self):
        V.terminal(state(0), 0); V.terminal(state(1), 1)
        for index in range(284):
            changed = state(1)
            changed[index] = 0 if changed[index] else 1
            with self.assertRaises(RuntimeError): V.terminal(changed, 1)
        for words in (state(0), state(1)[:-1], state(1) + [64]):
            with self.assertRaises(RuntimeError): V.terminal(words, 1)
        changed = state(0); changed[4] = 0
        with self.assertRaises(RuntimeError): V.terminal(changed, 0)

    def test_owners_describe_observed_coverage_not_launched_count(self):
        words = state(1); words[24:154] = [64] * 130
        self.assertEqual(V.terminal(words, 1)['useful_workgroups'], 1)
        words[24:88] = list(range(1, 65))
        self.assertEqual(V.terminal(words, 1)['useful_workgroups'], 64)
        for owner in (0, 65, True):
            words[153] = owner
            with self.assertRaises(RuntimeError): V.terminal(words, 1)

    def test_close_and_second_rank_terminal_failure_refuse(self):
        for mutation in ('close', 'state', 'profile', 'timing'):
            values = fixture(); profile = values[4]['profiles'][1]
            if mutation == 'close': profile['closed'] = False
            if mutation == 'state': profile['states'][1][-1] = 63
            if mutation == 'profile': profile['profile'] = 'baseline_v5'
            if mutation == 'timing': profile['host_dispatch_elapsed_ns'][1] = True
            with self.assertRaises(RuntimeError): checked(values)

    def test_inspected_input_and_actual_image_cannot_change(self):
        for key in ('input_sha256', 'initial_output_sha256'):
            values = fixture(); values[4][key] = copy.deepcopy(values[4][key])
            values[4][key][1][6] = 'e' * 64
            with self.assertRaises(RuntimeError): checked(values)

    def test_capture_extent_hash_and_path_are_exact(self):
        for mutation in ('short', 'hash', 'path', 'extra'):
            values = fixture(); record = values[4]['captures'][1][1]
            if mutation == 'short': values[5][record['path']] = values[5][record['path']][:-1]
            if mutation == 'hash': record['sha256'] = 'f' * 64
            if mutation == 'path': record['path'] += '.other'
            if mutation == 'extra': values[5]['/e/unexpected.bin'] = b'x'
            with self.assertRaises(RuntimeError): checked(values)

    def test_full_kv_last_word_mismatch_cannot_hide_in_report_hash(self):
        values = fixture(); record = values[4]['captures'][1][1]
        raw = bytearray(values[5][record['path']])
        offset = sum(V.EXTENTS[7:12]) - 1
        raw[offset] = 1
        values[5][record['path']] = bytes(raw); record['sha256'] = V.sha(bytes(raw))
        with self.assertRaises(RuntimeError): checked(values)

    def test_recomputed_stage_roster_and_hashes_required(self):
        for mutation in ('row', 'hash', 'extent', 'mismatch'):
            values = fixture(); rows = values[4]['stages']
            if mutation == 'row': rows.reverse()
            if mutation == 'hash': rows[7]['baseline_sha256'] = 'f' * 64
            if mutation == 'extent': rows[0]['elements'] -= 1
            if mutation == 'mismatch': rows[13]['mismatches'] = 1
            with self.assertRaises(RuntimeError): checked(values)

    def test_nonfinite_computed_outputs_refused_even_equal(self):
        for raw, width in ((b'\x80\x7f', 2), (b'\xc0\x7f', 2), (b'\x00\x00\x80\x7f', 4),
                           (b'\x00\x00\xc0\x7f', 4)):
            with self.assertRaises(RuntimeError): V.finite(raw, width)
        V.finite(b'\x01\x00\x00\x80', 2)
        V.finite(b'\x01\x00\x00\x00', 4)

    def test_failure_or_partial_report_never_becomes_success(self):
        values = fixture(); values[4]['schema'] = 'fe2o3-qwen-prefix-tiles-comparison-failure-v6'
        with self.assertRaises(RuntimeError): checked(values)
        values = fixture(); values[4]['completed_and_closed'] = False
        with self.assertRaises(RuntimeError): checked(values)


if __name__ == '__main__':
    unittest.main()
