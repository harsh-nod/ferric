"""File-backed supervisor mocks; never spawn a subprocess or GPU operation."""
from contextlib import ExitStack
import copy
import json
import os
from pathlib import Path
import resource
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import intake as I
import run as M


def observation(c):
    return dict(schema=M.DV.OBSERVATION_SCHEMA, status='RAW_CLOCKS_AND_EXACT_OUTPUT_INVARIANCE_OBSERVED',
        request=c['plan']['request'], mode='teacher_forced', parent=c['runtime']['parent'],
        worker=c['runtime']['worker'], prefix_image=c['runtime']['image'], device_sidecar={},
        baseline=c['baseline']['receipt_pin'], structural={'finite': True},
        owned_record_checks={'child_pid': 2}, raw_rows=1172, rank_packets=[592, 580], clock_samples=16,
        captured_payloads=4, compared_tensor_rows=152, all_payloads_tokens_and_tensors_equal=True,
        tensor_comparison=[dict(same_input_history=True, tensors=[dict(byte_equal=True)] * 38) for _ in range(4)],
        recorded_close_and_owner_reap_checked=True, raw_completion_ticks=True, raw_clock_counters=True,
        **{key: False for key in M.DV.FALSE})


class RunTests(unittest.TestCase):
    def scenario(self, root, failure=None):
        out = root / 'case'; pins = I.D.Pins(); calls = []
        request = dict(mode='teacher_forced', evidence_directory=str(out / 'native'))
        req = I.save(root / 'request.json', request); pin = I.save(root / 'plan.json', {})
        prior = I.save(root / 'prior.json', {})
        helper = SimpleNamespace(topology_sample=Mock(return_value={'idle': True}), require_idle=Mock())
        observer = SimpleNamespace(quiescent=Mock(), check_platform=Mock(), process_audit=Mock(), SMI_ARGV=['/smi'])
        runtime = dict(parent=dict(path='/actual/new-parent'), worker=dict(path='/actual/worker-cpu669'),
                       image=dict(path='/actual/v7.hsaco', bytes=53560, sha256='ab' * 32))
        c = dict(pins=pins, owned=object(), O=observer, topology=helper, out=out, request=request,
            policy='shared-full-currentness', runtime=runtime,
            plan=dict(request=req, baseline=prior, parent_cpu=pin, worker_cpu=pin,
                parent_runtime_review=pin, worker_runtime_review=pin, decode_review=pin,
                down2_lowering=pin, down2_image=pin, down2_review=pin),
            plan_pin=pin, environment={}, platform={}, baseline=dict(receipt_pin=prior),
            standalone=dict(pins=I.D.Pins()), supervisor_manifest=pin, down2_provenance={})

        def bounded(directory, argv, env, owned, case_root, gpu, seconds):
            calls.append((directory.name, gpu, seconds))
            command = I.save(directory / 'command.json', dict(argv=argv))
            started = I.save(directory / 'started.json', {})
            raw = b'{}\n'; err = b''
            if gpu:
                self.assertEqual(argv, ['/actual/new-parent', '--request', req['path'],
                    '--allow-unauthenticated-machine-code', '--observe-device-clocks'])
                native = out / 'native'; native.mkdir()
                for name in I.BODY: (native / name).write_bytes(b'x')
                (native / 'complete.json').write_bytes(raw)
                if failure != 'missing-device': (out / 'native-device-clock-v2.json').write_bytes(raw)
                err = b'closed producer progress checked by validator\n'
                if failure == 'missing-capture': (native / 'observation-3.bin').unlink()
            (directory / 'stdout').write_bytes(raw); (directory / 'stderr').write_bytes(err)
            value = dict(exit_code=1 if gpu and failure == 'owner' else 0, reason=None,
                cleanup_signalled=False, owned_groups_absent=True, owned_processes_reaped=True,
                lineage=[], command=command, started=started,
                stdout=I.D.read_file(directory / 'stdout')[0], stderr=I.D.read_file(directory / 'stderr')[0])
            return value, I.save(directory / 'result.json', value)

        def observed(context, records, sidecar, leaf, value):
            self.assertIs(context, c); self.assertEqual(set(records), I.BODY | {'complete.json'})
            self.assertEqual(sidecar['path'], str(out / 'native-device-clock-v2.json'))
            self.assertEqual(leaf['path'], str(out / 'parent/result.json'))
            self.assertGreater(value['stderr']['bytes'], 0)
            if failure == 'parser': raise ValueError('raw/Control/input/output join refused')
            result = observation(c)
            if failure == 'incomplete': result['raw_rows'] = 1171
            if failure == 'invariance': result['tensor_comparison'][3]['tensors'][-1] = dict(byte_equal=False)
            if failure == 'legacy': return dict(full152_tensor_rows_bitwise_equal=True)
            if failure == 'authority': result['numerical_acceptance'] = True
            return result

        def guard(_):
            if failure == 'post-drift' and len(calls) == 7: raise RuntimeError('actual input drift')

        def process_audit(_raw, _platform):
            if failure == 'post-audit' and calls[-1][0] == 'after-0': raise RuntimeError('post-audit refused')
            if failure == 'pre-audit' and calls[-1][0] == 'before-1': raise RuntimeError('pre-audit refused')
        observer.process_audit.side_effect = process_audit
        with ExitStack() as stack:
            stack.enter_context(patch.object(M, 'bounded', side_effect=bounded))
            stack.enter_context(patch.object(M, 'resources'))
            stack.enter_context(patch.object(I, 'guard', side_effect=guard))
            check = stack.enter_context(patch.object(M.DV, 'observe', side_effect=observed))
            stack.enter_context(patch.object(M.time, 'sleep'))
            stack.enter_context(patch.object(I.D, 'progress'))
            code = M.execute(c)
        result = json.loads((out / ('complete.json' if code == 0 else 'failure.json')).read_bytes())
        return code, result, calls, out, check.call_count

    def test_one_attempt_retains_all_records_and_six_audits(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, calls, out, checks = self.scenario(Path(name))
            self.assertEqual((code, checks), (0, 1))
            self.assertEqual([r[0] for r in calls], ['before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2'])
            self.assertEqual(sum(r[1] for r in calls), 1)
            self.assertEqual(set(value['retained_native']), I.BODY | {'complete.json'})
            self.assertEqual(value['device_sidecar']['path'], str(out / 'native-device-clock-v2.json'))
            self.assertEqual(value['schema'], 'ferric-p228-down2-clock-gpu-v1')
            self.assertEqual((value['native_attempts'], value['retries']), (1, 0))
            self.assertEqual(value['selected_runtime']['image']['path'], '/actual/v7.hsaco')
            for key in (*M.DV.FALSE, 'gpu_time', 'independent_tensor_acceptance',
                        'independent_numerical_acceptance', 'full_model_correctness', 'sustained_2048_256'):
                self.assertIs(value[key], False)

    def test_owner_parser_and_missing_captures_keep_evidence_without_retry(self):
        for failure in ('owner', 'parser', 'missing-capture', 'missing-device'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as name:
                code, value, calls, out, checks = self.scenario(Path(name), failure)
                self.assertEqual(code, 1); self.assertFalse((out / 'complete.json').exists())
                self.assertEqual(len(value['after_audits']), 3)
                self.assertIn('control-0.bin', value['retained_native'])
                self.assertEqual(sum(row[1] for row in calls), 1)
                self.assertEqual(checks, int(failure == 'parser'))

    def test_incomplete_rows_or_output_mismatch_never_publish_success(self):
        for failure in ('incomplete', 'invariance'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as name:
                code, value, calls, out, checks = self.scenario(Path(name), failure)
                self.assertEqual((code, checks), (1, 1))
                self.assertEqual(len(value['retained_native']), 14)
                self.assertEqual(len(value['after_audits']), 3)
                self.assertTrue((out / 'observation.json').exists())
                self.assertFalse((out / 'complete.json').exists())

    def test_old_parity_and_numerical_authority_never_complete(self):
        for failure in ('legacy', 'authority'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as name:
                code, value, calls, out, checks = self.scenario(Path(name), failure)
                self.assertEqual((code, checks), (1, 1))
                self.assertEqual(len(value['after_audits']), 3)
                self.assertFalse((out / 'complete.json').exists())

    def test_final_pin_drift_withholds_complete(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, _, out, _ = self.scenario(Path(name), 'post-drift')
            self.assertEqual(code, 1)
            self.assertIn('actual input drift', '\n'.join(value['failures']))
            self.assertFalse((out / 'complete.json').exists())

    def test_failed_pre_audit_prevents_native_but_runs_all_post_audits(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, calls, out, checks = self.scenario(Path(name), 'pre-audit')
            self.assertEqual((code, checks), (1, 0))
            self.assertEqual([r[0] for r in calls], ['before-0', 'before-1', 'after-0', 'after-1', 'after-2'])
            self.assertEqual((value['native_attempts'], value['retries']), (0, 0))
            self.assertEqual(len(value['after_audits']), 3)
            self.assertFalse((out / 'complete.json').exists())

    def test_one_failed_post_audit_does_not_skip_remaining_audits(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, calls, out, checks = self.scenario(Path(name), 'post-audit')
            self.assertEqual((code, checks), (1, 1))
            self.assertEqual([r[0] for r in calls[-3:]], ['after-0', 'after-1', 'after-2'])
            self.assertEqual(len(value['after_audits']), 2)
            self.assertEqual(len(value['retained_native']), 14)
            self.assertFalse((out / 'complete.json').exists())

    def test_failed_sidecar_bytes_retained_but_not_accepted(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name); path = root / 'native-device-clock-v2.json'
            self.assertIsNone(M.retained_device(I.D.Pins(), root))
            with self.assertRaises(RuntimeError): M.retained_device(I.D.Pins(), root, True)
            path.write_bytes(b'x' * (M.DV.MAX_BYTES + 1))
            self.assertEqual(M.retained_device(I.D.Pins(), root)['bytes'], M.DV.MAX_BYTES + 1)
            with self.assertRaises(RuntimeError): M.retained_device(I.D.Pins(), root, True)
            path.unlink(); path.symlink_to(root / 'missing')
            with self.assertRaises((RuntimeError, OSError)): M.retained_device(I.D.Pins(), root)

    def test_closed14_file_roster_refuses_missing_extra_and_aliases(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            for member in I.BODY: (root / member).write_bytes(b'x')
            with self.assertRaises(RuntimeError): M.retained(I.D.Pins(), root, True)
            (root / 'complete.json').write_bytes(b'{}')
            records, bodies = M.retained(I.D.Pins(), root, True)
            self.assertEqual(set(records), I.BODY | {'complete.json'}); self.assertEqual(set(bodies), I.BODY)
            (root / 'extra').write_bytes(b'x')
            with self.assertRaises(RuntimeError): M.retained(I.D.Pins(), root)
            (root / 'extra').unlink(); (root / 'request-0.json').unlink()
            (root / 'request-0.json').symlink_to(root / 'request-1.json')
            with self.assertRaises(RuntimeError): M.retained(I.D.Pins(), root, True)

    def test_inventory_propagates_walk_error_and_refuses_aliases_and_cap(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name); (root / 'body').write_bytes(b'x'); M.inventory(root)
            def fail_walk(_root, **options): options['onerror'](PermissionError('unreadable')); return iter(())
            with patch.object(M.os, 'walk', side_effect=fail_walk), self.assertRaises(PermissionError): M.inventory(root)
            with self.assertRaises(RuntimeError): M.inventory(root, M.CASE_CAP)
            os.link(root / 'body', root / 'hardlink')
            with self.assertRaises(RuntimeError): M.inventory(root)

    def test_original_limits_never_raised(self):
        self.assertEqual((M.NATIVE_SECONDS, M.AUDIT_SECONDS, M.CASE_SECONDS), (4000, 30, 4300))
        self.assertEqual((M.STREAM_CAP, M.CASE_CAP), (8 << 20, 64 << 20))
        for seconds, space in ((4000, 32 << 30), (30, 12 << 30)):
            with patch.object(M.os, 'sched_setaffinity'), patch.object(M.os, 'nice'), \
                 patch.object(M.resource, 'getrlimit', return_value=(resource.RLIM_INFINITY, resource.RLIM_INFINITY)), \
                 patch.object(M.resource, 'setrlimit') as call:
                M.child_limits(seconds)
                self.assertIn((resource.RLIMIT_AS, (space, space)), [x.args for x in call.call_args_list])
            with patch.object(M.os, 'sched_setaffinity'), patch.object(M.os, 'nice'), \
                 patch.object(M.resource, 'getrlimit', return_value=(1024, 2048)), patch.object(M.resource, 'setrlimit') as call:
                M.child_limits(seconds); self.assertTrue(all(x.args[1][0] <= 1024 for x in call.call_args_list))

    def test_forced_cleanup_nonzero_unreaped_and_bool_exit_are_not_success(self):
        value = dict(exit_code=0, reason=None, cleanup_signalled=False, owned_groups_absent=True, owned_processes_reaped=True)
        I.owned_success(value)
        for key, bad in (('exit_code', 1), ('exit_code', False), ('reason', 'failure'),
                         ('cleanup_signalled', True), ('owned_groups_absent', False), ('owned_processes_reaped', False)):
            with self.assertRaises(RuntimeError): I.owned_success(dict(value, **{key: bad}))

    def test_checked_schema_refuses_false_counts_identities_and_claims(self):
        c = dict(request={'mode': 'teacher_forced'}, policy='shared-full-currentness',
            plan={'request': {'path': '/request'}}, baseline={'receipt_pin': {'path': '/baseline'}},
            runtime={'parent': {'path': '/parent'}, 'worker': {'path': '/worker'}, 'image': {'path': '/v7'}})
        good = observation(c); M.checked_observation(c, good)
        for key, value in (('clock_samples', 15), ('clock_samples', True), ('raw_clock_counters', 1), ('raw_rows', 1172.0), ('captured_payloads', 3), ('compared_tensor_rows', 151),
                           ('rank_packets', [592, 579]), ('raw_completion_ticks', 1), ('mode', 'autoregressive'),
                           ('parent', {}), ('worker', {}), ('prefix_image', {}), ('baseline', {}),
                           ('owned_record_checks', {}), ('all_payloads_tokens_and_tensors_equal', False)):
            with self.subTest(key=key), self.assertRaises(RuntimeError): M.checked_observation(c, dict(good, **{key: value}))
        for key in M.DV.FALSE:
            for value in (True, 0, None):
                with self.subTest(key=key, value=value), self.assertRaises(RuntimeError): M.checked_observation(c, dict(good, **{key: value}))
        changed = copy.deepcopy(good); changed['tensor_comparison'][0]['tensors'].pop()
        with self.assertRaises(RuntimeError): M.checked_observation(c, changed)


if __name__ == '__main__': unittest.main()
