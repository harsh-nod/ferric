"""Synthetic paired-CPU admission only; no real receipt, build, or ELF is created."""
import copy
from pathlib import Path
from types import SimpleNamespace
import unittest

import ordered_cpu as O
import test_shared_cpu as B

ROOT = Path('/evidence/projection-ordered-segment-cpu-v228-v2')


def fixture():
    value = B.fixture()
    value.update(schema='ferric-p228-projection-ordered-segment-cpu-result-v1',
        complete_runtime_library_suite_selected=True, tests_passed=1848, tests_ignored=7,
        declared_test_additions={k: sorted(k + '::new_' + str(i) for i in range(n))
                                 for k, n in O.ADDED_COUNTS.items()})
    for i in range(12): value['phases']['extra-' + str(i)] = dict(exit_code=0, reason=None, group_absent=True)
    value['raw'] = {name: dict(path=str(ROOT / name), bytes=1, sha256='a' * 64) for name in
        O.B.SNAPSHOTS | {name + suffix for name in value['phases'] for suffix in
            ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}}
    value['binaries'][O.PARENT] = copy.deepcopy(value['binaries'][O.B.PARENT])
    for name, row in value['binaries'].items():
        role = 'worker' if name == O.B.WORKER else 'parent'
        source = ROOT / 'sources/ferric/adapters' / (
            'tp-peer-finite-engineering-worker-v1' if role == 'worker' else 'm1-engineering-execution-v1')
        path = str(ROOT / 'target' / role / 'debug' / name)
        row['binary']['path'] = path
        row['artifact'].update(executable=path, filenames=[path], manifest_path=str(source / 'Cargo.toml'))
        row['artifact']['target'].update(name=name,
            src_path=str(source / 'src' / ('main.rs' if role == 'worker' else 'bin/' + name + '.rs')))
    value['tests'] = {'worker-tests': B.outcomes('worker::', 562, 4, [[549, 0, 4], [13, 0, 0]]),
        'runtime-tests': B.outcomes('runtime::', 922, 3), 'parent-client': B.outcomes('parent::', 364)}
    return value


def delta():
    additions = dict(runtime=['runtime::added'], worker=['worker::added'],
        parent_library=['data::added', 'tp_finite_client::added'], parent_binary=['tests::ordered'])
    prior = dict(tests={'worker-tests': B.outcomes('oldworker::', 2, 4),
        'parent-client': B.outcomes('oldparent::', 1), O.B.DEFAULT + '-tests': B.outcomes('oldbin::', 1)})
    value = dict(declared_test_additions=additions, tests=copy.deepcopy(prior['tests']))
    value['tests']['worker-tests']['names'] += additions['worker']
    value['tests']['parent-client']['names'] += ['tp_finite_client::added']
    value['tests']['parent-added-0'] = dict(names=['data::added'], ignored=0)
    runtime = {'runtime::old_' + str(i) for i in range(900)}
    value['tests']['runtime-tests'] = dict(names=sorted(runtime | set(additions['runtime'])), ignored=3)
    value['tests'][O.PARENT + '-tests'] = dict(names=additions['parent_binary'], ignored=0)
    plan = dict(added_tests=additions, parent_selectors={'parent-client': 'tp_finite_client::'})
    return value, prior, plan, runtime


class OrderedCpuTests(unittest.TestCase):
    def test_four_roles_and_five_actual_products(self):
        value = fixture(); selected = O.contract(value, ROOT)
        self.assertEqual(set(selected), {'default', 'shared', 'ordered', 'worker'})
        self.assertNotEqual(selected['ordered'], selected['shared'])

    def test_no_predicted_completion_or_partial_runtime_suite(self):
        for key, bad in (('passed', False), ('complete_runtime_library_suite_selected', False),
                         ('tests_passed', 1846), ('tests_ignored', 4), ('postcheck_errors', ['drift'])):
            value = fixture(); value[key] = bad
            with self.subTest(key=key), self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_all75_leaves_and382_raw_records_are_required(self):
        value = fixture(); value['phases'].pop('extra-0')
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)
        value = fixture(); value['raw'].pop('sources-before.json')
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)
        value = fixture(); value['phases']['worker-build']['reason'] = 'forced cleanup'
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_worker_split_and_runtime_ignores_are_not_hidden(self):
        for name, bad in (('worker-tests', [[562, 0, 4]]), ('runtime-tests', [[925, 0, 0]])):
            value = fixture(); value['tests'][name]['summaries'] = bad
            with self.subTest(name=name), self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_role_path_profile_and_cargo_identity_cannot_be_substituted(self):
        for key in ('executable', 'profile', 'features'):
            value = fixture(); artifact = value['binaries'][O.PARENT]['artifact']
            artifact[key] = '/wrong' if key == 'executable' else (dict(artifact['profile'], test=True) if key == 'profile' else [])
            with self.subTest(key=key), self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_additions_are_closed_sorted_unique_full_names(self):
        value = fixture(); value['declared_test_additions']['runtime'].append('extra')
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)
        value = fixture(); value['declared_test_additions']['worker'].reverse()
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_all_old_names_and_extra_parent_selections_retained(self):
        values = delta(); O.test_delta(*values)
        for name in ('worker-tests', 'parent-client', 'parent-added-0', 'runtime-tests'):
            value, prior, plan, runtime = delta(); value['tests'][name]['names'].pop()
            with self.subTest(name=name), self.assertRaises(RuntimeError): O.test_delta(value, prior, plan, runtime)

    def test_runtime_baseline_has_exact900_names_and_no_addition_collision(self):
        value, prior, plan, runtime = delta(); runtime.pop()
        with self.assertRaises(RuntimeError): O.test_delta(value, prior, plan, runtime)
        value, prior, plan, runtime = delta(); runtime.pop(); runtime.add('runtime::added')
        with self.assertRaises(RuntimeError): O.test_delta(value, prior, plan, runtime)

    def test_old_receipt_namespace_is_refused_before_read(self):
        I = SimpleNamespace(E=Path('/evidence'))
        for label in ('projection-ar4-shared-host-cpu-v228-v1', 'projection-ordered-segment-cpu-v228-v1'):
            pin = dict(path='/evidence/' + label + '/complete.json', bytes=1, sha256='a' * 64)
            with self.subTest(label=label), self.assertRaises(RuntimeError): O.qualified(I, None, pin, {})

    def test_wrong_controller_or_plan_cannot_open_sources(self):
        I = SimpleNamespace(E=Path('/evidence'))
        value = dict(controller=dict(path='/other/run.py', sha256=O.CONTROLLER_SHA),
                     plan=dict(path='/other/inputs.json'))
        with self.assertRaises(RuntimeError): O.sources(I, None, value, ROOT)
        value = dict(controller=dict(path='/evidence/p228-projection-ordered-segment-cpu-v1/run.py',
                                     sha256='732df7080ee87561a6e4581f883896fc03b2ea8befd177bf110019e95289be23'),
                     plan=dict(path='/evidence/p228-projection-ordered-segment-cpu-v1/inputs.json'))
        with self.assertRaises(RuntimeError): O.sources(I, None, value, ROOT)


if __name__ == '__main__':
    unittest.main(verbosity=2)
