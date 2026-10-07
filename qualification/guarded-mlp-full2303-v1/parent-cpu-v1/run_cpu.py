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
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-parent-cpu-v228-v1')
FERRIC = ROOT / 'ferric'
PARENT_REL = 'adapters/m1-engineering-execution-v1'
PARENT = FERRIC / PARENT_REL
CARGO_HOME = ROOT / 'cargo-home'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
SUPPORT_SHA = '71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'
BASE_COMPLETE = dict(bytes=3942470, sha256='f2c161753e64f9f2b8b4049ae75cac13332b7eb14217eaad247c8c2e9e13d4ea')
BASE_SOURCES = dict(bytes=502636, sha256='54b516e6cd3d95973c725923d69d1fd7b9e6fa4ff7b3d4f73272282ed3ba2ae2')
WORKER_COMPLETE = dict(bytes=1731960, sha256='5c4ffff4b06c7ad1d73c6985c6bcb0193b596ab6618bbda44010347a9b26aa1a')
WORKER_SOURCES = dict(bytes=426647, sha256='00b15b5478ebbeb6fbd00a2de68e24bb6381a6f28a521fc566b95539aa3e35a5')
QUALIFIED_WORKER = dict(bytes=1691615, sha256='0b91b220824f093b0297134ff21e4ddc2dde64fd17909439bb15421fb862e55f')
QUALIFIED_WORKER_SOURCES = dict(bytes=412019, sha256='0fc0a33c61dd0e6f70903bb246654bfeb99095fd515be53c828d8d91f646e7cb')
PROPOSAL_PIN = dict(bytes=4291, sha256='e29ee6f6fd37ede4f9b4c990e0427bb287ce8e066de01f136fb4ab82961f6793')
WORKER_PROPOSAL_PIN = dict(bytes=11547, sha256='8472f1749c10257cadb01f585c18e39fb76c195381ded1408be8254a01627a6b')
WORKER_PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
LOCK_SHA = 'ec06e964ed72dc9b97d9f5769bd867bf05398781b6e17a9f605137742d6e19ea'
CACHE_PIN = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
PARENT_TOOLS_SHA = 'c38e83e58d01e60b7311efa1ff53cb94e69dbbcd227ee30bde6a04c7928590d9'
PARENT_TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin')
PACKAGE = 'ferric-m1-engineering-execution-v1'
BINARY = 'ferric-qwen3-finite-guarded-mlp-decode-engineering'
ADDED_BINARY = 'ferric-qwen3-guarded-mlp-full2303-engineering'
OLD_FEATURE = 'guarded-mlp-readiness-engineering'
FULL_FEATURE = 'guarded-mlp-full2303-engineering'
FEATURE = OLD_FEATURE + ',' + FULL_FEATURE
OLD_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-causal-layer0-parent-cpu-v228-v2'
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
    require(QUALIFIED_WORKER is not None and QUALIFIED_WORKER_SOURCES is not None,
            'actual Full worker receipt and source map are pending')
    fixed = {'parent-complete.json': BASE_COMPLETE, 'parent-sources.json': BASE_SOURCES,
             'worker-complete.json': WORKER_COMPLETE, 'worker-sources.json': WORKER_SOURCES,
             'qualified-worker-complete.json': QUALIFIED_WORKER,
             'qualified-worker-sources.json': QUALIFIED_WORKER_SOURCES,
             'parent-source-manifest.json': PROPOSAL_PIN, 'worker-source-manifest.json': WORKER_PROPOSAL_PIN}
    bodies = {name: read(h, name, row, readset) for name, row in inputs['lineage'].items()}
    require(all(compact(readset[name]) == row for name, row in fixed.items()),
            'literal actual baseline and source pins')
    base = json.loads(bodies['parent-complete.json'])
    old_map = json.loads(bodies['parent-sources.json'])
    worker = json.loads(bodies['worker-complete.json'])
    worker_map = json.loads(bodies['worker-sources.json'])
    qualified = json.loads(bodies['qualified-worker-complete.json'])
    qualified_map = json.loads(bodies['qualified-worker-sources.json'])
    proposal = json.loads(bodies['parent-source-manifest.json'])
    worker_proposal = json.loads(bodies['worker-source-manifest.json'])
    require(base['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-parent-cpu-v2'
            and len(old_map) == 1244 and len(base['phases']) == 60 and len(base['tests']) == 51
            and len(base['inventory']) == 918 and sum(v['passed'] for v in base['tests'].values()) == 444
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['parent_causal_file_reads_retained'] is True,
            'actual selected causal parent V2 baseline')
    require(worker['schema'] == 'ferric-guarded-mlp-readiness40-causal-layer0-worker-cpu-v1'
            and len(worker_map) == 1014 and len(worker['phases']) == 9
            and len(worker['inventory']) == 684
            and worker['tests']['worker-tests']['passed'] == 680
            and worker['tests']['worker-tests']['failed'] == 0
            and worker['tests']['worker-tests']['ignored'] == 4
            and worker['cli_executable_unchanged_across_tests'] is True, 'actual causal worker baseline')
    require(qualified['schema'] == 'ferric-guarded-mlp-full2303-worker-cpu-v1'
            and len(qualified_map) == 1020 and len(qualified['phases']) == 9
            and len(qualified['inventory']) == 698
            and qualified['tests']['worker-tests']['passed'] == 694
            and qualified['tests']['worker-tests']['failed'] == 0
            and qualified['tests']['worker-tests']['ignored'] == 4
            and qualified['full2303_source_added'] is True
            and qualified['full2303_native_execution'] is False
            and qualified['full2303_launch_feasibility'] is False
            and qualified['full2303_source_abort_ms'] == 3600000
            and qualified['inherited_causal_layer0_route_preserved'] is True
            and qualified['cli_executable_unchanged_across_tests'] is True
            and qualified['cli_executable_before_tests']['pin'] == qualified['artifacts']['worker']['pin']
            and set(qualified['artifacts']) == {'worker-lib', 'worker-bin-test', 'worker-wire-test',
                'worker-readiness-test', 'worker'}
            and len({r['pin']['path'] for r in qualified['artifacts'].values()}) == 5,
            'observed full-worker CPU/real-executable contract')
    for result, source_map, expected_pin in ((base, old_map, BASE_SOURCES),
            (worker, worker_map, WORKER_SOURCES), (qualified, qualified_map, QUALIFIED_WORKER_SOURCES)):
        require(result['passed'] is True and result['failure'] is None and result['postcheck_errors'] == []
                and result['input_sources'] == result['final_sources'] == source_map
                and result['source_unchanged'] is True and result['gpu_execution'] is False
                and compact(result['raw']['sources-after.json']) == expected_pin
                and all(p['exit_code'] == 0 and p['natural_exit'] is True and p['reaped'] is True
                    and p['process_group_absent'] is True and p['forced_cleanup'] is False
                    and p['timed_out'] is False and p['exception'] is None and p['storage_failure'] is None
                    for p in result['phases']), 'actual clean source/CPU lifecycle')
    require(proposal['schema'] == 'ferric-guarded-full2303-parent-source-proposal-v1'
            and proposal['source_only'] is True
            and proposal['base_revision'] == 'b61588712af31738d3a744fae81b6afcfdd9ea9a'
            and all(v is False for v in proposal['claims'].values())
            and proposal['unchanged_contracts'] == dict(pure_long_wire=True, pure_long_sequence=True,
                readiness40_native_entries=True, ar4_entries=True, stream_bytes=67108864,
                evidence_bytes=33554432, full_deadline_ms=3600000, launch_feasibility_established=False),
            'frozen parent source scope, not launch authority')
    require(worker_proposal['schema'] == 'ferric-guarded-mlp-full2303-worker-source-v1'
            and worker_proposal['source_only'] is True
            and compact(worker_proposal['base']['worker_complete']) == WORKER_COMPLETE
            and compact(worker_proposal['base']['worker_sources']) == WORKER_SOURCES
            and worker_proposal['canonical_commit'] == proposal['base_revision']
            and all(worker_proposal[k] is False for k in ('tests_executed', 'native_execution',
                'launch_feasibility', 'numerical_acceptance', 'performance_claim',
                'currentness_policy_changed', 'existing_ar4_cap_changed', 'existing_readiness_cap_changed')),
            'frozen full-worker source and unchanged old limits')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    baseline_worker = {n: compact(v) for n, v in worker_map.items() if n.startswith(WORKER_PREFIX)}
    actual_worker = {n: compact(v) for n, v in qualified_map.items() if n.startswith(WORKER_PREFIX)}
    authored_worker = {n: compact(v) for n, v in qualified['preformat_sources'].items()
                       if n.startswith(WORKER_PREFIX)}
    require(len(old) == 1241 and len(baseline_worker) == 197 and set(baseline_worker) <= set(old)
            and len(authored_worker) == len(actual_worker) == 203, 'explicit parent/worker source boundaries')
    transitions = {n: dict(before=old[n], after=r) for n, r in baseline_worker.items() if old[n] != r}
    require(len(transitions) == 8, 'eight directly authenticated causal-worker formatter transitions')
    expected_authored = dict(baseline_worker)
    worker_rows = {'ferric/' + r['path']: r for r in worker_proposal['files']}
    require(len(worker_rows) == len(worker_proposal['files']) == 13
            and all(n.startswith(WORKER_PREFIX) for n in worker_rows)
            and sum(r['before'] is None for r in worker_rows.values()) == 6,
            'thirteen worker overlays and six additions')
    for n, row in worker_rows.items():
        require(expected_authored.get(n) == row['before'], 'actual worker proposal preimage ' + n)
        expected_authored[n] = row['after']
    require(expected_authored == authored_worker and set(authored_worker) == set(actual_worker),
            'future worker actual preformat map exactly equals reviewed overlay')
    worker_changes = {n for n in actual_worker if actual_worker[n] != authored_worker[n]}
    require(worker_changes == set(qualified['format_changed_paths'])
            and worker_changes <= {n for n in worker_rows if n.endswith('.rs')},
            'qualified worker formatting changes remain explicit')
    expected = dict(old)
    expected.update(actual_worker)
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(parent_rows) == len(proposal['files']) == 7
            and all(n.startswith('ferric/' + PARENT_REL + '/') for n in parent_rows)
            and sum(r['before'] is None for r in parent_rows.values()) == 4,
            'seven parent overlays and four additions')
    for n, row in parent_rows.items():
        require(expected.get(n) == row['before'], 'actual parent proposal preimage ' + n)
        expected[n] = row['after']
    require(len(expected) == 1251, 'complete parent Ferric closure')
    wire_name = WORKER_PREFIX + 'src/finite_guarded_mlp_full2303_wire_v1.rs'
    require(worker_rows[wire_name]['after'] == proposal['worker_wire_api']['pin'],
            'parent API is the reviewed worker wire preformat body')
    scopes = base['tests']
    expected_lineage = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    expected_lineage |= {name + suffix for name in scopes for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == expected_lineage and len(bodies) == 112, 'closed112 direct lineage bodies')
    for name in expected_lineage - set(fixed):
        require(compact(readset[name]) == compact(base['raw'][name]), 'actual baseline raw ' + name)
    listed = core.inventory(bodies['parent-lib-list.stdout'].decode())
    require(listed == base['inventory'], 'actual918-name parent inventory')
    for name, prior in scopes.items():
        require(core.outcomes(bodies[name + '.stdout'].decode()) == prior
                and prior['failed'] == prior['ignored'] == 0, 'preserved raw selected outcomes ' + name)
        command = json.loads(bodies[name + '.command.json'])
        phase = next(p for p in base['phases'] if p['label'] == name)
        require(command['argv'] == phase['argv'], 'actual selected command join')
    require({n: compact(r) for n, r in before.items() if n.startswith('ferric/')} == expected
            and len(before) == 1254, 'complete source map from actual two CPU baselines and parent overlay')
    formatting = sorted(n for n in parent_rows if n.endswith('.rs'))
    require(inputs['parent_overlay'] == formatting and len(formatting) == 6, 'six parent Rust formatting inputs')
    require(h.pin(PARENT / 'Cargo.lock')['sha256'] == LOCK_SHA, 'unchanged parent lock')
    additions = sorted(proposal['new_tests']['parent_library'])
    prefix = 'tp_finite_client::long::full2303::tests::'
    source = (PARENT / 'src/tp_finite_client/long/full2303_tests.rs').read_text()
    require(sorted(prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source)) == additions
            and len(additions) == 6, 'six exact source-declared parent library methods')
    wire_path = 'adapters/tp-peer-finite-engineering-worker-v1/src/finite_guarded_mlp_full2303_wire_v1_tests.rs'
    wire_prefix = 'finite_guarded_mlp_full2303_wire_v1::tests::'
    wire_names = sorted(wire_prefix + r['name'] for r in worker_proposal['test_roster']
                        if r['path'] == wire_path)
    source = (FERRIC / wire_path).read_text()
    require(sorted(wire_prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source)) == wire_names
            and len(wire_names) == 4, 'four imported shared wire methods')
    additions += wire_names
    require(len(set(additions)) == 10 and not set(additions) & set(listed), 'ten distinct new library names')
    bin_names = proposal['new_tests']['parent_binary']
    source = (PARENT / ('src/bin/' + ADDED_BINARY + '.rs')).read_text()
    require(sorted('tests::' + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source)) == bin_names
            and len(bin_names) == 1, 'one exact new binary test')
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
    normalized = json.loads(json.dumps(current))
    packages = [p for p in normalized['packages'] if p['name'] == PACKAGE]
    require(len(packages) == 1, 'one parent metadata package')
    package = packages[0]
    require(package['features'].pop(FULL_FEATURE, None) == ['guarded-mlp-model-engineering'],
            'only the reviewed feature edge was added')
    targets = [t for t in package['targets'] if t['name'] == ADDED_BINARY]
    require(len(targets) == 1 and targets[0] == dict(kind=['bin'], crate_types=['bin'],
        name=ADDED_BINARY, src_path=str(PARENT / ('src/bin/' + ADDED_BINARY + '.rs')),
        edition='2024', **{'required-features': [FULL_FEATURE]}, doc=True, doctest=False, test=True),
        'one exact added binary metadata target')
    package['targets'].remove(targets[0])
    nodes = [n for n in normalized['resolve']['nodes'] if n['id'] == package['id']]
    require(len(nodes) == 1 and nodes[0]['features'].count(FULL_FEATURE) == 1,
            'one resolved Full feature')
    nodes[0]['features'].remove(FULL_FEATURE)
    require(normalized == relocate(old), 'unchanged locked graph after removing only new feature/target')
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
                and inputs['schema'] == 'ferric-guarded-mlp-full2303-parent-cpu-input-v1', 'closed parent input')
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
        require(listed == sorted(old_list + additions) and len(listed) == 928, 'old918 plus exact10 library names')
        selected = set()
        list_labels = {'guarded-bin-list': 'guarded-bin-tests', 'readiness-bin-list': 'readiness-bin-tests'}
        ordered_scopes = [p['label'] for p in base['phases'] if p['label'] in scopes or p['label'] in list_labels]
        require(len(ordered_scopes) == 53 and set(ordered_scopes) == set(scopes) | set(list_labels),
                'all51 original selected and two inventory recipes')
        for label in ordered_scopes:
            original = next(p for p in base['phases'] if p['label'] == label)
            argv = relocate(original['argv'][6:])
            require(argv[:2] == [cargo, 'test'] and argv[argv.index('--features') + 1] == OLD_FEATURE,
                    'unchanged original owned Cargo recipe')
            argv[argv.index('--features') + 1] = FEATURE
            leaf(label, argv, source=formatted)
            if label in list_labels:
                names = set(core.statuses(scopes[list_labels[label]]))
                require(core.inventory((OUT / (label + '.stdout')).read_text()) == sorted(names),
                        'preserved named binary inventory plus explicit addition')
                continue
            names = set(core.statuses(scopes[label]))
            if '--lib' in argv:
                selector = argv[argv.index('--lib') + 1]
                names |= {n for n in additions if n == selector or ('--exact' not in argv and selector in n)}
                require(not selected & names, 'inherited library selections overlap')
                selected |= names
            value = core.outcomes((OUT / (label + '.stdout')).read_text())
            require(core.statuses(value) == {n: 'ok' for n in names}
                    and value['failed'] == value['ignored'] == 0, 'all inherited and added named outcomes ' + label)
            tests[label] = value
        wire_names = {n for n in additions if n.startswith('finite_guarded_mlp_full2303_wire_v1::tests::')}
        require(len(wire_names) == 4 and not selected & wire_names, 'new disjoint shared-wire scope')
        leaf('parent-full2303-wire', [cargo, 'test', *common, *feature, '--lib',
             'finite_guarded_mlp_full2303_wire_v1::', '--', '--nocapture'], source=formatted)
        value = core.outcomes((OUT / 'parent-full2303-wire.stdout').read_text())
        require(core.statuses(value) == {n: 'ok' for n in wire_names}
                and value['failed'] == value['ignored'] == 0, 'all four shared-wire outcomes')
        tests['parent-full2303-wire'] = value
        selected |= wire_names
        require(set(additions) <= selected, 'every new library name selected')
        leaf('full2303-bin-list', [cargo, 'test', *common, *feature, '--bin', ADDED_BINARY,
             '--', '--list', '--format=terse'], source=formatted)
        require(core.inventory((OUT / 'full2303-bin-list.stdout').read_text()) == bin_names,
                'new Full binary exact inventory')
        leaf('full2303-bin-tests', [cargo, 'test', *common, *feature, '--bin', ADDED_BINARY,
             '--', '--nocapture'], source=formatted)
        value = core.outcomes((OUT / 'full2303-bin-tests.stdout').read_text())
        require(core.statuses(value) == {n: 'ok' for n in bin_names}
                and value['failed'] == value['ignored'] == 0, 'new Full binary exact outcomes')
        tests['full2303-bin-tests'] = value
        old_bins = sorted(base['artifacts'])
        require(len(old_bins) == 6 and BINARY in old_bins, 'six inherited parent products')
        build_bins = sorted(old_bins + [ADDED_BINARY])
        require(len(set(build_bins)) == 7, 'seven distinct parent products')
        args = [item for n in build_bins for item in ('--bin', n)]
        leaf('parent-builds', [cargo, 'build', '--profile', 'test', *common, *feature, *args, '--message-format=json'], source=formatted)
        records = h.build_records(OUT / 'parent-builds.stdout')
        for name in build_bins:
            candidates = [r for r in records if r.get('reason') == 'compiler-artifact'
                and r.get('manifest_path') == str(PARENT / 'Cargo.toml') and r.get('target', {}).get('name') == name
                and r['target']['kind'] == ['bin'] and r.get('profile', {}).get('test') is False]
            require(len(candidates) == 1, 'exact selected parent product')
            row = candidates[0]
            path = Path(row['executable'])
            require(path.resolve(strict=True) == path and path.is_relative_to(TARGET)
                    and {OLD_FEATURE, FULL_FEATURE} <= set(row['features']) and row['filenames'].count(str(path)) == 1, 'new target artifact')
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'parent ELF')
            artifacts[name] = dict(pin=h.pin(path), cargo_artifact=row)
        leaf('parent-default-check', [cargo, 'check', *common, '--lib', '--message-format=json'], source=formatted)
        require(len(phases) == 63 and len(tests) == 53 and sum(v['passed'] for v in tests.values()) == 455
                and sum(v['failed'] + v['ignored'] for v in tests.values()) == 0, '63 phases and455 selected passes')
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
    result = dict(schema='ferric-guarded-mlp-full2303-parent-cpu-v1', passed=failure is None, failure=failure,
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
        all_selected_parent_tests_executed=len(tests) == 53 and sum(v['passed'] for v in tests.values()) == 455,
        full2303_parent_route_added=True, full2303_native_execution=False,
        full2303_launch_feasibility=False, full2303_source_abort_ms=3600000,
        inherited_causal_layer0_parent_route_preserved=True,
        causal_layer0_parent_route_added=True, causal_layer0_native_execution=False,
        parent_causal_file_reads_retained=True,
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
