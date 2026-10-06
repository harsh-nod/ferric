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
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-host-shared-currentness-parent-cpu-v228-v1')
FERRIC = ROOT / 'ferric'
PARENT_REL = 'adapters/m1-engineering-execution-v1'
PARENT = FERRIC / PARENT_REL
CARGO_HOME = ROOT / 'cargo-home'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
SUPPORT_SHA = '71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'
BASE_SHA = '6ac67053d9b7d5d15772b5e4071a09933f013b6efa27394c31eb2b8279af884f'
TOOL_RECEIPT_SHA = '4a682798a23ac4c8accb0721f332a7b4e7484729bd692209d20f2883b501cee1'
PROPOSAL_SHA = '79af6715adfde3d4a977587119d9c652136d4b15fca109a3a8e21a882e74d330'
WORKER_SNAPSHOT_SHA = 'd2ed92454ec777292123f0e05ae11a00c504011662d2b00e769e5a74e9868386'
LOCK_SHA = 'ec06e964ed72dc9b97d9f5769bd867bf05398781b6e17a9f605137742d6e19ea'
CACHE_PIN = dict(bytes=96226, sha256='984b2a893ba64f35cda1cdceb5d379028416c00c36815c678200480c7e8904ad')
PARENT_TOOLS_SHA = 'c38e83e58d01e60b7311efa1ff53cb94e69dbbcd227ee30bde6a04c7928590d9'
PARENT_TOOLCHAIN = Path('/home/harmenon/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin')
PACKAGE = 'ferric-m1-engineering-execution-v1'
BINARY = 'ferric-qwen3-finite-guarded-mlp-decode-engineering'
FEATURE = 'guarded-mlp-model-engineering'
OLD_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/projection-ordered-segment-cpu-v228-v2'
OLD_CARGO = '/home/harmenon/ferric-asrock-42/toolchain/cargo'
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
    bodies = {name: read(h, name, row, readset) for name, row in inputs['lineage'].items()}
    for name, digest in (('base-complete.json', BASE_SHA), ('tool-complete.json', TOOL_RECEIPT_SHA),
                         ('parent-source-manifest.json', PROPOSAL_SHA),
                         ('worker-source.json', WORKER_SNAPSHOT_SHA),
                         ('capture-source-manifest.json', '2dc554b35f227180b1367f926d38e3332305f99666d9f9bd52b941240d1d899e'),
                         ('host-source-manifest.json', '8c36b978a705f6ce126ac0d3cd7f892f15e52aab752e020d84bd3e1f722021d7'),
                         ('shared-source-manifest.json', '5d0821afe584e59c145c05b0999f67ff24d090e094a3e0accc68c93601ae2456'),
                         ('qualified-worker-complete.json', 'fd4b55f53e2a61aafb49bb901796ca6c5d72a2e34d725aabc16cd4d14cef082e'),
                         ('qualified-worker-sources.json', '7dfba5bd71c4baffe607b89145d00240dbc667c782da02d669b77ca090ba757b'),
                         ('qualified-parent-complete.json', 'ab72e2316315ec6b0766284e0b4c26e834b2502cd4a6fc29c304a875950e3c8f'),
                         ('qualified-parent-sources.json', '9c003fe1f9b79498e9cf9843351f1699e2cb406b6b8517efd0ec27a87a0333c7')):
        require(readset[name]['sha256'] == digest, 'fixed provenance ' + name)
    base = json.loads(bodies['base-complete.json'])
    tools = json.loads(bodies['tool-complete.json'])
    proposal = json.loads(bodies['parent-source-manifest.json'])
    require(base['passed'] is True and base['postcheck_errors'] == [] and base['error'] is None
            and base['gpu_execution'] is False and tools['passed'] is True
            and tools['postcheck_errors'] == [], 'qualified CPU parents')
    require(h.pin(PARENT / 'Cargo.lock')['sha256'] == LOCK_SHA, 'unchanged parent lock')
    listed = core.inventory(bodies['parent-lib-list-stdout'].decode())
    require(len(listed) == 848 and compact(base['raw']['parent-lib-list-stdout'])
            == compact(readset['parent-lib-list-stdout']), 'historical full library inventory')
    scopes = {k: v for k, v in base['tests'].items() if k.startswith(('parent-', 'ferric-'))}
    require(len(scopes) == 44 and sum(v['passed'] for v in scopes.values()) == 364
            and all(v['ignored'] == 0 for v in scopes.values()), '44 historical selected scopes')
    expected_lineage = {'base-complete.json', 'tool-complete.json', 'parent-source-manifest.json',
        'worker-source.json', 'worker-proposal.json', 'parent-lib-list-stdout',
        'parent-metadata-stdout', 'committed-source.json', 'parent-tools.json'}
    expected_lineage |= {'capture-source-manifest.json', 'host-source-manifest.json', 'shared-source-manifest.json', 'qualified-worker-complete.json',
                         'qualified-worker-sources.json', 'qualified-parent-complete.json',
                         'qualified-parent-sources.json'}
    expected_lineage |= {n + suffix for n in scopes for suffix in ('-stdout', '-command.json')}
    require(set(bodies) == expected_lineage, 'closed104 parent lineage bodies')
    selected = set()
    for name, old in scopes.items():
        raw = name + '-stdout'
        command = name + '-command.json'
        require(compact(base['raw'][raw]) == compact(readset[raw])
                and compact(base['raw'][command]) == compact(readset[command]), 'historical raw scope pins')
        actual = core.outcomes(bodies[raw].decode())
        require(core.statuses(actual) == {n: 'ok' for n in old['names']}
                and actual['passed'] == old['passed'] and actual['failed'] == actual['ignored'] == 0,
                'historical raw named outcomes')
        if name.startswith('parent-'):
            require(not selected.intersection(old['names']), 'historical selections overlap')
            selected.update(old['names'])
    direct, shared = proposal['direct_new_lib_tests'], proposal['shared_new_lib_tests']
    require(len(direct) == len(shared) == 5 and len(set(direct + shared)) == 10
            and not set(direct + shared).intersection(listed)
            and len(proposal['new_binary_tests']) == 1 and proposal['binary'] == BINARY
            and proposal['feature'] == FEATURE, 'closed eleven new parent tests')
    capture = json.loads(bodies['capture-source-manifest.json'])
    require(capture['schema'] == 'ferric-guarded-mlp-model-stage-capture-source-v1'
            and capture['canonical_revision'] == '90b17ca054631aea59b03d50169d114c4663468f'
            and capture['runtime_sources_changed'] is False and capture['default_wire_changed'] is False
            and capture['kernel_images_changed'] is False and len(capture['files']) == 12,
            'closed capture source change')
    host = json.loads(bodies['host-source-manifest.json'])
    require(host['schema'] == 'ferric-guarded-mlp-model-host-observation-source-v1'
            and host['canonical_revision'] == '7f0d73ce594ee721576dc10e9d95dad973646bb0'
            and len(host['files']) == 13 and host['runtime_sources_changed'] is False
            and host['default_wire_changed'] is False and host['kernel_images_changed'] is False
            and host['performance_policy_changed'] is False
            and host['semantic_changes_relative_to_host_v1'] is False, 'closed rebased host source')
    shared_source = json.loads(bodies['shared-source-manifest.json'])
    require(shared_source['schema'] == 'ferric-guarded-mlp-model-host-shared-currentness-source-v1'
            and shared_source['canonical_revision'] == '1371774221e0b271e4aac1381c3df6f6d4a4592d'
            and len(shared_source['files']) == 8 and shared_source['runtime_sources_changed'] is False
            and shared_source['default_wire_changed'] is False and shared_source['kernel_images_changed'] is False
            and shared_source['performance_policy_changed'] is True
            and shared_source['default_policy_changed'] is False
            and shared_source['shared_full_currentness_source_added'] is True,
            'closed explicit shared-full-currentness source')
    qualified = {}
    for kind, count, phases in (('worker', 993, 9), ('parent', 1225, 55)):
        previous = json.loads(bodies['qualified-' + kind + '-complete.json'])
        source_map = json.loads(bodies['qualified-' + kind + '-sources.json'])
        require(previous['schema'] == 'ferric-guarded-mlp-host-observation-' + kind + '-cpu-v1'
                and previous['passed'] is True and previous['failure'] is None
                and previous['postcheck_errors'] == [] and previous['source_unchanged'] is True
                and previous['gpu_execution'] is False and len(source_map) == count
                and previous['input_sources'] == previous['final_sources'] == source_map
                and compact(previous['raw']['sources-after.json'])
                    == compact(readset['qualified-' + kind + '-sources.json']),
                'actual qualified ' + kind + ' source map')
        require(len(previous['phases']) == phases and all(row['exit_code'] == 0
                and row['natural_exit'] is True and row['reaped'] is True
                and row['process_group_absent'] is True and row['forced_cleanup'] is False
                and row['timed_out'] is False and row['exception'] is None for row in previous['phases']),
                'actual qualified lifecycle')
        for suffix in ('complete', 'sources'):
            require(compact(shared_source['base'][kind + '_' + suffix])
                    == compact(readset['qualified-' + kind + '-' + suffix + '.json']),
                    'shared declared actual host baseline')
        qualified[kind] = (previous, source_map)
    prior_parent, parent_sources = qualified['parent']
    require(prior_parent['inventory'] == sorted(listed + direct + shared + capture['new_tests']['parent_library']
            + host['new_tests']['parent_library'])
            and len(prior_parent['tests']) == 47
            and sum(v['passed'] for v in prior_parent['tests'].values()) == 387
            and all(v['failed'] == v['ignored'] == 0 for v in prior_parent['tests'].values()),
            'actual host parent inventory and selected outcomes')
    for name, old in scopes.items():
        names = set(old['names']) | ({n for n in direct + capture['new_tests']['parent_library'] if n.startswith('tp_finite_client::')} if name == 'parent-client' else set())
        require(core.statuses(prior_parent['tests'][name]) == {n: 'ok' for n in names},
                'actual parent preserved historical scope ' + name)
    require(core.statuses(prior_parent['tests']['parent-guarded-wire']) == {n: 'ok' for n in shared}
            and core.statuses(prior_parent['tests']['parent-guarded-host'])
                == {n: 'ok' for n in host['new_tests']['parent_library']}
            and core.statuses(prior_parent['tests']['guarded-bin-tests'])
                == {n: 'ok' for n in proposal['new_binary_tests'] + capture['new_tests']['parent_guarded_binary']
                    + host['new_tests']['parent_guarded_binary']},
            'actual prior added scopes')
    worker_prefix = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
    canonical = {n: compact(r) for n, r in parent_sources.items() if n.startswith('ferric/')}
    worker = {n: compact(r) for n, r in qualified['worker'][1].items() if n.startswith(worker_prefix)}
    require(len(canonical) == 1222 and len(worker) == 184 and set(worker) <= set(canonical),
            'qualified parent/worker source closure')
    inherited_changes = sorted(n for n, row in worker.items() if canonical[n] != row)
    require(len(inherited_changes) == 10, 'exact inherited host worker formatting transition')
    canonical.update(worker)
    committed = json.loads(bodies['committed-source.json'])
    require(committed['schema'] == 'ferric-guarded-mlp-parent-committed-source-v1'
            and committed['revision'] == inputs['git_revision'] == shared_source['canonical_revision']
            and set(committed['git_blobs']) == set(committed['files'])
            and committed['files'] == canonical, 'canonical source equals qualified host parent plus exact host worker')
    overlay = {'ferric/' + row['path']: {k: row[k] for k in ('before', 'after')} for row in shared_source['files']}
    require(committed['overlay'] == overlay and len(overlay) == 8 and sum(row['before'] is None for row in overlay.values()) == 0,
            'closed shared overlay lineage')
    expected_source = dict(canonical)
    for name, row in overlay.items():
        require(expected_source.get(name) == row['before'], 'shared overlay preimage')
        expected_source[name] = row['after']
    require({n: compact(r) for n, r in before.items() if n.startswith('ferric/')} == expected_source
            and len(expected_source) == 1222, 'only qualified source plus eight shared overlays')
    require(inputs['parent_overlay'] == sorted(n for n in overlay if n.startswith('ferric/' + PARENT_REL + '/')),
            'exact two parent formatter paths')
    additions = shared_source['new_tests']
    require(len(capture['new_tests']['parent_library']) == 4
            and len(capture['new_tests']['parent_guarded_binary']) == 1,
            'five inherited capture parent tests')
    require(len(additions['parent_library']) == 3 and len(additions['parent_guarded_binary']) == 1
            and not set(additions['parent_library']) & set(prior_parent['inventory'])
            and all(n.startswith('guarded_mlp_host_observation_v1::tests::')
                    for n in additions['parent_library']), 'four additive shared-currentness parent tests')
    proposal = dict(proposal, direct_new_lib_tests=direct + capture['new_tests']['parent_library'],
                    host_lib_tests=host['new_tests']['parent_library'] + additions['parent_library'],
                    new_binary_tests=sorted(proposal['new_binary_tests']
                        + capture['new_tests']['parent_guarded_binary']
                        + host['new_tests']['parent_guarded_binary'] + additions['parent_guarded_binary']))
    require(compact(base['raw']['parent-metadata-stdout']) == compact(readset['parent-metadata-stdout']),
            'historical metadata raw pin')
    require(readset['parent-tools.json']['sha256'] == PARENT_TOOLS_SHA, 'actual parent tool observation pin')
    parent_tools = json.loads(bodies['parent-tools.json'])
    require(set(parent_tools) == {'schema', 'toolchain', 'tool_pins', 'rustc_version',
                                  'controller', 'build_execution', 'gpu_execution'}
            and parent_tools['schema'] == 'ferric-guarded-mlp-parent-toolchain-v1'
            and parent_tools['toolchain'] == str(PARENT_TOOLCHAIN)
            and parent_tools['rustc_version'].startswith('rustc 1.97.1 (')
            and parent_tools['build_execution'] is False and parent_tools['gpu_execution'] is False
            and len(parent_tools['tool_pins']) == 8, 'actual parent-compatible toolchain observation')
    fixed = {'rustc', 'rustdoc', 'rustfmt', 'cargo', 'prlimit'}
    libraries = set(parent_tools['tool_pins']) - fixed
    require(fixed <= set(parent_tools['tool_pins']) and len(libraries) == 3
            and all(re.fullmatch(r'lib[A-Za-z0-9_.-]+', name) for name in libraries),
            'closed parent compiler executable/library roster')
    for name, row in parent_tools['tool_pins'].items():
        path = (Path('/usr/bin/prlimit') if name == 'prlimit' else
                PARENT_TOOLCHAIN / name if name in fixed else PARENT_TOOLCHAIN.parent / 'lib' / name)
        require(row['path'] == str(path) and h.pin(path) == row, 'observed parent tool changed')
    return base, parent_tools, proposal, bodies, scopes, listed


def relocate(value):
    if isinstance(value, str):
        return value.replace(OLD_ROOT + '/sources/ferric', str(FERRIC)).replace(
            OLD_ROOT + '/target/parent', str(TARGET)).replace(OLD_CARGO, str(CARGO_HOME))
    if isinstance(value, list):
        return [relocate(v) for v in value]
    if isinstance(value, dict):
        return {k: relocate(v) for k, v in value.items()}
    return value


def metadata_contract(h, old):
    current = json.loads((OUT / 'metadata.stdout').read_bytes())
    normalized = json.loads(json.dumps(current))
    root = next(p for p in normalized['packages'] if p['name'] == PACKAGE)
    expected = relocate(old)
    prior = next(p for p in expected['packages'] if p['name'] == PACKAGE)
    target = dict(next(t for t in prior['targets'] if t['name'] ==
        'ferric-qwen3-finite-projection-residual-decode-ordered-host-engineering'))
    target.update(name=BINARY, src_path=str(PARENT / 'src/bin' / (BINARY + '.rs')))
    target['required-features'] = [FEATURE]
    require(root['targets'].count(target) == 1 and root['features'].pop(FEATURE) == ['tp-batch-engineering'],
            'single additive parent feature and binary')
    root['targets'].remove(target)
    node = next(v for v in normalized['resolve']['nodes'] if v['id'] == root['id'])
    require(node['features'].count(FEATURE) == 1, 'selected guarded feature')
    node['features'].remove(FEATURE)
    require(normalized == expected, 'unchanged complete locked parent metadata except reviewed additions')
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
                and inputs['schema'] == 'ferric-guarded-mlp-host-shared-currentness-parent-cpu-input-v1', 'closed parent input')
        before = snapshot(h)
        require(inputs['files'] == {k: compact(v) for k, v in before.items()}, 'exact parent source input')
        h.save('sources-preformat.json', before)
        base, parent_tools, proposal, bodies, scopes, old_list = lineage(h, core, inputs, before, readset)
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
        metadata, dependencies = metadata_contract(h, json.loads(bodies['parent-metadata-stdout']))
        h.save('dependencies-before.json', dependencies)
        leaf('parent-lib-list', [cargo, 'test', *common, *feature, '--lib', '--', '--list', '--format=terse'], source=formatted)
        listed = core.inventory((OUT / 'parent-lib-list.stdout').read_text())
        additions = proposal['direct_new_lib_tests'] + proposal['shared_new_lib_tests'] + proposal['host_lib_tests']
        require(listed == sorted(old_list + additions) and len(listed) == 871, 'old848 plus exact23 parent names')
        selected = set()
        for label, previous in sorted(scopes.items()):
            argv = json.loads(bodies[label + '-command.json'])['argv']
            start = argv.index('test') + 1
            tail = argv[start:]
            pos = tail.index('--manifest-path')
            tail[pos + 1] = str(PARENT / 'Cargo.toml')
            tail[tail.index('--features') + 1] = FEATURE
            names = set(previous['names'])
            if label.startswith('parent-'):
                selector = tail[tail.index('--lib') + 1]
                names |= {n for n in additions if n == selector or ('--exact' not in tail and selector in n)}
                require(not selected & names, 'parent scope overlap')
                selected |= names
            leaf(label, [cargo, 'test', *tail], source=formatted)
            value = core.outcomes((OUT / (label + '.stdout')).read_text())
            require(core.statuses(value) == {n: 'ok' for n in names} and value['failed'] == value['ignored'] == 0,
                    'preserved selected parent named outcomes ' + label)
            tests[label] = value
        shared = proposal['shared_new_lib_tests']
        require(not set(shared) & selected, 'new shared tests already selected')
        leaf('parent-guarded-wire', [cargo, 'test', *common, *feature, '--lib', 'finite_guarded_mlp_decode_wire_v1::',
                                  '--', '--test-threads=2'], source=formatted)
        value = core.outcomes((OUT / 'parent-guarded-wire.stdout').read_text())
        require(core.statuses(value) == {n: 'ok' for n in shared}, 'five new shared wire outcomes')
        tests['parent-guarded-wire'] = value
        host_names = proposal['host_lib_tests']
        require(not set(host_names) & (selected | set(shared)), 'host scope overlaps existing selections')
        leaf('parent-guarded-host', [cargo, 'test', *common, *feature, '--lib',
            'guarded_mlp_host_observation_v1::', '--', '--test-threads=2'], source=formatted)
        value = core.outcomes((OUT / 'parent-guarded-host.stdout').read_text())
        require(core.statuses(value) == {n: 'ok' for n in host_names}
                and value['failed'] == value['ignored'] == 0, 'nine shared host observation outcomes')
        tests['parent-guarded-host'] = value
        for label, args in [('guarded-bin-list', ['--list', '--format=terse']),
                            ('guarded-bin-tests', ['--test-threads=2'])]:
            leaf(label, [cargo, 'test', *common, *feature, '--bin', BINARY, '--', *args], source=formatted)
        require(core.inventory((OUT / 'guarded-bin-list.stdout').read_text()) == proposal['new_binary_tests'], 'new bin inventory')
        value = core.outcomes((OUT / 'guarded-bin-tests.stdout').read_text())
        require(core.statuses(value) == {n: 'ok' for n in proposal['new_binary_tests']}, 'new bin outcome')
        tests['guarded-bin-tests'] = value
        old_bins = sorted(n for n in base['binaries'] if n != 'ferric-tp-peer-finite-engineering-worker-v1')
        require(len(old_bins) == 4, 'four inherited parent products')
        args = [item for n in old_bins + [BINARY] for item in ('--bin', n)]
        leaf('parent-builds', [cargo, 'build', '--profile', 'test', *common, *feature, *args, '--message-format=json'], source=formatted)
        records = h.build_records(OUT / 'parent-builds.stdout')
        for name in old_bins + [BINARY]:
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
        require(len(phases) == 55 and len(tests) == 47 and sum(v['passed'] for v in tests.values()) == 391
                and sum(v['failed'] + v['ignored'] for v in tests.values()) == 0, '55 phases and391 selected passes')
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
    raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
    for row in phases:
        for key in ('command', 'stdout', 'stderr'):
            if raw.get(Path(row[key]['path']).name) != row[key]:
                errors.append('raw phase join')
    failure = failure or ('postcheck failed' if errors else None)
    result = dict(schema='ferric-guarded-mlp-host-shared-currentness-parent-cpu-v1', passed=failure is None, failure=failure,
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
        all_selected_parent_tests_executed=len(tests) == 47 and sum(v['passed'] for v in tests.values()) == 391,
        shared_full_currentness_source_added=True, shared_full_currentness_native_execution=False,
        inherited_host_observation_source_added=True, inherited_capture_source_added=True,
        performance_policy_changed=True, default_policy_changed=False,
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
