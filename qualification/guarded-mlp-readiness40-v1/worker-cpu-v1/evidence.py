"""Bounded success-only CPU evidence export/retention; no project execution."""
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
DEST = F / 'qualification/guarded-mlp-readiness40-v1/worker-cpu-v1'
BASENAME = 'guarded-mlp-readiness-cli-worker-cpu-evidence-v228-v1.tar.gz'
TERMINAL_PIN = (1675803, '2b785c1b8384358f11d547be93614660171c94108aba5e50cdb8db7e4b462b77')
CONTROLLER_PIN = (37922, '640f8c694d3b4cbd1795e060916d548dfbc82b1fc944713637bd36dfad5ac6ce')
INPUT_PIN = (213680, 'fb83c65b615f0c5f3ddfaac83341c8052181f7ee87d87cb6812e779ff152e95d')
SOURCE_ARCHIVE = dict(bytes=442642, sha256='bff8785146d072134a2346b7ecb3f6ab5fe69fb71073806b88a0e5c36e6909d1')
ROOT = str(E / 'guarded-mlp-readiness-cli-worker-cpu-v228-v1')
PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
STAGERS = {
    'source-transport.py': (E / 'readiness-worker-transport-v1.py',
        (16924, '24e78564354805a994878def8d01928673c4cf7616baf814e53e2a5adef798c3')),
    'cache-transport.py': (E / 'readiness-worker-cache-stage-v1.py',
        (12798, 'c54bcff919611c3bf931d1706aa63dd7c53141eaf12292362e13fa975ae5949e')),
}
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'metadata', 'worker-tests-build',
          'worker-list', 'worker-ignored', 'worker-tests', 'worker-build')
MEMBERS = 84
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


def validate(bodies):
    exact(bodies['evidence/complete.json'], TERMINAL_PIN)
    exact(bodies['run_cpu.py'], CONTROLLER_PIN)
    exact(bodies['input-manifest.json'], INPUT_PIN)
    c = parse(bodies['evidence/complete.json'])
    i = parse(bodies['input-manifest.json'])
    require(c['schema'] == 'ferric-guarded-mlp-readiness-cli-worker-cpu-v1'
            and c['passed'] is True and c['failure'] is None and c['postcheck_errors'] == [], 'actual successful CPU terminal')
    require(c['source_unchanged'] is True and c['input_sources'] == c['final_sources']
            and c['cache_inputs_unchanged'] is True, 'actual postchecks')
    require(c['source_lineage'] == c['readset'] == i['source_lineage'] and len(c['readset']) == 8,
            'all eight original lineage bodies')
    require(c['paired_read_cli_dispatch_fixed'] is True and c['executable_cli_regression_added'] is True
            and c['cli_executable_unchanged_across_tests'] is True
            and c['inherited_paired_read_worker_route_preserved'] is True
            and c['full_worker_tests_executed'] is True, 'real executable regression admission')
    for key in ('gpu_execution', 'paired_read_native_execution', 'runtime_suite_rerun',
                'readiness40_native_execution', 'full2303_native_enabled',
                'existing_ar4_limits_changed', 'method_local_currentness_cadence_changed',
                'global_currentness_policy_changed', 'performance_policy_changed', 'default_policy_changed',
                'full_long_workload', 'whole_model_guarded_execution', 'numerical_acceptance',
                'performance_claim', 'production_authority'):
        require(c[key] is False, 'CPU-only scope: ' + key)
    require(all(c[key] is True for key in ('pure_long_sequence_source_added',
            'readiness_owner_source_added', 'readiness_cli_source_added',
            'inherited_paired_cli_regressions_preserved')), 'qualified composed readiness source')
    proposal = parse(bodies['inputs/composed-source-manifest.json'])
    rows = {'ferric/' + row['path']: row for row in proposal['files'] if row['path'].startswith(
            'adapters/tp-peer-finite-engineering-worker-v1/')}
    overlay = set(rows)
    require(len(rows) == 16 and overlay == set(i['worker_overlay']), 'closed sixteen worker overlay rows')
    additions = set(proposal['new_tests']['worker_library']) | set(proposal['new_tests']['worker_readiness_cli_integration'])
    require(len(proposal['new_tests']['worker_library']) == 34
            and len(proposal['new_tests']['worker_readiness_cli_integration']) == 1
            and len(additions) == 35, 'declared target-specific additive tests')
    require(tuple(row['label'] for row in c['phases']) == PHASES, 'exact nine completed phases')
    raw_names = {label + suffix for label in PHASES for suffix in
                 ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
    raw_names |= {'sources-preformat.json', 'sources-before.json', 'sources-after.json',
                  'dependencies-before.json', 'dependencies-after.json'}
    require(set(c['raw']) == raw_names and len(raw_names) == 50, 'closed original raw roster')
    expected = {'evidence/' + name: compact(row) for name, row in c['raw'].items()}
    expected['evidence/complete.json'] = pin(bodies['evidence/complete.json'])
    expected['input-manifest.json'] = compact(c['input_manifest'])
    for row in c['readset'].values():
        require(row['path'].startswith(ROOT + '/inputs/'), 'lineage namespace')
        expected[row['path'][len(ROOT) + 1:]] = compact(row)
    for name in overlay | {'run_cpu.py', 'supervisor.py'}:
        expected[name] = compact(c['final_sources'][name])
    expected['cargo-cache-manifest.json'] = compact(c['cache_provenance']['manifest'])
    expected['cargo-cache-stage-complete.json'] = compact(c['cache_provenance']['stage'])
    expected['stage-complete.json'] = pin(bodies['stage-complete.json'])
    for name, (_, fixed_pin) in STAGERS.items():
        exact(bodies[name], fixed_pin)
        expected[name] = pin(bodies[name])
    expected['evidence.py'] = pin(bodies['evidence.py'])
    require(set(expected) == set(bodies) and len(expected) == MEMBERS, 'exact 84 retained bodies')
    for name, row in expected.items():
        require(pin(bodies[name]) == row, 'retained body pin: ' + name)
    for name, row in c['raw'].items():
        require(row['path'] == ROOT + '/evidence/' + name, 'raw namespace')
    for phase in c['phases']:
        label = phase['label']
        require(phase['exit_code'] == 0 and phase['natural_exit'] is True and phase['reaped'] is True
                and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
                and phase['timed_out'] is False and phase['exception'] is None
                and phase['storage_failure'] is None and phase['observed_signals'] == [], 'clean natural leaf')
        require(parse(bodies['evidence/' + label + '.result.json']) == phase, 'original result join')
        require(parse(bodies['evidence/' + label + '.started.json']) ==
                dict(pid=phase['pid'], pgid=phase['pgid'], argv=phase['argv']), 'original process registration')
        for key, suffix in (('command', '.command.json'), ('stdout', '.stdout'), ('stderr', '.stderr')):
            require(phase[key] == c['raw'][label + suffix], 'original phase stream join')
    for key, filename in (('preformat_sources', 'sources-preformat.json'),
                          ('input_sources', 'sources-before.json'), ('final_sources', 'sources-after.json')):
        require(parse(bodies['evidence/' + filename]) == c[key], 'complete source map join')
    pre = {name: compact(row) for name, row in c['preformat_sources'].items()}
    final = {name: compact(row) for name, row in c['final_sources'].items()}
    require(pre == i['files'] and len(pre) == len(final) == 1010, 'exact full preformat source map')
    require(sum(name.startswith('fe2o3/') for name in final) == 813
            and sum(name.startswith(PREFIX) for name in final) == 195, 'runtime/worker source census')
    changed = {name for name in pre if pre[name] != final[name]}
    require(changed == set(c['format_changed_paths']) == overlay and len(c['format_changed_paths']) == 16,
            'only sixteen observed authorized formatter transitions')
    old = parse(bodies['inputs/worker-complete.json'])
    old_map = parse(bodies['inputs/worker-sources.json'])
    require(old_map == old['final_sources'] and compact(proposal['base']['worker_complete']) ==
            pin(bodies['inputs/worker-complete.json']), 'actual predecessor receipt and final map')
    require(len(old_map) == 999 and set(final) - set(old_map) == {n for n in overlay if rows[n]['before'] is None}
            and set(old_map) <= set(final), 'eleven additive source files only')
    require(sum(rows[n]['before'] is None for n in overlay) == 11, 'five replacements and eleven additions')
    for name in pre:
        if name in rows:
            before = compact(old_map[name]) if name in old_map else None
            require(rows[name]['before'] == before and rows[name]['after'] == pre[name], 'composed source pre/postimage')
        elif name not in {'run_cpu.py', 'supervisor.py'}:
            require(compact(old_map[name]) == pre[name] == final[name], 'unchanged inherited source')
    for layer in proposal['layers']:
        label = {'pure-long': 'pure-long', 'native-owner': 'native-owner', 'readiness-cli': 'readiness-cli'}[layer['name']]
        require(pin(bodies['inputs/' + label + '-source-manifest.json']) == compact(layer['manifest']),
                'retained original composition manifest')
    tests = c['tests']['worker-tests']
    observed = named(bodies['evidence/worker-tests.stdout'])
    require(observed == {row['name']: row['outcome'] for row in tests['named']}
            and len(tests['named']) == len(observed) == 662, 'all current named raw outcomes')
    prior = named(bodies['inputs/worker-tests.stdout'])
    require(prior == {row['name']: row['outcome'] for row in old['tests']['worker-tests']['named']}
            and len(prior) == 627 and all(observed[name] == outcome for name, outcome in prior.items())
            and set(observed) - set(prior) == additions and all(observed[name] == 'ok' for name in additions),
            'all old outcomes preserved and exactly 35 declared additions')
    require({key: tests[key] for key in ('passed', 'failed', 'ignored')} == dict(passed=658, failed=0, ignored=4),
            'actual complete suite totals')
    summary = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',
                         bodies['evidence/worker-tests.stdout'].decode(), re.M)
    require(summary == [('642', '0', '4', '0', '0'), ('0', '0', '0', '0', '0'), ('1', '0', '0', '0', '0'), ('15', '0', '0', '0', '0')],
            'actual library/binary/integration summaries')
    inventory = re.findall(r'^(\S+): test$', bodies['evidence/worker-list.stdout'].decode(), re.M)
    require(len(inventory) == len(set(inventory)) == 662 and sorted(inventory) == c['inventory']
            and set(inventory) == set(observed), 'actual complete inventory')
    require(bodies['evidence/dependencies-before.json'] == bodies['evidence/dependencies-after.json'],
            'recorded dependency hashes unchanged')
    cargo = {label: [parse(line) for line in bodies['evidence/' + label + '.stdout'].splitlines() if line]
             for label in ('worker-tests-build', 'worker-build')}
    require(set(c['artifacts']) == {'worker', 'worker-lib', 'worker-bin-test', 'worker-wire-test', 'worker-readiness-test'}, 'five product roles')
    for role, artifact in c['artifacts'].items():
        label = 'worker-build' if role == 'worker' else 'worker-tests-build'
        require(artifact['cargo_artifact'] in cargo[label]
                and artifact['cargo_artifact']['executable'] == artifact['pin']['path'], 'original Cargo product join')
    require(c['cli_executable_before_tests']['cargo_artifact'] in cargo['worker-tests-build']
            and c['cli_executable_before_tests']['pin'] == c['artifacts']['worker']['pin'], 'same tested real worker ELF')
    cache = parse(bodies['cargo-cache-manifest.json'])
    stage = parse(bodies['cargo-cache-stage-complete.json'])
    require(stage['passed'] is True and stage['shared_cache_changed'] is False and stage['lock_changed'] is False
            and stage['controller'] == pin(bodies['cache-transport.py'])
            and stage['files'] == cache['files'] ==
                {name: compact(row) for name, row in c['cache_provenance']['files'].items()}
            and stage['locks'] == cache['locks'] == c['cache_provenance']['locks']
            and cache['locked_packages'] == c['cache_provenance']['packages']
            and len(cache['files']) == 73 and len(cache['locked_packages']) == stage['packages'] == 39,
            'private immutable two-lock cache')
    staged = parse(bodies['stage-complete.json'])
    require(staged['passed'] is True and staged['root'] == ROOT and staged['source_files'] == 1010
            and staged['runtime_files'] == 813 and staged['worker_files'] == 195
            and staged['overlay_files'] == 16 and staged['input'] == pin(bodies['input-manifest.json'])
            and staged['archive'] == SOURCE_ARCHIVE
            and staged['controller'] == pin(bodies['source-transport.py'])
            and all(staged[key] is False for key in ('canonical_changed', 'lockfiles_changed',
                                                     'shared_cache_changed', 'project_execution')),
            'actual source staging receipt')
    return expected, c


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


def live_check(c, bodies):
    root = Path(ROOT)
    actual = {str(path.relative_to(root)) for scope in ('fe2o3', 'ferric') for path in paths_below(root / scope)}
    require(actual | {'run_cpu.py', 'supervisor.py'} == set(c['final_sources']), 'full live source closure')
    rows = {}
    def add(path, row):
        path = str(path)
        require(path not in rows or rows[path] == compact(row), 'conflicting live pin')
        rows[path] = compact(row)
    for name, row in c['final_sources'].items():
        require(row['path'] == ROOT + '/' + name, 'source namespace')
        add(row['path'], row)
    for row in list(c['readset'].values()) + list(c['tool_pins'].values()) + list(c['cache_provenance']['files'].values()):
        add(row['path'], row)
    for artifact in c['artifacts'].values():
        require(artifact['pin']['path'].startswith(ROOT + '/target/debug/'), 'owned binary path')
        add(artifact['pin']['path'], artifact['pin'])
    dependencies = parse(bodies['evidence/dependencies-after.json'])
    require(len(dependencies) == 29, 'resolved worker registry package roots')
    for directory, expected in dependencies.items():
        require(directory.startswith(ROOT + '/cargo-home/registry/src/'), 'private resolved dependency root')
        require({str(p.relative_to(directory)) for p in paths_below(Path(directory))} == set(expected),
                'resolved dependency source closure')
        for relative, row in expected.items():
            require(row['path'] == directory + '/' + relative, 'dependency namespace')
            add(row['path'], row)
    require(len(rows) <= 10000 and sum(r['bytes'] for r in rows.values()) <= 2 << 30, 'bounded live rehash set')
    for path, expected in rows.items():
        require(stream_pin(Path(path)) == expected, 'live pin drift: ' + path)
    for path, expected in c['configurations'].items():
        require((stream_pin(Path(path)) if os.path.lexists(path) else None) ==
                (compact(expected) if expected is not None else None), 'Cargo configuration drift')
    return len(rows)


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()


def export():
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'actual CPU host')
    root = Path(ROOT)
    require(root.resolve(strict=True) == root and E.resolve(strict=True) == E, 'closed source root')
    output = E / BASENAME
    require(not os.path.lexists(output) and os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 1 << 30,
            'fresh archive and space reserve')
    terminal_raw = read(root / 'evidence/complete.json', MAX_FILE)
    exact(terminal_raw, TERMINAL_PIN)
    c = parse(terminal_raw)
    require({str(p.relative_to(root / 'evidence')) for p in paths_below(root / 'evidence')} ==
            set(c['raw']) | {'complete.json'}, 'closed actual raw directory')
    bodies = {'evidence/complete.json': terminal_raw}
    for name, row in c['raw'].items():
        require('/' not in name and row['path'] == ROOT + '/evidence/' + name, 'closed raw path')
        bodies['evidence/' + name] = read(root / 'evidence' / name, MAX_FILE)
    require({str(p) for p in paths_below(root / 'inputs')} == {r['path'] for r in c['readset'].values()},
            'closed lineage directory')
    for row in c['readset'].values():
        require(row['path'].startswith(ROOT + '/inputs/'), 'owned lineage path')
        bodies[row['path'][len(ROOT) + 1:]] = read(Path(row['path']), MAX_FILE)
    inputs = parse(read(root / 'input-manifest.json', MAX_FILE))
    for name in set(inputs['worker_overlay']) | {'run_cpu.py', 'supervisor.py', 'input-manifest.json',
            'stage-complete.json', 'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json'}:
        path = PurePosixPath(name)
        require(str(path) == name and not path.is_absolute() and '..' not in path.parts, 'closed selected path')
        bodies[name] = read(root / name, MAX_FILE)
    for name, (path, fixed_pin) in STAGERS.items():
        bodies[name] = read(path, MAX_FILE)
        exact(bodies[name], fixed_pin)
    bodies['evidence.py'] = read(Path(__file__).resolve(), MAX_FILE)
    expected, c = validate(bodies)
    live_count = live_check(c, bodies)
    require(sum(map(len, bodies.values())) <= MAX_TOTAL, 'bounded selected evidence')
    manifest = dict(schema='ferric-readiness-worker-cpu-export-v1', files=expected,
        terminal=pin(terminal_raw), selected_files=MEMBERS, raw_files=50, source_rows=1010,
        formatted_overlay_files=16, lineage_files=8, binary_bodies_retained=False,
        live_sources_cache_dependencies_tools_and_products_rehashed=True, live_input_count=live_count,
        actual_tests=dict(passed=658, failed=0, ignored=4), native_execution=False,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False)
    archive_bodies = dict(bodies, **{'manifest.json': encoded(manifest)})
    with output.open('xb') as stream:
        with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, body in sorted(archive_bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(body), 0o644, 0
                tar.addfile(info, io.BytesIO(body))
        stream.flush(); os.fsync(stream.fileno())
    require(live_check(c, bodies) == live_count, 'complete live posthash census')
    for name, body in bodies.items():
        path = Path(__file__).resolve() if name == 'evidence.py' else STAGERS[name][0] if name in STAGERS else root / name
        require(read(path, MAX_FILE) == body, 'selected evidence posthash')
    print(json.dumps(dict(archive=dict(path=str(output), **stream_pin(output)), members=MEMBERS + 1,
        selected_files=MEMBERS, raw_files=50, expanded_bytes=sum(map(len, archive_bodies.values()))), sort_keys=True))


def retain(archive_sha):
    require(re.fullmatch('[0-9a-f]{64}', archive_sha) is not None, 'observed archive SHA required')
    require(DEST.parent.resolve(strict=True) == DEST.parent and not os.path.lexists(DEST), 'fresh retention destination')
    archive = W / BASENAME
    raw = read(archive, MAX_TOTAL)
    require(pin(raw)['sha256'] == archive_sha, 'actual archive pin')
    bodies, total = {}, 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as tar:
        for member in tar:
            path = PurePosixPath(member.name)
            require(member.isfile() and not member.pax_headers and str(path) == member.name
                    and not path.is_absolute() and '..' not in path.parts and '\\' not in member.name
                    and member.name not in bodies and len(bodies) < MEMBERS + 1
                    and 0 <= member.size <= MAX_FILE, 'closed regular archive member')
            total += member.size
            require(total <= MAX_TOTAL, 'expanded archive bound')
            body = tar.extractfile(member).read(member.size + 1)
            require(len(body) == member.size, 'complete retained member')
            bodies[member.name] = body
    require(len(bodies) == MEMBERS + 1, 'exact85-member evidence archive')
    manifest_raw = bodies.pop('manifest.json')
    manifest = parse(manifest_raw)
    require(bodies['evidence.py'] == read(Path(__file__).resolve(), MAX_FILE), 'same reviewed export/retain source')
    expected, terminal = validate(bodies)
    require(manifest['schema'] == 'ferric-readiness-worker-cpu-export-v1' and manifest['files'] == expected
            and manifest['terminal'] == pin(bodies['evidence/complete.json'])
            and manifest['selected_files'] == MEMBERS and manifest['formatted_overlay_files'] == 16
            and manifest['lineage_files'] == 8 and manifest['source_rows'] == 1010
            and manifest['raw_files'] == 50 and manifest['actual_tests'] == dict(passed=658, failed=0, ignored=4)
            and manifest['live_sources_cache_dependencies_tools_and_products_rehashed'] is True
            and all(manifest[k] is False for k in ('binary_bodies_retained', 'native_execution',
                                                  'full_long_workload', 'numerical_acceptance', 'performance_claim')),
            'closed CPU export observations')
    require(read(archive, MAX_TOTAL) == raw, 'archive prepublication posthash')
    bodies['manifest.json'] = manifest_raw
    DEST.mkdir(mode=0o755)
    for name, body in sorted(bodies.items()):
        path = DEST / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
        path.chmod(0o644)
    require(all(read(DEST / name, MAX_FILE) == body for name, body in bodies.items())
            and read(archive, MAX_TOTAL) == raw, 'all original evidence retained unchanged')
    report = dict(schema='ferric-readiness-worker-cpu-retention-v1', archive=pin(raw),
        files={n: pin(b) for n, b in sorted(bodies.items())}, original_files=MEMBERS + 1,
        terminal=pin(bodies['evidence/complete.json']), formatted_overlay_files=16,
        artifacts=terminal['artifacts'], actual_tests=dict(passed=658, failed=0, ignored=4),
        binary_bodies_retained=False, external_inputs_rehashed_locally=False,
        original_terminal_unchanged=True, new_project_execution=False, new_native_execution=False,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False)
    with (DEST / 'retention.json').open('xb') as stream:
        stream.write(encoded(report)); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(destination=str(DEST), original_files=MEMBERS + 1, tests_passed=658)))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B evidence.py export | retain ARCHIVE_SHA')
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_TOTAL, MAX_TOTAL))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    def interrupted(number, _frame):
        raise RuntimeError('evidence helper signal ' + str(number))
    for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.alarm(180)
    try:
        if sys.argv[1:] == ['export']:
            export()
        else:
            require(len(sys.argv) == 3 and sys.argv[1] == 'retain', 'closed data-only helper CLI')
            retain(sys.argv[2])
    finally:
        signal.alarm(0)
