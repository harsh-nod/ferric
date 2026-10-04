"""Read-only MI350 runtime observations, never an engineering review or GPU grant."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import stat
import sys
import time
import types

HELPER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'
OWNED_SHA = 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'
TOPOLOGY_SHA = '6016c30f46aa32abf9d175f329a0110e86cbf79d1363f1803dba3c86c1bad50a'
HOST = 'smci350-rck-g03-b19-03'
BINARY = 'gfx950-qwen-prefix-tiles-comparison-v6'
DEVICES = [16366993098680759275, 10838076764495710945]
ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8', LC_ALL='C.UTF-8',
           TZ='UTC', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1',
           HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
STREAM = 1 << 20
FLAGS = dict(authority='none', reviewed=False, gpu_execution=False, production_authority=False,
             runtime_premises_discharged=False, numerical_acceptance=False)


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def bootstrap():
    path = Path(__file__).resolve().with_name('run_row_facts_v2.py')
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((1 << 20) + 1)
        after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and len(raw) == before.st_size <= 1 << 20
        and stamp(before) == stamp(after) == stamp(path.lstat())
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'exact frozen custody helper')
    module = types.ModuleType('prefix_runtime_custody'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


D = bootstrap()
E, R = D.E, D.E.parents[1]
TOPOLOGY = R / 'evidence/resident-output-tp2-v217/run_p217_mi350.py'


def save(path, value):
    raw = (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()
    require(len(raw) <= 8 << 20, 'bounded observation JSON')
    with path.open('xb') as stream:
        stream.write(raw); stream.flush(); os.fsync(stream.fileno())
    return D.read_file(path, maximum=8 << 20)[0]


def read_pin(pins, record, maximum, retain=False):
    require(type(record) is dict and set(record) == {'path', 'bytes', 'sha256'}
        and type(record['bytes']) is int and 0 <= record['bytes'] <= maximum
        and type(record['path']) is str and type(record['sha256']) is str
        and re.fullmatch('[0-9a-f]{64}', record['sha256']), 'exact bounded FilePin')
    actual, raw = pins.read(Path(record['path']), record['sha256'], retain, maximum)
    require(actual == record, 'input extent and digest')
    return raw


def topology_identity(sample):
    require(type(sample) is dict and type(sample.get('devices')) is list
        and len(sample['devices']) == 2, 'paired topology sample')
    rows = []
    for row, uid in zip(sample['devices'], DEVICES):
        require(type(row['unique_id']) is int and row['unique_id'] == uid, 'ordered selected UID')
        rows.append({k: v for k, v in row.items() if k not in ('gpu_busy', 'memory_busy', 'vram_used')})
    return rows


def host_identity():
    host = os.uname().nodename
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    require(host == HOST and re.fullmatch(r'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}', boot),
            'actual expected host and boot identity')
    return dict(host=host, boot_id=boot)


def library_paths(readelf, ldd):
    needed = re.findall(r'\(NEEDED\).*Shared library: \[([^\]\n]+)\]', readelf)
    require(needed and len(needed) == len(set(needed)), 'unique actual ELF dependencies')
    paths, named, vdso = [], set(), 0
    for line in ldd.splitlines():
        line = line.strip()
        if re.fullmatch(r'linux-vdso\.so\.1 \(0x[0-9a-fA-F]+\)', line):
            vdso += 1; continue
        linked = re.fullmatch(r'(\S+) => (/\S+) \(0x[0-9a-fA-F]+\)', line)
        direct = re.fullmatch(r'(/\S+) \(0x[0-9a-fA-F]+\)', line)
        require(linked is not None or direct is not None, 'closed ldd output')
        path = linked.group(2) if linked else direct.group(1)
        require(path not in paths and '..' not in Path(path).parts and str(Path(path)) == path,
                'unique normalized library aliases')
        paths.append(path)
        name = linked.group(1) if linked else Path(path).name
        require(linked is not None or name == 'ld-linux-x86-64.so.2', 'exact direct ELF interpreter')
        require(name not in named, 'unique dependency name across linked and direct rows')
        named.add(name)
    require(vdso == 1 and 1 <= len(paths) <= 128 and set(needed) <= named,
            'complete observed dynamic dependency closure')
    return paths


def canonical_libraries(pins, paths):
    result = []
    for name in paths:
        path = Path(name); resolved = path.resolve(strict=True)
        record = pins.read(resolved, maximum=128 << 20)[0]
        require(record['bytes'] > 0 and path.resolve(strict=True) == resolved, 'stable library alias')
        result.append(dict(path=name, resolved=record))
    return result


def alias_guard(rows):
    for row in rows:
        require(Path(row['path']).resolve(strict=True) == Path(row['resolved']['path']),
                'library/tool alias changed')


def observed_file(path):
    """Pseudo-files have no trustworthy st_size; retain their bounded observed bytes."""
    if not path.exists():
        require(not path.is_symlink(), 'dangling software identity alias')
        return dict(path=str(path), present=False)
    resolved = path.resolve(strict=True)
    with resolved.open('rb') as stream:
        require(stat.S_ISREG(os.fstat(stream.fileno()).st_mode), 'regular software observation')
        raw = stream.read((64 << 10) + 1)
    require(len(raw) <= 64 << 10 and path.resolve(strict=True) == resolved, 'bounded software identity')
    return dict(path=str(path), resolved=str(resolved), present=True,
                observed_bytes=len(raw), observed_sha256=hashlib.sha256(raw).hexdigest(),
                text=raw.decode('utf-8'))


def software_snapshot():
    paths = [Path(p) for p in ('/etc/os-release', '/proc/version', '/sys/module/amdgpu/version',
                              '/sys/module/amdgpu/srcversion', '/opt/rocm/.info/version')]
    extra = sorted(Path('/opt/rocm/.info').glob('version-*'))
    require(len(extra) <= 32, 'bounded ROCm version-file roster')
    paths.extend(p for p in extra if p not in paths)
    value = os.uname()
    return dict(schema='ferric-p227-runtime-software-observation-v1', uname=dict(
        sysname=value.sysname, nodename=value.nodename, release=value.release,
        version=value.version, machine=value.machine), files=[observed_file(p) for p in paths],
        compiler_queried=False, **FLAGS)


def clean_owner(value):
    require(type(value.get('exit_code')) is int and value['exit_code'] == 0
        and value.get('reason') is None and value.get('owned_groups_absent') is True
        and value.get('owned_processes_reaped') is True and value.get('cleanup_signalled') is False,
        'clean successful bounded audit child')


def leaf(out, name, argv, owned, guard):
    require((name == 'readelf' and argv[:2] == ['/usr/bin/readelf', '-d'] and len(argv) == 3)
        or (name == 'ldd' and argv[:1] == ['/usr/bin/ldd'] and len(argv) == 2), 'closed audit commands')
    guard(); directory = out / name; directory.mkdir(mode=0o700)
    wrapped = ['/usr/bin/prlimit', '--as=2147483648', '--cpu=30', '--fsize=1048576', '--core=0', '--', *argv]
    command = save(directory / 'command.json', dict(argv=wrapped, audit_argv=argv, cwd=str(R),
        env=ENV, deadline_seconds=30, stream_cap_bytes=STREAM, gpu_execution_requested=False))
    outcome = D.run_coordinator(directory, wrapped, R, ENV, owned, save, deadline=30)
    # Preserve ownership failure before interpreting stream content or producing an audit record.
    owner = save(directory / 'owner.json', dict(outcome=outcome, command=command,
        started=D.read_file(directory / 'started.json', maximum=1 << 20)[0]
            if (directory / 'started.json').exists() else None, gpu_execution=False))
    stdout, raw = D.read_file(directory / 'stdout', retain=True, maximum=STREAM)
    stderr, err = D.read_file(directory / 'stderr', retain=True, maximum=STREAM)
    clean_owner(outcome); require(err == b'', 'audit stderr must be empty'); guard()
    record = save(directory / 'audit.json', dict(argv=argv, exit_code=outcome['exit_code'],
        deadline_seconds=30, stdout=stdout, stderr=stderr))
    return dict(audit=record, owner=owner, command=command), raw.decode('utf-8')


def execute(out, binary, pins, owned, topology):
    started = time.monotonic(); identity = host_identity(); libraries = []
    tools = canonical_libraries(pins, ['/usr/bin/prlimit', '/usr/bin/readelf', '/usr/bin/ldd'])
    def guard():
        require(time.monotonic() - started <= 120, 'bounded total observation interval')
        require(shutil.disk_usage(R).free >= 38 << 30, 'retained free-space floor')
        require(host_identity() == identity, 'host/boot unchanged')
        pins.recheck(); alias_guard(tools); alias_guard(libraries)
    guard(); out.mkdir(mode=0o700)
    save(out / 'inputs.json', dict(binary=binary, tools=tools, source_pins=pins.records))
    try:
        before = topology.topology_sample(); before_pin = save(out / 'topology-before.json', before)
        selected = topology_identity(before)
        software = save(out / 'software.json', software_snapshot())
        elf, elf_text = leaf(out, 'readelf', ['/usr/bin/readelf', '-d', binary['path']], owned, guard)
        dynamic, dynamic_text = leaf(out, 'ldd', ['/usr/bin/ldd', binary['path']], owned, guard)
        libraries.extend(canonical_libraries(pins, library_paths(elf_text, dynamic_text)))
        after = topology.topology_sample(); after_pin = save(out / 'topology-after.json', after)
        require(topology_identity(after) == selected, 'topology identity unchanged')
        guard()
        return save(out / 'complete.json', dict(schema='ferric-p227-prefix-runtime-audit-v1',
            binary=binary, **identity, devices=DEVICES, topology_identity=selected,
            topology_before=before_pin, topology_after=after_pin, software_audit=software,
            readelf=elf['audit'], ldd=dynamic['audit'], owners=[elf['owner'], dynamic['owner']],
            commands=[elf['command'], dynamic['command']], libraries=libraries, tools=tools,
            source_pins=pins.records, elapsed_seconds=time.monotonic() - started, **FLAGS))
    except BaseException as error:
        save(out / 'failed.json', dict(schema='ferric-p227-prefix-runtime-audit-failed-v1',
            error=repr(error), binary=binary, **identity, **FLAGS))
        raise


def arguments(args):
    require(len(args) == 4, 'usage: audit.py LABEL DEPLOYED_BINARY BYTES SHA256')
    label, path, extent, digest = args
    require(re.fullmatch(r'prefix-independent-runtime-audit-v228-v[1-9][0-9]{0,8}', label), 'fresh bounded label')
    require(re.fullmatch(r'[1-9][0-9]{0,9}', extent), 'canonical binary extent')
    binary = dict(path=path, bytes=int(extent), sha256=digest)
    bp = Path(path)
    require(bp.name == BINARY and bp.parent.parent == E
        and re.fullmatch(r'prefix-independent-deployment-v228-v[1-9][0-9]*', bp.parent.name),
        'actual portable deployed binary namespace')
    return label, binary


def main(args):
    label, binary = arguments(args)
    require(not sys.flags.optimize and os.getuid() == os.geteuid() == 9661
        and os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
        'bounded engineering controller identity')
    host_identity(); require(shutil.disk_usage(R).free >= 40 << 30, 'initial free-space floor')
    out = E / label; require(not out.exists() and not out.is_symlink(), 'exclusive output directory')
    bp = Path(binary['path'])
    os.umask(0o077)
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                        (resource.RLIMIT_FSIZE, 64 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (limit, limit))
    pins = D.Pins(); read_pin(pins, binary, 1 << 30)
    with bp.open('rb') as stream:
        require(stream.read(6) == b'\x7fELF\x02\x01', 'ELF64 little-endian audited artifact')
    pins.pin(Path(__file__).resolve()); pins.pin(Path(__file__).with_name('run_row_facts_v2.py'), HELPER_SHA)
    owned = D.load_module(pins, Path(__file__).with_name('frozen_owned.py'), OWNED_SHA, 'runtime_owned')
    topology = D.load_module(pins, TOPOLOGY, TOPOLOGY_SHA, 'runtime_topology')
    record = execute(out, binary, pins, owned, topology)
    D.progress(dict(complete=record, **FLAGS))


if __name__ == '__main__':
    main(sys.argv[1:])
