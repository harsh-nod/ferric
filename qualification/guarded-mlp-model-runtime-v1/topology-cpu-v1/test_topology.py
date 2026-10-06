"""Pure regression for sysfs KFD handles versus selected device unique IDs."""
import unittest

import run_model_gpu as R


def fixture():
    return dict(host='smci350-rck-g03-b19-03', boot='00000000-0000-0000-0000-000000000001', devices=[
        R.parse_topology_node(node, f'unique_id {identity}\ngfx_target_version 90500\nvendor_id 4098\n', f'{handle}\n')
        for node, identity, handle in zip((2, 3), R.IDS, (39903, 22482))])


class TopologyTests(unittest.TestCase):
    def test_actual_kfd_handles_are_not_unique_ids(self):
        value = fixture(); R.validate_topology(value)
        self.assertEqual([r['gpu_id'] for r in value['devices']], [39903, 22482])
        self.assertEqual([int(r['properties']['unique_id']) for r in value['devices']], R.IDS)
        self.assertNotEqual([r['gpu_id'] for r in value['devices']], R.IDS)

    def test_swapped_or_small_unique_ids_refuse(self):
        for identities in (list(reversed(R.IDS)), [39903, 22482], [R.IDS[0], R.IDS[0]]):
            value = fixture()
            for row, identity in zip(value['devices'], identities): row['properties']['unique_id'] = str(identity)
            with self.assertRaises(RuntimeError): R.validate_topology(value)
        value = fixture(); value['devices'].reverse()
        with self.assertRaises(RuntimeError): R.validate_topology(value)

    def test_wrong_gfx_and_invalid_handle_refuse(self):
        for target in ('95000', '90400', '0'):
            value = fixture(); value['devices'][0]['properties']['gfx_target_version'] = target
            with self.assertRaises(RuntimeError): R.validate_topology(value)
        for handle in (0, -1, True):
            value = fixture(); value['devices'][0]['gpu_id'] = handle
            with self.assertRaises(RuntimeError): R.validate_topology(value)

    def test_malformed_or_duplicate_properties_refuse(self):
        good = f'unique_id {R.IDS[0]}\ngfx_target_version 90500\n'
        for raw in ('', 'unique_id\n', good + 'unique_id 123\n', 'unique_id 1 2\n',
                    'gfx_target_version 90500\n', 'unique_id xyz\ngfx_target_version 90500\n',
                    'unique_id 1\ngfx_target_version unknown\n', good + 'x' * 16384):
            with self.assertRaises(RuntimeError): R.parse_topology_node(2, raw, '39903\n')
        for raw in ('', 'abc', '-1', '1 2', '1' * 65):
            with self.assertRaises(RuntimeError): R.parse_topology_node(2, good, raw)


if __name__ == '__main__':
    unittest.main(verbosity=2)
