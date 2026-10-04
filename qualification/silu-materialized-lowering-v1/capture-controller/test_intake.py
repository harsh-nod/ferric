"""Synthetic admission checks only; no package imports beyond local data helpers."""
import copy
from pathlib import Path
import unittest

import intake as I
import mlp_contracts as M
from test_capture_validation import fixture


def pin(name='input', digest='ab' * 32):
    return dict(path='/synthetic/' + name, bytes=1, sha256=digest)


def rust_pin(value):
    return dict(value, sha256=bytes(value['sha256']).hex())


def request_fixture():
    _, _, previous, _ = fixture()
    value = copy.deepcopy(previous)
    image = pin('silu', 'cd' * 32)
    value['layer']['mlp_tiles_image'] = dict(image, sha256=list(bytes.fromhex(image['sha256'])))
    value['layer']['session'] = [8] * 32
    value['layer']['evidence_directory'] = '/synthetic/new/native'
    runtime = dict(worker=rust_pin(previous['layer']['worker']),
                   image=rust_pin(previous['layer']['prefix_tiles_image']))
    return value, previous, runtime, Path('/synthetic/new'), image, rust_pin


def cpu_fixture():
    return dict(schema='ferric-p228-silu-materialized-cpu-result-v1', passed=True, error=None,
        postcheck_errors=[], source_unchanged=True, cpu_arithmetic_only=True, tests_passed=38,
        tests_ignored=0, phases={str(i): dict(exit_code=0, reason=None, group_absent=True) for i in range(16)},
        tests={name: dict(passed=n, ignored=0, names=[name + str(i) for i in range(n)])
               for name, n in I.TARGET_COUNTS.items()},
        **{key: False for key in ('gpu_execution', 'compiler_hsaco_reproduced', 'full_model_acceptance',
                                 'numerical_acceptance', 'performance_claim', 'production_authority')})


def lowering_fixture():
    plan = dict(mlp_cpu=pin('cpu'), mlp_lowering_complete=pin('lower'), mlp_image=pin('image'))
    cpu = dict(overlay=pin('source'), prior_lowering=pin('down2'))
    prior = dict(down2_provenance=dict(lowering=cpu['prior_lowering'], compiler_generation={'exact': 'generation'}))
    lower = dict(schema='ferric-p228-silu-materialized-lowering-result-v1', passed=True, error=None,
        postcheck_errors=[], candidate_cpu=plan['mlp_cpu'], source_manifest=cpu['overlay'],
        prior_lowering=cpu['prior_lowering'], compiler_generation=prior['down2_provenance']['compiler_generation'],
        package_manifest=pin('package'), **{key:False for key in
            ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority')})
    owner = dict(lower, schema='ferric-p228-silu-materialized-lowering-owned-result-v1',
        completion=plan['mlp_lowering_complete'], owned=dict(exit_code=0,reason=None,cleanup_signalled=False,
            owned_groups_absent=True,owned_processes_reaped=True))
    lower.update(commands=[dict(name=name) for name in I.STAGES],
        artifacts={**{str(i):pin(str(i)) for i in range(9)}, 'emitted/artifact.hsaco':plan['mlp_image']},
        fresh_checked_lowering=True, fresh_checked_replay=True, fresh_hsaco_emitted=True,
        frontend_recipe_is_diagnostic=True, unresolved_runtime_requirements=8)
    return lower, owner, plan, cpu, prior


class IntakeTests(unittest.TestCase):
    def test_closed_plan_new_namespace_and_actual_pin_fields(self):
        plan = {name:pin(name) for name in I.PLAN_FIELDS.split()}
        plan.update(schema=I.INPUT_SCHEMA, output_label='prefix-silu-materialized-capture-gpu-v228-v1')
        I.input_shape(plan)
        for key, value in [('schema','old'), ('output_label','prefix-projection-residual-capture-gpu-v228-v1'),
                           ('mlp_cpu',None), ('mlp_image','invented')]:
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                I.input_shape(dict(plan, **{key:value}))
        with self.assertRaises(RuntimeError): I.input_shape(dict(plan, extra=pin()))

    def test_pending_actuals_refuse_instead_of_projecting_success(self):
        for value in (None, (0,'ab'*32), (True,'ab'*32), (1,'bad'), [1,'ab'*32]):
            with self.assertRaises(RuntimeError): I.actual_tuple(value,'test')
        I.actual_tuple((1,'ab'*32),'test')

    def test_only_selected_mlp_image_changes_on_unchanged_native_route(self):
        I.request_shape(*request_fixture())

    def test_old_initial_images_inputs_deadlines_and_residual_are_preserved(self):
        for key in ('schema', 'source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids',
                    'prompt', 'prefix_tiles_image', 'dispatch_timeout_ms', 'child_deadline_ms'):
            args = list(request_fixture()); args[0]['layer'][key] = None
            with self.subTest(key=key), self.assertRaises(RuntimeError): I.request_shape(*args)
        args = list(request_fixture()); args[0]['projection_residual_image']['bytes'] += 1
        with self.assertRaises(RuntimeError): I.request_shape(*args)

    def test_old_image_fallback_alias_and_reused_session_refuse(self):
        for mode in ('image','session','output','worker'):
            args = list(request_fixture()); layer=args[0]['layer']; old=args[1]['layer']
            if mode=='image': layer['mlp_tiles_image']=copy.deepcopy(old['mlp_tiles_image'])
            elif mode=='session': layer['session']=copy.deepcopy(old['session'])
            elif mode=='output': layer['evidence_directory']='/other'
            else: layer['worker']=copy.deepcopy(old['worker']); layer['worker']['sha256']=[9]*32
            with self.subTest(mode=mode), self.assertRaises(RuntimeError): I.request_shape(*args)

    def test_new_cpu_named_census_and_natural_closure(self):
        cpu=cpu_fixture(); I.cpu_contract(cpu)
        for field,value in [('postcheck_errors',['drift']), ('tests_passed',37), ('tests_ignored',1),
                            ('source_unchanged',False), ('numerical_acceptance',True)]:
            with self.assertRaises(RuntimeError): I.cpu_contract(dict(cpu,**{field:value}))

    def test_cpu_missing_names_duplicates_and_killed_phase_refuse(self):
        for mode in ('name','duplicate','kill','phase'):
            cpu=cpu_fixture(); row=cpu['tests']['mlp_silu_materialized_v1']
            if mode=='name': row['names'].pop()
            elif mode=='duplicate': row['names'][0]=row['names'][1]
            elif mode=='kill': cpu['phases']['0']['reason']='timeout'
            else: cpu['phases'].pop('0')
            with self.assertRaises(RuntimeError): I.cpu_contract(cpu)

    def test_lowering_exact_generation_and_image_join(self):
        I.lowering_contract(*lowering_fixture())

    def test_lowering_must_keep_all_stages_and_undischarged_obligations(self):
        for mode in ('stages','runtime','generation','image','source'):
            args=list(lowering_fixture()); lower,owner,plan,cpu,prior=args
            if mode=='stages': lower['commands'].pop()
            elif mode=='runtime': lower['unresolved_runtime_requirements']=0
            elif mode=='generation': owner['compiler_generation']={'different':'generation'}
            elif mode=='image': lower['artifacts']['emitted/artifact.hsaco']=pin('other','ef'*32)
            else: lower['source_manifest']=pin('wrong')
            with self.assertRaises(RuntimeError): I.lowering_contract(*args)

    def test_lowering_owner_kill_reap_and_wrong_completion_refuse(self):
        for mode in ('signal','reap','completion'):
            args=list(lowering_fixture()); owner=args[1]
            if mode=='signal': owner['owned']['cleanup_signalled']=True
            elif mode=='reap': owner['owned']['owned_processes_reaped']=False
            else: owner['completion']=pin('wrong')
            with self.assertRaises(RuntimeError): I.lowering_contract(*args)

    def test_root_review_cannot_auto_approve_or_grant_authority(self):
        plan={key:pin(key) for key in I.REVIEW_BINDINGS}; plan['output_label']='fresh'
        runtime=dict(image=pin('v7'))
        capture=dict(prior=dict(standalone=dict(provenance={'v7':'exact'}),down2_provenance={'down2':'exact'}),
                     image_provenance={'residual':'exact'})
        mlp={'new':'exact'}
        value=dict(schema=I.REVIEW_SCHEMA,reviewed=True,authority='none',gpu_attempts=1,output_label='fresh',
            image=runtime['image'], image_provenance=capture['prior']['standalone']['provenance'],
            projection_provenance=capture['image_provenance'],prior_down2_provenance=capture['prior']['down2_provenance'],
            mlp_provenance=mlp,review_topics={key:'Synthetic substantive review fixture only.' for key in I.TOPICS},
            notes='Synthetic substantive review fixture only.',**{key:plan[key] for key in I.REVIEW_BINDINGS},
            **{key:False for key in I.REVIEW_FALSE})
        I.engineering_review(value,plan,runtime,capture,mlp)
        for key,bad in [('reviewed',False),('authority','launch'),('gpu_attempts',2),('notes','short'),
                        *[(key,True) for key in I.REVIEW_FALSE],('mlp_image',pin('other'))]:
            with self.subTest(key=key),self.assertRaises(RuntimeError):
                I.engineering_review(dict(value,**{key:bad}),plan,runtime,capture,mlp)

    def test_existing_mlp_descriptor_contract_does_not_widen_abi(self):
        image=pin('hsaco')
        fields=dict(authority='none',object_sha256=image['sha256'],object_bytes='1',
            entry_symbol_hex=M.SYMBOL.encode().hex(),target='gfx950:xnack-',code_object_version='6',
            explicit_argument_bytes='88',kernarg_segment_bytes='344',kernarg_alignment='8',workgroup='64,1,1',
            max_grid_workgroups='64,1,1',descriptor_sha256='ab'*32,canonical_code_object_digest='cd'*32,
            descriptor_symbol_hex='01')
        def raw(value):
            return ('fe2o3-finite-join-request-metadata-v1\n'+''.join(k+' '+v+'\n' for k,v in value.items())).encode()
        M.metadata(raw(fields),image)
        for key,value in [('authority','load'),('target','gfx942'),('kernarg_segment_bytes','424'),
                          ('object_sha256','ef'*32),('workgroup','32,1,1')]:
            with self.assertRaises(RuntimeError): M.metadata(raw(dict(fields,**{key:value})),image)


if __name__ == '__main__': unittest.main()
