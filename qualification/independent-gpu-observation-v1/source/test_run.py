import ast
import copy
import json
from pathlib import Path
import tempfile
import types
import unittest
from unittest import mock

import prepare as P
import run_case as M
from test_validation import fixture as legacy_fixture
from test_children import child_fixture


def platform():
    return dict(schema='ferric-p227-prefix-parity-platform-review-v1', authority='none', reviewed=True,
        host=P.HOST, boot_id='00000000-0000-0000-0000-000000000000', devices=M.V.DEVICES,
        topology_identity=[dict(device_path='/sys/devices/' + bdf) for bdf in M.SMI_ARGV[3:5]],
        software_audit=dict(path='/e/audit.json', bytes=1, sha256='a' * 64),
        notes='Synthetic fixture only, not an operator review.', production_authority=False,
        runtime_premises_discharged=False)


def idle_json():
    return json.dumps([dict(gpu=i, process_list=[dict(process_info='No running processes detected')])
                       for i in range(2)]).encode()


def fixture():
    requested, request_pin, baseline, inspected, observed, captured = legacy_fixture()
    inspected.update(schema=M.O.INSPECTION_SCHEMA, success=False, native_execution_attempted=False,
        paired_comparison_performed=False, immutable_input_readbacks_match=False, profiles=[], captures=[])
    observed.pop('stages')
    observed.update(schema=M.O.OBSERVATION_SCHEMA, success=True, native_execution_attempted=True,
                    paired_comparison_performed=False, bitwise_match=False)
    return requested, request_pin, baseline, inspected, observed, captured


class Tests(unittest.TestCase):
    def test_exact_actual_process_output_has_no_monitor_exception(self):
        self.assertEqual(len(M.process_audit(idle_json(), platform())), 2)
        rows = json.loads(idle_json())
        mutations = [[], rows[:1], list(reversed(rows)), rows + [rows[0]],
            [dict(rows[0], gpu=True), rows[1]], [dict(rows[0], process_list=[]), rows[1]],
            [dict(rows[0], process_list=[dict(pid=123)]), rows[1]],
            [dict(rows[0], extra=False), rows[1]],
            [dict(rows[0], process_list=[dict(process_info='Unknown')]), rows[1]]]
        for value in mutations:
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                M.process_audit(json.dumps(value).encode(), platform())

    def test_process_output_binds_physical_pair_not_just_gpu_ordinals(self):
        value = platform(); value['topology_identity'].reverse()
        with self.assertRaises(RuntimeError): M.process_audit(idle_json(), value)
        for key in ('reviewed', 'production_authority', 'runtime_premises_discharged'):
            value = platform(); value[key] = not value[key]
            with self.assertRaises(RuntimeError): M.process_audit(idle_json(), value)

    def test_native_and_audit_limits_keep_existing_caps_without_raising_soft_limit(self):
        calls = []
        with mock.patch.object(M.os, 'sched_setaffinity') as affinity, mock.patch.object(M.os, 'nice') as nice, \
                mock.patch.object(M.resource, 'getrlimit', return_value=(1234, 999999999999)), \
                mock.patch.object(M.resource, 'setrlimit', side_effect=lambda kind, pair: calls.append((kind, pair))):
            M.child_limits(180)
        affinity.assert_called_once_with(0, {8, 9}); nice.assert_called_once_with(10)
        self.assertEqual(dict(calls)[M.resource.RLIMIT_AS], (1234, 1234))
        self.assertEqual(dict(calls)[M.resource.RLIMIT_CPU], (180, 180))
        self.assertEqual((M.CASE_CAP, M.STREAM_CAP, M.LEAF_SECONDS, M.CASE_SECONDS),
                         (32 << 20, 8 << 20, 180, 2400))

    def test_full_capture_bytes_fit_case_not_old_small_capture_cap(self):
        self.assertEqual(M.V.CASE_CAPTURE_BYTES, 19030016)
        self.assertLess(M.V.CASE_CAPTURE_BYTES, M.CASE_CAP)
        self.assertGreater(M.V.CASE_CAPTURE_BYTES, M.STREAM_CAP)
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            for i in range(4):
                with (base / str(i)).open('wb') as stream: stream.truncate(M.V.CAPTURE_BYTES)
            M.inventory(base)
            with self.assertRaises(RuntimeError): M.inventory(base, M.CASE_CAP)

    def test_fresh_case_sequence_and_failure_directory_cannot_be_retried(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary) / 'matrix'
            M.prior_cases({}, base, 0)
            base.mkdir()
            with self.assertRaises(RuntimeError): M.prior_cases({}, base, 0)
            (base / M.V.CASES[0]).mkdir()
            (base / M.V.CASES[0] / 'failure.json').write_text('{}')
            with self.assertRaises((RuntimeError, FileNotFoundError)):
                M.prior_cases(dict(pins=P.D.Pins()), base, 1)
        for value in ('-1', '6', '00', '1.0'):
            with self.assertRaises(RuntimeError): M.arguments(value)

    def test_any_owned_failure_or_forced_cleanup_is_not_success(self):
        value = dict(exit_code=0, reason=None, owned_groups_absent=True,
                     owned_processes_reaped=True, cleanup_signalled=False)
        M.leaf_success(value)
        for key, replacement in [('exit_code', True), ('exit_code', 1), ('reason', 'deadline'),
                ('owned_groups_absent', False), ('owned_processes_reaped', False), ('cleanup_signalled', True)]:
            with self.assertRaises(RuntimeError): M.leaf_success(dict(value, **{key: replacement}))

    def execution(self, directory, failure=None, retain_context=False):
        requested, _, baseline, inspected, observed, captured = fixture()
        label = 'prefix-independent-profile-gpu-v228-v1'; out = directory / label / M.V.CASES[0]
        requested['capture_directory'] = str(out / 'captures')
        request_pin = P.save(directory / 'request.json', requested)
        inspected['request_sha256'] = observed['request_sha256'] = request_pin['sha256']
        new_captures = {}
        for ranks in observed['captures']:
            for record in ranks:
                raw = captured[record['path']]
                record['path'] = str(out / 'captures' / Path(record['path']).name)
                new_captures[record['path']] = raw
        if failure == 'different_finite':
            record = observed['captures'][1][0]
            raw = b'\x80\x3f' + new_captures[record['path']][2:]
            new_captures[record['path']] = raw; record['sha256'] = M.V.sha(raw)
        pins = P.D.Pins()
        sample = dict(devices=[dict(unique_id=uid, gpu_busy=0, memory_busy=0, vram_used=1)
                               for uid in M.V.DEVICES])
        helper = types.SimpleNamespace(topology_sample=lambda: copy.deepcopy(sample),
            require_idle=lambda values: M.V.require(len(values) == 3, 'three samples'))
        owned = types.SimpleNamespace(processes=lambda: {})
        prepared = dict(cases=[dict(request=request_pin)], binary=dict(path='/e/binary', bytes=1, sha256='b' * 64),
                        object=dict(path='/e/image', bytes=1, sha256='c' * 64),
                        compiler_complete={}, native_complete={}, compiler_generation={})
        c = dict(pins=pins, owned=owned, topology=helper, inputs=dict(matrix_label=label),
            requests=[(requested, baseline)], prepared=prepared, prepared_pin={}, environment=P.ENV,
            provenance={key: prepared[key] for key in ('binary', 'object', 'compiler_complete',
                'native_complete', 'compiler_generation')}, runtime=dict(libraries=[]), platform=platform())
        calls = []
        def bounded(folder, argv, env, owned, case_root, gpu, seconds):
            calls.append((folder.name, gpu, seconds))
            if folder.name == 'native' and failure == 'cleanup_raise':
                raise RuntimeError('synthetic emergency cleanup failure')
            raw = idle_json()
            exit_code = 0
            if folder.name == 'inspection': raw = json.dumps(inspected).encode()
            if folder.name == 'native':
                (out / 'captures').mkdir()
                for name, data in new_captures.items(): Path(name).write_bytes(data)
                body = copy.deepcopy(observed)
                if failure == 'native':
                    body.update(schema='fe2o3-qwen-prefix-tiles-independent-profiles-failure-v1', bitwise_match=False,
                                completed_and_closed=False)
                    exit_code = 1
                if failure == 'old_schema': body['schema'] = 'fe2o3-qwen-prefix-tiles-comparison-observation-v6'
                if failure == 'parity_claim': body['bitwise_match'] = True
                raw = json.dumps(body).encode()
            (folder / 'stdout').write_bytes(raw); (folder / 'stderr').write_bytes(b'')
            command = P.save(folder / 'command.json', dict(argv=argv, env=env, cwd=str(M.R),
                deadline_seconds=seconds, affinity=[8, 9], nice=10, address_space_bytes=12 << 30,
                stream_cap_bytes=M.STREAM_CAP, gpu_execution_requested=gpu))
            child_records = child_fixture(out, request_pin, prepared['binary'], observed) if folder.name == 'native' else {}
            if child_records:
                start_value = json.loads(Path(child_records['started']['path']).read_bytes())
                start_value['command_sha256'] = command['sha256']
                Path(child_records['started']['path']).unlink()
                started = P.save(folder / 'started.json', start_value)
            else:
                started = P.save(folder / 'started.json', dict(command_sha256=command['sha256']))
            value = dict(exit_code=exit_code, reason=None, cleanup_signalled=False,
                owned_groups_absent=True, owned_processes_reaped=True, command=command, started=started,
                stdout=P.D.read_file(folder / 'stdout')[0], stderr=P.D.read_file(folder / 'stderr')[0],
                lineage=child_records.get('lineage', []), gpu_execution_requested=gpu)
            if folder.name == 'native' and failure == 'child_sidecar':
                (out / M.C.NAMES[-1]).unlink()
            return value, P.save(folder / 'result.json', value)
        def pinned(path, digest=None):
            if path == M.SMI_LAUNCHER: return dict(path=str(path), bytes=1, sha256=M.SMI_SHA)
            return real_pin(path, digest)
        real_pin = pins.pin
        with mock.patch.object(M, 'E', directory), mock.patch.object(M, 'resources'), \
                mock.patch.object(M, 'check_platform'), mock.patch.object(M, 'bounded', side_effect=bounded), \
                mock.patch.object(Path, 'resolve', autospec=True,
                    side_effect=lambda path, strict=False: M.SMI_LAUNCHER if str(path) == M.SMI_ARGV[0]
                    else real_resolve(path, strict=strict)), \
                mock.patch.object(pins, 'pin', side_effect=pinned), mock.patch.object(M.time, 'sleep'):
            result = M.execute(c, 0)
        receipt = json.loads((out / ('complete.json' if result == 0 else 'failure.json')).read_bytes())
        if retain_context: return result, receipt, calls, c, out
        return result, receipt, calls

    def test_owned_case_success_requires_three_audits_each_side_and_explicit_close(self):
        with tempfile.TemporaryDirectory() as temporary:
            result, receipt, calls = self.execution(Path(temporary))
        self.assertEqual(result, 0)
        self.assertEqual([name for name, _, _ in calls],
            ['inspection', 'before-0', 'before-1', 'before-2', 'native', 'after-0', 'after-1', 'after-2'])
        self.assertEqual(sum(gpu for _, gpu, _ in calls), 1)
        self.assertEqual(receipt['checked']['capture_bytes'], 19030016)
        self.assertEqual(len(receipt['retained_captures']), 4)
        self.assertEqual(len(receipt['retained_profile_files']), 10)
        self.assertEqual(receipt['profile_children']['profile_attempts'], 2)

    def test_native_failure_keeps_all_closed_captures_then_postaudits_without_retry(self):
        with tempfile.TemporaryDirectory() as temporary:
            result, receipt, calls = self.execution(Path(temporary), 'native')
        self.assertEqual(result, 1)
        self.assertFalse(receipt['passed']); self.assertEqual(receipt['native_attempts'], 1)
        self.assertEqual(len(receipt['retained_captures']), 4)
        self.assertEqual([name for name, _, _ in calls][-3:], ['after-0', 'after-1', 'after-2'])

    def test_cleanup_exception_still_attempts_all_three_postaudits(self):
        with tempfile.TemporaryDirectory() as temporary:
            result, receipt, calls = self.execution(Path(temporary), 'cleanup_raise')
        self.assertEqual(result, 1); self.assertFalse(receipt['passed'])
        self.assertEqual([name for name, _, _ in calls][-3:], ['after-0', 'after-1', 'after-2'])
        self.assertEqual(sum(gpu for _, gpu, _ in calls), 1)

    def test_independent_finite_difference_is_not_a_parity_or_numerical_gate(self):
        with tempfile.TemporaryDirectory() as temporary, \
                mock.patch.object(M.O, 'compare_retained', side_effect=AssertionError('no math inside GPU scope')):
            result, receipt, calls = self.execution(Path(temporary), 'different_finite')
        self.assertEqual(result, 0)
        self.assertFalse(receipt['checked']['bitwise_match'])
        self.assertFalse(receipt['independent_numerical_acceptance'])
        self.assertNotEqual(receipt['checked']['stage_sha256'][0][0][0],
                            receipt['checked']['stage_sha256'][1][0][0])
        self.assertEqual([name for name, _, _ in calls][-3:], ['after-0', 'after-1', 'after-2'])

    def test_old_parent_schema_and_parity_claim_refuse_with_all_postaudits(self):
        for failure in ('old_schema', 'parity_claim'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as temporary:
                result, receipt, calls = self.execution(Path(temporary), failure)
            self.assertEqual(result, 1); self.assertFalse(receipt['passed'])
            self.assertEqual([name for name, _, _ in calls][-3:], ['after-0', 'after-1', 'after-2'])
            self.assertEqual(receipt['native_attempts'], 1)

    def test_replay_case_rechecks_real_file_backed_capture_custody_without_launch(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            result, receipt, _, c, out = self.execution(directory, retain_context=True)
            self.assertEqual(result, 0)
            with mock.patch.object(M, 'E', directory), mock.patch.object(M, 'check_platform'), \
                    mock.patch.object(M, 'bounded', side_effect=AssertionError('read-only replay')), \
                    mock.patch.object(M.O, 'compare_retained', side_effect=AssertionError('separate CPU scope')):
                replayed = M.replay_case(c, P.D.read_file(out / 'complete.json')[0])
            self.assertEqual(replayed['request_pin'], receipt['request'])
            self.assertEqual(replayed['case_directory'], str(out))
            self.assertEqual(replayed['native_result'], receipt['leaves']['native']['result'])

    def test_replay_case_refuses_receipt_drift_before_any_launch(self):
        for mutation in ('audit_duplicate', 'provenance', 'attempt', 'authority', 'captures', 'schema',
                         'leaf_file', 'leaf_result'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                directory = Path(temporary)
                _, receipt, _, c, out = self.execution(directory, retain_context=True)
                if mutation == 'audit_duplicate': receipt['after_audits'][1] = receipt['after_audits'][0]
                if mutation == 'provenance': receipt['compiler_generation'] = dict(old=True)
                if mutation == 'attempt': receipt['native_attempts'] = True
                if mutation == 'authority': receipt['independent_numerical_acceptance'] = True
                if mutation == 'captures': receipt['retained_captures'].popitem()
                if mutation == 'schema': receipt['schema'] = 'ferric-p227-prefix-parity-gpu-observation-v4'
                if mutation == 'leaf_file': receipt['leaves']['native']['retained_files']['stdout']['path'] = '/old/stdout'
                if mutation == 'leaf_result': receipt['leaves']['native']['result'] = receipt['leaves']['inspection']['result']
                (out / 'complete.json').write_text(json.dumps(receipt))
                with mock.patch.object(M, 'E', directory), mock.patch.object(M, 'check_platform'), \
                        mock.patch.object(M, 'bounded', side_effect=AssertionError('read-only replay')), \
                        self.assertRaises(RuntimeError):
                    M.replay_case(c, P.D.read_file(out / 'complete.json')[0])

    def test_lifecycle_and_resource_helpers_are_exact_preimage_ast(self):
        root = Path(__file__).resolve().parent
        def functions(path):
            return {node.name: ast.dump(node, include_attributes=False)
                    for node in ast.parse(path.read_text()).body if isinstance(node, ast.FunctionDef)}
        before, after = functions(root / 'preimage/run_case.py'), functions(root / 'run_case.py')
        for name in ('arguments', 'resources', 'inventory', 'quiescent', 'leaf_success', 'child_limits',
                     'bounded', 'topology_identity', 'check_platform', 'process_audit', 'capture_bytes', 'replay_leaf'):
            self.assertEqual(before[name], after[name], name)

    def test_missing_child_sidecar_keeps_partial_controls_and_all_captures_without_success(self):
        with tempfile.TemporaryDirectory() as temporary:
            result, receipt, calls = self.execution(Path(temporary), 'child_sidecar')
        self.assertEqual(result, 1); self.assertFalse(receipt['passed'])
        self.assertIsNone(receipt['profile_children']); self.assertEqual(len(receipt['retained_profile_files']), 9)
        self.assertEqual(len(receipt['retained_captures']), 4)
        self.assertEqual([name for name, _, _ in calls][-3:], ['after-0', 'after-1', 'after-2'])
        self.assertEqual(sum(gpu for _, gpu, _ in calls), 1)


real_resolve = Path.resolve


if __name__ == '__main__':
    unittest.main()
