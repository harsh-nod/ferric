"""Export and source-join a qualified diagnostic census, without compiler/GPU execution."""
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
ROOT = E / 'guarded-mlp-semantic-census-publication-v228-v2'
CPU = E / 'guarded-mlp-semantic-call-census-cpu-v228-v2'
FAILED = E / 'guarded-mlp-semantic-call-census-cpu-v228-v1'
BASE = E / 'guarded-mlp-semantic-call-inspect-cpu-v228-v1'
READER = 'fe2o3/crates/fe2o3-mir-model/src/bin/fe2o3-semantic-capture-inspect.rs'
before = {}
members = {}
originals = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def read(path):
    first = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(first.st_mode)
            and first.st_size <= 16 << 20, 'bounded ordinary original')
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(first) == stamp(path.lstat()) and len(raw) == first.st_size, 'changed input')
    row = dict(path=str(path), **pin(raw))
    require(str(path) not in before or before[str(path)] == row, 'input drift')
    before[str(path)] = row
    return raw


def add(name, path, expected=None):
    raw = read(path)
    require(expected is None or pin(raw) == compact(expected), 'original pin: ' + name)
    require(name not in members and len(members) < 255
            and sum(map(len, members.values())) + len(raw) <= 256 << 20, 'archive bounds')
    members[name] = raw
    originals[name] = before[str(path)]
    return raw


def put(name, raw):
    p = ROOT / name
    p.parent.mkdir(parents=True, exist_ok=True)
    with p.open('xb') as stream:
        stream.write(raw)
    require(p.read_bytes() == raw, 'export readback')


def coordinate(raw, offset):
    prefix = raw[:offset].decode('utf-8')
    return [prefix.count('\n') + 1, len(prefix.rsplit('\n', 1)[-1]) + 1]


def main():
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and Path(__file__).resolve() == ROOT / 'export.py', 'host/owner/script')
    os.sched_setaffinity(0, {8, 9})
    require(os.getpriority(os.PRIO_PROCESS, 0) == 10, 'nice10')
    require(not (ROOT / 'manifest.json').exists(), 'fresh publication')
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    require(boot == '2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a', 'reconciled boot')
    cpu_raw = add('cpu/evidence/complete.json', CPU / 'evidence/complete.json')
    require(pin(cpu_raw)['sha256'] == '60898b32e9d93cdae3ec5359177055213898dc27c0e14e5bf68b5d727376a5a7',
            'qualified CPU receipt')
    cpu = json.loads(cpu_raw)
    require(cpu['passed'] and cpu['failure'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] and cpu['input_sources'] == cpu['final_sources']
            and len(cpu['phases']) == 15 and cpu['tests']['reader']['passed'] == 18
            and cpu['tests']['reader']['failed'] == cpu['tests']['reader']['ignored'] == 0,
            'exact passing qualification scope')
    retired = []
    for phase in cpu['phases']:
        require(phase['exit_code'] == 0 and phase['natural_exit'] and phase['reaped']
                and phase['process_group_absent'] and not phase['timed_out']
                and not phase['forced_cleanup'] and phase['exception'] is None
                and phase['storage_failure'] is None, 'clean leaf terminal')
        for key in ('pid', 'pgid'):
            require(not Path('/proc/' + str(phase[key])).exists(), 'saved child still present')
            retired.append(phase[key])
    require(not Path('/proc/' + str(cpu['execution_scope']['pid'])).exists(), 'controller retired')
    for name, row in cpu['raw'].items():
        require('/' not in name, 'raw basename')
        add('cpu/evidence/' + name, CPU / 'evidence' / name, row)
    for name in ('run_cpu.py', 'prepare_inputs.py', 'supervisor.py', 'input-manifest.json'):
        expected = cpu['input_manifest'] if name == 'input-manifest.json' else cpu['final_sources'][name]
        add('cpu/' + name, CPU / name, expected)
    for name, key in (('complete.json', 'base_cpu_complete'), ('sources-after.json', 'base_cpu_sources'),
                      ('dependencies-after.json', 'base_cpu_dependencies')):
        add('base/' + name, BASE / 'evidence' / name, cpu[key])
    require(set(cpu['source_delta']) == {READER}, 'only diagnostic source changed')
    old = add('source-before/' + READER, BASE / READER, cpu['source_delta'][READER]['before'])
    new = add('source-after/' + READER, CPU / READER, cpu['source_delta'][READER]['after'])
    relative = READER.removeprefix('fe2o3/')
    patch = ''.join(difflib.unified_diff(old.decode().splitlines(keepends=True),
                                      new.decode().splitlines(keepends=True),
                                      fromfile='a/' + relative, tofile='b/' + relative)).encode()
    put('source/' + relative, new)
    put('reader.patch', patch)
    reader_pin = cpu['artifacts']['reader']['pin']
    require(pin(read(Path(reader_pin['path']))) == compact(reader_pin), 'reader ELF still exact')
    for key, row in cpu['final_sources'].items():
        if key.startswith(('captures/', 'call-captures/', 'census-captures/')):
            add(key, Path(row['path']), row)
    failed_raw = add('failed-v1/evidence/failed.json', FAILED / 'evidence/failed.json')
    require(pin(failed_raw)['sha256'] == 'c789b43cbc42e30cef017fb810f2c5856c610d91492e429edacbb8636d4dadda',
            'preserved original diagnostic failure')
    failed = json.loads(failed_raw)
    require(not failed['passed'] and failed['source_unchanged'] and not failed['postcheck_errors']
            and failed['tests']['reader']['passed'] == 18, 'failed original scope')
    for name in ('run_cpu.py', 'prepare_inputs.py', 'input-manifest.json', READER):
        expected = failed['input_manifest'] if name == 'input-manifest.json' else failed['final_sources'][name]
        add('failed-v1/' + name, FAILED / name, expected)
    for phase in failed['phases']:
        require(phase['natural_exit'] and phase['reaped'] and phase['process_group_absent'],
                'failed leaf clean terminal')
        for key in ('pid', 'pgid'):
            require(not Path('/proc/' + str(phase[key])).exists(), 'failed child still present')
        if phase['label'] == 'census-v16-fixed':
            for key in ('command', 'stdout', 'stderr'):
                row = phase[key]
                add('failed-v1/evidence/' + Path(row['path']).name, Path(row['path']), row)
    joined = {}
    for version, provider in (('v16', 'guarded-mlp-wave-effect-diagnostic-cpu-v228-v2'),
                               ('v17', 'guarded-mlp-coverage-inline-cpu-v228-v1')):
        pair = E / ('guarded-mlp-early-stop-compile-v228-' + version)
        device = E / provider / 'fe2o3/crates/fe2o3-device/src'
        half = device / 'half.rs'
        coverage = device / 'finite_join/wave_mlp_tiles_v2.rs'
        half_raw = add('source-join/' + version + '/half.rs', half)
        coverage_raw = add('source-join/' + version + '/wave_mlp_tiles_v2.rs', coverage)
        for arm in ('fixed', 'early'):
            key = 'census-' + version + '-' + arm
            raw = read(CPU / ('evidence/' + key + '.stdout'))
            census = json.loads(raw)
            require(census == cpu['inspections'][key], 'original census stdout join')
            put('census/' + version + '-' + arm + '.json', raw)
            fixture = pair / arm / 'src/lib.rs'
            fixture_raw = add('source-join/' + version + '/' + arm + '/lib.rs', fixture)
            allowed = {str(half): (half, half_raw), str(coverage): (coverage, coverage_raw),
                       'src/lib.rs': (fixture, fixture_raw)}
            for name in ('sources-before.json', 'sources-after.json'):
                snapshot = json.loads(add('source-join/' + version + '/' + arm + '/' + name,
                                          pair / (arm + '-run/evidence/') / name))
                for path, body in allowed.values():
                    require(snapshot[str(path)] == dict(path=str(path), **pin(body)),
                            'actual compile before/after source pin')
            map_key = 'census-captures/' + version + '/' + arm + '/semantic-source-map-v1.json'
            mapping = json.loads(read(Path(cpu['final_sources'][map_key]['path'])))
            files = {row['identity']: row for row in mapping['files']}
            origins = []
            selected = [(f'function-{row["function_index"]}', row)
                        for row in census['functions']]
            selected += [(f'call-{row["function_index"]}-{row["block_index"]}', row)
                         for row in census['calls'] if row['callable_kind'] == 'Defined']
            for label, row in selected:
                for name in ('call_site', 'expansion'):
                    origin = row[name]
                    if origin is None:
                        continue
                    if origin['map_resolution'] == 'FileAbsent':
                        require(origin['file_identity'] not in files
                                and origin['display_path'] is None and origin['file_bytes'] is None
                                and not origin['source_file_contents_verified'], 'closed unresolved origin')
                        origins.append(dict(record=label + '.' + name, resolved=False, origin=origin))
                        continue
                    require(origin['map_resolution'] == 'RecordedFile'
                            and origin['display_path'] in allowed
                            and not origin['source_file_contents_verified'], 'predeclared source origin')
                    path, body = allowed[origin['display_path']]
                    source_file = files[origin['file_identity']]
                    require(source_file['display_path'] == origin['display_path']
                            and source_file['byte_len'] == origin['file_bytes'] == len(body),
                            'map identity/path/extent join')
                    start, end = origin['byte_range']
                    require(0 <= start < end <= len(body) and end - start <= 8192
                            and coordinate(body, start) == origin['start']
                            and coordinate(body, end) == origin['end'], 'source span/coordinate join')
                    origins.append(dict(record=label + '.' + name, resolved=True,
                                        origin=origin, source=before[str(path)],
                                        span_utf8=body[start:end].decode('utf-8')))
            helper_index = 2 if version == 'v16' else 1
            helpers = [row for row in census['functions'] if row['role'] == 'InternalHelper']
            half_helper = census['functions'][helper_index]
            require(half_helper['call_site']['display_path'] == str(half)
                    and half_helper['call_site']['byte_range'] == [6723, 6803]
                    and half_raw[6723:6803].decode()
                    == 'pub const fn is_finite(self) -> bool {\n        (self.0 & 0x7f80) != 0x7f80\n    }',
                    'exact BF16 finiteness helper body')
            half_calls = [row for row in census['calls'] if row['callee_function_index'] == helper_index]
            require(len(half_calls) == 11
                    and len(helpers) == (2 if version == 'v16' else 1)
                    and census['defined_call_count'] == (12 if version == 'v16' else 11),
                    'actual before/after defined helper census')
            joined[version + '-' + arm] = dict(
                function_count=census['function_count'], block_count=census['block_count'],
                call_count=census['call_count'], defined_call_count=census['defined_call_count'],
                finite_predicate_helper=half_helper,
                finite_predicate_call_blocks=[row['block_index'] for row in half_calls],
                origins=origins)
    after = {path: dict(path=path, **pin(Path(path).read_bytes())) for path in before}
    require(after == before, 'all source/receipt/artifact inputs unchanged')
    result = dict(schema='ferric-semantic-census-source-join-v1', passed=True,
                  inputs=before, inputs_after=after, arms=joined,
                  boot_id=boot, inspected_leaf_pids_absent=sorted(set(retired)),
                  source_contents_join_verified=True, unresolved_origins_not_resolved=True,
                  kir_binding_types_observed=False, kir_to_semantic_call_join_verified=False,
                  gpu_execution=False, production_authority=False, compiler_hsaco_reproduced=False)
    joined_raw = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()
    put('source-join/result.json', joined_raw)
    add('source-join/result.json', ROOT / 'source-join/result.json')
    add('export.py', ROOT / 'export.py')
    members['originals.json'] = (json.dumps(dict(schema='ferric-census-originals-v1',
                                                originals=originals), indent=2, sort_keys=True) + '\n').encode()
    require(len(members) <= 256 and sum(map(len, members.values())) <= 256 << 20, 'archive final bound')
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode='w', format=tarfile.USTAR_FORMAT) as archive:
        for name, body in sorted(members.items()):
            info = tarfile.TarInfo(name)
            info.size, info.mode, info.mtime = len(body), 0o600, 0
            archive.addfile(info, io.BytesIO(body))
    compressed = gzip.compress(buffer.getvalue(), mtime=0)
    put('originals.tar.gz', compressed)
    with tarfile.open(fileobj=io.BytesIO((ROOT / 'originals.tar.gz').read_bytes()), mode='r:gz') as archive:
        entries = archive.getmembers()
        require(len(entries) == len(members) and [row.name for row in entries] == sorted(members),
                'archive closed roster')
        for row in entries:
            require(row.isfile() and row.size == len(members[row.name])
                    and archive.extractfile(row).read() == members[row.name], 'archive exact readback')
    put('cpu/complete.json', cpu_raw)
    put('cpu/reader-tests.stdout', read(CPU / 'evidence/reader-tests.stdout'))
    manifest = dict(schema='ferric-semantic-census-publication-v1',
                    cpu=dict(receipt=pin(cpu_raw), phases=15, passed_tests=18, ignored_tests=0,
                             elapsed_seconds=cpu['elapsed_seconds'], artifacts=cpu['artifacts']),
                    source_join=pin(joined_raw), failed_v1=pin(failed_raw), reader_patch=pin(patch),
                    archive=pin(compressed), members=len(members), originals=len(originals),
                    expanded_bytes=sum(map(len, members.values())),
                    kir_binding_types_observed=False, gpu_execution=False, numerical_acceptance=False,
                    performance_claim=False, production_authority=False, full_model_acceptance=False)
    put('manifest.json', (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode())
    print(json.dumps(manifest, sort_keys=True))


if __name__ == '__main__':
    main()
