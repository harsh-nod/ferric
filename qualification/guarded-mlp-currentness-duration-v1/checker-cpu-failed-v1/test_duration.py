"""Synthetic instrumented Tail mutations; no measured GPU or model result."""
import copy
import hashlib
import json
import unittest

import test_readiness as R
import test_shared as H
import test_matched as K
import test_tail as T
import validate_duration as D


def record(policy):
    scopes = {}
    for group, source in zip(D.GROUPS, ('banks', 'layers', 'tails')):
        scopes[group] = {key: dict(calls=policy['counts'][source][count] // 38, elapsed_ns=1)
                         for key, count in zip(D.CATEGORIES, D.COUNTS)}
    rows = [dict(position=p, measured=None if p < 2 else
                 dict(**copy.deepcopy(scopes), bank_guarded_body_ns=5)) for p in range(40)]
    return dict(schema=D.RECORD_SCHEMA, instrumented=True,
        policy_sha256=list(hashlib.sha256(R.wire(policy) + b'\n').digest()),
        session=policy['session'], worker_sha256=policy['worker_sha256'],
        transcript_sha256=policy['transcript_sha256'], forwards=rows,
        host_elapsed_nanoseconds=True, bank_guarded_body_includes_callbacks=True,
        numerical_acceptance=False, performance_claim=False, execution_authority=False)


def wire(policy, diagnostic):
    return R.wire(policy) + b'\n' + R.wire(diagnostic) + b'\n'


def fixture():
    original = T.fixture()
    policy = T.record(original['value'][0])
    H.install(original['value'], wire(policy, record(policy)))
    case = K.make_case('shared', original['value'], original['root'])
    case['mode'] = 'tail_duration'
    case['wrapper']['schema'] = D.T.WRAPPERS['tail_timed']
    case['wrapper']['currentness_policy'] = policy
    return case


def check(case):
    _, request, bodies, prompt = case['value']
    return D.validate(case['mode'], R.wire(case['wrapper']), case['summary'], request,
                      case['root'], lambda p: bodies[p['path']], prompt)


class DurationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.case = fixture()

    def test_complete_case_retains_both_original_records_and124_spans(self):
        case = copy.deepcopy(self.case)
        before = copy.deepcopy(case['value'][2])
        got = check(case)
        self.assertEqual(before, case['value'][2])
        self.assertEqual(got['timing']['disjoint_spans'], 124)
        self.assertEqual(got['ordinary']['completed_forwards'], 40)
        self.assertEqual(got['ordinary']['generated_tokens'], [])
        self.assertEqual(got['policy']['policy_record'], case['wrapper']['currentness_policy'])
        rows = got['policy']['diagnostic_record']['forwards']
        self.assertEqual([r['measured'] for r in rows[:2]], [None, None])
        self.assertEqual(len(rows[2:]), 38)
        self.assertTrue(got['instrumented'] and got['policy']['no_policy_bytes_discarded'])
        for key in ('gpu_timing', 'numerical_acceptance', 'performance_claim', 'production_authority',
                    'full_long_workload', 'outer_owned_lineage_checked', 'cpu_qualification_checked'):
            self.assertIs(got[key], False)

    def test_exact_two_canonical_bounded_original_lines(self):
        summary = self.case['value'][0]
        policy = T.record(summary)
        diagnostic = record(policy)
        first, second = R.wire(policy) + b'\n', R.wire(diagnostic) + b'\n'
        for raw in (b'', first, second, first + second[:-1], first + second + b'\n',
                    first + second + second, second + first, first + b' ' + second,
                    first + json.dumps(diagnostic, indent=2).encode() + b'\n',
                    first + R.wire(dict(reversed(list(diagnostic.items())))) + b'\n',
                    first + second.replace(b'{', b'{"instrumented":true,', 1),
                    b' ' * (D.STDERR_MAX_BYTES + 1), first + b' ' * D.MAX_BYTES + b'\n'):
            with self.subTest(raw=raw[:24]), self.assertRaises(ValueError):
                D.validate_record(raw, summary)

    def test_original_policy_and_instrumented_routes_cannot_admit_each_other(self):
        summary = self.case['value'][0]
        policy = T.record(summary)
        raw = wire(policy, record(policy))
        with self.assertRaises(ValueError): D.T.validate_record(raw, summary)
        with self.assertRaises(ValueError): D.validate_record(R.wire(policy) + b'\n', summary)
        for mode in ('tail_timed', 'census_timed', 'default', 'shared', 'full2303', True):
            case = copy.deepcopy(self.case)
            case['mode'] = mode
            with self.subTest(mode=mode), self.assertRaises(ValueError): check(case)

    def test_every_identity_and_claim_flag_is_bound_to_healthy_policy(self):
        summary = self.case['value'][0]
        policy = T.record(summary)
        for key in ('policy_sha256', 'session', 'worker_sha256', 'transcript_sha256'):
            diagnostic = record(policy)
            diagnostic[key] = diagnostic[key].copy()
            diagnostic[key][0] ^= 1
            with self.subTest(key=key), self.assertRaises(ValueError):
                D.validate_record(wire(policy, diagnostic), summary)
        for key in ('instrumented', 'host_elapsed_nanoseconds', 'bank_guarded_body_includes_callbacks',
                    'numerical_acceptance', 'performance_claim', 'execution_authority'):
            for wrong in (not record(policy)[key], int(record(policy)[key])):
                diagnostic = record(policy)
                diagnostic[key] = wrong
                with self.subTest(key=key, wrong=wrong), self.assertRaises(ValueError):
                    D.validate_record(wire(policy, diagnostic), summary)
        for key in ('native_closed', 'child_exit_zero', 'process_group_absent'):
            changed = copy.deepcopy(summary)
            changed[key] = False
            with self.assertRaises(ValueError):
                D.validate_record(wire(policy, record(policy)), changed)

    def test_fixed_order_presence_and_unmeasured_first_two_forwards(self):
        summary = self.case['value'][0]
        policy = T.record(summary)
        for mutation in range(7):
            diagnostic = record(policy)
            rows = diagnostic['forwards']
            if mutation == 0: rows.pop()
            elif mutation == 1: rows[3]['position'] = 2
            elif mutation == 2: rows[2], rows[3] = rows[3], rows[2]
            elif mutation == 3: rows[0]['measured'] = rows[2]['measured']
            elif mutation == 4: rows[2]['measured'] = None
            elif mutation == 5: rows[0]['position'] = False
            else: rows.append(copy.deepcopy(rows[-1]))
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                D.validate_record(wire(policy, diagnostic), summary)

    def test_every_category_count_joins_each_original_scope(self):
        summary = self.case['value'][0]
        policy = T.record(summary)
        for group in D.GROUPS:
            for key in D.CATEGORIES:
                diagnostic = record(policy)
                diagnostic['forwards'][2]['measured'][group][key]['calls'] += 1
                with self.subTest(group=group, key=key), self.assertRaises(ValueError):
                    D.validate_record(wire(policy, diagnostic), summary)

    def test_per_forward_bank_census_cannot_cancel_between_rows(self):
        summary = self.case['value'][0]
        policy = T.record(summary)
        for key in D.CATEGORIES:
            diagnostic = record(policy)
            diagnostic['forwards'][2]['measured']['bank'][key]['calls'] += 1
            diagnostic['forwards'][3]['measured']['bank'][key]['calls'] -= 1
            with self.subTest(key=key), self.assertRaises(ValueError):
                D.validate_record(wire(policy, diagnostic), summary)

    def test_call_and_duration_types_widths_bounds_and_bank_containment(self):
        summary = self.case['value'][0]
        policy = T.record(summary)
        for field in ('calls', 'elapsed_ns'):
            for wrong in (True, 1.0, -1, 1 << 64):
                diagnostic = record(policy)
                diagnostic['forwards'][2]['measured']['tail']['before'][field] = wrong
                with self.subTest(field=field, wrong=wrong), self.assertRaises(ValueError):
                    D.validate_record(wire(policy, diagnostic), summary)
        for body in (3, D.WHOLE_NS + 1, True, -1):
            diagnostic = record(policy)
            diagnostic['forwards'][2]['measured']['bank_guarded_body_ns'] = body
            with self.assertRaises(ValueError): D.validate_record(wire(policy, diagnostic), summary)
        diagnostic = record(policy)
        tail = diagnostic['forwards'][2]['measured']['tail']
        tail['before']['calls'] = tail['after']['calls'] = D.U64_MAX
        with self.assertRaises(ValueError): D.validate_record(wire(policy, diagnostic), summary)
        diagnostic = record(policy)
        for row in diagnostic['forwards'][2:]:
            row['measured']['layers']['before']['elapsed_ns'] = D.WHOLE_NS
        with self.assertRaises(ValueError): D.validate_record(wire(policy, diagnostic), summary)

    def test_unknown_fields_and_noncanonical_nested_key_order_are_rejected(self):
        summary = self.case['value'][0]
        policy = T.record(summary)
        for level in range(5):
            diagnostic = record(policy)
            values = [diagnostic, diagnostic['forwards'][2], diagnostic['forwards'][2]['measured'],
                      diagnostic['forwards'][2]['measured']['bank'],
                      diagnostic['forwards'][2]['measured']['bank']['before']]
            values[level]['extra'] = 0
            with self.assertRaises(ValueError): D.validate_record(wire(policy, diagnostic), summary)
        diagnostic = record(policy)
        bank = diagnostic['forwards'][2]['measured']['bank']
        diagnostic['forwards'][2]['measured']['bank'] = dict(reversed(list(bank.items())))
        with self.assertRaises(ValueError): D.validate_record(wire(policy, diagnostic), summary)

    def test_large_integer_variable_poll_counts_remain_exact(self):
        summary = self.case['value'][0]
        policy = T.record(summary)
        diagnostic = record(policy)
        local = (1 << 53) + 3
        policy['counts']['tails'].update(local_checkpoints=local, before_calls=local + 608,
            after_calls=local + 608, generation_probes=2 * local + 114)
        tail = diagnostic['forwards'][2]['measured']['tail']
        for key, count in zip(D.CATEGORIES, D.COUNTS):
            tail[key]['calls'] = policy['counts']['tails'][count] - sum(
                row['measured']['tail'][key]['calls'] for row in diagnostic['forwards'][3:])
        diagnostic['policy_sha256'] = list(hashlib.sha256(R.wire(policy) + b'\n').digest())
        got = D.validate_record(wire(policy, diagnostic), summary)
        self.assertGreater(got['diagnostic']['forwards'][2]['measured']['tail']['root_generation']['calls'],
                           1 << 53)

    def test_full_case_rechecks_original_files_capture_and_timing(self):
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

    def test_same_side_comparison_rechecks_both_diagnostic_copies_and_original_file(self):
        baseline = K.fixture('default')
        admitted = K.check(baseline)['ordinary']
        case = copy.deepcopy(self.case)
        checked = check(case)
        bodies = {**baseline['value'][2], **case['value'][2]}
        read = lambda p: bodies[p['path']]
        parity = D.compare_same_side(case['summary'], baseline['summary'], checked, admitted, read)
        self.assertTrue(parity['instrumented'])
        self.assertFalse(parity['performance_claim'] or parity['numerical_acceptance'])
        for mutation in range(4):
            changed = copy.deepcopy(checked)
            if mutation == 0: changed['policy']['diagnostic_record']['instrumented'] = False
            elif mutation == 1: changed['ordinary']['currentness_duration_diagnostic']['execution_authority'] = True
            elif mutation == 2: changed['policy']['file']['sha256'] = '0' * 64
            else: changed['instrumented'] = False
            with self.assertRaises(ValueError):
                D.compare_same_side(case['summary'], baseline['summary'], changed, admitted, read)
        bodies[checked['policy']['file']['path']] += b'\n'
        with self.assertRaises(ValueError):
            D.compare_same_side(case['summary'], baseline['summary'], checked, admitted, read)


if __name__ == '__main__':
    unittest.main()
