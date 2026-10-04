"""File-backed orchestration mocks; no process or GPU execution."""
from contextlib import ExitStack
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
    return dict(schema='ferric-p228-prefix-decode-independent-observation-v1',
        status='FINITE_FOUR_FORWARD_OBSERVED', structural_observation_complete=True,
        mode=c['request']['mode'], policy=c['policy'], request=c['plan']['request'],
        prefix_image=c['selected_runtime']['image'], structural={'finite': True},
        owned_record_checks=dict(child_pid=2, outer_pidfd_observed=False), host_observation={},
        captured_tensor_rows=152, inputs={}, recorded_close_and_owner_reap_checked=True,
        gpu_launched=False, native_baseline_comparison_performed=False,
        independent_framework_comparison_performed=False, numerical_acceptance=False,
        independent_tensor_acceptance=False, independent_tensor_threshold=None,
        full_model_acceptance=False, current_source_binary_image_authority_verified=False,
        current_platform_idle_audits_verified=False, top_level_observer_reaping_verified=False,
        sustained_2048_256=False, performance_claim=False, production_authority=False)


class RunTests(unittest.TestCase):
    def scenario(self, root, failure=None, policy='baseline', mode='teacher_forced'):
        out = root / 'case'; pins = I.D.Pins(); calls = []
        request = dict(mode=mode, evidence_directory=str(out / 'native'))
        req = I.save(root / 'request.json', request); plan = I.save(root / 'plan.json', {})
        prior_cpu = I.save(root / 'prior-cpu.json', {})
        helper = SimpleNamespace(topology_sample=Mock(return_value={'idle': True}), require_idle=Mock())
        observer = SimpleNamespace(quiescent=Mock(), check_platform=Mock(), process_audit=Mock(), SMI_ARGV=['/smi'])
        c = dict(pins=pins, owned=object(), O=observer, topology=helper, out=out, request=request, policy=policy,
            runtime=dict(parent=dict(path='/actual/parent'), worker=dict(path='/actual/worker-cpu553')),
            plan=dict(request=req, deployment=plan, prior_deployment=prior_cpu, image_deployment=plan,
                standalone_prepared=plan, standalone_cases=[plan] * 6, numericals=[plan] * 6),
            plan_pin=plan, environment={}, platform={},
            deployment=dict(base_deployment=plan, worker_cpu=plan, worker_cpu_review=plan),
            base_deployment=dict(cpu=dict(complete=plan), cpu_review=plan),
            prior_deployment=dict(worker_cpu=prior_cpu, worker_cpu_review=prior_cpu),
            standalone_receipts=[plan] * 6, standalone=dict(pins=I.D.Pins()), supervisor_manifest=plan,
            selected_runtime=dict(parent=dict(path='/actual/parent'), worker=dict(path='/actual/worker-cpu553'),
                image=dict(path='/actual/v7.hsaco', bytes=53560, sha256='ab' * 32)),
            historical_runtime=dict(parent=dict(path='/actual/parent'), worker=dict(path='/actual/worker-cpu522'),
                image=dict(path='/actual/historical.hsaco', bytes=1, sha256='cd' * 32)))

        def bounded(directory, argv, env, owned, case_root, gpu, seconds):
            calls.append((directory.name, gpu, seconds))
            command = I.save(directory / 'command.json', dict(argv=argv))
            started = I.save(directory / 'started.json', {})
            raw = b'{}\n'; err = b''
            if gpu:
                self.assertEqual(argv[-2:], ['--allow-unauthenticated-machine-code', '--observe-host-policy'])
                native = out / 'native'; native.mkdir()
                for name in I.BODY: (native / name).write_bytes(b'x')
                (native / 'complete.json').write_bytes(raw)
                if failure != 'missing-host': (out / 'native-host-policy-v2.json').write_bytes(raw)
                err = b'closed producer progress checked by comparator\n'
                if failure == 'missing-capture': (native / 'observation-3.bin').unlink()
            (directory / 'stdout').write_bytes(raw); (directory / 'stderr').write_bytes(err)
            value = dict(exit_code=1 if gpu and failure == 'owner' else 0, reason=None,
                cleanup_signalled=False, owned_groups_absent=True, owned_processes_reaped=True,
                lineage=[], command=command, started=started,
                stdout=I.D.read_file(directory / 'stdout')[0], stderr=I.D.read_file(directory / 'stderr')[0])
            return value, I.save(directory / 'result.json', value)

        def compare_native(context, records, host_sidecar, leaf_record, value):
            self.assertIs(context, c); self.assertEqual(set(records), I.BODY | {'complete.json'})
            self.assertEqual(host_sidecar['path'], str(out / 'native-host-policy-v2.json'))
            self.assertEqual(leaf_record['path'], str(out / 'parent/result.json'))
            self.assertGreater(value['stderr']['bytes'], 0)
            if failure == 'parser': raise ValueError('closed native transcript refused')
            result = observation(c)
            if failure == 'incomplete': result['structural_observation_complete'] = False
            if failure == 'legacy': return dict(full152_tensor_rows_bitwise_equal=True)
            if failure == 'authority': result['numerical_acceptance'] = True
            return result

        def guard(_):
            if failure == 'post-drift' and len(calls) == 7: raise RuntimeError('actual input drift')

        def process_audit(_raw, _platform):
            if failure == 'post-audit' and calls[-1][0] == 'after-0':
                raise RuntimeError('post-audit process refused')
            if failure == 'pre-audit' and calls[-1][0] == 'before-1':
                raise RuntimeError('pre-audit process refused')
        observer.process_audit.side_effect = process_audit

        with ExitStack() as stack:
            stack.enter_context(patch.object(M, 'bounded', side_effect=bounded))
            stack.enter_context(patch.object(M, 'resources'))
            stack.enter_context(patch.object(I, 'guard', side_effect=guard))
            check = stack.enter_context(patch.object(I, 'compare_native', side_effect=compare_native))
            stack.enter_context(patch.object(M.time, 'sleep'))
            stack.enter_context(patch.object(I.D, 'progress'))
            code = M.execute(c)
        record = json.loads((out / ('complete.json' if code == 0 else 'failure.json')).read_bytes())
        return code, record, calls, out, check.call_count

    def test_one_mode_one_worker_path_all_three_post_audits(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, calls, out, checks = self.scenario(Path(name))
            self.assertEqual(code, 0); self.assertEqual(checks, 1)
            self.assertEqual([row[0] for row in calls], ['before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2'])
            self.assertEqual(sum(row[1] for row in calls), 1)
            self.assertEqual(set(value['retained_native']), I.BODY | {'complete.json'})
            self.assertEqual(value['mode'], 'teacher_forced'); self.assertEqual(value['retries'], 0)
            self.assertEqual(value['observation']['path'], str(out / 'observation.json'))
            self.assertEqual(value['host_sidecar']['path'], str(out / 'native-host-policy-v2.json'))
            self.assertFalse(value['gpu_time']); self.assertTrue(value['inclusive_nested_host_scopes'])
            self.assertEqual(value['schema'], 'ferric-p228-state-bank-batch-observation-v1')
            for key in ('independent_numerical_acceptance', 'numerical_acceptance',
                    'independent_tensor_acceptance', 'full_model_acceptance', 'full_model_correctness',
                    'native_baseline_comparison_performed', 'independent_framework_comparison_performed',
                    'sustained_2048_256', 'performance_claim', 'production_authority'):
                self.assertIs(value[key], False, key)
            self.assertNotIn('full152_tensor_rows_bitwise_equal', value['checked'])
            self.assertNotIn('comparison', value)
            self.assertEqual(value['selected_runtime']['image']['path'], '/actual/v7.hsaco')
            self.assertEqual(value['historical_runtime']['image']['path'], '/actual/historical.hsaco')
            self.assertNotIn('layer_receipt', value)

    def test_each_policy_is_one_separate_attempt_with_identical_six_audits(self):
        for policy in I.HC.H.POLICIES:
            with self.subTest(policy=policy), tempfile.TemporaryDirectory() as name:
                code, value, calls, _, checks = self.scenario(Path(name), policy=policy)
                self.assertEqual(code, 0)
                self.assertEqual(value['policy'], policy)
                self.assertEqual(checks, 1)
                self.assertEqual([c[0] for c in calls], ['before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2'])
                self.assertEqual(value['native_attempts'], 1)
                self.assertEqual(value['retries'], 0)

    def test_incomplete_structure_retains_observation_and_all14_files(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, calls, out, checks = self.scenario(Path(name), 'incomplete')
            self.assertEqual(code, 1); self.assertEqual(checks, 1)
            self.assertFalse(value['checked']['structural_observation_complete'])
            self.assertEqual(len(value['retained_native']), 14)
            self.assertTrue((out / 'observation.json').exists()); self.assertFalse((out / 'complete.json').exists())
            self.assertEqual(len(value['after_audits']), 3); self.assertEqual(sum(r[1] for r in calls), 1)

    def test_owner_parser_and_missing_body_failures_keep_evidence_no_retry(self):
        for failure in ('owner', 'parser', 'missing-capture', 'missing-host'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as name:
                code, value, calls, out, checks = self.scenario(Path(name), failure)
                self.assertEqual(code, 1); self.assertFalse((out / 'complete.json').exists())
                self.assertEqual(len(value['after_audits']), 3)
                self.assertIn('control-0.bin', value['retained_native'])
                self.assertEqual(sum(r[1] for r in calls), 1)
                self.assertEqual(checks, int(failure == 'parser'))

    def test_host_failure_bytes_retained_but_not_accepted_as_success(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name); path = root / 'native-host-policy-v2.json'
            self.assertIsNone(M.retained_host(I.D.Pins(), root))
            with self.assertRaises(RuntimeError): M.retained_host(I.D.Pins(), root, True)
            path.write_bytes(b'x' * 65537)
            self.assertEqual(M.retained_host(I.D.Pins(), root)['bytes'], 65537)
            with self.assertRaises(RuntimeError): M.retained_host(I.D.Pins(), root, True)
            path.unlink(); path.symlink_to(root / 'missing')
            with self.assertRaises((RuntimeError, OSError)): M.retained_host(I.D.Pins(), root)

    def test_final_input_drift_withholds_complete(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, _, out, _ = self.scenario(Path(name), 'post-drift')
            self.assertEqual(code, 1); self.assertTrue(value['checked']['structural_observation_complete'])
            self.assertIn('actual input drift', '\n'.join(value['failures']))
            self.assertFalse((out / 'complete.json').exists())

    def test_retained_exact14_extras_aliases_and_missing_complete(self):
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

    def test_inventory_propagates_walk_error_and_refuses_alias_and_cap(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name); (root / 'body').write_bytes(b'x'); M.inventory(root)
            def fail_walk(_root, **options): options['onerror'](PermissionError('unreadable')); return iter(())
            with patch.object(M.os, 'walk', side_effect=fail_walk), self.assertRaises(PermissionError): M.inventory(root)
            with self.assertRaises(RuntimeError): M.inventory(root, M.CASE_CAP)
            os.link(root / 'body', root / 'hardlink')
            with self.assertRaises(RuntimeError): M.inventory(root)

    def test_existing_limits_never_raised(self):
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

    def test_forced_cleanup_nonzero_or_unreaped_not_success(self):
        value = dict(exit_code=0, reason=None, cleanup_signalled=False, owned_groups_absent=True, owned_processes_reaped=True)
        I.owned_success(value)
        for key, bad in (('exit_code', 1), ('exit_code', False), ('reason', 'failure'),
                         ('cleanup_signalled', True), ('owned_groups_absent', False), ('owned_processes_reaped', False)):
            with self.assertRaises(RuntimeError): I.owned_success(dict(value, **{key: bad}))


    def test_both_modes_accept_structure_without_paired_numerical_gate(self):
        for mode in ('teacher_forced', 'autoregressive'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as name:
                code, value, calls, _, checks = self.scenario(Path(name), mode=mode)
                self.assertEqual(code, 0); self.assertEqual(checks, 1)
                self.assertEqual(value['mode'], mode)
                self.assertNotIn('full152_tensor_rows_bitwise_equal', value['checked'])
                self.assertIs(value['checked']['numerical_acceptance'], False)
                self.assertEqual(sum(row[1] for row in calls), 1)

    def test_old_parity_or_numerical_authority_never_completes(self):
        for failure in ('legacy', 'authority'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as name:
                code, value, calls, out, checks = self.scenario(Path(name), failure)
                self.assertEqual(code, 1); self.assertEqual(checks, 1)
                self.assertTrue((out / 'observation.json').is_file())
                self.assertFalse((out / 'complete.json').exists())
                self.assertEqual(len(value['after_audits']), 3)
                self.assertEqual(value['native_attempts'], 1)
                self.assertEqual(value['retries'], 0)
                self.assertEqual(sum(row[1] for row in calls), 1)

    def test_closed_result_schema_identity_counts_and_false_flags(self):
        c = dict(request={'mode': 'teacher_forced'}, policy='shared-full-currentness',
            plan={'request': {'path': '/request', 'bytes': 2, 'sha256': 'aa' * 32}},
            selected_runtime={'image': {'path': '/v7', 'bytes': 53560, 'sha256': 'bb' * 32}})
        good = observation(c)
        self.assertEqual(M.checked_observation(c, good), good['owned_record_checks'])
        mutations = (
            ('schema', 'ferric-p228-prefix-decode-host-policy-comparison-v2'),
            ('status', 'PASSED'), ('structural_observation_complete', 1),
            ('recorded_close_and_owner_reap_checked', 1), ('captured_tensor_rows', 152.0),
            ('captured_tensor_rows', 151), ('owned_record_checks', {}),
            ('mode', 'autoregressive'), ('policy', 'baseline'),
            ('request', dict(c['plan']['request'], sha256='cc' * 32)),
            ('prefix_image', dict(c['selected_runtime']['image'], sha256='cc' * 32)),
            ('independent_tensor_threshold', 0), ('full152_tensor_rows_bitwise_equal', True))
        for key, bad in mutations:
            with self.subTest(key=key, bad=bad), self.assertRaises(RuntimeError):
                M.checked_observation(c, dict(good, **{key: bad}))
        false_keys = [key for key, value in good.items() if value is False]
        for key in false_keys:
            for bad in (True, 0, None):
                with self.subTest(key=key, bad=bad), self.assertRaises(RuntimeError):
                    M.checked_observation(c, dict(good, **{key: bad}))

    def test_failed_post_audit_still_runs_every_post_audit_and_keeps_native(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, calls, out, checks = self.scenario(Path(name), 'post-audit')
            self.assertEqual(code, 1); self.assertEqual(checks, 1)
            self.assertEqual([row[0] for row in calls[-3:]], ['after-0', 'after-1', 'after-2'])
            self.assertEqual(len(value['after_audits']), 2)
            self.assertEqual(len(value['retained_native']), 14)
            self.assertIn('post-audit process refused', '\n'.join(value['failures']))
            self.assertFalse((out / 'complete.json').exists())
            self.assertEqual(value['native_attempts'], 1); self.assertEqual(value['retries'], 0)

    def test_failed_pre_audit_never_launches_native_and_still_runs_post_audits(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, calls, out, checks = self.scenario(Path(name), 'pre-audit')
            self.assertEqual(code, 1); self.assertEqual(checks, 0)
            self.assertEqual([row[0] for row in calls],
                ['before-0', 'before-1', 'after-0', 'after-1', 'after-2'])
            self.assertEqual(sum(row[1] for row in calls), 0)
            self.assertEqual(value['native_attempts'], 0); self.assertEqual(value['retries'], 0)
            self.assertEqual(len(value['after_audits']), 3)
            self.assertEqual(value['retained_native'], {})
            self.assertFalse((out / 'complete.json').exists())


    def test_completion_separates_cpu633_cpu522_cpu553_and_v7_without_authority(self):
        with tempfile.TemporaryDirectory() as name:
            code, value, calls, _, _ = self.scenario(Path(name), policy='shared-full-currentness')
            self.assertEqual(code, 0)
            self.assertEqual(value['parent'], value['historical_runtime']['parent'])
            self.assertEqual(value['worker']['path'], '/actual/worker-cpu553')
            self.assertEqual(value['historical_runtime']['worker']['path'], '/actual/worker-cpu522')
            self.assertNotEqual(value['worker_cpu_complete'], value['prior_worker_cpu_complete'])
            self.assertEqual(value['prior_deployment'], value['prior_worker_cpu_complete'])
            self.assertEqual(len(value['standalone_cases']), 6)
            self.assertEqual(len(value['numericals']), 6)
            self.assertEqual(value['selected_runtime']['image']['path'], '/actual/v7.hsaco')
            self.assertEqual(len(calls), 7)
            self.assertIs(value['numerical_acceptance'], False)
            self.assertIs(value['performance_claim'], False)


if __name__ == '__main__': unittest.main()
