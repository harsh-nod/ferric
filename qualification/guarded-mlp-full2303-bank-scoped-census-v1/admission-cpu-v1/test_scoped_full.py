"""Synthetic Full extent and compact-policy cases; no native or model execution."""
import copy
import hashlib
import json
import struct
import unittest

import test_full as F
import validate_full as V
import validate_scoped_full as S


def policy(summary):
    b = summary['bootstrap']['sequence']
    return dict(schema='FerricFull2303ScopedWarmPolicyV1',
        execution_profile='Full2303ScopedWarmCurrentnessV1', session=b['scope']['session'],
        device_ids=b['device_ids'], child_pid=b['scope']['child_identity'],
        worker_sha256=summary['request']['base']['worker']['sha256'],
        profile_sha256=summary['profile_sha256'], registration_sha256=b['registration'],
        transcript_sha256=summary['transcript_sha256'], completed_forwards=2303,
        prompt_positions=2048, generated_token_count=256,
        generated_tokens_sha256=list(hashlib.sha256(struct.pack('<256I', *summary['generated_tokens'])).digest()),
        capture_positions=[0, 2047, 2048, 2302], first_scoped_position=2, layers_per_forward=36,
        counts=dict(ordinary_layers=72, scoped_layers=82836, full_discoveries=165672,
            local_checkpoints=414180, before_calls=911196, after_calls=911196,
            generation_probes=1076868), scoped_warm_currentness=True,
        full_entry_exit_per_scoped_layer=True, participant_local_between_boundaries=True,
        scope_includes_prefix_mlp_hidden=True, temporal_equivalent_to_full=False,
        default_group_policy_unchanged=True, shared_full_currentness=False,
        cache_kernel_admission=False, operational_currentness=False, host_observer=False,
        paired_hidden_reads=False, paired_terminal=False, native_closed=True,
        full_long_workload=True, numerical_acceptance=False, performance_claim=False,
        production_authority=False)


def install(value, raw):
    summary, _, bodies, _ = value
    path = summary['files']['child_stderr']['path']
    summary['files']['bytes_before_summary'] += len(raw) - len(bodies[path])
    bodies[path] = raw
    summary['files']['child_stderr'] = F.pin(path, raw)


def check(value, wrapper=None):
    summary, request, bodies, prompt = value
    raw = F.summary_bytes(summary)
    if wrapper is None:
        wrapper = dict(schema='FerricFull2303ScopedWarmObservationV1', observation=summary,
                       currentness_policy=json.loads(bodies[summary['files']['child_stderr']['path']]))
    return S.validate(F.wire(wrapper) + b'\n', raw, request, F.DIRECTORY,
                      lambda pin: bodies[pin['path']], prompt)


class ScopedFullTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ordinary = F.fixture()
        cls.original = copy.deepcopy(cls.ordinary)
        install(cls.original, F.wire(policy(cls.original[0])) + b'\n')
        F.summary_bytes(cls.original[0])

    def value(self):
        return copy.deepcopy(self.original)

    def test_complete_full_scope_uses_original_policy_and_own256_history(self):
        value = self.value()
        original = value[2][value[0]['files']['child_stderr']['path']]
        checked = check(value)
        self.assertEqual(checked['ordinary']['completed_forwards'], 2303)
        self.assertEqual(checked['ordinary']['generated_tokens'], [3] * 256)
        self.assertEqual([c['position'] for c in checked['ordinary']['captures']], [0, 2047, 2048, 2302])
        self.assertEqual(checked['policy']['policy_record'], policy(value[0]))
        self.assertEqual(original, value[2][value[0]['files']['child_stderr']['path']])
        self.assertTrue(checked['policy']['no_policy_bytes_discarded'])
        self.assertEqual(checked['policy']['discovery_count_scope'], 'scoped-layer-windows-only')
        for name in ('outer_owned_lineage_checked', 'cpu_qualification_checked',
                     'launch_feasibility_admitted', 'numerical_acceptance', 'performance_claim'):
            self.assertFalse(checked[name])

    def test_ordinary_empty_stderr_and_closed_callback_identity_are_preserved(self):
        self.assertNotIn('scoped_warm_policy', F.check(copy.deepcopy(self.ordinary)))
        with self.assertRaises(ValueError):
            F.check(self.value())
        value = self.value()
        summary, request, bodies, prompt = value
        with self.assertRaisesRegex(ValueError, 'only fixed'):
            V.validate(F.summary_bytes(summary), request, F.DIRECTORY, lambda p: bodies[p['path']],
                       prompt, _policy_validator=lambda raw, o: {})
        install(value, b'')
        wrapper = dict(schema=S.WRAPPER, observation=summary, currentness_policy=policy(summary))
        with self.assertRaises(ValueError):
            check(value, wrapper)

    def test_compact_counters_refuse_drift_overflow_bool_and_preserve_large_integers(self):
        good = policy(self.original[0])['counts']
        for field in good:
            for changed in (True, float(good[field]), good[field] + 1):
                value = dict(good); value[field] = changed
                with self.assertRaises(ValueError): S.counts(value)
        for local in (0, (1 << 64) - 1):
            value = dict(good); value['local_checkpoints'] = local
            with self.assertRaises(ValueError): S.counts(value)
        value = dict(good)
        local = (1 << 53) + 7
        value.update(local_checkpoints=local, before_calls=local + 4 * 82836,
                     after_calls=local + 4 * 82836, generation_probes=2 * local + 3 * 82836)
        self.assertEqual(S.counts(value), value)
        value['scoped_layers_by_forward'] = [0, 0] + [36] * 2301
        with self.assertRaises(ValueError): S.counts(value)

    def test_every_policy_authority_and_identity_field_is_closed(self):
        summary = self.original[0]
        good = policy(summary)
        for field, old in good.items():
            if field == 'counts':
                continue
            value = copy.deepcopy(good)
            if type(old) is bool: value[field] = not old
            elif type(old) is str: value[field] += 'x'
            elif type(old) is int: value[field] += 1
            else: value[field][0] ^= 1
            with self.assertRaises(ValueError, msg=field):
                S.validate_record(F.wire(value) + b'\n', summary)

    def test_single_exact_record_refuses_framing_duplicates_and_type_substitution(self):
        summary = self.original[0]
        raw = F.wire(policy(summary)) + b'\n'
        for bad in (b'', raw[:-1], raw + raw, raw + b' ', b' ' * 4097,
                    raw.replace(b'"completed_forwards":2303', b'"completed_forwards":2303.0'),
                    raw.replace(b'"completed_forwards":2303', b'"completed_forwards":true'),
                    raw.replace(b'"schema":', b'"extra":0,"schema":'),
                    raw.replace(b'"schema":', b'"schema":"duplicate","schema":')):
            with self.assertRaises(ValueError):
                S.validate_record(bad, summary)

    def test_own_history_hash_refuses_first_last_short_and_out_of_vocab(self):
        summary = copy.deepcopy(self.original[0])
        raw = F.wire(policy(summary)) + b'\n'
        for position in (0, 255):
            changed = copy.deepcopy(summary); changed['generated_tokens'][position] ^= 1
            with self.assertRaises(ValueError): S.validate_record(raw, changed)
        for tokens in ([3] * 255, [3] * 257, [True] * 256, [151936] * 256):
            changed = copy.deepcopy(summary); changed['generated_tokens'] = tokens
            with self.assertRaises(ValueError): S.validate_record(raw, changed)
        value = self.value()
        value[0]['generated_tokens'][255] ^= 1
        install(value, F.wire(policy(value[0])) + b'\n')
        with self.assertRaisesRegex(ValueError, 'Close'):
            check(value)

    def test_late_original_records_feedback_pages_and_chain_remain_required(self):
        for position, edit in (
            (40, lambda f: f['completion']['chain'].__setitem__(0, f['completion']['chain'][0] ^ 1)),
            (2048, lambda f: f['request']['command'].__setitem__('token', 7)),
            (2302, lambda f: f['request']['command']['cache_metadata'].__setitem__(144, 144)),
        ):
            value = self.value()
            F.change_frame(value, position, edit)
            with self.assertRaises(ValueError): check(value)

    def test_wrapper_cannot_substitute_observation_policy_or_profile(self):
        value = self.value()
        summary = value[0]
        for mutation in ('schema', 'observation', 'currentness_policy', 'extra'):
            wrapper = dict(schema=S.WRAPPER, observation=copy.deepcopy(summary),
                           currentness_policy=policy(summary))
            if mutation == 'schema': wrapper['schema'] = 'FerricReadiness40Position5ScopedWarmObservationV1'
            elif mutation == 'observation': wrapper['observation']['completed_forwards'] = 40
            elif mutation == 'currentness_policy': wrapper['currentness_policy']['native_closed'] = False
            else: wrapper['extra'] = True
            with self.assertRaises(ValueError): check(value, wrapper)
        for profile in ('readiness40', 'readiness40_position5'):
            changed = copy.deepcopy(summary); changed['bootstrap']['sequence']['profile'] = profile
            with self.assertRaises(ValueError):
                S.validate_record(F.wire(policy(summary)) + b'\n', changed)

    def test_original_hashes_budgets_deadlines_and_retirement_flags_remain_required(self):
        for field in ('child_exit_zero', 'process_group_absent', 'native_closed'):
            value = self.value(); value[0][field] = False
            with self.assertRaises(ValueError): check(value)
        value = self.value(); value[0]['files']['bytes_before_summary'] += 1
        with self.assertRaises(ValueError): check(value)
        value = self.value(); value[0]['request']['base']['child_deadline_ms'] = 3600001
        with self.assertRaises(ValueError): check(value)
        value = self.value(); path = value[0]['files']['child_stderr']['path']; value[2][path] += b' '
        with self.assertRaises(ValueError): check(value)


if __name__ == '__main__':
    unittest.main()
