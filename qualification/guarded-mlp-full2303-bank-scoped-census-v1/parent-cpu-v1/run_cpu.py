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
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-bank-scoped-census-parent-cpu-v228-v1')
FERRIC = ROOT / 'ferric'
PARENT_REL = 'adapters/m1-engineering-execution-v1'
PARENT = FERRIC / PARENT_REL
CARGO_HOME = ROOT / 'cargo-home'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
SUPPORT_SHA = '71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'
BASE_COMPLETE = {'bytes':4095966,'sha256':'e3915ff3e15e7d52fb57675bda0cc0752ec349c1a8317b0ef826ddcdaed70d44'}
BASE_SOURCES = {'bytes':526507,'sha256':'0ab51eeff901cd2405faf7dbe63197539ae6173346456d9dabd1833366aab2ff'}
PROPOSAL_PINS = {'parent-proposal.json':{'bytes':15696,'sha256':'265c8d1891ddc8216839ad3699b519a0d9fa4a481d198525c0bf3274aaed495e'},'worker-proposal.json':{'bytes':26569,'sha256':'67d92d767311cb5a7e508c3f489e26fa14d3e0f38c77117cb1503a30dd68b51c'}}
BASE_WORKER_COMPLETE = {'bytes':2442525,'sha256':'4d68df6862da654f9044cb08ad5ecedf008fb96f1fa72345d5382d6c96b89e41'}
WORKER_COMPLETE = {"bytes":2475794,"sha256":"a7fcea3f09cfcda84ba556f60b5f1578ed378a45fc84bb81fbfa6ed31273cd64"}
BASE_WORKER_SOURCES = {'bytes':431540,'sha256':'145b177805d0eefdac0e868729d28d26f1646259116a9ff10e316250e1668d69'}
WORKER_SOURCES = {"bytes":438675,"sha256":"5372e185504d11ed89d45cfb3d3e32672f93e8d1bbf3d04e44932b280300528c"}
WORKER_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-bank-scoped-census-cpu-v228-v1'
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
OLD_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-census-parent-cpu-v228-v1'
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
    fixed = dict(PROPOSAL_PINS, **{'parent-complete.json': BASE_COMPLETE,
        'parent-sources.json': BASE_SOURCES, 'worker-complete.json': WORKER_COMPLETE,
        'worker-sources.json': WORKER_SOURCES,
        'baseline-worker-sources.json': BASE_WORKER_SOURCES})
    bodies = {name: read(h, name, row, readset) for name, row in inputs['lineage'].items()}
    require(all(compact(readset[name]) == row for name, row in fixed.items()), 'literal actual parent/worker/source pins')
    base = json.loads(bodies['parent-complete.json'])
    old_map = json.loads(bodies['parent-sources.json'])
    proposal = json.loads(bodies['parent-proposal.json'])
    worker = json.loads(bodies['worker-complete.json'])
    worker_map = json.loads(bodies['worker-sources.json'])
    worker_proposal = json.loads(bodies['worker-proposal.json'])
    base_worker_map = json.loads(bodies['baseline-worker-sources.json'])
    require(base['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-census-parent-cpu-v1'
            and len(old_map) == 1282 and len(base['phases']) == 67 and len(base['tests']) == 57
            and len(base['inventory']) == 1001 and sum(v['passed'] for v in base['tests'].values()) == 535
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['full2303_scoped_warm_parent_source_added'] is True
            and base['full2303_scoped_policy_prepublication_checked'] is True
            and base['readiness40_scoped_warm_parent_source_added'] is True
            and base['scoped_warm_host_timing_parent_source_added'] is True
            and base['readiness40_bank_scoped_warm_parent_source_added'] is True
            and base['allocation_preflights_changed'] is True
            and base['readiness40_bank_scoped_census_parent_source_added'] is True
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual535 selected Census parent baseline')
    for receipt, source_map, expected_pin, phase_count in (
            (base, old_map, BASE_SOURCES, 67), (worker, worker_map, WORKER_SOURCES, 26)):
        require(receipt['passed'] is True and receipt['failure'] is None
                and receipt['postcheck_errors'] == [] and receipt['source_unchanged'] is True
                and receipt['gpu_execution'] is False
                and receipt['input_sources'] == receipt['final_sources'] == source_map
                and compact(receipt['raw']['sources-after.json']) == expected_pin
                and len(receipt['phases']) == phase_count
                and all(p['exit_code'] == 0 and p['natural_exit'] is True and p['reaped'] is True
                    and p['process_group_absent'] is True and p['forced_cleanup'] is False
                    and p['timed_out'] is False and p['exception'] is None and p['storage_failure'] is None
                    for p in receipt['phases']), 'actual clean source/CPU lifecycle')
    require(worker['schema'] == 'ferric-guarded-mlp-full2303-bank-scoped-census-cpu-v1'
            and worker['source_generation'] == 'full2303-bank-scoped-census-coupled-v1'
            and len(worker_map) == 1053 and len(worker['artifacts']) == 11
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (799, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1165, 0, 8)
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['runtime_source_changed'] is False
            and worker['scoped_bank_rearm_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_warm_source_added'] is True
            and worker['full2303_scoped_warm_source_added'] is True
            and worker['inherited_scoped_currentness_runtime_preserved'] is True
            and worker['scoped_capacity_census_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_census_source_added'] is True
            and worker['allocation_preflights_changed'] is True
            and worker['full2303_bank_scoped_census_source_added'] is True
            and worker['inherited_readiness40_bank_scoped_census_preserved'] is True
            and worker['full2303_deadline_changed'] is False
            and all(worker[k] is False for k in ('full2303_scoped_warm_native_execution',
                'readiness40_bank_scoped_warm_native_execution', 'readiness40_bank_scoped_census_native_execution',
                'full2303_bank_scoped_census_native_execution',
                'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual Full bank-census worker and unchanged runtime with tested real executable')
    require(compact(worker['readset']['worker-proposal.json']) == fixed['worker-proposal.json']
            and compact(worker['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual coupled direct proposal and predecessor readset')
    require(proposal['schema'] == 'ferric-full2303-bank-scoped-census-parent-source-v1'
            and compact(proposal['base']['parent_receipt']) == BASE_COMPLETE
            and compact(proposal['base']['parent_sources']) == BASE_SOURCES
            and compact(proposal['requires']['worker_source_manifest']) == fixed['worker-proposal.json']
            and all(proposal[k] is False for k in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'default_routes_changed', 'default_group_policy_changed',
                'runtime_api_changed', 'deadline_changed', 'reference_feedback',
                'temporal_equivalent_to_full', 'performance_claim', 'numerical_acceptance',
                'full2303_launch_admitted')), 'exact Full bank-census parent proposal')
    require(worker_proposal['schema'] == 'ferric-full2303-bank-scoped-census-worker-source-v1'
            and len(worker_proposal['files']) == 15
            and worker_proposal['source_generation'] == 'full2303-bank-scoped-census-worker-v1'
            and worker_proposal['new_explicit_full2303_route'] is True
            and worker_proposal['first_two_forwards_unchanged'] is True
            and worker_proposal['full_child_abort_ms'] == 3600000
            and worker_proposal['per_dispatch_timeout_max_ms'] == 10000
            and compact(worker_proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(worker_proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and worker_proposal['conditional_qualification'] == dict(worker_source_files=224,
                passed=799, ignored=4, inventory=803, new_library_methods=19,
                new_readiness_cli_methods=1, existing_targets_and_old_named_statuses_must_be_preserved=True)
            and all(worker_proposal[k] is False for k in ('compiled', 'tested', 'canonical_changed',
                'native_execution', 'runtime_changed', 'default_group_policy_changed',
                'existing_selectors_changed', 'currentness_temporal_equivalence_claim',
                'performance_claim', 'numerical_acceptance', 'full_launch_admitted')),
            'qualified Full bank-census worker source lineage')
    old_runtime = {n: compact(r) for n, r in base_worker_map.items() if n.startswith('fe2o3/')}
    require(len(old_runtime) == 825
            and {n: compact(r) for n, r in worker_map.items() if n.startswith('fe2o3/')} == old_runtime
            and {n: compact(r) for n, r in worker['preformat_sources'].items()
                 if n.startswith('fe2o3/')} == old_runtime,
            'all825 actual Census runtime bodies unchanged through worker qualification')
    worker_rows = {'ferric/' + r['path']: r for r in worker_proposal['files']}
    require(len(worker_rows) == 15 and sum(r['before'] is None for r in worker_rows.values()) == 4
            and all(n.startswith(WORKER_PREFIX) and n.endswith('.rs') and r['repository'] == 'ferric'
                    for n, r in worker_rows.items()), 'fifteen closed worker paths with four additions')
    require(all(worker_rows.get('ferric/' + r['path']) == {k: r[k] for k in ('repository', 'path', 'before', 'after')}
                for r in proposal['requires']['imported_policy_rows'])
            and len(proposal['requires']['imported_policy_rows']) == 2,
            'exact imported worker policy and test source rows')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1279 and sum(n.startswith(WORKER_PREFIX) for n in old) == 220
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual1279 parent source closure with220 worker bodies')
    pre_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    require(pre_worker == {n: compact(r) for n, r in base_worker_map.items()
                           if n.startswith(WORKER_PREFIX)},
            'all220 actual parent worker bodies equal the coupled baseline')
    for n, row in worker_rows.items():
        require(pre_worker.get(n) == row['before'], 'actual worker preimage or absence ' + n)
        pre_worker[n] = row['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 224 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in worker_map) == 825,
            'only fifteen qualified worker format paths; all224 final worker bodies preserved')
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(parent_rows) == len(proposal['files']) == 6
            and sum(r['before'] is None for r in parent_rows.values()) == 1
            and all(n.startswith('ferric/' + PARENT_REL + '/') and n.endswith('.rs')
                    and r['repository'] == 'ferric' for n, r in parent_rows.items()),
            'exact six parent paths with one addition')
    expected = {n: r for n, r in old.items() if not n.startswith(WORKER_PREFIX)}
    expected.update(qualified_worker)
    for n, row in parent_rows.items():
        require(expected.get(n) == row['before'], 'actual parent preimage or absence ' + n)
        expected[n] = row['after']
    require(len(expected) == 1284 and not set(parent_rows) & set(worker_rows),
            '1284 Ferric bodies with disjoint ownership')
    rows = dict(worker_rows, **parent_rows)
    scopes = base['tests']
    expected_lineage = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    expected_lineage |= {name + suffix for name in scopes for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == expected_lineage and len(bodies) == 123, 'closed123 direct lineage bodies')
    for name in expected_lineage - set(fixed):
        require(compact(readset[name]) == compact(base['raw'][name]), 'actual baseline raw ' + name)
    listed = core.inventory(bodies['parent-lib-list.stdout'].decode())
    require(listed == base['inventory'], 'actual1001-name parent inventory')
    for name, prior in scopes.items():
        require(core.outcomes(bodies[name + '.stdout'].decode()) == prior
                and prior['failed'] == prior['ignored'] == 0, 'preserved raw selected outcomes ' + name)
        command = json.loads(bodies[name + '.command.json'])
        phase = next(p for p in base['phases'] if p['label'] == name)
        require(command['argv'] == phase['argv'], 'actual selected command join')
    require({n: compact(r) for n, r in before.items() if n.startswith('ferric/')} == expected
            and len(before) == 1287, 'complete actual baseline, qualified worker and six parent overlays')
    require(inputs['parent_overlay'] == sorted(parent_rows), 'only six parent Rust formatting inputs')
    require(h.pin(PARENT / 'Cargo.lock')['sha256'] == LOCK_SHA, 'unchanged parent lock')
    test_groups = (
        (PARENT / 'src/tp_finite_client/long/full2303_tests.rs',
         'tp_finite_client::long::full2303::tests::', 17, 6),
        (FERRIC / 'adapters/tp-peer-finite-engineering-worker-v1/src/finite_guarded_mlp_full2303_bank_scoped_census_v1_tests.rs',
         'finite_guarded_mlp_full2303_bank_scoped_census_v1::tests::', 7, 7))
    additions = []
    for path, prefix, total, added in test_groups:
        declared = [prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', path.read_text())]
        new = [n for n in declared if n not in listed]
        require(len(declared) == len(set(declared)) == total and len(new) == added,
                'exact source-declared new and inherited methods')
        additions.extend(new)
    additions.sort()
    declared_additions = proposal['qualified_new_tests']
    require(len(additions) == len(set(additions)) == 13
            and additions == sorted(declared_additions['parent-client']
                                    + declared_additions['full-bank-census-policy']),
            'exact thirteen additive library names')
    bin_names = sorted(declared_additions['full2303-bin'])
    source = (PARENT / 'src/bin/ferric-qwen3-guarded-mlp-full2303-engineering.rs').read_text()
    declared_bin = sorted('tests::' + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source))
    old_bin = set(core.statuses(scopes['full2303-bin-tests']))
    require(len(bin_names) == 1 and not set(bin_names) & old_bin
            and declared_bin == sorted(old_bin | set(bin_names)), 'exact one additive Full2303 CLI test')
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
    require(current == relocate(old), 'unchanged locked dependency/feature/target graph')
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
            and PARENT_TOOLS_SHA is not None and WORKER_COMPLETE is not None and WORKER_SOURCES is not None, 'python3 -B run_cpu.py INPUT_SHA; bind actual parent tools first')
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
                and inputs['schema'] == 'ferric-guarded-mlp-full2303-bank-scoped-census-parent-cpu-input-v1', 'closed parent input')
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
        require(listed == sorted(old_list + additions) and len(listed) == 1014, 'old1001 plus exact13 library names')
        selected = set()
        list_labels = {'guarded-bin-list': 'guarded-bin-tests', 'readiness-bin-list': 'readiness-bin-tests',
                       'full2303-bin-list': 'full2303-bin-tests'}
        ordered_scopes = [p['label'] for p in base['phases'] if p['label'] in scopes or p['label'] in list_labels]
        require(len(ordered_scopes) == 60 and set(ordered_scopes) == set(scopes) | set(list_labels),
                'all57 original selected and three inventory recipes')
        for label in ordered_scopes:
            original = next(p for p in base['phases'] if p['label'] == label)
            argv = relocate(original['argv'][6:])
            require(argv[:2] == [cargo, 'test'] and argv[argv.index('--features') + 1] == FEATURE,
                    'unchanged original owned Cargo recipe')
            leaf(label, argv, source=formatted)
            if label in list_labels:
                names = set(core.statuses(scopes[list_labels[label]]))
                if label == 'full2303-bin-list':
                    names |= set(bin_names)
                require(core.inventory((OUT / (label + '.stdout')).read_text()) == sorted(names),
                        'preserved named binary inventory plus explicit addition')
                continue
            names = set(core.statuses(scopes[label]))
            if label == 'full2303-bin-tests':
                names |= set(bin_names)
            if '--lib' in argv:
                selector = argv[argv.index('--lib') + 1]
                names |= {n for n in additions if n == selector or ('--exact' not in argv and selector in n)}
                require(not selected & names, 'inherited library selections overlap')
                selected |= names
            value = core.outcomes((OUT / (label + '.stdout')).read_text())
            require(core.statuses(value) == {n: 'ok' for n in names}
                    and value['failed'] == value['ignored'] == 0, 'all inherited and added named outcomes ' + label)
            tests[label] = value
        label = 'parent-full2303-bank-census-policy'
        selector = 'finite_guarded_mlp_full2303_bank_scoped_census_v1::tests::'
        names = {n for n in additions if n.startswith(selector)}
        require(len(names) == 7 and not selected & names, 'separate seven-method pure Full policy scope')
        leaf(label, [cargo, 'test', *common, *feature, '--lib', selector, '--', '--test-threads=2'], source=formatted)
        value = core.outcomes((OUT / (label + '.stdout')).read_text())
        require(core.statuses(value) == {n: 'ok' for n in names}
                and value['failed'] == value['ignored'] == 0, 'all seven pure Full policy outcomes')
        tests[label] = value
        selected |= names
        require(set(additions) <= selected, 'every new library method selected exactly once')
        build_bins = sorted(base['artifacts'])
        require(len(build_bins) == 7 and BINARY in build_bins and ADDED_BINARY in build_bins,
                'seven unchanged parent products')
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
        require(len(phases) == 68 and len(tests) == 58 and sum(v['passed'] for v in tests.values()) == 549
                and sum(v['failed'] + v['ignored'] for v in tests.values()) == 0, '68 phases and549 selected passes')
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
    result = dict(schema='ferric-guarded-mlp-full2303-bank-scoped-census-parent-cpu-v1', passed=failure is None, failure=failure,
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
        all_selected_parent_tests_executed=len(tests) == 58 and sum(v['passed'] for v in tests.values()) == 549,
        readiness40_bank_scoped_warm_parent_source_added=True,
        readiness40_bank_scoped_warm_native_execution=False, allocation_preflights_changed=True,
        readiness40_bank_scoped_census_parent_source_added=True,
        readiness40_bank_scoped_census_native_execution=False,
        allocation_preflights_changed_only_for_explicit_warm_position5=False,
        allocation_preflights_changed_only_for_explicit_warm_position5_or_full=True,
        full2303_bank_scoped_census_parent_source_added=True,
        full2303_bank_scoped_census_native_execution=False, full2303_deadline_changed=False,
        full2303_bank_scoped_census_policy_prepublication_checked=True,
        readiness40_scoped_warm_parent_source_added=True, readiness40_scoped_warm_native_execution=False,
        scoped_warm_host_timing_parent_source_added=True, currentness_temporal_equivalence_claim=False,
        readiness40_shared_full_parent_route_added=True, readiness40_shared_full_native_execution=False,
        shared_full_host_timing_parent_source_added=True, shared_full_host_timing_native_execution=False,
        selected_readiness_currentness_policy_changed=True, qualified_worker_sources_preserved=True,
        worker_qualification=WORKER_COMPLETE, worker_source_manifest=WORKER_SOURCES,
        parent_host_timing_source_added=True, parent_host_timing_native_execution=False,
        parent_host_timing_rows=40, parent_host_timing_disjoint_spans=124,
        ordinary_wire_schema_changed=False, ordinary_observation_schema_changed=False,
        full2303_scoped_warm_parent_source_added=True, full2303_scoped_warm_native_execution=False,
        full2303_scoped_policy_prepublication_checked=True, full2303_launch_admitted=False,
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
