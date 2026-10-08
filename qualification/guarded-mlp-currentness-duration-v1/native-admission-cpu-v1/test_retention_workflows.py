"""Synthetic capsule workflows, not observations or permission for native execution.

The closed fixture directory contains unchanged, authenticated Tail V2 originals.
Only an in-memory derivative is renamed and given synthetic diagnostic durations.
Its policies, ownership records and payloads are checked by the real production
validators. Tests never run a process, open model/ELF data, or mock an admission
predicate. verify/retain are exercised; the host/live-tree export entry is not.
"""
import copy
from contextlib import contextmanager
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import retention_tool as E


FIXTURE_DIRECTORY = Path(__file__).resolve().parent / 'fixtures' / 'tail-matched-v2'
ORIGINAL_MANIFEST = dict(bytes=616896,
    sha256='70dbc23f2fddd83bb5406e111954e544033a2c9168877fc976895f1f74d7ea65')
FIXTURE_ROOT = Path('/synthetic/currentness-duration-retention-only')
CASE = 'tail_duration'
CATEGORIES = ('before', 'discover', 'after', 'root_generation')
POLICY_COUNTS = ('before_calls', 'full_discoveries', 'after_calls', 'generation_probes')


def encoded(value):
    return json.dumps(value, separators=(',', ':'), ensure_ascii=True,
                      allow_nan=False).encode()


def identity(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def file_pin(path, body, rust=False):
    result = dict(path=str(path), **identity(body))
    if rust:
        result['sha256'] = list(bytes.fromhex(result['sha256']))
    return result


def original_bodies():
    """Authenticate the fixed original subset before creating any synthetic data."""
    manifest_raw = (FIXTURE_DIRECTORY / 'manifest.json').read_bytes()
    if identity(manifest_raw) != ORIGINAL_MANIFEST:
        raise AssertionError('fixed original Tail V2 manifest drift')
    manifest = json.loads(manifest_raw)
    names = {name for name in manifest['files']
             if name.startswith(('baseline/', 'tail/'))}
    names |= {'tail-input.json', 'tail-request.json', 'prepared-inputs.json'}
    actual = {p.relative_to(FIXTURE_DIRECTORY).as_posix()
              for p in FIXTURE_DIRECTORY.rglob('*') if p.is_file()}
    if len(names) != 151 or actual != names | {'manifest.json'}:
        raise AssertionError('closed152 original fixture bodies')
    bodies = {}
    for name in sorted(names):
        path = FIXTURE_DIRECTORY / name
        if path.is_symlink() or path.resolve(strict=True) != path:
            raise AssertionError('ordinary fixture body')
        body = path.read_bytes()
        if identity(body) != manifest['files'][name]:
            raise AssertionError('original fixture body drift: ' + name)
        bodies[name] = body
    if json.loads(bodies['tail/complete.json'])['passed'] is not True:
        raise AssertionError('source fixture is the original successful Tail case')
    return bodies


def rewrite(value, replacements):
    if isinstance(value, str):
        for old, new in replacements:
            if value.startswith(old):
                return new + value[len(old):]
        return value
    if isinstance(value, list):
        return [rewrite(item, replacements) for item in value]
    if isinstance(value, dict):
        return {rewrite(key, replacements): rewrite(item, replacements)
                for key, item in value.items()}
    return value


def synthetic_diagnostic(policy, original_policy):
    """Distribute original aggregate counts; all elapsed values are synthetic 1ns."""
    groups = {}
    for group, source in zip(('bank', 'layers', 'tail'), ('banks', 'layers', 'tails')):
        groups[group] = []
        for index in range(38):
            row = {}
            for category, count in zip(CATEGORIES, POLICY_COUNTS):
                quotient, remainder = divmod(policy['counts'][source][count], 38)
                row[category] = dict(calls=quotient + (index < remainder), elapsed_ns=1)
            groups[group].append(row)
    rows = [dict(position=position, measured=None if position < 2 else
                 dict(bank=groups['bank'][position - 2],
                      layers=groups['layers'][position - 2],
                      tail=groups['tail'][position - 2], bank_guarded_body_ns=5))
            for position in range(40)]
    return dict(schema='FerricReadiness40TailCurrentnessDurationsV1', instrumented=True,
        policy_sha256=list(hashlib.sha256(original_policy).digest()),
        session=policy['session'], worker_sha256=policy['worker_sha256'],
        transcript_sha256=policy['transcript_sha256'], forwards=rows,
        host_elapsed_nanoseconds=True, bank_guarded_body_includes_callbacks=True,
        numerical_acceptance=False, performance_claim=False, execution_authority=False)


@contextmanager
def bindings(fixture):
    # Only source/data bindings change. Every verification predicate stays real.
    with patch.multiple(E, REMOTE=FIXTURE_ROOT, ROOT_PINS=fixture['root_pins'],
                        ADMISSION=fixture['admission']):
        yield


def update_native(fixture, stderr=None, edit_summary=None):
    """Refresh declared byte custody, never the cached admitted observation."""
    bodies = fixture['bodies']
    prefix = CASE + '/native/'
    summary = json.loads(bodies[prefix + 'complete.json'])
    if stderr is not None:
        bodies[prefix + 'child-stderr.bin'] = stderr
    summary['files']['child_stderr'] = file_pin(
        FIXTURE_ROOT / prefix / 'child-stderr.bin', bodies[prefix + 'child-stderr.bin'], True)
    summary['files']['bytes_before_summary'] = sum(len(bodies[prefix + name]) for name in
        ('frames.ndjson', 'child-stderr.bin', 'capture-0.bin', 'capture-5.bin',
         'capture-16.bin', 'capture-39.bin'))
    if edit_summary is not None:
        edit_summary(summary)
    for _ in range(16):
        raw = encoded(summary)
        if (summary['files']['summary_bytes'] == len(raw)
                and summary['files']['total_bytes'] == summary['files']['bytes_before_summary'] + len(raw)):
            break
        summary['files']['summary_bytes'] = len(raw)
        summary['files']['total_bytes'] = summary['files']['bytes_before_summary'] + len(raw)
    else:
        raise AssertionError('synthetic summary accounting did not converge')
    bodies[prefix + 'complete.json'] = raw
    timing = json.loads(bodies[prefix + 'host-timing.json'])
    timing['ordinary_complete'] = file_pin(FIXTURE_ROOT / prefix / 'complete.json', raw, True)
    bodies[prefix + 'host-timing.json'] = encoded(timing)
    wrapper = json.loads(bodies[CASE + '/parent/stdout'])
    wrapper['observation'] = summary
    status = wrapper['host_timing']
    status['file'] = file_pin(FIXTURE_ROOT / prefix / 'host-timing.json',
                              bodies[prefix + 'host-timing.json'], True)
    status['ordinary_retained_bytes'] = summary['files']['total_bytes']
    status['retained_bytes_with_timing'] = status['ordinary_retained_bytes'] + status['file']['bytes']
    bodies[CASE + '/parent/stdout'] = encoded(wrapper)


def reconcile(fixture):
    """Rehash a fixture envelope so semantic mutations reach real validators."""
    bodies, terminal = fixture['bodies'], fixture['terminal']
    for row in terminal['phases']:
        prefix = CASE + '/' + row['label'] + '/'
        command = bodies[prefix + 'command.json']
        started = json.loads(bodies[prefix + 'started.json'])
        started['command_sha256'] = identity(command)['sha256']
        bodies[prefix + 'started.json'] = encoded(started)
        for key, suffix in (('command', 'command.json'), ('started', 'started.json'),
                            ('stdout', 'stdout'), ('stderr', 'stderr')):
            row[key] = file_pin(FIXTURE_ROOT / prefix / suffix, bodies[prefix + suffix])
        bodies[prefix + 'result.json'] = encoded({key: value for key, value in row.items() if key != 'label'})
    terminal_name = 'complete.json' if terminal['passed'] else 'failed.json'
    for name in ('complete.json', 'failed.json'):
        bodies.pop(CASE + '/' + name, None)
    terminal['raw'] = {name[len(CASE) + 1:]: file_pin(FIXTURE_ROOT / name, body)
                       for name, body in bodies.items() if name.startswith(CASE + '/')}
    terminal['native_started'] = terminal['raw'].get('parent/started.json')
    body = encoded(terminal)
    bodies[CASE + '/' + terminal_name] = body
    fixture['outcomes'] = {CASE: dict(name=terminal_name, sha256=identity(body)['sha256'])}


def make_fixture():
    originals = original_bodies()
    original_terminal = json.loads(originals['tail/complete.json'])
    old_plan = json.loads(originals['tail-input.json'])
    old_root = str(Path(old_plan['request']['path']).parent)
    old_worker = str(Path(old_plan['worker_cpu']['path']).parent.parent)
    old_parent = str(Path(old_plan['parent_cpu']['path']).parent.parent)
    replacements = [(old_root + '/tail/', str(FIXTURE_ROOT / CASE) + '/'),
                    (old_root + '/tail-', str(FIXTURE_ROOT / (CASE + '-'))),
                    (old_root, str(FIXTURE_ROOT)),
                    (old_worker, str(E.WORKER_ROOT)), (old_parent, str(E.PARENT_ROOT))]
    bodies = {name: body for name, body in originals.items() if name.startswith('baseline/')}
    directory = Path(__file__).resolve().parent
    for name in E.ROOT_PINS:
        if name.endswith('.py'):
            bodies[name] = (directory / name).read_bytes()
    bodies['retention_tool.py'] = Path(E.__file__).resolve().read_bytes()
    for name, body in originals.items():
        if not name.startswith('tail/') or name in ('tail/complete.json', 'tail/pair.json'):
            continue
        target = CASE + name[len('tail'):]
        if name.endswith('.json') or name == 'tail/parent/stdout':
            value = rewrite(json.loads(body), replacements)
            if name.endswith('/command.json'):
                value['cwd'] = str(FIXTURE_ROOT.parent)
            body = encoded(value)
        bodies[target] = body
    terminal = rewrite(original_terminal, replacements)
    terminal.update(schema='ferric-guarded-mlp-readiness40-tail-currentness-duration-gpu-v1',
        case=CASE, instrumented=True, currentness_duration_diagnostic_requested=True,
        bank_guarded_body_includes_callbacks=True, matched_speed_comparison=False)
    terminal.pop('prior_census'); terminal.pop('matched_pair')
    terminal['checker_cpu'] = copy.deepcopy(E.CHECKER)
    admission = copy.deepcopy(terminal['admission'])
    for side in ('worker', 'parent'):
        old_path = str(Path(old_plan[side + '_cpu']['path']).parent / 'sources-after.json')
        admission[side + '_sources'] = rewrite(original_terminal['readset'][old_path], replacements)
    terminal['admission'] = admission
    plan = rewrite(old_plan, replacements)
    plan.update(schema='ferric-guarded-mlp-readiness40-tail-currentness-duration-gpu-input-v1',
                case=CASE, worker_sources=admission['worker_sources'], parent_sources=admission['parent_sources'])
    request = rewrite(json.loads(originals['tail-request.json']), replacements)
    bodies[CASE + '-request.json'] = encoded(request)
    plan['request'] = file_pin(FIXTURE_ROOT / (CASE + '-request.json'), bodies[CASE + '-request.json'])
    bodies[CASE + '-input.json'] = encoded(plan)
    terminal['plan'] = file_pin(FIXTURE_ROOT / (CASE + '-input.json'), bodies[CASE + '-input.json'])
    terminal['controller'] = file_pin(FIXTURE_ROOT / 'run_model_gpu.py', bodies['run_model_gpu.py'])
    prepared = rewrite(json.loads(originals['prepared-inputs.json']), replacements)
    prepared.update(schema='ferric-guarded-mlp-readiness40-tail-currentness-duration-input-preparation-v1',
        modes=[CASE], instrumented=True, matched_speed_comparison=False,
        bank_guarded_body_includes_callbacks=True, scoped_tail_requested=True,
        currentness_policies={CASE: 'bank_scoped_census_tail_duration'},
        requests={CASE: plan['request']}, plans={CASE: terminal['plan']},
        sessions=[request['base']['session']],
        controller=file_pin(FIXTURE_ROOT / 'prepare_model_inputs.py', bodies['prepare_model_inputs.py']))
    prepared.pop('identical_parent_and_worker_products')
    prepared.pop('scoped_tail_requested_only_for_candidate')
    bodies['prepared-inputs.json'] = encoded(prepared)
    root_pins = {name: identity(bodies[name]) for name in E.ROOT_PINS}
    terminal['readset'] = {path: row for path, row in terminal['readset'].items()
                           if not path.startswith(str(FIXTURE_ROOT) + '/')}
    for name in root_pins:
        if ((name.endswith('.py') and name != 'prepare_model_inputs.py')
                or name in (CASE + '-input.json', CASE + '-request.json')):
            row = file_pin(FIXTURE_ROOT / name, bodies[name])
            terminal['readset'][row['path']] = row
    for row in [*admission.values(), E.CHECKER]:
        terminal['readset'][row['path']] = row
    fixture = dict(bodies=bodies, terminal=terminal, root_pins=root_pins,
                   admission={name: E.compact(row) for name, row in admission.items()})
    original_policy = originals['tail/native/child-stderr.bin']
    policy = json.loads(original_policy)
    update_native(fixture, original_policy + encoded(synthetic_diagnostic(policy, original_policy)) + b'\n')
    with bindings(fixture):
        *_, diagnostic = E.validation_modules(bodies)
        summary_raw = bodies[CASE + '/native/complete.json']
        summary = json.loads(summary_raw)
        read_body = lambda row: E.native_body(bodies, row)
        checked = diagnostic.validate(CASE, bodies[CASE + '/parent/stdout'], summary_raw,
            request, FIXTURE_ROOT / CASE / 'native', read_body,
            summary['bootstrap']['sequence']['prompt_tokens'])
        baseline_raw, baseline_checked = E.verify_baseline(bodies, None)
        parity = diagnostic.compare_same_side(summary_raw, baseline_raw, checked, baseline_checked, read_body)
    terminal.update(observation=checked['ordinary'], matched_timing=checked, same_side_parity=parity)
    for name, value in (('observation.json', checked['ordinary']), ('matched.json', checked), ('parity.json', parity)):
        bodies[CASE + '/' + name] = encoded(value)
    reconcile(fixture)
    return fixture


def write_archive(path, fixture, observation, extra=None):
    bodies = dict(fixture['bodies'])
    manifest = dict(schema='ferric-guarded-mlp-readiness40-tail-currentness-duration-retention-v1',
        source_root=str(FIXTURE_ROOT), files={name: identity(body) for name, body in sorted(bodies.items())},
        observation=observation, outcomes=fixture['outcomes'])
    bodies['manifest.json'] = encoded(manifest)
    with tarfile.open(path, 'w:gz', format=tarfile.USTAR_FORMAT) as tar:
        for name, body in sorted(bodies.items()):
            row = tarfile.TarInfo(name); row.size = len(body); row.mode = 0o600
            tar.addfile(row, io.BytesIO(body))
        if extra is not None:
            row = tarfile.TarInfo(extra); row.size = 0
            tar.addfile(row, io.BytesIO(b''))
    return identity(path.read_bytes())['sha256'], bodies


class RetentionWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = make_fixture()
        with bindings(cls.fixture):
            admitted = E.verify(cls.fixture['bodies'], cls.fixture['outcomes'])
        if admitted['passed'] is not True:
            raise AssertionError('mutation baseline must pass every real retention predicate')

    def fresh(self):
        return copy.deepcopy(self.fixture)

    def verify(self, fixture):
        with bindings(fixture):
            return E.verify(fixture['bodies'], fixture['outcomes'])

    def roundtrip(self, fixture):
        observation = self.verify(fixture)
        with tempfile.TemporaryDirectory(prefix='synthetic-retention-only-') as directory:
            archive = Path(directory) / 'fixture.tar.gz'
            destination = Path(directory) / 'retained-fixture'
            sha, expected = write_archive(archive, fixture, observation)
            with bindings(fixture):
                retained = E.retain(archive, sha, destination)
            self.assertEqual(retained['members'], len(expected))
            self.assertEqual({p.relative_to(destination).as_posix(): p.read_bytes()
                              for p in destination.rglob('*') if p.is_file()}, expected)
            self.assertEqual(retained['cases'], observation['cases'])
            self.assertEqual(retained['passed'], observation['passed'])
            self.assertFalse(retained['native_rerun'] or retained['gpu_execution']
                             or retained['numerical_acceptance'] or retained['performance_claim'])
        return observation

    def test_complete_success_verify_and_retain_original_byte_closure(self):
        fixture = self.fresh()
        before = copy.deepcopy(fixture['bodies'])
        got = self.roundtrip(fixture)
        self.assertEqual(before, fixture['bodies'])
        self.assertTrue(got['passed'] and got['original_owned_lineage_revalidated']
                        and got['semantic_and_payload_parity_revalidated'])
        self.assertEqual((got['selected_original_bodies'], got['raw_files']), (166, 73))
        self.assertEqual(got['cases'][CASE]['matched_timing']['timing']['disjoint_spans'], 124)
        self.assertEqual(got['cases'][CASE]['readiness_completed_forwards'], 40)
        self.assertNotIn('matched_pair', got)

    def test_failed_prefix_verify_and_retain_without_success_promotion(self):
        fixture = self.fresh(); terminal = fixture['terminal']
        labels = {'parent-readelf', 'after-0', 'after-1', 'after-2'}
        terminal['phases'] = [row for row in terminal['phases'] if row['label'] in labels]
        terminal['phases'][0]['exit_code'] = 101
        terminal.update(passed=False, errors=['synthetic fixture audit failure'], native_attempts=0,
            observation=None, matched_timing=None, same_side_parity=None, owned_worker_lineage_verified=False,
            native_spawn_observed=False, gpu_execution_requested=False, gpu_execution=False,
            gpu_execution_confirmed=False, partial_gpu_execution_possible=False,
            model_source_authenticated_by_qualified_parent=False)
        retained = {'initial-topology.json'} | {name + '-topology.json' for name in labels if name.startswith('after-')}
        retained |= {label + '/' + name for label in labels
                     for name in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
        fixture['bodies'] = {name: body for name, body in fixture['bodies'].items()
                             if not name.startswith(CASE + '/') or name[len(CASE) + 1:] in retained}
        reconcile(fixture)
        got = self.roundtrip(fixture)
        self.assertFalse(got['passed'] or got['original_owned_lineage_revalidated'])
        self.assertEqual(got['raw_files'], 24)
        self.assertEqual(got['cases'][CASE]['original_terminal']['name'], 'failed.json')
        self.assertIsNone(got['cases'][CASE]['matched_timing'])
        self.assertEqual(json.loads(fixture['bodies'][CASE + '/failed.json'])['errors'],
                         ['synthetic fixture audit failure'])

    def test_late_failed_case_preserves_completed_raw_without_relabelling(self):
        fixture = self.fresh()
        fixture['terminal'].update(passed=False, errors=['synthetic fixture posthash failure'])
        reconcile(fixture)
        got = self.roundtrip(fixture)
        self.assertFalse(got['passed'] or got['cases'][CASE]['retained_success_revalidated'])
        self.assertEqual(got['raw_files'], 73)
        self.assertEqual(got['cases'][CASE]['original_terminal']['name'], 'failed.json')

    def test_absent_case_verify_and_retain_without_invented_terminal(self):
        fixture = self.fresh()
        fixture['bodies'] = {name: body for name, body in fixture['bodies'].items()
                             if not name.startswith(CASE + '/')}
        fixture['outcomes'] = {CASE: None}
        got = self.roundtrip(fixture)
        self.assertFalse(got['passed'] or got['cases'][CASE]['present'])
        self.assertIsNone(got['cases'][CASE]['original_terminal'])
        self.assertEqual((got['selected_original_bodies'], got['raw_files']), (92, 0))

    def test_original_stderr_drift_fails_even_with_outer_raw_pin_refreshed(self):
        fixture = self.fresh()
        fixture['bodies'][CASE + '/native/child-stderr.bin'] += b'\n'
        reconcile(fixture)
        with self.assertRaises((ValueError, RuntimeError)):
            self.verify(fixture)

    def test_wrong_policy_sha_and_relabelled_policy_refused_after_full_repin(self):
        for field in ('policy_sha256', 'worker_sha256', 'performance_claim'):
            fixture = self.fresh()
            first, second = fixture['bodies'][CASE + '/native/child-stderr.bin'].splitlines()
            diagnostic = json.loads(second)
            if field == 'performance_claim':
                diagnostic[field] = True
            else:
                diagnostic[field][0] ^= 1
            update_native(fixture, first + b'\n' + encoded(diagnostic) + b'\n')
            reconcile(fixture)
            with self.subTest(field=field), self.assertRaises((ValueError, RuntimeError)):
                self.verify(fixture)

    def test_changed_first_policy_cannot_hide_behind_consistent_second_sha(self):
        fixture = self.fresh()
        first, second = fixture['bodies'][CASE + '/native/child-stderr.bin'].splitlines()
        policy = json.loads(first); policy['native_closed'] = False
        changed = encoded(policy) + b'\n'
        diagnostic = json.loads(second); diagnostic['policy_sha256'] = list(hashlib.sha256(changed).digest())
        update_native(fixture, changed + encoded(diagnostic) + b'\n')
        reconcile(fixture)
        with self.assertRaises((ValueError, RuntimeError)):
            self.verify(fixture)

    def test_capture_and_transcript_original_body_drift_are_refused(self):
        for name in ('capture-5.bin', 'frames.ndjson'):
            fixture = self.fresh()
            fixture['bodies'][CASE + '/native/' + name] += b'\0'
            reconcile(fixture)
            with self.subTest(name=name), self.assertRaises((ValueError, RuntimeError)):
                self.verify(fixture)

    def test_healthy_close_and_owned_worker_ancestry_are_not_metadata_only(self):
        fixture = self.fresh()
        update_native(fixture, edit_summary=lambda value: value.update(native_closed=False))
        reconcile(fixture)
        with self.assertRaises((ValueError, RuntimeError)):
            self.verify(fixture)
        for field in ('ppid', 'sid', 'pgid'):
            fixture = self.fresh()
            native = next(row for row in fixture['terminal']['phases'] if row['label'] == 'parent')
            native['lineage'][1]['identity'][field] += 1
            reconcile(fixture)
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.verify(fixture)

    def test_source_map_binding_and_readset_custody_cannot_be_substituted(self):
        for side in ('worker', 'parent'):
            fixture = self.fresh(); plan = json.loads(fixture['bodies'][CASE + '-input.json'])
            key = side + '_sources'
            plan[key]['sha256'] = '0' * 64
            raw = encoded(plan); name = CASE + '-input.json'
            fixture['bodies'][name] = raw; fixture['root_pins'][name] = identity(raw)
            fixture['terminal']['plan'] = file_pin(FIXTURE_ROOT / name, raw)
            prepared = json.loads(fixture['bodies']['prepared-inputs.json'])
            prepared['plans'][CASE] = fixture['terminal']['plan']
            fixture['bodies']['prepared-inputs.json'] = encoded(prepared)
            fixture['root_pins']['prepared-inputs.json'] = identity(fixture['bodies']['prepared-inputs.json'])
            fixture['terminal']['readset'][str(FIXTURE_ROOT / name)] = fixture['terminal']['plan']
            reconcile(fixture)
            with self.subTest(side=side, mutation='plan'), self.assertRaises(RuntimeError):
                self.verify(fixture)
            fixture = self.fresh()
            row = fixture['terminal']['admission'][key]
            fixture['terminal']['readset'][row['path']] = dict(row, sha256='0' * 64)
            reconcile(fixture)
            with self.subTest(side=side, mutation='readset'), self.assertRaises(RuntimeError):
                self.verify(fixture)

    def test_missing_extra_and_wrong_observed_outcomes_are_refused(self):
        for mutation in ('missing', 'extra', 'terminal_sha', 'absent', 'failed_name'):
            fixture = self.fresh()
            if mutation == 'missing': fixture['bodies'].pop(CASE + '/native/capture-0.bin')
            elif mutation == 'extra': fixture['bodies'][CASE + '/unexpected.txt'] = b'fixture only'
            elif mutation == 'terminal_sha': fixture['outcomes'][CASE]['sha256'] = '0' * 64
            elif mutation == 'absent': fixture['outcomes'][CASE] = None
            else: fixture['outcomes'][CASE]['name'] = 'failed.json'
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                self.verify(fixture)

    def test_retain_rejects_wrong_archive_hash_duplicate_escape_and_manifest_observation(self):
        fixture = self.fresh(); observation = self.verify(fixture)
        with tempfile.TemporaryDirectory(prefix='synthetic-retention-refusal-') as directory:
            root = Path(directory)
            for mutation in ('sha', 'duplicate', 'escape', 'observation'):
                archive = root / (mutation + '.tar.gz')
                changed = copy.deepcopy(observation)
                if mutation == 'observation': changed['passed'] = False
                extra = 'prepared-inputs.json' if mutation == 'duplicate' else '../outside' if mutation == 'escape' else None
                sha, _ = write_archive(archive, fixture, changed, extra)
                if mutation == 'sha': sha = '0' * 64
                destination = root / (mutation + '-retained')
                with self.subTest(mutation=mutation), bindings(fixture), self.assertRaises(RuntimeError):
                    E.retain(archive, sha, destination)
                self.assertFalse(destination.exists())

    def test_success_retention_refuses_an_existing_destination(self):
        fixture = self.fresh(); observation = self.verify(fixture)
        with tempfile.TemporaryDirectory(prefix='synthetic-retention-existing-') as directory:
            root = Path(directory); archive = root / 'fixture.tar.gz'; destination = root / 'existing'
            sha, _ = write_archive(archive, fixture, observation)
            destination.mkdir(); sentinel = destination / 'sentinel'; sentinel.write_bytes(b'unchanged fixture')
            with bindings(fixture), self.assertRaises(RuntimeError):
                E.retain(archive, sha, destination)
            self.assertEqual(sentinel.read_bytes(), b'unchanged fixture')


if __name__ == '__main__':
    unittest.main()
