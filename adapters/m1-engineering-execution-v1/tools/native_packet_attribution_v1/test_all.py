"""Flat staged harness includes byte-pinned native_wait_attribution_v1 fixtures."""
import importlib.util
import json
from pathlib import Path
import unittest


def load(name):
    path = Path(__file__).with_name(name + '.py')
    if not path.exists():
        path = path.parent.parent / 'native_wait_attribution_v1' / path.name
    value = importlib.util.module_from_spec(importlib.util.spec_from_file_location(name, path))
    value.__spec__.loader.exec_module(value)
    return value


class IntegrationTests(unittest.TestCase):
    def test_packet_wait_controller_binding(self):
        legacy, packet = load('test_capture_binding'), load('test_packet_observation')
        check, raw, _ = legacy.fixture()
        rows, _ = packet.campaign()
        for row in rows:
            row['device_unique_id'] = 16366993098680759275
        stream = raw + b''.join(json.dumps(r).encode() + b'\n' for r in rows)
        replay = packet.p.Replay(check.legacy, legacy.b, legacy.f.w)
        value = replay.replay(stream, {'arm': 'A'}, {}, None)
        self.assertTrue(value['accepted'])
        self.assertTrue(value['legacy']['wait']['controller_spans_bound'])
        rows[4]['next_write'] += 1
        stream = raw + b''.join(json.dumps(r).encode() + b'\n' for r in rows)
        with self.assertRaises(ValueError): replay.replay(stream, {'arm': 'A'}, {}, None)


if __name__ == '__main__':
    legacy, packets = load('test_capture_binding'), load('test_packet_observation')
    suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromTestCase(cls) for cls in (
        legacy.f.WaitObservationTests, legacy.CaptureBindingTests, packets.PacketTests, IntegrationTests))
    raise SystemExit(0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1)
