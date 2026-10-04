"""Synthetic source/census/parser/selection tests; not actual CPU or GPU evidence."""
import copy
import json
from pathlib import Path
import types
import unittest
from unittest.mock import Mock, patch

import bank_batch_portable as B


def pin(path, sha='a' * 64, size=1):
    return dict(path=str(path), bytes=size, sha256=sha)


def overlay():
    evidence = Path('/owned/evidence')
    rows = []
    for project, directory, root, names in (
        ('fe2o3', B.RUNTIME_DIR, B.RUNTIME_ROOT, B.RUNTIME_FILES),
        ('ferric', B.FERRIC_DIR, B.FERRIC_ROOT, B.FERRIC_FILES),
    ):
        for name in names:
            added = name in ('engineering_gfx950_peer_state_bank_v1.rs',
                             'engineering_gfx950_peer_state_bank_v1_tests.rs', 'bank_batch_tests.rs')
            rows.append(dict(project=project, path=root + name,
                source=directory + ('/source/' + name if project == 'fe2o3' else '/draft/' + root + name),
                before=None if added else dict(bytes=1, sha256='a' * 64),
                after=dict(bytes=2, sha256='b' * 64)))
    return dict(schema='ferric-p228-state-bank-batch-cpu-overlay-v1',
        runtime_preimages=pin(evidence / B.RUNTIME_DIR / 'preimages.json'),
        ferric_source_manifest=pin(evidence / B.FERRIC_DIR / 'source-pins.json'), files=rows), evidence


def source_map(rows):
    result = {r['project'] + '/' + r['path']: r['before'] for r in rows if r['before'] is not None}
    result.update({f'unchanged/{i}': dict(bytes=3, sha256='c' * 64) for i in range(6928 - len(result))})
    return result


def cpu_fixture():
    inherited = [pin('/prior/input' + str(n)) for n in range(127)]
    fresh = [pin('/new/input' + str(n)) for n in range(17)]
    inputs = inherited + fresh + inherited[:7]
    aliases = {p['path']: dict(original=p, relocated=pin('/prior/objects/' + str(n)))
               for n, p in enumerate(inherited + [pin('/prior/review')])}
    cpu = dict(schema='ferric-p228-state-bank-batch-cpu-result-v1', passed=True,
        error=None, postcheck_errors=[], inputs=inputs, package_manifest=pin('/new/manifest'),
        overlay=pin('/new/overlay'), prior_cpu_complete=pin('/prior/complete'), source_unchanged=True,
        metadata={}, phases={}, tests={}, binaries={B.P.WORKER: dict(binary=pin('/new/worker'))},
        tests_passed=553, tests_ignored=4, empty_initial_target=True, external_cargo_cache_reused=True,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False, production_authority=False,
        raw={str(n): pin('/raw/' + str(n)) for n in range(109)})
    return cpu, aliases


class Store:
    """Already-authenticated storage boundary for semantic routing tests."""
    def __init__(self, bodies):
        self.bodies = bodies

    def get(self, record, maximum=None):
        return self.bodies[record['path']]


def artifact_fixture():
    directory = Path('/owned/state-bank-batch-cpu-v228-v1')
    worker = directory / 'sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'
    binary = pin(directory / 'target/debug' / B.P.WORKER, 'd' * 64, 8)
    artifact = dict(reason='compiler-artifact', manifest_path=str(worker), executable=binary['path'],
        target=dict(name=B.P.WORKER, kind=['bin']), profile=dict(test=False, opt_level='2'))
    cpu = dict(binaries={B.P.WORKER: dict(binary=binary, artifact=artifact)})
    prior = dict(binaries={B.P.WORKER: dict(binary=pin('/prior/worker', 'e' * 64, 8))})
    store = Store({binary['path']: b'\x7fELF\x02\x01xx'})
    raw = lambda: (json.dumps(artifact) + '\n' + json.dumps(dict(reason='build-finished', success=True)) + '\n').encode()
    return store, cpu, raw, directory, worker, prior


def parser_fixture():
    prior_runtime = set()
    for _name, selector, count in B.FILTERS:
        if selector == B.BANK_SELECTOR:
            continue
        named = {name for name in B.S.NEW_TESTS if name.startswith(selector)}
        named.update(selector + 'inherited_' + str(n) for n in range(count - len(named)))
        prior_runtime.update(named)
    prior_main = {'old_worker_' + str(n) for n in range(389)}
    shared = {'shared_wire_' + str(n) for n in range(13)}
    prior_worker = prior_main | shared
    ignored = {'old_worker_' + str(n) for n in range(385, 389)}
    inventory = lambda names: ''.join(name + ': test\n' for name in sorted(names)).encode()
    def outcome(names, ignored, summaries):
        text = ''.join('test ' + name + ' ... ' + ('ignored' if name in ignored else 'ok') + '\n'
                       for name in sorted(names))
        text += ''.join(f'test result: ok. {passed} passed; {failed} failed; {skipped} ignored;\n'
                        for passed, failed, skipped in summaries)
        return text.encode()
    prior = dict(raw={name: pin('/prior/' + name) for name in
        ('runtime-list-stdout', 'worker-list-stdout', 'worker-tests-stdout')})
    store = Store({'/prior/runtime-list-stdout': inventory(prior_runtime),
        '/prior/worker-list-stdout': inventory(prior_worker),
        '/prior/worker-tests-stdout': outcome(prior_worker, ignored, [[385, 0, 4], [13, 0, 0]])})
    runtime, worker = prior_runtime | B.BANK_TESTS, prior_worker | B.WORKER_TESTS
    outputs = {'runtime-list': inventory(runtime), 'worker-list': inventory(worker)}
    tests = {}
    for name, selector, count in B.FILTERS:
        subset = {value for value in runtime if selector in value}
        outputs[name] = outcome(subset, set(), [[count, 0, 0]])
        tests[name] = B.P.test_results(outputs[name], subset, [[count, 0, 0]])
    outputs['worker-tests'] = outcome(worker, ignored, [[396, 0, 4], [13, 0, 0]])
    tests['worker'] = B.P.test_results(outputs['worker-tests'], worker, [[396, 0, 4], [13, 0, 0]])
    return outputs, dict(tests=tests), store, prior


class SourceTests(unittest.TestCase):
    def test_frozen_source_gate_refuses_missing_actual_source_pin(self):
        B.frozen_source_gate()
        for field in ('PACKAGE_SHA', 'RUNNER_SHA', 'OVERLAY_SHA'):
            with patch.object(B, field, None), self.subTest(field=field), self.assertRaises(RuntimeError):
                B.frozen_source_gate()

    def test_exact_formatted_overlay_roster(self):
        value, evidence = overlay()
        self.assertIs(B.overlay_shape(value, evidence), value)
        self.assertEqual(sum(row['before'] is None for row in value['files']), 3)

    def test_overlay_refuses_old_namespace_traversal_duplicate_and_wrong_new_path(self):
        for mutation in ('old', 'traversal', 'duplicate', 'extra_new'):
            value, evidence = overlay()
            if mutation == 'old':
                value['runtime_preimages']['path'] = str(evidence / 'p228-state-bank-batch-runtime-v1/preimages.json')
            elif mutation == 'traversal':
                value['files'][0]['source'] = '../elsewhere'
            elif mutation == 'duplicate':
                value['files'][1] = copy.deepcopy(value['files'][0])
            else:
                value['files'][0]['before'] = None
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                B.overlay_shape(value, evidence)

    def test_source_delta_exact6931_without_mutating_prior(self):
        value, _ = overlay()
        prior = source_map(value['files'])
        before = copy.deepcopy(prior)
        result = B.apply_source_map(prior, value['files'])
        self.assertEqual(len(result), 6931)
        self.assertEqual(prior, before)
        self.assertEqual(len(set(result) - set(prior)), 3)

    def test_first_middle_last_preimage_drift_refused(self):
        value, _ = overlay()
        prior = source_map(value['files'])
        for index in (0, 4, 9):
            rows = copy.deepcopy(value['files'])
            rows[index]['before'] = dict(bytes=88, sha256='8' * 64)
            with self.subTest(index=index), self.assertRaises(RuntimeError):
                B.apply_source_map(prior, rows)

    def test_source_noop_or_wrong_prior_census_refused(self):
        value, _ = overlay()
        prior = source_map(value['files'])
        rows = copy.deepcopy(value['files'])
        rows[0]['after'] = rows[0]['before']
        with self.assertRaises(RuntimeError):
            B.apply_source_map(prior, rows)
        prior.pop(next(iter(prior)))
        with self.assertRaises(RuntimeError):
            B.apply_source_map(prior, value['files'])


class InputTests(unittest.TestCase):
    def test_exact151_input_entries_and144_unique_identities(self):
        cpu, _ = cpu_fixture()
        self.assertEqual(len(B.exact_inputs(cpu['inputs'], list(reversed(cpu['inputs'])))), 144)

    def test_same_set_with_changed_duplicate_multiplicity_refused(self):
        cpu, _ = cpu_fixture()
        changed = copy.deepcopy(cpu['inputs'])
        changed[-1] = copy.deepcopy(changed[-2])
        with self.assertRaises(RuntimeError):
            B.exact_inputs(changed, cpu['inputs'])

    def test_conflicting_inherited_pin_refused(self):
        cpu, _ = cpu_fixture()
        cpu['inputs'][-1] = dict(cpu['inputs'][-1], sha256='f' * 64)
        with self.assertRaises(RuntimeError):
            B.unique_pins(cpu['inputs'])

    def test_delta_courier_keeps_only129_new_records(self):
        cpu, aliases = cpu_fixture()
        records = B.records(pin('/new/complete'), cpu, pin('/new/review'), aliases)
        self.assertEqual(len(records), 129)
        self.assertFalse(set(records).intersection(aliases))
        self.assertIn('/new/worker', records)

    def test_delta_courier_rejects_missing_inherited_identity(self):
        cpu, aliases = cpu_fixture()
        del aliases['/prior/input0']
        with self.assertRaises(RuntimeError):
            B.records(pin('/new/complete'), cpu, pin('/new/review'), aliases)

    def test_receipt_rejects_declared_failure_wrong_count_and_authority(self):
        for key, value in (('passed', False), ('tests_passed', 552), ('tests_ignored', 0),
                           ('source_unchanged', False), ('gpu_execution', True), ('production_authority', True)):
            cpu, _ = cpu_fixture()
            cpu[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                B.receipt_shape(cpu)


class RecipeAndParserTests(unittest.TestCase):
    def test_recipes_preserve_old17_and_add_exact_four_filters(self):
        directory = Path('/owned/state-bank-batch-cpu-v228-v1')
        tools = {'worker': {'root': '/compiler'}}
        old, worker = B.S.recipes(directory, tools)
        rows, actual_worker = B.recipes(directory, tools)
        self.assertEqual(len(rows), 21)
        self.assertEqual(rows[:14], old[:14])
        self.assertEqual(rows[-3:], old[-3:])
        self.assertEqual([r[0] for r in rows[14:18]], [r[0] for r in B.EXTRA_FILTERS])
        self.assertEqual(actual_worker, worker)

    def test_actual_named_outcome_parser_accepts_synthetic553_only_with_inventory(self):
        outputs, cpu, store, prior = parser_fixture()
        B.test_evidence(outputs, cpu, store, prior)
        self.assertEqual(sum(row['passed'] for row in cpu['tests'].values()), 553)

    def test_missing_new_bank_or_worker_inventory_refused(self):
        for field, name in (('runtime-list', next(iter(B.BANK_TESTS))), ('worker-list', next(iter(B.WORKER_TESTS)))):
            outputs, cpu, store, prior = parser_fixture()
            outputs[field] = outputs[field].replace((name + ': test\n').encode(), b'')
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                B.test_evidence(outputs, cpu, store, prior)

    def test_new_worker_test_cannot_be_ignored(self):
        outputs, cpu, store, prior = parser_fixture()
        name = next(iter(B.WORKER_TESTS))
        outputs['worker-tests'] = outputs['worker-tests'].replace(
            ('test ' + name + ' ... ok').encode(), ('test ' + name + ' ... ignored').encode())
        with self.assertRaises(RuntimeError):
            B.test_evidence(outputs, cpu, store, prior)

    def test_old_runtime_test_removal_or_new_unlisted_test_refused(self):
        for actual in ({'new'}, {'old', 'new', 'extra'}):
            with self.subTest(actual=actual), self.assertRaises(RuntimeError):
                B.exact_extension(actual, {'old'}, {'new'})


class ArtifactAndPackageTests(unittest.TestCase):
    def test_worker_pin_comes_from_actual_compiler_artifact(self):
        store, cpu, raw, directory, worker, prior = artifact_fixture()
        self.assertEqual(B.worker_artifact(store, cpu, raw(), directory, worker, prior),
                         cpu['binaries'][B.P.WORKER]['binary'])

    def test_old_binary_test_harness_or_wrong_output_path_refused(self):
        for mutation in ('old', 'test', 'path'):
            store, cpu, raw, directory, worker, prior = artifact_fixture()
            emitted = cpu['binaries'][B.P.WORKER]
            if mutation == 'old':
                emitted['binary']['sha256'] = prior['binaries'][B.P.WORKER]['binary']['sha256']
            elif mutation == 'test':
                emitted['artifact']['profile']['test'] = True
            else:
                emitted['artifact']['executable'] = '/wrong/output'
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                B.worker_artifact(store, cpu, raw(), directory, worker, prior)

    def test_missing_completion_and_nonelf_refused(self):
        store, cpu, raw, directory, worker, prior = artifact_fixture()
        with self.assertRaises(RuntimeError):
            B.worker_artifact(store, cpu, raw().splitlines()[0], directory, worker, prior)
        store.bodies[cpu['binaries'][B.P.WORKER]['binary']['path']] = b'not ELF!'
        with self.assertRaises(RuntimeError):
            B.worker_artifact(store, cpu, raw(), directory, worker, prior)

    def test_old_supervisor_package_cannot_authenticate_new_helpers(self):
        directory = Path(B.__file__).resolve().parent
        I = types.SimpleNamespace(__file__=str(directory / 'intake.py'),
            package_record=lambda _: (dict(files=[dict(path='intake.py')]), pin('/old/manifest')))
        with self.assertRaises(RuntimeError):
            B.checked_package(I, object())

    def test_successor_package_must_be_colocated_and_include_all_new_helpers(self):
        directory = Path(B.__file__).resolve().parent
        manifest = dict(files=[dict(path=name) for name in sorted(B.FILES)])
        I = types.SimpleNamespace(__file__=str(directory / 'intake.py'),
            package_record=lambda _: (manifest, pin('/new/manifest')))
        self.assertEqual(B.checked_package(I, object())[0], manifest)
        I.__file__ = '/elsewhere/intake.py'
        with self.assertRaises(RuntimeError):
            B.checked_package(I, object())


class UtilityTests(unittest.TestCase):
    def test_runtime_selection_uses_verified_new_worker(self):
        import audit_bank_batch_runtime as A
        record = pin('/deployment/complete.json')
        pins = types.SimpleNamespace(read=Mock(return_value=(record, None)))
        runtime = dict(parent=pin('/parent'), worker=pin('/new-worker'))
        with patch.object(B, 'deployment', return_value=({}, runtime)) as verified:
            out, binary = A.selected(['state-bank-batch-runtime-worker-v228-v1', 'worker',
                                      record['path'], record['sha256']], pins)
        self.assertEqual(binary, runtime['worker'])
        self.assertEqual(out, A.I.E / 'state-bank-batch-runtime-worker-v228-v1')
        verified.assert_called_once_with(A.I.D, pins, record, A.I.historical_deployment)

    def test_old_runtime_namespace_or_unknown_role_refused_before_reads(self):
        import audit_bank_batch_runtime as A
        for label, role in (('resident-state-runtime-worker-v228-v1', 'worker'),
                            ('state-bank-batch-runtime-image-v228-v1', 'image')):
            pins = types.SimpleNamespace(read=Mock())
            with self.subTest(role=role), self.assertRaises(RuntimeError):
                A.selected([label, role, '/deployment', 'a' * 64], pins)
            pins.read.assert_not_called()

    def test_export_requires_all_actual_input_arguments_before_host_work(self):
        import export_bank_batch as X
        with patch.object(X.os, 'getuid') as identity, self.assertRaises(RuntimeError):
            X.main([])
        identity.assert_not_called()


if __name__ == '__main__':
    unittest.main()
