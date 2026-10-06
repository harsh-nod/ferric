"""JSON semantic joins for an in-memory data validator and its retained result."""
import json
import unittest

same_observation = None


class ObservationRoundtripTests(unittest.TestCase):
    def test_tuple_frontiers_and_u64_survive_json(self):
        checked = dict(final_queue_frontiers=[[(5, 3), (10, 7)]],
            device_ids=[16366993098680759275, 10838076764495710945, 18446744073709551615],
            passed=False)
        retained = json.loads(json.dumps(checked))
        self.assertNotEqual(checked, retained)
        self.assertEqual(retained['device_ids'], checked['device_ids'])
        self.assertTrue(same_observation(checked, retained))
        self.assertTrue(same_observation(checked, dict(reversed(list(retained.items())))))

    def test_changed_numbers_types_or_structure_refuse(self):
        original = dict(frontiers=[(5, 3)], device=18446744073709551615, passed=False)
        for key, value in [('frontiers', [[5, 4]]), ('frontiers', [[5, 3, 0]]),
                           ('device', 18446744073709551614), ('device', float(18446744073709551615)),
                           ('passed', 0), ('frontiers', {'0': [5, 3]})]:
            changed = dict(original); changed[key] = value
            self.assertFalse(same_observation(original, changed))
        self.assertFalse(same_observation(original, dict(original, extra=None)))

    def test_nonfinite_data_refuses(self):
        for value in (float('nan'), float('inf'), float('-inf')):
            with self.assertRaises(ValueError): same_observation({'value': value}, {'value': value})


if __name__ == '__main__':
    from revalidate_tf4_v3 import same_observation
    unittest.main(verbosity=2)
