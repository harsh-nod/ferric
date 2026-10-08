"""Synthetic outer admission only; no child processes, native code or model access."""
import copy
from contextlib import ExitStack, redirect_stdout
import hashlib
import io
import json
from pathlib import Path
import struct
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import prepare_model_inputs as G
import retention_tool as E
import run_model_gpu as R


FEATURE = 'engineering-currentness-duration-diagnostics'
PARENT_FEATURES = ['guarded-mlp-readiness-engineering', 'guarded-mlp-full2303-engineering', FEATURE]
PARENT_ARTIFACT_FEATURES = sorted(PARENT_FEATURES + ['guarded-mlp-model-engineering',
    'tp-batch-engineering', 'tp-engineering'])
PARENT_NAME = 'ferric-qwen3-guarded-mlp-readiness-engineering'


def fixture():
    receipt = dict(path='/synthetic/worker/evidence/complete.json', bytes=17, sha256='a' * 64)
    sources = dict(path='/synthetic/worker/evidence/sources-after.json', bytes=19, sha256='b' * 64)
    compact = lambda row: {key: row[key] for key in ('bytes', 'sha256')}
    lineage = {'worker-proposal.json': dict(path='/synthetic/worker-proposal.json', **R.DURATION_WORKER_SOURCE),
               'runtime-proposal.json': dict(path='/synthetic/runtime-proposal.json', **R.DURATION_RUNTIME_SOURCE)}
    common = dict(qualification_mode='diagnostic', currentness_duration_diagnostic_build=True,
                  currentness_duration_native_execution=False, readset=lineage)
    worker = dict(copy.deepcopy(common), source_generation='currentness-duration-diagnostic-coupled-v2',
        selected_runtime_features=sorted(['default', 'engineering-gfx950', FEATURE]),
        selected_worker_features=[FEATURE], default_check_no_default_features=True,
        default_check_diagnostic_feature_requested=True, currentness_duration_runtime_source_added=True,
        currentness_duration_worker_source_added=True, runtime_source_changed=True,
        artifacts={'worker': {'cargo_artifact': {'features': [FEATURE]}}})
    parent = dict(copy.deepcopy(common), source_generation='currentness-duration-diagnostic-parent-v2',
        currentness_duration_parent_source_added=True, selected_parent_features=list(PARENT_FEATURES),
        artifacts={PARENT_NAME: {'cargo_artifact': {'features': list(PARENT_ARTIFACT_FEATURES)}}},
        worker_qualification=compact(receipt), worker_source_manifest=compact(sources))
    return worker, parent, receipt, sources


class AdmissionTests(unittest.TestCase):
    def setUp(self):
        self.args = fixture()
        for module in (R, G):
            module.diagnostic_cpu_contract(*self.args)

    def refused(self, args):
        for module in (R, G):
            with self.subTest(module=module.__name__), self.assertRaises(RuntimeError):
                module.diagnostic_cpu_contract(*args)

    def test_matching_diagnostic_extension_is_pure_and_shared(self):
        before = copy.deepcopy(self.args)
        self.assertIsNone(R.diagnostic_cpu_contract(*self.args))
        self.assertIsNone(G.diagnostic_cpu_contract(*self.args))
        self.assertEqual(before, self.args)
        self.assertEqual(R.DURATION_WORKER_SOURCE, G.DURATION_WORKER_SOURCE)
        self.assertEqual(R.DURATION_RUNTIME_SOURCE, G.DURATION_RUNTIME_SOURCE)

    def test_default_or_mixed_modes_are_not_diagnostic_admission(self):
        for side in (0, 1):
            for mode in ('default', 'tail', 'tail_duration', True):
                args = copy.deepcopy(self.args)
                args[side]['qualification_mode'] = mode
                self.refused(args)

    def test_exact_repaired_generations_are_required(self):
        for side in (0, 1):
            args = copy.deepcopy(self.args)
            args[side]['source_generation'] = args[side]['source_generation'].replace('-v2', '-v1')
            self.refused(args)

    def test_both_diagnostic_build_and_no_prior_native_authority_are_required(self):
        for side in (0, 1):
            for field, value in [('currentness_duration_diagnostic_build', False),
                                 ('currentness_duration_native_execution', True),
                                 ('currentness_duration_diagnostic_build', 1)]:
                args = copy.deepcopy(self.args)
                args[side][field] = value
                self.refused(args)

    def test_selected_runtime_feature_vector_is_exact(self):
        for features in ([], ['default', 'engineering-gfx950'], [FEATURE],
                         sorted(['default', 'engineering-gfx950', FEATURE, 'extra'])):
            args = copy.deepcopy(self.args)
            args[0]['selected_runtime_features'] = features
            self.refused(args)

    def test_worker_feature_request_and_cargo_product_must_agree(self):
        for field in ('selected_worker_features', 'artifact'):
            args = copy.deepcopy(self.args)
            if field == 'artifact':
                args[0]['artifacts']['worker']['cargo_artifact']['features'] = []
            else:
                args[0][field] = []
            self.refused(args)

    def test_worker_mode_source_and_feature_flags_are_exact_booleans(self):
        for field in ('default_check_no_default_features', 'default_check_diagnostic_feature_requested',
                      'currentness_duration_runtime_source_added', 'currentness_duration_worker_source_added',
                      'runtime_source_changed'):
            for value in (False, 1):
                args = copy.deepcopy(self.args)
                args[0][field] = value
                self.refused(args)

    def test_parent_feature_selection_and_artifact_are_both_bound(self):
        for field, features in (
                ('selected_parent_features', PARENT_FEATURES[:2]),
                ('artifact', sorted(f for f in PARENT_ARTIFACT_FEATURES if f != FEATURE)),
                ('artifact', sorted(PARENT_FEATURES)),
                ('artifact', sorted(PARENT_ARTIFACT_FEATURES + ['extra']))):
            args = copy.deepcopy(self.args)
            if field == 'artifact':
                args[1]['artifacts'][PARENT_NAME]['cargo_artifact']['features'] = features
            else:
                args[1][field] = features
            self.refused(args)
        args = copy.deepcopy(self.args)
        args[1]['currentness_duration_parent_source_added'] = False
        self.refused(args)

    def test_original_runtime_and_repaired_worker_lineage_cannot_be_substituted(self):
        for side in (0, 1):
            for name in ('worker-proposal.json', 'runtime-proposal.json'):
                for key, value in [('sha256', '0' * 64), ('bytes', 0)]:
                    args = copy.deepcopy(self.args)
                    args[side]['readset'][name][key] = value
                    self.refused(args)

    def test_parent_joins_exact_worker_receipt_and_final_source_map(self):
        for field in ('worker_qualification', 'worker_source_manifest'):
            for key, value in [('sha256', '0' * 64), ('bytes', 1)]:
                args = copy.deepcopy(self.args)
                args[1][field][key] = value
                self.refused(args)

    def test_outer_runner_refuses_pending_cpu_and_product_bindings_before_reads(self):
        class NoRead:
            def read(self, *args, **kwargs):
                raise AssertionError('pending bindings must refuse before external reads')
        with patch.multiple(R, WORKER=None, WORKER_CPU=None, WORKER_SOURCES=None,
                            PARENT=None, PARENT_CPU=None, PARENT_SOURCES=None):
            with self.assertRaises(RuntimeError):
                R.cpu_admission({}, NoRead())

    def test_only_explicit_diagnostic_case_is_selected_before_io(self):
        self.assertEqual(R.ORDER, ('tail_duration',))
        self.assertEqual(G.ORDER, R.ORDER)
        self.assertEqual(E.ORDER, R.ORDER)
        for case in ('tail', 'census', 'default', 'full2303', '', True):
            with self.subTest(case=case), self.assertRaises(RuntimeError):
                R._run_case(case, 0, 1)

    def test_retainer_refuses_unbound_sources_instead_of_inventing_success(self):
        with self.assertRaises(RuntimeError):
            E.verify({}, {'tail_duration': None})
        with self.assertRaises(RuntimeError):
            E.verify({}, {'census': None, 'tail': None})

    def test_single_case_raw_roster_and_all_inherited_bounds_remain_fixed(self):
        self.assertEqual(len(E.matched_names('tail_duration')), 73)
        self.assertNotIn('pair.json', E.matched_names('tail_duration'))
        self.assertIn('native/child-stderr.bin', E.matched_names('tail_duration'))
        self.assertIn('native/host-timing.json', E.matched_names('tail_duration'))
        self.assertEqual(len(E.LABELS), 11)
        self.assertEqual((R.NATIVE_SECONDS, R.CASE_SECONDS, R.AUDIT_SECONDS), (4000, 4300, 30))
        self.assertEqual((R.STREAM_CAP, R.CASE_CAP), (8 << 20, 64 << 20))
        self.assertEqual(R.IDS, [16366993098680759275, 10838076764495710945])
        self.assertEqual(R.CHECKER_TESTS, 126)


def wire(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n').encode()


def body_pin(path, raw):
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact_pin(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


class SyntheticQualification:
    """Closed synthetic receipt bodies, never an observed qualification result."""
    def __init__(self, worker_root=R.WORKER_ROOT, parent_root=R.PARENT_ROOT):
        self.worker_root, self.parent_root = Path(worker_root), Path(parent_root)
        self.bodies = {}
        self.worker, self.parent, _, _ = fixture()
        true_worker = '''scoped_currentness_runtime_source_added scoped_currentness_worker_source_added
            explicit_scoped_readiness_selector_source_added inherited_terminal_pair_runtime_preserved
            legacy_terminal_dispatch_preserved fresh_terminal_opt_in_refused inherited_peer_read_pair_runtime_preserved
            scoped_bank_rearm_runtime_source_added readiness40_bank_scoped_warm_source_added
            inherited_scoped_currentness_runtime_preserved shared_test_fixture_repaired full_model_long_request_enabled
            full2303_scoped_warm_source_added inherited_bank_scoped_currentness_preserved
            scoped_capacity_census_runtime_source_added readiness40_bank_scoped_census_source_added
            allocation_preflights_changed inherited_readiness40_bank_scoped_census_preserved
            full2303_bank_scoped_census_source_added scoped_tail_runtime_source_added
            readiness40_bank_scoped_census_tail_source_added cli_executable_unchanged_across_tests
            full_runtime_tests_executed full_worker_tests_executed selected_facade_doctests_executed'''.split()
        false_worker = '''scoped_currentness_native_execution currentness_temporal_equivalence_claim
            global_currentness_policy_changed arena_policy_changed default_fresh_arena_policy_changed
            ordinary_read_api_changed public_runtime_limits_changed lockfiles_changed shared_cache_changed
            readiness40_bank_scoped_warm_native_execution readiness40_bank_scoped_census_native_execution
            full2303_scoped_warm_native_execution full2303_bank_scoped_census_native_execution
            readiness40_bank_scoped_census_tail_native_execution full2303_deadline_changed full2303_launch_admitted
            gpu_qualified numerical_acceptance performance_claim production_authority all_crate_doctests_executed'''.split()
        true_parent = '''shared_test_fixture_repaired all_selected_parent_tests_executed
            position5_diagnostic_parent_route_added inherited_readiness_parent_route_preserved
            readiness40_bank_scoped_warm_parent_source_added allocation_preflights_changed
            allocation_preflights_changed_only_for_explicit_warm_position5_or_full
            full2303_bank_scoped_census_parent_source_added full2303_bank_scoped_census_policy_prepublication_checked
            readiness40_bank_scoped_census_tail_parent_source_added
            readiness40_bank_scoped_census_tail_policy_prepublication_checked
            readiness40_bank_scoped_census_parent_source_added full2303_scoped_warm_parent_source_added
            readiness40_scoped_warm_parent_source_added scoped_warm_host_timing_parent_source_added
            qualified_worker_sources_preserved shared_full_host_timing_parent_source_added
            parent_host_timing_source_added'''.split()
        false_parent = '''full_parent_library_suite_executed position5_native_execution readiness_native_execution
            full_long_workload readiness40_bank_scoped_warm_native_execution
            allocation_preflights_changed_only_for_explicit_warm_position5 full2303_bank_scoped_census_native_execution
            full2303_deadline_changed readiness40_bank_scoped_census_tail_native_execution
            readiness40_bank_scoped_census_native_execution full2303_scoped_warm_native_execution
            full2303_launch_admitted readiness40_scoped_warm_native_execution currentness_temporal_equivalence_claim
            ordinary_wire_schema_changed ordinary_observation_schema_changed shared_full_host_timing_native_execution
            parent_host_timing_native_execution default_policy_changed global_currentness_policy_changed'''.split()
        for value, yes, no in ((self.worker, true_worker, false_worker), (self.parent, true_parent, false_parent)):
            value.update({key: True for key in yes})
            value.update({key: False for key in no})
        prefix = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
        worker_names = [prefix + 'src/fixture_%03d.rs' % i for i in range(236)]
        self.worker['final_sources'] = self.source_map(self.worker_root,
            worker_names + ['fe2o3/fixture_%03d.rs' % i for i in range(829)]
            + ['helper_%d.py' % i for i in range(4)])
        self.parent['final_sources'] = self.source_map(self.parent_root,
            worker_names + ['ferric/parent/fixture_%04d.rs' % i for i in range(1066)])
        for value, root, schema, phases in (
                (self.worker, self.worker_root, 'ferric-guarded-mlp-currentness-duration-cpu-v1', 27),
                (self.parent, self.parent_root, 'ferric-guarded-mlp-currentness-duration-parent-cpu-v1', 69)):
            value.update(schema=schema, passed=True, failure=None, postcheck_errors=[], source_unchanged=True,
                gpu_execution=False, input_sources=copy.deepcopy(value['final_sources']),
                controller=self.put(root / 'run_cpu.py', b'# synthetic qualified controller\n'),
                input_manifest=self.put(root / 'input-manifest.json', b'{"synthetic":true}\n'),
                phases=[dict(label='synthetic-phase-%02d' % i, exit_code=0, natural_exit=True,
                    reaped=True, process_group_absent=True, forced_cleanup=False, timed_out=False,
                    exception=None, storage_failure=None) for i in range(phases)])
        elf = b'\x7fELF\x02\x01' + bytes(12) + b'\x3e\x00' + bytes(44)
        roles = ['kfd-lib', 'engineering-worker-test', 'guarded-facade-test', 'debug-trap-test',
                 'telemetry-env-test', 'telemetry-test', 'worker-lib', 'worker-bin-test',
                 'worker-readiness-test', 'worker-wire-test', 'worker']
        self.worker['artifacts'] = {name: dict(pin=self.put(self.worker_root / 'target' / name, elf),
            cargo_artifact=dict(features=[FEATURE])) for name in roles}
        self.worker['cli_executable_before_tests'] = copy.deepcopy(self.worker['artifacts']['worker'])
        self.parent['artifacts'] = {name: dict(pin=self.put(self.parent_root / 'target' / name, elf),
            cargo_artifact=dict(features=list(PARENT_ARTIFACT_FEATURES), target=dict(name=name, kind=['bin']),
                profile=dict(test=False), executable=str(self.parent_root / 'target' / name)))
            for name in [PARENT_NAME] + ['synthetic-parent-product-%d' % i for i in range(6)]}
        self.worker['inventories'] = dict(runtime=['runtime::synthetic_%04d' % i for i in range(1200)],
                                          worker=['worker::synthetic_%04d' % i for i in range(857)])
        self.worker['tests'] = {'kfd-tests': dict(passed=1192, failed=0, ignored=8),
            'worker-tests': dict(passed=853, failed=0, ignored=4),
            'interface-doc-tests': dict(passed=10, failed=0, ignored=0)}
        self.worker['doc_parser_tests'] = dict(passed=8, failed=0, project_execution=False, gpu_execution=False)
        self.parent['inventory'] = ['parent::synthetic_%04d' % i for i in range(1058)]
        self.parent['tests'] = {'synthetic-scope-%02d' % i: dict(passed=15 if i == 58 else 10,
            failed=0, ignored=0) for i in range(59)}
        self.parent.update(parent_host_timing_rows=40, parent_host_timing_disjoint_spans=124)
        self.seal()

    @staticmethod
    def source_map(root, names):
        return {name: body_pin(root / name, ('// synthetic ' + name + '\n').encode()) for name in names}

    def put(self, path, body):
        self.bodies[str(path)] = body
        return body_pin(path, body)

    def seal(self, link_worker=True):
        for name, value, root in (('worker', self.worker, self.worker_root), ('parent', self.parent, self.parent_root)):
            source_pin = self.put(root / 'evidence/sources-after.json', wire(value['final_sources']))
            value['raw'] = {'sources-after.json': source_pin}
        self.worker_pin = self.put(self.worker_root / 'evidence/complete.json', wire(self.worker))
        if link_worker:
            self.parent['worker_qualification'] = compact_pin(self.worker_pin)
            self.parent['worker_source_manifest'] = compact_pin(self.worker['raw']['sources-after.json'])
        self.parent_pin = self.put(self.parent_root / 'evidence/complete.json', wire(self.parent))
        self.plan = dict(worker_cpu=self.worker_pin, parent_cpu=self.parent_pin,
            worker=self.worker['artifacts']['worker']['pin'], parent=self.parent['artifacts'][PARENT_NAME]['pin'],
            worker_sources=self.worker['raw']['sources-after.json'], parent_sources=self.parent['raw']['sources-after.json'])

    def bindings(self):
        return {name: compact_pin(self.plan[key]) for name, key in (
            ('WORKER', 'worker'), ('WORKER_CPU', 'worker_cpu'), ('WORKER_SOURCES', 'worker_sources'),
            ('PARENT', 'parent'), ('PARENT_CPU', 'parent_cpu'), ('PARENT_SOURCES', 'parent_sources'))}

    def materialize(self):
        for path, raw in self.bodies.items():
            target = Path(path)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)


class PinnedAudit:
    """The I/O boundary reads exact fixture bytes; admission itself is not mocked."""
    def __init__(self, bodies):
        self.bodies, self.reads = bodies, []

    @staticmethod
    def parse(raw):
        return G.parse(raw)

    def read(self, path, expected=None, limit=64 << 20, retain=True):
        path = str(path)
        raw = self.bodies[path]
        pin = body_pin(path, raw)
        if len(raw) > limit or (expected is not None and pin != expected):
            raise RuntimeError('synthetic original body/pin mismatch')
        self.reads.append(path)
        return (raw if retain else b''), pin


class CpuAdmissionWorkflowTests(unittest.TestCase):
    def admit(self, value, plan=None, bindings=None):
        audit = PinnedAudit(value.bodies)
        with patch.multiple(R, WORKER_ROOT=value.worker_root, PARENT_ROOT=value.parent_root,
                            **(bindings if bindings is not None else value.bindings())):
            result = R.cpu_admission(value.plan if plan is None else plan, audit)
        return result, audit

    def test_fully_bound_success_reads_both_receipts_maps_controllers_inputs_and_elfs(self):
        value = SyntheticQualification()
        result, audit = self.admit(value)
        self.assertEqual(result, value.plan)
        expected = {row['path'] for row in value.plan.values()}
        expected |= {v[k]['path'] for v in (value.worker, value.parent) for k in ('controller', 'input_manifest')}
        self.assertEqual(set(audit.reads), expected)
        self.assertEqual((len(value.worker['final_sources']), len(value.parent['final_sources'])), (1069, 1302))

    def test_cross_mode_generation_and_lineage_are_rejected_after_hash_admission(self):
        for side in ('worker', 'parent'):
            for field, wrong in [('qualification_mode', 'default'), ('source_generation', 'obsolete-v1')]:
                value = SyntheticQualification()
                getattr(value, side)[field] = wrong
                value.seal()
                with self.subTest(side=side, field=field), self.assertRaises(RuntimeError):
                    self.admit(value)
            for name in ('worker-proposal.json', 'runtime-proposal.json'):
                value = SyntheticQualification()
                getattr(value, side)['readset'][name]['sha256'] = '0' * 64
                value.seal()
                with self.subTest(side=side, lineage=name), self.assertRaisesRegex(RuntimeError, 'source lineage'):
                    self.admit(value)
        for field in ('worker_qualification', 'worker_source_manifest'):
            value = SyntheticQualification()
            value.parent[field]['sha256'] = '0' * 64
            value.seal(link_worker=False)
            with self.subTest(join=field), self.assertRaisesRegex(RuntimeError, 'exact qualified worker sources'):
                self.admit(value)

    def test_every_bound_product_receipt_and_map_pin_is_checked(self):
        for key in ('worker', 'worker_cpu', 'worker_sources', 'parent', 'parent_cpu', 'parent_sources'):
            value = SyntheticQualification()
            plan = copy.deepcopy(value.plan)
            plan[key]['sha256'] = '0' * 64
            with self.subTest(key=key), self.assertRaisesRegex(RuntimeError, 'actual readiness CPU/ELF pin|body/pin mismatch'):
                self.admit(value, plan=plan)

    def test_wrong_cpu_root_and_controller_identity_are_rejected(self):
        value = SyntheticQualification()
        plan = copy.deepcopy(value.plan)
        plan['parent_cpu']['path'] = '/synthetic/wrong-root/complete.json'
        with self.assertRaisesRegex(RuntimeError, 'qualification roots'):
            self.admit(value, plan=plan)
        value.parent['controller']['path'] += '.wrong'
        value.seal()
        with self.assertRaisesRegex(RuntimeError, 'controller identity'):
            self.admit(value)

    def test_original_source_map_bytes_and_all236_worker_identities_are_required(self):
        value = SyntheticQualification()
        value.bodies[value.plan['parent_sources']['path']] += b' '
        with self.assertRaisesRegex(RuntimeError, 'body/pin mismatch'):
            self.admit(value)
        value = SyntheticQualification()
        key = next(k for k in value.parent['final_sources'] if '/tp-peer-finite-engineering-worker-v1/' in k)
        for field in ('input_sources', 'final_sources'):
            value.parent[field][key]['sha256'] = '0' * 64
        value.seal()
        with self.assertRaisesRegex(RuntimeError, 'all236'):
            self.admit(value)

    def test_source_unchanged_and_postcheck_status_cannot_be_forged(self):
        for side in ('worker', 'parent'):
            for field, wrong in [('source_unchanged', False), ('postcheck_errors', ['synthetic failure']),
                                 ('passed', False), ('failure', 'synthetic failure')]:
                value = SyntheticQualification()
                getattr(value, side)[field] = wrong
                value.seal()
                with self.subTest(side=side, field=field), self.assertRaisesRegex(RuntimeError, 'qualified readiness CPU terminal'):
                    self.admit(value)

    def test_all_cpu_phase_ownership_deadline_and_retirement_checks_are_active(self):
        for side in ('worker', 'parent'):
            for field, wrong in [('exit_code', True), ('exit_code', 1), ('natural_exit', False),
                                 ('reaped', False), ('process_group_absent', False), ('forced_cleanup', True),
                                 ('timed_out', True), ('exception', 'synthetic'), ('storage_failure', 'synthetic')]:
                value = SyntheticQualification()
                getattr(value, side)['phases'][-1][field] = wrong
                value.seal()
                with self.subTest(side=side, field=field), self.assertRaisesRegex(RuntimeError, 'CPU leaf lifecycle'):
                    self.admit(value)

    def test_named_inventory_selected_counts_and_source_extents_are_not_relaxed(self):
        for mutation in ('runtime', 'worker', 'parent', 'scope', 'sources'):
            value = SyntheticQualification()
            if mutation in ('runtime', 'worker'):
                value.worker['inventories'][mutation].pop()
            elif mutation == 'parent':
                value.parent['inventory'].pop()
            elif mutation == 'scope':
                value.parent['tests']['synthetic-scope-00']['passed'] += 1
            else:
                key = next(iter(value.parent['final_sources']))
                for field in ('input_sources', 'final_sources'):
                    del value.parent[field][key]
            value.seal()
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                self.admit(value)

    def test_product_features_cargo_target_and_pretest_worker_identity_are_checked(self):
        for mutation in ('worker_features', 'parent_features', 'parent_kind', 'parent_test', 'pretest'):
            value = SyntheticQualification()
            if mutation == 'worker_features':
                value.worker['artifacts']['worker']['cargo_artifact']['features'] = []
            elif mutation == 'parent_features':
                value.parent['artifacts'][PARENT_NAME]['cargo_artifact']['features'] = sorted(PARENT_FEATURES)
            elif mutation == 'parent_kind':
                value.parent['artifacts'][PARENT_NAME]['cargo_artifact']['target']['kind'] = ['lib']
            elif mutation == 'parent_test':
                value.parent['artifacts'][PARENT_NAME]['cargo_artifact']['profile']['test'] = True
            else:
                value.worker['cli_executable_before_tests']['pin']['sha256'] = '0' * 64
            value.seal()
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                self.admit(value)

    def test_rebound_non_elf_product_bytes_still_fail_elf_admission(self):
        for side, role in (('worker', 'worker'), ('parent', PARENT_NAME)):
            value = SyntheticQualification()
            pin = getattr(value, side)['artifacts'][role]['pin']
            replacement = value.put(pin['path'], b'not an ELF product')
            getattr(value, side)['artifacts'][role]['pin'] = replacement
            if side == 'worker':
                value.worker['cli_executable_before_tests']['pin'] = copy.deepcopy(replacement)
            value.seal()
            with self.subTest(side=side), self.assertRaisesRegex(RuntimeError, 'x86_64 ELF'):
                self.admit(value)


class PreparerWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='duration-admission-synthetic-')
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name).resolve()
        self.root = self.base / 'native'
        self.root.mkdir()
        self.value = SyntheticQualification(self.base / 'worker', self.base / 'parent')
        self.value.materialize()
        self.script = self.root / 'prepare_model_inputs.py'
        self.script.write_bytes(Path(G.__file__).read_bytes())
        self.model = self.base / 'synthetic-model-directory'
        self.model.mkdir()
        self.tokens = list(range(2048))
        prompt_bodies = dict(tokens=struct.pack('<2048I', *self.tokens),
            manifest=wire(dict(input_token_ids=self.tokens)), text=b'synthetic prompt, not model input\n')
        self.prompt = {name: self.asset('prompt-' + name, raw) for name, raw in prompt_bodies.items()}
        self.images = {name: self.asset(name, ('synthetic image ' + name + '\n').encode())
                       for name in ('prefix_image', 'tiles_image', 'projection_image', 'guarded_image')}
        ordinary_images = {name: G.rust(self.asset('ordinary-' + name, ('synthetic ' + name).encode()))
                           for name in ('prefix', 'mlp', 'residual', 'tail')}
        self.old = dict(schema='FerricFiniteGuardedMlpDecodeRequestV1',
            decode=dict(schema='synthetic historical schema', source=str(self.model),
                worker=G.rust(self.value.plan['worker']), images=ordinary_images,
                expected_bundle_id=[3] * 32, expected_model_id=[4] * 32, device_ids=list(R.IDS),
                session=[1] * 32, mode='teacher_forced', prompt={k: G.rust(v) for k, v in self.prompt.items()},
                dispatch_timeout_ms=10000, child_deadline_ms=3600000,
                prefix_image=G.rust(self.images['prefix_image']), tiles_image=G.rust(self.images['tiles_image'])),
            projection_image=G.rust(self.images['projection_image']), guarded_image=G.rust(self.images['guarded_image']))
        self.historical_path = self.base / 'historical-request.json'
        self.rewrite_historical()

    def asset(self, name, raw):
        path = self.base / name
        path.write_bytes(raw)
        return body_pin(path, raw)

    def rewrite_historical(self):
        raw = wire(self.old)
        self.historical_path.write_bytes(raw)
        self.historical_pin = body_pin(self.historical_path, raw)

    def execute(self, overrides=None, session=bytes([2] * 32)):
        value = self.value
        extent = lambda p: (p['bytes'], p['sha256'])
        bindings = dict(ROOT=self.root, WORKER_ROOT=value.worker_root, PARENT_ROOT=value.parent_root,
            WORKER_CPU=extent(value.plan['worker_cpu']), WORKER_ELF=extent(value.plan['worker']),
            WORKER_SOURCES=extent(value.plan['worker_sources']), PARENT_CPU=extent(value.plan['parent_cpu']),
            PARENT_ELF=extent(value.plan['parent']), PARENT_SOURCES=extent(value.plan['parent_sources']),
            OLD_REQUEST=extent(self.historical_pin), PROMPT={k: extent(v) for k, v in self.prompt.items()},
            IMAGES={k: extent(v) for k, v in self.images.items()}, INPUTS={}, DEADLINE=float('inf'),
            __file__=str(self.script))
        bindings.update(overrides or {})
        argv = [str(self.script), value.plan['parent_cpu']['path'], value.plan['parent_cpu']['sha256'],
                value.plan['parent']['path'], str(self.historical_path)]
        output = io.StringIO()
        with ExitStack() as stack:
            stack.enter_context(patch.multiple(G, **bindings))
            stack.enter_context(patch.object(G.sys, 'argv', argv))
            stack.enter_context(patch.object(G.sys, 'dont_write_bytecode', True))
            for name, result in (('getuid', 9661), ('geteuid', 9661),
                                 ('uname', SimpleNamespace(nodename='smci350-rck-g03-b19-03')),
                                 ('umask', 0o077), ('sched_setaffinity', None), ('getpriority', 10),
                                 ('nice', 10), ('urandom', session)):
                stack.enter_context(patch.object(G.os, name, return_value=result))
            stack.enter_context(patch.object(G.resource, 'getrlimit', return_value=(-1, -1)))
            stack.enter_context(patch.object(G.resource, 'setrlimit'))
            stack.enter_context(patch.object(G.signal, 'signal'))
            stack.enter_context(patch.object(G.signal, 'setitimer'))
            stack.enter_context(redirect_stdout(output))
            result = G.main()
            readset = copy.deepcopy(G.INPUTS)
        return result, json.loads(output.getvalue()), readset

    def test_full_main_reads_real_fixture_files_and_emits_exact_original_three_body_roster(self):
        before_globals = (G.ROOT, G.WORKER_CPU, G.PARENT_CPU, G.INPUTS, G.DEADLINE)
        result, printed, readset = self.execute()
        self.assertEqual(result, 0)
        self.assertEqual(before_globals, (G.ROOT, G.WORKER_CPU, G.PARENT_CPU, G.INPUTS, G.DEADLINE))
        names = {'tail_duration-request.json', 'tail_duration-input.json', 'prepared-inputs.json'}
        self.assertEqual({p.name for p in self.root.iterdir()}, names | {'prepare_model_inputs.py'})
        for path, pin in readset.items():
            self.assertEqual(body_pin(path, Path(path).read_bytes()), pin)
        request = json.loads((self.root / 'tail_duration-request.json').read_bytes())
        plan = json.loads((self.root / 'tail_duration-input.json').read_bytes())
        receipt = json.loads((self.root / 'prepared-inputs.json').read_bytes())
        self.assertEqual(printed['requests'], receipt['requests'])
        self.assertEqual(printed['plans'], receipt['plans'])
        self.assertEqual(printed['receipt'], readset[str(self.root / 'prepared-inputs.json')])
        self.assertEqual(receipt['readset'], {k: v for k, v in readset.items()
                                           if k != str(self.root / 'prepared-inputs.json')})
        expected = {str(self.script), str(self.historical_path)}
        expected |= {p['path'] for p in self.value.plan.values()}
        expected |= {v[k]['path'] for v in (self.value.worker, self.value.parent)
                     for k in ('controller', 'input_manifest')}
        expected |= {p['path'] for p in self.prompt.values()} | {p['path'] for p in self.images.values()}
        expected |= {p['path'] for p in self.old['decode']['images'].values()}
        expected |= {str(self.root / n) for n in names}
        self.assertEqual(set(readset), expected)
        self.assertEqual({k: plan[k] for k in self.value.plan}, self.value.plan)
        self.assertEqual(plan['request'], receipt['requests']['tail_duration'])
        self.assertEqual((plan['case'], plan['profile']), ('tail_duration', 'readiness40_position5'))
        self.assertEqual(request['schema'], 'FerricGuardedMlpReadiness40Position5RequestV1')
        base = request['base']
        for key in ('source', 'images', 'expected_bundle_id', 'expected_model_id', 'device_ids',
                    'prompt', 'dispatch_timeout_ms', 'child_deadline_ms'):
            self.assertEqual(base[key], self.old['decode'][key])
        self.assertEqual(base['session'], [2] * 32)
        self.assertNotEqual(base['session'], self.old['decode']['session'])
        self.assertEqual(base['worker'], G.rust(self.value.plan['worker']))
        self.assertEqual(base['evidence_directory'], str(self.root / 'tail_duration/native'))
        self.assertNotIn('mode', base)
        for key, pin in self.images.items():
            self.assertEqual(request[key], G.rust(pin))
        self.assertEqual((receipt['prompt_tokens'], receipt['prompt_positions_requested'],
                          receipt['generated_tokens_requested']), (2048, 40, 0))
        self.assertEqual(receipt['capture_positions'], [0, 5, 16, 39])
        self.assertTrue(receipt['instrumented'])
        for key in ('native_execution', 'gpu_execution', 'gpu_timing', 'full_long_workload',
                    'matched_speed_comparison', 'numerical_acceptance', 'performance_claim'):
            self.assertIs(receipt[key], False)

    def test_main_refuses_default_parent_even_when_original_receipt_hash_is_rebound(self):
        self.value.parent['qualification_mode'] = 'default'
        self.value.seal()
        self.value.materialize()
        with self.assertRaisesRegex(RuntimeError, 'only actual diagnostic V2'):
            self.execute()
        self.assertEqual({p.name for p in self.root.iterdir()}, {'prepare_model_inputs.py'})

    def test_main_refuses_original_receipt_map_and_product_byte_substitution(self):
        for key in ('parent_cpu', 'parent_sources', 'parent'):
            path = Path(self.value.plan[key]['path'])
            before = path.read_bytes()
            path.write_bytes(before + b' ')
            try:
                with self.subTest(key=key), self.assertRaises(RuntimeError):
                    self.execute()
            finally:
                path.write_bytes(before)

    def test_main_checks_natural_reaped_absent_qualification_ownership(self):
        self.value.parent['phases'][-1]['process_group_absent'] = False
        self.value.seal()
        self.value.materialize()
        with self.assertRaisesRegex(RuntimeError, 'CPU natural clean lifecycle'):
            self.execute()

    def test_main_preserves_original_device_and_deadline_rejections(self):
        for key, wrong in (('device_ids', list(reversed(R.IDS))), ('dispatch_timeout_ms', 10001),
                           ('child_deadline_ms', 3600001)):
            before = copy.deepcopy(self.old['decode'][key])
            self.old['decode'][key] = wrong
            self.rewrite_historical()
            try:
                with self.subTest(key=key), self.assertRaisesRegex(RuntimeError, 'unchanged device/deadline bounds'):
                    self.execute()
            finally:
                self.old['decode'][key] = before
                self.rewrite_historical()
        self.assertEqual({p.name for p in self.root.iterdir()}, {'prepare_model_inputs.py'})

    def test_main_rejects_reused_session_and_nonfresh_output_namespace(self):
        with self.assertRaisesRegex(RuntimeError, 'fresh diagnostic readiness session'):
            self.execute(session=bytes([1] * 32))
        collision = self.root / 'tail_duration-request.json'
        collision.write_bytes(b'original collision sentinel')
        with self.assertRaisesRegex(RuntimeError, 'fresh one-shot preparation'):
            self.execute()
        self.assertEqual(collision.read_bytes(), b'original collision sentinel')

    def test_main_keeps_fixed_image_and_all2048_prompt_hash_joins_active(self):
        for path in (Path(self.images['guarded_image']['path']), Path(self.prompt['tokens']['path'])):
            before = path.read_bytes()
            path.write_bytes(before + b' ')
            try:
                with self.subTest(path=path.name), self.assertRaisesRegex(RuntimeError, 'exact data input pin'):
                    self.execute()
            finally:
                path.write_bytes(before)

    def test_main_refuses_pending_parent_tuple_before_reading_qualification(self):
        with self.assertRaisesRegex(RuntimeError, 'CPU/ELF pins remain pending'):
            self.execute(overrides={'PARENT_CPU': None})
        self.assertEqual({p.name for p in self.root.iterdir()}, {'prepare_model_inputs.py'})


if __name__ == '__main__':
    unittest.main()
