"""Synthetic continuation policy tests; no Cargo or process creation."""
import copy
import importlib.util
from pathlib import Path
import types
import unittest

spec = importlib.util.spec_from_file_location('linked_rope_emission', Path(__file__).with_name('run.py'))
M = importlib.util.module_from_spec(spec)
spec.loader.exec_module(M)


class ContinuationTests(unittest.TestCase):
    def qualification(self):
        h = types.SimpleNamespace(PHASES={str(i) for i in range(13)}, HANDOFF={'original': 'handoff'})
        pin = {'actual': 'qualification'}
        common = dict(passed=True, error=None, postcheck_errors=[], actual_capture_join_passed=True,
            fresh_hsaco_emitted=False, gpu_execution=False, production_authority=False,
            package={'sha256': M.QUALIFIER_PACKAGE_SHA}, proposal={'sha256': M.QUALIFIER_SOURCE_SHA})
        value = dict(common, schema='ferric-p228-kir-indexed-formal-join-cpu-result-v2',
            staged_join_qualification_completed=True, source_unchanged=True, limits_changed=False,
            fresh_compiler_built=False, full_compiler_cohort_requalified=False,
            phases={name: {} for name in h.PHASES}, raw={str(i): {} for i in range(73)},
            artifacts={'lower-tests': {'path': str(M.TARGET / 'debug/deps/lower')},
                'finalizer-tests': {'path': str(M.TARGET / 'debug/examples/test'), 'sha256': M.TEST_ELF_SHA}},
            formatted_sources={str(i): {} for i in range(5)}, retained_handoff=h.HANDOFF,
            actual_join={'actual_capture_join_passed': True},
            tests={name: dict(passed=passed, ignored=ignored) for name, passed, ignored in
                (('lower', 785, 0), ('indexed_subset_repeat', 20, 0), ('finalizer', 190, 15))})
        value['raw'].update({name: dict(bytes=1, sha256=M.SOURCE_SHA)
            for name in ('sources-before.json', 'sources-after.json')})
        owner = dict(common, schema='ferric-p228-kir-indexed-formal-join-owned-result-v2', completion=pin)
        return value, owner, pin, h

    def producer(self):
        lower = types.SimpleNamespace(CAPTURES=('semantic', 'neutral', 'target', 'handoff'),
            FALSE_FLAGS=('gpu_execution', 'production_authority'))
        c = dict(generation={'original': 'compiler'}, manifest={'actual': 'package'},
            cpu_pin={'actual': 'CPU33'}, source_manifest={'actual': 'RoPE'})
        common = dict(passed=False, postcheck_errors=[], compiler_generation=c['generation'],
            package_manifest=c['manifest'], candidate_cpu=c['cpu_pin'], source_manifest=c['source_manifest'])
        value = dict(common, schema='ferric-p228-rope-materialized-rpo-lowering-result-v1',
            error='AssertionError: actual-inert-join', commands=[dict(name=name) for name in M.PRODUCER_PHASES],
            artifacts={name: {} for name in lower.CAPTURES}, fresh_checked_lowering=False,
            fresh_checked_replay=False, fresh_hsaco_emitted=False, gpu_execution=False, production_authority=False)
        owner = dict(common, schema='ferric-p228-rope-materialized-rpo-lowering-owned-result-v1', completion=None,
            owned=dict(exit_code=1, reason=None, cleanup_signalled=False, owned_groups_absent=True, owned_processes_reaped=True))
        return value, owner, c, lower

    def template(self, name='emit'):
        target = M.E / 'rpo-compiler-cpu-v228-v2/target'
        command = dict(name=name, expected_exit=0, affinity=[8, 9], nice=10,
            cache_cap_bytes=6 << 30, gpu_execution=False, deadline_seconds=900,
            argv=['old-tool', str(M.PRODUCER / 'prefix-tiles.handoff-v3'),
                  str(M.PRODUCER / 'emitted'), str(M.PRODUCER / 'extracted/module.ll')],
            env=dict(CARGO_TARGET_DIR=str(M.PRODUCER / 'target'), TMPDIR=str(M.PRODUCER / 'tmp'),
                LD_LIBRARY_PATH=str(target / 'debug/deps') + ':/nightly/lib', HIP_VISIBLE_DEVICES=''))
        roles = {key: {'path': str(M.TARGET / 'debug/examples' / key)}
            for key in ('finalizer', 'metadata', 'finalizer-tests')}
        return command, target, roles

    def test_actual_qualified_consumer_accepted(self):
        M.qualified_gate(*self.qualification())

    def test_consumer_must_have_full_successful_qualification(self):
        for key, value in (('passed', False), ('postcheck_errors', ['drift']), ('actual_capture_join_passed', False),
                           ('source_unchanged', False), ('staged_join_qualification_completed', False)):
            fixture = self.qualification(); fixture[0][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): M.qualified_gate(*fixture)

    def test_consumer_source_and_selected_test_are_exact(self):
        for key in ('sources-before.json', 'sources-after.json'):
            fixture = self.qualification(); fixture[0]['raw'][key]['sha256'] = 'wrong'
            with self.assertRaises(RuntimeError): M.qualified_gate(*fixture)
        fixture = self.qualification(); fixture[0]['artifacts']['finalizer-tests']['sha256'] = 'old-consumer'
        with self.assertRaises(RuntimeError): M.qualified_gate(*fixture)

    def test_consumer_owner_and_proposal_cannot_be_substituted(self):
        for key in ('completion', 'proposal', 'package'):
            fixture = self.qualification(); fixture[1][key] = {'sha256': 'wrong'}
            with self.subTest(key=key), self.assertRaises(RuntimeError): M.qualified_gate(*fixture)

    def test_consumer_full_census_and_repeats_remain_separate(self):
        for name in ('lower', 'indexed_subset_repeat', 'finalizer'):
            fixture = self.qualification(); fixture[0]['tests'][name]['passed'] += 1
            with self.assertRaises(RuntimeError): M.qualified_gate(*fixture)
        fixture = self.qualification(); fixture[0]['raw'].pop('0')
        with self.assertRaises(RuntimeError): M.qualified_gate(*fixture)

    def test_consumer_cannot_claim_emission_or_compiler_qualification(self):
        for key in ('fresh_hsaco_emitted', 'fresh_compiler_built', 'full_compiler_cohort_requalified',
                    'gpu_execution', 'production_authority', 'limits_changed'):
            fixture = self.qualification(); fixture[0][key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError): M.qualified_gate(*fixture)

    def test_exact_failed_producer_is_retained_as_failed(self):
        M.producer_gate(*self.producer())
        for key in ('passed', 'fresh_checked_lowering', 'fresh_checked_replay', 'fresh_hsaco_emitted'):
            fixture = self.producer(); fixture[0][key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError): M.producer_gate(*fixture)

    def test_producer_captures_and_three_completed_leaves_required(self):
        fixture = self.producer(); fixture[0]['commands'].append(dict(name='actual-inert-join'))
        with self.assertRaises(RuntimeError): M.producer_gate(*fixture)
        fixture = self.producer(); fixture[0]['artifacts'].pop('handoff')
        with self.assertRaises(RuntimeError): M.producer_gate(*fixture)

    def test_producer_owner_must_be_natural_failed_and_reaped(self):
        for key, value in (('exit_code', 0), ('exit_code', True), ('reason', 'timeout'),
                           ('cleanup_signalled', True), ('owned_groups_absent', False), ('owned_processes_reaped', False)):
            fixture = self.producer(); fixture[1]['owned'][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): M.producer_gate(*fixture)

    def test_producer_generation_is_not_replaced_with_consumer(self):
        for key in ('compiler_generation', 'package_manifest', 'candidate_cpu', 'source_manifest'):
            fixture = self.producer(); fixture[0][key] = {'new': 'consumer'}
            with self.subTest(key=key), self.assertRaises(RuntimeError): M.producer_gate(*fixture)

    def test_command_replaces_only_outputs_tool_loader_and_temporary_target(self):
        template, target, roles = self.template(); original = copy.deepcopy(template)
        result = M.consumer_command(template, target, roles)
        self.assertEqual(template, original)
        self.assertEqual(result['argv'], [roles['finalizer']['path'], str(M.PRODUCER / 'prefix-tiles.handoff-v3'),
            str(M.OUT / 'emitted'), str(M.OUT / 'extracted/module.ll')])
        self.assertEqual(result['env']['LD_LIBRARY_PATH'], str(M.TARGET / 'debug/deps') + ':/nightly/lib')
        self.assertEqual(result['env']['CARGO_TARGET_DIR'], str(M.TARGET))
        self.assertEqual(result['env']['TMPDIR'], str(M.OUT / 'tmp'))

    def test_only_three_consumer_tool_roles_change(self):
        for name, role in (('actual-inert-join', 'finalizer-tests'), ('emit', 'finalizer'), ('descriptor-metadata', 'metadata')):
            template, target, roles = self.template(name)
            self.assertEqual(M.consumer_command(template, target, roles)['argv'][0], roles[role]['path'])
        for name in ('extract-retained', 'elf-notes', 'disassembly'):
            template, target, roles = self.template(name)
            self.assertEqual(M.consumer_command(template, target, roles)['argv'][0], 'old-tool')

    def test_command_rejects_unreviewed_bounds_and_diagnostic_flag(self):
        for key, value in (('name', 'checked-lowering'), ('expected_exit', 101), ('affinity', [0, 1]),
                           ('nice', 0), ('cache_cap_bytes', 7 << 30), ('gpu_execution', True)):
            template, target, roles = self.template(); template[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError): M.consumer_command(template, target, roles)
        template, target, roles = self.template(); template['env']['FE2O3_KIR_JOIN_WORK_DIAGNOSTIC_V1'] = '1'
        with self.assertRaises(RuntimeError): M.consumer_command(template, target, roles)

    def test_command_rejects_unexpected_old_namespace_or_unbuilt_tool(self):
        for key in ('CARGO_TARGET_DIR', 'TMPDIR', 'LD_LIBRARY_PATH'):
            template, target, roles = self.template(); template['env'][key] = '/other'
            with self.assertRaises(RuntimeError): M.consumer_command(template, target, roles)
        template, target, roles = self.template(); del roles['finalizer']
        with self.assertRaises(RuntimeError): M.consumer_command(template, target, roles)

    def test_only_exact_active_target_is_excluded_from_immutability(self):
        prior = M.PRODUCER / 'target'
        self.assertEqual(M.protected_targets((prior, M.TARGET, prior)), (prior,))
        for paths in ((M.TARGET,), (M.TARGET.parent,), (M.TARGET / 'nested',)):
            with self.assertRaises(RuntimeError): M.protected_targets(paths)

    def test_eight_phases_do_not_rerun_producer_or_full_cohorts(self):
        self.assertEqual(len(M.PHASES), 8)
        self.assertEqual(M.PHASES[:2], ('metadata', 'tool-build'))
        self.assertFalse(set(M.PHASES) & {'checked-lowering', 'actual-replay', 'lower-tests', 'finalizer-tests'})
        self.assertIn('fresh_checked_replay', M.FALSE_FLAGS)
        self.assertIn('full_compiler_cohort_requalified', M.FALSE_FLAGS)


if __name__ == '__main__':
    unittest.main(verbosity=2)
