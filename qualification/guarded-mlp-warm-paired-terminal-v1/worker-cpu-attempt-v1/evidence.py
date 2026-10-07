"""Retain the observed V1 rustfmt failure unchanged; data only."""
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import resource
import signal
import stat
import sys
import tarfile

W = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004')
F = Path('/home/harsh/ferric-p227-integration')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = str(E / 'guarded-mlp-warm-paired-terminal-worker-cpu-v228-v1')
DEST = F / 'qualification/guarded-mlp-warm-paired-terminal-v1/worker-cpu-attempt-v1'
BASENAME = 'guarded-mlp-warm-paired-terminal-worker-cpu-failed-v228-v1.tar.gz'
TERMINAL_PIN = (1036998, '137f9861d6ec1a23ad076c186190882bf760293ea8b6b29c52ffefe3659590f6')
CONTROLLER_PIN = (39374, '0419c838930119d28fa42668a2155baff5f7b7d407b082728245393f9eea8c47')
INPUT_PIN = (210358, '175561b9dca4ac9a5b80ccb73008ed756b9ed43fad66dae3ff82ea3aa072cda3')
SOURCE_ARCHIVE = dict(bytes=817841, sha256='eb5eba5a1cb3788005d62304aa5bbfcf4311fce867b9154cef0f30ff9168d22f')
PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
STAGERS = {
    'source-transport.py': (E / 'warm-paired-terminal-worker-transport-v1.py',
        (18563, '3539a9e6572d1d87c13f284b4b7c325455edbce712561757e736dbc57c521361')),
    'cache-transport.py': (E / 'warm-paired-terminal-worker-cache-v1.py',
        (12805, '6158399f5ab1d162eae089b45c5db5e2a81be7011ba9637128ea09a0cdc36c16')),
}
LINEAGE = {
    'worker_complete': ('worker-complete.json', (1704432, '728339a954436b3e191e5efac8c42146521f7acd57c73ac81dc0d4ad55cdd421')),
    'worker_sources': ('worker-sources.json', (420819, 'c75c35bd7c987a4e52f1897a3d7afd2923e32b6d4768e5420473aa3c4527ce6c')),
    'worker_tests': ('worker-tests.stdout', (78963, '8fceb3e8d48f4cfe7460ba72e4517d7332a9033703b7e534fab61dadc7949925')),
    'worker_list': ('worker-list.stdout', (74219, '47523a1dc84e0bf495d268895beb990430ce0f27307b780d3dec9dc725a4a95d')),
    'terminal_proposal': ('terminal-source-manifest.json', (10821, '6422971e027dfb90c6ac962d72a7cb977f40e60bbd7edbc5bc52c5fe91a36e8a')),
    'runtime_complete': ('runtime-complete.json', (2250677, 'bf0fa20a37ff3bf748ef5984df0dceb5cd1c453ef5c05c7111779b31cda33720')),
    'runtime_sources': ('runtime-sources.json', (407038, 'f8024397177d83cdd1d07040b55c97afc5190988676bb80f67a9af4ae0554718')),
}
MEMBERS = 36
MAX_FILE, MAX_TOTAL = 16 << 20, 64 << 20

def require(ok, message):
    if not ok:
        raise RuntimeError(message)

def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())

def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}

def exact(body, expected):
    require((len(body), hashlib.sha256(body).hexdigest()) == expected, 'fixed observed body pin')

def parse(body):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(body, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))

def read(path, limit):
    require(path.resolve(strict=True) == path, 'canonical ordinary input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= limit, 'bounded regular input')
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed before read')
        body = stream.read(limit + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed during read')
    require(stamp(path.lstat()) == stamp(before) and len(body) == before.st_size, 'input changed after read')
    return body

def named(body):
    rows = re.findall(r'^test (\S+) \.\.\. (ok|ignored)(?:, [^\n]*)?$', body.decode(), re.M)
    require(len(rows) == len({name for name, _ in rows}), 'duplicate named test')
    return dict(rows)

def validate(bodies, terminal_sha):
    require(terminal_sha == TERMINAL_PIN[1], 'exact observed failed attempt')
    exact(bodies['evidence/failed.json'], TERMINAL_PIN)
    exact(bodies['run_cpu.py'], CONTROLLER_PIN)
    exact(bodies['input-manifest.json'], INPUT_PIN)
    c = parse(bodies['evidence/failed.json'])
    i = parse(bodies['input-manifest.json'])
    require(c['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-worker-cpu-v1'
            and c['passed'] is False and c['failure'] ==
                "RuntimeError('rustfmt did not finish naturally/reaped/successfully')"
            and c['postcheck_errors'] == [] and c['input_sources'] is None
            and c['source_unchanged'] is False and c['format_changed_paths'] == []
            and c['cache_inputs_unchanged'] is True, 'original failed formatting gate')
    require(c['tests'] == c['artifacts'] == c['local_dependencies'] == {}
            and c['inventory'] == c['ignored'] == [] and c['cli_executable_before_tests'] is None
            and c['cli_executable_unchanged_across_tests'] is False
            and c['full_worker_tests_executed'] is False
            and c['worker_guarded_source_compiled'] is False, 'no build or current test result')
    require(all(c[k] is False for k in ('gpu_execution', 'warm_paired_terminal_native_execution',
                'position5_native_execution', 'readiness40_native_execution', 'full2303_native_enabled',
                'paired_read_native_execution', 'runtime_suite_rerun', 'global_currentness_policy_changed',
                'hidden_read_policy_changed', 'default_policy_changed', 'performance_policy_changed',
                'native_guarded_worker_qualified', 'whole_model_guarded_execution', 'full_long_workload',
                'numerical_acceptance', 'performance_claim', 'production_authority')), 'failed CPU-only scope')
    require(c['source_lineage'] == c['readset'] == i['source_lineage']
            and set(c['readset']) == set(LINEAGE), 'seven complete original lineage bodies')
    for role, (name, expected) in LINEAGE.items():
        exact(bodies['inputs/' + name], expected)
        require(c['readset'][role]['path'] == ROOT + '/inputs/' + name
                and compact(c['readset'][role]) == pin(bodies['inputs/' + name]), 'exact lineage pin')
    old = parse(bodies['inputs/worker-complete.json'])
    old_map = parse(bodies['inputs/worker-sources.json'])
    runtime = parse(bodies['inputs/runtime-complete.json'])
    runtime_map = parse(bodies['inputs/runtime-sources.json'])
    require(old['passed'] is True and old['failure'] is None and old['postcheck_errors'] == []
            and old['input_sources'] == old['final_sources'] == old_map and len(old_map) == 1010
            and runtime['passed'] is True and runtime['failure'] is None
            and runtime['postcheck_errors'] == [] and runtime['full_runtime_tests_executed'] is True
            and runtime['terminal_pair_runtime_added'] is True and runtime['gpu_execution'] is False
            and runtime['input_sources'] == runtime['final_sources'] == runtime_map
            and len(runtime_map) == 1014
            and old['tool_pins'] == runtime['tool_pins'] == i['tool_pins'] == c['tool_pins'],
            'separate actual worker and runtime qualifications')
    require(c['baseline_tests'] == old['tests']['worker-tests']
            and named(bodies['inputs/worker-tests.stdout']) ==
                {r['name']: r['outcome'] for r in c['baseline_tests']['named']}
            and {k: c['baseline_tests'][k] for k in ('passed', 'failed', 'ignored')} ==
                dict(passed=663, failed=0, ignored=4), 'historical tests are not current passes')
    proposal = parse(bodies['inputs/terminal-source-manifest.json'])
    overlay = {'ferric/' + r['path']: r for r in proposal['files']
               if r['repository'] == 'ferric' and ('ferric/' + r['path']).startswith(PREFIX)}
    require(len(overlay) == 11 and set(overlay) == set(i['worker_overlay']), 'eleven original overlays')
    pre = {n: compact(r) for n, r in c['preformat_sources'].items()}
    final = {n: compact(r) for n, r in c['final_sources'].items()}
    expected_pre = {n: compact(r) for n, r in runtime_map.items() if n.startswith('fe2o3/')}
    require(len(expected_pre) == 815, 'exact qualified terminal runtime')
    worker = {n: compact(r) for n, r in old_map.items() if n.startswith(PREFIX)}
    require(len(worker) == 195, 'exact current worker')
    expected_pre.update(worker)
    for name, row in overlay.items():
        require(expected_pre[name] == row['before'], 'overlay original preimage')
        expected_pre[name] = row['after']
    expected_pre.update({n: pin(bodies[n]) for n in ('run_cpu.py', 'supervisor.py')})
    require(pre == i['files'] == expected_pre and set(pre) == set(final) and len(pre) == 1012,
            'complete staged source reconstruction')
    changed = sorted(n for n in pre if pre[n] != final[n])
    require(len(changed) == 9 and set(changed) <= set(overlay), 'actual partial formatter changes only')
    require(parse(bodies['evidence/sources-preformat.json']) == c['preformat_sources']
            and parse(bodies['evidence/sources-after.json']) == c['final_sources']
            and parse(bodies['evidence/dependencies-after.json']) == {}, 'original final and partial maps')
    require(len(c['phases']) == 1, 'only rustfmt was attempted')
    phase = c['phases'][0]
    require(phase['label'] == 'rustfmt' and phase['exit_code'] == 1 and phase['natural_exit'] is True
            and phase['reaped'] is True and phase['process_group_absent'] is True
            and phase['forced_cleanup'] is False and phase['timed_out'] is False
            and phase['exception'] is None and phase['storage_failure'] is None
            and phase['observed_signals'] == [], 'clean owned nonzero formatter exit')
    require(parse(bodies['evidence/rustfmt.result.json']) == phase
            and parse(bodies['evidence/rustfmt.started.json']) ==
                dict(pid=phase['pid'], pgid=phase['pgid'], argv=phase['argv']), 'raw lifecycle joins')
    for key, suffix in (('command', '.command.json'), ('stdout', '.stdout'), ('stderr', '.stderr')):
        require(phase[key] == c['raw']['rustfmt' + suffix], 'original formatter stream joins')
    raw_names = {'rustfmt' + s for s in ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
    raw_names |= {'sources-preformat.json', 'sources-after.json', 'dependencies-after.json'}
    require(set(c['raw']) == raw_names and len(raw_names) == 8, 'closed failed raw directory')
    expected = {'evidence/' + n: compact(r) for n, r in c['raw'].items()}
    for name, row in c['raw'].items():
        require(row['path'] == ROOT + '/evidence/' + name, 'owned raw path')
    expected['evidence/failed.json'] = pin(bodies['evidence/failed.json'])
    expected['input-manifest.json'] = compact(c['input_manifest'])
    for role, (name, _) in LINEAGE.items():
        expected['inputs/' + name] = compact(c['readset'][role])
    for name in set(overlay) | {'run_cpu.py', 'supervisor.py'}:
        expected[name] = final[name]
    expected['cargo-cache-manifest.json'] = compact(c['cache_provenance']['manifest'])
    expected['cargo-cache-stage-complete.json'] = compact(c['cache_provenance']['stage'])
    expected['stage-complete.json'] = pin(bodies['stage-complete.json'])
    for name, (_, fixed_pin) in STAGERS.items():
        exact(bodies[name], fixed_pin)
        expected[name] = pin(bodies[name])
    expected['evidence.py'] = pin(bodies['evidence.py'])
    require(set(expected) == set(bodies) and len(expected) == MEMBERS, 'exact36 original selected bodies')
    for name, row in expected.items():
        require(pin(bodies[name]) == row, 'original selected body pin: ' + name)
    cache = parse(bodies['cargo-cache-manifest.json'])
    stage = parse(bodies['cargo-cache-stage-complete.json'])
    require(stage['passed'] is True and stage['shared_cache_changed'] is False and stage['lock_changed'] is False
            and stage['controller'] == pin(bodies['cache-transport.py'])
            and stage['files'] == cache['files'] ==
                {n: compact(r) for n, r in c['cache_provenance']['files'].items()}
            and stage['locks'] == cache['locks'] == c['cache_provenance']['locks']
            and cache['locked_packages'] == c['cache_provenance']['packages']
            and len(cache['files']) == 73 and len(cache['locked_packages']) == stage['packages'] == 39,
            'original private immutable cache')
    staged = parse(bodies['stage-complete.json'])
    require(staged['passed'] is True and staged['root'] == ROOT and staged['source_files'] == 1012
            and staged['runtime_files'] == 815 and staged['worker_files'] == 195 and staged['overlay_files'] == 11
            and staged['input'] == pin(bodies['input-manifest.json']) and staged['archive'] == SOURCE_ARCHIVE
            and staged['controller'] == pin(bodies['source-transport.py'])
            and all(staged[k] is False for k in ('canonical_changed', 'lockfiles_changed',
                                               'shared_cache_changed', 'project_execution')),
            'actual original source stage')
    return expected, c, changed


def paths_below(root):
    require(root.resolve(strict=True) == root and root.is_dir(), 'canonical tree root')
    found = []
    for directory, dirs, files in os.walk(root, followlinks=False):
        require(len(found) <= 10000, 'bounded file tree')
        for name in dirs:
            require(stat.S_ISDIR((Path(directory) / name).lstat().st_mode), 'no directory links')
        for name in files:
            path = Path(directory) / name
            require(stat.S_ISREG(path.lstat().st_mode), 'no special or linked input')
            found.append(path)
    return found

def stream_pin(path):
    require(path.resolve(strict=True) == path, 'canonical live input')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= 512 << 20, 'bounded live input')
        digest, size = hashlib.sha256(), 0
        while True:
            block = stream.read(1 << 20)
            if not block:
                break
            size += len(block); digest.update(block)
            require(size <= before.st_size, 'live input grew')
        require(stamp(os.fstat(stream.fileno())) == stamp(before) == stamp(path.lstat())
                and size == before.st_size, 'live input changed')
    return dict(bytes=size, sha256=digest.hexdigest())

def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()

def live_check(c):
    root = Path(ROOT)
    actual = {str(p.relative_to(root)) for scope in ('fe2o3', 'ferric') for p in paths_below(root / scope)}
    require(actual | {'run_cpu.py', 'supervisor.py'} == set(c['final_sources']), 'complete live source roster')
    rows = {}
    def add(row):
        path = row['path']
        require(path not in rows or rows[path] == compact(row), 'conflicting live pin')
        rows[path] = compact(row)
    for name, row in c['final_sources'].items():
        require(row['path'] == ROOT + '/' + name, 'owned source path')
        add(row)
    for row in list(c['readset'].values()) + list(c['tool_pins'].values()) + list(c['cache_provenance']['files'].values()):
        add(row)
    require(len(rows) <= 1200 and sum(r['bytes'] for r in rows.values()) <= 2 << 30, 'bounded source/tool/cache readset')
    for path, row in rows.items():
        require(stream_pin(Path(path)) == row, 'live original input drift: ' + path)
    for path, row in c['configurations'].items():
        require((stream_pin(Path(path)) if os.path.lexists(path) else None) ==
                (compact(row) if row is not None else None), 'Cargo configuration drift')
    return len(rows)


def export(terminal_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'original CPU host')
    root, output = Path(ROOT), E / BASENAME
    require(root.resolve(strict=True) == root and E.resolve(strict=True) == E
            and not os.path.lexists(output) and os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 1 << 30,
            'fresh bounded archive destination')
    terminal = read(root / 'evidence/failed.json', MAX_FILE)
    exact(terminal, TERMINAL_PIN)
    c = parse(terminal)
    require({str(p.relative_to(root / 'evidence')) for p in paths_below(root / 'evidence')} ==
            set(c['raw']) | {'failed.json'}, 'all failed raw files with no extra success receipt')
    require({str(p) for p in paths_below(root / 'inputs')} == {r['path'] for r in c['readset'].values()},
            'complete seven-body lineage tree')
    names = {'evidence/failed.json', 'run_cpu.py', 'supervisor.py', 'input-manifest.json',
             'stage-complete.json', 'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json'}
    names |= {'evidence/' + n for n in c['raw']}
    names |= {'inputs/' + name for name, _ in LINEAGE.values()}
    names |= set(parse(read(root / 'input-manifest.json', MAX_FILE))['worker_overlay'])
    bodies = {}
    for name in names:
        path = PurePosixPath(name)
        require(str(path) == name and not path.is_absolute() and '..' not in path.parts, 'closed selected path')
        bodies[name] = read(root / name, MAX_FILE)
    for name, (path, expected) in STAGERS.items():
        bodies[name] = read(path, MAX_FILE)
        exact(bodies[name], expected)
    bodies['evidence.py'] = read(Path(__file__).resolve(), MAX_FILE)
    expected, c, changed = validate(bodies, terminal_sha)
    count = live_check(c)
    manifest = dict(schema='ferric-warm-paired-terminal-worker-cpu-failed-export-v1',
        files=expected, terminal=pin(terminal), passed=False, current_tests_executed=False,
        raw_files=8, lineage_files=7, source_rows=1012, final_overlay_files=11,
        observed_partial_formatter_changes=changed, original_format_changed_paths=c['format_changed_paths'],
        selected_files=MEMBERS, live_input_count=count, live_source_tool_cache_postcheck=True,
        binary_bodies_retained=False, receipt_rewritten=False, new_project_execution=False,
        native_execution=False, numerical_acceptance=False, performance_claim=False)
    archive_bodies = dict(bodies, **{'manifest.json': encoded(manifest)})
    require(sum(map(len, archive_bodies.values())) <= MAX_TOTAL, 'expanded body bound')
    with output.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, body in sorted(archive_bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(body), 0o644, 0
                tar.addfile(info, io.BytesIO(body))
        stream.flush(); os.fsync(stream.fileno())
    require(live_check(c) == count, 'complete live posthash census')
    for name, body in bodies.items():
        path = Path(__file__).resolve() if name == 'evidence.py' else STAGERS[name][0] if name in STAGERS else root / name
        require(read(path, MAX_FILE) == body, 'original selected body posthash')
    print(json.dumps(dict(archive=dict(path=str(output), **stream_pin(output)), members=MEMBERS + 1,
                         expanded_bytes=sum(map(len, archive_bodies.values())), passed=False)))


def retain(archive_sha, terminal_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha) is not None, 'observed archive SHA')
    require(DEST.parent.resolve(strict=True) == DEST.parent and not os.path.lexists(DEST), 'fresh failed capsule')
    archive = W / BASENAME
    raw = read(archive, MAX_TOTAL)
    require(pin(raw)['sha256'] == archive_sha, 'exact original archive')
    bodies, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        for member in tar:
            path = PurePosixPath(member.name)
            require(member.isfile() and not member.pax_headers and str(path) == member.name
                    and not path.is_absolute() and '..' not in path.parts and '\\' not in member.name
                    and member.name not in bodies and len(bodies) < MEMBERS + 1
                    and 0 <= member.size <= MAX_FILE, 'closed regular archive member')
            total += member.size
            require(total <= MAX_TOTAL, 'expanded archive cap')
            body = tar.extractfile(member).read(member.size + 1)
            require(len(body) == member.size, 'complete archive body')
            bodies[member.name] = body
    require(len(bodies) == MEMBERS + 1, 'exact37 archive bodies')
    manifest_raw = bodies.pop('manifest.json')
    m = parse(manifest_raw)
    require(bodies['evidence.py'] == read(Path(__file__).resolve(), MAX_FILE), 'same reviewed exporter/retainer')
    expected, c, changed = validate(bodies, terminal_sha)
    require(m['schema'] == 'ferric-warm-paired-terminal-worker-cpu-failed-export-v1'
            and m['files'] == expected and m['terminal'] == pin(bodies['evidence/failed.json'])
            and m['selected_files'] == MEMBERS and m['raw_files'] == 8 and m['lineage_files'] == 7
            and m['source_rows'] == 1012 and m['final_overlay_files'] == 11
            and m['observed_partial_formatter_changes'] == changed
            and m['original_format_changed_paths'] == [] and m['live_source_tool_cache_postcheck'] is True
            and all(m[k] is False for k in ('passed', 'current_tests_executed', 'binary_bodies_retained',
                  'receipt_rewritten', 'new_project_execution', 'native_execution', 'numerical_acceptance',
                  'performance_claim')), 'failure remains failure with original partial source changes')
    require(read(archive, MAX_TOTAL) == raw, 'archive prepublication posthash')
    bodies['manifest.json'] = manifest_raw
    DEST.mkdir(mode=0o755)
    for name, body in sorted(bodies.items()):
        path = DEST / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
    require(all(read(DEST / name, MAX_FILE) == body for name, body in bodies.items())
            and read(archive, MAX_TOTAL) == raw, 'all original failure bodies retained unchanged')
    report = dict(schema='ferric-warm-paired-terminal-worker-cpu-failed-retention-v1',
        archive=pin(raw), files={n: pin(b) for n, b in sorted(bodies.items())}, original_files=MEMBERS + 1,
        terminal=pin(bodies['evidence/failed.json']), passed=False, current_test_count=0,
        observed_partial_formatter_changes=changed, original_receipt_unchanged=True,
        external_inputs_rehashed_locally=False, canonical_source_changed=False,
        new_project_execution=False, native_execution=False, numerical_acceptance=False, performance_claim=False)
    with (DEST / 'retention.json').open('xb') as stream:
        stream.write(encoded(report)); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(destination=str(DEST), original_files=MEMBERS + 1, passed=False)))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B evidence_failed_v1.py export TERMINAL_SHA | retain ARCHIVE_SHA TERMINAL_SHA')
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_TOTAL, MAX_TOTAL))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    def interrupted(number, _frame):
        raise RuntimeError('evidence signal ' + str(number))
    for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.alarm(180)
    try:
        require(len(sys.argv) in (3, 4) and re.fullmatch('[0-9a-f]{64}', sys.argv[2]) is not None, 'observed SHA argument')
        if sys.argv[1] == 'export' and len(sys.argv) == 3:
            export(sys.argv[2])
        else:
            require(sys.argv[1] == 'retain' and len(sys.argv) == 4, 'closed failure-only CLI')
            retain(sys.argv[2], sys.argv[3])
    finally:
        signal.alarm(0)
