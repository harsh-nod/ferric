"""Current55c V19 cell refusals; synthetic metadata, no native execution."""
import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('baseline_selector_fixtures',
    Path(__file__).parent / 'harness/test_native_launch.py')
helpers = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(helpers)
cell = helpers.cell


def closed(spec, setup):
    return {**cell.COMMON, **spec['closed_expected'], 'live_profile': cell.ticks.PROFILE,
        'schema': 'FerricQwen3TpBatchClosedV2', 'authority': 'none',
        'attention_mode': 'query-hoist-v14', 'performance_qualified': False,
        'worker_pids': setup['worker_pids'], 'execution_completed': True,
        'all_workers_exited': True, 'rank_dispatch_counts': [87711]}


class V19CellTests(unittest.TestCase):
    def test_closed_v19_has_the_exact_copy_annotations(self):
        spec, _ = helpers.fixture()
        setup = helpers.setup(spec)
        value = closed(spec, setup)
        self.assertEqual(value['kv_copy_mode'], 'parallel-c1-v19')
        self.assertEqual(value['kv_copy_artifact'], spec['closed_expected']['kv_copy_artifact'])
        cell.check_closed(value, setup, 87711, spec)

    def test_baseline_selector_rejects_even_when_v19_selector_is_present(self):
        for flag in ('--ordered64-baseline-packet-ticks', '--token-program-backend', '--token-program-fence-mode'):
            spec, _ = helpers.fixture()
            spec['argv'] += [flag, 'baseline']
            with self.subTest(flag=flag), self.assertRaises(ValueError):
                cell.shape(spec)

    def test_native_width_or_other_diagnostics_reject(self):
        for flag in ('--native-prefill-rows', '--prefill32-pages-mode', '--ordered64-runtime-counters',
                     '--ordered64-host-timing', '--model-timestamps-output', '--host-timing',
                     '--runtime-profile', '--ordered64-active-poll-10ms', '--diagnostic-active-poll-10ms'):
            spec, _ = helpers.fixture()
            spec['argv'] += [flag, 'value']
            with self.subTest(flag=flag), self.assertRaises(ValueError):
                cell.shape(spec)

    def test_each_v19_metadata_binding_is_required_at_every_scope(self):
        spec, _ = helpers.fixture()
        for key in ('requested_kv_copy_mode', 'kv_copy_mode', 'kv_append_mode',
                    'kv_copy_artifact', 'kv_copy_artifact_path'):
            for scope in ('setup', 'profile', 'closed'):
                setup = helpers.setup(spec)
                value = closed(spec, setup) if scope == 'closed' else setup
                target = value['performance_profile'] if scope == 'profile' else value
                target.pop(key)
                with self.subTest(key=key, scope=scope), self.assertRaises(ValueError):
                    if scope == 'closed':
                        cell.check_closed(value, setup, 87711, spec)
                    else:
                        cell.check_setup(value, spec)

    def test_missing_or_duplicate_v19_selector_rejects(self):
        for duplicate in (False, True):
            spec, _ = helpers.fixture()
            index = spec['argv'].index('--ordered64-packet-ticks')
            if duplicate:
                spec['argv'] += spec['argv'][index:index + 2]
            else:
                del spec['argv'][index:index + 2]
            with self.subTest(duplicate=duplicate), self.assertRaises(ValueError):
                cell.shape(spec)

    def test_old_profile_rejects_setup_profile_and_closed(self):
        spec, _ = helpers.fixture()
        old = 'prefill16-decode-ordered64-baseline-kv-packet-ticks-v1'
        for scope in ('setup', 'profile', 'closed'):
            setup = helpers.setup(spec)
            value = closed(spec, setup) if scope == 'closed' else setup
            target = value['performance_profile'] if scope == 'profile' else value
            target['live_profile'] = old
            with self.subTest(scope=scope), self.assertRaises(ValueError):
                if scope == 'closed':
                    cell.check_closed(value, setup, 87711, spec)
                else:
                    cell.check_setup(value, spec)

    def test_instrumented_request_never_becomes_latency_admitted(self):
        record = {'output_tokens': 128, 'batches': 135, 'dispatches': 87711,
                  'arrival_ns': 1, 'completed_ns': 1001, 'ttft_ns': 10, 'tpot_ns': 5}
        summary = cell.summarize([record], 0)
        self.assertFalse(summary['latency_admitted'])
        bad = copy.deepcopy(record)
        bad['dispatches'] -= 1
        with self.assertRaises(ValueError):
            cell.summarize([bad], 0)

    def test_closed_requires_full_dispatches_and_same_worker(self):
        spec, _ = helpers.fixture()
        setup = helpers.setup(spec)
        for key, value in (('rank_dispatch_counts', [87710]), ('worker_pids', [124]),
                           ('all_workers_exited', False), ('execution_completed', False)):
            actual = {**closed(spec, setup), key: value}
            with self.subTest(key=key), self.assertRaises(ValueError):
                cell.check_closed(actual, setup, 87711, spec)


if __name__ == '__main__':
    unittest.main()
