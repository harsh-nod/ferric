"""Offline packet-profile accounting, privacy, rejection and SVG structure gates."""
import copy
import csv
import io
import json
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET

import target_gpu_profile_v1 as p


def fixtures():
    roster = [{'loaded_kernel_ordinal': i, 'kernel_symbol': f'kernel_{i}', 'kernarg_bytes': 352} for i in range(1, 20)]
    packets, forwards = [], []
    for generation in range(1, 37):
        for slot in range(616):
            layer, operation, family = p.stage(slot)
            ordinal = slot % 19 + 1
            values = [generation, p.phase(generation), slot, layer, operation, family, ordinal,
                      f'kernel_{ordinal}', 352, 64, 64, slot * 10, (slot + 1) * 10, 10**9, 10]
            packets.append(dict(zip(p.PACKET_FIELDS, map(str, values), strict=True)))
        values = [generation, p.phase(generation), 616, 6160, 6160, 8000, 10**9, 'a' * 64]
        forwards.append(dict(zip(p.FORWARD_FIELDS, map(str, values), strict=True)))
    return packets, forwards, roster


class ProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw, cls.forward_raw, cls.roster = fixtures()
        cls.packets, cls.forwards = p.checked_data(cls.raw, cls.forward_raw, cls.roster)

    def test_exact_phase_boundaries_and_schedule(self):
        self.assertEqual([p.phase(i) for i in range(1, 37)].count('prompt'), 5)
        self.assertEqual(p.stage(613), ('', 'final_norm', 'normalization'))
        self.assertEqual(p.stage(612), ('35', 'mlp_residual', 'residual add'))
        for value in (True, 0, 37):
            with self.assertRaises(ValueError): p.phase(value)
        for value in (True, -1, 616):
            with self.assertRaises(ValueError): p.stage(value)

    def test_complete_roster_and_phase_rejections(self):
        for rows in (self.raw[:-1], self.raw + self.raw[:1], list(reversed(self.raw))):
            with self.assertRaises(ValueError): p.checked_data(rows, self.forward_raw, self.roster)
        rows = copy.deepcopy(self.raw); rows[5 * 616]['phase'] = 'prompt'
        with self.assertRaises(ValueError): p.checked_data(rows, self.forward_raw, self.roster)
        with self.assertRaises(ValueError): p.checked_data(self.raw, self.forward_raw[:-1], self.roster)

    def test_closed_schema_no_absolute_timestamps_or_private_fields(self):
        for key in ('pid', 'host_path', 'raw_start_gpu_ticks', 'start_system_ticks', 'extra'):
            rows = self.raw.copy(); rows[0] = dict(rows[0], **{key: '1'})
            with self.assertRaises(ValueError): p.checked_data(rows, self.forward_raw, self.roster)

    def test_numeric_root_abi_geometry_and_interval_rejections(self):
        changes = [('duration_ns_floor', 'NaN'), ('duration_ns_floor', '0'), ('duration_ns_floor', '12'),
                   ('start_system_ticks_from_forward_start', '-1'), ('end_system_ticks_from_forward_start', '0'),
                   ('system_frequency_hz', '0'), ('kernel_symbol', 'wrong'), ('kernarg_bytes', '96'),
                   ('loaded_kernel_ordinal', '20'), ('workgroup_x', '32'), ('grid_x', '65'), ('slot', '01')]
        for key, value in changes:
            rows = self.raw.copy(); rows[0] = dict(rows[0], **{key: value})
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                p.checked_data(rows, self.forward_raw, self.roster)

    def test_sums_spans_payloads_and_host_domain_rejections(self):
        for key, value in [('sum_packet_intervals_ns', '6159'), ('packet_interval_span_ns_floor', '6161'),
                           ('instrumented_host_batch_elapsed_ns', '0'), ('payload_sha256', 'bad'),
                           ('packet_count', '615'), ('system_frequency_hz', '999999999')]:
            rows = self.forward_raw.copy(); rows[0] = dict(rows[0], **{key: value})
            with self.subTest(key=key), self.assertRaises(ValueError): p.checked_data(self.raw, rows, self.roster)

    def test_floor_difference_is_not_fabricated_sum(self):
        rows = self.raw.copy(); rows[0] = dict(rows[0], duration_ns_floor='9')
        forwards = self.forward_raw.copy(); forwards[0] = dict(forwards[0], sum_packet_intervals_ns='6159')
        packets, checked = p.checked_data(rows, forwards, self.roster)
        self.assertEqual(packets[0]['duration_ns_floor'], 9)
        self.assertEqual(checked[0]['packet_interval_span_ns_floor'], 6160)

    def test_normalized_counts_and_all_phase_sums(self):
        operations = p.aggregates(self.packets, 'operation')
        families = p.aggregates(self.packets, 'family')
        for scope, expected in [('all', 22176), ('prompt', 3080), ('decode', 19096)]:
            rows = [row for row in families if row['phase'] == scope]
            self.assertEqual(sum(row['packet_count'] for row in rows), expected)
            self.assertEqual(sum(row['sum_packet_intervals_ns'] for row in rows), expected * 10)
            self.assertEqual(sum(row['packets_per_forward'] for row in rows), 616)
        norms = p.summary(self.forwards, operations)['normalization_shapes']
        self.assertEqual([row['packets_per_forward'] for row in norms], [73, 73, 73, 72, 72, 72])

    def test_svg_all_points_and_exact_sequential_timeline(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            p.forward_plot(root / 'forward.svg', self.forwards)
            p.timeline_plot(root / 'timeline.svg', self.packets)
            p.family_plot(root / 'family.svg', p.aggregates(self.packets, 'family'))
            forward = ET.parse(root / 'forward.svg').getroot()
            points = [node for node in forward.iter() if node.tag.endswith('circle')]
            self.assertEqual(len(points), 108)
            for key in ('packet_interval_span_ns_floor', 'sum_packet_intervals_ns', 'instrumented_host_batch_elapsed_ns'):
                self.assertEqual([int(node.attrib['data-generation']) for node in points if node.attrib['data-series'] == key], list(range(1, 37)))
            timeline = ET.parse(root / 'timeline.svg').getroot()
            rectangles = [node for node in timeline.iter() if 'data-slot' in node.attrib]
            self.assertEqual([int(node.attrib['data-slot']) for node in rectangles], list(range(616)))
            self.assertTrue(all(node.attrib['data-generation'] == '6' and float(node.attrib['width']) > 0 for node in rectangles))
            self.assertIn('NOT parallel lanes', (root / 'timeline.svg').read_text())
            families = [node for node in ET.parse(root / 'family.svg').getroot().iter() if 'data-family' in node.attrib]
            self.assertEqual(len(families), 26)

    def test_bounded_reads_and_duplicate_json(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name); (root / 'a').write_bytes(b'123'); (root / 'link').symlink_to(root / 'a')
            self.assertEqual(p.bounded(root / 'a', 3), b'123')
            with self.assertRaises(ValueError): p.bounded(root / 'a', 2)
            with self.assertRaises(OSError): p.bounded(root / 'link')
        for data in (b'{"x":1,"x":2}', b'{"x":NaN}'):
            with self.assertRaises(ValueError): p.decode(data)

    def test_committed_assets_reproduce_and_contain_no_private_identifiers(self):
        assets = Path(__file__).resolve().parents[1] / 'docs/assets/asrock-target8b-gpu-profile-v1'
        if not assets.is_dir():
            self.skipTest('publication assets not exported yet')
        with tempfile.TemporaryDirectory() as name:
            output = Path(name) / 'regenerated'; p.render(assets, output)
            self.assertEqual({path.name for path in assets.iterdir()}, {path.name for path in output.iterdir()})
            for path in assets.iterdir():
                self.assertEqual(path.read_bytes(), (output / path.name).read_bytes(), path.name)
                content = path.read_text()
                for forbidden in ('/home/', '/tmp/', '/proc/', 'harmenon', '1956390207832604050', '"pid"', 'worker_pid'):
                    self.assertNotIn(forbidden, content, path.name)
            summary = json.loads((output / 'summary.json').read_text())
            self.assertEqual(summary['all_packets'], 22176)
            self.assertFalse(summary['gpu_overlap_measured'])


if __name__ == '__main__': unittest.main(verbosity=2)
