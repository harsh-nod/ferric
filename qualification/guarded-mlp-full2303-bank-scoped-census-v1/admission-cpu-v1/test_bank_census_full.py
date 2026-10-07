"""Synthetic Full bank/census policy mutations; no native or model execution."""
import copy
import hashlib
import unittest

import test_full as F
import test_scoped_full as G
import validate_full as V
import validate_scoped_full as L
import validate_bank_census_full as N


def policy(summary):
    value = G.policy(summary)
    layers = value['counts']
    subset = 16 * 82836
    for field in ('local_checkpoints', 'before_calls', 'after_calls'):
        layers[field] += subset
    layers['generation_probes'] += 2 * subset
    local = 361 * 2301
    value.update(schema='FerricFull2303BankScopedCensusPolicyV1',
        execution_profile='Full2303BankScopedCensusCurrentnessV1',
        counts=dict(layers=layers,
            banks=dict(ordinary_initial_banks=2, scoped_rearms=2301,
                final_generations=[1152, 1151], full_discoveries=4602,
                local_checkpoints=local, before_calls=2 * local + 9204,
                after_calls=2 * local + 9204, generation_probes=2 * local + 6903),
            census=dict(warm_layers=82836, preflights=165672,
                rank_checkpoints=1325376, owner_counts=[1536, 1540])),
        scoped_bank_rearm=True, scoped_capacity_census=True,
        full_entry_exit_per_scoped_bank=True,
        allocation_preflights_outside_windows=False, census_counters_are_layer_subset=True)
    return {name: value[name] for name in N.FIELDS.split()}


def check(value, wrapper=None):
    summary, request, bodies, prompt = value
    raw = F.summary_bytes(summary)
    if wrapper is None:
        wrapper = dict(schema=N.WRAPPER, observation=summary,
                       currentness_policy=V.parse(bodies[summary['files']['child_stderr']['path']]))
    return N.validate(F.wire(wrapper) + b'\n', raw, request, F.DIRECTORY,
                      lambda pin: bodies[pin['path']], prompt)


class BankCensusFullTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ordinary = F.fixture()
        cls.original = copy.deepcopy(cls.ordinary)
        G.install(cls.original, F.wire(policy(cls.original[0])) + b'\n')
        F.summary_bytes(cls.original[0])

    def value(self):
        return copy.deepcopy(self.original)

    def test_complete_full_bank_census_preserves_original_bytes_and_own_history(self):
        value = self.value()
        before = copy.deepcopy(value[2])
        got = check(value)
        self.assertEqual(got['ordinary']['generated_tokens'], [3] * 256)
        self.assertEqual(got['ordinary']['completed_forwards'], 2303)
        self.assertEqual([r['position'] for r in got['ordinary']['captures']], [0, 2047, 2048, 2302])
        self.assertEqual(got['policy']['policy_record'], policy(value[0]))
        self.assertEqual(value[2], before)
        self.assertTrue(got['policy']['census_counters_are_layer_subset'])
        self.assertFalse(got['policy']['owner_counts_runtime_recomputed'])
        for name in ('outer_owned_lineage_checked', 'cpu_qualification_checked',
                     'launch_feasibility_admitted', 'numerical_acceptance', 'performance_claim',
                     'production_authority'):
            self.assertFalse(got[name])

    def test_fixed_callbacks_and_old_ordinary_scoped_routes_remain_closed(self):
        self.assertNotIn('bank_scoped_census_policy', F.check(copy.deepcopy(self.ordinary)))
        value = self.value()
        summary, request, bodies, prompt = value
        for callback in (N.validate_record, lambda raw, row: {}):
            with self.assertRaisesRegex(ValueError, 'only fixed'):
                V.validate(F.summary_bytes(summary), request, F.DIRECTORY,
                           lambda pin: bodies[pin['path']], prompt, _policy_validator=callback)
        for callback in (L.validate_record, lambda raw, row: {}):
            with self.assertRaisesRegex(ValueError, 'only fixed'):
                N._validate_full(F.summary_bytes(summary), request, F.DIRECTORY,
                                 lambda pin: bodies[pin['path']], prompt, _policy_validator=callback)
        with self.assertRaises(ValueError): F.check(self.value())
        with self.assertRaises(ValueError): G.check(self.value())
        old = copy.deepcopy(self.ordinary)
        G.install(old, F.wire(G.policy(old[0])) + b'\n')
        self.assertEqual(G.check(old)['ordinary']['completed_forwards'], 2303)
        with self.assertRaises(ValueError): check(old)

    def test_banks_require_every_extent_generation_count_type_and_checked_formula(self):
        good = policy(self.original[0])['counts']['banks']
        for field in good:
            value = copy.deepcopy(good)
            if field == 'final_generations':
                value[field][0] += 1
            else:
                value[field] += 1
            with self.subTest(field=field), self.assertRaises(ValueError): N.bank_counts(value)
        for field in good:
            if field == 'final_generations': continue
            for bad in (True, float(good[field]), -1, 1 << 64):
                value = copy.deepcopy(good); value[field] = bad
                with self.subTest(field=field, bad=bad), self.assertRaises(ValueError): N.bank_counts(value)
        for generations in ([1151, 1152], [True, 1151], [1152.0, 1151], [1152], [1152, 1151, 1]):
            value = copy.deepcopy(good); value['final_generations'] = generations
            with self.assertRaises(ValueError): N.bank_counts(value)
        value = copy.deepcopy(good); value['local_checkpoints'] = (1 << 64) - 1
        with self.assertRaises(ValueError): N.bank_counts(value)

    def test_census_requires_fixed_subset_and_two_individual_reject_only_owner_counts(self):
        good = policy(self.original[0])['counts']['census']
        for field in ('warm_layers', 'preflights', 'rank_checkpoints'):
            for bad in (good[field] + 1, True, float(good[field]), -1, 1 << 32):
                value = copy.deepcopy(good); value[field] = bad
                with self.assertRaises(ValueError): N.census_counts(value)
        for owners in ([0, 1540], [1536, 0], [2049, 1540], [1536, 2049],
                       [True, 1540], [1536, 1540.0], [1536], [1536, 1540, 1]):
            value = copy.deepcopy(good); value['owner_counts'] = owners
            with self.assertRaises(ValueError): N.census_counts(value)
        value = copy.deepcopy(good); value['owner_counts'] = [1, 2048]
        self.assertEqual(N.census_counts(value), value)

    def test_checked_census_subtraction_refuses_valid_totals_with_invalid_residue(self):
        good = policy(self.original[0])['counts']
        for local, before in ((1325376, 1656720), (1408212, 3147768)):
            value = copy.deepcopy(good)
            value['layers'].update(local_checkpoints=local, before_calls=before, after_calls=before,
                                   generation_probes=2 * local + 3 * 82836)
            self.assertEqual(L.counts(value['layers']), value['layers'])
            with self.assertRaises(ValueError): N.counts(value)
        value = copy.deepcopy(good); value['layers'] = G.policy(self.original[0])['counts']
        self.assertEqual(L.counts(value['layers']), value['layers'])
        with self.assertRaises(ValueError): N.counts(value)

    def test_large_integer_counters_remain_exact_and_nested_keys_are_closed(self):
        value = policy(self.original[0])['counts']
        local = (1 << 53) + 7
        subset = value['census']['rank_checkpoints']
        value['layers'].update(local_checkpoints=local + subset,
            before_calls=local + 4 * 82836 + subset, after_calls=local + 4 * 82836 + subset,
            generation_probes=2 * local + 3 * 82836 + 2 * subset)
        value['banks'].update(local_checkpoints=local, before_calls=2 * local + 9204,
            after_calls=2 * local + 9204, generation_probes=2 * local + 6903)
        self.assertEqual(N.counts(value), value)
        for field in ('layers', 'banks', 'census'):
            bad = copy.deepcopy(value); bad[field]['extra'] = 0
            with self.assertRaises(ValueError): N.counts(bad)
        for field in ('layers', 'banks', 'census'):
            bad = copy.deepcopy(value); bad.pop(field)
            with self.assertRaises(ValueError): N.counts(bad)
        for bad in ({**value, 'extra': 1}, [value], None):
            with self.assertRaises(ValueError): N.counts(bad)

    def test_every_policy_identity_authority_and_scope_field_is_closed(self):
        summary = self.original[0]; good = policy(summary)
        for field, old in good.items():
            if field == 'counts': continue
            value = copy.deepcopy(good)
            if type(old) is bool: value[field] = not old
            elif type(old) is str: value[field] += 'x'
            elif type(old) is int: value[field] += 1
            else: value[field][0] ^= 1
            with self.subTest(field=field), self.assertRaises(ValueError):
                N.validate_record(F.wire(value) + b'\n', summary)

    def test_policy_framing_duplicates_order_and_original_hash_are_required(self):
        summary = self.original[0]; raw = F.wire(policy(summary)) + b'\n'
        reordered = policy(summary); reordered['schema'] = reordered.pop('schema')
        for bad in (b'', raw[:-1], raw + raw, raw + b' ', b' ' * 4097,
                    F.wire(reordered) + b'\n',
                    raw.replace(b'"completed_forwards":2303', b'"completed_forwards":2303.0'),
                    raw.replace(b'"schema":', b'"extra":0,"schema":'),
                    raw.replace(b'"schema":', b'"schema":"duplicate","schema":')):
            with self.assertRaises(ValueError): N.validate_record(bad, summary)
        value = self.value(); path = value[0]['files']['child_stderr']['path']; value[2][path] += b' '
        with self.assertRaisesRegex(ValueError, 'hash'): check(value)

    def test_first_last_own_output_and_exact256_token_extent_are_authenticated(self):
        summary = copy.deepcopy(self.original[0]); raw = F.wire(policy(summary)) + b'\n'
        for position in (0, 255):
            changed = copy.deepcopy(summary); changed['generated_tokens'][position] ^= 1
            with self.assertRaises(ValueError): N.validate_record(raw, changed)
        for tokens in ([3] * 255, [3] * 257, [True] * 256, [151936] * 256):
            changed = copy.deepcopy(summary); changed['generated_tokens'] = tokens
            with self.assertRaises(ValueError): N.validate_record(raw, changed)
        value = self.value(); value[0]['generated_tokens'][-1] ^= 1
        G.install(value, F.wire(policy(value[0])) + b'\n')
        with self.assertRaisesRegex(ValueError, 'Close'): check(value)

    def test_late_original_feedback_page_chain_and_close_cannot_be_relabelled(self):
        for position, edit in (
            (2302, lambda f: f['completion']['chain'].__setitem__(0, f['completion']['chain'][0] ^ 1)),
            (2048, lambda f: f['request']['command'].__setitem__('token', 7)),
            (2302, lambda f: f['request']['command']['cache_metadata'].__setitem__(144, 144)),
        ):
            value = self.value(); F.change_frame(value, position, edit)
            with self.assertRaises(ValueError): check(value)
        value = self.value(); value[0]['close']['native_closed'] = False
        with self.assertRaisesRegex(ValueError, 'Close'): check(value)

    def test_wrapper_and_original_stderr_are_separate_without_policy_substitution(self):
        value = self.value()
        for field in ('schema', 'observation', 'currentness_policy', 'extra'):
            wrapper = dict(schema=N.WRAPPER, observation=copy.deepcopy(value[0]),
                           currentness_policy=policy(value[0]))
            if field == 'schema': wrapper['schema'] = L.WRAPPER
            elif field == 'observation': wrapper['observation']['completed_forwards'] = 40
            elif field == 'currentness_policy': wrapper[field]['counts']['census']['owner_counts'][1] -= 1
            else: wrapper['extra'] = True
            with self.subTest(field=field), self.assertRaises(ValueError): check(value, wrapper)
        value = self.value(); G.install(value, b'')
        wrapper = dict(schema=N.WRAPPER, observation=value[0], currentness_policy=policy(value[0]))
        with self.assertRaises(ValueError): check(value, wrapper)
        with self.assertRaises(ValueError):
            N.validate(b' ' * ((128 << 10) + 1), F.summary_bytes(self.original[0]),
                       self.original[1], F.DIRECTORY, lambda p: self.original[2][p['path']], self.original[3])

    def test_original_deadlines_accounting_capture_bytes_and_health_remain_required(self):
        for field in ('child_exit_zero', 'process_group_absent', 'native_closed'):
            value = self.value(); value[0][field] = False
            with self.assertRaises(ValueError): check(value)
        for bad in (True, 3600001):
            value = self.value(); value[0]['request']['base']['child_deadline_ms'] = bad
            with self.assertRaises(ValueError): check(value)
        value = self.value(); value[0]['files']['bytes_before_summary'] += 1
        with self.assertRaises(ValueError): check(value)
        value = self.value(); path = value[0]['files']['captures'][-1]['file']['path']
        value[2][path] = value[2][path][:-1] + bytes([value[2][path][-1] ^ 1])
        with self.assertRaises(ValueError): check(value)


if __name__ == '__main__':
    unittest.main()
