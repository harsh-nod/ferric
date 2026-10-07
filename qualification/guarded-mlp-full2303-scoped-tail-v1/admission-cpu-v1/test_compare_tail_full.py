"""Synthetic Full bank/census behavior admission; no observed native outcome."""
import copy
import json
import unittest
from unittest import mock

import compare_full as C
import compare_tail_full as S
import test_full as F
import test_tail_full as G
import test_scoped_full as H
import validate_full as V
import validate_tail_full as P
from test_compare_scoped_full import DECODED, MARKER, reference_fixture, rejoin


def originals(value):
    summary, request, _, _ = value
    summary_raw = F.summary_bytes(summary)
    wrapper = dict(schema=P.WRAPPER, observation=summary, currentness_policy=G.policy(summary))
    command = F.wire(dict(argv=['/task/ferric-qwen3-guarded-mlp-full2303-engineering', '--request',
        '/task/request.json', S.SELECTOR, '--allow-unauthenticated-machine-code'], gpu_execution_requested=True))
    result, started = F.lineage()
    started['command_sha256'] = C.compact(command)['sha256']
    result['gpu_execution_requested'] = True
    bodies = {'wrapper.json': F.wire(wrapper) + b'\n', 'summary.json': summary_raw,
        'request.json': F.wire(request), 'parent-command.json': command,
        'parent-started.json': F.wire(started), 'parent-stderr': MARKER}
    bodies['parent-result.json'] = F.wire(result)
    return rejoin(bodies)



class TailBehaviorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = F.fixture()
        H.install(cls.base, F.wire(G.policy(cls.base[0])) + b'\n')
        F.summary_bytes(cls.base[0])
        cls.reference = reference_fixture(cls.base[1])

    def attempt(self, value=None, bodies=None, reference=None):
        value = copy.deepcopy(self.base) if value is None else value
        bodies = originals(value) if bodies is None else bodies
        reference = self.reference if reference is None else reference
        with mock.patch.object(C, 'authenticate_reference', return_value=reference):
            return S.admit(bodies, {n: C.compact(b) for n, b in bodies.items()},
                           lambda p: value[2][p['path']], lambda n, m: b'')

    def test_tail_seven_originals_join_scoped_stdout_retained_summary_and_own256(self):
        value = copy.deepcopy(self.base)
        before = value[2][value[0]['files']['child_stderr']['path']]
        got = self.attempt(value)
        self.assertTrue(got['passed'])
        self.assertTrue(got['independent_own_histories'])
        self.assertEqual(got['comparison']['generated_ids_checked'], 256)
        self.assertTrue(got['tail']['outer_owned_lineage_checked'])
        self.assertTrue(got['tail']['ordinary']['process_retirement_independently_checked'])
        self.assertEqual(got['tail']['ordinary']['completed_forwards'], 2303)
        self.assertEqual(got['tail']['ordinary']['generated_tokens'], [3] * 256)
        self.assertEqual(got['tail']['policy']['policy_record'], G.policy(value[0]))
        self.assertEqual(before, value[2][value[0]['files']['child_stderr']['path']])
        self.assertEqual(set(got['native_admission_pins']), set(S.INPUT_NAMES))
        for key in ('numerical_acceptance', 'full_tensor_numerical_acceptance', 'performance_claim',
                    'production_authority', 'temporal_equivalent_to_full', 'native_launch_admitted',
                    'external_model_rehashed', 'cpu_and_elf_qualification_checked', 'native_execution'):
            self.assertFalse(got[key])

    def test_tail_different_genuine_reference_history_fails_all256_without_tie_exemption(self):
        reference = reference_fixture(self.base[1], token=7)
        got = self.attempt(reference=reference)
        self.assertFalse(got['passed'])
        self.assertTrue(got['comparison']['decoded_bytes_equal'])
        rows = got['comparison']['generated_id_mismatches']
        self.assertEqual(len(rows), 256)
        self.assertEqual(rows[0], dict(index=0, position=2047, native=3, reference=7))
        self.assertEqual(rows[-1], dict(index=255, position=2302, native=3, reference=7))
        self.assertFalse(got['comparison']['reference_tie_exemption'])
        self.assertEqual(reference['passes'][0]['cases'][1]['input_token'], 7)
        self.assertEqual(got['tail']['ordinary']['generated_tokens'], [3] * 256)

    def test_tail_exact_decoded_bytes_fail_without_normalization_when_ids_match(self):
        for decoded in (DECODED + b'\n', DECODED.replace(b'\x00', b'\r\n'), b'\xfe' + DECODED[1:]):
            with self.subTest(decoded=decoded):
                got = self.attempt(reference=reference_fixture(self.base[1], decoded=decoded))
                self.assertFalse(got['passed'])
                self.assertTrue(got['comparison']['generated_ids_equal'])
                self.assertFalse(got['comparison']['decoded_bytes_equal'])

    def test_tail_roster_and_all_observed_pins_precede_fixed_reference_authentication(self):
        good = originals(copy.deepcopy(self.base))
        for name in S.INPUT_NAMES:
            bad = dict(good); bad.pop(name)
            with mock.patch.object(C, 'authenticate_reference', side_effect=AssertionError('not reached')):
                with self.assertRaises(ValueError):
                    S.admit(bad, {n: C.compact(b) for n, b in bad.items()}, lambda p: b'', lambda n, m: b'')
            pins = {n: C.compact(b) for n, b in good.items()}
            pins[name]['sha256'] = '0' * 64
            with mock.patch.object(C, 'authenticate_reference', side_effect=AssertionError('not reached')):
                with self.assertRaises(ValueError):
                    S.admit(good, pins, lambda p: b'', lambda n, m: b'')
        with mock.patch.object(C, 'reference_data', side_effect=AssertionError('must not project')):
            with self.assertRaises(ValueError):
                S.admit(good, {n: C.compact(b) for n, b in good.items()}, lambda p: b'', lambda n, m: b'{}')

    def test_tail_wrapper_summary_and_owned_stdout_are_separate_original_joins(self):
        for mode in ('wrapper', 'summary', 'stdout', 'policy', 'extra'):
            bodies = originals(copy.deepcopy(self.base))
            if mode == 'wrapper':
                value = json.loads(bodies['wrapper.json']); value['observation']['completed_forwards'] = 40
                bodies['wrapper.json'] = F.wire(value); rejoin(bodies)
            elif mode == 'summary':
                value = json.loads(bodies['summary.json']); value['native_closed'] = False
                bodies['summary.json'] = F.wire(value)
            elif mode == 'stdout':
                value = json.loads(bodies['parent-result.json'])
                value['stdout'] = dict(path='/task/summary.json', **C.compact(bodies['summary.json']))
                bodies['parent-result.json'] = F.wire(value)
            else:
                value = json.loads(bodies['wrapper.json'])
                if mode == 'policy': value['currentness_policy']['native_closed'] = False
                else: value['extra'] = True
                bodies['wrapper.json'] = F.wire(value); rejoin(bodies)
            with self.subTest(mode=mode), self.assertRaises(ValueError): self.attempt(bodies=bodies)
        ordinary = {n: originals(copy.deepcopy(self.base))[n] for n in C.INPUT_NAMES}
        with mock.patch.object(C, 'authenticate_reference', return_value=self.reference):
            with self.assertRaises(ValueError):
                C.admit(ordinary, {n: C.compact(b) for n, b in ordinary.items()},
                        lambda p: self.base[2][p['path']], lambda n, m: b'')

    def test_tail_only_exact_scoped_full_command_and_gpu_requested_flags_are_admitted(self):
        for mode in ('default', 'scoped', 'bank_census', 'readiness', 'basename', 'relative', 'missing_opt', 'extra', 'command_gpu', 'result_gpu'):
            bodies = originals(copy.deepcopy(self.base)); command = json.loads(bodies['parent-command.json'])
            if mode == 'default': command['argv'][3] = '--observe-guarded-full2303'
            elif mode == 'bank_census': command['argv'][3] = '--observe-guarded-full2303-bank-scoped-census-v1'
            elif mode == 'scoped': command['argv'][3] = '--observe-guarded-full2303-scoped-warm'
            elif mode == 'readiness': command['argv'][3] = '--observe-guarded-readiness40-position5-scoped-warm'
            elif mode == 'basename': command['argv'][0] += '-wrong'
            elif mode == 'relative': command['argv'][2] = 'request.json'
            elif mode == 'missing_opt': command['argv'].pop()
            elif mode == 'extra': command['argv'].append('--fallback')
            elif mode == 'command_gpu': command['gpu_execution_requested'] = False
            else:
                result = json.loads(bodies['parent-result.json']); result['gpu_execution_requested'] = False
                bodies['parent-result.json'] = F.wire(result)
            bodies['parent-command.json'] = F.wire(command)
            started = json.loads(bodies['parent-started.json'])
            started['command_sha256'] = C.compact(bodies['parent-command.json'])['sha256']
            bodies['parent-started.json'] = F.wire(started); rejoin(bodies)
            with self.subTest(mode=mode), self.assertRaises(ValueError): self.attempt(bodies=bodies)

    def test_tail_original_policy_unselected_chain_feedback_and_final_close_cannot_be_laundered(self):
        for mode in ('policy', 'chain', 'feedback', 'last_output', 'close', 'capture'):
            value = copy.deepcopy(self.base)
            if mode == 'policy': H.install(value, F.wire(G.policy(value[0])) + b'\n\n')
            elif mode == 'chain': F.change_frame(value, 79, lambda f: f['completion']['chain'].__setitem__(0, f['completion']['chain'][0] ^ 1))
            elif mode == 'feedback': F.change_frame(value, 2048, lambda f: f['request']['command'].__setitem__('token', 7))
            elif mode == 'last_output':
                value[0]['generated_tokens'][-1] = 7
                H.install(value, F.wire(G.policy(value[0])) + b'\n')
            elif mode == 'close': value[0]['close']['native_closed'] = False
            else:
                path = value[0]['files']['captures'][-1]['file']['path']
                value[2][path] = value[2][path][:-1] + bytes([value[2][path][-1] ^ 1])
            with self.subTest(mode=mode), self.assertRaises(ValueError): self.attempt(value=value)

    def test_tail_owned_retirement_and_reference_prompt_model_bundle_are_mandatory(self):
        for mode in ('forced', 'absent', 'ancestry', 'marker', 'started'):
            bodies = originals(copy.deepcopy(self.base)); result = json.loads(bodies['parent-result.json'])
            if mode == 'forced': result['cleanup_signalled'] = True
            elif mode == 'absent': result['owned_groups_absent'] = False
            elif mode == 'ancestry': result['lineage'][1]['identity']['ppid'] = 98
            elif mode == 'marker': bodies['parent-stderr'] += MARKER
            else:
                started = json.loads(bodies['parent-started.json']); started['command_sha256'] = '0' * 64
                bodies['parent-started.json'] = F.wire(started)
            bodies['parent-result.json'] = F.wire(result); rejoin(bodies)
            with self.subTest(mode=mode), self.assertRaises((ValueError, RuntimeError)): self.attempt(bodies=bodies)
        for mode in ('prompt', 'tokens', 'model_id', 'bundle_id'):
            reference = copy.deepcopy(self.reference)
            if mode == 'prompt': reference['prompt_pins']['tokens']['sha256'] = '0' * 64
            elif mode == 'tokens': reference['tokens'][2047] ^= 1
            else: reference['inner'][mode] = '0' * 64
            with self.subTest(mode=mode), self.assertRaises(ValueError): self.attempt(reference=reference)


if __name__ == '__main__':
    unittest.main()
