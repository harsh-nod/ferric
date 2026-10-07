import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('selected_native_test', Path(__file__).with_name('selected_native.py'))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def fixture(arm='B'):
    controller = {'path': '/private/controller-' + arm, 'sha256': arm.lower() * 64}
    worker = {'path': '/private/worker', 'sha256': 'c' * 64}
    spec = {'arm': arm, 'mode': 'latency', 'argv': [controller['path'], '--max-batches', '810'],
            'controller': controller, 'worker': worker, 'prompt': 'frozen prompt', 'reference': {'ids': [1]}}
    result = {'accepted': True, 'raw_replay_passed': True, 'instrumented': False, 'latency_admitted': True,
        'setup': {'max_batches': 810, 'head_precision': 'fp32-v8', 'dtype': 'BF16', 'tensor_parallel': 1,
            'token_program': {'counter_diagnostic': False}, 'performance_profile': {'runtime_profiling': False},
            'worker_pids': [10], 'session_id': 'old', 'setup_seconds': 0.1}}
    build = {'controllers': {'live-' + arm: controller}, 'worker': worker,
             'controller_source': {'sha256': 'd' * 64}, 'runtime_source': {'sha256': 'e' * 64}}
    report = {'experiment': 'native-prefill613-ttft-only-v1', 'default_promotion': False,
              'vendor_comparison_performed': False, 'status': 'inconclusive'}
    return arm, spec, result, build, report


class SelectedArmTests(unittest.TestCase):
    def test_only_budget_and_session_fields_change(self):
        values = fixture()
        before = copy.deepcopy(values)
        selected = m.project_arm(*values)
        self.assertEqual(values, before)
        self.assertEqual(selected['argv'], ['/private/controller-B', '--max-batches', '5670'])
        self.assertEqual(selected['model_dispatches'], 3683862)
        self.assertEqual(selected['native_promotion_status'], 'inconclusive')
        self.assertFalse(selected['default_promotion'])
        self.assertFalse(m.SESSION_FIELDS & selected['expected_setup'].keys())

    def test_both_explicit_native_arms_are_admitted_without_claiming_gain(self):
        for arm in ('A', 'B'):
            selected = m.project_arm(*fixture(arm))
            self.assertEqual(selected['arm'], arm)
            self.assertFalse(selected['vendor_comparison_performed'])

    def test_counter_or_boundary_composition_refused(self):
        for flag in ('--token-program-backend', '--token-program-fence-mode'):
            values = fixture()
            values[1]['argv'] += [flag, 'unrelated']
            with self.assertRaises(ValueError):
                m.project_arm(*values)

    def test_partial_or_instrumented_native_receipt_refused(self):
        for key, value in (('accepted', False), ('raw_replay_passed', False),
                           ('instrumented', True), ('latency_admitted', False)):
            values = fixture()
            values[2][key] = value
            with self.assertRaises(ValueError):
                m.project_arm(*values)

    def test_wrong_binary_or_precision_refused(self):
        for mutation in ('controller', 'worker', 'head', 'dtype', 'tp', 'profile'):
            values = fixture()
            if mutation in ('controller', 'worker'):
                values[1][mutation] = {**values[1][mutation], 'sha256': 'f' * 64}
            elif mutation == 'profile':
                values[2]['setup']['performance_profile']['runtime_profiling'] = True
            else:
                values[2]['setup'][{'head': 'head_precision', 'dtype': 'dtype', 'tp': 'tensor_parallel'}[mutation]] = 'wrong'
            with self.assertRaises(ValueError):
                m.project_arm(*values)

    def test_exact_native_budget_required(self):
        for bad in ('135', '10000', '5670'):
            values = fixture()
            values[1]['argv'][-1] = bad
            with self.assertRaises(ValueError):
                m.project_arm(*values)

    def test_stable_setup_field_and_unknown_field_drift_refused(self):
        selected = m.project_arm(*fixture())
        actual = {**selected['expected_setup'], 'worker_pids': [20], 'session_id': 'new', 'setup_seconds': 1}
        m.validate_setup(actual, selected)
        for key in ('max_batches', 'unrecognized'):
            with self.assertRaises(ValueError):
                m.validate_setup({**actual, key: 'changed'}, selected)

    def test_other_campaign_or_implicit_arm_refused_before_file_io(self):
        for value in ({}, {'schema': 'other', 'arm': 'B', 'plan': {}, 'report': {}},
                      {'schema': 'FerricV17HttpSelectionV1', 'arm': 'newest', 'plan': {}, 'report': {}}):
            with self.assertRaises(ValueError):
                m.admit(value)

    def test_explicit_v19_projection_is_separate_from_v17(self):
        values = fixture()
        values[4]['experiment'] = 'native-cached-abi-preparation-only-v1'
        selected = m.project_arm(*values, campaign='V19')
        self.assertEqual(selected['schema'], 'FerricV19SelectedHttpArmV1')
        self.assertFalse(selected['default_promotion'])
        self.assertEqual(selected['model_dispatches'], 3683862)

    def test_v19_cannot_borrow_v17_report_scope(self):
        with self.assertRaises(ValueError):
            m.project_arm(*fixture(), campaign='V19')

    def test_arbitrary_later_campaign_is_not_a_profile_substitution(self):
        with self.assertRaises(ValueError):
            m.project_arm(*fixture(), campaign='V20')

    def test_v19_has_distinct_exact_qualified_contract_and_replay(self):
        self.assertEqual(m.ADAPTERS['V19'][:2], (
            '58b65d85930fc490a0aabe3e4241aac08720d302c3bc7230f198459d2415acf8',
            '85a48cd74dbfec50c7eef68486ed9eb5aa30090c690b256c8b94ac17d693dedc'))
        self.assertNotEqual(m.ADAPTERS['V17'][:2], m.ADAPTERS['V19'][:2])


if __name__ == '__main__':
    unittest.main()
