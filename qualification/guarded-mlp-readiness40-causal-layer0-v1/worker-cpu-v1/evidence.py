"""Bounded CPU success/clean-failure evidence export/retention; no project execution."""
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
DEST = F / 'qualification/guarded-mlp-readiness40-causal-layer0-v1/worker-cpu-v1'
BASENAME = 'guarded-mlp-readiness40-causal-layer0-worker-cpu-evidence-v228-v1.tar.gz'
CONTROLLER_PIN = (39748, '111d2d6fc18d410592867fec04dc4483ddbd36f55172ef23f724f9245edd3168')
INPUT_PIN = (210246, '68c42d5adb751ade3a646a60eb2e2b148a062de59bcc298ee77fdd0286310a8d')
SOURCE_ARCHIVE = dict(bytes=803944, sha256='6551f06cff97f7190edb423c2ab366d84de9e1745ee026d3c662333d5de98dd5')
ROOT = str(E / 'guarded-mlp-readiness40-causal-layer0-worker-cpu-v228-v1')
PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
STAGERS = {
    'source-transport.py': (E / 'causal-layer0-worker-transport-v1.py',
        (18737, '35e14b509d859c0f5c3cf8ab8c775d431ad1a44ac1399608b5473378a28d17bb')),
    'cache-transport.py': (E / 'causal-layer0-worker-cache-v1.py',
        (12811, 'a1435ce944887cac9aff4fbc20a2e87103bf5297d95b8b38477fab75c78a8c6d')),
}
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'metadata', 'worker-tests-build',
          'worker-list', 'worker-ignored', 'worker-tests', 'worker-build')
MEMBERS = 76
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
    rows = re.findall(r'^test (\S+) \.\.\. (ok|ignored|FAILED)(?:, [^\n]*)?$', body.decode(), re.M)
    require(len(rows) == len({name for name, _ in rows}), 'duplicate named test')
    return dict(rows)


def validate(bodies, terminal_sha):
    terminal_names = set(bodies) & {'evidence/complete.json', 'evidence/failed.json'}
    require(len(terminal_names) == 1, 'exact one original terminal')
    terminal_name = next(iter(terminal_names))
    require(re.fullmatch('[0-9a-f]{64}', terminal_sha) is not None
            and pin(bodies[terminal_name])['sha256'] == terminal_sha, 'observed terminal SHA')
    exact(bodies['run_cpu.py'], CONTROLLER_PIN)
    exact(bodies['input-manifest.json'], INPUT_PIN)
    c = parse(bodies[terminal_name])
    i = parse(bodies['input-manifest.json'])
    require(c['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-worker-cpu-v1'
            and type(c['passed']) is bool and c['postcheck_errors'] == [], 'actual clean CPU terminal')
    success = c['passed']
    require(terminal_name == ('evidence/complete.json' if success else 'evidence/failed.json')
            and ((c['failure'] is None) if success else
                 (isinstance(c['failure'], str) and bool(c['failure']))), 'original outcome remains unchanged')
    require(c['cache_inputs_unchanged'] is True, 'actual cache postcheck')
    if c['input_sources'] is not None:
        require(c['source_unchanged'] is True and c['input_sources'] == c['final_sources'],
                'actual postformat source postcheck')
    else:
        require(not success and c['source_unchanged'] is False, 'failed preformat-only attempt')
    require(c['source_lineage'] == c['readset'] == i['source_lineage'] and len(c['readset']) == 7,
            'all seven original lineage bodies')
    require(c['paired_read_cli_dispatch_fixed'] is True and c['executable_cli_regression_added'] is True
            and c['inherited_paired_read_worker_route_preserved'] is True,
            'real executable regression source admission')
    for key in ('gpu_execution', 'causal_layer0_native_execution', 'paired_read_native_execution', 'runtime_suite_rerun',
                'readiness40_native_execution', 'position5_native_execution', 'capture_count_changed', 'full2303_native_enabled',
                'existing_ar4_limits_changed', 'hidden_read_policy_changed',
                'warm_paired_terminal_native_execution', 'runtime_source_qualified_by_paired_read_cpu',
                'global_currentness_policy_changed', 'performance_policy_changed', 'default_policy_changed',
                'full_long_workload', 'whole_model_guarded_execution', 'numerical_acceptance',
                'performance_claim', 'production_authority'):
        require(c[key] is False, 'CPU-only scope: ' + key)
    require(all(c[key] is True for key in ('pure_long_sequence_source_added',
            'readiness_owner_source_added', 'readiness_cli_source_added',
            'inherited_paired_cli_regressions_preserved', 'position5_diagnostic_source_added',
            'inherited_readiness40_profile_preserved', 'warm_paired_terminal_source_added',
            'selected_terminal_cadence_changed', 'method_local_currentness_cadence_changed',
            'runtime_source_qualified_by_terminal_cpu', 'causal_layer0_source_added',
            'inherited_warm_paired_terminal_route_preserved')), 'qualified causal source scope')
    require(c['causal_capture_positions'] == list(range(6)) and c['causal_layer'] == 0, 'closed causal selection')
    proposal = parse(bodies['inputs/causal-source-manifest.json'])
    exact(bodies['inputs/causal-source-manifest.json'],
          (8279, '9cce663d74160516c14033e8417c54dbce666b25e61dc232c11b494c055f558c'))
    require(proposal['schema'] == 'ferric-readiness40-causal-layer0-source-proposal-v1',
            'closed causal source proposal')
    rows = {'ferric/' + row['path']: row for row in proposal['files'] if row['path'].startswith(
            'adapters/tp-peer-finite-engineering-worker-v1/')}
    overlay = set(rows)
    require(len(rows) == 9 and overlay == set(i['worker_overlay'])
            and sum(row['before'] is None for row in rows.values()) == 2, 'seven replacements/two additions')
    library = ['native_guarded_mlp_readiness_cli_v1::causal_selector_tests::' + name
               if name.startswith('causal_selector_') else
               'resident_layer::capture_v1::causal_v1::tests::' + name
               for name in proposal['new_tests']['worker_library']]
    readiness = proposal['new_tests']['worker_readiness_binary']
    additions = set(library + readiness)
    require(len(library) == 6 and len(readiness) == 1 and len(additions) == 7
            and i['new_tests'] == {'worker-lib': library, 'worker-bin-test': [],
                                  'worker-wire-test': [], 'worker-readiness-test': readiness},
            'exact target-specific seven source additions')
    labels = tuple(row['label'] for row in c['phases'])
    require(0 < len(labels) <= len(PHASES) and labels == PHASES[:len(labels)]
            and (not success or labels == PHASES), 'closed nonempty original phase prefix')
    raw_names = {label + suffix for label in labels for suffix in
                 ('.command.json', '.started.json', '.result.json', '.stdout', '.stderr')}
    raw_names |= {'sources-preformat.json', 'sources-after.json', 'dependencies-after.json'}
    if c['input_sources'] is not None:
        raw_names.add('sources-before.json')
    if 'worker-tests-build' in labels:
        raw_names.add('dependencies-before.json')
    require(set(c['raw']) == raw_names and (not success or len(raw_names) == 50),
            'closed original raw prefix roster')
    expected = {'evidence/' + name: compact(row) for name, row in c['raw'].items()}
    expected[terminal_name] = pin(bodies[terminal_name])
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
    require(set(expected) == set(bodies) and len(expected) <= MEMBERS and (not success or len(expected) == MEMBERS),
            'exact selected prefix or 76 successful retained bodies')
    for name, row in expected.items():
        require(pin(bodies[name]) == row, 'retained body pin: ' + name)
    for name, row in c['raw'].items():
        require(row['path'] == ROOT + '/evidence/' + name, 'raw namespace')
    for phase in c['phases']:
        label = phase['label']
        require(type(phase['exit_code']) is int
                and (phase['exit_code'] == 0 or (not success and label == labels[-1]))
                and phase['natural_exit'] is True and phase['reaped'] is True
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
        if c[key] is not None:
            require(parse(bodies['evidence/' + filename]) == c[key], 'complete source map join')
    pre = {name: compact(row) for name, row in c['preformat_sources'].items()}
    final = {name: compact(row) for name, row in c['final_sources'].items()}
    require(pre == i['files'] and len(pre) == len(final) == 1014, 'exact full preformat source map')
    require(sum(name.startswith('fe2o3/') for name in final) == 815
            and sum(name.startswith(PREFIX) for name in final) == 197, 'runtime/worker source census')
    changed = {name for name in pre if pre[name] != final[name]}
    require(changed <= overlay, 'only observed authorized formatter transitions')
    if c['input_sources'] is not None:
        require(changed == set(c['format_changed_paths']) and len(c['format_changed_paths']) == len(changed),
                'recorded completed formatter transitions')
    else:
        require(c['format_changed_paths'] == [], 'original incomplete formatter metadata is not rewritten')
    old = parse(bodies['inputs/worker-complete.json'])
    old_map = parse(bodies['inputs/worker-sources.json'])
    exact(bodies['inputs/worker-complete.json'],
          (1709799, '28e110ab7ebd6d28e9f31499165f364bac729e7e6d0a408b6cb3a369044a340b'))
    exact(bodies['inputs/worker-sources.json'],
          (420673, '8672f496167bbe2879a2d2d81fa7cf4d292cb9d91fabc598b7dcf7b6a113c31f'))
    exact(bodies['inputs/worker-tests.stdout'],
          (80215, '3c873404f8862e3ad749510cf8b93b1cc71700e3968eefb3a8516156a626d669'))
    exact(bodies['inputs/worker-list.stdout'],
          (75411, '2392825ab706930e33c8465301757b5183d65fe364021e49f3962b82c8757242'))
    require(old_map == old['final_sources'] == old['input_sources'] and old['passed'] is True
            and old['failure'] is None and old['postcheck_errors'] == [] and len(old_map) == 1012,
            'actual current-worker predecessor and final map')
    runtime = parse(bodies['inputs/runtime-complete.json'])
    runtime_map = parse(bodies['inputs/runtime-sources.json'])
    exact(bodies['inputs/runtime-complete.json'],
          (2250677, 'bf0fa20a37ff3bf748ef5984df0dceb5cd1c453ef5c05c7111779b31cda33720'))
    exact(bodies['inputs/runtime-sources.json'],
          (407038, 'f8024397177d83cdd1d07040b55c97afc5190988676bb80f67a9af4ae0554718'))
    require(runtime['passed'] is True and runtime['failure'] is None and runtime['postcheck_errors'] == []
            and runtime['input_sources'] == runtime['final_sources'] == runtime_map and len(runtime_map) == 1014
            and runtime['full_runtime_tests_executed'] is True and runtime['terminal_pair_runtime_added'] is True
            and runtime['gpu_execution'] is False and runtime['global_currentness_policy_changed'] is False
            and runtime['tool_pins'] == old['tool_pins'] == c['tool_pins'] == i['tool_pins'],
            'separate actual runtime qualification and matching toolchain')
    expected_pre = {name: compact(row) for name, row in runtime_map.items() if name.startswith('fe2o3/')}
    require(len(expected_pre) == 815, 'exact actual terminal runtime body roster')
    worker = {name: compact(row) for name, row in old_map.items() if name.startswith(PREFIX)}
    require(len(worker) == 195, 'actual predecessor worker roster')
    expected_pre.update(worker)
    for name, row in rows.items():
        require(expected_pre.get(name) == row['before'], 'actual worker preimage or new absence')
        expected_pre[name] = row['after']
    expected_pre.update({name: pin(bodies[name]) for name in ('run_cpu.py', 'supervisor.py')})
    require(expected_pre == pre, 'composed runtime/worker/overlay/controller preformat closure')
    require(all(pre[name] == final[name] for name in pre if name not in overlay),
            'all non-overlay source bodies unchanged')
    if success:
        require(c['full_worker_tests_executed'] is True
                and c['cli_executable_unchanged_across_tests'] is True, 'actual complete tested executable gate')
        tests = c['tests']['worker-tests']
        observed = named(bodies['evidence/worker-tests.stdout'])
        require(observed == {row['name']: row['outcome'] for row in tests['named']}
                and len(tests['named']) == len(observed) == 684, 'all current named raw outcomes')
        prior = named(bodies['inputs/worker-tests.stdout'])
        require(prior == {row['name']: row['outcome'] for row in old['tests']['worker-tests']['named']}
                and len(prior) == 677 and all(observed[name] == outcome for name, outcome in prior.items())
                and set(observed) - set(prior) == additions and all(observed[name] == 'ok' for name in additions),
                'all old outcomes preserved and exactly seven declared additions')
        require({key: tests[key] for key in ('passed', 'failed', 'ignored')} == dict(passed=680, failed=0, ignored=4),
                'actual complete suite totals')
        summary = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',
                             bodies['evidence/worker-tests.stdout'].decode(), re.M)
        require(summary == [('661', '0', '4', '0', '0'), ('0', '0', '0', '0', '0'), ('3', '0', '0', '0', '0'), ('16', '0', '0', '0', '0')],
                'actual library/binary/integration summaries')
        inventory = re.findall(r'^(\S+): test$', bodies['evidence/worker-list.stdout'].decode(), re.M)
        require(len(inventory) == len(set(inventory)) == 684 and sorted(inventory) == c['inventory']
                and set(inventory) == set(observed), 'actual complete inventory')
        require(bodies['evidence/dependencies-before.json'] == bodies['evidence/dependencies-after.json'],
                'recorded dependency hashes unchanged')
    else:
        require(c['full_worker_tests_executed'] == ('worker-tests' in c['tests']),
                'failure retains original complete-suite flag')
        for label, observed_tests in c['tests'].items():
            require(label == 'worker-tests' and label in labels
                    and named(bodies['evidence/' + label + '.stdout']) ==
                        {row['name']: row['outcome'] for row in observed_tests['named']},
                    'any originally admitted suite joins its raw outcomes')
    if 'dependencies-before.json' in c['raw']:
        require(bodies['evidence/dependencies-before.json'] == bodies['evidence/dependencies-after.json'],
                'recorded dependency hashes unchanged')
    else:
        require(parse(bodies['evidence/dependencies-after.json']) == {}, 'no unobserved dependency metadata')
    cargo = {label: [parse(line) for line in bodies['evidence/' + label + '.stdout'].splitlines() if line]
             for label in ('worker-tests-build', 'worker-build') if label in labels}
    roles = {'worker', 'worker-lib', 'worker-bin-test', 'worker-wire-test', 'worker-readiness-test'}
    require(set(c['artifacts']) <= roles and (not success or set(c['artifacts']) == roles), 'closed product roles')
    for role, artifact in c['artifacts'].items():
        label = 'worker-build' if role == 'worker' else 'worker-tests-build'
        require(label in cargo and artifact['cargo_artifact'] in cargo[label]
                and artifact['cargo_artifact']['executable'] == artifact['pin']['path'], 'original Cargo product join')
    if c['cli_executable_before_tests'] is not None:
        require(c['cli_executable_before_tests']['cargo_artifact'] in cargo['worker-tests-build'],
                'real pretest ELF from actual Cargo stream')
    if success:
        require(c['cli_executable_before_tests']['pin'] == c['artifacts']['worker']['pin'],
                'same tested and finally built real worker ELF')
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
    require(staged['passed'] is True and staged['root'] == ROOT and staged['source_files'] == 1014
            and staged['runtime_files'] == 815 and staged['worker_files'] == 197
            and staged['overlay_files'] == 9 and staged['input'] == pin(bodies['input-manifest.json'])
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
    if c['cli_executable_before_tests'] is not None:
        artifact = c['cli_executable_before_tests']['pin']
        require(artifact['path'].startswith(ROOT + '/target/debug/'), 'owned pretest binary path')
        add(artifact['path'], artifact)
    dependencies = parse(bodies['evidence/dependencies-after.json'])
    require(len(dependencies) == (29 if 'dependencies-before.json' in c['raw'] else 0),
            'observed resolved worker registry package roots')
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


def export(terminal_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'actual CPU host')
    root = Path(ROOT)
    require(root.resolve(strict=True) == root and E.resolve(strict=True) == E, 'closed source root')
    output = E / BASENAME
    require(not os.path.lexists(output) and os.statvfs(E).f_bavail * os.statvfs(E).f_frsize >= 1 << 30,
            'fresh archive and space reserve')
    terminals = [name for name in ('complete.json', 'failed.json')
                 if os.path.lexists(root / 'evidence' / name)]
    require(len(terminals) == 1, 'exact one original terminal path')
    terminal_name = 'evidence/' + terminals[0]
    terminal_raw = read(root / terminal_name, MAX_FILE)
    require(pin(terminal_raw)['sha256'] == terminal_sha, 'observed terminal SHA')
    c = parse(terminal_raw)
    require({str(p.relative_to(root / 'evidence')) for p in paths_below(root / 'evidence')} ==
            set(c['raw']) | set(terminals), 'closed actual raw directory')
    bodies = {terminal_name: terminal_raw}
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
    expected, c = validate(bodies, terminal_sha)
    live_count = live_check(c, bodies)
    require(sum(map(len, bodies.values())) <= MAX_TOTAL, 'bounded selected evidence')
    manifest = dict(schema='ferric-readiness40-causal-layer0-worker-cpu-export-v1', files=expected,
        terminal=pin(terminal_raw), terminal_name=terminal_name, passed=c['passed'],
        selected_files=len(expected), raw_files=len(c['raw']), source_rows=1014,
        formatted_overlay_files=9, lineage_files=7, binary_bodies_retained=False,
        observed_formatter_changes=sorted(name for name in c['preformat_sources']
            if c['preformat_sources'][name] != c['final_sources'][name]),
        live_sources_cache_dependencies_tools_and_products_rehashed=True, live_input_count=live_count,
        actual_tests={k: c['tests']['worker-tests'][k] for k in ('passed', 'failed', 'ignored')}
            if 'worker-tests' in c['tests'] else None, native_execution=False,
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
    print(json.dumps(dict(archive=dict(path=str(output), **stream_pin(output)), members=len(archive_bodies),
        selected_files=len(expected), raw_files=len(c['raw']), passed=c['passed'], expanded_bytes=sum(map(len, archive_bodies.values()))), sort_keys=True))


def retain(archive_sha, terminal_sha):
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
    require(27 <= len(bodies) <= MEMBERS + 1, 'bounded original outcome evidence archive')
    manifest_raw = bodies.pop('manifest.json')
    manifest = parse(manifest_raw)
    require(bodies['evidence.py'] == read(Path(__file__).resolve(), MAX_FILE), 'same reviewed export/retain source')
    expected, terminal = validate(bodies, terminal_sha)
    terminal_name = 'evidence/complete.json' if terminal['passed'] else 'evidence/failed.json'
    actual_tests = ({k: terminal['tests']['worker-tests'][k] for k in ('passed', 'failed', 'ignored')}
                    if 'worker-tests' in terminal['tests'] else None)
    changed = sorted(name for name in terminal['preformat_sources']
                     if terminal['preformat_sources'][name] != terminal['final_sources'][name])
    require(manifest['schema'] == 'ferric-readiness40-causal-layer0-worker-cpu-export-v1'
            and manifest['files'] == expected and manifest['terminal'] == pin(bodies[terminal_name])
            and manifest['terminal_name'] == terminal_name and manifest['passed'] is terminal['passed']
            and manifest['selected_files'] == len(expected) and manifest['formatted_overlay_files'] == 9
            and manifest['lineage_files'] == 7 and manifest['source_rows'] == 1014
            and manifest['raw_files'] == len(terminal['raw']) and manifest['actual_tests'] == actual_tests
            and manifest['observed_formatter_changes'] == changed
            and manifest['live_sources_cache_dependencies_tools_and_products_rehashed'] is True
            and all(manifest[k] is False for k in ('binary_bodies_retained', 'native_execution',
                                                  'full_long_workload', 'numerical_acceptance', 'performance_claim')),
            'closed original CPU outcome observations')
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
    report = dict(schema='ferric-readiness40-causal-layer0-worker-cpu-retention-v1', archive=pin(raw),
        files={n: pin(b) for n, b in sorted(bodies.items())}, original_files=len(bodies),
        terminal=pin(bodies[terminal_name]), terminal_name=terminal_name, passed=terminal['passed'],
        formatted_overlay_files=9, observed_formatter_changes=changed,
        artifacts=terminal['artifacts'], actual_tests=actual_tests,
        binary_bodies_retained=False, external_inputs_rehashed_locally=False,
        original_terminal_unchanged=True, new_project_execution=False, new_native_execution=False,
        full_long_workload=False, numerical_acceptance=False, performance_claim=False)
    with (DEST / 'retention.json').open('xb') as stream:
        stream.write(encoded(report)); stream.flush(); os.fsync(stream.fileno())
    print(json.dumps(dict(destination=str(DEST), original_files=len(bodies),
                          passed=terminal['passed'], actual_tests=actual_tests)))


def integration(terminal_sha, output):
    require(output.is_absolute() and output.parent.resolve(strict=True) == output.parent
            and not output.is_relative_to(F) and not os.path.lexists(output), 'fresh proposal output outside canonical tree')
    manifest_raw = read(DEST / 'manifest.json', MAX_FILE)
    manifest = parse(manifest_raw)
    require(len(manifest['files']) == MEMBERS, 'retained original roster')
    require({str(p.relative_to(DEST)) for p in paths_below(DEST)} ==
            set(manifest['files']) | {'manifest.json', 'retention.json'}, 'closed retained capsule')
    bodies = {name: read(DEST / name, MAX_FILE) for name in manifest['files']}
    require(bodies['evidence.py'] == read(Path(__file__).resolve(), MAX_FILE), 'same qualified retention source')
    expected, terminal = validate(bodies, terminal_sha)
    require(terminal['passed'] is True, 'failed outcomes never authorize source integration')
    require(manifest['files'] == expected and manifest['terminal'] == pin(bodies['evidence/complete.json']),
            'retained manifest joins')
    old_map = parse(bodies['inputs/worker-sources.json'])
    overlay = set(parse(bodies['input-manifest.json'])['worker_overlay'])
    final = terminal['final_sources']
    current, desired = {}, {}
    paths = {name for name in old_map if name.startswith(PREFIX)}
    final_paths = {name for name in final if name.startswith(PREFIX)}
    require(len(paths) == 195 and len(final_paths) == 197 and final_paths == paths | overlay,
            'entire predecessor and two-added worker source roster')
    for name in sorted(final_paths):
        path = F / name.removeprefix('ferric/')
        if name in paths:
            raw = read(path, MAX_FILE)
            require(pin(raw) == compact(old_map[name]), 'canonical source preimage: ' + name)
            current[name] = raw
        else:
            require(name in overlay and not os.path.lexists(path), 'new canonical path absent')
            raw = None
            current[name] = None
        desired[name] = bodies[name] if name in overlay else raw
        require(pin(desired[name]) == compact(final[name]), 'composed qualified worker postimage')
    patch = ['*** Begin Patch\n']
    rows = []
    for name in sorted(overlay):
        before, after = current[name], desired[name]
        require((before is None or before.endswith(b'\n')) and after.endswith(b'\n'), 'text patch final newlines')
        path = str(F / name.removeprefix('ferric/'))
        if before is None:
            patch.append('*** Add File: ' + path + '\n')
        else:
            patch += ['*** Update File: ' + path + '\n', '@@\n']
            patch.extend('-' + line + '\n' for line in before.decode().splitlines())
        patch.extend('+' + line + '\n' for line in after.decode().splitlines())
        rows.append(dict(path=name.removeprefix('ferric/'), before=pin(before) if before is not None else None, after=pin(after)))
    patch.append('*** End Patch\n')
    patch_raw = ''.join(patch).encode()
    require(len(rows) == 9 and sum(row['before'] is None for row in rows) == 2
            and len(patch_raw) <= MAX_FILE, 'seven replacements and two additions')
    require(all((read(F / name.removeprefix('ferric/'), MAX_FILE) == body if body is not None else
                 not os.path.lexists(F / name.removeprefix('ferric/'))) for name, body in current.items())
            and read(DEST / 'manifest.json', MAX_FILE) == manifest_raw
            and all(read(DEST / name, MAX_FILE) == body for name, body in bodies.items()), 'all integration inputs posthashed')
    report = dict(schema='ferric-readiness40-causal-layer0-worker-integration-plan-v1',
        terminal=pin(bodies['evidence/complete.json']), files=rows, patch=pin(patch_raw),
        composed_worker_sources={name.removeprefix('ferric/'): pin(body) for name, body in sorted(desired.items())},
        source_files=197, canonical_changed=False, patch_applied=False, project_execution=False,
        gpu_execution=False, numerical_acceptance=False, performance_claim=False)
    output.mkdir(mode=0o700)
    for name, body in [('integrate-qualified.patch', patch_raw), ('integrate-qualified.json', encoded(report))]:
        with (output / name).open('xb') as stream:
            stream.write(body); stream.flush(); os.fsync(stream.fileno())
        require(read(output / name, MAX_FILE) == body, 'integration artifact readback')
    print(json.dumps(dict(output=str(output), rows=9, full_worker_sources=197, patch=pin(patch_raw))))


if __name__ == '__main__':
    require(__debug__ and sys.dont_write_bytecode, 'python3 -B evidence.py export TERMINAL_SHA | retain ARCHIVE_SHA TERMINAL_SHA | integrate TERMINAL_SHA OUTPUT')
    resource.setrlimit(resource.RLIMIT_AS, (512 << 20, 512 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_TOTAL, MAX_TOTAL))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    def interrupted(number, _frame):
        raise RuntimeError('evidence helper signal ' + str(number))
    for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
        signal.signal(number, interrupted)
    signal.alarm(180)
    try:
        require(len(sys.argv) in (3, 4) and re.fullmatch('[0-9a-f]{64}', sys.argv[2]) is not None, 'observed SHA argument')
        if sys.argv[1] == 'export' and len(sys.argv) == 3:
            export(sys.argv[2])
        elif sys.argv[1] == 'retain' and len(sys.argv) == 4:
            require(re.fullmatch('[0-9a-f]{64}', sys.argv[3]) is not None, 'observed terminal SHA')
            retain(sys.argv[2], sys.argv[3])
        else:
            require(sys.argv[1] == 'integrate' and len(sys.argv) == 4, 'closed data-only helper CLI')
            integration(sys.argv[2], Path(sys.argv[3]))
    finally:
        signal.alarm(0)
