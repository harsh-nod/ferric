"""Check or emit seven actual Tail V2 parent postimages; never apply or execute them."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import time

F = Path('/home/harsh/ferric-p227-integration')
RT = Path('/home/harsh/fe2o3-p228-runtime')
Q = F / 'qualification/guarded-mlp-scoped-tail-v1'
CAPSULE = Q / 'parent-cpu-v2'
WORKER_CAPSULE = Q / 'cpu-attempt-v2'
REMOTE = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v228-v2')
PARENT = 'ferric/adapters/m1-engineering-execution-v1/'
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
TERMINAL = {"bytes":4163125,"sha256":"a59c39cfa2be0ec11d0ffa6a352576814917243627ce9a73aca1d4f6501266e9"}
SOURCES = {"bytes":538263,"sha256":"f8b92a7b220c9733e75aa7745498d915217241369ea2a003edfbdda55f52d4b2"}
INPUT = {"bytes":273483,"sha256":"537f133a1588b1dc5c615b796b1ce597bf6ae53f6cb5fc6d8c802f4069a7853a"}
COLLECTOR = {"bytes":62183,"sha256":"65cf0ec579348ebaefeb676fcd107744cae21c7a0f9256135dd8f6e5d2c1c423"}
CONTROLLER = {"bytes":54467,"sha256":"032c79d4e94ea5f83aead5aa94c939d8b70bbb7058ecc8b9ef38ace4db15771b"}
BASE = {"bytes":525041,"sha256":"cf69d8bb33a1c9bf2fa0238e18da01ef70bba182a488388de378d1f611bc35fb"}
WORKER_TERMINAL = {"bytes":2448697,"sha256":"3c4fe3a6c4942b20ec61fd7ea5c3409526e2167c43d24a62fb8660c34b0e713e"}
WORKER_SOURCES = {"bytes":424416,"sha256":"5de4929b2832eda55a53f99945fc4b64a9fbda28b4d4f8bef07f2e8d5e000d6a"}
PROPOSAL = {"bytes":25554,"sha256":"cfc7add994656a5418fbc77abb0f6d16d820c3f1763ba8b5835b1e5df89bd3b2"}
BASE_TERMINAL = dict(bytes=4092678, sha256='6580942cb6b39d69b6fdac91a48ac6e0a25cd4e7648c4175152f3620ff7bacb6')
REPAIR = dict(bytes=4324, sha256='8e5ffe3f2b4d87b9967ef993e31385c42461dbdd92a49c1f63a2578e1b8e6128')
DEADLINE = float('inf')


def require(ok, why):
    if not ok:
        raise ValueError(why)


def tick():
    require(time.monotonic() < DEADLINE, 'integration data deadline')


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def valid_pin(row):
    return (type(row) is dict and set(row) == {'bytes', 'sha256'}
            and type(row['bytes']) is int and 0 < row['bytes'] <= 16 << 20
            and type(row['sha256']) is str and re.fullmatch('[0-9a-f]{64}', row['sha256']))


def parse(raw):
    def fields(rows):
        result = {}
        for name, value in rows:
            require(name not in result, 'duplicate JSON field')
            result[name] = value
        return result
    return json.loads(raw, object_pairs_hook=fields,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def read(path, expected=None):
    tick()
    before = path.lstat()
    require(path.is_absolute() and path.resolve(strict=True) == path
            and stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= 16 << 20, 'canonical bounded file')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    def stamp(row):
        return (row.st_dev, row.st_ino, row.st_size, row.st_mtime_ns, row.st_ctime_ns, row.st_mode)
    with os.fdopen(fd, 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input open drift')
        raw = stream.read((16 << 20) + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input read drift')
    require(stamp(path.lstat()) == stamp(before) and len(raw) == before.st_size
            and (expected is None or pin(raw) == compact(expected)), 'input pin drift: ' + str(path))
    return raw


def ordinary(name):
    return (type(name) is str and name not in ('', '.') and not Path(name).is_absolute()
            and Path(name).as_posix() == name and '..' not in Path(name).parts)


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + '\n').encode()


def main():
    global DEADLINE
    require(__debug__ and sys.dont_write_bytecode
            and ((len(sys.argv) == 3 and sys.argv[1] == 'emit')
                 or (len(sys.argv) == 2 and sys.argv[1] == 'check')),
            'python3 -B parent-integration-v2.py emit FRESH_DIRECTORY | check')
    mode = sys.argv[1]
    require(valid_pin(TERMINAL) and valid_pin(SOURCES), 'actual successful terminal/source pins pending')
    DEADLINE = time.monotonic() + 180
    def stop(_number, _frame):
        raise RuntimeError('integration data signal or deadline')
    for number in (signal.SIGALRM, signal.SIGTERM, signal.SIGHUP, signal.SIGINT):
        signal.signal(number, stop)
    signal.setitimer(signal.ITIMER_REAL, 180)
    for kind, cap in ((resource.RLIMIT_AS, 768 << 20), (resource.RLIMIT_FSIZE, 16 << 20),
                      (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        cap = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (cap, cap))
    out = Path(sys.argv[2]) if mode == 'emit' else None
    if out is not None:
        require(out.is_absolute() and out.parent.resolve(strict=True) == out.parent
                and not os.path.lexists(out) and all(out != root and root not in out.parents
                    for root in (F, RT, CAPSULE, WORKER_CAPSULE)),
                'fresh output outside repositories and evidence')
    manifest_raw = read(CAPSULE / 'manifest.json')
    manifest = parse(manifest_raw)
    require(manifest['schema'] == 'ferric-readiness40-bank-scoped-census-tail-parent-cpu-selected-evidence-v1'
            and manifest['terminal_name'] == 'complete.json' and manifest['passed'] is True
            and manifest['terminal'] == TERMINAL and len(manifest['files']) == 507,
            'complete actual 508-original parent capsule')
    bodies = {}
    for name, expected in manifest['files'].items():
        require(ordinary(name) and name != 'manifest.json', 'ordinary original capsule name')
        bodies[name] = read(CAPSULE / name, expected)
        require(sum(map(len, bodies.values())) <= 96 << 20, 'bounded selected originals')
    require(sum(map(len, bodies.values())) <= 96 << 20, 'bounded selected originals')
    bodies['manifest.json'] = manifest_raw
    def value(name, expected=None):
        raw = bodies[name]
        require(expected is None or pin(raw) == expected, 'known original body: ' + name)
        return parse(raw)
    result = value('evidence/complete.json', TERMINAL)
    after = value('evidence/sources-after.json', SOURCES)
    inputs = value('input-manifest.json', INPUT)
    require(pin(bodies['evidence.py']) == COLLECTOR and pin(bodies['run_cpu.py']) == CONTROLLER,
            'actual qualified controller and reviewed bound collector')
    require(result['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-tail-parent-cpu-v1'
            and result['source_generation'] == 'scoped-tail-parent-v2'
            and result['shared_test_fixture_repaired'] is True
            and result['passed'] is True and result['failure'] is None
            and result['postcheck_errors'] == [] and result['source_unchanged'] is True
            and result['all_selected_parent_tests_executed'] is True
            and len(result['phases']) == 68 and len(result['tests']) == 58
            and len(result['inventory']) == 1032 and len(result['raw']) == 344
            and len(result['artifacts']) == 7, 'complete successful qualification only')
    require(result['final_sources'] == result['input_sources'] == after
            and len(after) == len(inputs['files']) == 1293
            and {name: compact(row) for name, row in result['preformat_sources'].items()} == inputs['files'],
            'original preformat and invariant postformat maps')
    require(result['worker_qualification'] == WORKER_TERMINAL
            and result['worker_source_manifest'] == WORKER_SOURCES, 'actual successful worker lineage')
    for name, expected in result['raw'].items():
        require(expected['path'] == str(REMOTE / 'evidence' / name)
                and pin(bodies['evidence/' + name]) == compact(expected), 'all original raw bodies')
    for phase in result['phases']:
        label = phase['label']
        require(phase['exit_code'] == 0 and phase['natural_exit'] is True
                and phase['reaped'] is True and phase['process_group_absent'] is True
                and phase['forced_cleanup'] is False and phase['timed_out'] is False
                and phase['exception'] is None and phase['storage_failure'] is None
                and phase['observed_signals'] == []
                and value('evidence/' + label + '.result.json') == phase, 'natural retired original phase')
        command = value('evidence/' + label + '.command.json')
        started = value('evidence/' + label + '.started.json')
        require(command['argv'] == started['argv'] == phase['argv']
                and started['pid'] == started['pgid'] == phase['pid'] == phase['pgid'],
                'original command and process identity')
    for label, test in result['tests'].items():
        names = re.findall(r'^test (.+?) \.\.\. (ok|ignored)(?:,.*)?$',
                           bodies['evidence/' + label + '.stdout'].decode(), re.M)
        require(names == [(row['name'], row['outcome']) for row in test['named']]
                and all(outcome == 'ok' for _, outcome in names)
                and len(names) == test['passed'] and test['failed'] == test['ignored'] == 0,
                'exact original named successful tests')
    require(sum(row['passed'] for row in result['tests'].values()) == 568, 'all568 original selected passes')
    require(len(inputs['lineage']) == 131 and len(set(inputs['lineage'])) == 131
            and all('inputs/' + name in bodies for name in inputs['lineage']), 'all131 original lineage bodies')
    proposal = value('inputs/parent-proposal.json', PROPOSAL)
    require(proposal['source_generation'] == 'bank-scoped-census-tail-parent-v2'
            and proposal['requires']['worker_qualification_generation'] == 'scoped-tail-coupled-v2',
            'explicit repaired worker and parent source generation')
    repair = value('inputs/worker-repair.json', REPAIR)
    require(repair['source_generation'] == 'scoped-tail-shared-test-repair-v2',
            'original authenticated test-only repair')
    baseline = value('inputs/parent-sources.json', BASE)
    base_terminal = value('inputs/parent-complete.json', BASE_TERMINAL)
    require(base_terminal['passed'] is True and base_terminal['final_sources'] == baseline,
            'actual successful Full parent baseline, never the failed Tail parent')
    worker_raw = read(WORKER_CAPSULE / 'evidence/sources-after.json', WORKER_SOURCES)
    worker = parse(worker_raw)
    worker_terminal_raw = read(WORKER_CAPSULE / 'evidence/complete.json', WORKER_TERMINAL)
    worker_result = parse(worker_terminal_raw)
    require(pin(bodies['inputs/worker-sources.json']) == WORKER_SOURCES
            and pin(bodies['inputs/worker-complete.json']) == WORKER_TERMINAL
            and worker_result['passed'] is True
            and worker_result['source_generation'] == 'scoped-tail-coupled-v2'
            and worker_result['runtime_source_changed'] is False
            and worker_result['shared_test_fixture_repaired'] is True
            and worker_result['final_sources'] == worker, 'both retained original repaired worker joins')
    worker = {name: compact(row) for name, row in worker.items() if name.startswith(WORKER)}
    require(len(worker) == 228, 'all228 actual qualified worker sources')
    before = {name: compact(row) for name, row in baseline.items() if name.startswith('ferric/')}
    before.update(worker)
    final = {name: compact(row) for name, row in after.items() if name.startswith('ferric/')}
    require(len(before) == 1288 and len(final) == 1290
            and all(inputs['files'][name] == expected == final[name] for name, expected in worker.items()),
            'entire composed parent and unchanged actual worker map')
    rows = proposal['files']
    require(len(rows) == 7 and sum(row['before'] is None for row in rows) == 2
            and all(row['repository'] == 'ferric' and row['path'].endswith('.rs') for row in rows),
            'seven Rust parent rows and exactly two additions')
    overlay = {}
    for row in rows:
        name = 'ferric/' + row['path']
        require(name.startswith(PARENT) and name not in overlay
                and before.get(name) == row['before'] and inputs['files'][name] == row['after']
                and pin(bodies[name]) == final[name], 'authored predecessor to actual qualified postimage')
        overlay[name] = row
    require(sorted(overlay) == inputs['parent_overlay'], 'exact formatter ownership')
    formatted = sorted(name for name in after if compact(after[name]) != inputs['files'][name])
    require(len(result['format_changed_paths']) == len(set(result['format_changed_paths']))
            and sorted(result['format_changed_paths']) == formatted and set(formatted) <= set(overlay),
            'only exact parent formatter changes')
    require(set(final) == set(before) | set(overlay)
            and all(final[name] == expected for name, expected in before.items() if name not in overlay),
            'no unrelated postimage changes')
    lock = PARENT + 'Cargo.lock'
    require(before[lock] == final[lock] == pin(bodies[lock]), 'original parent lock remains exact')
    canonical = {}
    for name, expected in (before if mode == 'emit' else final).items():
        canonical[name] = read(F / name.removeprefix('ferric/'), expected)
    if mode == 'emit':
        for name in set(final) - set(before):
            require(not os.path.lexists(F / name.removeprefix('ferric/')), 'new parent path absent')
    for name, raw in bodies.items():
        require(read(CAPSULE / name) == raw, 'original capsule final posthash')
    require(read(WORKER_CAPSULE / 'evidence/sources-after.json') == worker_raw
            and read(WORKER_CAPSULE / 'evidence/complete.json') == worker_terminal_raw,
            'worker original final posthash')
    for name, raw in canonical.items():
        require(read(F / name.removeprefix('ferric/')) == raw, 'canonical final source posthash')
    tick()
    if mode == 'check':
        signal.setitimer(signal.ITIMER_REAL, 0)
        print(json.dumps(dict(schema='ferric-tail-parent-source-postcheck-v2', passed=True,
            terminal=TERMINAL, sources=SOURCES, canonical_postimages=len(final),
            unchanged_worker_bodies=len(worker), original_parent_lock_preserved=True,
            complete_postimage_map=final, native_execution=False,
            numerical_acceptance=False, performance_claim=False), sort_keys=True))
        return
    patch = ['*** Begin Patch\n']
    hunk_proofs = {}
    for name in sorted(overlay):
        path = F / name.removeprefix('ferric/')
        new = bodies[name].decode()
        require(new.endswith('\n'), 'qualified source terminal newline')
        new_lines = new.splitlines(keepends=True)
        require(new_lines and all(line.endswith('\n') and '\r' not in line for line in new_lines),
                'qualified LF source lines')
        if name in before:
            old = canonical[name].decode()
            require(old.endswith('\n'), 'canonical source terminal newline')
            old_lines = old.splitlines(keepends=True)
            require(old_lines and all(line.endswith('\n') and '\r' not in line
                                      for line in old_lines + new_lines), 'LF source lines')
            # One full-file hunk has exactly one possible old-line occurrence.
            starts = [i for i in range(len(old_lines)) if old_lines[i:i + len(old_lines)] == old_lines]
            require(starts == [0], 'unique complete old-file hunk context')
            hunk_proofs[name] = dict(old_file=pin(canonical[name]), new_file=final[name],
                old_lines=len(old_lines), matching_old_starts=starts, end_of_file_anchor=True)
            patch += ['*** Update File: ' + str(path) + '\n', '@@\n']
            patch += ['-' + line for line in old_lines]
        else:
            patch += ['*** Add File: ' + str(path) + '\n']
        patch += ['+' + line for line in new_lines]
        if name in before:
            patch += ['*** End of File\n']
    require(len(hunk_proofs) == 5, 'five unique replacement hunks and two additions')
    patch += ['*** End Patch\n']
    outputs = {'integration.patch': ''.join(patch).encode(),
        'source-map.json': encoded(dict(before=before, after=final)),
        'integration.json': encoded(dict(schema='ferric-tail-parent-source-integration-plan-v2',
            passed=True, terminal=TERMINAL, sources=SOURCES, input=INPUT,
            actual_worker_terminal=WORKER_TERMINAL, actual_worker_sources=WORKER_SOURCES,
            parent_rows={name: dict(before=overlay[name]['before'], after=final[name]) for name in sorted(overlay)},
            replacement_hunk_proofs=hunk_proofs, repair=REPAIR,
            canonical_preimages=1288, canonical_postimages=1290, unchanged_worker_bodies=228,
            original_parent_lock_preserved=True, patch_applied=False, native_execution=False,
            numerical_acceptance=False, performance_claim=False))}
    for name, raw in bodies.items():
        require(read(CAPSULE / name) == raw, 'original capsule final posthash')
    require(read(WORKER_CAPSULE / 'evidence/sources-after.json') == worker_raw
            and read(WORKER_CAPSULE / 'evidence/complete.json') == worker_terminal_raw,
            'worker original final posthash')
    for name, raw in canonical.items():
        require(read(F / name.removeprefix('ferric/')) == raw, 'canonical final preimage posthash')
    tick()
    out.mkdir(mode=0o700)
    for name, raw in outputs.items():
        require(len(raw) <= 16 << 20, 'bounded integration output')
        with (out / name).open('xb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        require(read(out / name) == raw, 'integration output readback')
    tick()
    signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=True, output=str(out),
        files={name: pin(raw) for name, raw in outputs.items()}, patch_applied=False), sort_keys=True))


if __name__ == '__main__':
    main()
