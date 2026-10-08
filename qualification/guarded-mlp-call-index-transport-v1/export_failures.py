"""Retain exact failed qualifications and the unchanged-parent control."""
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-call-index-transport-publication-v228-v1'
CPU = E / 'guarded-mlp-call-index-transport-cpu-v228-v4'
CPU_SHA = 'fd40879fc85e939f527407ff77f6b7d3fe71ea6c784253f5505e0c4d4a6fd484'
PARENT_SHA = '2c837e1a1b344c7bb125f8fef6a0ab2759d445062fe00296b3e3fa6209124b6a'
FAILURE_TEST = ('production_semantic_kir_v1::wave_task_entry_parameter_tests::access_roots::'
                'retained_fields::retained_nested_enum_referent_scalar_move_invalidates_saved_references')
REPAIR = ('fe2o3/crates/fe2o3-lower-mir-kernel/src/production_semantic_kir_v1/'
          'retained_nested_enum_transport_v1_tests.rs')
ATTEMPTS = (
    ('transport-v2', 'guarded-mlp-call-index-transport-cpu-v228-v2',
     '5375d5ef018472bffb5bb92c62f862a3055f06947045477d227e7804972b2998', 17, 0),
    ('transport-v3', 'guarded-mlp-call-index-transport-cpu-v228-v3',
     'd0af282850d0cfd73290bf3fa9e456189b564aee71add392f430e2714cf86621', 18, 765),
    ('unchanged-parent', 'guarded-mlp-parent-lowerer-control-cpu-v228-v1',
     PARENT_SHA, 18, 764),
)


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def read(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= 16 << 20, 'bounded canonical original')
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(path.lstat()) and len(raw) == before.st_size, 'original changed')
    return raw


def put(name, raw):
    path = ROOT / name
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('xb') as stream:
        stream.write(raw)
    require(read(path) == raw, 'export differs')


def clean(phase, code):
    require(phase['exit_code'] == code and phase['natural_exit'] is True
            and phase['reaped'] is True and phase['process_group_absent'] is True
            and phase['timed_out'] is False and phase['forced_cleanup'] is False
            and phase['exception'] is None and phase['storage_failure'] is None
            and phase['observed_signals'] == [] and phase['adopted_reaped'] == [],
            'clean owned retirement required')


def main():
    require(os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.getuid() == os.geteuid() == 9661, 'MI350 owner')
    os.sched_setaffinity(0, {8, 9})
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    cpu_raw = read(CPU / 'evidence/complete.json')
    require(pin(cpu_raw)['sha256'] == CPU_SHA, 'exact qualified successor')
    cpu = json.loads(cpu_raw)
    require(cpu['passed'] is True and cpu['source_unchanged'] is True
            and cpu['postcheck_errors'] == [] and len(cpu['source_delta']) == 4,
            'qualified four-file successor')
    base_raw = read(Path(cpu['base_cpu_complete']['path']))
    require(pin(base_raw) == compact(cpu['base_cpu_complete']), 'exact source parent')
    base = json.loads(base_raw)
    qualified = {name: compact(row) for name, row in cpu['final_sources'].items()
                 if name.startswith('fe2o3/')}
    baseline = {name: compact(row) for name, row in base['final_sources'].items()
                if name.startswith('fe2o3/')}
    results = {}
    for label, directory, digest, count, unit_passed in ATTEMPTS:
        source = E / directory
        raw = read(source / 'evidence/failed.json')
        require(pin(raw)['sha256'] == digest, 'exact failed terminal')
        failed = json.loads(raw)
        require(failed['passed'] is False and failed['source_unchanged'] is True
                and failed['postcheck_errors'] == [] and failed['input_sources'] == failed['final_sources']
                and len(failed['phases']) == count and 'backend' not in failed['artifacts']
                and 'lowerer-unit' not in failed['tests'], 'failed, not qualified')
        require([p['label'] for p in failed['phases']] ==
                [p['label'] for p in cpu['phases'][:count]], 'same phase sequence')
        expected_reason = ("RuntimeError('complete lowerer library inventory with exact nonignored emission/refusal controls')"
                           if count == 17 else
                           "RuntimeError('lowerer-unit-tests did not finish naturally/reaped/successfully')")
        require(failed['failure'] == expected_reason, 'exact failed gate')
        for index, phase in enumerate(failed['phases']):
            clean(phase, 101 if count == 18 and index == 17 else 0)
        project = {name: compact(row) for name, row in failed['final_sources'].items()
                   if name.startswith('fe2o3/')}
        require(len(project) == 5808 and set(project) == set(qualified), 'closed source roster')
        if label == 'unchanged-parent':
            require(project == baseline and failed['source_delta'] == {}, 'unchanged source control')
            require(failed['tests']['lowerer'] == base['tests']['lowerer'], 'parent integration outcomes')
        else:
            delta = {name: dict(before=project[name], after=qualified[name])
                     for name in project if project[name] != qualified[name]}
            require(delta == {REPAIR: cpu['source_delta'][REPAIR]}, 'only exact test-coordinate repair')
            require(failed['tests']['lowerer'] == cpu['tests']['lowerer'], 'transport integration outcomes')
        for key in ('compiler', 'device'):
            require(failed['tests'][key] == cpu['tests'][key], 'same full named test outcomes')
        unit = None
        if count == 18:
            output = read(source / 'evidence/lowerer-unit-tests.stdout').decode()
            lines = re.findall(r'^test ([A-Za-z0-9_:]+)(?: - should panic)? \.\.\. (ok|FAILED|ignored)$',
                               output, re.M)
            expected = failed['lowerer_unit_inventory']
            require(len(lines) == len(expected) == unit_passed + 1
                    and sorted(name for name, _ in lines) == sorted(expected)
                    and failed['lowerer_unit_ignored'] == []
                    and all(status == ('FAILED' if name == FAILURE_TEST else 'ok')
                            for name, status in lines), 'exact complete failed unit roster')
            summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
                                   r'(\d+) ignored; (\d+) measured; (\d+) filtered out;', output, re.M)
            require(summaries == [('FAILED', str(unit_passed), '1', '0', '0', '0')]
                    and output.count('test result:') == 1
                    and 'left: (8, Some(7), 5)' in output and 'right: (7, Some(4), 5)' in output,
                    'exact inherited failure observation')
            unit = dict(passed=unit_passed, failed=1, ignored=0, failing_test=FAILURE_TEST)
        members, originals = {}, {}
        def add(name, path, expected=None):
            body = read(path)
            require(name not in members and len(members) < 127
                    and sum(map(len, members.values())) + len(body) <= 256 << 20,
                    'bounded unique failure originals')
            if expected is not None:
                require(pin(body) == compact(expected), 'original receipt join')
            members[name] = body
            originals[name] = dict(path=str(path), **pin(body))
        add('evidence/failed.json', source / 'evidence/failed.json', pin(raw))
        require(len(failed['raw']) == (89 if count == 17 else 94), 'exact failed raw roster')
        for name, row in failed['raw'].items():
            require('/' not in name, 'ordinary raw name')
            add('evidence/' + name, source / 'evidence' / name, row)
        for name in ('run_cpu.py', 'prepare_inputs.py', 'supervisor.py',
                     'qualification_helpers.py', 'input-manifest.json'):
            expected = failed['input_manifest'] if name == 'input-manifest.json' else failed['final_sources'][name]
            add(name, source / name, expected)
        for name in sorted(cpu['source_delta']):
            add(name, source / name, failed['final_sources'][name])
        add('export_failures.py', ROOT / 'export_failures.py')
        members['originals.json'] = (json.dumps(dict(originals=originals), indent=2, sort_keys=True) + '\n').encode()
        require(len(members) <= 128 and sum(map(len, members.values())) <= 256 << 20, 'final archive bounds')
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode='w', format=tarfile.USTAR_FORMAT) as archive:
            for name, body in sorted(members.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(body), 0o600, 0
                archive.addfile(info, io.BytesIO(body))
        compressed = gzip.compress(buffer.getvalue(), mtime=0)
        archive_name = 'failures/' + label + '.tar.gz'
        put(archive_name, compressed)
        with tarfile.open(fileobj=io.BytesIO(read(ROOT / archive_name)), mode='r:gz') as archive:
            entries = archive.getmembers()
            require([entry.name for entry in entries] == sorted(members), 'closed archive readback')
            for entry in entries:
                require(entry.isfile() and entry.size == len(members[entry.name])
                        and archive.extractfile(entry).read() == members[entry.name], 'exact archive bytes')
        put('failures/' + label + '/failed.json', raw)
        if count == 18:
            put('failures/' + label + '/lowerer-unit-tests.stdout',
                read(source / 'evidence/lowerer-unit-tests.stdout'))
        results[label] = dict(receipt=pin(raw), phases=count, reason=failed['failure'],
                              unit_executed=count == 18, unit_qualified=False, unit_outcomes=unit,
                              backend_qualified=False, archive=dict(name=archive_name, **pin(compressed)),
                              members=len(members), originals=len(originals),
                              expanded_bytes=sum(map(len, members.values())))
    result = dict(schema='ferric-index-transport-failed-attempts-v1', cpu=pin(cpu_raw),
                  attempts=results, gpu_execution=False, production_authority=False)
    put('failures/manifest.json', (json.dumps(result, indent=2, sort_keys=True) + '\n').encode())
    print(json.dumps(dict(manifest=pin(read(ROOT / 'failures/manifest.json')), attempts=results),
                     sort_keys=True))


if __name__ == '__main__':
    main()
