"""Publish the bounded call reader and exact source join; no compile/GPU credit."""
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
ROOT = E / 'guarded-mlp-semantic-call-publication-v228-v1'
CPU = E / 'guarded-mlp-semantic-call-inspect-cpu-v228-v1'
BASE = E / 'guarded-mlp-semantic-inspect-cpu-v228-v1'
JOIN = E / 'guarded-mlp-call-source-join-v228-v1'
PAIR = E / 'guarded-mlp-early-stop-compile-v228-v12'
OLD_PAIR = E / 'guarded-mlp-early-stop-compile-v228-v8'
PROVIDER = E / 'guarded-mlp-inline-fndef-origin-cpu-v228-v1/fe2o3/crates/fe2o3-device/src/finite_join/wave_mlp_tiles_v2.rs'
READER = 'fe2o3/crates/fe2o3-mir-model/src/bin/fe2o3-semantic-capture-inspect.rs'


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


def main():
    require(os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.getuid() == os.geteuid() == 9661, 'MI350 owner')
    os.sched_setaffinity(0, {8, 9})
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    members, originals = {}, {}

    def add(name, path):
        raw = read(path)
        require(name not in members and len(members) < 255, 'unique bounded roster')
        require(sum(map(len, members.values())) + len(raw) <= 256 << 20, 'expanded archive bound')
        members[name] = raw
        originals[name] = dict(path=str(path), **pin(raw))
        return raw

    raw = read(CPU / 'evidence/complete.json')
    require(pin(raw)['sha256'] == '862acab9993408c8303ba047219b2fca5c2c8a4d9c940807b22569b8f3257a7b',
            'exact qualified reader terminal')
    cpu = json.loads(raw)
    require(cpu['passed'] is True and cpu['failure'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] is True and cpu['input_sources'] == cpu['final_sources']
            and len(cpu['phases']) == 11, 'complete unchanged CPU scope')
    require(cpu['tests']['reader']['passed'] == 14 and cpu['tests']['reader']['failed'] == 0
            and cpu['tests']['reader']['ignored'] == 0, 'exact reader test outcomes')
    for phase in cpu['phases']:
        require(phase['exit_code'] == 0 and phase['natural_exit'] is True
                and phase['reaped'] is True and phase['process_group_absent'] is True
                and phase['timed_out'] is False and phase['forced_cleanup'] is False
                and phase['exception'] is None and phase['storage_failure'] is None,
                'clean owned retirement required')
    for name, row in cpu['raw'].items():
        require('/' not in name and pin(read(CPU / 'evidence' / name)) == compact(row), 'raw CPU join')
        add('cpu/evidence/' + name, CPU / 'evidence' / name)
    add('cpu/evidence/complete.json', CPU / 'evidence/complete.json')
    for name in ('run_cpu.py', 'prepare_inputs.py', 'supervisor.py', 'input-manifest.json'):
        body = add('cpu/' + name, CPU / name)
        expected = cpu['input_manifest'] if name == 'input-manifest.json' else cpu['final_sources'][name]
        require(pin(body) == compact(expected), 'CPU auxiliary join')
    require(pin(add('base/complete.json', BASE / 'evidence/complete.json'))
            == compact(cpu['base_cpu_complete']), 'exact reader parent')
    require(set(cpu['source_delta']) == {READER}, 'single diagnostic reader source delta')
    old = add('source-before/' + READER, BASE / READER)
    new = add('source-after/' + READER, CPU / READER)
    require(pin(old) == cpu['source_delta'][READER]['before']
            and pin(new) == cpu['source_delta'][READER]['after'], 'source delta pre/postimages')
    relative = READER.removeprefix('fe2o3/')
    patch = ''.join(difflib.unified_diff(old.decode().splitlines(keepends=True),
                                      new.decode().splitlines(keepends=True),
                                      fromfile='a/' + relative, tofile='b/' + relative)).encode()
    put('source/' + relative, new)
    put('reader.patch', patch)
    add('reader.patch', ROOT / 'reader.patch')
    put('cpu/complete.json', raw)
    put('cpu/reader-tests.stdout', read(CPU / 'evidence/reader-tests.stdout'))
    for label, pair in (('captures', OLD_PAIR), ('call-captures', PAIR)):
        for arm in ('fixed', 'early'):
            for name in ('result.json', 'compile.stderr', 'semantic-mir-v1.bin', 'semantic-source-map-v1.json'):
                directory = 'evidence' if name in ('result.json', 'compile.stderr') else 'fe2o3-engineering-diagnostics-v1'
                key = label + '/' + arm + '/' + name
                body = add(key, pair / (arm + '-run') / directory / name)
                require(pin(body) == compact(cpu['final_sources'][key]), 'original qualified capture')
    for name in ('fixed', 'early', 'call-fixed', 'call-early'):
        body = read(CPU / ('evidence/inspect-' + name + '.stdout'))
        require(json.loads(body) == cpu['inspections'][name], 'exact inspection JSON join')
        put('inspections/' + name + '.json', body)
    joined_raw = add('source-join/result.json', JOIN / 'result.json')
    require(pin(joined_raw)['sha256'] == '3585842d8321ae9172aa42ba72e93a63d791f4409970324357973f8ae5af3e78',
            'exact source-join terminal')
    joined = json.loads(joined_raw)
    require(joined['passed'] is True and joined['inputs'] == joined['inputs_after']
            and joined['source_contents_join_verified'] is True
            and all(joined[key] is False for key in ('kir_binding_types_observed', 'gpu_execution',
                                                    'production_authority', 'compiler_hsaco_reproduced')),
            'source join scope')
    predetermined = [JOIN / 'join.py', CPU / 'evidence/complete.json', PROVIDER]
    for arm in ('fixed', 'early'):
        predetermined += [CPU / ('evidence/inspect-call-' + arm + '.stdout'),
                          PAIR / (arm + '-run/evidence/sources-before.json'),
                          PAIR / (arm + '-run/evidence/sources-after.json'),
                          PAIR / (arm + '-run/fe2o3-engineering-diagnostics-v1/semantic-source-map-v1.json'),
                          PAIR / arm / 'src/lib.rs']
    require(set(joined['inputs']) == set(map(str, predetermined)), 'closed source join inputs')
    for index, path in enumerate(predetermined):
        body = add('source-join/inputs/' + str(index) + '/' + path.name, path)
        require(pin(body) == compact(joined['inputs'][str(path)]), 'source-join original')
    put('source-join/result.json', joined_raw)
    put('source-join/join.py', read(JOIN / 'join.py'))
    add('export.py', ROOT / 'export.py')
    members['originals.json'] = (json.dumps(dict(schema='ferric-semantic-call-originals-v1',
                                                originals=originals), indent=2, sort_keys=True) + '\n').encode()
    require(len(members) <= 256 and sum(map(len, members.values())) <= 256 << 20,
            'complete archive bounds include ledger')
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode='w', format=tarfile.USTAR_FORMAT) as archive:
        for name, body in sorted(members.items()):
            info = tarfile.TarInfo(name)
            info.size, info.mode, info.mtime = len(body), 0o600, 0
            archive.addfile(info, io.BytesIO(body))
    compressed = gzip.compress(buffer.getvalue(), mtime=0)
    put('originals.tar.gz', compressed)
    with tarfile.open(fileobj=io.BytesIO(read(ROOT / 'originals.tar.gz')), mode='r:gz') as archive:
        entries = archive.getmembers()
        require(len(entries) == len(members) and [row.name for row in entries] == sorted(members),
                'exact closed archive roster')
        for entry in entries:
            require(entry.isfile() and entry.size == len(members[entry.name])
                    and archive.extractfile(entry).read() == members[entry.name], 'archive byte readback')
    manifest = dict(schema='ferric-semantic-call-publication-v1',
                    cpu=dict(receipt=pin(raw), phases=11, passed_tests=14, ignored_tests=0,
                             elapsed_seconds=cpu['elapsed_seconds'], artifacts=cpu['artifacts']),
                    source_join=pin(joined_raw), reader_patch=pin(patch), archive=pin(compressed),
                    members=len(members), originals=len(originals), expanded_bytes=sum(map(len, members.values())),
                    kir_binding_types_observed=False, gpu_execution=False, numerical_acceptance=False,
                    performance_claim=False, production_authority=False, full_model_acceptance=False)
    put('manifest.json', (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode())
    print(json.dumps(dict(manifest=pin(read(ROOT / 'manifest.json')), archive=pin(compressed),
                          members=len(members), originals=len(originals)), sort_keys=True))


if __name__ == '__main__':
    main()
