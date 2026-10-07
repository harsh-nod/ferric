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
EXPECTED_ROOT = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-readiness40-bank-scoped-warm-parent-cpu-v228-v2')
FERRIC = ROOT / 'ferric'
PARENT_REL = 'adapters/m1-engineering-execution-v1'
PARENT = FERRIC / PARENT_REL
CARGO_HOME = ROOT / 'cargo-home'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
SUPPORT_SHA = '71c71673a2de445d386b91a488268ca6e4b9a4815abfd3150160d2c545a31c13'
BASE_COMPLETE = {'bytes': 3997921, 'sha256': 'c145839bb36b2dc437590bb1bcdc6e4977f0fabc74c4889cfcb5345b57f5b6d8'}
BASE_SOURCES = {'bytes': 508077, 'sha256': '50e41ee555b648ccf93aec5aae95b1b1040b936ce426d56cd620dd524106f5b9'}
PROPOSAL_PINS = {'parent-proposal.json': {'bytes': 19134, 'sha256': '072568d76e5055b934aa51bf50c7d8d1bad4b0d10fc19441df23de15fd8eb643'}, 'worker-proposal.json': {'bytes': 15970, 'sha256': 'bb28743219931b14202c725650ff45841e3857a3cc5112390896dfc163c6626c'}, 'runtime-proposal.json': {'bytes': 10029, 'sha256': '2798cf16b04d0c77c5a1df155513a99c226907c1db46558e4d882da9ddf2e7ff'}, 'worker-repair.json': {'bytes': 17360, 'sha256': '70f9d4e91c0c2d22bdc647f6cac67aa889fd7e9972c21050caa19cb5d4845b53'}}
BASE_WORKER_COMPLETE = {'bytes': 2402339, 'sha256': 'a44623c8586dd5ba00eb55f0d549eb11b01c7c53b96f363749246eef1037d2a1'}
WORKER_COMPLETE = {'bytes': 2395639, 'sha256': 'e522aafde4204c24be8fac9b1ecf8d49f8e3389819ad8cfda765f2145647c432'}
BASE_WORKER_SOURCES = {'bytes': 430941, 'sha256': 'c3dbb35ea085d4166e4b5ee58b29732ebfe2f14c63599f693ceefdc5a8959d37'}
WORKER_SOURCES = {'bytes': 423235, 'sha256': '73e3ad534a3d48bddd717780215142dd588f4a776dd25f003305854f70d7c285'}
WORKER_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-scoped-bank-rearm-cpu-v228-v2'
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
OLD_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-full2303-scoped-warm-parent-cpu-v228-v2'
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
        'worker-sources.json': WORKER_SOURCES})
    bodies = {name: read(h, name, row, readset) for name, row in inputs['lineage'].items()}
    require(all(compact(readset[name]) == row for name, row in fixed.items()), 'literal actual parent/worker/source pins')
    base = json.loads(bodies['parent-complete.json'])
    old_map = json.loads(bodies['parent-sources.json'])
    proposal = json.loads(bodies['parent-proposal.json'])
    worker = json.loads(bodies['worker-complete.json'])
    worker_map = json.loads(bodies['worker-sources.json'])
    worker_proposal = json.loads(bodies['worker-proposal.json'])
    require(base['schema'] == 'ferric-guarded-mlp-full2303-scoped-warm-parent-cpu-v1'
            and len(old_map) == 1270 and len(base['phases']) == 66 and len(base['tests']) == 56
            and len(base['inventory']) == 972 and sum(v['passed'] for v in base['tests'].values()) == 504
            and all(v['failed'] == v['ignored'] == 0 for v in base['tests'].values())
            and base['full2303_scoped_warm_parent_source_added'] is True
            and base['full2303_scoped_policy_prepublication_checked'] is True
            and base['readiness40_scoped_warm_parent_source_added'] is True
            and base['scoped_warm_host_timing_parent_source_added'] is True
            and base['qualified_worker_sources_preserved'] is True
            and base['worker_qualification'] == BASE_WORKER_COMPLETE
            and base['worker_source_manifest'] == BASE_WORKER_SOURCES
            and base['parent_host_timing_rows'] == 40 and base['parent_host_timing_disjoint_spans'] == 124
            and len(base['artifacts']) == 7, 'actual504 selected Full-scoped parent baseline')
    for receipt, source_map, expected_pin, phase_count in (
            (base, old_map, BASE_SOURCES, 66), (worker, worker_map, WORKER_SOURCES, 26)):
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
    require(worker['schema'] == 'ferric-guarded-mlp-scoped-bank-rearm-cpu-v1'
            and worker['source_generation'] == 'scoped-bank-rearm-coupled-v2'
            and len(worker_map) == 1042 and len(worker['artifacts']) == 11
            and (worker['tests']['worker-tests']['passed'], worker['tests']['worker-tests']['failed'],
                 worker['tests']['worker-tests']['ignored']) == (760, 0, 4)
            and (worker['tests']['kfd-tests']['passed'], worker['tests']['kfd-tests']['failed'],
                 worker['tests']['kfd-tests']['ignored']) == (1152, 0, 8)
            and worker['cli_executable_unchanged_across_tests'] is True
            and worker['cli_executable_before_tests']['pin'] == worker['artifacts']['worker']['pin']
            and worker['runtime_source_changed'] is True
            and worker['scoped_bank_rearm_runtime_source_added'] is True
            and worker['readiness40_bank_scoped_warm_source_added'] is True
            and worker['full2303_scoped_warm_source_added'] is True
            and worker['inherited_scoped_currentness_runtime_preserved'] is True
            and all(worker[k] is False for k in ('full2303_scoped_warm_native_execution',
                'readiness40_bank_scoped_warm_native_execution', 'allocation_preflights_changed',
                'full2303_launch_admitted',
                'currentness_temporal_equivalence_claim', 'global_currentness_policy_changed',
                'numerical_acceptance', 'performance_claim')),
            'actual coupled bank runtime and worker with tested real executable')
    require(compact(worker['readset']['worker-proposal.json']) == fixed['worker-proposal.json']
            and compact(worker['readset']['runtime-proposal.json']) == fixed['runtime-proposal.json']
            and compact(worker['readset']['worker-repair.json']) == fixed['worker-repair.json']
            and compact(worker['readset']['baseline-complete.json']) == BASE_WORKER_COMPLETE
            and compact(worker['readset']['baseline-sources.json']) == BASE_WORKER_SOURCES,
            'actual coupled direct proposal and predecessor readset')
    require(proposal['schema'] == 'ferric-bank-scoped-warm-readiness40-parent-source-v1'
            and proposal['source_generation'] == 'bank-scoped-warm-parent-v2-full-parent-rebase'
            and compact(proposal['base']['parent_receipt']) == BASE_COMPLETE
            and compact(proposal['base']['parent_sources']) == BASE_SOURCES
            and compact(proposal['requires']['worker_proposal']) == fixed['worker-proposal.json']
            and compact(proposal['requires']['runtime_proposal']) == fixed['runtime-proposal.json']
            and all(proposal[k] is False for k in ('canonical_changed', 'compiled', 'tested',
                'native_execution', 'default_routes_changed', 'default_group_policy_changed',
                'temporal_equivalent_to_full', 'performance_claim', 'numerical_acceptance')),
            'exact rebased bank-scoped parent proposal')
    original_worker = worker_proposal
    repaired_raw = bodies['worker-repair.json']
    worker_proposal = json.loads(repaired_raw)
    repair = worker_proposal['repair']
    repair_path = "adapters/tp-peer-finite-engineering-worker-v1/src/native_guarded_mlp_readiness_cli_v1_tests.rs"
    require(worker_proposal['source_generation'] == 'bank-scoped-warm-worker-v2-local-unsafe-test-allowance'
            and {k: worker_proposal['predecessor_manifest'][k] for k in ('bytes', 'sha256')}
                == dict(bytes=15970, sha256='bb28743219931b14202c725650ff45841e3857a3cc5112390896dfc163c6626c')
            and {k: v for k, v in original_worker.items() if k not in ('files', 'readme', 'patch')}
                == {k: v for k, v in worker_proposal.items() if k not in
                    ('files', 'readme', 'patch', 'source_generation', 'predecessor_manifest', 'repair')}
            and repair['path'] == repair_path
            and repair['before'] == {"bytes":19939,"sha256":"27c422af23b93adc05c3a8619015c9a0af0506e9f20d2b2d8ff319e85e460e33"}
            and repair['after'] == {"bytes":20118,"sha256":"8b97a5446abdab7071d02587cbe9fcea071ff8c383f5e435ca202fdc8fb56545"}
            and all(repair[key] is False for key in ('production_changed', 'test_names_changed',
                'assertions_changed', 'crate_or_module_lint_changed', 'observed_worker_attempt_passed')),
            'original worker proposal retained with sole reviewed test-local repair')
    old_rows = {row['path']: row for row in original_worker['files']}
    new_rows = {row['path']: row for row in worker_proposal['files']}
    require(len(old_rows) == len(new_rows) == len(original_worker['files']) == len(worker_proposal['files']) == 11
            and set(old_rows) == set(new_rows)
            and all(new_rows[name] == (dict(row, after=repair['after']) if name == repair_path else row)
                    for name, row in old_rows.items())
            and old_rows[repair_path]['after'] == repair['before'],
            'eleven unchanged preimages and exact one-test postimage transition')

    require(worker_proposal['schema'] == 'ferric-readiness40-bank-scoped-warm-worker-source-v2'
            and len(worker_proposal['files']) == 11
            and compact(worker_proposal['base']['worker_receipt']) == BASE_WORKER_COMPLETE
            and compact(worker_proposal['base']['worker_sources']) == BASE_WORKER_SOURCES
            and worker_proposal['conditional'] == dict(worker_files=216, worker_passed=760,
                worker_ignored=4, worker_inventory=764, coupled_runtime_files=822,
                coupled_source_files_including_four_helpers=1042)
            and all(worker_proposal[k] is False for k in ('compiled', 'tested', 'canonical_changed',
                'native_execution', 'runtime_changed', 'default_group_policy_changed',
                'currentness_temporal_equivalence_claim', 'performance_claim',
                'numerical_acceptance', 'full_launch_admitted', 'allocation_preflights_changed')),
            'qualified worker source lineage')
    runtime_proposal = json.loads(bodies['runtime-proposal.json'])
    require(runtime_proposal['schema'] == 'ferric-scoped-bank-rearm-runtime-source-v1'
            and compact(worker_proposal['requires']['runtime_manifest']) == fixed['runtime-proposal.json']
            and runtime_proposal['conditional']['runtime_map_rows'] == 822
            and runtime_proposal['conditional']['runtime_passed'] == 1152
            and runtime_proposal['scope']['ordinary_rearm_unchanged'] is True
            and runtime_proposal['scope']['temporal_equivalence_claim'] is False,
            'same reviewed bank runtime is qualified by the coupled worker result')
    worker_rows = {'ferric/' + r['path']: r for r in worker_proposal['files']}
    require(len(worker_rows) == 11 and sum(r['before'] is None for r in worker_rows.values()) == 4
            and all(n.startswith(WORKER_PREFIX) and n.endswith('.rs') and r['repository'] == 'ferric'
                    for n, r in worker_rows.items()), 'eleven closed worker paths with four additions')
    require(all(worker_rows.get('ferric/' + r['path']) == {k: r[k] for k in ('repository', 'path', 'before', 'after')}
                for r in proposal['requires']['imported_policy_rows'])
            and len(proposal['requires']['imported_policy_rows']) == 2,
            'exact imported worker policy and test source rows')
    old = {n: compact(v) for n, v in old_map.items() if n.startswith('ferric/')}
    require(len(old) == 1267 and sum(n.startswith(WORKER_PREFIX) for n in old) == 212
            and all(old_map[n]['path'] == OLD_ROOT + '/' + n for n in old),
            'one actual1267 parent source closure with212 worker bodies')
    pre_worker = {n: r for n, r in old.items() if n.startswith(WORKER_PREFIX)}
    for n, row in worker_rows.items():
        require(pre_worker.get(n) == row['before'], 'actual worker preimage or absence ' + n)
        pre_worker[n] = row['after']
    qualified_worker = {n: compact(r) for n, r in worker_map.items() if n.startswith(WORKER_PREFIX)}
    require(len(qualified_worker) == 216 and set(qualified_worker) == set(pre_worker)
            and {n: compact(r) for n, r in worker['preformat_sources'].items() if n.startswith(WORKER_PREFIX)}
                == pre_worker
            and all(qualified_worker[n] == r for n, r in pre_worker.items() if n not in worker_rows)
            and all(worker_map[n]['path'] == WORKER_ROOT + '/' + n for n in qualified_worker)
            and sum(n.startswith('fe2o3/') for n in worker_map) == 822,
            'only eleven qualified worker format paths; all216 final worker bodies preserved')
    parent_rows = {'ferric/' + r['path']: r for r in proposal['files']}
    require(len(parent_rows) == len(proposal['files']) == 7
            and sum(r['before'] is None for r in parent_rows.values()) == 2
            and all(n.startswith('ferric/' + PARENT_REL + '/') and n.endswith('.rs')
                    and r['repository'] == 'ferric' for n, r in parent_rows.items()),
            'exact seven parent paths with two additions')
    expected = {n: r for n, r in old.items() if not n.startswith(WORKER_PREFIX)}
    expected.update(qualified_worker)
    for n, row in parent_rows.items():
        require(expected.get(n) == row['before'], 'actual parent preimage or absence ' + n)
        expected[n] = row['after']
    require(len(expected) == 1273 and not set(parent_rows) & set(worker_rows),
            '1273 Ferric bodies with disjoint ownership')
    rows = dict(worker_rows, **parent_rows)
    scopes = base['tests']
    expected_lineage = set(fixed) | {'parent-lib-list.stdout', 'metadata.stdout'}
    expected_lineage |= {name + suffix for name in scopes for suffix in ('.stdout', '.command.json')}
    require(set(bodies) == expected_lineage and len(bodies) == 122, 'closed122 direct lineage bodies')
    for name in expected_lineage - set(fixed):
        require(compact(readset[name]) == compact(base['raw'][name]), 'actual baseline raw ' + name)
    listed = core.inventory(bodies['parent-lib-list.stdout'].decode())
    require(listed == base['inventory'], 'actual972-name parent inventory')
    for name, prior in scopes.items():
        require(core.outcomes(bodies[name + '.stdout'].decode()) == prior
                and prior['failed'] == prior['ignored'] == 0, 'preserved raw selected outcomes ' + name)
        command = json.loads(bodies[name + '.command.json'])
        phase = next(p for p in base['phases'] if p['label'] == name)
        require(command['argv'] == phase['argv'], 'actual selected command join')
    require({n: compact(r) for n, r in before.items() if n.startswith('ferric/')} == expected
            and len(before) == 1276, 'complete actual baseline, qualified worker and seven parent overlays')
    require(inputs['parent_overlay'] == sorted(parent_rows), 'only seven parent Rust formatting inputs')
    require(h.pin(PARENT / 'Cargo.lock')['sha256'] == LOCK_SHA, 'unchanged parent lock')
    test_groups = (
        (PARENT / 'src/tp_finite_client/long/readiness_bank_scoped_tests.rs',
         'tp_finite_client::long::readiness::bank_scoped::tests::', 4, 4),
        (PARENT / 'src/tp_finite_client/long/readiness_host_timing_tests.rs',
         'tp_finite_client::long::readiness::host_timing::tests::', 16, 2),
        (FERRIC / 'adapters/tp-peer-finite-engineering-worker-v1/src/finite_guarded_mlp_readiness_bank_scoped_v2_tests.rs',
         'finite_guarded_mlp_readiness_bank_scoped_v2::tests::', 8, 8))
    additions = []
    for path, prefix, total, added in test_groups:
        declared = [prefix + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', path.read_text())]
        new = [n for n in declared if n not in listed]
        require(len(declared) == len(set(declared)) == total and len(new) == added,
                'exact source-declared new and inherited methods')
        additions.extend(new)
    additions.sort()
    require(len(additions) == len(set(additions)) == 14
            and additions == sorted(r['qualified_name'] for r in proposal['test_roster']
                if r['target'] == 'parent_library'), 'exact fourteen additive library names')
    bin_names = sorted(r['qualified_name'] for r in proposal['test_roster']
                       if r['target'] == 'parent_readiness_binary')
    source = (PARENT / 'src/bin/ferric-qwen3-guarded-mlp-readiness-engineering.rs').read_text()
    declared_bin = sorted('tests::' + n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', source))
    old_bin = set(core.statuses(scopes['readiness-bin-tests']))
    require(len(bin_names) == 1 and not set(bin_names) & old_bin
            and declared_bin == sorted(old_bin | set(bin_names)), 'exact one additive readiness CLI test')
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
                and inputs['schema'] == 'ferric-guarded-mlp-readiness40-bank-scoped-warm-parent-cpu-input-v1', 'closed parent input')
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
        require(listed == sorted(old_list + additions) and len(listed) == 986, 'old972 plus exact14 library names')
        selected = set()
        list_labels = {'guarded-bin-list': 'guarded-bin-tests', 'readiness-bin-list': 'readiness-bin-tests',
                       'full2303-bin-list': 'full2303-bin-tests'}
        ordered_scopes = [p['label'] for p in base['phases'] if p['label'] in scopes or p['label'] in list_labels]
        require(len(ordered_scopes) == 59 and set(ordered_scopes) == set(scopes) | set(list_labels),
                'all56 original selected and three inventory recipes')
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
            if label == 'readiness-bin-tests':
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
        policy_selector = 'finite_guarded_mlp_readiness_bank_scoped_v2::tests'
        policy_names = {n for n in additions if n.startswith(policy_selector + '::')}
        require(len(policy_names) == 8 and not policy_names & selected, 'eight new disjoint bank-scoped policy methods')
        leaf('parent-readiness-bank-scoped-policy', [cargo, 'test', *common, *feature, '--lib',
             policy_selector, '--', '--test-threads=1'], source=formatted)
        value = core.outcomes((OUT / 'parent-readiness-bank-scoped-policy.stdout').read_text())
        require(core.statuses(value) == {n: 'ok' for n in policy_names}
                and value['failed'] == value['ignored'] == 0, 'all eight imported bank-scoped policy methods')
        tests['parent-readiness-bank-scoped-policy'] = value
        selected |= policy_names
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
        require(len(phases) == 67 and len(tests) == 57 and sum(v['passed'] for v in tests.values()) == 519
                and sum(v['failed'] + v['ignored'] for v in tests.values()) == 0, '67 phases and519 selected passes')
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
    result = dict(schema='ferric-guarded-mlp-readiness40-bank-scoped-warm-parent-cpu-v1', passed=failure is None, failure=failure,
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
        all_selected_parent_tests_executed=len(tests) == 57 and sum(v['passed'] for v in tests.values()) == 519,
        readiness40_bank_scoped_warm_parent_source_added=True,
        readiness40_bank_scoped_warm_native_execution=False, allocation_preflights_changed=False,
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
