"""Retain the actual CPU attempt as data; never rebuild or execute a test."""
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import stat
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-paired-native-cpu-v228-v1'
ARCHIVE = E / 'guarded-mlp-paired-native-cpu-evidence-v228-v1.tar.gz'
RECEIPT_SHA = '9153dba525b757cbe9074574287e47eaa522afafd7dad54c13fa1efc814b50a2'
INPUT_SHA = 'bc2d92e3a6239cb0e5a5de2b658df20c1526f3acdf4cff2ab6cafc215b2432ef'
PHASES = ('rustc-version', 'metadata', 'default-check', 'kfd-tests-build',
          'kfd-list', 'kfd-ignored', 'kfd-tests', 'combined-owner-tests', 'combined-memory-tests', 'paired-tests', 'paired-native-tests')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def stamp(s):
    return (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_uid, s.st_gid,
            s.st_size, s.st_mtime_ns, s.st_ctime_ns)


def read(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical path')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= 64 << 20, 'bounded ordinary body')
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'open identity')
        body = stream.read((64 << 20) + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'read identity')
    require(len(body) == before.st_size and stamp(path.lstat()) == stamp(before), 'post identity')
    return body, dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def main():
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(not os.path.lexists(ARCHIVE), 'fresh archive')
    signal.alarm(180)
    files, bodies = {}, {}

    def retain(name, path, expected=None):
        require(name not in files and not Path(name).is_absolute() and '..' not in Path(name).parts,
                'unique relative member')
        body, pin = read(path)
        require(expected is None or pin == expected, 'body pin: ' + name)
        require(len(files) < 100 and sum(map(len, bodies.values())) + len(body) < 64 << 20,
                'bounded capsule')
        files[name], bodies[name] = pin, body
        return json.loads(body) if name.endswith('.json') else body

    r = retain('evidence/complete.json', ROOT / 'evidence/complete.json')
    require(files['evidence/complete.json']['sha256'] == RECEIPT_SHA
            and r['schema'] == 'ferric-guarded-mlp-paired-native-cpu-v1'
            and r['passed'] is True and r['failure'] is None and r['postcheck_errors'] == []
            and r['input_sources'] == r['final_sources'] and r['source_unchanged'] is True,
            'actual successful terminal')
    require(r['coordinator_implemented'] is True, 'private coordinator present')
    for key in ('gpu_execution', 'native_test_executed', 'worker_integrated',
                'native_peer_ordering_qualified', 'production_authority', 'performance_claim'):
        require(r[key] is False, 'CPU-only authority')
    inputs = retain('input-manifest.json', ROOT / 'input-manifest.json', r['input_manifest'])
    require(files['input-manifest.json']['sha256'] == INPUT_SHA and len(inputs['files']) == 796
            and inputs['source_lineage'] == r['source_lineage'], 'input join')
    require({n: {k: p[k] for k in ('bytes', 'sha256')} for n, p in r['input_sources'].items()}
            == inputs['files'], 'all source and harness rows join frozen inputs')
    for name, key in (('run_cpu.py', 'controller'), ('supervisor.py', 'supervisor')):
        retain(name, ROOT / name, r[key])
    allowed = {label + suffix for label in PHASES for suffix in
               ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
    allowed |= {'sources-before.json', 'sources-after.json', 'dependencies-before.json', 'dependencies-after.json'}
    require(set(r['raw']) == allowed and {p.name for p in (ROOT / 'evidence').iterdir()}
            == allowed | {'complete.json'}, 'exact raw roster')
    for name, pin in sorted(r['raw'].items()):
        retain('evidence/' + name, ROOT / 'evidence' / name, pin)
    require([p['label'] for p in r['phases']] == list(PHASES), 'eleven phases')
    for phase in r['phases']:
        require(phase['exit_code'] == 0 and phase['natural_exit'] is True
                and phase['reaped'] is True and phase['process_group_absent'] is True
                and phase['forced_cleanup'] is False and phase['timed_out'] is False
                and phase['exception'] is None and phase['storage_failure'] is None
                and not Path('/proc', str(phase['pid'])).exists(), 'terminal owned child')
        require(json.loads(bodies['evidence/' + phase['label'] + '.result.json']) == phase, 'phase join')
    for name in ('sources-before.json', 'sources-after.json'):
        require(json.loads(bodies['evidence/' + name]) == r['input_sources'], 'source join')
    require(json.loads(bodies['evidence/dependencies-before.json'])
            == json.loads(bodies['evidence/dependencies-after.json']), 'dependencies unchanged')
    lineage = inputs['source_lineage']
    require(set(lineage) == {'base_complete', 'base_sources', 'base_stdout', 'base_controller', 'overlay'},
            'closed lineage')
    for name in ('base_complete', 'base_sources', 'base_stdout', 'base_controller'):
        suffix = '.stdout' if name == 'base_stdout' else '.py' if name == 'base_controller' else '.json'
        retain('lineage/' + name + suffix, Path(lineage[name]['path']), lineage[name])
    base = json.loads(bodies['lineage/base_sources.json'])
    expected = {n: {k: p[k] for k in ('bytes', 'sha256')}
                for n, p in base.items() if n.startswith('fe2o3/')}
    require(len(lineage['overlay']) == 2 and len(expected) == 793, 'source ancestry')
    selected = {'fe2o3/' + n for n in ('Cargo.toml', 'Cargo.toml.original', 'Cargo.lock',
                                      'Cargo.lock.input', 'rust-toolchain.toml')}
    for name, row in lineage['overlay'].items():
        key = 'fe2o3/' + name
        require(expected.get(key) == row['before'], 'overlay preimage')
        expected[key] = row['after']
        selected.add(key)
    require({n: p for n, p in inputs['files'].items() if n.startswith('fe2o3/')} == expected
            and len(expected) == 794, 'exact source delta')
    for name in sorted(selected):
        retain(name, ROOT / name, r['final_sources'][name])
    for name in ('rustfmt.command.json', 'rustfmt.started.json', 'rustfmt.result.json',
                 'rustfmt.stdout', 'rustfmt.stderr', 'format-outcome.json', 'complete.json'):
        retain('format/' + name, ROOT / 'format' / name)
    retain('stage_cpu.py', E / 'stage_guarded_mlp_paired_native_v228_v1.py')
    require(files['stage_cpu.py']['sha256'] == 'c7e2aa2434b32fe451f7e3d4331834dddf05e91dda083a61c7fab39a2ab33589',
            'actual stager identity')
    formatted = json.loads(bodies['format/complete.json'])
    format_phase = json.loads(bodies['format/rustfmt.result.json'])
    format_command = json.loads(bodies['format/rustfmt.command.json'])
    format_started = json.loads(bodies['format/rustfmt.started.json'])
    archive_pin = dict(bytes=30447, sha256='e716a76388ab4757c1a0340fc3959ca9eb5c95d36fb638e81b19bbbd31dfc942')
    require(formatted['passed'] is True and formatted['failure'] is None
            and formatted['baseline_unchanged'] is True and formatted['postchecks_passed'] is True
            and formatted['input_manifest'] == {k: r['input_manifest'][k] for k in ('bytes', 'sha256')}
            and formatted['archive'] == archive_pin and formatted['phases'] == [format_phase]
            and json.loads(bodies['format/format-outcome.json'])['phases'] == [format_phase]
            and format_phase['exit_code'] == 0 and format_phase['natural_exit'] is True
            and format_phase['reaped'] is True and format_phase['process_group_absent'] is True
            and format_phase['argv'] == format_command['argv'] == format_started['argv']
            and format_phase['pid'] == format_started['pid']
            and format_phase['pgid'] == format_started['pgid'], 'format terminal/input/phase joins')
    for key in ('command', 'stdout', 'stderr'):
        require(format_phase[key] == files['format/' + Path(format_phase[key]['path']).name],
                'format raw pin join')
    require((r['tests']['kfd-tests']['passed'], r['tests']['kfd-tests']['failed'],
             r['tests']['kfd-tests']['ignored']) == (1029, 0, 5)
            and len(r['inventory']) == 1034 and len(r['artifacts']) == 5
            and r['native_test_name'] in r['ignored'], 'CPU census')
    for artifact in r['artifacts'].values():
        require(read(Path(artifact['pin']['path']))[1] == artifact['pin'], 'artifact unchanged')
    retain('export_cpu.py', Path(__file__).resolve())
    manifest = dict(schema='ferric-paired-native-mlp-cpu-retention-v1', files=files,
                    receipt=files['evidence/complete.json'], passed=True, selected_sources=sorted(selected),
                    full_source_tree_retained=False, exported_binary_bodies=False,
                    actual_artifact_metadata=r['artifacts'], gpu_execution=False, performance_claim=False)
    bodies['retention-manifest.json'] = (json.dumps(manifest, sort_keys=True, indent=2) + '\n').encode()
    with ARCHIVE.open('xb') as stream, tarfile.open(fileobj=stream, mode='w:gz') as archive:
        for name, body in sorted(bodies.items()):
            info = tarfile.TarInfo(name)
            info.size, info.mode, info.mtime = len(body), 0o600, 0
            archive.addfile(info, io.BytesIO(body))
    require(all(read(Path(row['path']))[1] == row for row in files.values()), 'post-export input drift')
    print(json.dumps(dict(archive=read(ARCHIVE)[1], members=len(bodies),
                         expanded_bytes=sum(map(len, bodies.values()))), sort_keys=True))


if __name__ == '__main__':
    main()
