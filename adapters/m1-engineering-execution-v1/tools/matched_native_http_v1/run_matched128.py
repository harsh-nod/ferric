#!/usr/bin/env python3
"""Private matched native HTTP cells, always under the qualified outer supervisor."""
import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import stat
import subprocess
import sys
import threading
import time
import types
import uuid

ROOT = Path(__file__).resolve().parent
NATIVE_EXECUTION_ENABLED = True

def local(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / filename)
    value = importlib.util.module_from_spec(spec)
    sys.modules[name] = value
    spec.loader.exec_module(value)
    return value

contract = local('native_http_contract', 'matched128_contract.py')
life = local('native_http_life', 'frozen/native_lifecycle.py')
private = local('native_http_backend', 'http_lifecycle.py')
commands = local('native_http_commands', 'owned_command.py')
custody = local('native_http_container', 'container_custody.py')
require, read, write, digest = contract.require, contract.read, contract.write, contract.digest
MAX_RAW, MAX_SECONDS = 256 * 1024**2, 6000
TMPFS_ROOT = Path('/dev/shm')
SHARED_MEMORY_FREE_BYTES = 32 * 1024**3


def command(argv, out, label, timeout=30, limit=4 * 1024**2, monitor=None, check=True):
    status, stdout, _ = commands.run(argv, out, label, lifecycle=life, save=write,
        timeout=timeout, limit=limit, monitor=monitor, check=check)
    return status, stdout


def tmpfs_mount_record(raw):
    matches = []
    for line in raw.splitlines():
        before, separator, after = line.partition(' - ')
        fields, filesystem = before.split(), after.split()
        require(separator and len(fields) >= 6 and len(filesystem) >= 3, 'malformed mountinfo record')
        if fields[4] == str(TMPFS_ROOT):
            matches.append((fields, filesystem))
    require(len(matches) == 1 and matches[0][1][0] == 'tmpfs'
            and 'rw' in matches[0][0][5].split(',')
            and 'noexec' not in matches[0][0][5].split(','),
            'exact writable executable tmpfs mount required')
    return {'mountpoint': str(TMPFS_ROOT), 'filesystem': matches[0][1][0],
            'device': matches[0][0][2], 'mount_options': matches[0][0][5]}


def tmpfs_status():
    info = TMPFS_ROOT.lstat()
    require(stat.S_ISDIR(info.st_mode) and not TMPFS_ROOT.is_symlink()
            and TMPFS_ROOT.resolve(strict=True) == TMPFS_ROOT and info.st_uid == 0
            and info.st_mode & stat.S_ISVTX, 'canonical root-owned sticky shared-memory directory required')
    raw = Path('/proc/self/mountinfo').read_text()
    require(len(raw) <= 1024**2, 'bounded mountinfo required')
    result = tmpfs_mount_record(raw)
    result['free_bytes'] = shutil.disk_usage(TMPFS_ROOT).free
    require(result['free_bytes'] >= SHARED_MEMORY_FREE_BYTES, 'shared-memory free-space floor violated')
    return result


def cache_directory_identity(path, expected=None):
    require(path.parent == TMPFS_ROOT
            and re.fullmatch(r'ferric-matched128-(?:vllm|sglang)-[0-9a-f]{16}', path.name) is not None
            and path.resolve(strict=True) == path, 'exact canonical disposable cache path required')
    info = path.lstat()
    require(stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid()
            and stat.S_IMODE(info.st_mode) == 0o700 and info.st_dev == TMPFS_ROOT.stat().st_dev,
            'owned mode0700 nonsymlink cache on shared-memory filesystem required')
    value = {'path': str(path), 'device': info.st_dev, 'inode': info.st_ino,
             'uid': info.st_uid, 'mode': stat.S_IMODE(info.st_mode)}
    require(expected is None or value == expected, 'owned cache root identity changed')
    return value


@contextmanager
def absolute_timeout(seconds):
    started = time.monotonic()
    prior_timer = signal.getitimer(signal.ITIMER_REAL)
    def expired(_signum, _frame):
        raise TimeoutError('bounded preflight absolute deadline exceeded')
    prior_handler = signal.signal(signal.SIGALRM, expired)
    signal.setitimer(signal.ITIMER_REAL, min(seconds, prior_timer[0]) if prior_timer[0] else seconds)
    try:
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, prior_handler)
        if prior_timer[0]:
            signal.setitimer(signal.ITIMER_REAL, max(0.001, prior_timer[0] - (time.monotonic() - started)), prior_timer[1])


class GpuBusyError(ValueError):
    """Only residual utilization without process/identity drift may settle."""


def idle_roster(value):
    require(type(value) is dict and set(value) == {f'card{i}' for i in range(8)}, 'unexpected GPU/process roster')
    idle = True
    for index, expected in enumerate(contract.GPU_IDS):
        card = value[f'card{index}']
        require(int(card['Unique ID'], 16) == expected, 'physical GPU identity drifted')
        require(not any('pid' in key.lower() or 'process' in key.lower() for key in card), 'GPU process evidence present')
        for key in ('GPU use (%)', 'GPU Memory Allocated (VRAM%)', 'GPU Memory Read/Write Activity (%)'):
            metric = card.get(key)
            require(type(metric) is str and metric.isascii() and metric.isdecimal()
                    and 0 <= int(metric) <= 100, 'GPU utilization metric invalid')
            idle = idle and metric == '0'
    if not idle:
        raise GpuBusyError('all eight GPUs must be idle')
    return value


def gpu_snapshot(out, label, timeout=25):
    _, raw = command(['/opt/rocm/bin/rocm-smi', '--showuniqueid', '--showuse', '--showmemuse',
                      '--showpids', '--json'], out, label, timeout=timeout, limit=65536)
    return idle_roster(json.loads(raw))


def settle_gpu_idle(out):
    started = time.monotonic()
    deadline = started + 30
    receipt = {'schema': 'FerricMatched128GpuIdleSettlementV1', 'deadline_seconds': 30,
               'started_monotonic': started, 'settled': False, 'attempts': []}
    try:
        for index in range(31):
            remaining = deadline - time.monotonic()
            require(remaining > 0, 'post-cleanup GPU idle settling deadline exceeded')
            label = f'gpu-postflight-{index:04d}'
            attempt = {'label': label, 'started_monotonic': time.monotonic()}
            receipt['attempts'].append(attempt)
            try:
                value = gpu_snapshot(out, label, timeout=min(5, remaining))
                require(time.monotonic() <= deadline, 'post-cleanup GPU idle sample completed after deadline')
                attempt['status'] = 'idle'
                receipt['settled'] = True
                return value
            except GpuBusyError as error:
                attempt.update(status='residual_utilization', error=str(error))
            except BaseException as error:
                attempt.update(status='invalid', error=str(error)[:4096], error_type=type(error).__name__)
                raise
            finally:
                attempt['completed_monotonic'] = time.monotonic()
            remaining = deadline - time.monotonic()
            require(remaining > 0, 'post-cleanup GPU idle settling deadline exceeded')
            time.sleep(min(1, remaining))
        raise TimeoutError('post-cleanup GPU idle attempt bound exceeded')
    finally:
        receipt['completed_monotonic'] = time.monotonic()
        write(out / 'gpu-postflight-settling.json', receipt)


def exception_chain(error):
    """Retain masked startup/cleanup failures without tracebacks or unbounded cycles."""
    records, seen = [], set()
    current = error
    while current is not None and id(current) not in seen and len(records) < 8:
        seen.add(id(current))
        prior = current.__cause__ if current.__cause__ is not None else current.__context__
        records.append({'type': type(current).__name__, 'message': str(current)[:4096],
                        'classification': 'workload_or_resource_or_lifecycle_invalid',
                        'older_error_relationship': 'cause' if current.__cause__ is not None else
                            'context' if prior is not None else None})
        current = prior
    return list(reversed(records)), current is not None


def resources(out, cache=None):
    memory = {}
    for line in Path('/proc/meminfo').read_text().splitlines():
        fields = line.split()
        if fields[0] in ('MemAvailable:', 'MemTotal:'):
            memory[fields[0][:-1]] = int(fields[1]) * 1024
    require(memory.get('MemAvailable', 0) >= 128 * 1024**3, 'host available-memory floor violated')
    free = shutil.disk_usage(out).free
    require(free >= 64 * 1024**3, 'owned-stage free-space floor violated')
    root_free = shutil.disk_usage('/').free
    require(root_free >= 64 * 1024**3, 'root-disk free-space floor violated')
    output_entries = output_bytes = 0
    for parent, dirs, files in os.walk(out, followlinks=False):
        output_entries += len(dirs) + len(files)
        require(output_entries <= 20000, 'owned HTTP evidence entry budget')
        for name in dirs + files:
            info = (Path(parent) / name).lstat()
            require(not stat.S_ISLNK(info.st_mode) and info.st_uid == os.getuid(), 'owned nonsymlink HTTP evidence')
            if name in files:
                require(stat.S_ISREG(info.st_mode) and info.st_size <= MAX_RAW, 'bounded regular HTTP evidence')
                output_bytes += info.st_size
        require(output_bytes <= 2 * 1024**3, 'aggregate HTTP evidence byte budget')
    count = size = 0
    shared_memory = None
    if cache is not None:
        shared_memory = tmpfs_status()
        for root, dirs, files in os.walk(cache, followlinks=False):
            count += len(dirs) + len(files)
            require(count <= 20000, 'owned cache entry budget exhausted')
            for name in files:
                info = (Path(root) / name).lstat()
                size += info.st_size
                require(size <= 8 * 1024**3, 'owned cache byte budget exhausted')
    return {'memory': memory, 'disk_free_bytes': free, 'owned_cache_bytes': size,
            'owned_cache_entries': count, 'root_disk_free_bytes': root_free, 'shared_memory': shared_memory,
            'owned_output_entries': output_entries, 'owned_output_bytes': output_bytes}


def json_http(client, port, method, path, body=None, timeout=120):
    conn = client.DeadlineHTTPConnection('127.0.0.1', port, time.monotonic_ns() + int(timeout * 1e9))
    try:
        data = None if body is None else json.dumps(body, allow_nan=False).encode()
        conn.request(method, path, body=data, headers={} if data is None else {'Content-Type': 'application/json'})
        response = conn.getresponse()
        require(response.status == 200, f'HTTP status {response.status} at {path}')
        raw = response.read(2 * 1024**2 + 1)
        require(len(raw) <= 2 * 1024**2, 'diagnostic body budget exhausted')
        return json.loads(raw) if raw else {'http_status': response.status}
    finally:
        conn.dispose()


class Container:
    def __init__(self, engine, plan, out, intent):
        self.engine, self.plan, self.out = engine, plan, out
        custody.validate_intent(intent)
        require(intent['image'] == contract.IMAGES[engine]['id'], 'predeclared container image')
        self.intent, self.identity = intent, None
        self.name = intent['name']
        self.id = None
        self.resource_lock = threading.Lock()
        self.cache, self.observed = TMPFS_ROOT / self.name, out / 'runtime-dtype'
        self.cache_identity = None
        self.cache_retired = False
        self.cache_create_attempted = False
        self.observed.mkdir(mode=0o700)
        self.counter = 0
        self.next_monitor = 0
        self.closed = False
        self.argv = contract.baseline_argv(engine, self.name, self.cache, plan['render_device'],
                                           Path(__file__).parent, self.observed)

    def prepare_cache(self):
        with self.resource_lock:
            shared_memory = tmpfs_status()
            require(not os.path.lexists(self.cache), 'fresh disposable cache required')
            self.cache_create_attempted = True
            self.cache.mkdir(mode=0o700)
            info = self.cache.lstat()
            self.cache_identity = {'path': str(self.cache), 'device': info.st_dev, 'inode': info.st_ino,
                                   'uid': info.st_uid, 'mode': stat.S_IMODE(info.st_mode)}
            cache_directory_identity(self.cache, self.cache_identity)
            write(self.out / 'cache-identity.json', {
                'schema': 'FerricMatched128TmpfsCacheV1', 'identity': self.cache_identity,
                'shared_memory': shared_memory, 'cache_bytes_limit': 8 * 1024**3,
                'cache_entries_limit': 20000, 'root_disk_floor_bytes': 64 * 1024**3,
                'memory_available_floor_bytes': 128 * 1024**3,
                'shared_memory_free_floor_bytes': SHARED_MEMORY_FREE_BYTES})
        engine = self.engine
        if engine == 'sglang':
            # Skip AITER's recursive image-cache copy without changing module lookup.
            (self.cache / '.aiter').mkdir(mode=0o700)
            (self.cache / '.aiter' / 'jit').mkdir(mode=0o700)
            write(self.out / 'aiter-cache-preparation.json', {
                'schema': 'FerricMatched128AiterCachePreparationV1',
                'image_id': contract.IMAGES['sglang']['id'],
                'container_directory': '/matched-cache/.aiter/jit',
                'created_empty': True, 'aiter_jit_dir_override': False,
                'kernel_or_backend_change': False})

    def cache_for_monitor(self):
        if self.cache_identity is None or self.cache_retired:
            require(not os.path.lexists(self.cache), 'unowned or retired cache path appeared')
        else:
            cache_directory_identity(self.cache, self.cache_identity)
        return self.cache

    def inspect(self, label, size=False):
        args = ['docker', 'inspect'] + (['--size'] if size else []) + [self.id or self.name]
        _, raw = command(args, self.out, label, limit=1024**2)
        values = json.loads(raw)
        require(len(values) == 1, 'owned container inspect cardinality invalid')
        value = values[0]
        require(value['Name'] == '/' + self.name
                and value['Config']['Labels'].get('ferric.matched128') == self.name
                and value['Image'] == contract.IMAGES[self.engine]['id'], 'container ownership/image drifted')
        require(self.id is None or value['Id'] == self.id, 'owned container ID drifted')
        self.identity = custody.observe(values, self.intent, self.identity)
        self.id = value['Id']
        return value

    def start(self):
        image = contract.IMAGES[self.engine]
        _, raw = command(['docker', 'image', 'inspect', image['digest']], self.out, 'image-inspect', limit=1024**2)
        value = json.loads(raw)
        require(len(value) == 1 and value[0]['Id'] == image['id'] and image['digest'] in value[0]['RepoDigests'],
                'exact cached image ID/digest required; no pull allowed')
        self.prepare_cache()
        write(self.out / 'launch-argv.json', self.argv)
        # Register the unique name before create, so a create timeout is still cleaned.
        self.created_attempted = True
        command(self.argv, self.out, 'container-create')
        self.inspect('container-created')
        command(['docker', 'start', self.id], self.out, 'container-start')
        self.inspect('container-started')

    def monitor(self):
        if time.monotonic() < self.next_monitor:
            return
        self.next_monitor = time.monotonic() + 5
        with self.resource_lock:
            resources(self.out, self.cache_for_monitor())
        value = self.inspect(f'container-monitor-{self.counter:04d}', size=True)
        self.counter += 1
        require(self.counter <= 1300, 'container monitor bound exceeded')
        require(value['State']['Running'] and not value['State'].get('OOMKilled'), 'owned container stopped/OOM')
        require(value.get('SizeRw', 0) <= 8 * 1024**3, 'container writable-layer budget exhausted')

    def close(self):
        if not getattr(self, 'created_attempted', False):
            self.retire_cache()
            self.closed = True
            return
        clean = False
        try:
            value = self.inspect('container-before-close')
            command(['docker', 'stop', '--time', '30', self.id], self.out, 'container-stop', timeout=40)
            value = self.inspect('container-stopped')
            clean = not value['State']['Running'] and not value['State'].get('OOMKilled') and value['State']['ExitCode'] in (0, 143)
            status, logs = command(['docker', 'logs', '--timestamps', self.id], self.out,
                                  'container-logs', limit=8 * 1024**2, check=False)
            stderr = (self.out / 'container-logs.stderr').read_bytes()
            forced = self.engine == 'vllm' and any(b'[shutdown] Process manager: force killing remaining' in raw
                                                 for raw in (logs, stderr))
            write(self.out / 'container-shutdown.json', {'schema': 'FerricNativeHttpContainerShutdownV1',
                'host_exit_clean': clean, 'log_exit_status': status, 'internal_force_kill_observed': forced})
            clean = clean and status == 0 and not forced
        finally:
            def cleanup_command(argv, label, **kwargs):
                return commands.run(argv, self.out, label, lifecycle=life, save=write, check=False, **kwargs)
            retired = custody.retire(cleanup_command, self.intent, self.identity)
            write(self.out / 'container-retirement.json', retired)
            self.retire_cache()
            self.closed = True
        require(clean, 'container teardown forced or failed; timing is invalid')

    def retire_cache(self):
        if self.cache_retired:
            return
        if self.cache_identity is None:
            require(not self.cache_create_attempted or not os.path.lexists(self.cache),
                    'cache creation has no verified identity; retain exact path for investigation: ' + str(self.cache))
            return
        with self.resource_lock:
            cache_directory_identity(self.cache, self.cache_identity)
        _, raw = command(['docker', 'container', 'ls', '-a', '--no-trunc', '--format', '{{json .}}'],
                         self.out, 'cache-retire-container-census', timeout=10, limit=1024**2)
        rows = [json.loads(line) for line in raw.splitlines()]
        require(len(rows) <= 10000 and all(type(row) is dict and type(row.get('ID')) is str
                and re.fullmatch(r'[0-9a-f]{64}', row['ID']) is not None
                and type(row.get('Names')) is str and bool(row['Names']) for row in rows),
                'complete container census required')
        require(all(row['ID'] != self.id and self.name not in row['Names'].split(',') for row in rows),
                'owned container remains; disposable cache cannot be retired')
        status, raw = command(['sudo', '-n', '/usr/bin/lsof', '-nP', '+D', str(self.cache)],
                              self.out, 'cache-retire-no-use', timeout=60, limit=4 * 1024**2, check=False)
        require(status == 1 and not raw.strip()
                and not (self.out / 'cache-retire-no-use.stderr').read_bytes().strip(),
                'all-UID cache no-use scan refused retirement')
        with self.resource_lock:
            cache_directory_identity(self.cache, self.cache_identity)
            require(shutil.rmtree.avoids_symlink_attacks, 'fd-safe owned cache deletion required')
            shutil.rmtree(self.cache)
            require(not os.path.lexists(self.cache), 'owned disposable cache survived retirement')
            self.cache_retired = True
            write(self.out / 'cache-retirement.json', {
                'schema': 'FerricMatched128TmpfsCacheRetirementV1', 'identity': self.cache_identity,
                'container_absent': True, 'all_uid_no_use': True, 'cache_absent': True,
                'deletion_uid': os.getuid(), 'shared_tmpfs_removed': False})


class Ferric:
    def __init__(self, serving, plan, out, prompt):
        self.serving, self.plan, self.out, self.prompt = serving, plan, out, prompt
        self.backend = self.server = self.thread = None
        self.server_loop_entered = threading.Event()
        self.closed = False
        self.finals, self.events = [], []

    def start(self):
        outer, serving = self, self.serving
        for name in ('controller', 'worker'):
            binding = self.plan['ferric'][name]
            require(digest(binding['path']) == binding['sha256'], 'selected executable changed')
        base = private.backend_class(serving, life, write)

        class Backend(base):
            def __init__(self, config):
                self.admission_lock = threading.Lock()
                self.admissions = 0
                super().__init__(config, owner=outer, output=outer.out)

            def submit(self, prompt, max_tokens):
                with self.admission_lock:
                    if self.admissions >= 42:
                        raise serving.RequestError(429, 'frozen request cap exhausted')
                    if prompt != outer.prompt or max_tokens != 128:
                        raise serving.RequestError(400, 'frozen matched workload differs')
                    pending = super().submit(prompt, max_tokens)
                    self.admissions += 1
                    return pending

            def _event(self, event):
                if event.get('event') == 'request':
                    require(len(outer.finals) < 42, 'request evidence cap')
                    outer.finals.append(event)
                if event.get('event') not in ('token', 'batch'):
                    require(len(outer.events) < 256, 'non-token evidence cap')
                    outer.events.append(event)
                super()._event(event)

        config = serving.Config(argv=tuple(self.plan['ferric']['argv']), model=contract.MODEL,
            max_inflight=1, ready_timeout_seconds=600, request_timeout_seconds=120,
            shutdown_timeout_seconds=15)
        self.backend = Backend(config)
        self.backend.wait_ready()
        contract.selection_adapter(self.plan['driver_sources']).validate_setup(
            self.backend.setup, self.plan['ferric'])
        write(self.out / 'ferric-setup.json', self.backend.setup)
        self.server = serving.Server(18980, self.backend)
        def serve():
            self.server_loop_entered.set()
            self.server.serve_forever(poll_interval=0.05)
        self.thread = threading.Thread(target=serve, name='private-matched-http')
        self.thread.start()

    def monitor(self):
        resources(self.out)
        require(self.backend is not None and self.backend.running(), 'bound controller exited during request')

    def close(self):
        cleanup = None
        try:
            if self.server is not None and self.thread is not None and self.thread.ident is not None \
                    and self.thread.is_alive():
                require(self.server_loop_entered.wait(1), 'started HTTP thread did not enter its server loop')
                self.server.shutdown()
            if self.backend is not None:
                cleanup = self.backend.close()
        finally:
            if self.server is not None:
                self.server.server_close()
            if self.thread is not None and self.thread.ident is not None:
                self.thread.join(timeout=5)
            write(self.out / 'ferric-final-events.json', self.events)
            self.closed = (private.normal_completion(cleanup)
                and (self.thread is None or not self.thread.is_alive()))
            write(self.out / 'ferric-teardown.json', {
                'schema': 'FerricNativeMatchedHttpTeardownV1', 'normal_completion': self.closed,
                'controller_cleanup': cleanup, 'server_thread_joined': self.thread is None or not self.thread.is_alive()})
        require(self.closed, 'normal Closed/exit0/unsignaled cleanup required; teardown alone is not timing admission')


def inode_binding(binding):
    path = Path(binding['path'])
    require(digest(path) == binding['sha256'] and path.resolve(strict=True) == path, 'selected executable custody')
    info = path.stat()
    return {'path': str(path), 'dev': info.st_dev, 'ino': info.st_ino, 'size': info.st_size}


class ProbeObserver:
    def __init__(self, plan, engine, output, intent=None, prefix='outer'):
        self.plan, self.engine, self.output, self.intent = plan, engine, Path(output), intent
        require(prefix in ('inner', 'outer'), 'closed sample namespace')
        self.prefix = prefix
        self.counter = 0
        self.native_anchors = {'controller': None, 'worker': None}
        self.container_identity = None
        self.container_runtime = None
        self.worker = inode_binding(plan['ferric']['worker'])
        self.controller = inode_binding(plan['ferric']['controller'])

    def sample(self, phase, wrapper=None):
        descriptor = os.open(self.output / 'admission.lock',
            os.O_CREAT | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
        try:
            info = os.fstat(descriptor)
            require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid()
                    and info.st_nlink == 1 and stat.S_IMODE(info.st_mode) == 0o600, 'owned sample lock')
            deadline = time.monotonic() + 45
            while True:
                try:
                    fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError:
                    require(time.monotonic() < deadline, 'paired sample lock deadline')
                    time.sleep(0.05)
            return self._sample(phase, wrapper)
        finally:
            os.close(descriptor)

    def _sample(self, phase, wrapper=None):
        require(self.counter < 2200, 'finite sample roster exhausted')
        label = self.prefix + '-gpu-sample-' + str(self.counter).zfill(4)
        self.counter += 1
        owner = {'kind': 'empty', 'binding': None}
        if phase in ('startup', 'active'):
            if self.engine == 'ferric':
                require(wrapper is not None, 'owned wrapper anchor required')
                owner = {'kind': 'native', 'binding': {'wrapper': wrapper,
                    'controller_executable': self.controller, 'worker_executable': self.worker,
                    **self.native_anchors}}
            else:
                owner = {'kind': 'container', 'binding': {'intent': self.intent,
                    'identity': self.container_identity, 'runtime': self.container_runtime}}
        names = ('gpu_activity.py', 'gpu_attribution.py', 'gpu_probe.py',
                 'frozen/native_lifecycle.py', 'container_custody.py')
        value = {'schema': 'FerricNativeHttpGpuProbeInputV1', 'phase': phase,
            'owner': owner, 'devices': ['/dev/kfd', self.plan['render_device']],
            'sources': {name: self.plan['driver_sources'][name] for name in names}}
        path = self.output / (label + '-input.json')
        write(path, value)
        status, stdout = command(['sudo', '-n', '/usr/bin/python3', '-I', '-B', str(ROOT / 'probe_cli.py'),
            '--input', str(path), '--input-sha256', digest(path)], self.output, label,
            timeout=40, limit=8 * 1024**2, check=False)
        result = json.loads(stdout)
        require(status == 0 and result.get('accepted') is True and result.get('errors') == [],
                'privileged paired descriptor sample refused: ' + label)
        if owner['kind'] == 'native':
            self.native_anchors = result['retained_owner']
        elif owner['kind'] == 'container':
            self.container_identity = result['retained_owner']['identity']
            self.container_runtime = result['retained_owner']['runtime']
        return result

def ready(client, engine, owner, out):
    port = 18980 if engine == 'ferric' else contract.IMAGES[engine]['port']
    deadline = time.monotonic() + 600
    last = None
    while time.monotonic() < deadline:
        owner.monitor()
        try:
            value = json_http(client, port, 'GET', '/health', timeout=2)
            write(out / 'health.json', value)
            return
        except (ValueError, OSError) as error:
            last = str(error)
        time.sleep(0.5)
    raise TimeoutError('bounded readiness failed: ' + str(last))


def diagnostic(client, engine, owner, prompt, out, label):
    if engine != 'ferric':
        path, payload = contract.diagnostic_payload(engine, prompt)
        write(out / (label + '-request.json'), {'path': path, 'body': payload, 'timed': False})
        raw = json_http(client, contract.IMAGES[engine]['port'], 'POST', path, payload)
    else:
        count = len(owner.finals)
        result = client.request_one('http://127.0.0.1:18980/v1/completions', contract.MODEL,
                                   {'id': label, 'prompt': prompt, 'max_tokens': 128}, 120)
        write(out / (label + '-stream.json'), result)
        require(result['success'], 'Ferric untimed diagnostic failed')
        end = time.monotonic() + 2
        while len(owner.finals) == count and time.monotonic() < end:
            time.sleep(0.01)
        require(len(owner.finals) == count + 1, 'Ferric final token IDs missing')
        raw = owner.finals[-1]
    write(out / (label + '.json'), raw)
    return raw


def monitor_run(owner, out):
    owner.monitor()
    path = out / 'timed-raw.json'
    require(not path.exists() or path.stat().st_size <= MAX_RAW, 'timed raw evidence limit')


def execute(plan, plan_sha, engine, out, inputs, intent):
    workload, tokenizer, target, reference = inputs
    client = contract.module('matched_client', plan['client'], contract.CLIENT_SHA)
    serving = contract.module('matched_serving', plan['serving'], contract.SERVING_SHA) if engine == 'ferric' else None
    receipt = {'schema': 'FerricNativeMatched128EngineeringReceiptV1', 'engine': engine,
        'plan_sha256': plan_sha, 'settings': contract.SETTINGS, 'qualification': False,
        'framework_win_claim': False, 'timing_admitted': False, 'errors': [],
        'numerical_diagnostics': [], 'started_unix_ns': time.time_ns(), 'cleanup_completed': False}
    owner = None
    try:
        write(out / 'resource-preflight.json', resources(out))
        render = Path(plan['render_device'])
        require(render.is_char_device() and render.stat().st_gid == contract.RENDER_GID
                and Path('/dev/kfd').stat().st_gid == contract.RENDER_GID, 'GPU device ownership changed')
        unique = Path('/sys/class/drm') / render.name / 'device/unique_id'
        require(int(unique.read_text().strip(), 16) == contract.GPU_IDS[0], 'physical GPU0 identity changed')
        started = time.monotonic()
        with absolute_timeout(90):
            for entry in target['files']:
                path = Path(contract.TARGET) / entry['name']
                require(path.stat().st_size == entry['bytes'] and digest(path, 8 * 1024**3) == entry['sha256'],
                        'canonical target content changed')
        write(out / 'target-preflight.json', {'manifest_sha256': plan['target_manifest']['sha256'],
            'all_file_hashes_match': True, 'elapsed_seconds': time.monotonic() - started})
        port = 18980 if engine == 'ferric' else contract.IMAGES[engine]['port']
        with socket.socket() as probe:
            probe.bind(('127.0.0.1', port))
        prompt = workload['requests'][0]['prompt']
        owner = Ferric(serving, plan, out, prompt) if engine == 'ferric' else Container(engine, plan, out, intent)
        try:
            owner.start()
            ready(client, engine, owner, out)
            observer = ProbeObserver(plan, engine, out, intent, prefix='inner')
            write(out / 'active-attribution.json', observer.sample('active', life.process(os.getpid())))
            write(out / 'active-started.json', {'schema': 'FerricNativeHttpActiveStartV1',
                'wrapper': life.process(os.getpid()), 'positive_sample_required': True})
            if engine == 'ferric':
                resolved = {'ferric_setup': owner.backend.setup}
            else:
                files = list(owner.observed.glob('*.json'))
                require(len(files) == 1, 'one actual TP1 post-load dtype receipt required')
                resolved = contract.validate_resolved(read(files[0], limit=512 * 1024), engine,
                    plan['driver_sources']['sitecustomize.py'])
            identity = {'engine': engine, 'plan_sha256': plan_sha, 'settings': contract.SETTINGS,
                'target_manifest_sha256': plan['target_manifest']['sha256'],
                'image': contract.IMAGES.get(engine), 'ferric': plan['ferric'] if engine == 'ferric' else None,
                'resolved': resolved, 'startup_observer_touches_forward_path': False}
            write(out / 'identity.json', identity)
            before = diagnostic(client, engine, owner, prompt, out, 'diagnostic-before')
            receipt['numerical_diagnostics'].append(contract.numerical_diagnostic(engine, before, tokenizer, reference))
            status, _ = command(contract.client_argv(plan, engine, out), out, 'shared-client', timeout=5000,
                limit=4 * 1024**2, monitor=lambda: monitor_run(owner, out), check=False)
            receipt['client_exit_status'] = status
            after = diagnostic(client, engine, owner, prompt, out, 'diagnostic-after')
            receipt['numerical_diagnostics'].append(contract.numerical_diagnostic(engine, after, tokenizer, reference))
            write(out / 'active-final-attribution.json', observer.sample('active', life.process(os.getpid())))
            require(status == 0, 'shared timing client failure')
            timed = read(out / 'timed-raw.json', limit=MAX_RAW)
            result = contract.validate_timing(client, timed, plan, engine, reference, digest(out / 'identity.json'))
            receipt['timing_replay'] = result
            receipt['timing_admitted'] = result['timing_admitted'] and all(
                row['admitted'] for row in receipt['numerical_diagnostics'])
            if engine == 'ferric':
                require(len(owner.finals) == 42, 'all 42 Ferric request completions required')
                parity = [contract.numerical_diagnostic(engine, row, tokenizer, reference) for row in owner.finals]
                receipt['ferric_all_request_parity'] = parity
                receipt['timing_admitted'] = receipt['timing_admitted'] and all(row['admitted'] for row in parity)
        finally:
            with life.deferred_stop(deliver=False):
                write(out / 'cleanup-started.json', {'schema': 'FerricNativeHttpCleanupStartV1',
                    'wrapper': life.process(os.getpid()), 'timing_finished': True,
                    'scope': 'no new timing admitted; bounded teardown followed by mandatory empty GPU postflight'})
                owner.close()
                receipt['cleanup_completed'] = owner.closed
        write(out / 'resource-postflight.json', resources(out))
        if engine == 'ferric':
            contract.validate_ferric_closed(owner.events, owner.backend.setup, plan['ferric'], plan['driver_sources'])
        contract.validate_plan(plan)
    except BaseException as error:
        receipt['errors'], receipt['exception_chain_truncated'] = exception_chain(error)
        receipt['timing_admitted'] = False
        raise
    finally:
        receipt['completed_unix_ns'] = time.time_ns()
        receipt['cleanup_completed'] = owner is not None and owner.closed
        write(out / 'receipt.json', receipt)


@contextmanager
def gpu_lease(plan):
    require(socket.gethostname() == contract.HOST and os.getuid() == contract.HOST_UID,
            'frozen host and owner required')
    native = Path(plan['ferric']['native_evidence']['plan']['path'])
    require(native.name == 'plan.json', 'selected native prerequisite lock')
    descriptor = os.open(native.with_name('native.lock'), os.O_RDWR | os.O_NOFOLLOW)
    try:
        info = os.fstat(descriptor)
        require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1
                and stat.S_IMODE(info.st_mode) == 0o600, 'existing owned native lease required')
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield
    finally:
        os.close(descriptor)


def supervisor_save(path, value):
    require(path.name == 'launch-supervisor.json', 'only supervisor receipt may be replaced')
    if path.exists():
        info = path.lstat()
        require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1,
                'owned supervisor receipt required')
    with path.open('w') as stream:
        json.dump(value, stream, sort_keys=True, allow_nan=False)
        stream.write('\n')


def facilities():
    require(hasattr(os, 'pidfd_open') and hasattr(signal, 'pidfd_send_signal')
            and hasattr(os, 'WNOWAIT') and signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL,
            'kernel lifetime and non-reaping wait facilities required')


def retire_outer_cache(intent, output, cleanup_output, container_identity):
    """Only a retained exact creation inode permits cache cleanup after wrapper loss."""
    custody.validate_intent(intent)
    path = TMPFS_ROOT / intent['name']
    if not os.path.lexists(path):
        return {'absent': True, 'retired_by_outer': False, 'path': str(path)}
    record = read(output / 'cache-identity.json')
    require(record.get('schema') == 'FerricMatched128TmpfsCacheV1'
            and record.get('identity', {}).get('path') == str(path),
            'retained cache creation identity required; preserve an unbound cache')
    value = Container.__new__(Container)
    value.name, value.id = intent['name'], container_identity['id'] if container_identity is not None else None
    value.cache, value.out = path, cleanup_output
    value.cache_identity, value.cache_retired = record['identity'], False
    value.cache_create_attempted = True
    value.resource_lock = threading.Lock()
    value.retire_cache()
    return {'absent': value.cache_retired, 'retired_by_outer': True, 'path': str(path)}


def supervise(plan, plan_path, plan_sha, engine, out):
    require(plan['ferric'] is not None and plan['reference'] is not None,
            'complete selected native and independent numerical prerequisite required')
    intent = None
    if engine != 'ferric':
        name = 'ferric-matched128-' + engine + '-' + uuid.uuid4().hex[:16]
        intent = {'name': name, 'image': contract.IMAGES[engine]['id'],
                  'label_key': 'ferric.matched128', 'label_value': name}
    write(out / 'launch-intent.json', {'schema': 'FerricNativeHttpLaunchIntentV1',
        'plan_sha256': plan_sha, 'engine': engine, 'container': intent})
    observer = ProbeObserver(plan, engine, out, intent)
    stop = []
    previous = {number: signal.signal(number, lambda number, _frame: stop.append(number))
                for number in (signal.SIGINT, signal.SIGTERM)}
    final = {'schema': 'FerricNativeHttpOuterReceiptV1', 'engine': engine,
        'plan_sha256': plan_sha, 'timing_admitted': False, 'errors': [], 'status': 125}
    def check(initial):
        state = {'resources': resources(out)}
        if initial:
            state['gpu_idle'] = gpu_snapshot(out, 'gpu-preflight')
            state['attribution'] = observer.sample('preflight')
            return state
        wrapper = read(out / 'launch-supervisor.json')['wrapper_identity']
        if (out / 'cleanup-started.json').exists():
            marker = read(out / 'cleanup-started.json')
            require(marker.get('schema') == 'FerricNativeHttpCleanupStartV1'
                    and marker.get('wrapper') == wrapper and marker.get('timing_finished') is True,
                    'exact teardown marker required')
            state['phase'] = 'bounded_teardown_no_timing_admission'
            return state
        phase = 'active' if (out / 'active-started.json').exists() else 'startup'
        state['attribution'] = observer.sample(phase, wrapper)
        return state
    try:
        with gpu_lease(plan):
            base = types.SimpleNamespace(timestamp=lambda: str(time.time_ns()), save=supervisor_save,
                                         require_facilities=facilities)
            argv = ['/usr/bin/python3', '-I', '-B', str(ROOT / 'run_matched128.py'), '--execute',
                '--plan', str(plan_path), '--plan-sha256', plan_sha, '--output-dir', str(out), '--engine', engine,
                '--intent-sha256', digest(out / 'launch-intent.json')]
            try:
                outer = life.supervise_group(base, argv, out, check, lambda: stop[0] if stop else None,
                    duration=MAX_SECONDS, grace=25, kill_wait=5, interval=3)
                final['process_group'] = outer
            finally:
                try:
                    if intent is not None:
                        fallback = out / 'outer-container-cleanup'
                        fallback.mkdir(mode=0o700)
                        def cleanup_command(argv, label, **kwargs):
                            return commands.run(argv, fallback, label, lifecycle=life, save=write, check=False, **kwargs)
                        final['container'] = custody.retire(cleanup_command, intent, observer.container_identity)
                        final['cache'] = retire_outer_cache(intent, out, fallback,
                            final['container']['retained_identity'])
                finally:
                    final['postflight'] = observer.sample('postflight')
                    final['gpu_idle'] = settle_gpu_idle(out)
            require(outer['status'] == 0 and outer['cleanup_ok'] and outer['child_reaped']
                    and not outer['term_sent'] and not outer['kill_sent'] and not outer['errors'],
                    'clean unsignaled outer completion required')
            require(intent is None or not final['container']['stop_sent'], 'outer container fallback invalidates timing')
            require(intent is None or not final['cache']['retired_by_outer'], 'outer cache fallback invalidates timing')
            result = read(out / 'receipt.json')
            require(result.get('schema') == 'FerricNativeMatched128EngineeringReceiptV1'
                    and result.get('engine') == engine and result.get('plan_sha256') == plan_sha
                    and result.get('cleanup_completed') is True and result.get('errors') == [],
                    'complete inner receipt required')
            final['inner_receipt_sha256'] = digest(out / 'receipt.json')
            final['timing_admitted'] = result.get('timing_admitted') is True
            final['status'] = 0
    except BaseException as error:
        final['errors'], final['exception_chain_truncated'] = exception_chain(error)
        final['timing_admitted'] = False
    finally:
        for number, handler in previous.items():
            signal.signal(number, handler)
        write(out / 'outer-receipt.json', final)
    return final['status']


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--plan-sha256', required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--engine', choices=('ferric', 'vllm', 'sglang'), required=True)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--supervise', action='store_true')
    mode.add_argument('--execute', action='store_true')
    parser.add_argument('--intent-sha256')
    args = parser.parse_args()
    require(not (args.execute or args.supervise) or NATIVE_EXECUTION_ENABLED,
            'current55c width HTTP execution disabled pending complete native campaign and CPU qualification')
    require(args.plan.is_absolute() and args.plan.resolve(strict=True) == args.plan, 'canonical pinned launch plan')
    plan = read(args.plan, args.plan_sha256, 32 * 1024**2)
    inputs = contract.validate_plan(plan)
    out = args.output_dir
    require(out.is_absolute() and out.parent == TMPFS_ROOT and out.parent.resolve(strict=True) == out.parent
            and re.fullmatch(r'ferric-native-http-(?:ferric|vllm|sglang)-[a-z0-9-]{1,48}', out.name),
            'closed fresh executable-tmpfs output root')
    if args.execute:
        require(plan.get('cpu_qualification') is not None, 'exact successful remote CPU qualification required')
        require(args.intent_sha256 is not None and out.is_dir() and not out.is_symlink()
                and out.stat().st_uid == os.getuid() and stat.S_IMODE(out.stat().st_mode) == 0o700,
                'execute requires the precreated owned outer output')
        intent = read(out / 'launch-intent.json', args.intent_sha256)
        require(intent.get('schema') == 'FerricNativeHttpLaunchIntentV1'
                and intent.get('plan_sha256') == args.plan_sha256 and intent.get('engine') == args.engine,
                'exact outer launch intent required')
        anchor = life.process(os.getpid())
        require(anchor['pid'] == anchor['group'] == anchor['session'], 'execute requires owned wrapper session')
        deadline = time.monotonic() + 2
        while not (out / 'launch-supervisor.json').exists():
            require(time.monotonic() < deadline, 'outer ownership receipt missing')
            time.sleep(0.01)
        outer = read(out / 'launch-supervisor.json')
        require(outer.get('wrapper_identity') == anchor and outer.get('child_pid') == os.getpid()
                and outer.get('pidfd_opened') is True and outer.get('child_reaped') is False,
                'live unreaped outer ownership is mandatory')
        with life.handling_stop():
            execute(plan, args.plan_sha256, args.engine, out, inputs, intent['container'])
        return 0
    require(args.intent_sha256 is None and not out.exists(), 'fresh outer output and no direct-execute intent')
    tmpfs_status()
    out.mkdir(mode=0o700)
    write(out / 'frozen-plan.json', plan)
    if not args.supervise:
        write(out / 'preparation.json', {'schema': 'FerricNativeHttpPreparationV1',
            'servers_launched': False, 'timing_admitted': False, 'plan_sha256': args.plan_sha256,
            'next_step': 'separate fresh output with --supervise after actual qualification and selected-native binding'})
        return 0
    require(plan.get('cpu_qualification') is not None, 'exact successful remote HTTP CPU qualification required')
    return supervise(plan, args.plan, args.plan_sha256, args.engine, out)


if __name__ == '__main__':
    raise SystemExit(main())
