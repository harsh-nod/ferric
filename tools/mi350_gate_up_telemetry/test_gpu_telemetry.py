import importlib.util
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('gpu_telemetry', Path(__file__).with_name('gpu_telemetry.py'))
t = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(t)


class TelemetryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.device = self.root / 'devices/pci0000:00' / t.PCI
        self.device.mkdir(parents=True)
        self.link = self.root / 'class/drm/renderD128/device'
        self.link.parent.mkdir(parents=True)
        self.link.symlink_to(self.device, target_is_directory=True)
        self.write(self.device / 'unique_id', t.UNIQUE_ID + '\n')
        for name in t.DEVICE_FIELDS:
            self.write(self.device / name, '0\n')
        self.hwmon = self.device / 'hwmon/hwmon99'
        self.hwmon.mkdir(parents=True)
        self.write(self.hwmon / 'name', 'amdgpu\n')
        for name in t.HWMON_FIELDS:
            self.write(self.hwmon / name, '123\n')

    @staticmethod
    def write(path, text):
        path.write_text(text)

    def test_complete_raw_sample(self):
        value = t.snapshot(self.root)
        self.assertEqual(value['unique_id'], t.UNIQUE_ID)
        self.assertEqual(value['hwmon'], str(self.hwmon))
        self.assertEqual(set(value['device_fields']), set(t.DEVICE_FIELDS))
        self.assertEqual(set(value['hwmon_fields']), set(t.HWMON_FIELDS))
        self.assertEqual(value['hwmon_fields']['freq1_input'], {'status': 'ok', 'text': '123\n'})
        self.assertLess(value['started_monotonic_ns'], value['finished_monotonic_ns'])
        self.assertFalse(value['atomic_snapshot'])
        self.assertFalse(value['gpu_time_measured'])
        self.assertFalse(value['throttle_state_measured'])

    def test_schedule_includes_read_deadlines(self):
        t.validate_schedule(1200, 1)
        with self.assertRaisesRegex(ValueError, 'read deadlines'):
            t.validate_schedule(1200, 3)

    def test_invalid_schedules(self):
        for samples, interval in ((0, 1), (1201, 1), (1, 0), (1, 11),
                                  (1, float('nan')), (1, float('inf'))):
            with self.subTest(samples=samples, interval=interval):
                with self.assertRaises(ValueError):
                    t.validate_schedule(samples, interval)

    def test_unavailable_is_not_zero(self):
        (self.hwmon / 'power1_average').unlink()
        self.assertEqual(t.snapshot(self.root)['hwmon_fields']['power1_average'], {'status': 'unavailable'})

    def test_required_identity_missing(self):
        (self.device / 'unique_id').unlink()
        with self.assertRaises(FileNotFoundError):
            t.snapshot(self.root)

    def test_wrong_gpu(self):
        self.write(self.device / 'unique_id', '0\n')
        with self.assertRaisesRegex(ValueError, 'unique identity'):
            t.snapshot(self.root)

    def test_wrong_pci(self):
        wrong = self.device.with_name('0000:06:00.0')
        wrong.mkdir()
        self.link.unlink()
        self.link.symlink_to(wrong, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'PCI identity'):
            t.snapshot(self.root)

    def test_escape(self):
        self.link.unlink()
        self.link.symlink_to(Path('/tmp'), target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'PCI identity'):
            t.snapshot(self.root)

    def test_attribute_symlink(self):
        path = self.device / 'gpu_busy_percent'
        path.unlink()
        path.symlink_to(self.device / 'mem_busy_percent')
        with self.assertRaisesRegex(ValueError, 'canonical text'):
            t.snapshot(self.root)

    def test_non_regular_attribute(self):
        path = self.device / 'gpu_busy_percent'
        path.unlink()
        path.mkdir()
        with self.assertRaisesRegex(ValueError, 'canonical text'):
            t.snapshot(self.root)

    def test_oversized_attribute(self):
        self.write(self.device / 'gpu_busy_percent', '1' * 4097)
        with self.assertRaisesRegex(ValueError, 'canonical text'):
            t.snapshot(self.root)

    def test_nul_attribute(self):
        self.write(self.device / 'gpu_busy_percent', '\x00')
        with self.assertRaisesRegex(ValueError, 'NUL'):
            t.snapshot(self.root)

    def test_non_ascii_attribute(self):
        (self.device / 'gpu_busy_percent').write_bytes(b'\xff')
        with self.assertRaises(UnicodeDecodeError):
            t.snapshot(self.root)

    def test_ambiguous_hwmon(self):
        other = self.hwmon.with_name('hwmon100')
        other.mkdir()
        self.write(other / 'name', 'amdgpu\n')
        with self.assertRaisesRegex(ValueError, 'one amdgpu'):
            t.snapshot(self.root)

    def test_missing_amdgpu_hwmon(self):
        self.write(self.hwmon / 'name', 'different\n')
        with self.assertRaisesRegex(ValueError, 'one amdgpu'):
            t.snapshot(self.root)

    def test_hwmon_symlink(self):
        (self.hwmon.parent / 'hwmon100').symlink_to(self.hwmon, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'real hwmon'):
            t.snapshot(self.root)

    def test_hwmon_discovery_bound(self):
        for index in range(8):
            other = self.hwmon.with_name('hwmon' + str(index))
            other.mkdir()
            self.write(other / 'name', 'other\n')
        with self.assertRaisesRegex(ValueError, 'bounded hwmon'):
            t.snapshot(self.root)

    def test_deadline(self):
        with self.assertRaisesRegex(ValueError, 'deadline'):
            t.read_attribute(self.device / 'unique_id', self.device, time.monotonic() - 1)

    def test_permission_failure_not_unavailable(self):
        with mock.patch.object(t.os, 'open', side_effect=PermissionError('denied')):
            with self.assertRaises(PermissionError):
                t.snapshot(self.root)

    def test_opens_only_readonly_sysfs_files(self):
        original, calls = os.open, []
        def checked(path, flags, *args, **kwargs):
            calls.append(Path(path))
            self.assertTrue(Path(path).is_relative_to(self.device))
            self.assertEqual(flags & os.O_ACCMODE, os.O_RDONLY)
            self.assertTrue(flags & os.O_NOFOLLOW)
            return original(path, flags, *args, **kwargs)
        with mock.patch.object(t.os, 'open', side_effect=checked):
            t.snapshot(self.root)
        self.assertGreater(len(calls), 20)


if __name__ == '__main__':
    unittest.main()
