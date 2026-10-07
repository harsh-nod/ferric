"""Pure policy, boundary and lifecycle regressions; no native subprocesses."""
import copy
import hashlib
import json
import signal
import struct
import unittest
from unittest import mock

import run_model_gpu as runner
import validate_observation as v
from test_census import fixture


def paired_fixture():
    bootstrap, arena = fixture()
    bootstrap['schema'] = 'FerricGuardedMlpReusableAr4PairedTerminalBootstrapV1'
    arena['profile_sha256'] = list(v.profile(bootstrap))
    return bootstrap, dict(schema='FerricGuardedMlpReusableAr4PairedTerminalCensusV1',
        arena=arena, paired_terminal_dispatches=[0, 0, 36, 36], first_use_legacy=True,
        warm_retired_only=True, global_currentness_policy_changed=False,
        terminal_currentness_cadence_changed=True, performance_claim=False)


def encoded(value):
    return json.dumps(value, sort_keys=True, allow_nan=False).encode()


def identity_fixture():
    pin = lambda k: dict(path='/synthetic/' + k, bytes=1, sha256='a' * 64)
    plan = {k: pin(k) for k in ('parent_cpu', 'worker_cpu', 'parent', 'worker')}
    controller = pin('runner')
    request = dict(decode=dict(session=[2] * 32))
    value = dict(schema='ferric-guarded-mlp-warm-paired-terminal-gpu-v1', passed=True,
        errors=[], case='control', mode='ar4', paired_terminal_requested=False,
        native_attempts=1, retries=0, controller=controller, admission=copy.deepcopy(plan),
        default_full_currentness_requested=True, shared_full_currentness_requested=False,
        host_observation_requested=False, hidden_read_policy_changed=False,
        paired_terminal_dispatches=[0, 0, 0, 0],
        ordinary_comparison=dict(all_payloads_equal=True, all_histories_equal=True),
        observation=dict(actual_arena_census=dict(session=[1] * 32)))
    return value, plan, request, controller


class TerminalTests(unittest.TestCase):
    def test_distinct_profile_and_nested_census_preserve_legacy(self):
        old_bootstrap, old_census = fixture()
        bootstrap, census = paired_fixture()
        expected = hashlib.sha256(b'ferric-guarded-mlp-reusable-ar4-paired-terminal-profile-v1\0'
                                  + bytes(old_census['profile_sha256'])).digest()
        self.assertEqual(v.profile(bootstrap), expected)
        self.assertNotEqual(v.profile(old_bootstrap), expected)
        self.assertEqual(v.arena_census(encoded(census), bootstrap), census)
        self.assertEqual(v.arena_census(encoded(old_census), old_bootstrap), old_census)

    def test_profile_schema_and_unknown_fields_cannot_cross_modes(self):
        bootstrap, census = paired_fixture()
        old_bootstrap, old_census = fixture()
        for value, b in [(census, old_bootstrap), (old_census, bootstrap),
                         (dict(census, extra=1), bootstrap),
                         (dict(census, arena=dict(census['arena'], extra=1)), bootstrap)]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                v.arena_census(encoded(value), b)
        raw = encoded(census)
        with self.assertRaises(ValueError):
            v.arena_census(b'{"warm_retired_only":true,' + raw[1:], bootstrap)
        changed = copy.deepcopy(bootstrap)
        changed['decode']['mode'] = 'teacher_forced'
        with self.assertRaises(ValueError): v.profile(changed)

    def test_only_two_warm_generations_may_report_paired_dispatches(self):
        bootstrap, census = paired_fixture()
        for counts in ([36, 0, 36, 36], [0, 36, 36, 36], [0, 0, 35, 36],
                       [0, 0, 36, 35], [0, 0, 0, 0], [0, 0, 36],
                       [0, 0, 36, 36, 0], [False, 0, 36, 36], [0, 0, 36.0, 36]):
            changed = dict(census, paired_terminal_dispatches=counts)
            self.assertNotEqual(encoded(changed), encoded(census))
            with self.subTest(counts=counts), self.assertRaises(ValueError):
                v.arena_census(encoded(changed), bootstrap)

    def test_nested_close_plateau_and_cadence_flags_are_strict(self):
        bootstrap, census = paired_fixture()
        changes = []
        for key in ('first_use_legacy', 'warm_retired_only', 'terminal_currentness_cadence_changed'):
            changes += [dict(census, **{key: False}), dict(census, **{key: 1})]
        for key in ('global_currentness_policy_changed', 'performance_claim'):
            changes += [dict(census, **{key: True}), dict(census, **{key: 0})]
        for key, value in [('native_closed', False), ('completed_forwards', 3),
                           ('profile_sha256', [0] * 32), ('registration', [0] * 32)]:
            changes.append(dict(census, arena=dict(census['arena'], **{key: value})))
        changed = copy.deepcopy(census); changed['arena']['allocation_counts'][4][0] += 1
        changes.append(changed)
        for changed in changes:
            self.assertNotEqual(encoded(changed), encoded(census))
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                v.arena_census(encoded(changed), bootstrap)

    def test_segment_host_fields_use_fixed_u64_offsets_not_gpu_time(self):
        raw = bytearray(v.CONTROL_BYTES)
        expected = [0, (1 << 64) - 1] + [100 + i for i in range(34)]
        # Independent wire cursor: header; prefix[2], mlp[2], guards[2], prefix times[2].
        cursor = 16
        for value in expected:
            cursor += 2 * 284 * 4 + 2 * 548 * 4 + 2 * 4 * 4
            struct.pack_into('<QQ', raw, cursor, 17, 19); cursor += 16
            struct.pack_into('<Q', raw, cursor, value); cursor += 8 + 32
        self.assertEqual(cursor + 24, len(raw))
        self.assertEqual(v.segment_host_ns(raw), expected)
        for changed in (raw[:-1], raw + b'\0'):
            with self.assertRaises(ValueError): v.segment_host_ns(changed)

    def test_tested_runner_binding_normalization_is_narrow_and_literal(self):
        names = ['PLAN_SHA', 'WORKER', 'WORKER_CPU', 'PARENT', 'PARENT_CPU',
                 'CHECKER_CPU', 'CHECKER_CONTROLLER']
        source = '\n'.join(name + ' = None' for name in names) + '\ndef f():\n    return 7\n'
        changed = source.replace('WORKER = None', "WORKER = {'bytes': 1, 'sha256': 'a'}")
        self.assertEqual(runner.normalize_bindings(source), runner.normalize_bindings(changed))
        self.assertNotEqual(runner.normalize_bindings(source),
                            runner.normalize_bindings(source.replace('return 7', 'return 8')))
        for bad in (source + '\nWORKER = None\n', source.replace('WORKER = None', ''),
                    source.replace('WORKER = None', 'WORKER = open("/bad")')):
            with self.assertRaises((ValueError, RuntimeError)): runner.normalize_bindings(bad)

    def test_control_requires_same_elf_policy_success_and_distinct_session(self):
        value, plan, request, controller = identity_fixture()
        runner.control_identity(value, plan, request, controller)
        changes = [('passed', False), ('case', 'candidate'), ('paired_terminal_requested', True),
                   ('native_attempts', 2), ('shared_full_currentness_requested', True),
                   ('host_observation_requested', True), ('hidden_read_policy_changed', True)]
        for key, item in changes:
            changed = dict(value, **{key: item})
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                runner.control_identity(changed, plan, request, controller)
        for key in ('parent', 'worker', 'parent_cpu', 'worker_cpu'):
            changed = copy.deepcopy(value); changed['admission'][key]['sha256'] = 'b' * 64
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                runner.control_identity(changed, plan, request, controller)
        request['decode']['session'] = [1] * 32
        with self.assertRaises(RuntimeError): runner.control_identity(value, plan, request, controller)

    def test_case_exception_restores_outer_handlers_and_remaining_timer(self):
        marker = object()
        with mock.patch.object(runner.signal, 'getsignal', return_value=marker), \
             mock.patch.object(runner.signal, 'signal') as install, \
             mock.patch.object(runner.signal, 'getitimer', return_value=(100.0, 0.0)), \
             mock.patch.object(runner.signal, 'setitimer') as timer, \
             mock.patch.object(runner.time, 'monotonic', side_effect=[10.0, 10.0, 12.0]), \
             mock.patch.object(runner, '_run_case', side_effect=RuntimeError('synthetic')):
            with self.assertRaisesRegex(RuntimeError, 'synthetic'): runner.run_case('control')
        self.assertEqual(timer.call_args_list[-1], mock.call(signal.ITIMER_REAL, 98.0, 0.0))
        for number in runner.SIGNALS:
            self.assertIn(mock.call(number, marker), install.call_args_list)


if __name__ == '__main__':
    unittest.main(verbosity=2)
