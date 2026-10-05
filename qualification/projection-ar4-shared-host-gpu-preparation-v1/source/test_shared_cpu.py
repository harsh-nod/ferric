"""Synthetic scoped CPU admission; no receipt or executable is fabricated as actual."""
import copy
from pathlib import Path
from types import SimpleNamespace
import unittest

import shared_cpu as O


ROOT = Path('/evidence/projection-ar4-shared-host-cpu-v228-v1')


def outcomes(prefix, passed, ignored=0, summaries=None):
    return dict(passed=passed, ignored=ignored, names=[prefix + str(i) for i in range(passed + ignored)],
                summaries=summaries or [[passed, 0, ignored]])


def fixture():
    phases = {name: dict(exit_code=0, reason=None, group_absent=True)
              for name in ['worker-build', 'parent-builds'] + ['phase-' + str(i) for i in range(61)]}
    raw = {name: dict(path=str(ROOT / name), bytes=1, sha256='a' * 64) for name in
        O.SNAPSHOTS | {name + suffix for name in phases for suffix in
            ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}}
    binaries = {}
    for name in (O.PARENT, O.DEFAULT, O.PLAIN, O.WORKER):
        role = 'worker' if name == O.WORKER else 'parent'
        source = ROOT / 'sources/ferric/adapters' / (
            'tp-peer-finite-engineering-worker-v1' if role == 'worker' else 'm1-engineering-execution-v1')
        path = str(ROOT / 'target' / role / 'debug' / name)
        artifact = dict(reason='compiler-artifact', target=dict(name=name, kind=['bin'], crate_types=['bin'],
            src_path=str(source / 'src' / ('main.rs' if role == 'worker' else 'bin/' + name + '.rs'))),
            executable=path, filenames=[path], manifest_path=str(source / 'Cargo.toml'),
            profile=dict(test=False, opt_level='2', debug_assertions=True, overflow_checks=True, debuginfo=0),
            features=[] if role == 'worker' else ['tp-batch-engineering'])
        binaries[name] = dict(artifact=artifact, binary=dict(path=path, bytes=4000, sha256='b' * 64))
    value = dict(schema='ferric-p228-projection-ar4-shared-host-cpu-result-v1', passed=True,
        error=None, postcheck_errors=[], source_unchanged=True, empty_initial_target=True,
        metadata=dict(parent={}, worker={}), phases=phases, raw=raw, binaries=binaries,
        tests={'worker-tests': outcomes('worker::', 532, 4, [[519, 0, 4], [13, 0, 0]]),
               'parent-client': outcomes('parent::', 351)},
        tests_passed=883, tests_ignored=4, historical_runtime_tests_not_repeated=208)
    value.update({key: False for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
        'production_authority', 'full_cpu1037_cohort_requalified', 'compiler_requalified')})
    return value


def delta_fixture():
    shared = [O.REPORT + 'new_' + str(i) for i in range(8)]
    additions = dict(worker=shared + ['worker_new::case_' + str(i) for i in range(6)],
        parent_library=shared + ['tp_finite_client::new_' + str(i) for i in range(5)],
        parent_binary=['tests::new_parent'])
    prior = dict(tests={'worker-tests': outcomes('worker::', 518, 4),
        'parent-client': outcomes('tp_finite_client::old_', 328),
        'parent-projection-host-report': outcomes(O.REPORT + 'old_', 8),
        O.DEFAULT + '-tests': outcomes('tests::default_', 1)})
    value = dict(tests=copy.deepcopy(prior['tests']), declared_test_additions=copy.deepcopy(additions))
    value['tests']['worker-tests']['names'] += additions['worker']
    value['tests']['parent-client']['names'] += [n for n in additions['parent_library'] if not n.startswith(O.REPORT)]
    value['tests']['parent-projection-host-report']['names'] += shared
    value['tests'][O.PARENT + '-tests'] = dict(names=additions['parent_binary'])
    return value, prior, dict(added_tests=additions)


class SharedCpuTests(unittest.TestCase):
    def test_actual_artifact_fields_not_guessed_future_hash_constants(self):
        value = fixture(); selected = O.contract(value, ROOT)
        self.assertEqual(set(selected), {'default', 'shared', 'worker'})
        self.assertEqual(selected['shared']['artifact']['target']['name'], O.PARENT)
        value['binaries'][O.PARENT]['binary']['sha256'] = 'c' * 64
        self.assertEqual(O.contract(value, ROOT)['shared']['binary']['sha256'], 'c' * 64)

    def test_scoped_counts_and_natural_success_are_required(self):
        for key, bad in (('tests_passed', 1063), ('tests_ignored', 0), ('passed', False), ('postcheck_errors', ['drift'])):
            value = fixture(); value[key] = bad
            with self.subTest(key=key), self.assertRaises(RuntimeError): O.contract(value, ROOT)
        value = fixture(); value['phases']['worker-build']['exit_code'] = 101
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_cpu1037_and_full_cohort_are_not_current_observer_qualification(self):
        value = fixture(); value['schema'] = 'ferric-projection-ar4-cpu-result-v1'
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)
        for key in ('full_cpu1037_cohort_requalified', 'compiler_requalified', 'gpu_execution', 'performance_claim'):
            value = fixture(); value[key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_worker_library_shared_wire_split_and_named_outcomes(self):
        value = fixture(); value['tests']['worker-tests']['summaries'] = [[532, 0, 4]]
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)
        value = fixture(); value['tests']['worker-tests']['names'][-1] = value['tests']['worker-tests']['names'][0]
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_exact_raw_roster_and_original_role_paths(self):
        value = fixture(); value['raw']['sources-before.json']['path'] = '/different/sources-before.json'
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)
        value = fixture(); value['binaries'][O.PARENT]['binary']['path'] = str(ROOT / 'target/worker/debug' / O.PARENT)
        with self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_observer_cargo_target_not_plain_parent_or_test_harness(self):
        for key, bad in (('profile', dict(test=True, opt_level='2', debug_assertions=True, overflow_checks=True, debuginfo=0)),
                         ('features', []), ('executable', '/wrong')):
            value = fixture(); value['binaries'][O.PARENT]['artifact'][key] = bad
            with self.subTest(key=key), self.assertRaises(RuntimeError): O.contract(value, ROOT)

    def test_old_named_tests_survive_with_exact_additions_only(self):
        value, prior, proposal = delta_fixture(); O.test_delta(value, prior, proposal)
        for name in ('worker-tests', 'parent-client', 'parent-projection-host-report'):
            bad = copy.deepcopy(value); bad['tests'][name]['names'].pop()
            with self.subTest(name=name), self.assertRaises(RuntimeError): O.test_delta(bad, prior, proposal)

    def test_root_pin_namespace_refuses_old_cpu_before_any_reader(self):
        fake = SimpleNamespace(E=Path('/evidence'))
        pin = dict(path='/evidence/projection-ar4-cpu-v228-v1/complete.json', bytes=1, sha256='a' * 64)
        with self.assertRaises(RuntimeError):
            O.evidence(fake, None, dict(parent_cpu=pin, worker_cpu=pin, route='shared'), {})


if __name__ == '__main__':
    unittest.main(verbosity=2)
