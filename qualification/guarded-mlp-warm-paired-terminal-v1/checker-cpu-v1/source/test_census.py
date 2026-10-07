"""Synthetic rejection tests for the new closed reusable-arena evidence record."""
import copy
import json
import unittest

import validate_observation as v


def fixture():
    scope = dict(bundle_id=[1] * 32, model_id=[2] * 32, session=[3] * 32,
                 pool_identity=11, group_id=12, child_identity=13)
    part = lambda n: dict(bytes=16, sha256=[n] * 32)
    bootstrap = dict(schema='FerricGuardedMlpReusableAr4BootstrapV1',
        decode=dict(scope=scope, registration=[4] * 32, prefix_image=part(5),
                    tiles_image=part(6), timeout_ms=10000, mode='autoregressive',
                    input_tokens=[9112], device_ids=list(v.IDS)),
        projection_image=part(7), guarded_image=part(8))
    census = dict(schema='FerricGuardedMlpReusableAr4ArenaCensusV1',
        profile_sha256=list(v.profile(bootstrap)), registration=[4] * 32,
        session=[3] * 32, device_ids=list(v.IDS), completed_forwards=4,
        native_closed=True, performance_claim=False,
        allocation_counts=[[715, 711], [751, 747], [787, 783], [787, 783], [787, 783]])
    return bootstrap, census


class CensusTests(unittest.TestCase):
    def setUp(self):
        self.bootstrap, self.census = fixture()

    def check(self, census=None, bootstrap=None):
        return v.arena_census(json.dumps(self.census if census is None else census).encode(),
                              self.bootstrap if bootstrap is None else bootstrap)

    def test_actual_width_ids_and_closed_positive_record(self):
        self.assertEqual(self.check(), self.census)
        self.assertTrue(all(device > 2 ** 53 for device in self.check()['device_ids']))

    def test_missing_extra_or_duplicate_fields(self):
        changed = dict(self.census, extra=1)
        with self.assertRaises(ValueError): self.check(changed)
        changed = dict(self.census)
        del changed['registration']
        with self.assertRaises(ValueError): self.check(changed)
        raw = json.dumps(self.census).encode()
        with self.assertRaises(ValueError):
            v.arena_census(b'{"native_closed":true,' + raw[1:], self.bootstrap)

    def test_missing_extra_or_reordered_samples(self):
        for counts in (self.census['allocation_counts'][:-1],
                       self.census['allocation_counts'] + [[787, 783]],
                       list(reversed(self.census['allocation_counts']))):
            with self.subTest(counts=counts), self.assertRaises(ValueError):
                self.check(dict(self.census, allocation_counts=counts))

    def test_growth_rank_swap_or_non_integer_counts(self):
        for value in (True, 787.0, -1, 2049, 788, 783):
            changed = copy.deepcopy(self.census)
            changed['allocation_counts'][4][0] = value
            with self.subTest(value=value), self.assertRaises(ValueError): self.check(changed)

    def test_stale_profile_registration_session_and_devices(self):
        for key in ('profile_sha256', 'registration', 'session', 'device_ids'):
            changed = copy.deepcopy(self.census)
            changed[key][0] ^= 1
            with self.subTest(key=key), self.assertRaises(ValueError): self.check(changed)

    def test_close_and_claim_flags_are_strict(self):
        for key, value in (('native_closed', False), ('native_closed', 1),
                           ('performance_claim', True), ('performance_claim', 0),
                           ('completed_forwards', 3), ('completed_forwards', True)):
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                self.check(dict(self.census, **{key: value}))

    def test_fresh_schema_teacher_forcing_or_changed_bootstrap_refuse(self):
        for key, value in (('schema', 'FerricGuardedMlpDecodeBootstrapV1'),
                           ('mode', 'teacher_forced'), ('timeout_ms', 9999)):
            changed = copy.deepcopy(self.bootstrap)
            (changed if key == 'schema' else changed['decode'])[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError): self.check(bootstrap=changed)

    def test_bounds_and_nonfinite_json(self):
        for raw in (b'', b' ' * 4097, b'{"allocation_counts":NaN}'):
            with self.subTest(raw=raw[:20]), self.assertRaises(ValueError):
                v.arena_census(raw, self.bootstrap)


if __name__ == '__main__':
    unittest.main(verbosity=2)
