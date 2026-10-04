"""Synthetic policy/routing tests, not GPU or independent numerical evidence."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import types
import unittest

import observe as O
import validation as V


ROOT = Path('/synthetic/evidence')
CASE = 'patterned-pos16'
DIRECTORY = ROOT / 'prefix-independent-profile-gpu-v228-v1' / CASE


def pin(path, raw=b'x'):
    return dict(path=str(path), bytes=len(raw), sha256=V.sha(raw))


def state(profile):
    if profile == 0:
        return [1, 0, 65535, 65535, 0x55555555, 0] + [64] * 16
    return ([1, 0, 0, 31] + [1, 48, 1, 16, 64] * 2
            + ([4294967295] * 4 + [3]) * 2 + [1] * 130 + [64] * 130)


def fixture():
    baseline = dict(schema='fe2o3-qwen-resident-prefix-tp2-request-v1',
        devices=[dict(rank=rank, unique_id=uid) for rank, uid in enumerate(V.DEVICES)],
        producer=pin(ROOT / 'baseline.hsaco'), consumer=pin(ROOT / 'consumer.hsaco'),
        pair_reference=pin(ROOT / 'pair.json'), timeout_ms=10000,
        history_kind='patterned', position=16,
        prefix_requests=[pin(ROOT / ('source-rank' + str(rank) + '.json')) for rank in range(2)])
    requested = dict(schema='fe2o3-qwen-prefix-tiles-comparison-request-v6',
        baseline_request=pin(ROOT / 'baseline.json'),
        tiles=dict(object=pin(ROOT / 'candidate.hsaco'), descriptor_sha256='a' * 64,
            canonical_code_object_digest='b' * 64, entry_symbol=V.SYMBOL,
            descriptor_symbol=V.SYMBOL + '.kd'),
        reviews=[pin(ROOT / (kind + '.json')) for kind in V.KINDS], timeout_ms=10000,
        capture_directory=str(DIRECTORY / 'captures'))
    request_pin = pin(ROOT / 'request.json', json.dumps(requested).encode())
    inspected = dict(schema=O.INSPECTION_SCHEMA, authority='none',
        request_sha256=request_pin['sha256'], baseline_request=requested['baseline_request'],
        baseline_image_sha256=baseline['producer']['sha256'], tiles=copy.deepcopy(requested['tiles']),
        devices=V.DEVICES, history_kind='patterned', position=16, reviews=requested['reviews'],
        opened_device=False, completed_and_closed=False, bitwise_match=False,
        success=False, native_execution_attempted=False, paired_comparison_performed=False,
        immutable_input_readbacks_match=False, profiles=[], captures=[],
        input_sha256=[['c' * 64] * 7 for _ in range(2)],
        initial_output_sha256=[['d' * 64] * 7 for _ in range(2)],
        data_root_bytes=V.EXTENTS, v5_grid=[128, 1, 1], v6_grid=[4096, 1, 1], workgroup=[64, 1, 1],
        stage_order=list(V.STAGES), capture_bytes_per_rank=V.CAPTURE_BYTES,
        capture_format=V.FORMAT, **{name: False for name in V.FALSE_FIELDS})
    for rank in range(2):
        inspected['input_sha256'][rank][5] = V.sha(struct.pack('<145I', 16,
            *[(page * 5 + 7) % 144 for page in range(144)]))
        inspected['input_sha256'][rank][6] = str(rank + 1) * 64
    captured, capture_pins = {}, []
    zero = bytes(V.CAPTURE_BYTES)
    for label in ('baseline-v5', 'tiles-v6'):
        pair = []
        for rank in range(2):
            path = DIRECTORY / 'captures' / (label + '-rank' + str(rank) + '.bin')
            captured[str(path)] = zero
            pair.append(pin(path, zero))
        capture_pins.append(pair)
    observed = dict(copy.deepcopy(inspected), schema=O.OBSERVATION_SCHEMA,
        success=True, opened_device=True, native_execution_attempted=True,
        completed_and_closed=True, immutable_input_readbacks_match=True,
        profiles=[dict(profile=name, states=[state(index), state(index)],
            host_dispatch_elapsed_ns=[1, 2], closed=True) for index, name in enumerate(O.PROFILES)],
        captures=capture_pins, timing_boundary=V.TIMING)
    weights = [dict(path=str(ROOT / ('o-rank' + str(rank) + '.bf16')), bytes=V.EXTENTS[6],
                    sha256=inspected['input_sha256'][rank][6]) for rank in range(2)]
    plan = dict(schema=O.PLAN_SCHEMA, case=CASE, case_directory=str(DIRECTORY), request=request_pin,
        binary=pin(ROOT / 'native-example'), inspection_result=pin(DIRECTORY / 'inspection/result.json'),
        native_result=pin(DIRECTORY / 'native/result.json'), output_weights=weights)
    return requested, request_pin, baseline, inspected, observed, captured, plan


def check(values):
    requested, request_pin, baseline, inspected, observed, captured, _ = values
    return O.observation(observed, inspected, requested, request_pin, baseline, CASE, captured)


def change_capture(values, profile, rank, offset, replacement):
    record = values[4]['captures'][profile][rank]
    raw = bytearray(values[5][record['path']])
    raw[offset:offset + len(replacement)] = replacement
    raw = bytes(raw)
    values[5][record['path']] = raw
    record['sha256'] = V.sha(raw)


class Tracker:
    def __init__(self, fail=False):
        self.records, self.calls, self.fail = {}, 0, fail

    def recheck(self):
        self.calls += 1
        if self.fail:
            raise RuntimeError('synthetic changed input')


def mocked_numerical(plans, hashes, fail=None):
    calls = []
    false_fields = ('independent_numerical_acceptance', 'production_authority')

    def compare(plan, read, frozen):
        calls.append(plan['profile'])
        if plan['profile'] == fail:
            raise RuntimeError('synthetic numerical bound rejection')
        index = O.PROFILES.index(plan['profile'])
        return dict(schema='ferric-p228-prefix-profile-conditional-numerical-v1',
            profile=plan['profile'], case=plan['case'], conditional_operator_checks_passed=True,
            paired_comparison_performed=False, input_pins={},
            rows=[dict(profile=plan['profile'], rank=rank, capture=plan['ranks'][rank]['capture'],
                stage_sha256=hashes[index][rank], conditioning=dict(
                    output_weights=plan['ranks'][rank]['output_weights'])) for rank in range(2)],
            **{name: False for name in false_fields})
    return types.SimpleNamespace(compare_profile=compare, FALSE_FIELDS=false_fields), calls


class PolicyTests(unittest.TestCase):
    def test_frozen_legacy_validation_and_child_custody_bytes(self):
        for name, digest in (('validation.py', O.VALIDATION_SHA), ('child_evidence.py', O.CHILD_EVIDENCE_SHA)):
            self.assertEqual(hashlib.sha256(Path(O.__file__).with_name(name).read_bytes()).hexdigest(), digest)
        self.assertEqual(len(O.C.NAMES), 10)

    def test_inert_inspection_is_closed_and_new(self):
        values = fixture()
        O.inspection(values[3], values[0], values[1], values[2], CASE)
        for key in ('success', 'opened_device', 'native_execution_attempted',
                    'immutable_input_readbacks_match', 'paired_comparison_performed', *V.FALSE_FIELDS):
            wrong = dict(values[3], **{key: True})
            with self.assertRaises(RuntimeError):
                O.inspection(wrong, values[0], values[1], values[2], CASE)
        with self.assertRaises(RuntimeError):
            O.inspection(dict(values[3], unknown=False), values[0], values[1], values[2], CASE)

    def test_finite_profile_differences_do_not_require_parity(self):
        values = fixture()
        change_capture(values, 1, 1, 0, b'\x80\x3f')
        result = check(values)
        self.assertEqual(result['capture_bytes'], 19030016)
        self.assertNotEqual(result['stage_sha256'][0][1][0], result['stage_sha256'][1][1][0])
        self.assertFalse(result['paired_comparison_performed'])
        self.assertFalse(result['bitwise_match'])
        self.assertFalse(result['independent_numerical_acceptance'])
        self.assertNotIn('compared_rows', result)

    def test_old_parity_failure_and_success_reports_are_refused(self):
        for schema in ('fe2o3-qwen-prefix-tiles-comparison-observation-v6',
                       'fe2o3-qwen-prefix-tiles-independent-profiles-failure-v1'):
            values = fixture()
            values[4]['schema'] = schema
            with self.assertRaises(RuntimeError):
                check(values)
        for key, value in (('bitwise_match', True), ('paired_comparison_performed', True),
                           ('success', 1), ('stages', [])):
            values = fixture()
            values[4][key] = value
            with self.assertRaises(RuntimeError):
                check(values)

    def test_inspected_immutable_input_and_initial_hash_drift_refused(self):
        for key in ('input_sha256', 'initial_output_sha256'):
            values = fixture()
            values[4][key][1][6] = 'f' * 64
            with self.assertRaises(RuntimeError):
                check(values)

    def test_parent_image_and_scoped_review_drift_refused(self):
        for key, value in (('baseline_image_sha256', 'e' * 64), ('reviews', []),
                           ('position', 16.0), ('v6_grid', [True, 1, 1])):
            values = fixture()
            values[4][key] = value
            with self.assertRaises(RuntimeError):
                check(values)

    def test_profile_order_close_state_and_timing_refused(self):
        mutations = [lambda row: row.update(profile='baseline_v5'), lambda row: row.update(closed=False),
            lambda row: row['states'][1].__setitem__(283, 63),
            lambda row: row['host_dispatch_elapsed_ns'].__setitem__(1, True)]
        for mutate in mutations:
            values = fixture()
            mutate(values[4]['profiles'][1])
            with self.assertRaises(RuntimeError):
                check(values)

    def test_whole_capture_extent_pin_path_and_extra_body_refused(self):
        for mutation in ('extent', 'digest', 'path', 'extra'):
            values = fixture()
            record = values[4]['captures'][1][1]
            if mutation == 'extent':
                values[5][record['path']] = values[5][record['path']][:-1]
            elif mutation == 'digest':
                record['sha256'] = 'f' * 64
            elif mutation == 'path':
                record['path'] += '.wrong'
            else:
                values[5]['/synthetic/unexpected.bin'] = b'x'
            with self.assertRaises(RuntimeError):
                check(values)

    def test_every_computed_stage_and_current_kv_slot_must_be_finite(self):
        for stage in range(7):
            values = fixture()
            offset = sum(V.EXTENTS[7:7 + stage])
            if stage in (3, 4):
                offset += 192 * 1024
            change_capture(values, 1, stage % 2, offset,
                           b'\x00\x00\x80\x7f' if stage == 6 else b'\x80\x7f')
            with self.assertRaises(RuntimeError):
                check(values)

    def test_future_kv_poison_is_hashed_not_claimed_as_revalidated_history(self):
        values = fixture()
        change_capture(values, 1, 0, sum(V.EXTENTS[7:10]), b'\xc1\x7f')
        result = check(values)
        self.assertNotEqual(result['stage_sha256'][0][0][3], result['stage_sha256'][1][0][3])
        self.assertIn('untouched_kv_bytes_rechecked', O.FALSE_FIELDS)
        self.assertIn('historical_kv_numerics_checked', O.FALSE_FIELDS)

    def test_numerical_plans_bind_each_profile_capture_and_input_hashes(self):
        values = fixture()
        plans = O.numerical_plans(values[4], values[0], CASE, values[6]['output_weights'])
        self.assertEqual([row['profile'] for row in plans], list(O.PROFILES))
        for index, plan in enumerate(plans):
            self.assertEqual(plan['baseline_request'], values[0]['baseline_request'])
            for rank, row in enumerate(plan['ranks']):
                self.assertEqual(row['capture'], values[4]['captures'][index][rank])
                self.assertEqual(row['input_sha256'], values[4]['input_sha256'][rank])
                self.assertEqual(row['output_weights'], values[6]['output_weights'][rank])

    def test_wrong_weight_extent_rank_or_path_join_refused(self):
        for mutate in (lambda rows: rows.reverse(), lambda rows: rows[0].update(bytes=1),
                       lambda rows: rows[1].update(path=rows[0]['path'])):
            values = fixture()
            weights = values[6]['output_weights']
            mutate(weights)
            with self.assertRaises(RuntimeError):
                O.numerical_plans(values[4], values[0], CASE, weights)

    def test_plan_requires_new_label_exact_case_and_owned_result_locations(self):
        plan = fixture()[6]
        self.assertEqual(O.plan_shape(plan, ROOT), DIRECTORY)
        for key, value in (('schema', 'legacy'), ('case', 'patterned-pos0'),
            ('case_directory', str(DIRECTORY).replace('independent-profile', 'parity')),
            ('native_result', pin(ROOT / 'result.json')), ('extra', False)):
            with self.assertRaises(RuntimeError):
                O.plan_shape(dict(plan, **{key: value}), ROOT)

    def test_owned_leaf_replay_keeps_command_deadline_reap_and_no_retry_gates(self):
        plan = fixture()[6]
        directory = DIRECTORY / 'native'
        env = dict(PATH='/usr/bin:/bin', HIP_VISIBLE_DEVICES='')
        command = dict(argv=[plan['binary']['path'], O.EXECUTE, plan['request']['path'], plan['request']['sha256']],
            env=env, cwd='/synthetic', deadline_seconds=180, affinity=[8, 9], nice=10,
            address_space_bytes=12 << 30, stream_cap_bytes=8 << 20, gpu_execution_requested=True)
        command_pin = pin(directory / 'command.json', json.dumps(command).encode())
        start_pin = pin(directory / 'started.json')
        documents = {command_pin['path']: command, start_pin['path']: dict(command_sha256=command_pin['sha256'])}
        result = dict(exit_code=0, reason=None, owned_groups_absent=True, owned_processes_reaped=True,
            cleanup_signalled=False, command=command_pin, started=start_pin,
            stdout=pin(directory / 'stdout', b'{}'), stderr=pin(directory / 'stderr', b''),
            gpu_execution_requested=True)
        P = types.SimpleNamespace(ENV=env, R=Path('/synthetic'),
            document=lambda pins, row: documents[row['path']],
            read=lambda pins, row, *args, **kwargs: b'' if row == result['stderr'] else b'{}')
        self.assertEqual(O.replay_leaf(P, None, result, plan['binary'], plan['request'],
                                      directory, O.EXECUTE, True), {})
        for key, value in (('exit_code', True), ('exit_code', 1), ('reason', 'deadline'),
            ('owned_groups_absent', False), ('owned_processes_reaped', False), ('cleanup_signalled', True),
            ('gpu_execution_requested', False)):
            with self.assertRaises(RuntimeError):
                O.replay_leaf(P, None, dict(result, **{key: value}), plan['binary'],
                              plan['request'], directory, O.EXECUTE, True)
        for key, value in (('deadline_seconds', 181), ('argv', command['argv'][:1] + ['--execute'] + command['argv'][2:]),
                           ('env', {})):
            documents[command_pin['path']] = dict(command, **{key: value})
            with self.assertRaises(RuntimeError):
                O.replay_leaf(P, None, result, plan['binary'], plan['request'], directory, O.EXECUTE, True)
        documents[command_pin['path']] = command

    def test_reference_result_capture_rank_stage_and_authority_joins(self):
        values = fixture()
        checked = check(values)
        plans = O.numerical_plans(values[4], values[0], CASE, values[6]['output_weights'])
        numerical, _ = mocked_numerical(plans, checked['stage_sha256'])
        good = numerical.compare_profile(plans[1], None, None)
        O.numerical_result(good, plans[1], checked['stage_sha256'][1], numerical)
        mutations = [lambda row: row.update(profile='baseline_v5'),
            lambda row: row.update(independent_numerical_acceptance=True),
            lambda row: row['rows'][1].update(rank=True),
            lambda row: row['rows'][0].update(capture=plans[0]['ranks'][0]['capture']),
            lambda row: row['rows'][0].update(stage_sha256=['f' * 64] * 7)]
        for mutate in mutations:
            changed = copy.deepcopy(good)
            mutate(changed)
            with self.assertRaises(RuntimeError):
                O.numerical_result(changed, plans[1], checked['stage_sha256'][1], numerical)

    def compare_fixture(self, fail=None, changed=False):
        values = fixture()
        hashes = check(values)['stage_sha256']
        plans = O.numerical_plans(values[4], values[0], CASE, values[6]['output_weights'])
        numerical, calls = mocked_numerical(plans, hashes, fail)
        read, pins = Tracker(changed), Tracker()
        custody = types.SimpleNamespace(read=lambda *args, **kwargs: None)
        return plans, hashes, numerical, calls, read, pins, custody

    def test_success_routes_both_profiles_to_separate_reference_calls_and_rechecks(self):
        plans, hashes, numerical, calls, read, pins, custody = self.compare_fixture()
        rows = O.compare_plans(plans, hashes, custody, pins, read, numerical, None)
        self.assertEqual(calls, list(O.PROFILES))
        self.assertTrue(all(row['conditional_operator_checks_passed'] for row in rows))
        self.assertEqual((read.calls, pins.calls), (2, 2))

    def test_baseline_reference_rejection_does_not_accept_or_skip_candidate(self):
        plans, hashes, numerical, calls, read, pins, custody = self.compare_fixture('baseline_v5')
        rows = O.compare_plans(plans, hashes, custody, pins, read, numerical, None)
        self.assertEqual(calls, list(O.PROFILES))
        self.assertFalse(rows[0]['conditional_operator_checks_passed'])
        self.assertIsNone(rows[0]['checked'])
        self.assertIn('bound rejection', rows[0]['error'])
        self.assertTrue(rows[1]['conditional_operator_checks_passed'])

    def test_candidate_reference_rejection_preserves_only_conditional_baseline_result(self):
        plans, hashes, numerical, calls, read, pins, custody = self.compare_fixture('tiles_v6')
        rows = O.compare_plans(plans, hashes, custody, pins, read, numerical, None)
        self.assertTrue(rows[0]['conditional_operator_checks_passed'])
        self.assertFalse(rows[1]['conditional_operator_checks_passed'])
        self.assertEqual(calls, list(O.PROFILES))

    def test_changed_reader_input_is_fatal_not_a_numerical_failure_to_skip(self):
        plans, hashes, numerical, calls, read, pins, custody = self.compare_fixture(changed=True)
        with self.assertRaisesRegex(RuntimeError, 'changed input'):
            O.compare_plans(plans, hashes, custody, pins, read, numerical, None)
        self.assertEqual(calls, ['baseline_v5'])

    def test_first_underlying_reader_failure_is_fatal_before_it_registers_a_pin(self):
        plans, hashes, numerical, calls, read, pins, custody = self.compare_fixture()
        attempted = pin(ROOT / 'first-input.bin', b'original')

        class FailedFirstReader(Tracker):
            def __call__(self, record, maximum):
                self.asserted_record = record
                raise RuntimeError('first read digest mismatch')

        failing = FailedFirstReader()

        def compare(plan, guarded, frozen):
            calls.append(plan['profile'])
            guarded(attempted, 64 << 10)
            raise AssertionError('unreachable after failed input authentication')

        numerical.compare_profile = compare
        with self.assertRaisesRegex(O.NumericalCustodyError, 'first read digest mismatch'):
            O.compare_plans(plans, hashes, custody, pins, failing, numerical, None)
        self.assertEqual(failing.records, {})
        self.assertEqual(failing.asserted_record, attempted)
        self.assertEqual(calls, ['baseline_v5'])

    def test_first_custody_pin_failure_prevents_underlying_reader_and_candidate(self):
        plans, hashes, numerical, calls, read, pins, custody = self.compare_fixture()
        attempted = pin(ROOT / 'missing-input.bin', b'original')
        observed = []

        def refused_read(*args, **kwargs):
            raise RuntimeError('missing pinned input')

        class UntouchedReader(Tracker):
            def __call__(self, record, maximum):
                observed.append(record)
                return b'original'

        def compare(plan, guarded, frozen):
            calls.append(plan['profile'])
            guarded(attempted, 64 << 10)

        numerical.compare_profile = compare
        custody.read = refused_read
        with self.assertRaisesRegex(O.NumericalCustodyError, 'missing pinned input'):
            O.compare_plans(plans, hashes, custody, pins, UntouchedReader(), numerical, None)
        self.assertEqual(calls, ['baseline_v5'])
        self.assertEqual(observed, [])

    def test_guarded_reader_records_attempt_and_rejects_bad_bytes_or_conflicting_pin(self):
        attempted = pin(ROOT / 'input.bin', b'original')
        custody = types.SimpleNamespace(read=lambda *args, **kwargs: None)
        guarded = O.CustodyReader(custody, None, lambda *args: b'wrong')
        with self.assertRaises(O.NumericalCustodyError):
            guarded(attempted)
        self.assertEqual(guarded.records, {attempted['path']: attempted})
        guarded = O.CustodyReader(custody, None, lambda *args: b'original')
        self.assertEqual(guarded(attempted), b'original')
        with self.assertRaisesRegex(O.NumericalCustodyError, 'conflicting requested'):
            guarded(dict(attempted, sha256='0' * 64))

    def test_no_deployment_audit_gpu_or_numerical_authority(self):
        for name in ('deployment_verified', 'platform_audits_verified', 'gpu_execution_verified',
                     'arithmetic_prerequisites_verified', 'independent_numerical_acceptance',
                     'full_model_correctness', 'production_authority', 'performance_claim'):
            self.assertIn(name, O.FALSE_FIELDS)
        self.assertEqual(O.NUMERICAL_SHA, '4b72aeac6167006151b258536cc834f39e7a55da15ec10744df2af438bf3b46d')


if __name__ == '__main__':
    unittest.main()
