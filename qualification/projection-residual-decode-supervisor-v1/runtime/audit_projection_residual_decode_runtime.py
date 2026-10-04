"""Select an actual qualified TF4 ELF; reuse the frozen non-native runtime audit."""
import hashlib
import os
from pathlib import Path
import re
import resource
import shutil
import stat
import sys
import types

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
AUDITOR = ('p227-prefix-runtime-audit-v2', '9b80913aa0dac2da2a52bfe6e053a4db09a5e4c5647731fd269f39719e8bb062')
AUDIT_SHA = 'def16c2f69c082fa273596bef0caf7a2af195d07d2ac6f96d944706d5f44a514'
HELPER_SHA = '244c12f62c8a439aa01b44380fc464008b94006bc919201c8d510c64177fc820'
CPU_LABEL = 'projection-residual-decode-cpu-v228-v1'
CPU_SCHEMA = 'ferric-projection-residual-decode-cpu-result-v1'
PRIOR_CPU_SHA = 'bf1a12f78981d9ff9b8157e1dec6dca300752b238680e16380b98e9d1260bafb'
NAMES = dict(parent='ferric-qwen3-finite-projection-residual-decode-engineering',
             worker='ferric-tp-peer-finite-engineering-worker-v1')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def bootstrap():
    path = E / AUDITOR[0] / 'run_row_facts_v2.py'
    require(path.resolve(strict=True) == path and not path.is_symlink(), 'canonical frozen custody helper')
    with path.open('rb') as stream:
        before = os.fstat(stream.fileno()); raw = stream.read((1 << 20) + 1); after = os.fstat(stream.fileno())
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stat.S_ISREG(before.st_mode) and 0 < len(raw) == before.st_size <= 1 << 20
        and stamp(before) == stamp(after) == stamp(path.lstat())
        and hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'exact frozen custody helper bytes')
    module = types.ModuleType('projection_residual_decode_runtime_custody'); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module, path


def read_pin(pins, record, maximum=16 << 20):
    require(type(record) is dict and set(record) == {'path', 'bytes', 'sha256'}
        and type(record['path']) is str and type(record['bytes']) is int
        and 0 < record['bytes'] <= maximum and type(record['sha256']) is str
        and re.fullmatch('[0-9a-f]{64}', record['sha256']), 'closed nonempty FilePin')
    actual, raw = pins.read(Path(record['path']), record['sha256'], retain=True, maximum=maximum)
    require(actual == record, 'exact recorded extent and digest')
    return raw


def qualified(value):
    require(value['schema'] == CPU_SCHEMA and value['passed'] is True
        and value['error'] is None and value['postcheck_errors'] == []
        and value['source_unchanged'] is True and value['empty_initial_target'] is True
        and all(value[k] is False for k in
            ('gpu_execution', 'numerical_acceptance', 'performance_claim', 'production_authority')),
        'successful CPU-only qualification and postchecks')
    require(value['prior_completion']['path'] == str(E / 'projection-residual-runtime-cpu-v228-v1/complete.json')
        and value['prior_completion']['sha256'] == PRIOR_CPU_SHA
        and value['prior_completion']['bytes'] == 569967, 'unchanged actual CPU988 prerequisite')
    require(set(value['metadata']) == {'parent', 'worker'}
        and len(value['phases']) == 87 and len(value['binaries']) == 17,
        'declared joint phase and executable roster')
    for phase in value['phases'].values():
        require(type(phase['exit_code']) is int and phase['exit_code'] == 0
            and phase['reason'] is None and phase['group_absent'] is True,
            'every declared CPU leaf naturally completed with no group')
    passed, ignored = 0, 0
    require(type(value['tests']) is dict and value['tests'], 'actual test records')
    for result in value['tests'].values():
        require(type(result['passed']) is int and result['passed'] >= 0
            and type(result['ignored']) is int and result['ignored'] >= 0
            and len(result['names']) == len(set(result['names'])) == result['passed'] + result['ignored']
            and all(type(n) is int and n >= 0 for row in result['summaries'] for n in row)
            and all(len(row) == 3 and row[1] == 0 for row in result['summaries'])
            and sum(row[0] for row in result['summaries']) == result['passed']
            and sum(row[2] for row in result['summaries']) == result['ignored'],
            'actual named passing/ignored test accounting')
        passed += result['passed']; ignored += result['ignored']
    require(type(value['tests_passed']) is int and passed == value['tests_passed'] > 0
        and type(value['tests_ignored']) is int and ignored == value['tests_ignored'] == 4,
        'actual counts, not a predicted passing result')
    return value


def cpu_artifact(D, pins, cpu_pin, role):
    value = qualified(D.parse(read_pin(pins, cpu_pin, 4 << 20)))
    root = E / CPU_LABEL
    require(value['controller']['path'] == str(E / 'p228-projection-residual-decode-cpu-v1/run.py'),
            'recorded executed controller namespace')
    read_pin(pins, value['controller'], 128 << 10)
    before, after = (value['raw'][name] for name in ('sources-before.json', 'sources-after.json'))
    require(before['path'] == str(root / 'sources-before.json')
        and after['path'] == str(root / 'sources-after.json'), 'qualified source snapshot namespace')
    sources = D.parse(read_pin(pins, before))
    require(sources and sources == D.parse(read_pin(pins, after)), 'formatted source snapshots unchanged')
    artifact = value['binaries'][NAMES[role]]
    selected, record = artifact['artifact'], artifact['binary']
    directory = 'm1-engineering-execution-v1' if role == 'parent' else 'tp-peer-finite-engineering-worker-v1'
    source = root / 'sources/ferric/adapters' / directory
    require(record['path'] == str(root / 'target' / role / 'debug' / NAMES[role])
        and selected['reason'] == 'compiler-artifact'
        and selected['executable'] == record['path'] and selected['filenames'] == [record['path']]
        and selected['manifest_path'] == str(source / 'Cargo.toml')
        and selected['target']['name'] == NAMES[role]
        and selected['target']['kind'] == selected['target']['crate_types'] == ['bin']
        and selected['target']['src_path'] == str(source / 'src' /
            ('main.rs' if role == 'worker' else 'bin/' + NAMES[role] + '.rs'))
        and selected['profile'] == dict(debug_assertions=True, debuginfo=0,
            opt_level='2', overflow_checks=True, test=False), 'exact selected opt2 executable artifact')
    phase = 'parent-builds' if role == 'parent' else 'worker-build'
    stream = value['raw'][phase + '-stdout']
    require(stream['path'] == str(root / (phase + '-stdout')), 'selected build stream namespace')
    raw = read_pin(pins, stream, 32 << 20)
    rows = [D.parse(line) for line in raw.splitlines() if line.strip()]
    require(sum(row == selected for row in rows) == 1
        and hashlib.sha256(raw).hexdigest() == value['phases'][phase]['stdout_sha256'],
        'actual Cargo artifact occurs once in its retained successful build stream')
    return record


def selected(D, args, pins):
    require(len(args) == 6, 'LABEL ROLE CPU_PATH CPU_SHA BINARY_PATH BINARY_SHA')
    label, role, cpu_path, cpu_sha, binary_path, binary_sha = args
    require(role in NAMES and re.fullmatch(
        'projection-residual-decode-runtime-' + role + r'-v228-v[1-9][0-9]{0,8}', label),
        'closed role and fresh audit label')
    require(all(type(v) is str and re.fullmatch('[0-9a-f]{64}', v) for v in (cpu_sha, binary_sha)),
            'explicit actual CPU and binary SHA256')
    cpu, binary = Path(cpu_path), Path(binary_path)
    root = E / CPU_LABEL
    require(str(cpu) == cpu_path and cpu == root / 'complete.json', 'actual CPU receipt namespace')
    require(str(binary) == binary_path and binary == root / 'target' / role / 'debug' / NAMES[role],
            'exact new joint-qualified executable namespace')
    cpu_pin, _ = pins.read(cpu, cpu_sha, retain=False, maximum=4 << 20)
    original = cpu_artifact(D, pins, cpu_pin, role)
    binary_pin, raw = pins.read(binary, binary_sha, retain=True, maximum=128 << 20)
    require(binary_pin == original and os.access(binary, os.X_OK), 'actual selected executable identity/mode')
    require(len(raw) >= 20 and raw[:6] == b'\x7fELF\x02\x01' and raw[18:20] == b'\x3e\x00',
            'actual ELF64 little-endian x86_64 executable')
    return E / label, binary_pin


def main(args):
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
        and 'PYTHONPATH' not in os.environ and 'PYTHONHOME' not in os.environ
        and os.getuid() == os.geteuid() == 9661 and os.sched_getaffinity(0) == {8, 9}
        and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'same bounded runtime-audit identity')
    require(shutil.disk_usage(E.parents[1]).free >= 40 << 30, 'existing initial disk floor')
    os.umask(0o077)
    for kind, limit in ((resource.RLIMIT_AS, 2 << 30), (resource.RLIMIT_CPU, 120),
                        (resource.RLIMIT_FSIZE, 64 << 20), (resource.RLIMIT_CORE, 0)):
        resource.setrlimit(kind, (limit, limit))
    D, helper = bootstrap(); pins = D.Pins()
    pins.pin(helper, HELPER_SHA); pins.pin(Path(__file__).resolve())
    out, binary = selected(D, args, pins)
    require(not os.path.lexists(out), 'fresh audit output')
    base = D.package(pins, *AUDITOR)
    audit = D.load_module(pins, base / 'audit.py', AUDIT_SHA, 'projection_residual_decode_runtime_auditor')
    audit.host_identity()
    audit.read_pin(pins, binary, 1 << 30)
    topology = D.load_module(pins, audit.TOPOLOGY, audit.TOPOLOGY_SHA, 'projection_residual_decode_runtime_topology')
    owned = D.load_module(pins, base / 'frozen_owned.py', audit.OWNED_SHA, 'projection_residual_decode_runtime_owned')
    record = audit.execute(out, binary, pins, owned, topology)
    D.progress(dict(complete=record, gpu_execution=False, reviewed=False))


if __name__ == '__main__':
    main(sys.argv[1:])
