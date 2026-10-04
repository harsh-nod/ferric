"""CPU-only routing tests; no fabricated native or reference qualification."""
from pathlib import Path
import tempfile
from types import SimpleNamespace
import types
import unittest
from unittest.mock import Mock, patch

import run as R


def pin(path='/retained/input', size=2, sha='ab' * 32):
    return dict(path=path, bytes=size, sha256=sha)


def rows(history=(True, True, True, True)):
    return [dict(position=i, same_input_history=same,
        tensors=[dict(byte_equal=j != 0) for j in range(38)] if same else None)
        for i, same in enumerate(history)]


def keys(value, names):
    R.require(type(value) is dict and set(value) == set(names.split()), 'closed keys')


def leaf_fixture():
    directory = R.E / 'prefix-independent-decode-tf4-shared-full-currentness-gpu-v228-v1'
    docs, leaves, reads = {}, {}, []
    for name in R.LEAVES:
        path = directory / name
        files = {n: pin(str(path / n), 0 if n == 'stderr' else 2)
                 for n in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
        value = dict(exit_code=0, reason=None, cleanup_signalled=False,
            owned_groups_absent=True, owned_processes_reaped=True,
            gpu_execution_requested=name == 'parent', command=files['command.json'],
            started=files['started.json'], stdout=files['stdout'], stderr=files['stderr'])
        docs[str(path / 'result.json')] = value
        docs[str(path / 'command.json')] = dict(argv=R.SMI_ARGV, cwd=str(R.E.parents[1]),
            deadline_seconds=30, affinity=[8, 9], nice=10, address_space_bytes=12 << 30,
            file_cap_bytes=64 << 20, stream_cap_bytes=8 << 20, gpu_execution_requested=False)
        docs[str(path / 'started.json')] = dict(command_sha256=files['command.json']['sha256'])
        leaves[name] = dict(result=files['result.json'], retained_files=files)
    receipt = dict(leaves=leaves)
    for when in ('before', 'after'):
        receipt[when + '_audits'] = []
        for i in range(3):
            name = when + '-' + str(i)
            topology = pin(str(directory / (name + '-topology.json')))
            docs[topology['path']] = {'synthetic': True}
            receipt[when + '_audits'].append(dict(topology=topology, process_result=leaves[name]['result']))
    def clean(value):
        R.require(type(value['exit_code']) is int and value['exit_code'] == 0 and value['reason'] is None
            and value['cleanup_signalled'] is False and value['owned_groups_absent'] is True
            and value['owned_processes_reaped'] is True, 'clean owner')
    I = SimpleNamespace(V=SimpleNamespace(require=R.require, keys=keys), R=R.E.parents[1],
        read=lambda _pins, p, *_args: reads.append(p), doc=lambda _pins, p, *_args: docs[p['path']],
        owned_success=clean)
    return I, directory, receipt, docs, reads


class DiagnosticTests(unittest.TestCase):
    def test_full152_summary_is_not_a_numerical_acceptance(self):
        actual = R.summary(rows())
        self.assertEqual(actual['captured_tensor_rows'], 152)
        self.assertEqual(actual['compared_tensor_rows'], 152)
        self.assertEqual(actual['byte_equal_tensor_rows'], 148)
        self.assertEqual(actual['differing_tensor_rows'], 4)
        self.assertNotIn('numerical_acceptance', actual)

    def test_history_divergence_keeps_later_positions_incomparable(self):
        actual = R.summary(rows((True, False, False, False)))
        self.assertEqual(actual['comparable_positions'], [0])
        self.assertEqual(actual['incomparable_positions'], [1, 2, 3])
        self.assertEqual(actual['compared_tensor_rows'], 38)

    def test_summary_refuses_recovered_history_or_invented_tensor_rows(self):
        bad = rows((True, False, False, False)); bad[1]['tensors'] = []
        bad_count = rows(); bad_count[0]['tensors'].pop()
        bad_type = rows(); bad_type[0]['same_input_history'] = 1
        for value in (rows()[:-1], rows((True, False, True, True)), bad, bad_count, bad_type):
            with self.subTest(value=str(value)[:80]), self.assertRaises(ValueError):
                R.summary(value)

    def test_reference_routing_uses_exact_mode_and_all_four_observations(self):
        for mode in ('teacher_forced', 'autoregressive'):
            with self.subTest(mode=mode):
                expected_records = [object()] * 4; expected_payloads = [b'reference'] * 4
                observed = dict(request={'mode': mode}); files = {f'observation-{i}.bin': bytes([i]) for i in range(4)}
                C = SimpleNamespace(reference=Mock(return_value=(expected_records, expected_payloads)),
                    records=Mock(return_value=['candidate'] * 4), compare_rows=Mock(return_value=['diagnostic']),
                    compare=Mock(side_effect=AssertionError('paired wrapper must not run')))
                I = SimpleNamespace(read=Mock()); H = {'diagnostics': object()}
                plan = dict(comparison_reference=pin())
                self.assertEqual(R.compare_retained(I, C, H, object(), plan, observed, files), ['diagnostic'])
                self.assertEqual(C.reference.call_args.args[1:], (plan['comparison_reference'], mode, H))
                C.compare_rows.assert_called_once_with(expected_records, expected_payloads,
                    ['candidate'] * 4, [bytes([i]) for i in range(4)], H['diagnostics'])
                C.compare.assert_not_called()

    def test_reference_pin_failure_is_fatal_not_a_tensor_difference(self):
        C = SimpleNamespace(reference=Mock(side_effect=RuntimeError('changed reference')), compare_rows=Mock())
        with self.assertRaisesRegex(RuntimeError, 'changed reference'):
            R.compare_retained(SimpleNamespace(read=Mock()), C, {}, object(),
                dict(comparison_reference=pin()), dict(request={'mode': 'teacher_forced'}), {})
        C.compare_rows.assert_not_called()

    def test_all_seven_owned_leaves_and_six_audits_are_rehashed(self):
        I, directory, receipt, docs, reads = leaf_fixture()
        parent = R.replay_leaves(I, object(), receipt, directory)
        self.assertIs(parent, docs[str(directory / 'parent/result.json')])
        self.assertEqual(len(reads), 35)

    def test_missing_leaf_file_or_post_audit_refuses(self):
        for change in ('leaf', 'file', 'audit'):
            I, directory, receipt, _, _ = leaf_fixture()
            if change == 'leaf': del receipt['leaves']['after-2']
            elif change == 'file': del receipt['leaves']['before-0']['retained_files']['stdout']
            else: receipt['after_audits'].pop()
            with self.subTest(change=change), self.assertRaises(ValueError):
                R.replay_leaves(I, object(), receipt, directory)

    def test_unclean_owner_wrong_record_join_and_wrong_audit_envelope_refuse(self):
        for change in ('owner', 'join', 'argv', 'deadline', 'gpu', 'topology'):
            I, directory, receipt, docs, _ = leaf_fixture()
            if change == 'owner': docs[str(directory / 'parent/result.json')]['owned_processes_reaped'] = False
            elif change == 'join': receipt['leaves']['parent']['result'] = pin('/wrong')
            elif change == 'argv': docs[str(directory / 'after-0/command.json')]['argv'] = ['/wrong']
            elif change == 'deadline': docs[str(directory / 'after-0/command.json')]['deadline_seconds'] = 31
            elif change == 'gpu': docs[str(directory / 'before-0/result.json')]['gpu_execution_requested'] = True
            else: receipt['after_audits'][0]['topology'] = pin('/wrong-topology')
            with self.subTest(change=change), self.assertRaises(ValueError):
                R.replay_leaves(I, object(), receipt, directory)

    def test_bad_digest_and_existing_output_refuse_before_source_loading(self):
        with tempfile.TemporaryDirectory() as name, patch.object(R, 'bootstrap') as load:
            with self.assertRaises(ValueError): R.execute('/no/receipt', 'bad', str(Path(name) / 'fresh'))
            with self.assertRaises(ValueError): R.execute('/no/receipt', 'aa' * 32, name)
            load.assert_not_called()

    def test_every_result_authority_flag_remains_false_by_construction(self):
        self.assertEqual(len(R.FALSE), len(set(R.FALSE)))
        self.assertTrue({'gpu_execution_requested', 'numerical_acceptance', 'full_model_acceptance',
            'production_authority', 'performance_claim', 'top_level_observer_reaping_verified'} <= set(R.FALSE))
        self.assertTrue(all(value is False for value in dict.fromkeys(R.FALSE, False).values()))

    def test_source_snapshot_includes_own_three_files_and_loaded_sources_not_tensors(self):
        records = {'/retained/helpers/math.py': pin('/retained/helpers/math.py'),
            '/retained/package/manifest.json': pin('/retained/package/manifest.json'),
            '/retained/observation.bin': pin('/retained/observation.bin')}
        def record(path):
            value = pin(str(path)); records[str(path)] = value
            return value
        pins = SimpleNamespace(records=records, pin=record)
        before = R.source_pins(None, pins)
        self.assertEqual(len(before), 5)
        self.assertNotIn('/retained/observation.bin', before)
        self.assertTrue({'run.py', 'test_run.py', 'README.md'} <= {Path(path).name for path in before})
        after = {path: record(path) for path in before}
        self.assertEqual(before, after)

    def test_frozen_compare_rows_never_recovers_after_history_divergence(self):
        path = R.E / 'p227-prefix-decode-gpu-qualification-v2/compare.py'
        raw = path.read_bytes()
        self.assertEqual(R.hashlib.sha256(raw).hexdigest(),
            '8154580de7f5ad40fd4897ce264fc3d92c9a109808ea7ea5dab6d0486f01e622')
        C = types.ModuleType('actual_frozen_diagnostic_comparator'); C.__file__ = str(path)
        exec(compile(raw, str(path), 'exec'), C.__dict__)
        left = [dict(input_token=t, output_token=0) for t in (1, 2, 3, 4)]
        right = [dict(input_token=t, output_token=0) for t in (1, 7, 3, 4)]
        diag = SimpleNamespace(validate_case=lambda record, raw, position: {'tiny': raw},
            compare_tensor=Mock(return_value={'diagnostic_only': True}))
        actual = C.compare_rows(left, [b'\0\0'] * 4, right, [b'\0\0'] * 4, diag)
        self.assertEqual([row['same_input_history'] for row in actual], [True, False, False, False])
        self.assertTrue(all(row['tensors'] is None for row in actual[1:]))
        self.assertEqual(diag.compare_tensor.call_count, 1)
        tf_size, tf_sha = C.REFERENCES['tf4'][:2]
        with self.assertRaisesRegex(ValueError, 'reference owner receipt'):
            C.reference(Mock(side_effect=AssertionError('must refuse before reading')),
                pin('/reference', tf_size, tf_sha), 'autoregressive', {})


if __name__ == '__main__':
    unittest.main(verbosity=2)
