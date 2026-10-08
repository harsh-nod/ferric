"""Synthetic three-record admission mutations, never native evidence."""
import copy
import hashlib
import json
import unittest

import test_readiness as R
import test_shared as H
import test_matched as K
import test_duration as O
import validate_forward as F


def record(policy, diagnostic):
    rows = []
    for old in diagnostic['forwards']:
        phases = [1] * len(F.PHASES)
        if old['measured'] is not None:
            measured = old['measured']
            phases[3] = measured['bank_guarded_body_ns']
            phases[4] = sum(measured['layers'][key]['elapsed_ns'] for key in F.D.CATEGORIES)
            phases[5] = sum(measured['tail'][key]['elapsed_ns'] for key in F.D.CATEGORIES)
        rows.append(dict(position=old['position'], phase_ns=phases, forward_body_ns=sum(phases)))
    return dict(schema=F.RECORD_SCHEMA, instrumented=True,
        policy_sha256=list(hashlib.sha256(R.wire(policy) + b'\n').digest()),
        currentness_record_sha256=list(hashlib.sha256(R.wire(diagnostic) + b'\n').digest()),
        session=policy['session'], worker_sha256=policy['worker_sha256'],
        transcript_sha256=policy['transcript_sha256'], phase_order=list(F.PHASES), forwards=rows,
        host_elapsed_nanoseconds=True, disjoint_phases=True, currentness_durations_nested=True,
        gpu_timing=False, numerical_acceptance=False, performance_claim=False, execution_authority=False)


def wire(policy, diagnostic, forward):
    return O.wire(policy, diagnostic) + R.wire(forward) + b'\n'


def install(case, raw):
    H.install(case['value'], raw)
    updated = K.make_case('shared', case['value'], case['root'])
    updated['mode'] = F.MODE
    updated['wrapper']['schema'] = F.T.WRAPPERS['tail_timed']
    updated['wrapper']['currentness_policy'] = case['wrapper']['currentness_policy']
    return updated


def fixture():
    case = O.fixture()
    summary = case['value'][0]
    policy = O.policy_record(summary)
    diagnostic = O.record(policy)
    return install(case, wire(policy, diagnostic, record(policy, diagnostic)))


def check(case):
    _, request, bodies, prompt = case['value']
    return F.validate(case['mode'], R.wire(case['wrapper']), case['summary'], request,
                      case['root'], lambda pin: bodies[pin['path']], prompt)


class ForwardTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case = fixture()
        check(cls.case)  # Every mutation begins from genuine full admission.

    def records(self):
        summary = self.case['value'][0]
        policy = O.policy_record(summary)
        diagnostic = O.record(policy)
        return summary, policy, diagnostic, record(policy, diagnostic)

    def test_complete_case_preserves_first_two_records_and124_spans(self):
        case = copy.deepcopy(self.case)
        before = copy.deepcopy(case['value'][2])
        got = check(case)
        self.assertEqual(before, case['value'][2])
        self.assertEqual(got['schema'], F.SCHEMA)
        self.assertEqual(got['mode'], 'tail_forward')
        self.assertEqual(got['timing']['disjoint_spans'], 124)
        self.assertEqual(got['ordinary']['completed_forwards'], 40)
        self.assertEqual(got['ordinary']['generated_tokens'], [])
        policy, diagnostic, forward = (got['policy'][name] for name in
            ('policy_record', 'diagnostic_record', 'forward_record'))
        raw = case['value'][2][case['value'][0]['files']['child_stderr']['path']]
        first_two = b'\n'.join(raw.split(b'\n')[:2]) + b'\n'
        self.assertEqual(first_two, O.wire(policy, diagnostic))
        self.assertEqual(F.D.validate_record(first_two, case['value'][0]),
                         dict(policy=policy, diagnostic=diagnostic))
        self.assertEqual(forward, got['ordinary']['forward_phase_diagnostic'])
        self.assertEqual([row['measured'] for row in diagnostic['forwards'][:2]], [None, None])
        self.assertEqual(len(forward['forwards']), 40)
        for name in ('instrumented', 'parent_host_measurement', 'forward_phase_durations',
                     'currentness_durations_nested', 'disjoint_forward_phases'):
            self.assertIs(got[name], True)
        for name in ('gpu_timing', 'full_long_workload', 'numerical_acceptance', 'performance_claim',
                     'production_authority', 'outer_owned_lineage_checked', 'cpu_qualification_checked'):
            self.assertIs(got[name], False)

    def test_exact_three_lf_lines_and_canonical_json(self):
        summary, policy, diagnostic, forward = self.records()
        first, second, third = (R.wire(value) + b'\n' for value in (policy, diagnostic, forward))
        for raw in (b'', first, first + second, first + second + third[:-1],
                    first + second + third + b'\n', first + second + third + third,
                    second + first + third, first + third + second, first + second + b' ' + third,
                    first + second + json.dumps(forward, indent=2).encode() + b'\n',
                    (first + second + third).replace(b'\n', b'\r\n'),
                    first + second + third.replace(b'{', b'{"schema":"duplicate",', 1)):
            with self.subTest(raw=raw[:24]), self.assertRaises(ValueError):
                F.validate_record(raw, summary)

    def test_third_and_combined_caps_are_enforced_before_parsing(self):
        summary, policy, diagnostic, forward = self.records()
        prefix = O.wire(policy, diagnostic)
        with self.assertRaisesRegex(ValueError, 'bounded third'):
            F.validate_record(prefix + b' ' * F.MAX_BYTES + b'\n', summary)
        with self.assertRaisesRegex(ValueError, 'bounded entire'):
            F.validate_record(b' ' * (F.STDERR_MAX_BYTES + 1), summary)
        self.assertEqual(F.STDERR_MAX_BYTES, F.D.STDERR_MAX_BYTES)
        self.assertEqual((F.MAX_BYTES, F.STDERR_MAX_BYTES), (32768, 69632))
        for wrong in (bytearray(wire(policy, diagnostic, forward)), '', None):
            with self.assertRaises(ValueError): F.validate_record(wrong, summary)

    def test_old_routes_and_three_record_route_are_distinct(self):
        summary, policy, diagnostic, forward = self.records()
        raw = wire(policy, diagnostic, forward)
        for old in (F.D.validate_record, F.T.validate_record):
            with self.assertRaises(ValueError): old(raw, summary)
        with self.assertRaises(ValueError): F.validate_record(O.wire(policy, diagnostic), summary)
        for mode in ('tail_duration', 'tail_timed', 'census_timed', 'default', 'shared', 'full2303', True):
            case = copy.deepcopy(self.case)
            case['mode'] = mode
            with self.subTest(mode=mode), self.assertRaises(ValueError): check(case)

    def test_each_identity_and_lf_inclusive_original_hash_is_required(self):
        summary, policy, diagnostic, forward = self.records()
        for key in ('policy_sha256', 'currentness_record_sha256', 'session',
                    'worker_sha256', 'transcript_sha256'):
            changed = copy.deepcopy(forward)
            changed[key][0] ^= 1
            with self.subTest(key=key), self.assertRaises(ValueError):
                F.validate_record(wire(policy, diagnostic, changed), summary)
        for key, original in (('policy_sha256', policy), ('currentness_record_sha256', diagnostic)):
            changed = copy.deepcopy(forward)
            changed[key] = list(hashlib.sha256(R.wire(original)).digest())
            with self.subTest(key=key), self.assertRaises(ValueError):
                F.validate_record(wire(policy, diagnostic, changed), summary)
        for wrong in ([0] * 31, [True] * 32, [256] * 32, '0' * 64):
            changed = copy.deepcopy(forward)
            changed['currentness_record_sha256'] = wrong
            with self.assertRaises(ValueError): F.validate_record(wire(policy, diagnostic, changed), summary)

    def test_schema_and_all_authority_flags_are_exact_typed_values(self):
        summary, policy, diagnostic, forward = self.records()
        for key in ('instrumented', 'host_elapsed_nanoseconds', 'disjoint_phases',
                    'currentness_durations_nested', 'gpu_timing', 'numerical_acceptance',
                    'performance_claim', 'execution_authority'):
            for wrong in (not forward[key], int(forward[key])):
                changed = copy.deepcopy(forward)
                changed[key] = wrong
                with self.subTest(key=key, wrong=wrong), self.assertRaises(ValueError):
                    F.validate_record(wire(policy, diagnostic, changed), summary)
        for wrong in (F.D.RECORD_SCHEMA, 'FerricFull2303ForwardPhaseDurationsV1', None):
            changed = copy.deepcopy(forward)
            changed['schema'] = wrong
            with self.assertRaises(ValueError): F.validate_record(wire(policy, diagnostic, changed), summary)

    def test_exact_forty_ordered_rows_without_aliasing_or_duplicate_positions(self):
        summary, policy, diagnostic, forward = self.records()
        for mutation in range(6):
            changed = copy.deepcopy(forward)
            rows = changed['forwards']
            if mutation == 0: rows.pop()
            elif mutation == 1: rows.append(copy.deepcopy(rows[-1]))
            elif mutation == 2: rows[2], rows[3] = rows[3], rows[2]
            elif mutation == 3: rows[3] = copy.deepcopy(rows[2])
            elif mutation == 4: rows[0]['position'] = False
            else: changed['forwards'] = {}
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                F.validate_record(wire(policy, diagnostic, changed), summary)

    def test_nine_phase_order_extent_and_names_are_fixed(self):
        summary, policy, diagnostic, forward = self.records()
        for wrong in (list(reversed(F.PHASES)), list(F.PHASES[:-1]), list(F.PHASES) + ['extra'],
                      ['input'] * 9, 'input metadata embedding bank layers tail frame fence commit'):
            changed = copy.deepcopy(forward)
            changed['phase_order'] = wrong
            with self.assertRaises(ValueError): F.validate_record(wire(policy, diagnostic, changed), summary)
        for wrong in ([1] * 8, [1] * 10, None, {'input': 1}):
            changed = copy.deepcopy(forward)
            changed['forwards'][0]['phase_ns'] = wrong
            with self.assertRaises(ValueError): F.validate_record(wire(policy, diagnostic, changed), summary)

    def test_row_integer_types_and_unsigned_widths_are_strict(self):
        summary, policy, diagnostic, forward = self.records()
        for field in ('position', 'phase_ns', 'forward_body_ns'):
            for wrong in (True, 1.0, -1, 1 << 64):
                changed = copy.deepcopy(forward)
                row = changed['forwards'][0]
                if field == 'phase_ns': row[field][0] = wrong
                else: row[field] = wrong
                with self.subTest(field=field, wrong=wrong), self.assertRaises(ValueError):
                    F.validate_record(wire(policy, diagnostic, changed), summary)

    def test_exact_disjoint_sum_overflow_and_per_row_hour_bound(self):
        summary, policy, diagnostic, forward = self.records()
        for mutation in range(4):
            changed = copy.deepcopy(forward)
            row = changed['forwards'][0]
            if mutation == 0: row['forward_body_ns'] += 1
            elif mutation == 1: row['forward_body_ns'] -= 1
            elif mutation == 2:
                row['phase_ns'] = [(1 << 64) - 1, 1] + [0] * 7
                row['forward_body_ns'] = 0
            else:
                row['phase_ns'] = [F.WHOLE_NS + 1] + [0] * 8
                row['forward_body_ns'] = F.WHOLE_NS + 1
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                F.validate_record(wire(policy, diagnostic, changed), summary)

    def test_aggregate_hour_bound_is_not_only_a_per_row_bound(self):
        summary, policy, diagnostic, forward = self.records()
        row = forward['forwards'][0]
        row['phase_ns'] = [F.WHOLE_NS] + [0] * 8
        row['forward_body_ns'] = F.WHOLE_NS
        with self.assertRaisesRegex(ValueError, 'whole forty-forward'):
            F.validate_record(wire(policy, diagnostic, forward), summary)

    def test_warm_bank_layer_and_tail_containment_survives_exact_sum_repinning(self):
        summary, policy, diagnostic, forward = self.records()
        for index in (3, 4, 5):
            changed = copy.deepcopy(forward)
            row = changed['forwards'][2]
            row['phase_ns'][index] -= 1
            row['forward_body_ns'] = sum(row['phase_ns'])
            with self.subTest(index=index), self.assertRaisesRegex(ValueError, 'callback containment'):
                F.validate_record(wire(policy, diagnostic, changed), summary)
        changed = copy.deepcopy(forward)
        row = changed['forwards'][2]
        row['phase_ns'][3] = 4  # Four callbacks fit, but the enclosing bank body is five.
        row['forward_body_ns'] = sum(row['phase_ns'])
        with self.assertRaises(ValueError): F.validate_record(wire(policy, diagnostic, changed), summary)

    def test_old_callback_predicates_cannot_be_laundered_by_a_new_third_hash(self):
        summary, policy, diagnostic, _ = self.records()
        for mutation in range(5):
            changed = copy.deepcopy(diagnostic)
            if mutation == 0: changed['forwards'][0]['measured'] = copy.deepcopy(changed['forwards'][2]['measured'])
            elif mutation == 1: changed['forwards'][2]['measured'] = None
            elif mutation == 2: changed['forwards'][2]['measured']['bank']['before']['calls'] += 1
            elif mutation == 3: changed['performance_claim'] = True
            else: changed['policy_sha256'][0] ^= 1
            third = record(policy, changed)
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                F.validate_record(wire(policy, changed, third), summary)

    def test_unknown_fields_and_canonical_nested_order_are_rejected(self):
        summary, policy, diagnostic, forward = self.records()
        for mutation in range(4):
            changed = copy.deepcopy(forward)
            if mutation == 0: changed['extra'] = 0
            elif mutation == 1: changed['forwards'][0]['extra'] = 0
            elif mutation == 2: changed = dict(reversed(list(changed.items())))
            else: changed['forwards'][0] = dict(reversed(list(changed['forwards'][0].items())))
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                F.validate_record(wire(policy, diagnostic, changed), summary)

    def test_zero_durations_and_exact_aggregate_boundary_are_valid(self):
        summary, policy, diagnostic, _ = self.records()
        for row in diagnostic['forwards'][2:]:
            measured = row['measured']
            measured['bank_guarded_body_ns'] = 0
            for group in F.D.GROUPS:
                for category in F.D.CATEGORIES:
                    measured[group][category]['elapsed_ns'] = 0
        forward = record(policy, diagnostic)
        for row in forward['forwards']:
            row['phase_ns'] = [0] * 9
            row['forward_body_ns'] = 0
        got = F.validate_record(wire(policy, diagnostic, forward), summary)
        self.assertEqual(sum(row['forward_body_ns'] for row in got['forward']['forwards']), 0)
        forward['forwards'][0]['phase_ns'][0] = F.WHOLE_NS
        forward['forwards'][0]['forward_body_ns'] = F.WHOLE_NS
        got = F.validate_record(wire(policy, diagnostic, forward), summary)
        self.assertEqual(sum(row['forward_body_ns'] for row in got['forward']['forwards']), F.WHOLE_NS)

    def test_large_integer_original_callback_counts_remain_exact(self):
        summary, policy, diagnostic, _ = self.records()
        local = (1 << 53) + 3
        policy['counts']['tails'].update(local_checkpoints=local, before_calls=local + 608,
            after_calls=local + 608, generation_probes=2 * local + 114)
        tail = diagnostic['forwards'][2]['measured']['tail']
        for key, count in zip(F.D.CATEGORIES, F.D.COUNTS):
            tail[key]['calls'] = policy['counts']['tails'][count] - sum(
                row['measured']['tail'][key]['calls'] for row in diagnostic['forwards'][3:])
        diagnostic['policy_sha256'] = list(hashlib.sha256(R.wire(policy) + b'\n').digest())
        got = F.validate_record(wire(policy, diagnostic, record(policy, diagnostic)), summary)
        self.assertGreater(got['diagnostic']['forwards'][2]['measured']['tail']['root_generation']['calls'],
                           1 << 53)

    def test_full_case_rechecks_whole_original_stderr_frames_capture_and_timing(self):
        for field in ('stderr', 'frames', 'capture', 'timing', 'summary', 'policy'):
            case = copy.deepcopy(self.case)
            summary, _, bodies, _ = case['value']
            if field == 'stderr': bodies[summary['files']['child_stderr']['path']] += b'\n'
            elif field == 'frames': bodies[summary['files']['frames']['path']] += b'\n'
            elif field == 'capture': bodies[summary['files']['captures'][0]['file']['path']] += b'\0'
            elif field == 'timing': case['wrapper']['host_timing']['complete'] = False
            elif field == 'summary': case['summary'] += b' '
            else: case['wrapper']['currentness_policy']['native_closed'] = False
            with self.subTest(field=field), self.assertRaises(ValueError): check(case)

    def test_full_case_rejects_semantic_third_drift_even_with_fresh_whole_file_pin(self):
        case = copy.deepcopy(self.case)
        summary, policy, diagnostic, forward = self.records()
        forward['forwards'][2]['phase_ns'][4] = 0
        forward['forwards'][2]['forward_body_ns'] = sum(forward['forwards'][2]['phase_ns'])
        case = install(case, wire(policy, diagnostic, forward))
        self.assertEqual(case['value'][0]['files']['child_stderr'],
                         R.pin(case['value'][0]['files']['child_stderr']['path'],
                               wire(policy, diagnostic, forward)))
        with self.assertRaisesRegex(ValueError, 'callback containment'): check(case)

    def test_same_side_comparison_rechecks_all_original_record_copies(self):
        baseline = K.fixture('default')
        admitted = K.check(baseline)['ordinary']
        case = copy.deepcopy(self.case)
        checked = check(case)
        bodies = {**baseline['value'][2], **case['value'][2]}
        read = lambda pin: bodies[pin['path']]
        parity = F.compare_same_side(case['summary'], baseline['summary'], checked, admitted, read)
        self.assertEqual(parity['schema'], 'ferric-readiness40-tail-forward-duration-same-side-parity-v1')
        self.assertTrue(parity['forward_phase_durations'] and parity['disjoint_forward_phases'])
        self.assertFalse(parity['performance_claim'] or parity['numerical_acceptance'])
        for field in ('policy', 'diagnostic', 'forward', 'ordinary', 'pin', 'mode', 'flag'):
            changed = copy.deepcopy(checked)
            if field == 'policy': changed['policy']['policy_record']['native_closed'] = False
            elif field == 'diagnostic': changed['policy']['diagnostic_record']['instrumented'] = False
            elif field == 'forward': changed['policy']['forward_record']['execution_authority'] = True
            elif field == 'ordinary': changed['ordinary']['forward_phase_diagnostic']['disjoint_phases'] = False
            elif field == 'pin': changed['policy']['file']['sha256'] = '0' * 64
            elif field == 'mode': changed['mode'] = 'tail_duration'
            else: changed['currentness_durations_nested'] = False
            with self.subTest(field=field), self.assertRaises(ValueError):
                F.compare_same_side(case['summary'], baseline['summary'], changed, admitted, read)

    def test_same_side_comparison_rereads_entire_third_line_not_only_old_prefix(self):
        baseline = K.fixture('default')
        admitted = K.check(baseline)['ordinary']
        case = copy.deepcopy(self.case)
        checked = check(case)
        bodies = {**baseline['value'][2], **case['value'][2]}
        path = checked['policy']['file']['path']
        original = bodies[path]
        parts = original.split(b'\n')
        changed = json.loads(parts[2])
        changed['forwards'][0]['phase_ns'][0] += 1
        changed['forwards'][0]['forward_body_ns'] += 1
        bodies[path] = parts[0] + b'\n' + parts[1] + b'\n' + R.wire(changed) + b'\n'
        self.assertEqual(bodies[path].split(b'\n')[:2], original.split(b'\n')[:2])
        with self.assertRaisesRegex(ValueError, 'whole original three-record'):
            F.compare_same_side(case['summary'], baseline['summary'], checked, admitted,
                                lambda pin: bodies[pin['path']])


if __name__ == '__main__':
    unittest.main()

