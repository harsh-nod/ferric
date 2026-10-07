"""Pure serial admission/ordering and mocked case-timer regressions; no subprocesses."""
import copy
import unittest
from unittest import mock

import run_model_gpu as R
import run_serial as S


def plans():
    values, requests = {}, {}
    for index, name in enumerate(S.ORDER):
        request = dict(schema='FerricFiniteGuardedMlpDecodeRequestV1',
            decode=dict(mode='autoregressive', session=[index + 1] * 32,
                evidence_directory=str(S.ROOT / name / 'native'), worker='same-worker',
                source='same-model', prompt={'input': 'same-prompt'}, images={'prefix': 'same-image'}),
            projection_image='same-projection', guarded_image='same-guarded')
        requests[name] = request
        values[name] = dict(schema='ferric-guarded-mlp-peer-read-pair-gpu-input-v2', mode='ar4',
            case=name, paired_read=name.startswith('paired-'), parent_cpu='same-parent-cpu',
            worker_cpu='same-worker-cpu', parent='same-parent', worker='same-worker', request=name)
    return values, requests


class SerialTests(unittest.TestCase):
    def test_only_declared_literal_bindings_may_change_tested_source(self):
        source = b'PIN = None\nOTHER = 7\ndef run(x):\n    return x + OTHER\n'
        S.qualified_source(source, source.replace(b'PIN = None', b"PIN = {'sha256': 'actual'}"), {'PIN'})
        for changed in (source.replace(b'OTHER = 7', b'OTHER = 8'),
                        source.replace(b'return x + OTHER', b'return x'),
                        source.replace(b'PIN = None', b'RENAMED = None')):
            with self.assertRaises(RuntimeError): S.qualified_source(source, changed, {'PIN'})

    def test_four_plans_keep_same_elf_inputs_and_distinct_sessions(self):
        values, requests = plans()
        S.input_contract(values, requests)
        self.assertEqual(len({tuple(r['decode']['session']) for r in requests.values()}), 4)

    def test_plan_roster_policy_history_and_same_input_drift_refuse(self):
        for field in range(10):
            values, requests = plans(); name = S.ORDER[2]
            if field == 0: values.pop(name)
            elif field == 1: values[name]['paired_read'] = False
            elif field == 2: values[name]['paired_read'] = 1
            elif field == 3: values[name]['worker'] = 'other'
            elif field == 4: requests[name]['decode']['source'] = 'other'
            elif field == 5: requests[name]['decode']['session'] = [1] * 32
            elif field == 6: requests[name]['decode']['mode'] = 'teacher_forced'
            elif field == 7: requests[name]['decode']['evidence_directory'] = str(S.ROOT / 'control-0/native')
            elif field == 8: requests[name]['decode']['session'][0] = True
            else: requests[name]['guarded_image'] = 'other-image'
            with self.assertRaises(RuntimeError): S.input_contract(values, requests)

    def test_serial_admits_before_next_case_and_records_real_order(self):
        effects = []
        def invoke(name, begin):
            effects.append(('invoke', name)); return 0
        def admit(name):
            effects.append(('admit', name)); return {'path': name, 'bytes': 1, 'sha256': '1' * 64}
        def record(row): effects.append(('record', row['name']))
        clock = iter(range(100, 108))
        rows = S.drive_cases(invoke, admit, lambda: next(clock), record)
        self.assertEqual(effects, [(action, name) for name in S.ORDER for action in ('invoke', 'admit', 'record')])
        self.assertEqual([row['name'] for row in rows], list(S.ORDER))
        self.assertEqual([(row['started_monotonic_ns'], row['finished_monotonic_ns']) for row in rows],
                         [(100, 101), (102, 103), (104, 105), (106, 107)])

    def test_first_case_failure_or_admission_error_stops_without_retry(self):
        for failed in range(4):
            for admission in (False, True):
                calls, records = [], []
                def invoke(name, begin):
                    calls.append(name)
                    return 1 if not admission and name == S.ORDER[failed] else 0
                def admit(name):
                    if admission and name == S.ORDER[failed]: raise RuntimeError('failed actual terminal')
                    return {'path': name}
                clock = iter(range(100, 108))
                with self.assertRaises(RuntimeError):
                    S.drive_cases(invoke, admit, lambda: next(clock), records.append)
                self.assertEqual(calls, list(S.ORDER[:failed + 1]))
                self.assertEqual(len(records), failed)

    def test_nonmonotonic_or_noninteger_clocks_refuse(self):
        for values in ([100, 100], [100, 101, 100], [True], [-1], [1 << 64], [100, 1.5]):
            ticks = iter(values)
            with self.assertRaises(RuntimeError):
                S.drive_cases(lambda *_: 0, lambda name: {'path': name}, lambda: next(ticks), lambda _: None)

    def test_case_restores_signal_handlers_and_outer_timer_on_postcheck_exception(self):
        saved = {sig: object() for sig in R.SIGNALS}
        calls = []
        with mock.patch.object(R.signal, 'getsignal', side_effect=lambda sig: saved[sig]), \
             mock.patch.object(R.signal, 'getitimer', return_value=(500, 0)), \
             mock.patch.object(R.signal, 'signal', side_effect=lambda sig, handler: calls.append((sig, handler))), \
             mock.patch.object(R.signal, 'setitimer') as timer, \
             mock.patch.object(R.time, 'monotonic', side_effect=[100, 101, 102]), \
             mock.patch.object(R, '_run_case', side_effect=RuntimeError('postcheck read failed')):
            with self.assertRaisesRegex(RuntimeError, 'postcheck read failed'):
                R.run_case('control-0')
        self.assertEqual(calls[-4:], list(saved.items()))
        self.assertEqual(timer.call_args_list[-2:], [mock.call(R.signal.ITIMER_REAL, 0),
                                                    mock.call(R.signal.ITIMER_REAL, 498, 0)])

    def test_expired_case_deadline_refuses_without_disabling_or_rearming_timer(self):
        with mock.patch.object(R.time, 'monotonic', return_value=100), \
             mock.patch.object(R.signal, 'signal') as handler, \
             mock.patch.object(R.signal, 'setitimer') as timer:
            with self.assertRaisesRegex(RuntimeError, 'whole case deadline'):
                R.case_deadline(100)
            handler.assert_not_called(); timer.assert_not_called()


if __name__ == '__main__':
    unittest.main(verbosity=2)
