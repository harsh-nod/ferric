"""Synthetic source, inventory, and recipe tests; no compiler or GPU work."""
import copy
from pathlib import Path
import unittest

import run as A


def proposal():
    shared = [A.REPORT + 'shared_report_' + str(i) for i in range(8)]
    additions = {
        A.PARENT + '/src/bin/' + A.NEW_BIN + '.rs',
        A.PARENT + '/src/tp_finite_client/prefix_decode/projection_shared_host_tests.rs',
        A.WORKER + '/src/native_projection_residual_decode_shared_host_v1_tests.rs',
        A.WORKER + '/src/projection_residual_decode_shared_host_v1_tests.rs',
    }
    files = [dict(path=name, source='draft/' + name,
        before=None if name in additions else dict(bytes=10, sha256='a' * 64),
        after=dict(bytes=20, sha256='b' * 64)) for name in sorted(A.FILES)]
    result = dict(schema='ferric-p228-projection-ar4-shared-host-source-proposal-v1',
        base_cpu=dict(path=str(A.OLD / 'complete.json'), bytes=488734, sha256=A.OLD_SHA), files=files,
        base_sources_after=dict(path=str(A.OLD / 'sources-after.json'), bytes=1269472, sha256=A.SOURCE_SHA),
        renamed_tests=[], old_test_names_removed=[], new_parent_binary=A.NEW_BIN, tests_executed=False,
        added_tests=dict(worker=shared + ['worker::test_' + str(i) for i in range(6)],
                         parent_library=shared + ['tp_finite_client::test_' + str(i) for i in range(5)],
                         parent_binary=['tests::new_bin']),
        added_worker_test_executions=14, added_parent_test_executions=14,
        worker_selector='--engineering-native-projection-residual-decode-shared-host-v1',
        shared_runtime_options=[False, False, True], original_group_fence_policy_unchanged=False,
        fresh_full_currentness_preserved=True,
        configuration_clock='inclusive-host-wall-nanoseconds-before-observer-enable',
        configuration_time_in_snapshots=False)
    result.update({key: False for key in ('compiler_modified', 'provider_modified', 'kernel_modified',
        'kfd_modified', 'limits_changed', 'operational_currentness_enabled', 'admission_cache_enabled',
        'live_sources_modified', 'gpu_execution')})
    return result


def recipes():
    names = ['rustfmt', 'rustfmt-check', 'parent-builds', 'parent-projection-host-report',
             A.OLD_BIN + '-list', A.OLD_BIN + '-tests'] + ['retained-' + str(i) for i in range(55)]
    previous = {name: dict(argv=['cargo', name], env={'CARGO_TARGET_DIR': str(A.OLD / 'target/parent')},
                           deadline_seconds=1200, tools={'cargo': 'a' * 64}) for name in names}
    for name in ('rustfmt', 'rustfmt-check'):
        previous[name]['argv'] = ['rustfmt', '--config', 'skip_children=true', 'old.rs']
    previous['parent-builds']['argv'] = ['cargo', 'build', '--bin', A.PLAIN_BIN,
                                        '--bin', A.OLD_BIN, '--message-format=json']
    previous['parent-projection-host-report']['argv'] = ['cargo', 'test', '--lib', A.REPORT, '--']
    for suffix in ('list', 'tests'):
        previous[A.OLD_BIN + '-' + suffix]['argv'] = ['cargo', 'test', '--bin', A.OLD_BIN, '--', suffix]
    prior = dict(tests={'worker-tests': {}, 'parent-projection-host-report': {}, A.OLD_BIN + '-tests': {}},
                 phases={name: {} for name in names}, historical_runtime_tests_not_repeated=208)
    return previous, prior


def metadata():
    manifest = str(A.OLD / 'sources/ferric' / A.PARENT / 'Cargo.toml')
    target = dict(name=A.OLD_BIN, src_path=str(A.OLD / 'sources/ferric' / A.PARENT / 'src/bin' / (A.OLD_BIN + '.rs')),
                  kind=['bin'], **{'required-features': ['tp-batch-engineering']})
    return dict(packages=[dict(manifest_path=manifest, targets=[target])],
                target_directory=str(A.OLD / 'target/parent'), resolve={'nodes': ['unchanged']})


class PolicyTests(unittest.TestCase):
    def test_proposal_exact_roles_and_roster(self):
        A.proposal_shape(proposal())

    def test_proposal_refuses_removed_names_or_wrong_generation(self):
        for key, value in (('old_test_names_removed', ['old::test']), ('renamed_tests', [{}]),
                           ('compiler_modified', True), ('added_parent_test_executions', 13)):
            record = proposal(); record[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                A.proposal_shape(record)
        record = proposal(); record['base_cpu']['sha256'] = '0' * 64
        with self.assertRaises(RuntimeError):
            A.proposal_shape(record)

    def test_proposal_requires_explicit_shared_full_and_separate_configuration_time(self):
        for key, value in (('shared_runtime_options', [False, False, False]),
                           ('shared_runtime_options', [0, 0, 1]),
                           ('original_group_fence_policy_unchanged', True),
                           ('fresh_full_currentness_preserved', False),
                           ('configuration_time_in_snapshots', True),
                           ('operational_currentness_enabled', True), ('admission_cache_enabled', True)):
            record = proposal(); record[key] = value
            with self.subTest(key=key, value=value), self.assertRaises(RuntimeError):
                A.proposal_shape(record)

    def test_overlay_four_additions_and_exact_replacements(self):
        rows = proposal()['files']
        before = {'ferric/' + row['path']: row['before'] for row in rows if row['before'] is not None}
        after = A.overlay_map(before, rows)
        self.assertEqual(len(after), 14)
        self.assertTrue(all(row == dict(bytes=20, sha256='b' * 64) for row in after.values()))
        addition = next(row for row in rows if row['before'] is None)
        bad = copy.deepcopy(before); bad['ferric/' + addition['path']] = addition['after']
        with self.assertRaises(RuntimeError):
            A.overlay_map(bad, rows)
        bad = copy.deepcopy(before); bad[next(iter(bad))]['sha256'] = 'c' * 64
        with self.assertRaises(RuntimeError):
            A.overlay_map(bad, rows)

    def test_cargo_only_one_new_feature_gated_bin(self):
        before = dict(package={'name': 'parent'}, bin=[dict(name=A.PLAIN_BIN), dict(name=A.OLD_BIN)],
                      dependencies={'old': '1'})
        after = copy.deepcopy(before)
        after['bin'].append(dict(name=A.NEW_BIN, path='src/bin/' + A.NEW_BIN + '.rs',
                                 **{'required-features': ['tp-batch-engineering']}))
        A.cargo_delta(before, after)
        bad = copy.deepcopy(after); bad['dependencies']['old'] = '2'
        with self.assertRaises(RuntimeError):
            A.cargo_delta(before, bad)
        bad = copy.deepcopy(after); bad['bin'][-1]['required-features'] = []
        with self.assertRaises(RuntimeError):
            A.cargo_delta(before, bad)
        bad = copy.deepcopy(after); del bad['bin'][0]
        with self.assertRaises(RuntimeError):
            A.cargo_delta(before, bad)

    def test_full_inventory_exact_union_and_no_hidden_removal(self):
        self.assertEqual(A.extended_inventory({'old'}, {'old', 'new'}, ['new']), {'new'})
        for actual, added in (({'new'}, ['new']), ({'old', 'new', 'other'}, ['new']), ({'old'}, ['old'])):
            with self.subTest(actual=actual), self.assertRaises(RuntimeError):
                A.extended_inventory({'old'}, actual, added)

    def test_worker_library_and_shared_wire_summaries_remain_separate(self):
        summaries = A.worker_summaries([[505, 0, 4], [13, 0, 0]], 14)
        self.assertEqual(summaries, [(519, 0, 4), (13, 0, 0)])
        self.assertEqual(sum(row[0] for row in summaries), 532)
        self.assertEqual(sum(row[2] for row in summaries), 4)
        for previous, added in (([[518, 0, 4]], 14), ([[505, 0, 4], [13, 0, 0]], 13)):
            with self.subTest(previous=previous, added=added), self.assertRaises(RuntimeError):
                A.worker_summaries(previous, added)

    def test_selector_covers_only_appropriate_added_names(self):
        actual = {'client::old', 'client::new', 'report::new'}
        self.assertEqual(A.selected_names({'client::old'}, actual, {'client::new', 'report::new'}, 'client::'),
                         {'client::old', 'client::new'})
        with self.assertRaises(RuntimeError):
            A.selected_names({'client::old'}, actual | {'client::undeclared'}, {'client::new'}, 'client::')

    def test_existing_report_selector_keeps_old_and_new_methods(self):
        old = {A.REPORT + 'old_' + str(i) for i in range(8)}
        new = {A.REPORT + 'new_' + str(i) for i in range(8)}
        additions = new | {'tp_finite_client::new'}
        selected = A.selected_names(old, old | additions, additions, A.REPORT)
        self.assertEqual(selected, old | new)
        self.assertEqual(len(selected), 16)

    def test_metadata_only_exact_new_parent_target(self):
        out = A.E / 'projection-ar4-shared-host-cpu-v228-v1'
        previous = metadata(); current = A.relocate(previous, out)
        target = copy.deepcopy(current['packages'][0]['targets'][0])
        target['name'] = A.NEW_BIN
        target['src_path'] = str(out / 'sources/ferric' / A.PARENT / 'src/bin' / (A.NEW_BIN + '.rs'))
        current['packages'][0]['targets'].append(target)
        A.metadata_delta(current, previous, out, 'parent')
        bad = copy.deepcopy(current); bad['resolve']['nodes'] = ['drift']
        with self.assertRaises(RuntimeError):
            A.metadata_delta(bad, previous, out, 'parent')
        bad = copy.deepcopy(current); bad['packages'][0]['targets'][-1]['required-features'] = []
        with self.assertRaises(RuntimeError):
            A.metadata_delta(bad, previous, out, 'parent')

    def test_worker_metadata_has_no_new_target_exception(self):
        out = A.E / 'fresh'
        previous = metadata(); current = A.relocate(previous, out)
        A.metadata_delta(current, previous, out, 'worker')
        current['packages'][0]['targets'].append(dict(name='unexpected'))
        with self.assertRaises(RuntimeError):
            A.metadata_delta(current, previous, out, 'worker')

    def test_recipe_scope_sixty_three_and_unchanged_bounds(self):
        previous, prior = recipes()
        original = copy.deepcopy(previous)
        out = A.E / 'projection-ar4-shared-host-cpu-v228-v1'
        current = A.recipes_from(previous, prior, out, proposal()['files'])
        self.assertEqual(len(current), 63)
        self.assertEqual(set(current), set(previous) | {A.NEW_BIN + '-list', A.NEW_BIN + '-tests'})
        self.assertEqual(current['retained-0'], A.relocate(original['retained-0'], out))
        self.assertEqual(previous, original)
        self.assertEqual(current['parent-builds']['argv'], ['cargo', 'build', '--bin', A.PLAIN_BIN,
                         '--bin', A.OLD_BIN, '--bin', A.NEW_BIN, '--message-format=json'])
        self.assertEqual(current['parent-projection-host-report'],
                         A.relocate(original['parent-projection-host-report'], out))
        self.assertEqual(current[A.NEW_BIN + '-tests']['deadline_seconds'], 1200)
        self.assertEqual(current[A.NEW_BIN + '-tests']['tools'], {'cargo': 'a' * 64})

    def test_formatter_includes_thirteen_rust_bodies_not_cargo(self):
        previous, prior = recipes()
        current = A.recipes_from(previous, prior, A.E / 'fresh', proposal()['files'])
        for name in ('rustfmt', 'rustfmt-check'):
            self.assertEqual(len(current[name]['argv'][3:]), 13)
            self.assertTrue(all(value.endswith('.rs') for value in current[name]['argv'][3:]))

    def test_recipe_refuses_missing_old_phase_or_runtime_scope_change(self):
        for changed in ('missing', 'runtime', 'build'):
            previous, prior = recipes()
            if changed == 'missing':
                del previous['retained-0']
            elif changed == 'runtime':
                prior['historical_runtime_tests_not_repeated'] = 0
            else:
                previous['parent-builds']['argv'][3] = 'wrong-old-parent'
            with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                A.recipes_from(previous, prior, A.E / 'fresh', proposal()['files'])

    def test_prior_receipt_requires_natural_exact_cpu855(self):
        value = dict(schema='ferric-p228-projection-ar4-host-observation-cpu-result-v1',
            passed=True, error=None, postcheck_errors=[], source_unchanged=True,
            tests_passed=855, tests_ignored=4, historical_runtime_tests_not_repeated=208,
            phases={str(i): dict(exit_code=0, reason=None, group_absent=True) for i in range(61)},
            raw={str(i): {} for i in range(312)}, binaries={str(i): {} for i in range(3)})
        A.prior_contract(value)
        bad = copy.deepcopy(value); bad['phases']['0']['reason'] = 'deadline'
        with self.assertRaises(RuntimeError):
            A.prior_contract(bad)
        bad = copy.deepcopy(value); bad['tests_passed'] = 1037
        with self.assertRaises(RuntimeError):
            A.prior_contract(bad)


if __name__ == '__main__':
    unittest.main(verbosity=2)
