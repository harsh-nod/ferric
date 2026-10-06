"""Bounded CPU qualification of the standalone guarded Ferric worker."""

import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import sys
import time
import types


ROOT = Path(__file__).resolve().parent
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
EXPECTED_ROOT = E / 'guarded-mlp-worker-cpu-v228-v3'
INTERFACE = E / 'guarded-mlp-model-interface-cpu-v228-v1'
FERRIC = ROOT / 'ferric'
RUNTIME = ROOT / 'fe2o3'
CARGO_HOME = ROOT / 'cargo-home'
CACHE_MANIFEST_PIN = dict(bytes=22831, sha256='6c8b4eb4a9cf407dc0909dde3c574145ede603bdbe9abe8d4ff5bbc56a196ba1')
CACHE_LOCK_PIN = dict(bytes=7470, sha256='df2a4e0b9cf96687328a5b1e41937eb940e62144314d7f903c8c961da947f7aa')
WORKER = FERRIC / 'adapters/tp-peer-finite-engineering-worker-v1'
WORKER_REL = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
INTERFACE_COMPLETE = (1287578, '4a682798a23ac4c8accb0721f332a7b4e7484729bd692209d20f2883b501cee1')
INTERFACE_SOURCES = (321886, '569f5cdb89338ac9a8c8f0e759e455d876471be1e8f75989600cf0d1af185161')
HISTORY = {
    'worker_complete': ('worker-complete.json', 624332,
        '6ac67053d9b7d5d15772b5e4071a09933f013b6efa27394c31eb2b8279af884f'),
    'worker_tests': ('worker-tests.stdout', 66364,
        'c2248a930cb43027d787ed6a0017aafe4cce319469cbf6c82f3f6f1bc961e36d'),
    'worker_list': ('worker-list.stdout', 62453,
        '1df896119306dcd580d35184c26d3f4b802aa031f12c92dadb5c21c1f76f7576'),
}
PACKAGE = 'ferric-tp-peer-finite-engineering-worker-v1'
TARGETS = {
    'worker-lib': ('ferric_tp_peer_finite_engineering_worker_v1', 'lib', 'src/lib.rs'),
    'worker-bin-test': (PACKAGE, 'bin', 'src/main.rs'),
    'worker-wire-test': ('shared_wire', 'test', 'tests/shared_wire.rs'),
}
CRATES = {'fe2o3-amd-target', 'fe2o3-amdhsa-loader', 'fe2o3-aql', 'fe2o3-drm-uapi',
          'fe2o3-hsaco', 'fe2o3-kfd', 'fe2o3-kfd-uapi', 'fe2o3-runtime-model',
          'fe2o3-target-spec', PACKAGE}
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'metadata', 'worker-tests-build',
          'worker-list', 'worker-ignored', 'worker-tests', 'worker-build')
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 3600, 1800, 50


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def compact(rows):
    return {name: {key: row[key] for key in ('bytes', 'sha256')} for name, row in rows.items()}


def ordinary_relative(name):
    return (type(name) is str and Path(name).as_posix() == name
            and not Path(name).is_absolute() and '..' not in Path(name).parts
            and name not in ('', '.'))


def compact_pin(value):
    return (type(value) is dict and set(value) == {'bytes', 'sha256'}
            and type(value['bytes']) is int and 0 <= value['bytes'] <= 16 << 20
            and type(value['sha256']) is str and re.fullmatch(r'[0-9a-f]{64}', value['sha256']))


def load_supervisor():
    path = ROOT / 'supervisor.py'
    require(path.resolve(strict=True) == path and not path.is_symlink(), 'supervisor path')
    before = path.stat()
    require(before.st_size == 41485, 'supervisor extent')
    body = path.read_bytes()
    after = path.stat()
    stamp = lambda value: (value.st_dev, value.st_ino, value.st_size,
                           value.st_mtime_ns, value.st_ctime_ns)
    require(stamp(before) == stamp(after) and hashlib.sha256(body).hexdigest() == SUPERVISOR_SHA,
            'exact owned supervisor required')
    module = types.ModuleType('guarded_worker_owned_supervisor')
    module.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), module.__dict__)
    module.ROOT, module.SOURCE = ROOT, WORKER
    module.OUT, module.TARGET, module.TMP = OUT, TARGET, TMP
    return module


def source_snapshot(h):
    paths = [ROOT / 'run_cpu.py', ROOT / 'supervisor.py',
             *h.files_below(FERRIC), *h.files_below(RUNTIME)]
    require(809 < len(paths) <= 2000, 'closed worker/runtime source count bound')
    rows = {str(path.relative_to(ROOT)): h.pin(path) for path in sorted(paths)}
    require(len(rows) == len(paths) and sum(row['bytes'] for row in rows.values()) <= 64 << 20,
            'closed source extent bound')
    require(all(name in ('run_cpu.py', 'supervisor.py') or name.startswith('fe2o3/')
                or name.startswith(WORKER_REL) for name in rows), 'unexpected Ferric source subtree')
    return rows


def configurations(h):
    directories = {ROOT, FERRIC, RUNTIME, WORKER, *ROOT.parents, *WORKER.parents}
    paths = {directory / '.cargo' / name for directory in directories
             for name in ('config', 'config.toml')}
    paths |= {Path('/home/harmenon/.cargo') / name for name in ('config', 'config.toml')}
    paths |= {CARGO_HOME / name for name in ('config', 'config.toml')}
    result = {str(path): h.pin(path) if os.path.lexists(path) else None for path in sorted(paths)}
    require(not any(result.values()), 'inherited Cargo configuration refused')
    return result


def cache_contract(h):
    manifest_path = ROOT / 'cargo-cache-manifest.json'
    stage_path = ROOT / 'cargo-cache-stage-complete.json'
    manifest_pin, stage_pin = h.pin(manifest_path), h.pin(stage_path)
    require(compact({'manifest': manifest_pin})['manifest'] == CACHE_MANIFEST_PIN
            and stage_pin['bytes'] <= 1 << 20, 'actual private cache manifest/stage pins')
    manifest = json.loads(manifest_path.read_bytes())
    stage = json.loads(stage_path.read_bytes())
    require(manifest['schema'] == 'ferric-guarded-mlp-worker-cache-v1'
            and manifest['lock'] == CACHE_LOCK_PIN and len(manifest['packages']) == 29
            and len(manifest['files']) == 59 and manifest['cargo_execution'] is False
            and stage['schema'] == 'ferric-guarded-mlp-worker-cache-stage-v1'
            and stage['passed'] is True and stage['manifest'] == CACHE_MANIFEST_PIN
            and stage['cargo_home'] == str(CARGO_HOME) and stage['lock'] == CACHE_LOCK_PIN
            and stage['files'] == manifest['files'] and stage['packages'] == 29 and stage['cache_files'] == 59
            and all(stage[key] is False for key in ('shared_cache_changed', 'lock_changed',
                        'project_code_executed', 'crate_sources_extracted')), 'private cache stage contract')
    lock = h.pin(WORKER / 'Cargo.lock')
    require(compact({'lock': lock})['lock'] == CACHE_LOCK_PIN, 'locked dependency closure unchanged')
    files = {}
    for name, expected in manifest['files'].items():
        require(ordinary_relative(name) and name.startswith('registry/') and compact_pin(expected),
                'private immutable cache input row')
        path = CARGO_HOME / name
        require(path.resolve(strict=True) == path, 'private cache input alias')
        row = h.pin(path)
        require(compact({'cache': row})['cache'] == expected, 'private cache input identity')
        files[name] = row
    require(h.pin(manifest_path) == manifest_pin and h.pin(stage_path) == stage_pin,
            'private cache provenance changed during read')
    return dict(manifest=manifest_pin, stage=stage_pin, lock=lock, files=files,
                packages=manifest['packages'], inner_members=manifest['inner_members'],
                inner_expanded_bytes=manifest['inner_expanded_bytes'], cargo_home=str(CARGO_HOME))


def inventory(text):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.M)
    require(len(names) == len(set(names)) and ': benchmark' not in text,
            'closed unique worker test inventory')
    return sorted(names)


def outcomes(text):
    summaries, named, active, progress = [], [], [], set()
    for line in text.splitlines():
        notice = re.fullmatch(r'test ([A-Za-z0-9_:]+) has been running for over 60 seconds', line)
        if notice:
            name = notice.group(1)
            require(name not in progress and name not in {row['name'] for row in named},
                    'duplicate or completed worker progress notice')
            progress.add(name)
            continue
        result = re.fullmatch(r'test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?', line)
        if result:
            name, status = result.groups()
            row = dict(name=name, outcome=status)
            require(name not in {value['name'] for value in named}, 'duplicate named worker outcome')
            require(status != 'ignored' or name not in progress, 'ignored worker progress notice')
            named.append(row)
            active.append(row)
            continue
        summary = re.fullmatch(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
            r'(\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s', line)
        if summary:
            status, passed, failed, ignored, measured, filtered = summary.groups()
            row = dict(status=status, passed=int(passed), failed=int(failed), ignored=int(ignored),
                       measured=int(measured), filtered_out=int(filtered))
            require(all(sum(item['outcome'] == outcome for item in active) == row[key]
                        for outcome, key in (('ok', 'passed'), ('FAILED', 'failed'), ('ignored', 'ignored'))),
                    'worker named outcome and target summary differ')
            summaries.append(row)
            active = []
        else:
            require(not line.startswith('test '), 'malformed worker libtest result')
    require(summaries and not active and progress <= {row['name'] for row in named},
            'incomplete worker libtest outcome')
    return dict(summaries=summaries, named=named,
                passed=sum(row['passed'] for row in summaries),
                failed=sum(row['failed'] for row in summaries),
                ignored=sum(row['ignored'] for row in summaries))


def statuses(value):
    return {row['name']: row['outcome'] for row in value['named']}


def lineage_contract(h, inputs, before, readset):
    lineage = inputs['source_lineage']
    require(set(lineage) == {'interface_complete', 'interface_sources', 'worker_snapshot', *HISTORY},
            'closed worker source lineage')
    expected = {
        'interface_complete': (INTERFACE / 'evidence/complete.json', *INTERFACE_COMPLETE),
        'interface_sources': (INTERFACE / 'evidence/sources-after.json', *INTERFACE_SOURCES),
        **{key: (ROOT / 'inputs' / name, size, digest)
           for key, (name, size, digest) in HISTORY.items()},
    }
    snapshot_row = lineage['worker_snapshot']
    require(type(snapshot_row) is dict and set(snapshot_row) == {'path', 'bytes', 'sha256'}
            and snapshot_row['path'] == str(ROOT / 'inputs/worker-source.json')
            and compact_pin({key: snapshot_row[key] for key in ('bytes', 'sha256')}),
            'worker source snapshot pin')
    expected['worker_snapshot'] = (Path(snapshot_row['path']), snapshot_row['bytes'], snapshot_row['sha256'])
    bodies = {}
    for name, (path, size, digest) in expected.items():
        require(path.resolve(strict=True) == path and size <= 16 << 20, 'lineage ordinary path/extent')
        row = h.pin(path)
        require(row == lineage[name] and (row['bytes'], row['sha256']) == (size, digest),
                'worker lineage mismatch: ' + name)
        readset[name] = row
        bodies[name] = path.read_bytes()
        require(h.pin(path) == row, 'lineage changed during read')
    base = json.loads(bodies['interface_complete'])
    source_map = json.loads(bodies['interface_sources'])
    require(base['schema'] == 'ferric-guarded-mlp-interleaved-native-cpu-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['gpu_execution'] is False
            and base['public_engineering_guarded_interface_added'] is True
            and base['mixed_bank_implemented'] is True and base['native_mixed_bank_qualified'] is False
            and base['source_generation'] == GENERATION
            and base['input_sources'] == base['final_sources'] == source_map
            and base['raw']['sources-after.json'] == readset['interface_sources'],
            'qualified interface CPU lineage')
    require(len(base['phases']) == 20 and all(row['exit_code'] == 0 and row['natural_exit'] is True
            and row['reaped'] is True and row['process_group_absent'] is True
            and row['forced_cleanup'] is False and row['timed_out'] is False
            and row['exception'] is None for row in base['phases']), 'interface lifecycle')
    require(len(source_map) == 809 and all(ordinary_relative(name)
            and row['path'] == str(INTERFACE / name) for name, row in source_map.items()),
            'qualified interface source-map extent/path')
    runtime = {name: {key: row[key] for key in ('bytes', 'sha256')}
               for name, row in source_map.items() if name.startswith('fe2o3/')}
    require(len(runtime) == 807 and compact({name: row for name, row in before.items()
            if name.startswith('fe2o3/')}) == runtime, 'immutable qualified runtime copy')
    snapshot = json.loads(bodies['worker_snapshot'])
    require(set(snapshot) == {'schema', 'revision', 'files', 'overlay'}
            and snapshot['schema'] == 'ferric-guarded-mlp-worker-source-v1'
            and re.fullmatch(r'[0-9a-f]{40}', snapshot['revision'])
            and 1 <= len(snapshot['files']) <= 512 and 1 <= len(snapshot['overlay']) <= 128,
            'bounded worker source snapshot')
    worker = dict(snapshot['files'])
    require(all(ordinary_relative(name) and name.startswith(WORKER_REL) and compact_pin(row)
                for name, row in worker.items()), 'worker base source rows')
    require(sorted(snapshot['overlay']) == inputs['worker_overlay'], 'closed worker overlay roster')
    for name, row in snapshot['overlay'].items():
        require(ordinary_relative(name) and name.startswith(WORKER_REL + 'src/') and name.endswith('.rs')
                and set(row) == {'before', 'after'} and worker.get(name) == row['before']
                and compact_pin(row['after']), 'worker overlay source identity')
        worker[name] = row['after']
    require(compact({name: row for name, row in before.items() if name.startswith(WORKER_REL)}) == worker,
            'exact worker base-plus-overlay source closure')
    require(all(WORKER_REL + name in worker for name in
                ('Cargo.toml', 'Cargo.lock', 'src/lib.rs', 'src/main.rs', 'tests/shared_wire.rs')),
            'complete worker Cargo/lib/bin/integration inputs')
    history = json.loads(bodies['worker_complete'])
    prior = outcomes(bodies['worker_tests'].decode('utf-8'))
    listed = inventory(bodies['worker_list'].decode('utf-8'))
    require(history['schema'] == 'ferric-p228-projection-ordered-segment-cpu-result-v1'
            and history['passed'] is True and history['error'] is None
            and history['postcheck_errors'] == [] and history['gpu_execution'] is False,
            'qualified ordered worker baseline')
    for key, raw_name in (('worker_tests', 'worker-tests-stdout'), ('worker_list', 'worker-list-stdout')):
        require({k: history['raw'][raw_name][k] for k in ('bytes', 'sha256')}
                == {k: readset[key][k] for k in ('bytes', 'sha256')}, 'historical worker raw pin join')
    observation = history['tests']['worker-tests']
    require((prior['passed'], prior['failed'], prior['ignored']) == (562, 0, 4)
            and [[row[key] for key in ('passed', 'failed', 'ignored')] for row in prior['summaries']]
                == observation['summaries'] == [[549, 0, 4], [13, 0, 0]]
            and listed == sorted(statuses(prior)) == observation['names']
            and len(listed) == 566 and observation['passed'] == 562 and observation['ignored'] == 4,
            'exact historical worker named outcomes')
    additions = inputs['new_tests']
    require(set(additions) == set(TARGETS) and all(type(names) is list and names == sorted(set(names))
            and len(names) <= 256 and all(type(name) is str and re.fullmatch(r'[A-Za-z0-9_:]+', name)
                for name in names) for names in additions.values()), 'closed per-target additions')
    new = [name for names in additions.values() for name in names]
    expected_names = statuses(prior)
    require(new and len(new) == len(set(new)) and not set(new) & set(expected_names),
            'new worker tests must be unique additions')
    expected_names.update({name: 'ok' for name in new})
    summaries = [dict(prior['summaries'][0]),
                 dict(status='ok', passed=0, failed=0, ignored=0, measured=0, filtered_out=0),
                 dict(prior['summaries'][1])]
    for row, role in zip(summaries, TARGETS):
        row['passed'] += len(additions[role])
    return base, prior, expected_names, summaries, snapshot['revision']


def metadata_contract(h):
    value = json.loads((OUT / 'metadata.stdout').read_bytes())
    require(value['workspace_root'] == str(WORKER) and value['target_directory'] == str(TARGET),
            'standalone worker workspace/target')
    packages = {row['id']: row for row in value['packages']}
    require(len(packages) == len(value['packages']) <= 256, 'metadata package bound/uniqueness')
    members = [packages[key] for key in value['workspace_members']]
    require(len(members) == 1 and members[0]['name'] == PACKAGE
            and members[0]['manifest_path'] == str(WORKER / 'Cargo.toml'), 'standalone worker workspace')
    local, external = {}, {}
    for row in packages.values():
        path = Path(row['manifest_path'])
        require(path.resolve(strict=True) == path, 'dependency path alias')
        if row['source'] is None:
            expected = WORKER / 'Cargo.toml' if row['name'] == PACKAGE else RUNTIME / 'crates' / row['name'] / 'Cargo.toml'
            require(path == expected and row['name'] not in local, 'unlisted/duplicate local dependency')
            local[row['name']] = str(path)
        else:
            require(path.is_relative_to(CARGO_HOME)
                    and row['source'].startswith(('registry+', 'git+')), 'external dependency location')
            external[str(path.parent)] = {str(p.relative_to(path.parent)): h.pin(p)
                                         for p in h.files_below(path.parent, packed=False)}
    require(set(local) == CRATES, 'closed ten local packages')
    targets = members[0]['targets']
    for name, kind, source in TARGETS.values():
        require(sum(row['name'] == name and row['kind'] == [kind]
                    and row['src_path'] == str(WORKER / source) for row in targets) == 1,
                'selected worker target absent or ambiguous')
    return external, local


def selected_artifact(h, records, name, kind, source, testing):
    rows = [row for row in records if row.get('reason') == 'compiler-artifact'
            and row.get('manifest_path') == str(WORKER / 'Cargo.toml')
            and row.get('target', {}).get('name') == name
            and row['target']['kind'] == [kind] and row.get('profile', {}).get('test') is testing]
    require(len(rows) == 1, 'unique worker artifact')
    row = rows[0]
    require(row['target']['src_path'] == str(WORKER / source) and row['features'] == []
            and type(row.get('executable')) is str, 'worker artifact source/features')
    path = Path(row['executable'])
    require(path.is_relative_to(TARGET) and path.resolve(strict=True) == path
            and row['filenames'].count(str(path)) == 1, 'fresh target worker artifact')
    with path.open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'worker artifact ELF magic')
    return dict(pin=h.pin(path), cargo_artifact=row)


def main():
    started = time.monotonic()
    deadline = started + WHOLE_WALL
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]), 'python3 -B run_cpu.py INPUT_SHA')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and not any(os.path.lexists(path) for path in (OUT, TARGET, TMP)), 'fresh exact worker outputs')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID mismatch')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, '40 GiB initial free floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice level')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30),
                      (resource.RLIMIT_FSIZE, 1 << 30)):
        soft, hard = resource.getrlimit(kind)
        value = min([cap] + [value for value in (soft, hard) if value != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h = load_supervisor()
    h.sources = lambda: source_snapshot(h)
    original_scratch_bytes = h.scratch_bytes
    def scratch_bytes():
        total = original_scratch_bytes()
        if CARGO_HOME.exists():
            paths = h.files_below(CARGO_HOME, packed=False)
            require(len(paths) <= 10000, 'private Cargo cache file-count bound')
            for path in paths:
                try:
                    total += path.lstat().st_size
                except FileNotFoundError:
                    pass
        return total
    h.scratch_bytes = scratch_bytes
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - CLEANUP_RESERVE - time.monotonic()))
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    inputs = input_pin = preformat = formatted = final = config = baseline = revision = base = None
    readset, tool_pins, external, external_after, local, artifacts, tests = {}, {}, {}, {}, {}, {}, {}
    inventory_names, ignored, phases, errors, format_changed = [], [], [], [], []
    failure = None
    cache_provenance, cache_postchecked = None, False
    try:
        input_pin = h.pin(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'literal worker input mismatch')
        inputs = json.loads((ROOT / 'input-manifest.json').read_bytes())
        require(set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins',
                                'source_lineage', 'worker_overlay', 'new_tests'}
                and inputs['schema'] == 'ferric-guarded-mlp-worker-cpu-input-v1'
                and inputs['source_generation'] == GENERATION
                and type(inputs['worker_overlay']) is list
                and inputs['worker_overlay'] == sorted(set(inputs['worker_overlay'])),
                'worker input fields/generation')
        preformat = source_snapshot(h)
        require(inputs['files'] == compact(preformat), 'root-pinned worker preformat source map')
        require((WORKER / '../../../fe2o3').resolve(strict=True) == RUNTIME,
                'locked worker dependency must resolve to the qualified runtime')
        h.save('sources-preformat.json', preformat)
        base, baseline, expected_names, expected_summaries, revision = lineage_contract(h, inputs, preformat, readset)
        config = configurations(h)
        tool_pins = {name: h.pin(h.TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
        tool_pins['prlimit'] = h.pin(Path('/usr/bin/prlimit'))
        for name, size in h.SHARED_LIBRARIES.items():
            path = h.TOOLCHAIN_LIB / name
            require(path.resolve(strict=True) == path, 'tool shared library alias')
            tool_pins[name] = h.pin(path)
            require(tool_pins[name]['bytes'] == size, 'tool shared library extent')
        require(tool_pins == inputs['tool_pins'] == base['tool_pins'], 'qualified interface toolchain')
        cache_provenance = cache_contract(h)
        env = dict(HOME='/home/harmenon', PATH=str(h.TOOLCHAIN) + ':/usr/bin:/bin',
            CARGO_HOME=str(CARGO_HOME), CARGO_TARGET_DIR=str(TARGET),
            LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(h.TOOLCHAIN_LIB),
            TMPDIR=str(TMP), RUSTC=str(h.TOOLCHAIN / 'rustc'), RUSTDOC=str(h.TOOLCHAIN / 'rustdoc'),
            CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
            CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never', MALLOC_ARENA_MAX='2', RUST_BACKTRACE='1',
            CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0',
            CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
            CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
            CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
            ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        def leaf(label, argv, seconds=LEAF_WALL, tested=None):
            h.run(label, argv, env, phases, deadline, tested, seconds, WORKER)
        fmt = [str(h.TOOLCHAIN / 'rustfmt'), '--edition', '2024', '--config', 'skip_children=true']
        overlay = [str(ROOT / name) for name in inputs['worker_overlay']]
        leaf('rustfmt', [*fmt, *overlay], 120)
        formatted = source_snapshot(h)
        require(set(formatted) == set(preformat), 'rustfmt changed the source roster')
        format_changed = sorted(name for name in formatted if formatted[name] != preformat[name])
        require(set(format_changed) <= set(inputs['worker_overlay']), 'rustfmt touched outside declared overlay')
        h.save('sources-before.json', formatted)
        leaf('rustfmt-check', [*fmt, '--check', *overlay], 120, formatted)
        cargo = str(h.TOOLCHAIN / 'cargo')
        common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(WORKER / 'Cargo.toml')]
        selected = [*common, '--lib', '--bin', PACKAGE, '--test', 'shared_wire']
        leaf('rustc-version', [str(h.TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60, formatted)
        leaf('metadata', [cargo, 'metadata', '--offline', '--locked', '--format-version', '1',
                         '--manifest-path', str(WORKER / 'Cargo.toml')], 120, formatted)
        external, local = metadata_contract(h)
        h.save('dependencies-before.json', external)
        leaf('worker-tests-build', [cargo, 'test', *selected, '--no-run', '--message-format=json'], tested=formatted)
        records = h.build_records(OUT / 'worker-tests-build.stdout')
        require(sum(row.get('reason') == 'compiler-artifact'
                    and row.get('manifest_path') == str(WORKER / 'Cargo.toml')
                    and row.get('profile', {}).get('test') is True for row in records) == 3,
                'exact three selected worker test artifacts')
        for role, (name, kind, source) in TARGETS.items():
            artifacts[role] = selected_artifact(h, records, name, kind, source, True)
        require(len({row['pin']['path'] for row in artifacts.values()}) == 3, 'distinct test products')
        leaf('worker-list', [cargo, 'test', *selected, '--', '--list', '--format=terse'], 120, formatted)
        leaf('worker-ignored', [cargo, 'test', *selected, '--', '--ignored', '--list', '--format=terse'], 120, formatted)
        inventory_names = inventory((OUT / 'worker-list.stdout').read_text())
        ignored = inventory((OUT / 'worker-ignored.stdout').read_text())
        require(inventory_names == sorted(expected_names)
                and ignored == sorted(name for name, status in expected_names.items() if status == 'ignored'),
                'actual worker inventory differs from exact historical-plus-added roster')
        leaf('worker-tests', [cargo, 'test', *selected, '--', '--test-threads=1'], tested=formatted)
        value = outcomes((OUT / 'worker-tests.stdout').read_text())
        require(value['summaries'] == expected_summaries and statuses(value) == expected_names,
                'actual full worker named outcomes differ')
        tests['worker-tests'] = value
        leaf('worker-build', [cargo, 'build', *common, '--bin', PACKAGE, '--message-format=json'], tested=formatted)
        records = h.build_records(OUT / 'worker-build.stdout')
        artifacts['worker'] = selected_artifact(h, records, PACKAGE, 'bin', 'src/main.rs', False)
        require(len({row['pin']['path'] for row in artifacts.values()}) == 4,
                'production binary must be distinct from three test artifacts')
        require([row['label'] for row in phases] == list(PHASES), 'closed nine worker CPU phases')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(label, action):
        try:
            remaining = deadline - 5 - time.monotonic()
            require(remaining > 0, 'postcheck deadline')
            signal.setitimer(signal.ITIMER_REAL, remaining)
            action()
        except BaseException as error:
            errors.append(label + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    def source_check():
        nonlocal final
        final = source_snapshot(h)
        h.save('sources-after.json', final)
        if formatted is not None:
            require(final == formatted, 'qualified postformat source or lock drift')
        elif preformat is not None:
            require(set(final) == set(preformat) and all(final[name] == row
                for name, row in preformat.items() if name not in (inputs or {}).get('worker_overlay', [])),
                'failed format modified outside overlay')
    check('sources', source_check)
    if input_pin is not None:
        check('input', lambda: require(h.pin(ROOT / 'input-manifest.json') == input_pin, 'input drift'))
    if config is not None:
        check('configuration', lambda: require(configurations(h) == config, 'configuration drift'))
    if cache_provenance is not None:
        def cache_check():
            nonlocal cache_postchecked
            require(cache_contract(h) == cache_provenance, 'immutable private cache input/provenance drift')
            cache_postchecked = True
        check('private cache inputs', cache_check)
    for label, row in dict(tool_pins, **readset).items():
        check('readset ' + label, lambda row=row: require(h.pin(Path(row['path'])) == row, 'readset drift'))
    for directory, expected in external.items():
        def dependency_check(directory=directory, expected=expected):
            path = Path(directory)
            actual = {str(p.relative_to(path)): h.pin(p) for p in h.files_below(path, packed=False)}
            external_after[directory] = actual
            require(actual == expected, 'external dependency drift')
        check('dependency ' + directory, dependency_check)
    check('dependency ledger', lambda: h.save('dependencies-after.json', external_after))
    for label, artifact in artifacts.items():
        check('artifact ' + label, lambda artifact=artifact: require(
            h.pin(Path(artifact['pin']['path'])) == artifact['pin'], 'selected artifact drift'))
    raw = {}
    def raw_check():
        nonlocal raw
        raw = {path.name: h.pin(path) for path in sorted(OUT.iterdir()) if path.is_file()}
        for phase in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw[Path(phase[key]['path']).name] == phase[key], 'raw phase join')
    check('raw evidence', raw_check)
    failure = failure or ('postcheck failed' if errors else None)
    if time.monotonic() >= deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-guarded-mlp-worker-cpu-v1', passed=failure is None,
        failure=failure, postcheck_errors=errors, source_generation=GENERATION,
        controller=h.pin(ROOT / 'run_cpu.py'), supervisor=h.pin(ROOT / 'supervisor.py'),
        input_manifest=input_pin, source_lineage=inputs['source_lineage'] if inputs else None,
        readset=readset, worker_source_revision=revision, baseline_tests=baseline,
        preformat_sources=preformat, input_sources=formatted, final_sources=final,
        source_unchanged=formatted is not None and final == formatted, format_changed_paths=format_changed,
        phases=phases, artifacts=artifacts, tests=tests, inventory=inventory_names, ignored=ignored,
        full_worker_tests_executed='worker-tests' in tests, tool_pins=tool_pins, raw=raw,
        local_dependencies=local, configurations=config, elapsed_seconds=time.monotonic() - started,
        cache_provenance=cache_provenance, cache_inputs_unchanged=cache_postchecked,
        limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL,
            cleanup_reserve_seconds=CLEANUP_RESERVE, cpu_seconds=h.CPU_LIMIT,
            address_space_bytes=h.AS_LIMIT, file_bytes=h.FILE_LIMIT,
            cache_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT,
            initial_free_bytes=h.START_FREE, live_free_bytes=h.LIVE_FREE,
            affinity=[8, 9], nice=10, cargo_jobs=2),
        runtime_source_qualified_by_interface_cpu=base is not None, runtime_suite_rerun=False,
        worker_guarded_source_compiled='worker' in artifacts, gpu_execution=False,
        native_guarded_worker_qualified=False, whole_model_guarded_execution=False,
        numerical_acceptance=False, production_authority=False, performance_claim=False)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
    try:
        h.save('complete.json' if failure is None else 'failed.json', result)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=failure is None, failure=failure, phases=len(phases),
        tests={name: {key: value[key] for key in ('passed', 'failed', 'ignored')}
               for name, value in tests.items()}, output=str(OUT)), sort_keys=True))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
