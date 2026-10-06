"""Authenticate and copy a fresh worker source tree; execute no project code."""

import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys
import tarfile
import time


E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-host-shared-currentness-worker-cpu-v228-v1'
INTERFACE = E / 'guarded-mlp-model-interface-cpu-v228-v1'
ARCHIVE = E / 'guarded-mlp-host-shared-currentness-worker-input-v228-v1.tar.gz'
PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
REVISION = '1371774221e0b271e4aac1381c3df6f6d4a4592d'
CONTROLLER_PIN = dict(bytes=37166, sha256='fa63ab2dac6557177a8f75d8bfa17d48feff728ca66943fbadc4ed6f4d313ebd')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
INTERFACE_PINS = {
    'complete.json': dict(bytes=1287578, sha256='4a682798a23ac4c8accb0721f332a7b4e7484729bd692209d20f2883b501cee1'),
    'sources-after.json': dict(bytes=321886, sha256='569f5cdb89338ac9a8c8f0e759e455d876471be1e8f75989600cf0d1af185161'),
}
HISTORY = {
    'worker_complete': ('worker-complete.json', 1625743,
        'fd4b55f53e2a61aafb49bb901796ca6c5d72a2e34d725aabc16cd4d14cef082e'),
    'worker_tests': ('worker-tests.stdout', 71326,
        '078fe59cdff580ee766107ed36c115eb336b6ed7088e7c13211fbf1c332cac4f'),
    'worker_list': ('worker-list.stdout', 67068,
        '2597f965833ef33d2a11fe120b4ad1bcc91ea8843f7de3e938af176747b9c5e7'),
    'worker_sources': ('worker-sources.json', 408555,
        '7dfba5bd71c4baffe607b89145d00240dbc667c782da02d669b77ca090ba757b'),
    'shared_proposal': ('shared-source-manifest.json', 9148,
        '5d0821afe584e59c145c05b0999f67ff24d090e094a3e0accc68c93601ae2456'),
}
DEADLINE = time.monotonic() + 180
CREATED = False


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def ordinary_name(name):
    return (type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute()
            and '..' not in Path(name).parts and name not in ('', '.'))


def read(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 0 <= before.st_size <= 16 << 20, 'bounded ordinary staging input: ' + str(path))
    body = path.read_bytes()
    after = path.lstat()
    stamp = lambda value: (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
                           value.st_size, value.st_mtime_ns, value.st_ctime_ns)
    require(len(body) == before.st_size and stamp(before) == stamp(after), 'staging input drift')
    return body


def write(name, body):
    require(ordinary_name(name), 'ordinary stage destination')
    path = ROOT / name
    path.parent.mkdir(parents=True, exist_ok=True)
    require(path.parent.resolve(strict=True) == path.parent, 'stage parent alias')
    with path.open('xb') as stream:
        stream.write(body)


def main():
    global CREATED
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 3
            and all(re.fullmatch(r'[0-9a-f]{64}', value) for value in sys.argv[1:]),
            'python3 -B stage_worker_cpu.py ARCHIVE_SHA INPUT_SHA')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID mismatch')
    require(E.resolve(strict=True) == E and not os.path.lexists(ROOT), 'fresh exact staging root')
    require(os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 40 << 30, '40 GiB initial free floor')
    archive_body = read(ARCHIVE)
    require(pin(archive_body)['sha256'] == sys.argv[1], 'actual archive pin')
    with tarfile.open(fileobj=io.BytesIO(archive_body), mode='r:gz') as tar:
        members = tar.getmembers()
        require(len(members) == 193 and len({member.name for member in members}) == len(members)
                and all(member.isfile() and ordinary_name(member.name)
                        and 0 <= member.size <= 16 << 20 for member in members)
                and sum(member.size for member in members) <= 32 << 20,
                'bounded unique ordinary archive members')
        bodies = {member.name: tar.extractfile(member).read() for member in members}
    manifest_body = bodies['input-manifest.json']
    require(pin(manifest_body)['sha256'] == sys.argv[2], 'actual input-manifest pin')
    inputs = json.loads(manifest_body)
    require(set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins', 'source_lineage',
                            'worker_overlay', 'new_tests'}
            and inputs['schema'] == 'ferric-guarded-mlp-host-shared-currentness-worker-cpu-input-v1'
            and inputs['source_generation'] == GENERATION and len(inputs['files']) == 993,
            'closed worker CPU input contract')
    files = inputs['files']
    require(all(ordinary_name(name) and type(row) is dict and set(row) == {'bytes', 'sha256'}
                and type(row['bytes']) is int and 0 <= row['bytes'] <= 16 << 20
                and type(row['sha256']) is str and re.fullmatch(r'[0-9a-f]{64}', row['sha256'])
                for name, row in files.items()), 'closed source file pins')
    require(sum(row['bytes'] for row in files.values()) <= 64 << 20, 'total source extent bound')
    runtime = {name: row for name, row in files.items() if name.startswith('fe2o3/')}
    worker = {name: row for name, row in files.items() if name.startswith(PREFIX)}
    require(len(runtime) == 807 and len(worker) == 184
            and set(files) == set(runtime) | set(worker) | {'run_cpu.py', 'supervisor.py'},
            'exact runtime/worker/controller rosters')
    extras = {'input-manifest.json', 'inputs/worker-source.json',
              *('inputs/' + row[0] for row in HISTORY.values())}
    require(set(bodies) == set(worker) | {'run_cpu.py', 'supervisor.py'} | extras,
            'no runtime bodies or unlisted payloads in worker transport')
    require(files['run_cpu.py'] == pin(bodies['run_cpu.py']) == CONTROLLER_PIN
            and files['supervisor.py'] == pin(bodies['supervisor.py']) == SUPERVISOR_PIN,
            'reviewed controller/supervisor identity')
    require(all(pin(bodies[name]) == row for name, row in worker.items()), 'all worker body pins')
    lineage = inputs['source_lineage']
    require(set(lineage) == {'interface_complete', 'interface_sources', 'worker_snapshot', *HISTORY},
            'closed source lineage roster')
    before_interface = {}
    for key, name in (('interface_complete', 'complete.json'), ('interface_sources', 'sources-after.json')):
        path = INTERFACE / 'evidence' / name
        body = read(path)
        require(pin(body) == INTERFACE_PINS[name]
                and lineage[key] == dict(path=str(path), **pin(body)), 'qualified interface raw pins')
        before_interface[path] = body
    base = json.loads(before_interface[INTERFACE / 'evidence/complete.json'])
    source_map = json.loads(before_interface[INTERFACE / 'evidence/sources-after.json'])
    require(base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['gpu_execution'] is False
            and base['input_sources'] == base['final_sources'] == source_map and len(source_map) == 809
            and base['source_generation'] == GENERATION and base['tool_pins'] == inputs['tool_pins'],
            'qualified interface receipt and immutable toolchain join')
    require(all(ordinary_name(name) and row['path'] == str(INTERFACE / name)
                for name, row in source_map.items()), 'interface source-map paths')
    expected_runtime = {name: {key: row[key] for key in ('bytes', 'sha256')}
                        for name, row in source_map.items() if name.startswith('fe2o3/')}
    require(runtime == expected_runtime, 'runtime identity exactly matches qualified interface')
    for key, (name, size, digest) in HISTORY.items():
        body = bodies['inputs/' + name]
        require(pin(body) == dict(bytes=size, sha256=digest)
                and lineage[key] == dict(path=str(ROOT / 'inputs' / name), **pin(body)),
                'historical worker raw body join')
    snapshot_body = bodies['inputs/worker-source.json']
    require(lineage['worker_snapshot'] == dict(path=str(ROOT / 'inputs/worker-source.json'), **pin(snapshot_body)),
            'worker source snapshot identity')
    snapshot = json.loads(snapshot_body)
    require(set(snapshot) == {'schema', 'revision', 'files', 'overlay'}
            and snapshot['schema'] == 'ferric-guarded-mlp-worker-source-v1'
            and snapshot['revision'] == REVISION and len(snapshot['files']) == 184
            and len(snapshot['overlay']) == 6, 'exact worker Git-source ancestry')
    reconstructed = dict(snapshot['files'])
    for name, row in snapshot['overlay'].items():
        require(ordinary_name(name) and name.startswith(PREFIX + 'src/') and name.endswith('.rs')
                and set(row) == {'before', 'after'} and reconstructed.get(name) == row['before'],
                'worker source overlay preimage')
        reconstructed[name] = row['after']
    require(reconstructed == worker and sum(row['before'] is None for row in snapshot['overlay'].values()) == 0
            and inputs['worker_overlay'] == sorted(snapshot['overlay']), 'exact worker overlay postimages')
    tests = inputs['new_tests']
    require(set(tests) == {'worker-lib', 'worker-bin-test', 'worker-wire-test'}
            and len(tests['worker-lib']) == 6 and tests['worker-lib'] == sorted(set(tests['worker-lib']))
            and all(type(name) is str and re.fullmatch(r'[A-Za-z0-9_:]+', name) for name in tests['worker-lib'])
            and tests['worker-bin-test'] == [] and tests['worker-wire-test'] == [], 'explicit new test roster')
    old_sources = json.loads(bodies['inputs/worker-sources.json'])
    require(snapshot['files'] == {n: {k: r[k] for k in ('bytes', 'sha256')}
            for n, r in old_sources.items() if n.startswith(PREFIX)}, 'all184 qualified host-observation preimages')
    capture = json.loads(bodies['inputs/shared-source-manifest.json'])
    capture_rows = {'ferric/' + r['path']: {k: r[k] for k in ('before', 'after')}
                    for r in capture['files'] if ('ferric/' + r['path']).startswith(PREFIX)}
    require(capture['canonical_revision'] == REVISION and capture_rows == snapshot['overlay']
            and tests['worker-lib'] == capture['new_tests']['worker_library'], 'exact shared source/test joins')
    os.umask(0o077)
    ROOT.mkdir(mode=0o700)
    CREATED = True
    for name, expected in sorted(runtime.items()):
        body = read(INTERFACE / name)
        require(pin(body) == expected, 'qualified runtime body drift: ' + name)
        write(name, body)
    for name, body in sorted(bodies.items()):
        write(name, body)
    for name, expected in files.items():
        require(pin(read(ROOT / name)) == expected, 'staged source pin differs')
    for name, expected in runtime.items():
        require(pin(read(INTERFACE / name)) == expected,
                'qualified parent runtime changed during staging')
    require(all(read(path) == body for path, body in before_interface.items()), 'interface evidence drift')
    require(read(ARCHIVE) == archive_body and read(ROOT / 'input-manifest.json') == manifest_body,
            'archive/input changed during staging')
    require(time.monotonic() < DEADLINE, 'whole staging deadline')
    result = dict(passed=True, schema='ferric-guarded-mlp-host-shared-currentness-worker-stage-v1',
        archive=dict(path=str(ARCHIVE), **pin(archive_body)),
        input_manifest=dict(path=str(ROOT / 'input-manifest.json'), **pin(manifest_body)),
        source_files=len(files), runtime_files=807, worker_files=184, archive_members=len(bodies),
        qualified_parent_unchanged=True, formatting_executed=False, compiler_executed=False,
        project_code_executed=False, gpu_execution=False)
    write('stage-complete.json', (json.dumps(result, sort_keys=True, indent=2) + '\n').encode('utf-8'))
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    def interrupted(signum, _frame):
        raise RuntimeError('source staging interrupted: ' + str(signum))
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(signum, interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, DEADLINE - time.monotonic()))
    try:
        main()
    except BaseException as error:
        signal.setitimer(signal.ITIMER_REAL, 0)
        if CREATED:
            write('stage-failed.json', (json.dumps(dict(passed=False, failure=repr(error),
                formatting_executed=False, compiler_executed=False, project_code_executed=False,
                gpu_execution=False), sort_keys=True, indent=2) + '\n').encode('utf-8'))
        raise
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
