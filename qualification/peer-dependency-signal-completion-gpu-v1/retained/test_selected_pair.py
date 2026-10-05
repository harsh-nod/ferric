"""Selected-pair audit fixtures; no GPU discovery or native execution."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('selected_pair_gpu', Path(__file__).with_name('run_gpu.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)


def encoded(value):
    return json.dumps(value).encode()


def inventory():
    rows = [dict(gpu=index, bdf=f'0000:{index + 128:02x}:00.0',
                 uuid=f'{index:08x}-0000-1000-8000-{index:012x}',
                 kfd_id=100 + index, node_id=20 + index, partition_id=0) for index in range(8)]
    rows[1] = dict(gpu=1, bdf='0000:15:00.0', uuid='96ff75a0-0000-1000-8068-95650c2e8ae1',
                   kfd_id=22482, node_id=3, partition_id=0)
    rows[2] = dict(gpu=2, bdf='0000:65:00.0', uuid='d4ff75a0-0000-1000-80e5-658294b0b967',
                   kfd_id=46071, node_id=4, partition_id=0)
    return rows


def processes():
    rows = [dict(gpu=index, process_list=[{'process_info': 'No running processes detected'}])
            for index in range(8)]
    rows[0]['process_list'] = [{'pid': 12345, 'name': 'python', 'vram_usage': 1048576}]
    return rows


def body(node, name, text):
    raw = text.encode('ascii')
    return dict(path=str(R.SYSFS / str(node) / name),
                resolved_path=f'/sys/devices/virtual/kfd/kfd/topology/nodes/{node}/{name}',
                bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest(), text=text)


def snapshot():
    result = {}
    for node, gpu_id, uid, location, render in [
            (3, 22482, 10838076764495710945, 5376, 136),
            (4, 46071, 15340779317222226279, 25856, 144)]:
        text = (f'unique_id {uid}\ndomain 0\nlocation_id {location}\n'
                f'drm_render_minor {render}\ngfx_target_version 90500\nsimd_count 1024\n'
                'wave_front_size 64\n')
        result[str(node)] = dict(gpu_id=body(node, 'gpu_id', str(gpu_id) + '\n'),
                                 properties=body(node, 'properties', text))
    return result


class SelectedPairTests(unittest.TestCase):
    def test_actual_inventory_indices_bind_bdf_uuid_kfd_and_node(self):
        rows = inventory()
        self.assertEqual(R.inventory_rows(encoded(rows)), rows)
        self.assertEqual(R.inventory_rows(encoded(list(reversed(rows)))), list(reversed(rows)))
        self.assertEqual([row['gpu'] for row in R.SELECTED], [1, 2])

    def test_inventory_refuses_missing_duplicate_unknown_or_noninteger_rows(self):
        cases = []
        rows = inventory(); rows.pop(); cases.append(rows)
        rows = inventory(); rows[7] = copy.deepcopy(rows[6]); cases.append(rows)
        rows = inventory(); rows[0]['unrecognized'] = 0; cases.append(rows)
        rows = inventory(); rows[0]['gpu'] = False; cases.append(rows)
        rows = inventory(); rows[0]['kfd_id'] = 100.0; cases.append(rows)
        rows = inventory(); rows[7]['bdf'] = rows[6]['bdf']; cases.append(rows)
        for rows in cases:
            with self.subTest(rows=rows), self.assertRaises(RuntimeError):
                R.inventory_rows(encoded(rows))

    def test_inventory_refuses_every_selected_identity_change(self):
        for index in [1, 2]:
            for key, value in [('bdf', '0000:ff:00.0'), ('uuid', 'ffffffff-0000-1000-8000-000000000000'),
                               ('kfd_id', 99999), ('node_id', 999), ('partition_id', 1)]:
                rows = inventory(); rows[index][key] = value
                with self.subTest(index=index, key=key), self.assertRaisesRegex(RuntimeError, 'selected AMD-SMI'):
                    R.inventory_rows(encoded(rows))

    def test_foreign_work_is_retained_while_selected_pair_is_exact_idle(self):
        rows = processes()
        self.assertEqual(R.process_rows(encoded(rows)), rows)
        for index in [3, 4, 5, 6, 7]:
            rows[index]['process_list'] = [{'pid': index, 'name': 'foreign'}]
        self.assertEqual(R.process_rows(encoded(rows)), rows)
        self.assertEqual(rows[0]['process_list'][0]['pid'], 12345)

    def test_process_roster_never_hides_busy_or_missing_selected_gpu(self):
        for index in [1, 2]:
            for items in [[{'pid': 123, 'name': 'python'}], [],
                          [{'process_info': 'No running processes detected', 'extra': 0}]]:
                rows = processes(); rows[index]['process_list'] = items
                with self.subTest(index=index, items=items), self.assertRaises(RuntimeError):
                    R.process_rows(encoded(rows))
        rows = processes(); rows.pop()
        with self.assertRaises(RuntimeError):
            R.process_rows(encoded(rows))
        rows = processes(); rows[7]['gpu'] = 0
        with self.assertRaises(RuntimeError):
            R.process_rows(encoded(rows))

    def test_sysfs_join_preserves_full_u64_and_derives_selected_bdfs(self):
        identities = R.selected_identity(R.inventory_rows(encoded(inventory())), snapshot())
        self.assertEqual([row['unique_id'] for row in identities],
                         [10838076764495710945, 15340779317222226279])
        self.assertEqual([row['bdf'] for row in identities], ['0000:15:00.0', '0000:65:00.0'])
        self.assertEqual([row['node_id'] for row in identities], [3, 4])

    def test_sysfs_join_refuses_gpu_id_and_each_bound_property_change(self):
        for node in [3, 4]:
            changed = snapshot(); changed[str(node)]['gpu_id'] = body(node, 'gpu_id', '1\n')
            with self.assertRaisesRegex(RuntimeError, 'gpu_id differ'):
                R.selected_identity(inventory(), changed)
            for key in ['unique_id', 'domain', 'location_id', 'drm_render_minor', 'gfx_target_version', 'simd_count']:
                changed = snapshot(); original = changed[str(node)]['properties']['text']
                lines = ['%s 1' % key if line.startswith(key + ' ') else line for line in original.splitlines()]
                changed[str(node)]['properties'] = body(node, 'properties', '\n'.join(lines) + '\n')
                with self.subTest(node=node, key=key), self.assertRaisesRegex(RuntimeError, 'property identity'):
                    R.selected_identity(inventory(), changed)

    def test_sysfs_snapshot_refuses_malformed_duplicate_or_unbound_bodies(self):
        for text in ['unique_id 1\nunique_id 1\n', 'unique_id -1\n', 'bad line extra\n']:
            changed = snapshot(); changed['3']['properties'] = body(3, 'properties', text)
            with self.assertRaisesRegex(RuntimeError, 'malformed/duplicate'):
                R.selected_identity(inventory(), changed)
        for key, value in [('bytes', 4096), ('sha256', '0' * 64), ('path', '/wrong/properties')]:
            changed = snapshot(); changed['3']['properties'][key] = value
            with self.subTest(key=key), self.assertRaisesRegex(RuntimeError, 'sysfs body'):
                R.selected_identity(inventory(), changed)
        changed = snapshot(); del changed['4']
        with self.assertRaisesRegex(RuntimeError, 'node roster'):
            R.selected_identity(inventory(), changed)

    def test_audit_retains_foreign_rows_and_same_selected_identity_on_both_sides(self):
        saved = {}
        def save(name, value):
            saved[name] = copy.deepcopy(value)
        with patch.object(R, 'owned', side_effect=[encoded(inventory()), encoded(processes())] * 2) as owned, \
                patch.object(R, 'sysfs_snapshot', side_effect=[snapshot(), snapshot()]), \
                patch.object(R, 'save', side_effect=save):
            before = R.gpu_audit('before', [])
            after = R.gpu_audit('after', [])
        self.assertEqual(before['selected_identity'], after['selected_identity'])
        self.assertFalse(before['global_idle_required'])
        self.assertEqual(before['foreign_process_rows'][0], processes()[0])
        self.assertEqual(len(before['inventory']), 8)
        self.assertEqual(len(before['processes']), 8)
        self.assertEqual([call.args[0] for call in owned.call_args_list],
                         ['before-inventory', 'before-process', 'after-inventory', 'after-process'])
        self.assertEqual(set(saved), {'before-sysfs.json', 'before-audit.json', 'after-sysfs.json', 'after-audit.json'})

    def test_sysfs_reader_uses_actual_length_and_does_not_track_immutable_input(self):
        with tempfile.TemporaryDirectory() as temp, patch.multiple(R, INPUTS={}, HARD_DEADLINE=float('inf')):
            path = Path(temp).resolve() / 'gpu_id'; path.write_bytes(b'22482\n')
            observed = R.sysfs_body(path)
            self.assertEqual(observed['bytes'], 6)
            self.assertEqual(observed['sha256'], hashlib.sha256(b'22482\n').hexdigest())
            self.assertEqual(R.INPUTS, {})
            path.write_bytes(b'x' * 8193)
            with self.assertRaisesRegex(RuntimeError, 'exceeds bound'):
                R.sysfs_body(path)
        with patch.object(R, 'HARD_DEADLINE', 0), self.assertRaisesRegex(RuntimeError, 'deadline before'):
            R.sysfs_body(Path('/unopened'))

    def test_request_preserves_exact_selected_uids_and_unchanged_native_scope(self):
        value = R.parse(Path(__file__).with_name('request.json').read_bytes())
        self.assertEqual([row['unique_id'] for row in value['devices']],
                         [10838076764495710945, 15340779317222226279])
        self.assertEqual(value['schema'], 'ferric-peer-dependency-native-request-v228-v3')
        self.assertEqual(value['timeout_ms'], 60000)
        self.assertIs(value['witness'], True)
        self.assertEqual(value['artifact'], {'path': str(R.IMAGE), 'sha256': R.IMAGE_SHA})

    def test_extra_audits_have_explicit_reserve_without_raising_outer_cap(self):
        self.assertEqual(R.WHOLE_WALL, 900)
        self.assertEqual(R.AUDIT_WALL, 60)
        self.assertEqual(R.SYSFS_WALL, 5)
        self.assertEqual(R.POST_NATIVE_RESERVE, 260)
        self.assertEqual(600 + R.POST_NATIVE_RESERVE, 860)


if __name__ == '__main__':
    unittest.main()
