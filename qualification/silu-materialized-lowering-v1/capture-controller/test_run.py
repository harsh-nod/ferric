"""File-backed supervisor policy tests; no subprocess or GPU execution."""
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


class RunTests(unittest.TestCase):
    def scenario(self, root, failure=None):
        out = root / 'case'; pins = I.D.Pins(); calls = []
        request = dict(schema='FerricFiniteProjectionResidualLayerCaptureRequestV1', layer=dict(evidence_directory=str(out/'native')))
        req = I.save(root/'request.json', request); pin = I.save(root/'plan.json', {})
        prior = I.save(root/'prior.json', {})
        helper = SimpleNamespace(topology_sample=Mock(return_value={'idle':True}), require_idle=Mock())
        observer = SimpleNamespace(quiescent=Mock(), check_platform=Mock(), process_audit=Mock(), SMI_ARGV=['/smi'])
        runtime = dict(parent=dict(path='/actual/new-capture-parent'), worker=dict(path='/actual/worker-cpu669'),
                       image=dict(path='/actual/v7.hsaco', bytes=53560, sha256='ab'*32))
        c = dict(pins=pins, owned=object(), O=observer, topology=helper, out=out, request=request,
            runtime=runtime, plan=dict(request=req, baseline=prior, parent_cpu=pin, worker_cpu=pin,
                parent_runtime_review=pin, worker_runtime_review=pin, capture_review=pin,
                baseline_capture=prior, projection_image=pin, lowering_complete=pin, inspection_complete=pin,
                mlp_image=pin, mlp_cpu=pin, mlp_lowering_complete=pin, mlp_lowering_owner=pin),
            plan_pin=pin, environment={}, platform={}, baseline_capture_body=bytes(8192), baseline_input={'token':9112},
            baseline_capture_payload=prior,
            standalone=dict(pins=I.D.Pins()), supervisor_manifest=pin)
        parent = dict(pid=120,pgid=120,sid=120,ppid=100,uid=9661,starttime=20)

        def bounded(directory, argv, env, owned, case_root, gpu, seconds):
            calls.append((directory.name,gpu,seconds))
            command=I.save(directory/'command.json',dict(argv=argv))
            started=I.save(directory/'started.json',dict(parent=parent,supervisor_pid=100))
            raw=b'{}\n'; err=b''
            if gpu:
                self.assertEqual(argv,['/actual/new-capture-parent','--request',req['path'],
                    '--capture-projection-residual-layer-zero','--allow-unauthenticated-machine-code'])
                native=out/'native'; native.mkdir()
                for name in M.CV.BODY: (native/name).write_bytes(b'x')
                (native/'summary.json').write_bytes(b'{}')
                err=b'finite prefix layer candidate child pid=121 pgid=121; setup not acknowledged\n'
                if failure=='missing-capture': (native/'candidate-capture.bin').unlink()
                if failure=='marker': err += b'warning\n'
                if failure=='summary': raw=b'{"extra":true}\n'
            lineage=[dict(event='owned',identity=parent)]
            if gpu and failure=='lineage':
                lineage.append(dict(event='owned',identity=dict(parent,pid=999,starttime=21)))
            value=dict(exit_code=1 if gpu and failure=='owner' else 0,reason=None,
                cleanup_signalled=False,owned_groups_absent=True,owned_processes_reaped=True,
                lineage=lineage,command=command,started=started,
                stdout=None,stderr=None)
            (directory/'stdout').write_bytes(raw); (directory/'stderr').write_bytes(err)
            value['stdout']=I.D.read_file(directory/'stdout')[0]; value['stderr']=I.D.read_file(directory/'stderr')[0]
            return value,I.save(directory/'result.json',value)

        def validate(raw,files,actual_request,baseline,input_record):
            self.assertEqual(raw,b'{}\n'); self.assertEqual(set(files),M.CV.BODY)
            self.assertIs(actual_request,request); self.assertEqual(baseline,c['baseline_capture_body']); self.assertEqual(input_record,c['baseline_input'])
            if failure=='parser': raise RuntimeError('capture/current-output join refused')
            return dict(closed_child_pids=[121],pre_residual_arrays_equal=True,current_tf4_hidden_equal=None)

        def guard(_):
            if failure=='post-drift' and len(calls)==7: raise RuntimeError('actual input drift')

        def audit(_raw,_platform):
            if failure=='post-audit' and calls[-1][0]=='after-0': raise RuntimeError('post-audit refused')
            if failure=='pre-audit' and calls[-1][0]=='before-1': raise RuntimeError('pre-audit refused')
        observer.process_audit.side_effect=audit
        with ExitStack() as stack:
            stack.enter_context(patch.object(M,'bounded',side_effect=bounded))
            stack.enter_context(patch.object(M,'resources'))
            stack.enter_context(patch.object(I,'guard',side_effect=guard))
            check=stack.enter_context(patch.object(M.CV,'validate',side_effect=validate))
            stack.enter_context(patch.object(M.time,'sleep')); stack.enter_context(patch.object(I.D,'progress'))
            code=M.execute(c)
        value=json.loads((out/('complete.json' if code==0 else 'failure.json')).read_bytes())
        return code,value,calls,out,check.call_count

    def test_one_attempt_six_audits_exact_cli_and_closed_retention(self):
        with tempfile.TemporaryDirectory() as name:
            code,value,calls,out,checks=self.scenario(Path(name))
            self.assertEqual((code,checks),(0,1))
            self.assertEqual([row[0] for row in calls],['before-0','before-1','before-2','parent','after-0','after-1','after-2'])
            self.assertEqual(sum(row[1] for row in calls),1)
            self.assertEqual(set(value['retained_native']),M.CV.BODY|{'summary.json'})
            self.assertEqual(value['schema'],'ferric-p228-silu-materialized-capture-gpu-v1')
            self.assertEqual((value['native_attempts'],value['retries'],value['captured_arrays']),(1,0,28))
            self.assertFalse(value['owned_children']['workers'][0]['outer_pidfd_observed'])
            self.assertIsNone(value['owned_children']['workers'][0]['identity'])
            for key in ('numerical_acceptance','independent_framework_comparison_performed','full_model_correctness',
                        'full_model_acceptance','performance_claim','production_authority','full_forward',
                        'paired_comparison_performed','gpu_time','calibrated_nanoseconds','overlap_claim'):
                self.assertIs(value[key],False)

    def test_owner_parser_marker_summary_and_missing_capture_keep_evidence_without_retry(self):
        for failure in ('owner','parser','marker','summary','missing-capture','lineage'):
            with self.subTest(failure=failure),tempfile.TemporaryDirectory() as name:
                code,value,calls,out,checks=self.scenario(Path(name),failure)
                self.assertEqual(code,1); self.assertFalse((out/'complete.json').exists())
                self.assertEqual(len(value['after_audits']),3); self.assertIn('candidate-bootstrap.json',value['retained_native'])
                self.assertEqual(sum(row[1] for row in calls),1); self.assertIsNone(value['captured_arrays'])
                self.assertIsNone(value['current_tf4_hidden_equal'])

    def test_final_input_drift_withholds_complete(self):
        with tempfile.TemporaryDirectory() as name:
            code,value,_,out,_=self.scenario(Path(name),'post-drift')
            self.assertEqual(code,1); self.assertIn('actual input drift','\n'.join(value['failures']))
            self.assertFalse((out/'complete.json').exists())

    def test_failed_pre_audit_prevents_native_and_keeps_all_post_audits(self):
        with tempfile.TemporaryDirectory() as name:
            code,value,calls,out,checks=self.scenario(Path(name),'pre-audit')
            self.assertEqual((code,checks),(1,0))
            self.assertEqual([row[0] for row in calls],['before-0','before-1','after-0','after-1','after-2'])
            self.assertEqual((value['native_attempts'],value['retries']),(0,0))
            self.assertEqual(len(value['after_audits']),3)

    def test_failed_post_audit_does_not_skip_later_checks(self):
        with tempfile.TemporaryDirectory() as name:
            code,value,calls,out,checks=self.scenario(Path(name),'post-audit')
            self.assertEqual((code,checks),(1,1))
            self.assertEqual([row[0] for row in calls[-3:]],['after-0','after-1','after-2'])
            self.assertEqual(len(value['after_audits']),2); self.assertEqual(len(value['retained_native']),12)
            self.assertFalse((out/'complete.json').exists())

    def test_closed_twelve_file_roster_refuses_missing_extra_and_symlink(self):
        with tempfile.TemporaryDirectory() as name:
            root=Path(name)
            for member in M.CV.BODY: (root/member).write_bytes(b'x')
            with self.assertRaises(RuntimeError): M.retained(I.D.Pins(),root,True)
            (root/'summary.json').write_bytes(b'{}')
            records,bodies=M.retained(I.D.Pins(),root,True)
            self.assertEqual(set(records),M.CV.BODY|{'summary.json'}); self.assertEqual(set(bodies),M.CV.BODY)
            (root/'extra').write_bytes(b'x')
            with self.assertRaises(RuntimeError): M.retained(I.D.Pins(),root)
            (root/'extra').unlink(); (root/'candidate-capture.bin').unlink()
            (root/'candidate-capture.bin').symlink_to(root/'request.json')
            with self.assertRaises(RuntimeError): M.retained(I.D.Pins(),root,True)

    def test_failure_retention_allows_only_partial_known_files(self):
        with tempfile.TemporaryDirectory() as name:
            root=Path(name)/'native'
            self.assertEqual(M.retained(I.D.Pins(),root),({},{}))
            with self.assertRaises(RuntimeError): M.retained(I.D.Pins(),root,True)
            root.mkdir(); (root/'candidate-stderr.bin').write_bytes(b'failure')
            records,_=M.retained(I.D.Pins(),root)
            self.assertEqual(set(records),{'candidate-stderr.bin'})
            with self.assertRaises(RuntimeError): M.retained(I.D.Pins(),root,True)

    def test_inventory_propagates_walk_error_refuses_aliases_and_caps(self):
        with tempfile.TemporaryDirectory() as name:
            root=Path(name); (root/'body').write_bytes(b'x'); M.inventory(root)
            def fail_walk(_root,**options): options['onerror'](PermissionError('unreadable')); return iter(())
            with patch.object(M.os,'walk',side_effect=fail_walk),self.assertRaises(PermissionError): M.inventory(root)
            with self.assertRaises(RuntimeError): M.inventory(root,M.CASE_CAP)
            os.link(root/'body',root/'hardlink')
            with self.assertRaises(RuntimeError): M.inventory(root)

    def test_original_leaf_resources_and_case_bounds_are_unchanged(self):
        self.assertEqual((M.NATIVE_SECONDS,M.AUDIT_SECONDS,M.CASE_SECONDS),(4000,30,4300))
        self.assertEqual((M.STREAM_CAP,M.CASE_CAP),(8<<20,64<<20))
        for seconds,space in ((4000,32<<30),(30,12<<30)):
            with patch.object(M.os,'sched_setaffinity'),patch.object(M.os,'nice'), \
                 patch.object(M.resource,'getrlimit',return_value=(resource.RLIM_INFINITY,resource.RLIM_INFINITY)), \
                 patch.object(M.resource,'setrlimit') as call:
                M.child_limits(seconds)
                self.assertIn((resource.RLIMIT_AS,(space,space)),[row.args for row in call.call_args_list])
            with patch.object(M.os,'sched_setaffinity'),patch.object(M.os,'nice'), \
                 patch.object(M.resource,'getrlimit',return_value=(1024,2048)),patch.object(M.resource,'setrlimit') as call:
                M.child_limits(seconds); self.assertTrue(all(row.args[1][0]<=1024 for row in call.call_args_list))

    def test_failed_or_forced_owner_never_qualifies(self):
        value=dict(exit_code=0,reason=None,cleanup_signalled=False,owned_groups_absent=True,owned_processes_reaped=True)
        I.owned_success(value)
        for key,bad in (('exit_code',1),('exit_code',False),('reason','failure'),('cleanup_signalled',True),
                        ('owned_groups_absent',False),('owned_processes_reaped',False)):
            with self.assertRaises(RuntimeError): I.owned_success(dict(value,**{key:bad}))

    def test_lineage_checks_actual_child_identity_when_observed(self):
        parent=dict(pid=120,pgid=120,sid=120,ppid=100,uid=9661,starttime=20)
        child=dict(pid=121,pgid=121,sid=120,ppid=120,uid=9661,starttime=21)
        native=dict(started={},lineage=[dict(event='owned',identity=parent),dict(event='owned',identity=child)])
        with patch.object(I,'doc',return_value=dict(parent=parent,supervisor_pid=100)):
            result=M.lineage(None,native,dict(closed_child_pids=[121]))
            self.assertTrue(result['workers'][0]['outer_pidfd_observed'])
            for key,value in (('ppid',999),('pgid',120),('sid',121),('uid',0),('starttime',19)):
                bad=copy.deepcopy(native); bad['lineage'][1]['identity'][key]=value
                with self.assertRaises(RuntimeError): M.lineage(None,bad,dict(closed_child_pids=[121]))
            for pids in ([],[121,122],[120]):
                with self.assertRaises(RuntimeError): M.lineage(None,native,dict(closed_child_pids=pids))


if __name__ == '__main__': unittest.main()
