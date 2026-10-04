"""Pure synthetic regression fixtures; never import build tools or launch a process."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import Mock, patch

import resident_state_portable as S


def pin(path, digest='a' * 64, size=1):
    return dict(path=str(path), bytes=size, sha256=digest)


class Store:
    """Already-authenticated Store boundary; these fixtures test semantic joins."""
    def __init__(self, docs=None, bodies=None):
        self.docs = docs or {}
        self.bodies = bodies or {}

    def doc(self, row, maximum=None):
        return self.docs[row['path']]

    def get(self, row):
        return self.bodies.get(row['path'], b'synthetic authenticated body')


def source_fixture():
    root = Path('/owned/evidence/p228-resident-state-fence-cpu-v2')
    source = root.parent / 'p228-resident-state-fence-consolidation-v2'
    package_pin = pin(root / 'manifest.json', S.PACKAGE_SHA)
    overlay_pin = pin(root / 'overlay.json', S.OVERLAY_SHA)
    runner_pin = pin(root / 'run.py', S.RUNNER_SHA)
    readme_pin = pin(root / 'README.md')
    package_files = [runner_pin, overlay_pin, readme_pin]
    preimage_pin = pin(source / 'preimages.json')
    before = dict(bytes=1, sha256='b' * 64)
    after = dict(bytes=2, sha256='c' * 64)
    rows = [dict(path=f'crates/runtime/state_{i}.rs', before=dict(before), after=dict(after)) for i in range(8)]
    prior_sources = {f'fe2o3/unchanged_{i}': dict(before) for i in range(6920)}
    prior_sources.update({'fe2o3/' + row['path']: dict(before) for row in rows})
    installed = dict(prior_sources)
    installed.update({'fe2o3/' + row['path']: dict(after) for row in rows})
    inherited = [pin(root.parent / f'inherited_{i}') for i in range(24)]
    prior_cpu = dict(inputs=[*inherited, pin(root.parent / 'run_group_fence_cpu_p228_v2.py')],
                     raw={'sources-after.json': pin(root.parent / 'old-sources-after.json')})
    cpu = dict(inputs=[*inherited, package_pin, *package_files, preimage_pin,
                       *(dict(row['after'], path=str(source / 'source' / row['path'])) for row in rows)],
               package_manifest=package_pin, state_overlay=overlay_pin,
               raw={name: pin(root.parent / name) for name in
                    ('sources-base.json', 'sources-before.json', 'sources-after.json')})
    archive_map = {'ferric/original': dict(before)}
    docs = {
        package_pin['path']: dict(schema='ferric-p228-resident-state-fence-cpu-package-v1',
            files=[dict(row, path=Path(row['path']).name) for row in package_files]),
        overlay_pin['path']: dict(schema='ferric-p228-resident-state-fence-cpu-overlay-v1',
            baseline_head='725ecc6a500ff49e7dfaefb38b027f6bcc223ebf',
            source_directory=source.name, preimages={k: preimage_pin[k] for k in ('bytes', 'sha256')}, files=rows),
        preimage_pin['path']: dict(schema='ferric-p228-resident-state-fence-source-preimages-v1',
            base_commit='725ecc6a500ff49e7dfaefb38b027f6bcc223ebf',
            files={row['path']: row['before']['sha256'] for row in rows}),
        cpu['raw']['sources-base.json']['path']: archive_map,
        cpu['raw']['sources-before.json']['path']: copy.deepcopy(installed),
        cpu['raw']['sources-after.json']['path']: copy.deepcopy(installed),
    }
    prior_store = Store({prior_cpu['raw']['sources-after.json']['path']: prior_sources})
    return Store(docs), cpu, prior_store, prior_cpu, {'archives': {}}, archive_map


def phases_fixture():
    directory = Path('/owned/evidence/finite/resident-state-fence-cpu-v228-v2')
    tools = {'worker': {'root': '/nightly'}}
    recipes, _ = S.recipes(directory, tools)
    raw, docs, phases = {}, {}, {}
    for name in ('sources-base.json', 'sources-before.json', 'sources-after.json'):
        raw[name] = pin(directory / name)
    for name, argv, deadline in recipes:
        for suffix in ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json'):
            raw[name + suffix] = pin(directory / (name + suffix))
        command = dict(argv=argv, env=S.P.cpu_environment(directory, tools, 'worker'),
            tools=S.P.TOOL_PINS['worker'], deadline_seconds=deadline, cache_cap_bytes=6 << 30,
            affinity=[8, 9], nice=10, gpu_execution=False, expected_exit=0)
        result = dict(exit_code=0, reason=None, elapsed_seconds=1.0, group_absent=True,
            cache_bytes=1000, stdout_sha256='a' * 64, stderr_sha256='a' * 64)
        docs[raw[name + '-command.json']['path']] = command
        docs[raw[name + '-started.json']['path']] = dict(pid=100, pgid=100)
        docs[raw[name + '-result.json']['path']] = result
        phases[name] = dict(result)
    return Store(docs), dict(raw=raw, phases=phases), directory, tools


def tests_fixture():
    outputs, tests, runtime = {}, {}, []
    for name, selector, count in S.FILTERS:
        required = sorted(test for test in S.NEW_TESTS if selector in test)
        names = required + [selector + f'case_{i}' for i in range(count - len(required))]
        runtime += names
        outputs[name] = ('\n'.join('test ' + n + ' ... ok' for n in names)
            + f'\ntest result: ok. {count} passed; 0 failed; 0 ignored;\n').encode()
        tests[name] = dict(passed=count, ignored=0, names=sorted(names), summaries=[[count, 0, 0]])
    worker_names = [f'worker::case_{i}' for i in range(402)]
    outputs['runtime-list'] = ('\n'.join(n + ': test' for n in runtime) + '\n').encode()
    outputs['worker-list'] = ('\n'.join(n + ': test' for n in worker_names) + '\n').encode()
    outputs['worker-tests'] = ('\n'.join('test ' + n + ' ... ' + ('ignored' if i < 4 else 'ok')
        for i, n in enumerate(worker_names))
        + '\ntest result: ok. 385 passed; 0 failed; 4 ignored;\n'
        + 'test result: ok. 13 passed; 0 failed; 0 ignored;\n').encode()
    tests['worker'] = dict(passed=398, ignored=4, names=sorted(worker_names), summaries=[[385, 0, 4], [13, 0, 0]])
    return outputs, {'tests': tests}


def artifact_fixture():
    directory = Path('/owned/evidence/finite/resident-state-fence-cpu-v228-v2')
    worker = directory / 'sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'
    local = {name: str(directory / 'sources/fe2o3/crates' / name / 'Cargo.toml') for name in S.P.RUNTIME_CRATES}
    local[S.P.WORKER] = str(worker)
    packages = [dict(id=name, name=name, manifest_path=path, source=None) for name, path in local.items()]
    external = [dict(name=f'ext_{i}', version='1.0', source='registry', manifest=f'/cache/ext_{i}/Cargo.toml',
                     manifest_sha256='a' * 64) for i in range(39 - len(packages))]
    packages += [dict(id=row['name'], name=row['name'], version=row['version'], source=row['source'],
                      manifest_path=row['manifest']) for row in external]
    metadata = dict(target_directory=str(directory / 'target'), packages=packages,
        resolve={'nodes': [dict(id='fe2o3-kfd', features=['engineering-gfx950'])]})
    binary = pin(directory / 'target/debug' / S.P.WORKER, S.WORKER_SHA, 4780024)
    artifact = dict(reason='compiler-artifact', manifest_path=str(worker),
        target=dict(name=S.P.WORKER, kind=['bin']), profile=dict(test=False, opt_level='2'), executable=binary['path'])
    cpu = dict(metadata=dict(package_count=39, local=local, external=external),
               binaries={S.P.WORKER: dict(binary=binary, artifact=copy.deepcopy(artifact))})
    outputs = dict(metadata=json.dumps(metadata).encode(), **{'worker-build':
        (json.dumps(artifact) + '\n' + json.dumps(dict(reason='build-finished', success=True))).encode()})
    store = Store(bodies={binary['path']: b'\x7fELF\x02\x01synthetic'})
    return store, cpu, outputs, directory, worker, {'metadata': {'worker': {'external': copy.deepcopy(external)}}}


class ResidentStateTests(unittest.TestCase):
    def test_seventeen_recipes_preserve_original_twelve_and_limits(self):
        directory, tools = Path('/owned/cpu'), {'worker': {'root': '/nightly'}}
        rows, worker = S.recipes(directory, tools)
        old, old_worker = S.G.recipes(directory, tools)
        self.assertEqual(len(rows), 17)
        self.assertEqual(worker, old_worker)
        self.assertEqual([r for r in rows if r[0] not in {f[0] for f in S.EXTRA_FILTERS}], old)
        for name, argv, deadline in rows:
            self.assertEqual(deadline, 120 if name == 'metadata' else 1200)
            self.assertNotIn('--features', argv)
            self.assertEqual(argv[0], '/nightly/bin/cargo')

    def test_extra_runtime_inventory_is_forty_seven_and_names_are_covered(self):
        self.assertEqual(sum(row[2] for row in S.EXTRA_FILTERS), 47)
        self.assertEqual(sum(row[2] for row in S.FILTERS), 124)
        self.assertEqual(len(S.NEW_TESTS), 6)
        for name in S.NEW_TESTS:
            self.assertEqual(sum(selector in name for _, selector, _ in S.FILTERS), 1)

    def test_source_replay_accepts_only_eight_body_delta_without_live_source_paths(self):
        store, cpu, prior_store, prior_cpu, base_cpu, archive_map = source_fixture()
        with patch.object(S.P, 'git_source_map', return_value=archive_map):
            runner, overlay = S.source_delta(store, cpu, prior_store, prior_cpu, base_cpu)
        self.assertEqual(runner['sha256'], S.RUNNER_SHA)
        self.assertEqual(overlay, cpu['state_overlay'])

    def test_source_replay_rejects_wrong_preimage_even_with_matching_git_label(self):
        store, cpu, prior_store, prior_cpu, base_cpu, _ = source_fixture()
        store.docs[cpu['state_overlay']['path']]['files'][0]['before']['sha256'] = 'd' * 64
        with self.assertRaises(RuntimeError):
            S.source_delta(store, cpu, prior_store, prior_cpu, base_cpu)

    def test_source_replay_rejects_duplicate_source_and_missing_inherited_input(self):
        for mutation in ('duplicate', 'missing'):
            store, cpu, prior_store, prior_cpu, base_cpu, _ = source_fixture()
            if mutation == 'duplicate':
                rows = store.docs[cpu['state_overlay']['path']]['files']
                rows[1] = copy.deepcopy(rows[0])
            else:
                cpu['inputs'].pop(0)
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                S.source_delta(store, cpu, prior_store, prior_cpu, base_cpu)

    def test_source_replay_rejects_changed_source_snapshot(self):
        store, cpu, prior_store, prior_cpu, base_cpu, archive_map = source_fixture()
        store.docs[cpu['raw']['sources-after.json']['path']]['fe2o3/unchanged_0'] = dict(bytes=99, sha256='d' * 64)
        with patch.object(S.P, 'git_source_map', return_value=archive_map), self.assertRaises(RuntimeError):
            S.source_delta(store, cpu, prior_store, prior_cpu, base_cpu)

    def test_source_replay_rejects_wrong_archive_map(self):
        store, cpu, prior_store, prior_cpu, base_cpu, _ = source_fixture()
        with patch.object(S.P, 'git_source_map', return_value={}), self.assertRaises(RuntimeError):
            S.source_delta(store, cpu, prior_store, prior_cpu, base_cpu)

    def test_receipt_rejects_old_cohort_and_authority(self):
        value = dict(schema='ferric-p228-resident-state-fence-cpu-result-v1', passed=True, error=None,
            postcheck_errors=[], inputs=[], package_manifest={}, state_overlay={}, source_unchanged=True,
            metadata={}, phases={}, tests={}, binaries={}, tests_passed=522, tests_ignored=4,
            empty_initial_target=True, external_cargo_cache_reused=True, gpu_execution=False,
            numerical_acceptance=False, performance_claim=False, production_authority=False, raw={})
        S.receipt_shape(value)
        for key, wrong in (('tests_passed', 475), ('tests_passed', True), ('tests_ignored', 0),
                           ('source_unchanged', False), ('gpu_execution', True),
                           ('numerical_acceptance', True), ('performance_claim', True),
                           ('production_authority', True), ('error', 'failed')):
            with self.subTest(key=key, wrong=wrong), self.assertRaises(RuntimeError):
                S.receipt_shape(dict(value, **{key: wrong}))

    def test_phase_replay_accepts_all_seventeen_actual_recipe_shapes(self):
        store, cpu, directory, tools = phases_fixture()
        outputs, _ = S.phase_evidence(store, cpu, directory, tools)
        self.assertEqual(set(outputs), set(cpu['phases']))

    def test_phase_replay_rejects_missing_raw_record(self):
        store, cpu, directory, tools = phases_fixture()
        del cpu['raw']['mlp-state-stderr']
        with self.assertRaises(RuntimeError):
            S.phase_evidence(store, cpu, directory, tools)

    def test_phase_replay_rejects_changed_environment_or_command(self):
        for mutation in ('environment', 'command'):
            store, cpu, directory, tools = phases_fixture()
            command = store.docs[cpu['raw']['prefix-state-command.json']['path']]
            if mutation == 'environment':
                command['env']['HIP_VISIBLE_DEVICES'] = '0'
            else:
                command['argv'] = command['argv'] + ['--ignored']
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                S.phase_evidence(store, cpu, directory, tools)

    def test_phase_replay_rejects_killed_live_or_wrong_owned_leaf(self):
        for key, wrong in (('exit_code', -9), ('reason', 'deadline'), ('group_absent', False),
                           ('stdout_sha256', 'f' * 64)):
            store, cpu, directory, tools = phases_fixture()
            store.docs[cpu['raw']['worker-tests-result.json']['path']][key] = wrong
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                S.phase_evidence(store, cpu, directory, tools)
        store, cpu, directory, tools = phases_fixture()
        store.docs[cpu['raw']['worker-tests-started.json']['path']]['pgid'] = 200
        with self.assertRaises(RuntimeError):
            S.phase_evidence(store, cpu, directory, tools)

    def test_real_test_parser_accepts_synthetic_522_and_four_ignored(self):
        outputs, cpu = tests_fixture()
        S.test_evidence(outputs, cpu)

    def test_test_replay_requires_each_new_named_regression(self):
        outputs, cpu = tests_fixture()
        name = sorted(S.NEW_TESTS)[0]
        replacement = name + '_not_the_required_test'
        outputs = {key: raw.replace(name.encode(), replacement.encode()) for key, raw in outputs.items()}
        with self.assertRaises(RuntimeError):
            S.test_evidence(outputs, cpu)

    def test_test_replay_rejects_forged_totals_and_changed_outcomes(self):
        outputs, cpu = tests_fixture()
        cpu['tests']['worker']['passed'] = 522
        with self.assertRaises(RuntimeError):
            S.test_evidence(outputs, cpu)
        outputs, cpu = tests_fixture()
        outputs['worker-tests'] = outputs['worker-tests'].replace(b' ... ok', b' ... ignored', 1)
        with self.assertRaises(RuntimeError):
            S.test_evidence(outputs, cpu)

    def test_wrong_deployment_cohort_refuses_before_any_read(self):
        value = dict(schema=S.SCHEMA, base_deployment={'sha256': S.G.BASE_SHA}, prior_deployment={},
            worker_cpu={'sha256': S.CPU_SHA, 'bytes': 120723}, worker_cpu_review={}, aliases={}, runtime={})
        for key, wrong in (('schema', S.G.SCHEMA), ('base_deployment', {'sha256': 'a' * 64}),
                           ('worker_cpu', {'sha256': S.G.CPU_SHA, 'bytes': 120723})):
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                S.verify(None, None, dict(value, **{key: wrong}), Path('/unused'), None)

    def test_artifact_replay_accepts_new_worker_only_and_exact_metadata(self):
        S.artifact_evidence(*artifact_fixture())

    def test_artifact_replay_rejects_old_worker_or_non_elf(self):
        for mutation in ('old-worker', 'not-elf'):
            args = artifact_fixture()
            binary = args[1]['binaries'][S.P.WORKER]['binary']
            if mutation == 'old-worker':
                binary['sha256'] = S.G.WORKER_SHA
            else:
                args[0].bodies[binary['path']] = b'not ELF'
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                S.artifact_evidence(*args)

    def test_artifact_replay_rejects_wrong_profile_even_when_receipt_matches(self):
        for field, wrong in (('test', True), ('opt_level', '0')):
            args = artifact_fixture()
            artifact = args[1]['binaries'][S.P.WORKER]['artifact']
            artifact['profile'][field] = wrong
            args[2]['worker-build'] = (json.dumps(artifact) + '\n'
                + json.dumps(dict(reason='build-finished', success=True))).encode()
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                S.artifact_evidence(*args)

    def test_artifact_replay_rejects_live_feature_and_dependency_drift(self):
        for mutation in ('live-feature', 'dependency'):
            args = artifact_fixture()
            if mutation == 'live-feature':
                metadata = json.loads(args[2]['metadata'])
                metadata['resolve']['nodes'][0]['features'].append('live-validation')
                args[2]['metadata'] = json.dumps(metadata).encode()
            else:
                args[1]['metadata']['external'][0]['manifest_sha256'] = 'f' * 64
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                S.artifact_evidence(*args)

    def test_courier_exact_census_and_conflicting_aliases(self):
        inputs = [pin(f'/input/{i}') for i in range(37)]
        raw = {f'raw_{i}': pin(f'/raw/{i}') for i in range(88)}
        cpu = dict(inputs=inputs, raw=raw, binaries={S.P.WORKER: {'binary': pin('/worker')}})
        self.assertEqual(len(S.records(pin('/complete'), cpu, pin('/review'))), 128)
        with self.assertRaises(RuntimeError):
            S.records(pin('/complete'), cpu, pin('/input/0', digest='b' * 64))
        cpu['inputs'].pop()
        with self.assertRaises(RuntimeError):
            S.records(pin('/complete'), cpu, pin('/review'))

    def test_actual_new_worker_and_receipt_are_not_cpu475(self):
        self.assertNotEqual(S.WORKER_SHA, S.G.WORKER_SHA)
        self.assertNotEqual(S.CPU_SHA, S.G.CPU_SHA)

    def test_runtime_auditor_refuses_old_namespace_or_unknown_role_before_read(self):
        import audit_resident_state_runtime as A
        pins = Mock()
        for label, role in (('group-fence-runtime-worker-v228-v1', 'worker'),
                            ('resident-state-runtime-image-v228-v1', 'image')):
            with self.subTest(role=role), self.assertRaises(RuntimeError):
                A.selected([label, role, '/deployment/complete.json', 'a' * 64], pins)
        pins.read.assert_not_called()

    def test_runtime_auditor_selects_new_deployment_worker_or_unchanged_parent(self):
        import audit_resident_state_runtime as A
        record = pin('/deployment/complete.json')
        runtime = dict(parent=pin('/parent'), worker=pin('/new-worker', S.WORKER_SHA), image=pin('/image'))
        for role in ('parent', 'worker'):
            pins = Mock()
            pins.read.return_value = (record, b'{}')
            label = 'resident-state-runtime-' + role + '-v228-v1'
            with patch.object(A.S, 'deployment', return_value=({}, runtime)) as replay:
                out, binary = A.selected([label, role, record['path'], record['sha256']], pins)
            replay.assert_called_once_with(A.I.D, pins, record, A.I.historical_deployment)
            self.assertEqual(out, A.I.E / label)
            self.assertEqual(binary, runtime[role])


if __name__ == '__main__':
    unittest.main()
