"""Retain the failed first CPU attempt as data, without relaunching it."""
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import stat
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-retained-pair-cpu-v228-v1'
ARCHIVE = E / 'guarded-mlp-retained-pair-cpu-failure-v228-v1.tar.gz'


def read(path):
    before = path.lstat()
    assert path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
    assert before.st_size <= 64 << 20
    body = path.read_bytes()
    after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    assert len(body) == before.st_size and stamp(before) == stamp(after)
    return body, dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    assert __debug__ and os.getuid() == os.geteuid() == 9661
    assert os.uname().nodename == 'smci350-rck-g03-b19-03' and not os.path.lexists(ARCHIVE)
    signal.alarm(180)
    files, bodies = {}, {}

    def retain(name, path, expected=None):
        assert name not in bodies and not Path(name).is_absolute() and '..' not in Path(name).parts
        body, pin = read(path)
        assert expected is None or pin == expected
        assert len(files) < 128 and sum(map(len, bodies.values())) + len(body) < 64 << 20
        files[name], bodies[name] = pin, body
        return json.loads(body) if name.endswith('.json') else body

    r = retain('evidence/failed.json', ROOT / 'evidence/failed.json')
    assert files['evidence/failed.json']['bytes'] == 989999
    assert files['evidence/failed.json']['sha256'] == '64d5aa6ede01ec3bac7b12ec849259df6b602c51cba7dc38476b3b99bc00562c'
    assert r['passed'] is False and r['postcheck_errors'] == [] and r['source_unchanged'] is True
    assert r['failure'] == "RuntimeError('kfd-tests did not finish naturally/reaped/successfully')"
    assert r['input_sources'] == r['final_sources'] and len(r['input_sources']) == 803
    assert r['gpu_execution'] is False and r['production_authority'] is False and r['performance_claim'] is False
    phases = ('rustc-version', 'metadata', 'default-check', 'kfd-tests-build', 'kfd-list', 'kfd-ignored', 'kfd-tests')
    assert [p['label'] for p in r['phases']] == list(phases)
    for i, phase in enumerate(r['phases']):
        assert phase['exit_code'] == (101 if i == 6 else 0)
        assert all(phase[k] is True for k in ('natural_exit', 'reaped', 'process_group_absent'))
        assert all(phase[k] is False for k in ('forced_cleanup', 'timed_out'))
        assert phase['exception'] is None and phase['storage_failure'] is None
        assert not Path('/proc', str(phase['pid'])).exists()
    expected = {p + s for p in phases for s in ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
    expected |= {'sources-before.json', 'sources-after.json', 'dependencies-before.json', 'dependencies-after.json'}
    assert set(r['raw']) == expected
    assert {p.name for p in (ROOT / 'evidence').iterdir()} == expected | {'failed.json'}
    for name, pin in sorted(r['raw'].items()):
        retain('evidence/' + name, ROOT / 'evidence' / name, pin)
    for phase in r['phases']:
        assert json.loads(bodies['evidence/' + phase['label'] + '.result.json']) == phase
    for name in ('sources-before.json', 'sources-after.json'):
        assert json.loads(bodies['evidence/' + name]) == r['input_sources']
    inputs = retain('input-manifest.json', ROOT / 'input-manifest.json', r['input_manifest'])
    assert files['input-manifest.json']['sha256'] == 'c8cfe9d1ae45b3fcc56d7f6c45e1839f2b2b28c4c601a390100a444821082aef'
    assert inputs['source_lineage'] == r['source_lineage']
    assert inputs['files'] == {n: {k: row[k] for k in ('bytes', 'sha256')} for n, row in r['input_sources'].items()}
    for name, key in (('run_cpu.py', 'controller'), ('supervisor.py', 'supervisor')):
        retain(name, ROOT / name, r[key])
    selected = {'fe2o3/' + n for n in inputs['source_lineage']['overlay']}
    selected |= {'fe2o3/' + n for n in ('Cargo.toml', 'Cargo.toml.original', 'Cargo.lock', 'Cargo.lock.input', 'rust-toolchain.toml')}
    for name in sorted(selected):
        retain(name, ROOT / name, r['final_sources'][name])
    for name in ('base_complete', 'base_sources', 'base_stdout', 'base_controller'):
        suffix = '.stdout' if name == 'base_stdout' else '.py' if name == 'base_controller' else '.json'
        row = inputs['source_lineage'][name]
        retain('lineage/' + name + suffix, Path(row['path']), row)
    for name in ('rustfmt.command.json', 'rustfmt.started.json', 'rustfmt.result.json', 'rustfmt.stdout',
                 'rustfmt.stderr', 'format-outcome.json', 'complete.json'):
        retain('format/' + name, ROOT / 'format' / name)
    retain('stage_cpu.py', E / 'stage_guarded_mlp_retained_pair_v228_v1.py')
    assert files['stage_cpu.py']['sha256'] == '74073d627a0a1c1f36347dd162fdd4659e3b30b511772ae89e2f31a05008b74b'
    retain('export_failed_cpu.py', Path(__file__).resolve())
    assert len(files) == 68
    manifest = dict(schema='ferric-retained-pair-failed-cpu-retention-v1', files=files,
                    receipt=files['evidence/failed.json'], passed=False, retention_passed=True,
                    gpu_execution=False, performance_claim=False, exported_binary_bodies=False)
    bodies['retention-manifest.json'] = (json.dumps(manifest, sort_keys=True, indent=2) + '\n').encode()
    with ARCHIVE.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, body in sorted(bodies.items()):
            info = tarfile.TarInfo(name)
            info.size, info.mode, info.mtime = len(body), 0o600, 0
            archive.addfile(info, io.BytesIO(body))
    assert all(read(Path(row['path']))[1] == row for row in files.values())
    print(json.dumps(dict(archive=read(ARCHIVE)[1], members=len(bodies), expanded_bytes=sum(map(len, bodies.values())))))


if __name__ == '__main__':
    main()
