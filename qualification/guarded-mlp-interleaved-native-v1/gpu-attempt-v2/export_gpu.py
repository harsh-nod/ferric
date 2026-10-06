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
ROOT = E / 'guarded-mlp-interleaved-native-gpu-v228-v2'
ARCHIVE = E / 'guarded-mlp-interleaved-native-gpu-evidence-v228-v2.tar.gz'


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
            and r['schema'] == 'ferric-native-interleaved-mlp-gpu-result-v1'
            and type(r['passed']) is bool and r['passed'] == (terminal == 'complete.json'), 'actual terminal')
    require(r['paired_coordinator_tested'] == r['owner_lifecycle_tested'] == r['rearm_tested']
            == r['interleaving_tested'] == r['passed'], 'paired qualification scope')
    for key in ('full_model_acceptance', 'production_authority', 'performance_claim', 'worker_integrated'):
        require(r[key] is False, 'component scope only')
    require(names == set(r['raw']) | {terminal} and r['retries'] == 0, 'closed raw attempt')
    for name, pin in sorted(r['raw'].items()):
        require(Path(name).name == name and pin['path'] == str(ROOT / 'evidence' / name), 'raw path')
        retain('evidence/' + name, ROOT / 'evidence' / name, pin)
    inputs = retain('input-manifest.json', ROOT / 'input-manifest.json')
    require(files['input-manifest.json']['sha256'] == '50b4090bb8fba84b7747abeb9cba76a768f4ac437705498daf94af127ca40f77',
            'actual GPU input identity')
    require(set(inputs['files']) == {'run_gpu.py', 'supervisor.py', 'library_audit.py', 'verify_native.py', 'request.json',
                                    'r1.hsaco', 'mlp.hsaco', 'guarded.hsaco'},
            'closed harness and request')
    for name, pin in inputs['files'].items():
        retain(name, ROOT / name)
        require({k: files[name][k] for k in ('bytes', 'sha256')} == pin, 'input identity')
    if r['cpu_complete'] is not None:
        cpu = retain('lineage/cpu-complete.json', Path(r['cpu_complete']['path']), r['cpu_complete'])
        require(files['lineage/cpu-complete.json']['sha256']
                == '3adc90f6f97584787386b2d150e61a05a6ac5ea50d819111530b07755b98d47f', 'CPU ancestry')
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
            require(phase['natural_exit'] is True and phase['exit_code'] == 0 and phase['reaped'] is True
                    and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
                    and phase['timed_out'] is False and phase['exception'] is None
                    and phase['storage_failure'] is None, 'successful owned phase')
    if r['passed']:
        require(r['errors'] == r['postcheck_errors'] == [] and r['gpu_attempts'] == 1
                and r['completed_native_phases'] == 1 and r['native_spawn_observed'] is True
                and r['synthetic_paired_component_qualified'] is True
                and r['verification']['stdout_sha256'] == files['evidence/native.stdout']['sha256']
                and r['verification']['request_sha256'] == files['request.json']['sha256'], 'success joins')
        require(r['observation'] == json.loads(bodies['evidence/observation.json'])
                and r['verification'] == json.loads(bodies['evidence/verify.stdout']), 'native/verifier body joins')
    retain('stage_gpu.py', E / 'stage_native_interleaved_mlp_gpu_v228_v2.py')
    require(files['stage_gpu.py']['sha256'] == '59c5a490bbececc427dd315418408fbeca4d398836503d549097027878453665', 'actual GPU stager')
    retain('export_gpu.py', Path(__file__).resolve())
    manifest = dict(schema='ferric-native-interleaved-mlp-gpu-retention-v1', files=files,
                    receipt=files['evidence/' + terminal], passed=r['passed'],
                    exported_host_executable_bodies=False, retained_hsaco=True,
                    binary=r['binary'], gpu_attempts=r['gpu_attempts'],
                    paired_coordinator_tested=r['passed'], full_model_acceptance=False, performance_claim=False)
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
