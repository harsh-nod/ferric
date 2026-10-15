"""Export retained replay logs/capture/source evidence, not execution authority."""
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile

ROOT = Path(__file__).resolve().parent
E = ROOT.parent
RUN = E / 'guarded-mlp-fixed-formal-replay-cpu-v228-v1'
CPU = E / 'guarded-mlp-finite-predicate-inline-cpu-v228-v3'
PAIR = E / 'guarded-mlp-early-stop-compile-v228-v18'
RECEIPT_SHA = '12e961aaf9a5bf4d2a1a4750517568ec7284a974b82a852674c678559ca2f507'
members, originals = {}, {}


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
            and first.st_size <= 16 << 20, 'bounded ordinary evidence')
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(first) == stamp(path.lstat()) and len(raw) == first.st_size, 'evidence changed')
    return raw


def add(name, path, expected=None):
    raw = read(path)
    require(expected is None or pin(raw) == compact(expected), 'exact original join: ' + name)
    require(name not in members and len(members) < 127
            and sum(map(len, members.values())) + len(raw) <= 128 << 20, 'archive bounds')
    members[name] = raw
    originals[name] = dict(path=str(path), **pin(raw))
    return raw


def put(name, raw):
    path = ROOT / name
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('xb') as stream:
        stream.write(raw)
    require(path.read_bytes() == raw, 'export readback')


def main():
    require(ROOT.name == 'guarded-mlp-fixed-formal-replay-publication-v228-v1'
            and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and os.getuid() == os.geteuid() == 9661, 'host/owner')
    require(os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'CPU8/9 nice10')
    require(not (ROOT / 'manifest.json').exists(), 'fresh publication')
    raw = add('replay/evidence/complete.json', RUN / 'evidence/complete.json')
    require(pin(raw)['sha256'] == RECEIPT_SHA, 'actual replay receipt')
    replay = json.loads(raw)
    require(replay['passed'] and replay['failure'] is None and replay['postcheck_errors'] == []
            and replay['source_unchanged'] and replay['inputs_before'] == replay['inputs_after']
            and len(replay['inputs_after']) == 5846 and len(replay['phases']) == 3
            and replay['tests']['passed'] == 1 and replay['tests']['failed'] == 0
            and replay['tests']['ignored'] == 0 and replay['tests']['filtered_out'] == 1364
            and len(replay['default_ignored']) == 24
            and replay['production_source_changed'] is False
            and replay['worker_compilation_replayed'] is False
            and replay['full_compiler_suite_rerun'] is False
            and replay['gpu_execution'] is False and replay['production_authority'] is False,
            'exact replay qualification scope')
    retired = []
    for phase in replay['phases']:
        require(phase['exit_code'] == 0 and phase['natural_exit'] and phase['reaped']
                and phase['process_group_absent'] and not phase['timed_out']
                and not phase['forced_cleanup'] and phase['exception'] is None
                and phase['storage_failure'] is None, 'clean replay phase')
        for key in ('pid', 'pgid'):
            require(not Path('/proc/' + str(phase[key])).exists(), 'saved replay child still present')
            retired.append(phase[key])
        for key in ('command', 'stdout', 'stderr'):
            expected = phase[key]
            require(replay['raw'][Path(expected['path']).name] == expected, 'raw phase receipt join')
    require(not Path('/proc/' + str(replay['execution_scope']['pid'])).exists(), 'controller retired')
    for name, expected in replay['raw'].items():
        require('/' not in name, 'raw basename')
        add('replay/evidence/' + name, RUN / 'evidence' / name, expected)
    for name in ('run.py', 'prepare_inputs.py', 'input-manifest.json'):
        expected = replay['input_manifest'] if name == 'input-manifest.json' else replay['inputs_after'][str(RUN / name)]
        add('replay/' + name, RUN / name, expected)
        if name != 'input-manifest.json':
            put(name, read(RUN / name))
    for name in ('complete.json', 'sources-after.json', 'compiler-tests.command.json'):
        path = CPU / 'evidence' / name
        add('qualified-parent/evidence/' + name, path, replay['inputs_after'][str(path)])
    for name in ('input-manifest.json', 'supervisor.py', 'run_cpu.py', 'prepare_inputs.py',
                 'qualification_helpers.py'):
        path = CPU / name
        add('qualified-parent/' + name, path, replay['inputs_after'][str(path)])
    for name in ('result.json', 'compile.stderr', 'sources-before.json', 'sources-after.json'):
        path = PAIR / 'fixed-run/evidence' / name
        add('original-compile/evidence/' + name, path, replay['inputs_after'][str(path)])
    for name in ('semantic-mir-v1.bin', 'semantic-source-map-v1.json'):
        path = PAIR / 'fixed-run/fe2o3-engineering-diagnostics-v1' / name
        add('original-compile/diagnostics/' + name, path, replay['inputs_after'][str(path)])
        put('capture/' + name, read(path))
    for path in sorted((PAIR / 'fixed').rglob('*')):
        if path.is_file():
            add('original-compile/fixture/' + str(path.relative_to(PAIR / 'fixed')),
                path, replay['inputs_after'][str(path)])
    source_names = (
        'crates/rustc-codegen-fe2o3/src/production_pipeline/wave_mlp_tile_capture_v2_tests.rs',
        'crates/rustc-codegen-fe2o3/src/production_semantic_lineage_v3/replay_actual_mlp_tile_lineage_v2.rs',
        'crates/rustc-codegen-fe2o3/src/production_pipeline/conditional_formal_v1.rs',
        'crates/rustc-codegen-fe2o3/src/production_pipeline.rs',
    )
    for name in source_names:
        path = CPU / 'fe2o3' / name
        body = add('source/' + name, path, replay['inputs_after'][str(path)])
        put('source/' + name, body)
    put('replay/complete.json', raw)
    for name in ('fixed-formal-replay.stdout', 'fixed-formal-replay.stderr'):
        put('replay/' + name, read(RUN / 'evidence' / name))
    add('export.py', ROOT / 'export.py')
    scope = 'retained replay logs, captures, controllers, source ledgers and relevant source; excludes compiled ELF/toolchain/full source tree'
    members['originals.json'] = (json.dumps(dict(schema='ferric-fixed-formal-replay-originals-v1',
                                                archive_scope=scope, originals=originals),
                                          indent=2, sort_keys=True) + '\n').encode()
    require(len(members) <= 128 and sum(map(len, members.values())) <= 128 << 20, 'closed archive bound')
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode='w', format=tarfile.USTAR_FORMAT) as archive:
        for name, body in sorted(members.items()):
            entry = tarfile.TarInfo(name)
            entry.mode, entry.mtime, entry.size = 0o600, 0, len(body)
            archive.addfile(entry, io.BytesIO(body))
    compressed = gzip.compress(buffer.getvalue(), mtime=0)
    put('originals.tar.gz', compressed)
    with tarfile.open(fileobj=io.BytesIO((ROOT / 'originals.tar.gz').read_bytes()), mode='r:gz') as archive:
        entries = archive.getmembers()
        require([entry.name for entry in entries] == sorted(members), 'exact archive roster')
        for entry in entries:
            require(entry.isfile() and entry.size == len(members[entry.name])
                    and archive.extractfile(entry).read() == members[entry.name], 'archive byte readback')
    cpu = json.loads((CPU / 'evidence/complete.json').read_bytes())
    manifest = dict(schema='ferric-fixed-formal-replay-publication-v1', replay_receipt=pin(raw),
                    selected_test=replay['selected_test'], stages=replay['stages'],
                    tests=replay['tests'], phases=3, elapsed_seconds=replay['elapsed_seconds'],
                    input_count=5846, compiled_test_artifact=cpu['artifacts']['test']['pin'],
                    execution_scope=replay['execution_scope'], archive=pin(compressed),
                    archive_scope=scope, members=len(members), originals=len(originals),
                    expanded_bytes=sum(map(len, members.values())), saved_leaf_pids_absent=sorted(set(retired)),
                    prior_capture_publication_archive=dict(
                        path=str(E / 'guarded-mlp-finite-predicate-inline-publication-v228-v1/originals.tar.gz'),
                        sha256='65008a7eb7aeb5435beac87a56a3138568ca6fdba18552a282b91894f6e4c68d'),
                    formal_preimage_constructed_and_replayed_in_memory=True,
                    formal_preimage_bytes_observed=595372, serialized_formal_preimage_retained=False,
                    unresolved_runtime_requirements=8, signed_verus_proof_observed=False,
                    worker_compilation_replayed=False, gpu_execution=False,
                    production_authority=False, numerical_acceptance=False, performance_claim=False)
    put('manifest.json', (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode())
    print(json.dumps(dict(manifest=pin(read(ROOT / 'manifest.json')), archive=pin(compressed),
                          members=len(members), originals=len(originals),
                          expanded_bytes=manifest['expanded_bytes']), sort_keys=True))


if __name__ == '__main__':
    main()
