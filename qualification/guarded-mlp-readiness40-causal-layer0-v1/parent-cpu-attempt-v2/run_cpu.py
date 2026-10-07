"""Bounded parent-only qualification; no model, worker, or GPU execution."""
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
import tomllib
import types

ROOT = Path(__file__).resolve().parent
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-causal-layer0-parent-cpu-v228-v2')
FERRIC = ROOT / 'ferric'
PARENT_REL = 'adapters/m1-engineering-execution-v1'
PARENT = FERRIC / PARENT_REL
CARGO_HOME = ROOT / 'cargo-home'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
SUPPORT_SHA = '71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'
BASE_COMPLETE = dict(bytes=3901680, sha256='29af2c2ef9e4eeb93bdc0f57a5ae463c2baf751f2824e86430260d8a29ce1f3c')
BASE_SOURCES = dict(bytes=494635, sha256='a8ce9773a765fbb63fa18735818ac7defebb52d5c9b8e34140a1718a4b4042b5')
WORKER_COMPLETE = dict(bytes=1709799, sha256='28e110ab7ebd6d28e9f31499165f364bac729e7e6d0a408b6cb3a369044a340b')
WORKER_SOURCES = dict(bytes=420673, sha256='8672f496167bbe2879a2d2d81fa7cf4d292cb9d91fabc598b7dcf7b6a113c31f')
FIX_PIN = dict(bytes=2203, sha256='e4252b407d036ba8aefdf0a5d3d2235af2427b500242f060aa2a09105471369e')
QUALIFIED_WORKER = dict(bytes=1731960, sha256='5c4ffff4b06c7ad1d73c6985c6bcb0193b596ab6618bbda44010347a9b26aa1a')
QUALIFIED_WORKER_SOURCES = dict(bytes=426647, sha256='00b15b5478ebbeb6fbd00a2de68e24bb6381a6f28a521fc566b95539aa3e35a5')
FAILED_PARENT = dict(bytes=3739856, sha256='060c05edc7ec35703e54c8120fa630553fc7c263205e2b45fdddb0c03a69225b')
PROPOSAL_PIN = dict(bytes=8279, sha256='9cce663d74160516c14033e8417c54dbce666b25e61dc232c11b494c055f558c')
TRANSITIONS_PIN = dict(bytes=4196, sha256='80efcae58f7377d6804630935d0e3c3da93a69da8ee43793e14718a1a3f0a92b')
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
TEST_MODULES = {
    'adapters/m1-engineering-execution-v1/src/tp_finite_client/long/readiness_causal_tests.rs': 'tp_finite_client::long::readiness::causal::tests::',
    'adapters/m1-engineering-execution-v1/src/tp_finite_client/long/readiness_tests.rs': 'tp_finite_client::long::readiness::tests::',
}
LOCK_SHA = 'ec06e964ed72dc9b97d9f5769bd867bf05398781b6e17a9f605137742d6e19ea'
CACHE_PIN = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
PARENT_TOOLS_SHA = 'c38e83e58d01e60b7311efa1ff53cb94e69dbbcd227ee30bde6a04c7928590d9'
PARENT_TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin')
PACKAGE = 'ferric-m1-engineering-execution-v1'
BINARY = 'ferric-qwen3-finite-guarded-mlp-decode-engineering'
ADDED_BINARY = 'ferric-qwen3-guarded-mlp-readiness-engineering'
FEATURE = 'guarded-mlp-readiness-engineering'
OLD_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-warm-paired-terminal-parent-cpu-v228-v3'
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 3600, 1200, 50


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def load_support():
    path = ROOT / 'qualification_support.py'
    require(path.resolve(strict=True) == path and path.stat().st_size <= 65536, 'ordinary support')
    body = path.read_bytes()
    require(hashlib.sha256(body).hexdigest() == SUPPORT_SHA, 'reviewed worker parsing support')
    module = types.ModuleType('parent_qualification_support')
    module.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), module.__dict__)
    module.ROOT, module.WORKER = ROOT, PARENT
    module.OUT, module.TARGET, module.TMP = OUT, TARGET, TMP
    return module


def snapshot(h):
    paths = [ROOT / name for name in ('run_cpu.py', 'qualification_support.py', 'supervisor.py')]
    paths += h.files_below(FERRIC)
    require(1000 <= len(paths) <= 3000, 'bounded parent source closure')
    rows = {str(p.relative_to(ROOT)): h.pin(p) for p in sorted(paths)}
    require(len(rows) == len(paths) and sum(p['bytes'] for p in rows.values()) <= 64 << 20,
            'parent source extent')
    return rows


def configurations(h):
    directories = {ROOT, FERRIC, PARENT, CARGO_HOME, *ROOT.parents, *PARENT.parents}
    directories |= {p.parent for p in FERRIC.rglob('Cargo.toml')}
    paths = {d / '.cargo' / n for d in directories for n in ('config', 'config.toml')}
    paths |= {CARGO_HOME / n for n in ('config', 'config.toml')}
    paths |= {Path('/home/harmenon/.cargo') / n for n in ('config', 'config.toml')}
    result = {str(p): h.pin(p) if os.path.lexists(p) else None for p in sorted(paths)}
    require(not any(result.values()), 'inherited Cargo configuration refused')
    return result


def read(h, name, expected, readset):
    path = ROOT / 'inputs' / name
    require(path.resolve(strict=True) == path, 'lineage alias')
    pin = h.pin(path)
    require(compact(pin) == expected and pin['bytes'] <= 16 << 20, 'lineage identity ' + name)
    body = path.read_bytes()
    require(h.pin(path) == pin, 'lineage changed while reading')
    readset[name] = pin
    return body


def lineage(h, core, inputs, before, readset):
    require(PROPOSAL_PIN is not None, 'reviewed composed source pin pending')
    fixed = {'parent-complete.json': BASE_COMPLETE, 'parent-sources.json': BASE_SOURCES,
             'worker-complete.json': WORKER_COMPLETE, 'worker-sources.json': WORKER_SOURCES,
             'source-manifest.json': PROPOSAL_PIN, 'parent-worker-transitions.json': TRANSITIONS_PIN,
             'parent-read-fix.json': FIX_PIN, 'qualified-worker-complete.json': QUALIFIED_WORKER,
             'qualified-worker-sources.json': QUALIFIED_WORKER_SOURCES, 'parent-v1-failed.json': FAILED_PARENT}
    bodies = {name: read(h, name, row, readset) for name, row in inputs['lineage'].items()}
    require(all(compact(readset[name]) == row for name, row in fixed.items()), 'literal actual baseline and source pins')
    base = json.loads(bodies['parent-complete.json'])
    old_map = json.loads(bodies['parent-sources.json'])
    worker = json.loads(bodies['worker-complete.json'])
    worker_map = json.loads(bodies['worker-sources.json'])
    proposal = json.loads(bodies['source-manifest.json'])
    transitions_doc = json.loads(bodies['parent-worker-transitions.json'])
    require(proposal['schema'] == 'ferric-readiness40-causal-layer0-source-proposal-v1'
            and len(proposal['files']) == 14 and proposal['canonical_base'] == '7dcd40f42018c2bd9ae04eaab47d102246f1d15e'
            and proposal['native'] == dict(forwards=40, generated_tokens=0,
                selected_observations=[0, 5, 16, 39], diagnostic_positions=list(range(6)),
                layer=0, parts_per_position=34, payload_bytes=1598256,
                metadata_bound=131072, stderr_bound=2097152)
            and proposal['source_only'] is True
            and all(proposal[key] is False for key in ('kernel_or_image_bytes_changed',
                'framework_model_input_substitution', 'project_execution', 'tests_executed',
                'native_execution', 'numerical_acceptance', 'performance_claim', 'execution_ready_launcher')),
            'reviewed bounded causal layer-zero source only')
    fix = json.loads(bodies['parent-read-fix.json'])
    require(fix['schema'] == 'ferric-readiness40-causal-layer0-parent-read-fix-v2'
            and fix['predecessor_source'] == PROPOSAL_PIN
            and len(fix['files']) == 3 and all(fix[key] is False for key in
                ('worker_source_changed', 'runtime_source_changed', 'wire_or_image_changed',
                 'limits_changed', 'admission_weakened', 'tests_executed', 'native_execution')),
            'reviewed narrow parent byte-retention correction')
    qualified = json.loads(bodies['qualified-worker-complete.json'])
    qualified_map = json.loads(bodies['qualified-worker-sources.json'])
    require(qualified['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-worker-cpu-v1'
            and qualified['passed'] is True and qualified['failure'] is None and qualified['postcheck_errors'] == []
            and qualified['input_sources'] == qualified['final_sources'] == qualified_map
            and len(qualified_map) == 1014 and qualified['source_unchanged'] is True
            and compact(qualified['raw']['sources-after.json']) == QUALIFIED_WORKER_SOURCES
            and qualified['tests']['worker-tests']['passed'] == 680
            and qualified['tests']['worker-tests']['failed'] == 0
            and qualified['tests']['worker-tests']['ignored'] == 4
            and qualified['cli_executable_unchanged_across_tests'] is True
            and qualified['gpu_execution'] is False, 'actual unchanged causal worker qualification')
    authored_worker = {n: compact(r) for n, r in qualified['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
    actual_causal_worker = {n: compact(r) for n, r in qualified_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(authored_worker) == len(actual_causal_worker) == 197
            and set(authored_worker) == set(actual_causal_worker), 'closed197 current worker files')
    qualified_changes = {n for n in authored_worker if authored_worker[n] != actual_causal_worker[n]}
    require(qualified_changes == set(qualified['format_changed_paths'])
            and qualified_changes <= {'ferric/' + r['path'] for r in proposal['files']
                                     if ('ferric/' + r['path']).startswith(WORKER_PREFIX)},
            'separate actual worker formatting remains explicit')
    failed = json.loads(bodies['parent-v1-failed.json'])
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
        require(row['path'].startswith(PARENT_REL + '/') and row['path'] in original_rows
                and original_rows[row['path']]['after'] == row['before'], 'exact V1 authored correction preimage')
        original_rows[row['path']]['after'] = row['after']
    extra = fix['new_tests']['parent_library']
    require(extra == ['tp_finite_client::long::readiness::causal::tests::causal_parent_file_backed_sidecar_reads_and_rechecks_pinned_bytes'],
            'one exact file-backed regression')
    proposal['new_tests']['parent_library'] += [extra[0].rsplit('::', 1)[-1]]
    require(transitions_doc['parent_source'] == BASE_SOURCES and transitions_doc['worker_source'] == WORKER_SOURCES
            and transitions_doc['worker_files'] == 195, 'actual parent/worker transition provenance')
    require(base['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-parent-cpu-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['input_sources'] == base['final_sources'] == old_map
            and len(old_map) == 1240 and len(base['phases']) == 60 and len(base['tests']) == 51
            and len(base['inventory']) == 913 and sum(v['passed'] for v in base['tests'].values()) == 438
            and base['gpu_execution'] is False, 'actual selected parent predecessor')
    require(worker['schema'] == 'ferric-guarded-mlp-warm-paired-terminal-worker-cpu-v1'
            and worker['passed'] is True and worker['failure'] is None and worker['postcheck_errors'] == []
            and worker['source_unchanged'] is True and worker['input_sources'] == worker['final_sources'] == worker_map
            and len(worker_map) == 1012 and worker['gpu_execution'] is False
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['tests']['worker-tests']['passed'] == 673
            and worker['tests']['worker-tests']['failed'] == 0
            and worker['tests']['worker-tests']['ignored'] == 4, 'actual worker source baseline only')
    for result in (base, worker):
        require(all(p['exit_code'] == 0 and p['natural_exit'] is True and p['reaped'] is True
                    and p['process_group_absent'] is True and p['forced_cleanup'] is False
                    and p['timed_out'] is False and p['exception'] is None and p['storage_failure'] is None
                    for p in result['phases']), 'clean original CPU lifecycles')
    require(compact(base['raw']['sources-after.json']) == BASE_SOURCES
            and compact(worker['raw']['sources-after.json']) == WORKER_SOURCES, 'actual source-map joins')
    scopes = base['tests']
    expected_lineage = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    expected_lineage |= {name + suffix for name in scopes for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == expected_lineage and len(bodies) == 114, 'closed actual114 lineage including original failure')
    for name in expected_lineage - set(fixed):
        require(compact(readset[name]) == compact(base['raw'][name]), 'actual baseline raw ' + name)
    listed = core.inventory(bodies['parent-lib-list.stdout'].decode())
    require(listed == base['inventory'], 'actual913-name parent inventory')
    for name, prior in scopes.items():
        actual = core.outcomes(bodies[name + '.stdout'].decode())
        require(actual == prior and prior['failed'] == prior['ignored'] == 0, 'preserved raw selected outcomes ' + name)
        command = json.loads(bodies[name + '.command.json'])
        phase = next(p for p in base['phases'] if p['label'] == name)
        require(command['argv'] == phase['argv'], 'actual selected command join')
    expected = {name: compact(row) for name, row in old_map.items() if name.startswith('ferric/')}
    actual_worker = {name: compact(row) for name, row in worker_map.items() if name.startswith(WORKER_PREFIX)}
    require(len(expected) == 1237 and len(actual_worker) == 195
            and set(actual_worker) <= set(expected), 'unchanged standalone dependency/source boundaries')
    transitions = {'ferric/' + row['path']: row for row in transitions_doc['rows']}
    require(len(transitions) == len(transitions_doc['rows']) == 10
            and set(transitions) == {n for n in actual_worker if expected[n] != actual_worker[n]}
            and all(row['before'] == expected[n] and row['after'] == actual_worker[n]
                    for n, row in transitions.items()), 'exact ten authenticated inherited worker formatter transitions')
    expected.update(actual_worker)
    rows = {'ferric/' + row['path']: row for row in proposal['files']}
    require(len(proposal['files']) == len(rows) == 14
            and sum(name.startswith(WORKER_PREFIX) for name in rows) == 9
            and sum(name.startswith('ferric/' + PARENT_REL + '/') for name in rows) == 5,
            'closed nine worker/five parent source overlays')
    require(sum(row['before'] is None for row in rows.values()) == 4, 'four exact source additions')
    for name, row in rows.items():
        require(expected.get(name) == row['before'], 'actual composed source preimage ' + name)
        expected[name] = row['after']
    require({n: r for n, r in expected.items() if n.startswith(WORKER_PREFIX)} == authored_worker,
            'parent compiles actual causal worker authored preformat bodies')
    require({name: compact(row) for name, row in before.items() if name.startswith('ferric/')} == expected
            and len(before) == 1244, 'entire source map from actual parent and worker plus reviewed composition')
    formatting = sorted(name for name in rows if name.startswith('ferric/' + PARENT_REL + '/') and name.endswith('.rs'))
    require(inputs['parent_overlay'] == formatting and len(formatting) == 5, 'five parent Rust formatting inputs')
    require(h.pin(PARENT / 'Cargo.lock')['sha256'] == LOCK_SHA, 'unchanged parent lock')
    declared = proposal['new_tests']['parent_library']
    additions = []
    for path, prefix in TEST_MODULES.items():
        source = (FERRIC / path).read_text()
        names = re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source)
        additions += [prefix + name for name in names if name in declared]
    additions.sort()
    bin_names = ['tests::' + name for name in proposal['new_tests']['parent_readiness_binary']]
    bin_path = PARENT_REL + '/src/bin/' + ADDED_BINARY + '.rs'
    source = (FERRIC / bin_path).read_text()
    binary_methods = {'tests::' + name for name in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source)}
    require(len(additions) == len(set(additions)) == 5 and not set(additions) & set(listed)
            and sorted(n.rsplit('::', 1)[-1] for n in additions) == sorted(declared)
            and bin_names == ['tests::causal_parent_cli_has_a_separate_closed_mode']
            and set(bin_names) <= binary_methods,
            'five source-declared library additions and one readiness binary name')
    parent_tools = base['parent_toolchain_observation']
    require(parent_tools['schema'] == 'ferric-guarded-mlp-parent-toolchain-v1'
            and parent_tools['toolchain'] == str(PARENT_TOOLCHAIN)
            and parent_tools['tool_pins'] == base['tool_pins'] and len(base['tool_pins']) == 8,
            'actual unchanged parent toolchain')
    for row in parent_tools['tool_pins'].values():
        require(h.pin(Path(row['path'])) == row, 'parent tool drift')
    return base, parent_tools, proposal, bodies, scopes, listed, additions, bin_names


def relocate(value):
    if isinstance(value, str):
        return value.replace(OLD_ROOT, str(ROOT))
    if isinstance(value, list):
        return [relocate(v) for v in value]
    if isinstance(value, dict):
        return {k: relocate(v) for k, v in value.items()}
    return value


def metadata_contract(h, old):
    current = json.loads((OUT / 'metadata.stdout').read_bytes())
    require(current == relocate(old), 'unchanged complete locked parent metadata and features/targets')
    require(current['workspace_root'] == str(PARENT) and current['target_directory'] == str(TARGET)
            and len(current['packages']) == 209, 'standalone parent metadata')
    external = {}
    external_packages = 0
    for row in current['packages']:
        path = Path(row['manifest_path'])
        require(path.resolve(strict=True) == path, 'dependency ordinary path')
        if row['source'] is None:
            require(path.is_relative_to(FERRIC), 'local dependency escapes source closure')
        else:
            require(path.is_relative_to(CARGO_HOME), 'external dependency escapes private cache')
            external_packages += 1
            root = path.parent
            if row['source'].startswith('git+'):
                parts = path.relative_to(CARGO_HOME).parts
                require(parts[:2] == ('git', 'checkouts') and len(parts) >= 5, 'private Git checkout source')
                root = CARGO_HOME.joinpath(*parts[:4])
            if str(root) not in external:
                external[str(root)] = dependency_snapshot(h, root)
    require(external_packages == 181 and len(external) == 121,
            '181 external packages in118 registry plus3 complete Git source roots')
    return current, external


def dependency_snapshot(h, root):
    return {str(p.relative_to(root)): h.pin(p) for p in h.files_below(root, packed=False)
            if '.git' not in p.relative_to(root).parts}


def cache_contract(h, inputs):
    path = ROOT / 'cargo-cache-manifest.json'
    row = h.pin(path)
    require(compact(row) == inputs['cache_manifest'] == CACHE_PIN, 'root-bound private parent cache')
    value = json.loads(path.read_bytes())
    stage_path = ROOT / 'cargo-cache-stage-complete.json'
    stage_pin = h.pin(stage_path)
    require(stage_pin['bytes'] <= 1 << 20, 'bounded private cache stage receipt')
    stage = json.loads(stage_path.read_bytes())
    require(value['schema'] == 'ferric-guarded-mlp-parent-cache-v1'
            and value['lock'] == compact(h.pin(PARENT / 'Cargo.lock'))
            and len(value['packages']) == 118 and len(value['files']) == 247
            and len(value['git_commits']) == 3 and value['cargo_execution'] is False,
            'closed parent cache lock/packages')
    lock = tomllib.loads((PARENT / 'Cargo.lock').read_text())['package']
    registry = sorted((p['name'], p['version'], p['checksum']) for p in lock
                      if p.get('source', '').startswith('registry+'))
    require(registry == sorted((p['name'], p['version'], p['checksum']) for p in value['packages'])
            and {p['source'] for p in lock if p.get('source', '').startswith('git+')}
                == {p['source'] for p in value['git_commits']}, 'cache exact locked package/revision roster')
    require(stage['schema'] == 'ferric-guarded-mlp-parent-cache-stage-v1'
            and stage['passed'] is True and stage['manifest'] == compact(row)
            and stage['cargo_home'] == str(CARGO_HOME) and stage['lock'] == value['lock']
            and stage['files'] == value['files'] and stage['git_commits'] == value['git_commits']
            and stage['packages'] == 118 and stage['cache_files'] == 247
            and stage['inner_members'] == value['inner_members'] == 5023
            and stage['inner_expanded_bytes'] == value['inner_expanded_bytes'] == 85992791
            and stage['git_objects'] == value['git_objects'] == 7723
            and stage['git_object_bytes'] == value['git_object_bytes'] == 105759011
            and all(stage[k] is False for k in ('shared_cache_changed', 'lock_changed',
                'project_code_executed', 'crate_sources_extracted', 'git_sources_checked_out')),
            'authenticated cache staging contract')
    files = {}
    for name, expected in value['files'].items():
        require(type(name) is str and Path(name).as_posix() == name and not name.startswith('/')
                and '..' not in Path(name).parts, 'ordinary cache input')
        actual = h.pin(CARGO_HOME / name)
        require(compact(actual) == expected, 'private cache input changed')
        files[name] = actual
    require(h.pin(path) == row and h.pin(stage_path) == stage_pin, 'cache provenance changed during read')
    return dict(manifest=row, stage=stage_pin, files=files)


def main():
    started = time.monotonic()
    deadline = started + WHOLE_WALL
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1])
            and PARENT_TOOLS_SHA is not None, 'python3 -B run_cpu.py INPUT_SHA; bind actual parent tools first')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and not any(os.path.lexists(p) for p in (OUT, TARGET, TMP)), 'fresh exact parent root')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'parent qualification host/UID')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, 'initial free floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    nice = os.getpriority(os.PRIO_PROCESS, 0)
    require(nice in (0, 10), 'parent nice')
    if nice == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30)):
        old = resource.getrlimit(kind)
        limit = min([cap] + [v for v in old if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    core = load_support()
    h = core.load_supervisor()
    h.TOOLCHAIN, h.TOOLCHAIN_LIB = PARENT_TOOLCHAIN, PARENT_TOOLCHAIN.parent / 'lib'
    h.sources = lambda: snapshot(h)
    original_scratch = h.scratch_bytes
    def scratch():
        paths = h.files_below(CARGO_HOME, packed=False)
        require(len(paths) <= 150000, 'bounded private cache files')
        total = original_scratch()
        for p in paths:
            try:
                total += p.lstat().st_size
            except FileNotFoundError:
                pass
        return total
    h.scratch_bytes = scratch
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - CLEANUP_RESERVE - time.monotonic()))
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    phases, tests, artifacts, readset, tool_pins, dependencies, errors = [], {}, {}, {}, {}, {}, []
    inputs = input_pin = before = formatted = after = config = cache = metadata = parent_tools = None
    failure = None
    changed, listed = [], []
    try:
        input_pin = h.pin(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'root-bound parent input')
        inputs = json.loads((ROOT / 'input-manifest.json').read_bytes())
        require(set(inputs) == {'schema', 'files', 'lineage', 'parent_overlay', 'cache_manifest', 'git_revision'}
                and inputs['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-parent-cpu-input-v2', 'closed parent input')
        before = snapshot(h)
        require(inputs['files'] == {k: compact(v) for k, v in before.items()}, 'exact parent source input')
        h.save('sources-preformat.json', before)
        base, parent_tools, proposal, bodies, scopes, old_list, additions, bin_names = lineage(h, core, inputs, before, readset)
        h.SHARED_LIBRARIES = {name: row['bytes'] for name, row in parent_tools['tool_pins'].items()
                              if name not in {'rustc', 'rustdoc', 'rustfmt', 'cargo', 'prlimit'}}
        config = configurations(h)
        cache = cache_contract(h, inputs)
        require({str(p.relative_to(CARGO_HOME)) for p in h.files_below(CARGO_HOME, packed=False)}
                == set(cache['files']), 'fresh private cache contains only staged immutable inputs')
        tool_pins = {name: h.pin(h.TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
        tool_pins['prlimit'] = h.pin(Path('/usr/bin/prlimit'))
        tool_pins.update({name: h.pin(h.TOOLCHAIN_LIB / name) for name in h.SHARED_LIBRARIES})
        require(tool_pins == parent_tools['tool_pins'], 'actual bound parent1.97.1 toolchain')
        env = dict(HOME='/home/harmenon', PATH=str(h.TOOLCHAIN) + ':/usr/bin:/bin',
            CARGO_HOME=str(CARGO_HOME), CARGO_TARGET_DIR=str(TARGET), TMPDIR=str(TMP),
            LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(h.TOOLCHAIN_LIB),
            RUSTC=str(h.TOOLCHAIN / 'rustc'), RUSTDOC=str(h.TOOLCHAIN / 'rustdoc'),
            CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
            CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never', MALLOC_ARENA_MAX='2', RUST_BACKTRACE='1',
            RUSTC_BOOTSTRAP='fe2o3_device,fe2o3_macros',
            CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0',
            CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
            CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
            CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
            ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        def leaf(name, argv, seconds=LEAF_WALL, source=formatted):
            h.run(name, argv, env, phases, deadline, source, seconds, PARENT)
        fmt = [str(h.TOOLCHAIN / 'rustfmt'), '--edition', '2024', '--config', 'skip_children=true']
        paths = [str(ROOT / n) for n in inputs['parent_overlay']]
        leaf('rustfmt', fmt + paths, 120)
        formatted = snapshot(h)
        require(set(formatted) == set(before), 'formatter source roster')
        changed = [n for n in formatted if formatted[n] != before[n]]
        require(set(changed) <= set(inputs['parent_overlay']), 'formatter scope')
        h.save('sources-before.json', formatted)
        leaf('rustfmt-check', fmt + ['--check'] + paths, 120, formatted)
        leaf('rustc-version', [str(h.TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60, formatted)
        require((OUT / 'rustc-version.stdout').read_text() == parent_tools['rustc_version'],
                'current parent rustc version differs from its actual observation')
        cargo = str(h.TOOLCHAIN / 'cargo')
        common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(PARENT / 'Cargo.toml')]
        feature = ['--features', FEATURE]
        leaf('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path', str(PARENT / 'Cargo.toml'),
                         *feature, '--format-version', '1'], 120, formatted)
        metadata, dependencies = metadata_contract(h, json.loads(bodies['metadata.stdout']))
        h.save('dependencies-before.json', dependencies)
        leaf('parent-lib-list', [cargo, 'test', *common, *feature, '--lib', '--', '--list', '--format=terse'], source=formatted)
        listed = core.inventory((OUT / 'parent-lib-list.stdout').read_text())
        require(listed == sorted(old_list + additions) and len(listed) == 918, 'old913 plus exact5 library names')
        selected = set()
        list_labels = {'guarded-bin-list': 'guarded-bin-tests', 'readiness-bin-list': 'readiness-bin-tests'}
        ordered_scopes = [p['label'] for p in base['phases'] if p['label'] in scopes or p['label'] in list_labels]
        require(len(ordered_scopes) == 53 and set(ordered_scopes) == set(scopes) | set(list_labels),
                'all51 original selected and two inventory recipes')
        for label in ordered_scopes:
            original = next(p for p in base['phases'] if p['label'] == label)
            argv = relocate(original['argv'][6:])
            require(argv[:2] == [cargo, 'test'] and argv[argv.index('--features') + 1] == FEATURE,
                    'unchanged original owned Cargo recipe')
            leaf(label, argv, source=formatted)
            if label in list_labels:
                names = set(core.statuses(scopes[list_labels[label]]))
                if label == 'readiness-bin-list':
                    names |= set(bin_names)
                require(core.inventory((OUT / (label + '.stdout')).read_text()) == sorted(names),
                        'preserved named binary inventory plus explicit addition')
                continue
            names = set(core.statuses(scopes[label]))
            if '--lib' in argv:
                selector = argv[argv.index('--lib') + 1]
                names |= {n for n in additions if n == selector or ('--exact' not in argv and selector in n)}
                require(not selected & names, 'inherited library selections overlap')
                selected |= names
            elif label == 'readiness-bin-tests':
                names |= set(bin_names)
            value = core.outcomes((OUT / (label + '.stdout')).read_text())
            require(core.statuses(value) == {n: 'ok' for n in names}
                    and value['failed'] == value['ignored'] == 0, 'all inherited and added named outcomes ' + label)
            tests[label] = value
        require(set(additions) <= selected, 'every new library name selected')
        old_bins = sorted(base['artifacts'])
        require(len(old_bins) == 6 and BINARY in old_bins, 'six inherited parent products')
        args = [item for n in old_bins for item in ('--bin', n)]
        leaf('parent-builds', [cargo, 'build', '--profile', 'test', *common, *feature, *args, '--message-format=json'], source=formatted)
        records = h.build_records(OUT / 'parent-builds.stdout')
        for name in old_bins:
            candidates = [r for r in records if r.get('reason') == 'compiler-artifact'
                and r.get('manifest_path') == str(PARENT / 'Cargo.toml') and r.get('target', {}).get('name') == name
                and r['target']['kind'] == ['bin'] and r.get('profile', {}).get('test') is False]
            require(len(candidates) == 1, 'exact selected parent product')
            row = candidates[0]
            path = Path(row['executable'])
            require(path.resolve(strict=True) == path and path.is_relative_to(TARGET)
                    and FEATURE in row['features'] and row['filenames'].count(str(path)) == 1, 'new target artifact')
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'parent ELF')
            artifacts[name] = dict(pin=h.pin(path), cargo_artifact=row)
        leaf('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], source=formatted)
        require(len(phases) == 60 and len(tests) == 51 and sum(v['passed'] for v in tests.values()) == 444
                and sum(v['failed'] + v['ignored'] for v in tests.values()) == 0, '60 phases and444 selected passes')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(name, action):
        try:
            require(deadline - time.monotonic() > 5, 'postcheck reserve')
            signal.setitimer(signal.ITIMER_REAL, deadline - time.monotonic() - 5)
            action()
        except BaseException as error:
            errors.append(name + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    def source_check():
        nonlocal after
        after = snapshot(h)
        h.save('sources-after.json', after)
        if formatted is not None:
            require(after == formatted, 'parent source/lock changed')
        elif before is not None:
            require(set(after) == set(before) and all(after[n] == r for n, r in before.items()
                if n not in (inputs or {}).get('parent_overlay', [])), 'failed formatting escaped scope')
    check('sources', source_check)
    if config is not None:
        check('configuration', lambda: require(configurations(h) == config, 'Cargo config drift'))
    if cache is not None:
        check('cache', lambda: require(cache_contract(h, inputs) == cache, 'immutable cache drift'))
    if input_pin is not None:
        check('input', lambda: require(h.pin(ROOT / 'input-manifest.json') == input_pin, 'input drift'))
    for name, row in dict(readset, **tool_pins).items():
        check('input ' + name, lambda r=row: require(h.pin(Path(r['path'])) == r, 'readset drift'))
    for directory, expected in dependencies.items():
        check('dependency ' + directory, lambda d=directory, e=expected: require(
            dependency_snapshot(h, Path(d)) == e, 'dependency drift'))
    for name, row in artifacts.items():
        check('artifact ' + name, lambda r=row: require(h.pin(Path(r['pin']['path'])) == r['pin'], 'artifact drift'))
    check('scratch', lambda: require(h.scratch_bytes() <= h.CACHE_LIMIT, 'final scratch cap'))
    raw = {}
    def raw_check():
        nonlocal raw
        raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw.get(Path(row[key]['path']).name) == row[key], 'raw phase join')
    check('raw evidence', raw_check)
    failure = failure or ('postcheck failed' if errors else None)
    if time.monotonic() >= deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-guarded-mlp-readiness40-causal-layer0-parent-cpu-v2', passed=failure is None, failure=failure,
        postcheck_errors=errors, controller=h.pin(ROOT / 'run_cpu.py'), supervisor=h.pin(ROOT / 'supervisor.py'),
        input_manifest=input_pin, readset=readset, git_revision=inputs['git_revision'] if inputs else None,
        preformat_sources=before, input_sources=formatted, final_sources=after,
        source_unchanged=formatted is not None and after == formatted, format_changed_paths=changed,
        phases=phases, tests=tests, inventory=listed, artifacts=artifacts, metadata=metadata,
        tool_pins=tool_pins, parent_toolchain_observation=parent_tools,
        cache_provenance=cache, configurations=config, raw=raw,
        elapsed_seconds=time.monotonic() - started,
        limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL, cleanup_reserve_seconds=CLEANUP_RESERVE,
            address_space_bytes=h.AS_LIMIT, cache_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT,
            initial_free_bytes=h.START_FREE, live_free_bytes=h.LIVE_FREE, affinity=[8, 9], cargo_jobs=2),
        full_parent_library_suite_executed=False,
        all_selected_parent_tests_executed=len(tests) == 51 and sum(v['passed'] for v in tests.values()) == 444,
        causal_layer0_parent_route_added=True, causal_layer0_native_execution=False,
        parent_causal_file_reads_retained=True, previous_parent_failure_preserved=True,
        causal_capture_positions=list(range(6)), causal_layer=0,
        inherited_warm_paired_terminal_parent_route_preserved=True,
        warm_paired_terminal_parent_route_added=True, warm_paired_terminal_native_execution=False,
        selected_terminal_cadence_changed=True, global_currentness_policy_changed=False,
        hidden_read_policy_changed=False,
        position5_diagnostic_parent_route_added=True, position5_native_execution=False,
        inherited_readiness_parent_route_preserved=True, readiness_native_execution=False,
        inherited_paired_read_parent_route_preserved=True, paired_read_native_execution=False,
        method_local_currentness_cadence_changed=True, read_ns_scope_changed=False,
        inherited_reusable_arena_parent_route_added=True, reusable_arena_native_execution=False,
        default_arena_policy_changed=False, full_long_workload=False,
        inherited_shared_full_currentness_source_added=True,
        inherited_host_observation_source_added=True, inherited_capture_source_added=True,
        performance_policy_changed=False, default_policy_changed=False,
        runtime_suite_rerun=False, worker_suite_rerun=False, gpu_execution=False,
        full_model_acceptance=False, numerical_acceptance=False, performance_claim=False, production_authority=False)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
    try:
        h.save('complete.json' if failure is None else 'failed.json', result)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=failure is None, failure=failure, phases=len(phases),
        tests_passed=sum(v['passed'] for v in tests.values()), output=str(OUT)), sort_keys=True))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())

