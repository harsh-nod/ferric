"""Synthetic SiLU selection/diagnostic admission tests, not a native result."""
import copy
import sys
import unittest
from unittest.mock import patch

import intake as I
from test_intake import pin, request_fixture


def plan_fixture():
    return dict(baseline=pin('prefix-silu-materialized-capture-gpu-v228-v1/complete.json', *I.SILU_CAPTURE),
        mlp_cpu=pin('cpu', *I.SILU_CPU), mlp_lowering_complete=pin('lower', *I.SILU_LOWERING),
        mlp_lowering_owner=pin('owner', *I.SILU_OWNER), mlp_image=pin('image', *I.SILU_IMAGE),
        projection_image=pin('projection', *I.IMAGE), lowering_complete=pin('projection-lower', *I.LOWERING),
        inspection_complete=pin('projection-inspect', 1, I.INSPECTION_SHA))


def comparison_fixture():
    plan = plan_fixture()
    capture = dict(supervisor_manifest=pin('capture-manifest'), retained_native={'summary.json': pin('summary')},
                   worker=pin('layer-worker'), baseline=pin('old-tf4'))
    m = dict(schema='ferric-p228-silu-materialized-comparison-v1', input_token=9112, position=0,
        comparable_rows=24, comparisons=[{} for _ in range(24)], baseline_comparisons=[{} for _ in range(24)],
        unchanged_pre_swiglu_count=22, conditional_residual_words=16384, conditional_residuals_exact=True,
        conditional_residual_comparisons=[dict(byte_equal=True, elements=4096) for _ in range(4)],
        framework_product_control_exact=True, genuine_independent_framework_outputs=True,
        numerical_acceptance=False, full_layer_numerics_accepted=False, full_model_correctness=False, acceptance_threshold=None,
        inputs=dict(candidate_capture=capture['retained_native']['summary.json'], selected_mlp_image=plan['mlp_image'],
            projection_image=plan['projection_image'], worker=capture['worker'], current_tf4=capture['baseline'],
            framework_capture=pin('framework', 1, 'cf7512025bb469e06f87c32f788da807b4e6607297b98bb0b33c4135d1e2c78e'),
            original_framework=pin('old-framework', 1, '2edddf40cc6195fdd622e479e8e46f2a659f1b569106f5f4e51afdb83bdc2416')))
    value = dict(schema='ferric-p228-silu-materialized-comparison-observation-v1', completed=True,
        source_postchecks_passed=True, native_structural_replayed=True, candidate_owned_leaves_replayed=7,
        candidate_audits_replayed=6, framework_owned_leaf_results_rechecked=21, native_outer=plan['baseline'],
        baseline_outer=pin('corrected-layer', *I.CAPTURE), native_supervisor_manifest=capture['supervisor_manifest'],
        manifest=pin('comparison-manifest', 1, I.COMPARISON_PACKAGE_SHA),
        controller=pin('comparison-run', 1, I.COMPARISON_CONTROLLER_SHA),
        comparison_tests=pin('comparison-pure', 1, I.COMPARISON_PURE_SHA), comparison=m,
        **{k: False for k in ('gpu_execution', 'native_process_launched', 'numerical_acceptance',
                              'full_model_correctness', 'performance_claim', 'production_authority')})
    return value, plan, capture


class SiLUAdmissionTests(unittest.TestCase):
    def test_actual_cpu38_owner_lowering_and_image_must_all_match_layer(self):
        plan = plan_fixture(); I.silu_plan_bindings(plan, copy.deepcopy(plan))
        for key in plan:
            if key == 'baseline':
                continue
            bad = copy.deepcopy(plan); bad[key]['sha256'] = 'ff' * 32
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                I.silu_plan_bindings(bad, plan)

    def test_silu_only_replaces_selected_tiles_image_not_original_bootstrap_mlp(self):
        args = request_fixture(); request, prior, runtime, out, projection, mlp = args
        self.assertEqual(request['decode']['images'], prior['request']['images'])
        self.assertNotEqual(request['decode']['tiles_image'], prior['request']['tiles_image'])
        with patch.object(I, 'read', return_value=b''):
            I.request_check(None, *args)
        bad = copy.deepcopy(request); bad['decode']['images']['mlp'] = mlp
        with patch.object(I, 'read', return_value=b''), self.assertRaises(RuntimeError):
            I.request_check(None, bad, prior, runtime, out, projection, mlp)

    def test_old_down2_cannot_be_reselected_under_new_si_lu_plan(self):
        request, prior, runtime, out, projection, mlp = request_fixture()
        request['decode']['tiles_image'] = prior['plan']['down2_image']
        with patch.object(I, 'read', return_value=b''), self.assertRaises(RuntimeError):
            I.request_check(None, request, prior, runtime, out, projection, mlp)

    def test_layer_comparison_accepts_diagnostic_mismatches_without_fitted_bound(self):
        value, plan, capture = comparison_fixture()
        value['comparison']['comparisons'][0] = dict(byte_equal=False, max_abs_error=0.5)
        I.comparison_contract(value, plan, capture)
        self.assertFalse(value['comparison']['numerical_acceptance'])

    def test_layer_comparison_rejects_wrong_capture_image_and_unchecked_residual(self):
        original, plan, capture = comparison_fixture()
        for mode in ('capture', 'image', 'residual', 'fewer-rows', 'source', 'tests', 'framework'):
            value = copy.deepcopy(original)
            if mode == 'capture': value['native_outer'] = pin('other')
            elif mode == 'image': value['comparison']['inputs']['selected_mlp_image'] = pin('other')
            elif mode == 'residual': value['comparison']['conditional_residual_comparisons'][0]['byte_equal'] = False
            elif mode == 'fewer-rows': value['comparison']['comparisons'].pop()
            elif mode == 'source': value['controller']['sha256'] = 'ff' * 32
            elif mode == 'tests': value['comparison_tests']['sha256'] = 'ff' * 32
            else: value['comparison']['inputs']['framework_capture']['sha256'] = 'ff' * 32
            with self.subTest(mode=mode), self.assertRaises(RuntimeError):
                I.comparison_contract(value, plan, capture)

    def test_reference_or_gpu_success_does_not_mint_numerical_or_performance_authority(self):
        original, plan, capture = comparison_fixture()
        for key in ('numerical_acceptance', 'full_model_correctness', 'performance_claim', 'production_authority'):
            value = copy.deepcopy(original); value[key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                I.comparison_contract(value, plan, capture)

    def test_uncompleted_or_wrong_layer_refuses_before_replaying_old_evidence(self):
        plan = plan_fixture(); plan['baseline']['sha256'] = 'ff' * 32
        with patch.object(I, 'corrected_baseline') as previous, self.assertRaises(RuntimeError):
            I.baseline(None, plan)
        previous.assert_not_called()

    def test_private_dependency_aliases_restore_absent_none_and_object_on_all_exits(self):
        names = ('_silu_absent', '_silu_none', '_silu_present'); original = {name: sys.modules.get(name, ...) for name in names}
        marker = object()
        try:
            for failing in (False, True):
                sys.modules.pop(names[0], None); sys.modules[names[1]] = None; sys.modules[names[2]] = marker
                try:
                    with I.module_aliases({name: object() for name in names}):
                        self.assertTrue(all(sys.modules[name] is not marker and sys.modules[name] is not None for name in names))
                        if failing: raise ValueError('synthetic loader failure')
                except ValueError:
                    self.assertTrue(failing)
                self.assertNotIn(names[0], sys.modules); self.assertIsNone(sys.modules[names[1]])
                self.assertIs(sys.modules[names[2]], marker)
        finally:
            for name, value in original.items():
                if value is ...: sys.modules.pop(name, None)
                else: sys.modules[name] = value


if __name__ == '__main__':
    unittest.main()
