"""Synthetic linked-intake policy only; no compiler, GPU, or subprocess calls."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import intake as I


def pin(path, size=1, sha='ab' * 32):
    return dict(path=str(path), bytes=size, sha256=sha)


def fixture():
    success = dict(passed=True, error=None, postcheck_errors=[])
    owned = dict(exit_code=0, reason=None, cleanup_signalled=False,
                 owned_groups_absent=True, owned_processes_reaped=True)
    plan = dict(prefix_cpu=pin(I.E / 'rope-materialized-cpu-v228-v3/complete.json'),
        prefix_emission_complete=pin(I.EMISSION_DIR / 'complete.json'),
        prefix_emission_owner=pin(I.EMISSION_OWNER_DIR / 'complete.json'),
        prefix_image=pin(I.E / 'transport/candidate.hsaco', 7, 'cd' * 32))
    generation = dict(prerequisites=copy.deepcopy(I.RPO_PREREQUISITES), roles={'compiler': 'old'})
    producer = dict(schema='ferric-p228-rope-materialized-rpo-lowering-result-v1',
        passed=False, error='AssertionError: actual-inert-join', postcheck_errors=[],
        candidate_cpu=plan['prefix_cpu'], compiler_generation=generation,
        commands=[dict(name=name) for name in I.PRODUCER_STAGES],
        artifacts={'prefix-tiles.handoff-v3': copy.deepcopy(I.HANDOFF)},
        fresh_checked_lowering=False, fresh_checked_replay=False, fresh_hsaco_emitted=False)
    producer_owner = dict(schema='ferric-p228-rope-materialized-rpo-lowering-owned-result-v1',
        passed=False, postcheck_errors=[], completion=None, candidate_cpu=plan['prefix_cpu'],
        compiler_generation=copy.deepcopy(generation), owned=dict(owned, exit_code=1))
    source = pin(I.CONSUMER / 'sources-before.json', 2, I.CONSUMER_SOURCE_SHA)
    tests = {name: dict(passed=passed, ignored=ignored) for name, passed, ignored in
             (('lower', 785, 0), ('indexed_subset_repeat', 20, 0), ('finalizer', 190, 15))}
    actual_join = dict(test='retained-test', passed=1, ignored=0, filtered_out=204,
                       exit_code=0, actual_capture_join_passed=True, fresh_hsaco_emitted=False)
    consumer = dict(success, schema='ferric-p228-kir-indexed-formal-join-cpu-result-v2',
        actual_capture_join_passed=True, fresh_hsaco_emitted=False, source_unchanged=True,
        limits_changed=False, fresh_compiler_built=False, full_compiler_cohort_requalified=False,
        compiler_cpu=copy.deepcopy(I.RPO_PREREQUISITES['cpu']),
        raw={'sources-before.json': source, 'sources-after.json': dict(source, path=str(I.CONSUMER / 'sources-after.json'))},
        proposal=pin('indexed-proposal'), tests=tests, artifacts={'finalizer-tests': pin('qualified-test')},
        retained_handoff=copy.deepcopy(I.HANDOFF), actual_join=actual_join)
    consumer_owner = dict(success, schema='ferric-p228-kir-indexed-formal-join-owned-result-v2',
        completion=copy.deepcopy(I.CONSUMER_PIN), actual_capture_join_passed=True,
        fresh_hsaco_emitted=False, owned=copy.deepcopy(owned))
    common = dict(success, **{key: False for key in I.EMISSION_FALSE}, fresh_hsaco_emitted=True,
        producer=copy.deepcopy(I.PRODUCER_PIN), producer_owner=copy.deepcopy(I.PRODUCER_OWNER_PIN),
        consumer=copy.deepcopy(I.CONSUMER_PIN), consumer_owner=copy.deepcopy(I.CONSUMER_OWNER_PIN),
        package=pin(I.E / 'p228-rope-indexed-checked-emission-v1/manifest.json', 3, I.EMISSION_PACKAGE_SHA))
    artifacts = {name: pin(I.EMISSION_DIR / name) for name in I.EMISSION_ARTIFACTS}
    artifacts['emitted/source.handoff-v3'] = dict(I.HANDOFF, path=str(I.EMISSION_DIR / 'emitted/source.handoff-v3'))
    artifacts['emitted/artifact.hsaco'] = dict(plan['prefix_image'], path=str(I.EMISSION_DIR / 'emitted/artifact.hsaco'))
    tools = {role: pin(I.CONSUMER / 'target/debug/examples' / name) for role, name in
             (('finalizer', 'finite_join_engineering_hsaco_v1'), ('metadata', 'finite_join_request_metadata_v1'))}
    value = dict(copy.deepcopy(common), schema='ferric-p228-rope-indexed-checked-emission-result-v1',
        producer_commands=copy.deepcopy(producer['commands']), producer_captures=copy.deepcopy(producer['artifacts']),
        producer_generation=copy.deepcopy(generation), consumer_source=copy.deepcopy(source),
        consumer_proposal=copy.deepcopy(consumer['proposal']), consumer_tests=copy.deepcopy(tests),
        preserved_consumer_test_artifacts=copy.deepcopy(consumer['artifacts']), retained_handoff=copy.deepcopy(I.HANDOFF),
        retained_checked_producer_replay_authenticated=True, producer_consumer_generations_distinct=True,
        source_unchanged=True, fresh_consumer_tools_built=True, fresh_actual_inert_join_passed=True,
        unresolved_runtime_requirements=8, commands=[dict(name=name) for name in I.EMISSION_STAGES],
        phases={name: {} for name in I.EMISSION_STAGES}, tools=tools, artifacts=artifacts,
        actual_join=copy.deepcopy(actual_join))
    owner = dict(copy.deepcopy(common), schema='ferric-p228-rope-indexed-checked-emission-owned-result-v1',
        completion=copy.deepcopy(plan['prefix_emission_complete']), owned=copy.deepcopy(owned))
    return value, owner, plan, producer, producer_owner, consumer, consumer_owner


class LinkedAdmissionTests(unittest.TestCase):
    def test_linked_success_preserves_failed_producer(self):
        rows = fixture()
        I.linked_contract(*rows)
        self.assertFalse(rows[3]['passed'])
        self.assertFalse(rows[0]['fresh_checked_lowering'])

    def test_old_or_mixed_generation_schema_refuses(self):
        for index in (0, 1, 3, 4, 5, 6):
            rows = fixture(); rows[index]['schema'] += '-other'
            with self.subTest(index=index), self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_aggregate_failure_is_never_relabelled(self):
        for key in ('passed', 'fresh_checked_lowering', 'fresh_checked_replay', 'fresh_hsaco_emitted'):
            rows = fixture(); rows[3][key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_new_consumer_cannot_replace_producer_generation(self):
        rows = fixture(); rows[0]['producer_generation'] = {'consumer': 'replacement'}
        with self.assertRaises(RuntimeError): I.linked_contract(*rows)
        for key in ('producer', 'producer_owner', 'consumer', 'consumer_owner'):
            rows = fixture(); rows[0][key] = pin('other')
            with self.subTest(key=key), self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_source_and_three_checked_leaves_remain_joined(self):
        for key in ('producer_commands', 'producer_captures', 'consumer_source', 'consumer_proposal',
                    'consumer_tests', 'preserved_consumer_test_artifacts'):
            rows = fixture(); rows[0][key] = {} if key != 'producer_commands' else []
            with self.subTest(key=key), self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_handoff_and_transported_image_identity_refuse(self):
        for where in ('handoff', 'source-copy', 'image'):
            rows = fixture()
            record = rows[0]['retained_handoff'] if where == 'handoff' else rows[0]['artifacts'][
                'emitted/source.handoff-v3' if where == 'source-copy' else 'emitted/artifact.hsaco']
            record['sha256'] = 'ff' * 32
            with self.subTest(where=where), self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_all_eight_continuation_stages_and_six_artifacts_required(self):
        for where in ('commands', 'phases', 'artifacts'):
            rows = fixture()
            if where == 'commands': rows[0][where].reverse()
            else: rows[0][where].pop(next(iter(rows[0][where])))
            with self.subTest(where=where), self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_natural_owners_and_exact_completion_are_required(self):
        for index in (1, 4, 6):
            for key, value in (('exit_code', True), ('reason', 'timeout'), ('cleanup_signalled', True),
                               ('owned_groups_absent', False), ('owned_processes_reaped', False)):
                rows = fixture(); rows[index]['owned'][key] = value
                with self.subTest(index=index, key=key), self.assertRaises(RuntimeError): I.linked_contract(*rows)
        rows = fixture(); rows[1]['completion'] = pin('different')
        with self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_false_authority_flags_are_not_inherited_from_success(self):
        for key in I.EMISSION_FALSE:
            rows = fixture(); rows[0][key] = True
            with self.subTest(key=key), self.assertRaises(RuntimeError): I.linked_contract(*rows)
        rows = fixture(); rows[0]['unresolved_runtime_requirements'] = 0
        with self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_consumer_qualification_and_full_join_cannot_be_skipped(self):
        for key in ('fresh_actual_inert_join_passed', 'fresh_consumer_tools_built',
                    'retained_checked_producer_replay_authenticated', 'producer_consumer_generations_distinct'):
            rows = fixture(); rows[0][key] = False
            with self.subTest(key=key), self.assertRaises(RuntimeError): I.linked_contract(*rows)
        rows = fixture(); rows[0]['actual_join']['passed'] = 0
        with self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_consumer_tools_cannot_use_old_producer_paths(self):
        for role in ('finalizer', 'metadata'):
            rows = fixture(); rows[0]['tools'][role]['path'] = str(I.PRODUCER / role)
            with self.subTest(role=role), self.assertRaises(RuntimeError): I.linked_contract(*rows)

    def test_linked_command_changes_only_consumer_paths_and_placeholder_values(self):
        rows = fixture(); value = rows[0]
        template = dict(name='emit', cwd='/old-working-directory', argv=['old-tool', str(I.HANDOFF['path']),
            '@candidate.handoff.bytes', str(I.PRODUCER / 'emitted')],
            env=dict(CARGO_TARGET_DIR='old-target', TMPDIR='old-tmp', LD_LIBRARY_PATH='old-deps:/nightly/lib'),
            deadline_seconds=900, nice=10)
        before = copy.deepcopy(template)
        result = I.linked_command(template, {'@candidate.handoff.bytes': I.HANDOFF['bytes']}, value)
        self.assertEqual(template, before)
        self.assertEqual(result['argv'], [value['tools']['finalizer']['path'], I.HANDOFF['path'],
            str(I.HANDOFF['bytes']), str(I.EMISSION_DIR / 'emitted')])
        self.assertEqual(result['env']['LD_LIBRARY_PATH'], str(I.CONSUMER / 'target/debug/deps') + ':/nightly/lib')
        self.assertEqual(result['deadline_seconds'], 900)

    def test_genuine_cargo_tool_stream_and_wrong_role_rejection(self):
        value = fixture()[0]
        rows = [dict(reason='compiler-artifact', package_id='path+file:///s#fe2o3-hsaco-finalize@0.1',
            target=dict(name=Path(pin['path']).name, kind=['example']), profile=dict(test=False),
            executable=pin['path']) for pin in value['tools'].values()]
        rows.append(dict(reason='build-finished', success=True))
        encode = lambda: ('\n'.join(json.dumps(row) for row in rows) + '\n').encode()
        I.consumer_builds(encode(), value['tools'])
        rows[0]['profile']['test'] = True
        with self.assertRaises(RuntimeError): I.consumer_builds(encode(), value['tools'])
        rows[0]['profile']['test'] = False; rows[0]['executable'] = 'wrong-generation'
        with self.assertRaises(RuntimeError): I.consumer_builds(encode(), value['tools'])

    def test_bad_continuation_namespace_fails_before_reading(self):
        for field in ('prefix_emission_complete', 'prefix_emission_owner'):
            plan = fixture()[2]; plan[field]['path'] = str(I.PRODUCER / 'failed.json')
            with patch.object(I, 'doc') as read, self.assertRaises(RuntimeError): I.prefix_evidence(None, plan, {})
            read.assert_not_called()

    def test_scope_remains_unqualified_until_actual_new_package_tests(self):
        for key in ('PACKAGE_FILES', 'PURE_TESTS', 'TEST_RUNNER_SHA'):
            with patch.object(I, key, None), self.subTest(key=key), self.assertRaises(RuntimeError):
                I.package_record(None)


if __name__ == '__main__':
    unittest.main(verbosity=2)
