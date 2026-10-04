"""Closed input and artifact policies; synthetic data, no native admission."""
import copy
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import intake as I


def pin(name='record', size=1, digest='ab' * 32):
    return dict(path=str(I.E / name), bytes=size, sha256=digest)


def cpu():
    value = dict(schema='ferric-p228-projection-residual-runtime-cpu-result-v1',
        passed=True, error=None, postcheck_errors=[], source_unchanged=True,
        parent_rebuilt=True, worker_rebuilt=True, empty_initial_target=True,
        phases={str(i): dict(exit_code=0, reason=None, group_absent=True) for i in range(84)},
        tests=dict(worker=dict(passed=477, ignored=4), runtime={'r':dict(passed=208, ignored=0)},
                   parent={'p':dict(passed=303, ignored=0)}), tests_passed=988, tests_ignored=4,
        added_worker_tests=['worker::' + str(i) for i in range(16)],
        added_parent_tests=dict(lib=['parent::' + str(i) for i in range(13)], bin=['cli']),
        binaries={})
    for key in ('gpu_execution','numerical_acceptance','performance_claim','timestamp_calibration',
                'production_authority','compiler_hsaco_reproduced'):
        value[key] = False
    for role, name in (('parent',I.PARENT_NAME),('worker',I.WORKER_NAME)):
        binary=pin(name)
        artifact=dict(reason='compiler-artifact', target=dict(name=name,kind=['bin'],crate_types=['bin']),
            executable=binary['path'],filenames=[binary['path']],features=['tp-batch-engineering'] if role=='parent' else [],
            profile=dict(test=False,opt_level='2',debug_assertions=True,overflow_checks=True))
        value['binaries'][name]=dict(artifact=artifact,binary=binary)
    return value


def request_fixture():
    image=pin('image',10864,I.IMAGE[1]); worker=pin('new-worker'); v7=pin('v7'); down=pin('down2')
    previous=dict(mode='teacher_forced',source='/model',images={k:pin(k) for k in ('prefix','mlp','tail','residual')},
        expected_bundle_id=[1]*32,expected_model_id=[2]*32,device_ids=[7,9],prompt={k:pin(k) for k in ('tokens','manifest','text')},
        dispatch_timeout_ms=10000,child_deadline_ms=3600000,session=[3]*32,prefix_image=v7,tiles_image=down)
    layer={k:copy.deepcopy(v) for k,v in previous.items() if k not in ('mode','prefix_image','tiles_image')}
    layer.update(schema='FerricFinitePrefixLayerCaptureRequestV1',worker=worker,session=[4]*32,
        prefix_tiles_image=v7,mlp_tiles_image=down,evidence_directory=str(I.E/'case/native'))
    request=dict(schema='FerricFiniteProjectionResidualLayerCaptureRequestV1',layer=layer,projection_residual_image=image)
    prior=dict(request=previous,L=SimpleNamespace(rust_pin=lambda v:v),plan=dict(down2_image=down))
    runtime=dict(parent=pin('parent'),worker=worker,image=v7)
    return request,prior,runtime,I.E/'case',image


class IntakeTests(unittest.TestCase):
    def test_plan_is_separate_closed_schema_and_namespace(self):
        plan={key:pin(key) for key in I.PLAN_FIELDS.split()}
        plan.update(schema=I.INPUT_SCHEMA,output_label='prefix-projection-residual-capture-gpu-v228-v1')
        I.input_shape(plan)
        for change in ({'policy':'shared-full-currentness'},{'output_label':'prefix-layer0-native-capture-gpu-v228-v1'},
                       {'schema':'ferric-p228-layer0-native-capture-inputs-v1'}):
            with self.assertRaises(RuntimeError): I.input_shape(dict(plan,**change))

    def test_unset_actual_cpu_and_package_prerequisites_fail_closed(self):
        for value in (None,(1,'unknown'),(True,'ab'*32)):
            with self.assertRaises(RuntimeError): I.actual_tuple(value,'fixture')
        with patch.object(I,'PURE_TESTS',None), self.assertRaises(RuntimeError): I.package_record(None)

    def test_actual_joint_artifact_shape_retains_both_roles(self):
        with patch.object(I,'BINARIES',dict(parent=(1,'ab'*32),worker=(1,'ab'*32))):
            value=cpu(); self.assertEqual(set(I.qualified_cpu(value)),{'parent','worker'})
            for key,bad in (('worker_rebuilt',False),('parent_rebuilt',False),('source_unchanged',False),
                            ('tests_passed',1),('numerical_acceptance',True),('gpu_execution',True)):
                with self.subTest(key=key),self.assertRaises(RuntimeError): I.qualified_cpu(dict(value,**{key:bad}))

    def test_joint_phase_and_selected_elf_mutations_refuse(self):
        with patch.object(I,'BINARIES',dict(parent=(1,'ab'*32),worker=(1,'ab'*32))):
            for mode in ('phase','forced','count','feature','digest','overflow'):
                value=cpu(); target=value['binaries'][I.WORKER_NAME]
                if mode=='phase': value['phases']['0']['exit_code']=False
                elif mode=='forced': value['phases']['0']['reason']='timeout'
                elif mode=='count': value['phases'].pop('0')
                elif mode=='feature': target['artifact']['features']=['live-validation']
                elif mode=='digest': target['binary']['sha256']='cd'*32
                else: target['artifact']['profile']['overflow_checks']=False
                with self.subTest(mode=mode),self.assertRaises(RuntimeError): I.qualified_cpu(value)

    def test_nested_request_preserves_original_copy_image_and_both_upstream_images(self):
        values=request_fixture()
        with patch.object(I,'read',return_value=b'') as read:
            I.request_check(None,*values)
            self.assertEqual(read.call_count,10)

    def test_request_cannot_change_workload_or_restore_old_route(self):
        request,prior,runtime,out,image=request_fixture()
        for mode in ('copy','prefix','down','worker','token','session','output','deadline','extra'):
            bad=copy.deepcopy(request)
            if mode=='copy': bad['layer']['images']['residual']=image
            elif mode=='prefix': bad['layer']['prefix_tiles_image']=image
            elif mode=='down': bad['layer']['mlp_tiles_image']=image
            elif mode=='worker': bad['layer']['worker']=pin('old-worker')
            elif mode=='token': bad['layer']['prompt']['tokens']=pin('changed-token')
            elif mode=='session': bad['layer']['session']=prior['request']['session']
            elif mode=='output': bad['layer']['evidence_directory']='/tmp/other'
            elif mode=='deadline': bad['layer']['child_deadline_ms']=7200000
            else: bad['projection_residual_image']=pin('wrong-image')
            with self.subTest(mode=mode),patch.object(I,'read',return_value=b''),self.assertRaises(RuntimeError):
                I.request_check(None,bad,prior,runtime,out,image)

    def test_root_review_binds_image_cpu_and_prior_capture_without_authority(self):
        plan={key:pin(key) for key in I.REVIEW_BINDINGS}; plan['output_label']='new-case'
        runtime=dict(parent=plan['parent'],worker=plan['worker'],image=pin('v7'))
        prior=dict(plan=dict(down2_image=pin('down2')),standalone=dict(provenance={'source':'v7'}),
                   down2_provenance={'source':'down2'})
        image=dict(lowering=plan['lowering_complete'],inspection=plan['inspection_complete'])
        review=dict(schema=I.REVIEW_SCHEMA,reviewed=True,authority='none',gpu_attempts=1,
            output_label=plan['output_label'],image=runtime['image'],down2_image=prior['plan']['down2_image'],
            image_provenance=prior['standalone']['provenance'],down2_provenance=prior['down2_provenance'],
            projection_provenance=image,review_topics={key:'Actual substantive scoped root review notes.' for key in I.TOPICS},
            notes='Actual substantive scoped root review notes.',**{k:plan[k] for k in I.REVIEW_BINDINGS},
            **{k:False for k in I.REVIEW_FALSE})
        I.engineering_review(review,plan,runtime,prior,image)
        for key in I.REVIEW_FALSE:
            with self.subTest(key=key),self.assertRaises(RuntimeError):
                I.engineering_review(dict(review,**{key:True}),plan,runtime,prior,image)
        for key in I.REVIEW_BINDINGS:
            with self.subTest(key=key),self.assertRaises(RuntimeError):
                I.engineering_review(dict(review,**{key:pin('different')}),plan,runtime,prior,image)

    def test_actual_old_capture_identity_is_not_new_candidate_or_tf4(self):
        self.assertEqual(I.CAPTURE_LABEL,'prefix-layer0-native-capture-gpu-v228-v1')
        with self.assertRaises(RuntimeError):
            I.replay_capture(None,pin('wrong',I.CAPTURE[0],I.CAPTURE[1]),None,None,None,None,None)

    def test_unqualified_image_or_inspection_refuses_before_read(self):
        plan=dict(lowering_complete=pin('lower',*I.LOWERING),inspection_complete=pin('inspection',1,I.INSPECTION_SHA),
                  projection_image=pin('image',*I.IMAGE))
        for key in plan:
            bad=copy.deepcopy(plan); bad[key]['sha256']='ff'*32
            with patch.object(I,'doc') as read,self.assertRaises(RuntimeError): I.image_evidence(None,bad)
            read.assert_not_called()

    def test_selected_binary_cannot_use_different_name_or_content(self):
        original=pin(I.PARENT_NAME,20)
        for record in (pin('different',20),pin(I.PARENT_NAME,21)):
            with patch.object(I,'read') as read,self.assertRaises(RuntimeError):
                I.deployed_binary(None,record,original)
            read.assert_not_called()


if __name__ == '__main__':
    unittest.main()
