"""Synthetic source, inventory, and recipe tests; no compiler or GPU work."""
import copy
from pathlib import Path
import unittest

import run as A


def proposal():
    shared = [A.REPORT + 'report_' + str(i) for i in range(8)]
    files = []
    for index, name in enumerate(sorted(A.FILES)):
        files.append(dict(path=name, source='draft/' + name,
            before=None if index < 6 else dict(bytes=10, sha256='a' * 64),
            after=dict(bytes=20, sha256='b' * 64)))
    result = dict(schema='ferric-p228-projection-ar4-host-observation-source-proposal-v1',
        base_cpu=dict(path=str(A.OLD / 'complete.json'), bytes=583779, sha256=A.OLD_SHA), files=files,
        renamed_tests=[], old_test_names_removed=[], new_parent_binary=A.NEW_BIN, tests_executed=False,
        added_tests=dict(worker=shared + ['worker::test_' + str(i) for i in range(5)],
                         parent_library=shared + ['tp_finite_client::test_' + str(i) for i in range(4)],
                         parent_binary=['tests::new_bin']),
        added_worker_test_executions=13, added_parent_test_executions=13)
    result.update({key: False for key in ('compiler_modified', 'provider_modified', 'kernel_modified',
        'kfd_modified', 'policy_relaxation', 'live_sources_modified', 'gpu_execution')})
    return result


def recipes():
    names = ['runtime-list'] + ['runtime-' + str(i) for i in range(28)] + [
        'rustfmt', 'rustfmt-check', 'parent-builds', 'parent-projection-residual-decode-wire',
        A.OLD_BIN + '-list', A.OLD_BIN + '-tests'] + ['retained-' + str(i) for i in range(52)]
    previous = {name: dict(argv=['cargo', name], env={'CARGO_TARGET_DIR': str(A.OLD / 'target/parent')},
                           deadline_seconds=1200, tools={'cargo': 'a' * 64}) for name in names}
    for name in ('rustfmt', 'rustfmt-check'):
        previous[name]['argv'] = ['rustfmt', '--config', 'skip_children=true', 'old.rs']
    previous['parent-builds']['argv'] = ['cargo', 'build', '--bin', A.OLD_BIN,
                                        '--bin', 'other', '--message-format=json']
    previous['parent-projection-residual-decode-wire']['argv'] = ['cargo', 'test', '--lib', 'old::', '--']
    for suffix in ('list', 'tests'):
        previous[A.OLD_BIN + '-' + suffix]['argv'] = ['cargo', 'test', '--bin', A.OLD_BIN, '--', suffix]
    prior = dict(tests={'runtime-' + str(i): dict(passed=19 if i == 0 else 7) for i in range(28)})
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
                           ('compiler_modified', True), ('added_parent_test_executions', 12)):
            record = proposal(); record[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                A.proposal_shape(record)
        record = proposal(); record['base_cpu']['sha256'] = '0' * 64
        with self.assertRaises(RuntimeError):
            A.proposal_shape(record)

    def test_overlay_six_additions_and_exact_replacements(self):
        rows = proposal()['files']
        before = {'ferric/' + row['path']: row['before'] for row in rows if row['before'] is not None}
        after = A.overlay_map(before, rows)
        self.assertEqual(len(after), 17)
        self.assertTrue(all(row == dict(bytes=20, sha256='b' * 64) for row in after.values()))
        bad = copy.deepcopy(before); bad['ferric/' + rows[0]['path']] = rows[0]['after']
        with self.assertRaises(RuntimeError):
            A.overlay_map(bad, rows)
        bad = copy.deepcopy(before); bad[next(iter(bad))]['sha256'] = 'c' * 64
        with self.assertRaises(RuntimeError):
            A.overlay_map(bad, rows)

    def test_cargo_only_one_new_feature_gated_bin(self):
        before = dict(package={'name': 'parent'}, bin=[dict(name=A.OLD_BIN)], dependencies={'old': '1'})
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

    def test_full_inventory_exact_union_and_no_hidden_removal(self):
        self.assertEqual(A.extended_inventory({'old'}, {'old', 'new'}, ['new']), {'new'})
        for actual, added in (({'new'}, ['new']), ({'old', 'new', 'other'}, ['new']), ({'old'}, ['old'])):
            with self.subTest(actual=actual), self.assertRaises(RuntimeError):
                A.extended_inventory({'old'}, actual, added)

    def test_worker_library_and_shared_wire_summaries_remain_separate(self):
        summaries = A.worker_summaries([[492, 0, 4], [13, 0, 0]], 13)
        self.assertEqual(summaries, [(505, 0, 4), (13, 0, 0)])
        self.assertEqual(sum(row[0] for row in summaries), 518)
        self.assertEqual(sum(row[2] for row in summaries), 4)
        for previous, added in (([[505, 0, 4]], 13), ([[492, 0, 4], [13, 0, 0]], 12)):
            with self.subTest(previous=previous, added=added), self.assertRaises(RuntimeError):
                A.worker_summaries(previous, added)

    def test_selector_covers_only_appropriate_added_names(self):
        actual = {'client::old', 'client::new', 'report::new'}
        self.assertEqual(A.selected_names({'client::old'}, actual, {'client::new', 'report::new'}, 'client::'),
                         {'client::old', 'client::new'})
        with self.assertRaises(RuntimeError):
            A.selected_names({'client::old'}, actual | {'client::undeclared'}, {'client::new'}, 'client::')

    def test_metadata_only_exact_new_parent_target(self):
        out = A.E / 'projection-ar4-host-observation-cpu-v228-v1'
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

    def test_recipe_scope_sixty_one_and_unchanged_bounds(self):
        previous, prior = recipes()
        original = copy.deepcopy(previous)
        out = A.E / 'projection-ar4-host-observation-cpu-v228-v1'
        current, omitted = A.recipes_from(previous, prior, out, proposal()['files'])
        self.assertEqual(len(current), 61)
        self.assertEqual(len(omitted), 28)
        self.assertNotIn('runtime-list', current)
        self.assertEqual(current['retained-0'], A.relocate(original['retained-0'], out))
        self.assertEqual(previous, original)
        self.assertEqual(current['parent-builds']['argv'],
                         ['cargo', 'build', '--bin', A.OLD_BIN, '--bin', A.NEW_BIN, '--message-format=json'])
        self.assertEqual(current['parent-projection-host-report']['argv'], ['cargo', 'test', '--lib', A.REPORT, '--'])

    def test_formatter_includes_sixteen_rust_bodies_not_cargo(self):
        previous, prior = recipes()
        current, _ = A.recipes_from(previous, prior, A.E / 'fresh', proposal()['files'])
        for name in ('rustfmt', 'rustfmt-check'):
            self.assertEqual(len(current[name]['argv'][3:]), 16)
            self.assertTrue(all(value.endswith('.rs') for value in current[name]['argv'][3:]))

    def test_recipe_refuses_different_omitted_runtime_census(self):
        previous, prior = recipes(); prior['tests']['runtime-0']['passed'] = 20
        with self.assertRaises(RuntimeError):
            A.recipes_from(previous, prior, A.E / 'fresh', proposal()['files'])

    def test_prior_receipt_requires_natural_exact_cpu1037(self):
        value = dict(schema='ferric-projection-ar4-cpu-result-v1', passed=True, error=None,
            postcheck_errors=[], source_unchanged=True, tests_passed=1037, tests_ignored=4,
            phases={str(i): dict(exit_code=0, reason=None, group_absent=True) for i in range(87)},
            raw={str(i): {} for i in range(442)}, binaries={str(i): {} for i in range(17)})
        A.prior_contract(value)
        bad = copy.deepcopy(value); bad['phases']['0']['reason'] = 'deadline'
        with self.assertRaises(RuntimeError):
            A.prior_contract(bad)
        bad = copy.deepcopy(value); bad['tests_passed'] = 1022
        with self.assertRaises(RuntimeError):
            A.prior_contract(bad)


if __name__ == '__main__':
    unittest.main(verbosity=2)
