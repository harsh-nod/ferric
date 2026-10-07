"""Bounded read-only MI350 text telemetry; never opens a GPU execution device."""
import argparse
import json
import os
from pathlib import Path
import socket
import stat
import time

PCI = '0000:05:00.0'
UNIQUE_ID = 'e3233d81d822f3eb'
DEVICE_FIELDS = (
    'gpu_busy_percent', 'mem_busy_percent', 'pp_dpm_sclk', 'pp_dpm_mclk',
    'pp_dpm_socclk', 'pp_dpm_fclk', 'power_dpm_force_performance_level', 'power_dpm_state',
)
HWMON_FIELDS = (
    'freq1_label', 'freq1_input', 'freq2_label', 'freq2_input',
    'power1_label', 'power1_input', 'power1_average', 'power1_cap',
    'power1_cap_default', 'power1_cap_min', 'power1_cap_max',
    'temp2_label', 'temp2_input', 'temp2_crit', 'temp3_label', 'temp3_input', 'temp3_crit',
)


def require(ok, message):
    if not ok:
        raise ValueError(message)


def identity(info):
    return info.st_dev, info.st_ino, info.st_mode, info.st_uid


def validate_schedule(samples, interval_seconds):
    require(1 <= samples <= 1200 and 0.1 <= interval_seconds <= 10
            and samples * 2 + (samples - 1) * interval_seconds <= 3600,
            'bounded sampling duration including read deadlines')


def read_attribute(path, root, deadline, optional=False):
    require(time.monotonic() < deadline, 'telemetry deadline')
    try:
        before = path.lstat()
    except FileNotFoundError:
        if optional:
            return {'status': 'unavailable'}
        raise
    require(path.resolve(strict=True) == path and path.is_relative_to(root)
            and stat.S_ISREG(before.st_mode) and before.st_size <= 4096, 'bounded canonical text attribute')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC), 'rb') as source:
        require(identity(os.fstat(source.fileno())) == identity(before), 'opened attribute identity')
        raw = source.read(4097)
        require(identity(os.fstat(source.fileno())) == identity(before) == identity(path.lstat()),
                'stable attribute identity')
    require(len(raw) <= 4096 and time.monotonic() < deadline, 'bounded attribute read')
    text = raw.decode('ascii')
    require('\x00' not in text, 'text attribute contains NUL')
    return {'status': 'ok', 'text': text}


def snapshot(sys_root=Path('/sys')):
    started, wall = time.monotonic_ns(), time.time_ns()
    deadline = time.monotonic() + 2
    sys_root = Path(sys_root).resolve(strict=True)
    link = sys_root / 'class/drm/renderD128/device'
    device = link.resolve(strict=True)
    require(device.is_relative_to(sys_root / 'devices') and device.name == PCI, 'fixed GPU PCI identity')
    before = identity(device.stat())
    unique = read_attribute(device / 'unique_id', device, deadline)
    require(unique['text'].strip() == UNIQUE_ID, 'fixed GPU unique identity')
    values = {name: read_attribute(device / name, device, deadline, optional=True) for name in DEVICE_FIELDS}
    parent = device / 'hwmon'
    require(parent.resolve(strict=True) == parent and parent.is_dir(), 'canonical device hwmon directory')
    candidates = []
    with os.scandir(parent) as entries:
        for count, entry in enumerate(entries, 1):
            require(count <= 8 and time.monotonic() < deadline, 'bounded hwmon discovery')
            require(entry.name.startswith('hwmon') and entry.is_dir(follow_symlinks=False), 'real hwmon child')
            path = Path(entry.path)
            if read_attribute(path / 'name', device, deadline)['text'].strip() == 'amdgpu':
                candidates.append(path)
    require(len(candidates) == 1, 'one amdgpu hwmon source')
    hwmon = candidates[0]
    hwmon_identity = identity(hwmon.stat())
    sensors = {name: read_attribute(hwmon / name, device, deadline, optional=True) for name in HWMON_FIELDS}
    require(link.resolve(strict=True) == device and identity(device.stat()) == before
            and identity(hwmon.stat()) == hwmon_identity
            and read_attribute(device / 'unique_id', device, deadline) == unique, 'stable GPU and sensor identities')
    return {'schema': 'FerricGpuTextTelemetryR1', 'started_monotonic_ns': started,
            'finished_monotonic_ns': time.monotonic_ns(), 'started_wall_ns': wall,
            'pci': PCI, 'unique_id': UNIQUE_ID, 'device': str(device), 'hwmon': str(hwmon),
            'device_fields': values, 'hwmon_fields': sensors, 'atomic_snapshot': False,
            'gpu_time_measured': False, 'throttle_state_measured': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--samples', type=int, default=1)
    parser.add_argument('--interval-seconds', type=float, default=1)
    args = parser.parse_args()
    require(socket.gethostname() == 'smci350-rck-g03-b19-03' and os.getuid() == 9661,
            'fixed telemetry host and owner')
    validate_schedule(args.samples, args.interval_seconds)
    parent = args.output.parent
    info = parent.lstat()
    require(args.output.name == 'samples.jsonl' and parent.parent == Path('/dev/shm')
            and parent.name.startswith('ferric-gate-up-telemetry-')
            and parent.resolve(strict=True) == parent and stat.S_ISDIR(info.st_mode)
            and info.st_uid == 9661 and stat.S_IMODE(info.st_mode) == 0o700
            and info.st_dev == Path('/dev/shm').stat().st_dev, 'owned private telemetry output')
    os.umask(0o077)
    total = 0
    with args.output.open('xb') as output:
        for index in range(args.samples):
            if index:
                time.sleep(args.interval_seconds)
            value = snapshot()
            raw = (json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n').encode()
            total += len(raw)
            require(len(raw) <= 128 * 1024 and total <= 16 * 1024**2, 'bounded telemetry output')
            require(identity(parent.stat()) == identity(info), 'output directory identity')
            output.write(raw)
            output.flush()
    print(json.dumps({'samples': args.samples, 'bytes': total, 'gpu_devices_opened': False,
                      'settings_changed': False}, sort_keys=True))


if __name__ == '__main__':
    main()
