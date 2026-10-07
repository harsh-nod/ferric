"""Bounded data-only export/retention; never imports or executes project code."""
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
import time
import tomllib

ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-causal-layer0-parent-cpu-v228-v2')
OLD_ROOT = str(ROOT.parent / 'guarded-mlp-warm-paired-terminal-parent-cpu-v228-v3')
PARENT = 'ferric/adapters/m1-engineering-execution-v1/'
WORKER = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
FEATURE = 'guarded-mlp-readiness-engineering'
BINARY = 'ferric-qwen3-finite-guarded-mlp-decode-engineering'
INPUT = dict(bytes=259716, sha256='a498caf42174c349d19134944be6a9450996642760721af8504fdd35a976b133')
SOURCE_ARCHIVE = dict(bytes=1661509, sha256='3a44e2a45d6625ae78f87592552016359be049969c0474859368d0ab211cfff8')
CACHE = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
HELPERS = {
    'run_cpu.py': dict(bytes=39137, sha256='a3f199ad8b251307b73eec66284e0f3a740d68b24a1dee3e25a4d4c0e2436013'),
    'qualification_support.py': dict(bytes=34328, sha256='71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'),
    'supervisor.py': dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'),
    'parent_transport.py': dict(bytes=23252, sha256='dac6bb466a5f2440680c1151a68ea6a23622d14364e607b2318f7f42af6fe456'),
    'stage_parent_cache.py': dict(bytes=14350, sha256='e5d3121a1beabd1ead2db2202a2d0d80ddc0b7e2b2995de4bc722fdea71f412e'),
}
BASE_PINS = {
    'parent-complete.json': dict(bytes=3901680, sha256='29af2c2ef9e4eeb93bdc0f57a5ae463c2baf751f2824e86430260d8a29ce1f3c'),
    'parent-sources.json': dict(bytes=494635, sha256='a8ce9773a765fbb63fa18735818ac7defebb52d5c9b8e34140a1718a4b4042b5'),
    'worker-complete.json': dict(bytes=1709799, sha256='28e110ab7ebd6d28e9f31499165f364bac729e7e6d0a408b6cb3a369044a340b'),
    'worker-sources.json': dict(bytes=420673, sha256='8672f496167bbe2879a2d2d81fa7cf4d292cb9d91fabc598b7dcf7b6a113c31f'),
    'source-manifest.json': dict(bytes=8279, sha256='9cce663d74160516c14033e8417c54dbce666b25e61dc232c11b494c055f558c'),
    'parent-read-fix.json': dict(bytes=2203, sha256='e4252b407d036ba8aefdf0a5d3d2235af2427b500242f060aa2a09105471369e'),
    'qualified-worker-complete.json': dict(bytes=1731960, sha256='5c4ffff4b06c7ad1d73c6985c6bcb0193b596ab6618bbda44010347a9b26aa1a'),
    'qualified-worker-sources.json': dict(bytes=426647, sha256='00b15b5478ebbeb6fbd00a2de68e24bb6381a6f28a521fc566b95539aa3e35a5'),
    'parent-v1-failed.json': dict(bytes=3739856, sha256='060c05edc7ec35703e54c8120fa630553fc7c263205e2b45fdddb0c03a69225b'),
    'parent-worker-transitions.json': dict(bytes=4196, sha256='80efcae58f7377d6804630935d0e3c3da93a69da8ee43793e14718a1a3f0a92b'),
}
MAX_FILE, MAX_TOTAL, MAX_MEMBERS = 16 << 20, 96 << 20, 512
DEADLINE = None


def require(ok, message):
    if not ok:
        raise ValueError(message)


def guard():
    require(DEADLINE is not None and time.monotonic() < DEADLINE, 'whole evidence deadline')


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def parse(body):
    def pairs(rows):
        out = {}
        for key, value in rows:
            require(key not in out, 'duplicate JSON field')
            out[key] = value
        return out
    return json.loads(body, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()


def relative(name):
    path = PurePosixPath(name)
    require(type(name) is str and name and str(path) == name and not path.is_absolute()
            and all(p not in ('.', '..') for p in path.parts) and '\\' not in name, 'ordinary relative path')
    return name


def read(path, maximum=MAX_FILE, keep=True):
    guard()
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical input path')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    digest, chunks, size = hashlib.sha256(), [], 0
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= maximum, 'bounded regular body')
        while True:
            guard()
            chunk = stream.read(min(1 << 20, maximum + 1 - size))
            if not chunk:
                break
            size += len(chunk)
            require(size <= maximum, 'body grew past bound')
            digest.update(chunk)
            if keep:
                chunks.append(chunk)
        after = os.fstat(stream.fileno())
    require(stamp(before) == stamp(after) == stamp(path.lstat()) and size == before.st_size, 'body changed during read')
    return b''.join(chunks) if keep else dict(bytes=size, sha256=digest.hexdigest())


def tree(root, omit_git=False):
    root = Path(root)
    require(root.resolve(strict=True) == root and root.is_dir(), 'ordinary tree root')
    names = set()
    for directory, dirs, files in os.walk(root, followlinks=False,
            onerror=lambda e: (_ for _ in ()).throw(e)):
        guard()
        require(not any((Path(directory) / n).is_symlink() for n in dirs), 'directory alias')
        if omit_git:
            dirs[:] = [n for n in dirs if n != '.git']
        for name in files:
            rel = str((Path(directory) / name).relative_to(root))
            if omit_git and '.git' in Path(rel).parts:
                continue
            names.add(relative(rel))
            require(len(names) <= 150000, 'bounded tree roster')
    return names


def outcomes(raw):
    summaries, named, active, progress = [], [], [], set()
    for line in raw.decode().splitlines():
        notice = re.fullmatch(r'test ([A-Za-z0-9_:]+) has been running for over 60 seconds', line)
        if notice:
            name = notice.group(1)
            require(name not in progress and name not in {r['name'] for r in named}, 'test progress')
            progress.add(name)
            continue
        match = re.fullmatch(r'test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?', line)
        if match:
            name, status = match.groups()
            require(name not in {r['name'] for r in named}, 'duplicate named result')
            row = dict(name=name, outcome=status)
            named.append(row); active.append(row)
            continue
        match = re.fullmatch(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                            r'(\d+) measured; (\d+) filtered out; finished in [0-9.]+s', line)
        if match:
            state, *numbers = match.groups()
            row = dict(zip(('passed', 'failed', 'ignored', 'measured', 'filtered_out'), map(int, numbers)), status=state)
            require(all(sum(r['outcome'] == status for r in active) == row[key]
                for key, status in [('passed', 'ok'), ('failed', 'FAILED'), ('ignored', 'ignored')]), 'named summary mismatch')
            summaries.append(row); active = []
        else:
            require(not line.startswith('test '), 'malformed test output')
    require(summaries and not active and progress <= {r['name'] for r in named}, 'incomplete named output')
    return dict(summaries=summaries, named=named, **{k: sum(r[k] for r in summaries) for k in ('passed', 'failed', 'ignored')})


def inventory(raw):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', raw.decode(), re.M)
    require(len(names) == len(set(names)) and ': benchmark' not in raw.decode(), 'test inventory')
    return sorted(names)


def relocate(value):
    if isinstance(value, str):
        return value.replace(OLD_ROOT, str(ROOT))
    if isinstance(value, list):
        return [relocate(v) for v in value]
    if isinstance(value, dict):
        return {k: relocate(v) for k, v in value.items()}
    return value


def recipes(base, inputs):
    tools = base['parent_toolchain_observation']; tc = tools['toolchain']
    cargo = tc + '/cargo'; manifest = str(ROOT / PARENT / 'Cargo.toml')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', manifest]
    feature = ['--features', FEATURE]
    fmt = [tc + '/rustfmt', '--edition', '2024', '--config', 'skip_children=true']
    paths = [str(ROOT / n) for n in inputs['parent_overlay']]
    rows = [('rustfmt', fmt + paths, 120), ('rustfmt-check', fmt + ['--check'] + paths, 120),
        ('rustc-version', [tc + '/rustc', '--version', '--verbose'], 60),
        ('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path', manifest,
                      *feature, '--format-version', '1'], 120),
        ('parent-lib-list', [cargo, 'test', *common, *feature, '--lib', '--', '--list', '--format=terse'], 1200)]
    for old in base['phases']:
        if old['label'] in base['tests'] or old['label'] in ('guarded-bin-list', 'readiness-bin-list'):
            argv = relocate(old['argv'][6:])
            require(argv[argv.index('--features') + 1] == FEATURE, 'unchanged selected feature')
            rows.append((old['label'], argv, 1200))
    bins = sorted(base['artifacts'])
    rows += [('parent-builds', [cargo, 'build', '--profile', 'test', *common, *feature,
                              *[v for n in bins for v in ('--bin', n)], '--message-format=json'], 1200),
             ('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], 1200)]
    require(len(rows) == 60 and len({r[0] for r in rows}) == 60, 'exact sixty recipes')
    return rows


def verify(bodies, terminal_name, terminal_sha):
    guard()
    def body(name, expected=None):
        value = bodies[relative(name)]
        require(expected is None or pin(value) == compact(expected), 'body pin: ' + name)
        return value
    def value(name, expected=None):
        return parse(body(name, expected))
    require(terminal_name in ('complete.json', 'failed.json'), 'terminal basename')
    result = value('evidence/' + terminal_name)
    require(pin(body('evidence/' + terminal_name))['sha256'] == terminal_sha, 'observed terminal SHA')
    success = result['passed'] is True
    require(type(result['passed']) is bool and (terminal_name == 'complete.json') == success
            and (result['failure'] is None) == success and result['postcheck_errors'] == []
            and result['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-parent-cpu-v2', 'clean actual outcome')
    for key in ('gpu_execution', 'causal_layer0_native_execution', 'position5_native_execution', 'readiness_native_execution', 'full_long_workload', 'runtime_suite_rerun',
                'worker_suite_rerun', 'full_model_acceptance', 'numerical_acceptance', 'performance_claim',
                'production_authority', 'performance_policy_changed', 'default_policy_changed',
                'warm_paired_terminal_native_execution', 'global_currentness_policy_changed',
                'hidden_read_policy_changed', 'read_ns_scope_changed', 'full_parent_library_suite_executed'):
        require(result[key] is False, 'false authority/scope: ' + key)
    require(result['parent_causal_file_reads_retained'] is True
            and result['previous_parent_failure_preserved'] is True
            and result['causal_layer0_parent_route_added'] is True
            and result['inherited_warm_paired_terminal_parent_route_preserved'] is True
            and result['causal_capture_positions'] == list(range(6)) and result['causal_layer'] == 0
            and result['warm_paired_terminal_parent_route_added'] is True
            and result['method_local_currentness_cadence_changed'] is True
            and result['selected_terminal_cadence_changed'] is True
            and result['position5_diagnostic_parent_route_added'] is True
            and result['inherited_readiness_parent_route_preserved'] is True,
            'supported inherited parent source scope')
    for n, p in HELPERS.items():
        body(n, p)
    require(compact(result['controller']) == HELPERS['run_cpu.py']
            and compact(result['supervisor']) == HELPERS['supervisor.py'], 'reviewed executed controller')
    inputs = value('input-manifest.json', INPUT)
    require(compact(result['input_manifest']) == INPUT and len(inputs['files']) == 1244
            and len(inputs['lineage']) == 114, 'actual input map')
    for n, p in inputs['lineage'].items():
        body('inputs/' + n, p)
        require(compact(result['readset'][n]) == p
                and result['readset'][n]['path'] == str(ROOT / 'inputs' / n), 'lineage readset join')
    require(set(result['readset']) == set(inputs['lineage']), 'closed lineage readset')
    for n, p in BASE_PINS.items():
        body('inputs/' + n, p)
    base = value('inputs/parent-complete.json'); old = value('inputs/parent-sources.json')
    worker = value('inputs/worker-complete.json'); wm = value('inputs/worker-sources.json')
    proposal = value('inputs/source-manifest.json')
    transitions_doc = value('inputs/parent-worker-transitions.json')
    require(base['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-parent-cpu-v1'
            and base['passed'] is True and base['final_sources'] == old and len(base['tests']) == 51
            and sum(v['passed'] for v in base['tests'].values()) == 438
            and len(base['inventory']) == 913 and len(base['phases']) == 60
            and worker['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-worker-cpu-v1'
            and worker['passed'] is True and worker['final_sources'] == wm
            and worker['tests']['worker-tests']['passed'] == 673
            and worker['tests']['worker-tests']['ignored'] == 4, 'actual parent/worker lineage')
    require(proposal['schema'] == 'ferric-readiness40-causal-layer0-source-proposal-v1',
            'closed causal layer0 source proposal')
    fix = value('inputs/parent-read-fix.json')
    require(fix['schema'] == 'ferric-readiness40-causal-layer0-parent-read-fix-v2'
            and fix['predecessor_source'] == BASE_PINS['source-manifest.json']
            and len(fix['files']) == 3 and all(fix[key] is False for key in
                ('worker_source_changed', 'runtime_source_changed', 'wire_or_image_changed',
                 'limits_changed', 'admission_weakened', 'tests_executed', 'native_execution')),
            'reviewed narrow parent byte-retention correction')
    qualified = value('inputs/qualified-worker-complete.json')
    qualified_map = value('inputs/qualified-worker-sources.json')
    require(qualified['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-worker-cpu-v1'
            and qualified['passed'] is True and qualified['failure'] is None and qualified['postcheck_errors'] == []
            and qualified['input_sources'] == qualified['final_sources'] == qualified_map
            and len(qualified_map) == 1014 and qualified['source_unchanged'] is True
            and compact(qualified['raw']['sources-after.json']) == BASE_PINS['qualified-worker-sources.json']
            and qualified['tests']['worker-tests']['passed'] == 680
            and qualified['tests']['worker-tests']['failed'] == 0
            and qualified['tests']['worker-tests']['ignored'] == 4
            and qualified['cli_executable_unchanged_across_tests'] is True
            and qualified['gpu_execution'] is False, 'actual unchanged causal worker qualification')
    authored_worker = {n: compact(r) for n, r in qualified['preformat_sources'].items() if n.startswith(WORKER)}
    actual_causal_worker = {n: compact(r) for n, r in qualified_map.items() if n.startswith(WORKER)}
    require(len(authored_worker) == len(actual_causal_worker) == 197
            and set(authored_worker) == set(actual_causal_worker), 'closed197 current worker files')
    qualified_changes = {n for n in authored_worker if authored_worker[n] != actual_causal_worker[n]}
    require(qualified_changes == set(qualified['format_changed_paths'])
            and qualified_changes <= {'ferric/' + r['path'] for r in proposal['files']
                                     if ('ferric/' + r['path']).startswith(WORKER)},
            'separate actual worker formatting remains explicit')
    failed = value('inputs/parent-v1-failed.json')
    require(failed['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-parent-cpu-v1'
            and failed['passed'] is False and isinstance(failed['failure'], str)
            and failed['postcheck_errors'] == [] and len(failed['phases']) == 33
            and failed['phases'][-1]['label'] == 'parent-client'
            and failed['phases'][-1]['exit_code'] == 101
            and all(p['natural_exit'] is True and p['reaped'] is True and p['process_group_absent'] is True
                    and p['forced_cleanup'] is False and p['timed_out'] is False
                    for p in failed['phases']), 'original failure is history only, never a successful baseline')
    original_rows = {r['path']: r for r in proposal['files']}
    require(len({r['path'] for r in fix['files']}) == 3, 'three unique correction paths')
    for row in fix['files']:
        require(row['path'].startswith(PARENT.removeprefix('ferric/')) and row['path'] in original_rows
                and original_rows[row['path']]['after'] == row['before'], 'exact V1 authored correction preimage')
        original_rows[row['path']]['after'] = row['after']
    extra = fix['new_tests']['parent_library']
    require(extra == ['tp_finite_client::long::readiness::causal::tests::causal_parent_file_backed_sidecar_reads_and_rechecks_pinned_bytes'],
            'one exact file-backed regression')
    proposal['new_tests']['parent_library'] += [extra[0].rsplit('::', 1)[-1]]
    lineage = set(BASE_PINS) | {'parent-lib-list.stdout', 'metadata.stdout'}
    lineage |= {n + suffix for n in base['tests'] for suffix in ('.stdout', '.command.json')}
    require(set(inputs['lineage']) == lineage and len(lineage) == 114, 'exact114 lineage including actual worker and failed V1')
    for n in lineage - set(BASE_PINS):
        require(inputs['lineage'][n] == compact(base['raw'][n]), 'baseline raw ancestry')
    pre = value('evidence/sources-preformat.json')
    before = value('evidence/sources-before.json') if result['input_sources'] is not None else None
    after = value('evidence/sources-after.json')
    require(result['preformat_sources'] == pre and result['input_sources'] == before
            and after == result['final_sources']
            and inputs['files'] == {n: compact(v) for n, v in pre.items()}
            and set(pre) == set(after) and len(after) == 1244, 'all source maps')
    if before is not None:
        require(result['source_unchanged'] is True and before == after, 'clean post-format sources')
    else:
        require(not success and result['source_unchanged'] is False, 'failed preformat-only attempt')
    require(all(row['path'] == str(ROOT / relative(n)) for n, row in after.items()), 'source path mapping')
    expected = {n: compact(v) for n, v in old.items() if n.startswith('ferric/')}
    work = {n: compact(v) for n, v in wm.items() if n.startswith(WORKER)}
    transitions = {'ferric/' + r['path']: r for r in transitions_doc['rows']}
    require(transitions_doc['parent_source'] == BASE_PINS['parent-sources.json']
            and transitions_doc['worker_source'] == BASE_PINS['worker-sources.json']
            and transitions_doc['worker_files'] == 195
            and len(expected) == 1237 and len(work) == 195 and len(transitions) == 10
            and set(transitions) == {n for n in work if expected[n] != work[n]}
            and all(r['before'] == expected[n] and r['after'] == work[n] for n, r in transitions.items()),
            'ten actual inherited worker formatter transitions')
    expected.update(work)
    overlay = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(overlay) == 14 and len(proposal['files']) == 14
            and sum(n.startswith(WORKER) for n in overlay) == 9
            and sum(n.startswith(PARENT) for n in overlay) == 5
            and sum(r['before'] is None for r in overlay.values()) == 4, 'reviewed14 source rows/four additions')
    for n, r in overlay.items():
        require(expected.get(n) == r['before'], 'overlay preimage')
        expected[n] = r['after']
    require({n: p for n, p in expected.items() if n.startswith(WORKER)} == authored_worker,
            'parent worker input matches separately qualified authored preformat bytes')
    expected.update({n: HELPERS[n] for n in ('run_cpu.py', 'qualification_support.py', 'supervisor.py')})
    require(inputs['files'] == expected, 'entire composed source map')
    formatted = sorted(n for n in after if compact(after[n]) != compact(pre[n]))
    allowed = sorted(n for n in overlay if n.startswith(PARENT) and n.endswith('.rs'))
    require(inputs['parent_overlay'] == allowed and len(allowed) == 5
            and set(formatted) <= set(allowed)
            and result['format_changed_paths'] == (formatted if before is not None else []),
            'only five parent Rust sources may format')
    parent_sources = {n for n in overlay if n.startswith(PARENT)} | {PARENT + 'Cargo.lock'}
    require(len(parent_sources) == 6, 'five parent postimages plus unchanged lock')
    for n in parent_sources:
        body(n, after[n])
    stage = value('stage.json')
    require(stage['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-parent-source-stage-v2'
            and stage['passed'] is True and stage['archive'] == SOURCE_ARCHIVE and stage['input'] == INPUT
            and stage['controller'] == HELPERS['parent_transport.py'] and stage['root'] == str(ROOT)
            and [stage[k] for k in ('source_files', 'ferric_files', 'overlay_files', 'inherited_worker_transitions', 'helpers', 'lineage_files')]
                == [1244, 1241, 14, 10, 3, 114]
            and all(stage[k] is False for k in ('project_execution', 'canonical_changed', 'shared_cache_changed', 'lockfiles_changed')),
            'actual source staging')
    cache = value('cargo-cache-manifest.json', CACHE); cs = value('cargo-cache-stage-complete.json')
    provenance = result['cache_provenance']
    require(compact(provenance['manifest']) == CACHE
            and compact(provenance['stage']) == pin(body('cargo-cache-stage-complete.json'))
            and len(cache['files']) == 247 and len(cache['packages']) == 118 and len(cache['git_commits']) == 3
            and {n: compact(p) for n, p in provenance['files'].items()} == cache['files']
            and cs['passed'] is True and cs['manifest'] == CACHE and cs['files'] == cache['files']
            and cs['git_commits'] == cache['git_commits'] and cs['cargo_home'] == str(ROOT / 'cargo-home')
            and cs['lock'] == cache['lock'] == pin(body(PARENT + 'Cargo.lock')),
            'immutable private247-file cache')
    require(all(cs[k] is False for k in ('shared_cache_changed', 'lock_changed', 'project_code_executed',
                                        'crate_sources_extracted', 'git_sources_checked_out')), 'cache staging is data only')
    lock = tomllib.loads(body(PARENT + 'Cargo.lock').decode())['package']
    require(sorted((p['name'], p['version'], p['checksum']) for p in lock if p.get('source', '').startswith('registry+'))
            == sorted((p['name'], p['version'], p['checksum']) for p in cache['packages']), 'exact locked registry roster')
    require({p['source'] for p in lock if p.get('source', '').startswith('git+')}
            == {p['source'] for p in cache['git_commits']}, 'exact locked Git revisions')
    planned = recipes(base, inputs); phases = result['phases']
    require(1 <= len(phases) <= 60 and [p['label'] for p in phases] == [p[0] for p in planned[:len(phases)]], 'serial recipe prefix')
    raw_names = {name + suffix for name, _, _ in planned[:len(phases)]
                 for suffix in ('.command.json', '.started.json', '.stdout', '.stderr', '.result.json')}
    raw_names |= {'sources-preformat.json', 'sources-after.json'}
    if before is not None:
        raw_names.add('sources-before.json')
    if 'dependencies-before.json' in result['raw']:
        raw_names.add('dependencies-before.json')
    require(set(result['raw']) == raw_names, 'exact raw phase/map roster')
    for n, p in result['raw'].items():
        require(p['path'] == str(ROOT / 'evidence' / n), 'raw identity')
        body('evidence/' + n, p)
    first_old = value('inputs/' + next(iter(base['tests'])) + '.command.json')
    environment = relocate(first_old['env'])
    prefix = ['/usr/bin/prlimit', '--as=12884901888', '--cpu=1200', '--fsize=1073741824', '--core=0', '--']
    for index, (phase, (label, argv, maximum)) in enumerate(zip(phases, planned)):
        require(phase['argv'] == prefix + argv and phase['natural_exit'] is True and phase['reaped'] is True
                and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
                and phase['timed_out'] is False and phase['exception'] is None and phase['storage_failure'] is None
                and phase['observed_signals'] == [] and (phase['exit_code'] == 0 or (not success and index == len(phases)-1)),
                'natural bounded phase')
        require(value('evidence/' + label + '.result.json') == phase, 'exact original phase result')
        command = value('evidence/' + label + '.command.json'); started = value('evidence/' + label + '.started.json')
        require(command['argv'] == phase['argv'] == started['argv'] and command['env'] == environment
                and command['cwd'] == str(ROOT / PARENT) and 0 < command['wall_timeout_seconds'] <= maximum
                and started['pid'] == started['pgid'] == phase['pid'] == phase['pgid'], 'command/start/resource joins')
        for k in ('command', 'stdout', 'stderr'):
            require(result['raw'][label + '.' + ('command.json' if k == 'command' else k)] == phase[k], 'phase raw pin')
    additions = ['tp_finite_client::long::readiness::tests::' + n
                 if n == 'causal_parent_request_cannot_enter_ordinary_or_position5_routes' else
                 'tp_finite_client::long::readiness::causal::tests::' + n
                 for n in proposal['new_tests']['parent_library']]
    bin_names = ['tests::' + n for n in proposal['new_tests']['parent_readiness_binary']]
    require(len(additions) == len(set(additions)) == 5
            and not set(additions) & set(base['inventory'])
            and bin_names == ['tests::causal_parent_cli_has_a_separate_closed_mode'],
            'exact five library and one readiness-bin additions')
    expected_tests = {}
    for label, old_test in base['tests'].items():
        require(outcomes(body('inputs/' + label + '.stdout')) == old_test, 'prior actual selected outcomes')
        names = {r['name'] for r in old_test['named']}
        argv = next(argv for name, argv, _ in planned if name == label)
        if '--lib' in argv:
            selector = argv[argv.index('--lib') + 1]
            names |= {n for n in additions if n == selector or ('--exact' not in argv and selector in n)}
        elif label == 'readiness-bin-tests':
            names |= set(bin_names)
        expected_tests[label] = names
    require(len(expected_tests) == 51 and sum(map(len, expected_tests.values())) == 444, 'preserved selections plus six')
    require(set(result['tests']) <= set(expected_tests), 'only declared selections')
    for label, observed in result['tests'].items():
        require(outcomes(body('evidence/' + label + '.stdout')) == observed
                and {r['name']: r['outcome'] for r in observed['named']} == {n: 'ok' for n in expected_tests[label]}
                and observed['failed'] == observed['ignored'] == 0, 'exact actual named selection')
    if result['inventory']:
        require(inventory(body('evidence/parent-lib-list.stdout')) == result['inventory']
                == sorted(base['inventory'] + additions) and len(result['inventory']) == 918, '918 library inventory')
    products = result['artifacts']
    records = [parse(line) for line in body('evidence/parent-builds.stdout').splitlines() if line.startswith(b'{')] \
        if 'parent-builds.stdout' in result['raw'] else []
    for name, product in products.items():
        row = product['cargo_artifact']; p = product['pin']
        candidates = [r for r in records if r.get('reason') == 'compiler-artifact'
            and r.get('manifest_path') == str(ROOT / PARENT / 'Cargo.toml')
            and r.get('target', {}).get('name') == name and r['target']['kind'] == ['bin']
            and r.get('profile', {}).get('test') is False]
        require(candidates == [row] and p['path'] == row['executable']
                and Path(p['path']).is_relative_to(ROOT / 'target') and FEATURE in row['features']
                and row['filenames'].count(p['path']) == 1, 'actual Cargo-selected parent product')
    require(result['tool_pins'] == base['tool_pins'] == result['parent_toolchain_observation']['tool_pins']
            and result['parent_toolchain_observation'] == base['parent_toolchain_observation']
            and all(v is None for v in result['configurations'].values()), 'actual tool/configuration closure')
    if success:
        require(len(phases) == 60 and len(result['tests']) == 51 and set(result['tests']) == set(expected_tests)
                and sum(v['passed'] for v in result['tests'].values()) == 444
                and set(products) == set(base['artifacts'])
                and result['all_selected_parent_tests_executed'] is True and len(result['raw']) == 304,
                'complete60/51/444/six-product success')
    expected_bodies = {'evidence/' + n for n in raw_names | {terminal_name}}
    expected_bodies |= {'inputs/' + n for n in lineage} | set(HELPERS) | parent_sources
    expected_bodies |= {'input-manifest.json', 'stage.json', 'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json', 'evidence.py'}
    require(set(bodies) == expected_bodies and (not success or len(bodies) == 435), 'closed selected evidence bodies')
    return result


def live_rehash(result):
    sources = result['final_sources']
    require(tree(ROOT / 'ferric') == {n[len('ferric/'):] for n in sources if n.startswith('ferric/')}, 'live full source roster')
    for n, row in sources.items():
        require(read(ROOT / n, keep=False) == compact(row), 'live source drift')
    for row in result['tool_pins'].values():
        require(read(Path(row['path']), 1 << 30, False) == compact(row), 'live tool drift')
    for row in result['cache_provenance']['files'].values():
        require(Path(row['path']).is_relative_to(ROOT / 'cargo-home')
                and read(Path(row['path']), 64 << 20, False) == compact(row), 'live immutable cache drift')
    dep_path = ROOT / 'evidence/dependencies-before.json'
    roots = parse(read(dep_path)) if dep_path.exists() else {}
    require(not result['passed'] or len(roots) == 121, 'complete dependency roots')
    for name, rows in roots.items():
        root = Path(name)
        require(root.is_relative_to(ROOT / 'cargo-home') and tree(root, True) == set(rows), 'live dependency roster')
        for rel, row in rows.items():
            require(row['path'] == str(root / relative(rel))
                    and read(root / rel, 64 << 20, False) == compact(row), 'live dependency drift')
    for value in result['artifacts'].values():
        row = value['pin']; path = Path(row['path'])
        require(read(path, 1 << 30, False) == compact(row), 'live product drift')
        with path.open('rb') as stream:
            require(stream.read(4) == b'\x7fELF', 'actual selected ELF')
    require(all(not os.path.lexists(p) for p in result['configurations']), 'live Cargo configuration drift')
    return dict(sources=len(sources), dependency_roots=len(roots), cache_files=len(result['cache_provenance']['files']),
                tools=len(result['tool_pins']), products=len(result['artifacts']))


def unpack(raw):
    bodies, seen, total = {}, set(), 0
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for member in archive:
            guard(); relative(member.name)
            require(member.isfile() and member.name not in seen and len(seen) < MAX_MEMBERS
                    and 0 <= member.size <= MAX_FILE and not member.pax_headers, 'closed regular bounded archive')
            seen.add(member.name); total += member.size
            require(total <= MAX_TOTAL, 'expanded evidence bound')
            data = archive.extractfile(member).read(MAX_FILE + 1)
            require(len(data) == member.size, 'complete archive member')
            bodies[member.name] = data
    return bodies


def main():
    global DEADLINE
    DEADLINE = time.monotonic() + 300
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError('whole evidence deadline')))
    signal.setitimer(signal.ITIMER_REAL, 300)
    resource.setrlimit(resource.RLIMIT_AS, (768 << 20, 768 << 20))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_CPU, (240, 240))
    require(__debug__ and sys.dont_write_bytecode, 'nonoptimized python3 -I -S -B')
    require(len(sys.argv) == 7 and sys.argv[1] in ('export', 'retain'),
        'export TERMINAL_SHA STAGER CACHE_STAGER ARCHIVE TERMINAL_NAME | retain ARCHIVE ARCHIVE_SHA TERMINAL_SHA DEST TERMINAL_NAME')
    mode = sys.argv[1]; terminal_name = sys.argv[6]
    own = read(Path(__file__).resolve())
    if mode == 'export':
        terminal_sha, stage_path, cache_path, destination = sys.argv[2:6]
        require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03', 'export host/UID')
        require(re.fullmatch('[0-9a-f]{64}', terminal_sha), 'observed terminal SHA')
        original = read(ROOT / 'evidence' / terminal_name); result = parse(original)
        inputs = parse(read(ROOT / 'input-manifest.json')); proposal = parse(read(ROOT / 'inputs/source-manifest.json'))
        paths = {n: ROOT / n for n in ('input-manifest.json', 'stage.json', 'cargo-cache-manifest.json', 'cargo-cache-stage-complete.json',
                                      'run_cpu.py', 'qualification_support.py', 'supervisor.py')}
        paths.update({'parent_transport.py': Path(stage_path), 'stage_parent_cache.py': Path(cache_path)})
        paths.update({'evidence/' + n: ROOT / 'evidence' / n for n in set(result['raw']) | {terminal_name}})
        paths.update({'inputs/' + n: ROOT / 'inputs' / n for n in inputs['lineage']})
        names = {'ferric/' + r['path'] for r in proposal['files'] if ('ferric/' + r['path']).startswith(PARENT)}
        names.add(PARENT + 'Cargo.lock'); paths.update({n: ROOT / n for n in names})
        bodies = {n: read(p) for n, p in paths.items()}; bodies['evidence.py'] = own
        require(bodies['evidence/' + terminal_name] == original and tree(ROOT / 'evidence') == set(result['raw']) | {terminal_name},
                'complete original evidence directory')
        result = verify(bodies, terminal_name, terminal_sha)
        live = live_rehash(result)
        require(all(read(p) == bodies[n] for n, p in paths.items()) and read(Path(__file__).resolve()) == own, 'selected final posthashes')
        manifest = dict(schema='ferric-readiness40-causal-layer0-parent-cpu-selected-evidence-v2', terminal_name=terminal_name,
            terminal=pin(original), files={n: pin(b) for n, b in sorted(bodies.items())},
            passed=result['passed'], live_rehashed=live, source_body_scope='five parent overlays plus unchanged Cargo.lock',
            all_raw_and_lineage_retained=True, binary_bodies_retained=False, dependency_bodies_retained=False,
            original_receipts_unchanged=True, project_execution=False, gpu_execution=False,
            full_model_acceptance=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
        bodies['manifest.json'] = encoded(manifest)
        require(len(bodies) <= MAX_MEMBERS and sum(map(len, bodies.values())) <= MAX_TOTAL, 'selected archive bounds')
        destination = Path(destination)
        require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
                and not os.path.lexists(destination), 'fresh archive destination')
        guard()
        with destination.open('xb') as stream:
            with tarfile.open(fileobj=stream, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive:
                for n, data in sorted(bodies.items()):
                    guard(); info = tarfile.TarInfo(n); info.size = len(data); info.mode = 0o600; info.mtime = 0
                    archive.addfile(info, io.BytesIO(data))
            stream.flush(); os.fsync(stream.fileno())
        raw = read(destination, MAX_TOTAL)
        require(unpack(raw) == bodies, 'roundtrip exact evidence archive')
        report = dict(archive=pin(raw), members=len(bodies), expanded_bytes=sum(map(len, bodies.values())),
                      terminal=pin(original), passed=result['passed'], live_rehashed=live)
    else:
        archive_path, archive_sha, terminal_sha, destination = sys.argv[2:6]
        require(all(re.fullmatch('[0-9a-f]{64}', s) for s in (archive_sha, terminal_sha)), 'observed archive/terminal SHA')
        raw = read(Path(archive_path), MAX_TOTAL)
        require(pin(raw)['sha256'] == archive_sha, 'actual archive SHA')
        bodies = unpack(raw); manifest_raw = bodies.pop('manifest.json'); manifest = parse(manifest_raw)
        require(manifest['schema'] == 'ferric-readiness40-causal-layer0-parent-cpu-selected-evidence-v2'
                and manifest['terminal_name'] == terminal_name
                and manifest['files'] == {n: pin(b) for n, b in bodies.items()}
                and bodies['evidence.py'] == own, 'closed original selected pin manifest and exact reviewed helper')
        result = verify(bodies, terminal_name, terminal_sha)
        require(manifest['passed'] is result['passed'] and manifest['terminal'] == pin(bodies['evidence/' + terminal_name]), 'manifest actual outcome')
        bodies['manifest.json'] = manifest_raw
        require(read(Path(archive_path), MAX_TOTAL) == raw, 'archive input posthash')
        destination = Path(destination)
        require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent
                and not os.path.lexists(destination), 'fresh retention destination')
        destination.mkdir(mode=0o755)
        for n, data in sorted(bodies.items()):
            guard(); path = destination / relative(n); path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('xb') as stream:
                stream.write(data); stream.flush(); os.fsync(stream.fileno())
            require(read(path) == data, 'retained body mismatch')
        require(read(Path(archive_path), MAX_TOTAL) == raw and read(Path(__file__).resolve()) == own
                and all(read(destination / n) == v for n, v in bodies.items()), 'final retained/input posthashes')
        report = dict(schema='ferric-readiness40-causal-layer0-parent-cpu-retention-v2', archive=pin(raw), terminal=manifest['terminal'],
            files={n: pin(v) for n, v in sorted(bodies.items())}, passed=result['passed'], original_receipts_unchanged=True,
            external_bodies_rehashed_locally=False, project_execution=False, gpu_execution=False,
            full_model_acceptance=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
        guard()
        with (destination / 'retention.json').open('xb') as stream:
            stream.write(encoded(report)); stream.flush(); os.fsync(stream.fileno())
    guard(); print(json.dumps(report if mode == 'export' else dict(destination=str(destination), passed=result['passed'], files=len(bodies))))
    signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()
