"""Data-only retention of one terminal native attempt, including failures."""
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

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-interleaved-debugger-gpu-v228-v3'
ARCHIVE = E / 'guarded-mlp-interleaved-debugger-evidence-v228-v3.tar.gz'


def require(value, message):
    if not value:
        raise RuntimeError(message)


def stamp(s):
    return (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_uid, s.st_gid,
            s.st_size, s.st_mtime_ns, s.st_ctime_ns)


def read(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical file')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= 64 << 20, 'bounded ordinary body')
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'open identity')
        body = stream.read((64 << 20) + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'read identity')
    require(len(body) == before.st_size and stamp(path.lstat()) == stamp(before), 'post identity')
    return body, dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    require(len(sys.argv) == 2 and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'actual terminal SHA required')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(not os.path.lexists(ARCHIVE), 'fresh archive')
    signal.alarm(180)
    files, bodies = {}, {}

    def retain(name, path, expected=None):
        require(name not in files and not Path(name).is_absolute() and '..' not in Path(name).parts,
                'unique relative member')
        body, pin = read(path)
        require(expected is None or pin == expected, 'body pin: ' + name)
        require(len(files) < 100 and sum(map(len, bodies.values())) + len(body) < 64 << 20, 'bounded capsule')
        files[name], bodies[name] = pin, body
        return json.loads(body) if name.endswith('.json') else body

    names = {p.name for p in (ROOT / 'evidence').iterdir()}
    terminals = names & {'complete.json', 'failed.json'}
    require(len(terminals) == 1, 'one terminal')
    terminal = next(iter(terminals))
    r = retain('evidence/' + terminal, ROOT / 'evidence' / terminal)
    require(files['evidence/' + terminal]['sha256'] == sys.argv[1]
            and r['schema'] == 'ferric-interleaved-mlp-debugger-result-v1'
            and type(r['passed']) is bool and r['passed'] == (terminal == 'complete.json'), 'actual terminal')
    require(r['paired_coordinator_tested'] == r['owner_lifecycle_tested'] == r['rearm_tested']
            == r['interleaving_tested'] == r['synthetic_paired_component_qualified'] == False,
            'debugger capture does not qualify the component')
    require(r['debugger_assisted'] is True and r['refusal_capture_completed'] == r['passed'], 'debugger scope')
    for key in ('full_model_acceptance', 'production_authority', 'performance_claim', 'worker_integrated'):
        require(r[key] is False, 'component scope only')
    require(names == set(r['raw']) | {terminal} and r['retries'] == 0, 'closed raw attempt')
    for name, pin in sorted(r['raw'].items()):
        require(Path(name).name == name and pin['path'] == str(ROOT / 'evidence' / name), 'raw path')
        retain('evidence/' + name, ROOT / 'evidence' / name, pin)
    inputs = retain('input-manifest.json', ROOT / 'input-manifest.json')
    require(files['input-manifest.json']['sha256'] == 'e4f430d7b85f472e728c7cdf3eb5f3e6f62584fbf77fee617e7930533422dd54',
            'actual GPU input identity')
    require(set(inputs['files']) == {'run_gpu.py', 'supervisor.py', 'library_audit.py', 'verify_native.py', 'request.json',
                                    'owned_children.py', 'capture.gdb',
                                    'r1.hsaco', 'mlp.hsaco', 'guarded.hsaco'},
            'closed harness and request')
    for name, pin in inputs['files'].items():
        retain(name, ROOT / name)
        require({k: files[name][k] for k in ('bytes', 'sha256')} == pin, 'input identity')
    if r['cpu_complete'] is not None:
        cpu = retain('lineage/cpu-complete.json', Path(r['cpu_complete']['path']), r['cpu_complete'])
        require(files['lineage/cpu-complete.json']['sha256']
                == '09746cb353c2b1a6aa8e64a6aaefbb6451d9a538893856bac019105831e351f6', 'CPU ancestry')
        require(r['binary'] == cpu['artifacts']['kfd-lib']['pin']
                or (r['binary'] is None and not r['passed']), 'CPU ELF ancestry')
    request = json.loads(bodies['request.json'])
    for index, name in enumerate(('r1.hsaco', 'mlp.hsaco', 'guarded.hsaco')):
        require(request['images'][index] == str(ROOT / name)
                and files[name]['sha256'] == request['image_sha256'][index], 'retained three-image roles')
    for phase in r['phases']:
        label = phase['label']
        require(json.loads(bodies['evidence/' + label + '.result.json']) == phase, 'actual phase result')
        for key in ('command', 'stdout', 'stderr'):
            require(phase[key] == files['evidence/' + Path(phase[key]['path']).name], 'phase raw join')
        if r['passed']:
            require(phase['natural_exit'] is True and phase['exit_code'] == (101 if label == 'native' else 0)
                    and phase['reaped'] is True
                    and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
                    and phase['timed_out'] is False and phase['exception'] is None
                    and phase['storage_failure'] is None, 'successful owned phase')
    if r['passed']:
        require(r['errors'] == r['postcheck_errors'] == [] and r['gpu_attempts'] == 1
                and r['completed_native_phases'] == 1 and r['native_spawn_observed'] is True
                and r['refusal_capture_completed'] is True, 'diagnostic success joins')
        require(r['observation'] == json.loads(bodies['evidence/observation.json']), 'predicate body join')
        require(r['token_chain'] == json.loads(bodies['evidence/token-chain.json']), 'token chain body join')
        lines = bodies['evidence/native.stdout'].splitlines()
        require([json.loads(line.split(b'=', 1)[1]) for line in lines
                 if line.startswith(b'FERRIC_ROLE_PREDICATE_V1=')] == [r['observation']], 'raw predicate join')
        require([json.loads(line.split(b'=', 1)[1]) for line in lines
                 if line.startswith(b'FERRIC_DEBUGGER_EXIT_V1=')]
                == [dict(mode='native', exit_codes=[101], hits=1)], 'natural inferior refusal')
        require(len(r['owned_retirements']) == len(r['phases'])
                and all(row['complete'] and not row['forced'] and not row['deferred_signals']
                        for row in r['owned_retirements']), 'natural owned descendants')
    retain('stage_gpu.py', E / 'stage_interleaved_mlp_debugger_v228_v3.py')
    require(files['stage_gpu.py']['sha256'] == 'f41512cdc81856714957807cea41d5136a415e0645df6adbe36808ae3e5c1eea', 'actual GPU stager')
    retain('export_gpu.py', Path(__file__).resolve())
    manifest = dict(schema='ferric-interleaved-mlp-debugger-retention-v1', files=files,
                    receipt=files['evidence/' + terminal], passed=r['passed'],
                    exported_host_executable_bodies=False, retained_hsaco=True,
                    binary=r['binary'], gpu_attempts=r['gpu_attempts'],
                    debugger_assisted=True, refusal_capture_completed=r['passed'],
                    paired_coordinator_tested=False, full_model_acceptance=False, performance_claim=False)
    bodies['retention-manifest.json'] = (json.dumps(manifest, sort_keys=True, indent=2) + '\n').encode()
    with ARCHIVE.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, body in sorted(bodies.items()):
            info = tarfile.TarInfo(name)
            info.size, info.mode, info.mtime = len(body), 0o600, 0
            archive.addfile(info, io.BytesIO(body))
    require(all(read(Path(row['path']))[1] == row for row in files.values()), 'post-export body drift')
    print(json.dumps(dict(archive=read(ARCHIVE)[1], members=len(bodies),
                         expanded_bytes=sum(map(len, bodies.values())), passed=r['passed']), sort_keys=True))


if __name__ == '__main__':
    main()
