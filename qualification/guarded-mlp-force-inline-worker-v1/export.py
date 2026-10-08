"""Retain original qualification, source changes, and paired compile evidence."""
import difflib
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-force-inline-worker-publication-v228-v1'
BASE = E / 'guarded-mlp-force-inline-accessors-cpu-v228-v1'
PAIR = E / 'guarded-mlp-early-stop-compile-v228-v11'
CPU = E / 'guarded-mlp-force-inline-worker-cpu-v228-v1'
JOIN9 = E / 'guarded-mlp-helper-source-join-v228-v3'
JOIN10 = E / 'guarded-mlp-helper-source-join-v228-v4'
CPUS = {
    'correction': ('guarded-mlp-force-inline-worker-cpu-v228-v1',
                   'abcc3218698d408918af940d99a2aa9dad9ec4fb209fb64efd78978af0f8b6e2',
                   11, {'compiler': (1331, 24), 'device': (359, 0)}),
}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def read(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= 16 << 20, 'bounded ordinary original')
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(before) == stamp(path.lstat()) and len(raw) == before.st_size, 'original changed')
    return raw


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


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
            and phase['exception'] is None and phase['storage_failure'] is None,
            'actual clean process retirement')


def main():
    require(os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.getuid() == os.geteuid() == 9661, 'MI350 owner')
    os.sched_setaffinity(0, {8, 9})
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    members, originals, cpus, arms = {}, {}, {}, {}

    def add(name, path):
        raw = read(path)
        require(name not in members and len(members) < 256, 'unique bounded archive roster')
        require(sum(map(len, members.values())) + len(raw) <= 256 << 20, 'expanded archive bound')
        members[name] = raw
        originals[name] = dict(path=str(path), **pin(raw))
        return raw

    for role, (directory, digest, phase_count, counts) in CPUS.items():
        root = E / directory
        raw = read(root / 'evidence/complete.json')
        require(pin(raw)['sha256'] == digest, 'qualified CPU receipt')
        cpu = json.loads(raw)
        require(cpu['passed'] is True and cpu['failure'] is None and cpu['postcheck_errors'] == []
                and cpu['source_unchanged'] is True and len(cpu['phases']) == phase_count
                and set(cpu['tests']) == set(counts), 'qualified CPU scope')
        for key, (passed, ignored) in counts.items():
            require(cpu['tests'][key]['passed'] == passed and cpu['tests'][key]['ignored'] == ignored
                    and cpu['tests'][key]['failed'] == 0, 'actual exact CPU outcomes')
        for phase in cpu['phases']:
            clean(phase, 0)
        require(cpu['input_sources'] == cpu['final_sources'], 'source mutation')
        for name, row in cpu['raw'].items():
            require(pin(read(root / 'evidence' / name)) == compact(row), 'raw CPU receipt join')
        for path in sorted((root / 'evidence').iterdir()):
            if path.is_file():
                add('cpu/' + role + '/evidence/' + path.name, path)
        auxiliary = ['run_cpu.py', 'prepare_inputs.py', 'supervisor.py', 'input-manifest.json']
        if role == 'correction':
            auxiliary += ['qualification_helpers.py', 'review_source.py', 'source-review.json']
        for name in auxiliary:
            add('cpu/' + role + '/' + name, root / name)
        diff = []
        for name, delta in sorted(cpu['source_delta'].items()):
            new = read(root / name)
            require(pin(new) == delta['after'], 'source postimage pin')
            old = b''
            if delta['before'] is not None:
                old = add(role + '/source-before/' + name, BASE / name)
                require(pin(old) == delta['before'], 'source preimage pin')
            else:
                require(not (BASE / name).exists(), 'new source already exists in parent')
            add(role + '/source-after/' + name, root / name)
            relative = name.removeprefix('fe2o3/')
            put('source/' + role + '/' + relative, new)
            diff.extend(difflib.unified_diff(old.decode().splitlines(keepends=True),
                                           new.decode().splitlines(keepends=True),
                                           fromfile='a/' + relative if old else '/dev/null',
                                           tofile='b/' + relative))
        put(role + '.patch', ''.join(diff).encode())
        put('cpu/' + role + '/complete.json', raw)
        for phase in cpu['phases']:
            if phase['label'] in ('reader-tests', 'compiler-tests', 'device-tests', 'inspect-fixed', 'inspect-early'):
                name = phase['label'] + '.stdout'
                put('cpu/' + role + '/' + name, read(root / 'evidence' / name))
        cpus[role] = dict(receipt=pin(raw), phases=phase_count, tests=counts,
                          elapsed_seconds=cpu['elapsed_seconds'], artifacts=cpu['artifacts'])
        add(role + '/source-base-complete.json', BASE / 'evidence/complete.json')

    review_raw = read(CPU / 'source-review.json')
    require(pin(review_raw)['sha256'] == '22e8ebe525bf15305415ed1f98e31fd161bdfec4271ceed0b196eaf89487aa1a',
            'exact independently reviewed two-attribute source delta')
    put('source-review.json', review_raw)
    for arm, digest in (
        ('fixed', '70400214a942d2bc534e01a4b5ef757dbb42a320f4613fe7ec7c6bd819117427'),
        ('early', 'ef572709628d9ef7fff91fe66eead7230d0cdd6b99c19256294211dbf5633d44'),
    ):
        raw = add('previous-helper/' + arm + '.json', JOIN9 / (arm + '-run/evidence/result.json'))
        require(pin(raw)['sha256'] == digest, 'original V10 source diagnosis')
    add('pair/probe.py', PAIR / 'probe.py')
    add('next-helper/inspect.py', JOIN10 / 'inspect.py')
    for arm in ('fixed', 'early'):
        for path in sorted((PAIR / arm).rglob('*')):
            if path.is_file():
                add('pair/' + arm + '/' + str(path.relative_to(PAIR / arm)), path)
        run = PAIR / (arm + '-run')
        evidence = run / 'evidence'
        result = json.loads(read(evidence / 'result.json'))
        require(result['arm'] == arm and result['postcheck_errors'] == [] and len(result['phases']) == 1,
                'terminal paired compiler scope')
        code = 0 if result['compile_accepted'] else 1
        clean(result['phases'][0], code)
        require(read(evidence / 'sources-before.json') == read(evidence / 'sources-after.json'),
                'paired compile source mutation')
        inputs = json.loads(read(evidence / 'inputs.json'))
        require(inputs['backend_cpu_receipt']['sha256'] == CPUS['correction'][1], 'qualified compiler join')
        for path in sorted(evidence.iterdir()):
            if path.is_file():
                add('pair/' + arm + '-run/evidence/' + path.name, path)
        for name, row in result['diagnostic_files'].items():
            raw = add('pair/' + arm + '-run/diagnostic/' + name, Path(row['path']))
            require(pin(raw) == compact(row), 'exact compiler diagnostic')
        products = {}
        for path in sorted((run / 'fe2o3-engineering-v1').rglob('*')):
            if path.is_file():
                relative = str(path.relative_to(run))
                products[relative] = pin(add('pair/' + arm + '-run/' + relative, path))
        require(bool(products) == bool(result['compile_accepted']), 'compiler product/result agreement')
        for name in ('result.json', 'compile.stderr'):
            put('pair/' + arm + '/' + name, read(evidence / name))
        arms[arm] = dict(result=pin(read(evidence / 'result.json')), accepted=result['compile_accepted'],
                         elapsed_seconds=result['elapsed_seconds'], products=products,
                         diagnostics=result['diagnostic_files'])
        next_evidence = JOIN10 / (arm + '-run/evidence')
        joined = json.loads(read(next_evidence / 'result.json'))
        require(joined['passed'] is True and joined['error'] is None
                and joined['source_contents_join_verified'] is True
                and joined['compile_result']['sha256'] == arms[arm]['result']['sha256'],
                'actual current helper-source join')
        clean(joined['phases'][0], 0)
        for path in sorted(next_evidence.iterdir()):
            if path.is_file():
                add('next-helper/' + arm + '/' + path.name, path)
        put('next-helper/' + arm + '.json', read(next_evidence / 'result.json'))
        arms[arm]['next_helper'] = joined['helper']['identity']
        arms[arm]['next_helper_join'] = pin(read(next_evidence / 'result.json'))

    add('export.py', ROOT / 'export.py')
    raw_manifest = (json.dumps(dict(schema='ferric-force-inline-worker-originals-v1', originals=originals),
                               indent=2, sort_keys=True) + '\n').encode()
    members['originals.json'] = raw_manifest
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode='w', format=tarfile.USTAR_FORMAT) as archive:
        for name, raw in sorted(members.items()):
            info = tarfile.TarInfo(name)
            info.size, info.mode, info.mtime = len(raw), 0o600, 0
            archive.addfile(info, io.BytesIO(raw))
    compressed = gzip.compress(buffer.getvalue(), mtime=0)
    put('originals.tar.gz', compressed)
    manifest = dict(schema='ferric-force-inline-worker-publication-v1', cpu=cpus, pair=arms,
                    source_review=pin(review_raw), archive=pin(compressed), members=len(members),
                    originals=len(originals), expanded_bytes=sum(map(len, members.values())),
                    gpu_execution=False, numerical_acceptance=False, performance_claim=False,
                    production_authority=False, full_model_acceptance=False)
    put('manifest.json', (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode())
    print(json.dumps(dict(manifest=pin(read(ROOT / 'manifest.json')), archive=pin(compressed),
                          members=len(members), originals=len(originals)), sort_keys=True))


if __name__ == '__main__':
    main()
