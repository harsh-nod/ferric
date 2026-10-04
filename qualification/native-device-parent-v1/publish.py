"""Replay retained parent CPU evidence and publish bounded records, not executables."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import types

HELPER_SHA = 'af21f9226556d79e9a8b3ae9a96b759808ccaca573f348eaa575eb0c8049d9b9'
PACKAGE_SHA = '753760a5ea122e0d2d0c47fe393d9171c41efef94917bae0011738ae934e69f1'
PURE_CONTROLLER_SHA = '5bef84b48de03a445fdcf139654389e7e885d387eebd7622069f19a8673cfe9e'
FAILED_SHA = 'd5fae24609182d0b8cb3818b3fd67c9647fb7efe8d32f0046440dbb540b52e57'
PRIOR_SHA = '1fc4d17534161e6e7f96e6d0eba0a2455227ba2166623823f6a74f845b93deae'
SOURCE_SHA = '1bf4e1e1271995e032a0394020ca18cea5603cb1f12169221cc9a8310402e394'
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
PARENT = 'adapters/m1-engineering-execution-v1/'
BIN = 'ferric-qwen3-finite-prefix-decode-device-engineering'
DEVICE = 'tp_finite_client::prefix_decode::device_v1::tests::'
DATA = 'prefix_decode_device_observation_v1::tests::'
COMMITS = {'ferric': 'dc04a484a2424baa453f73fc832b2fa890875b44',
           'fe2o3': '9a321f3f98e597a75e8ebeafdda169ec10e12e9e'}
TREES = {'ferric': '46dbd95e493a3bcc47654be139fd2e2a7443cf21',
         'fe2o3': '7d7701d5a453f2d6b5f5c336c8e83e5a836b5b84'}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def helper(repo):
    path = repo / 'qualification/native-device-routing-v1/publish.py'
    require(path.resolve(strict=True) == path and path.is_file() and not path.is_symlink(), 'data helper path')
    raw = path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == HELPER_SHA, 'exact published data helper')
    module = types.ModuleType('qualified_publication_data_helpers')
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    module.verify(path, sha=HELPER_SHA)
    return module, path


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'unoptimized verifier required')
    require(isinstance(PACKAGE_SHA, str) and re.fullmatch('[0-9a-f]{64}', PACKAGE_SHA), 'actual V2 package freeze required')
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('cpu', 'cpu-sha', 'pure', 'pure-sha', 'prior', 'parent', 'repo', 'package',
                'source-inputs', 'archives', 'pure-controller', 'publish-to', 'failed'):
        parser.add_argument('--' + key, required=True)
    a = parser.parse_args()
    repo, package, cpu_dir, pure_dir, prior_dir = map(Path, (a.repo, a.package, a.cpu, a.pure, a.prior))
    H, helper_path = helper(repo)
    failed_dir = Path(a.failed)
    failed = H.document(failed_dir / 'failed.json', sha=FAILED_SHA)
    require(H.CHECKED[str(failed_dir / 'failed.json')]['bytes'] == 157637
            and failed['schema'] == 'ferric-p228-device-parent-cpu-result-v1'
            and failed['passed'] is False and failed['source_unchanged'] is True
            and failed['postcheck_errors'] == [] and failed['binaries'] == {}, 'retained fixture-only failed V1')
    failure_phases = {'parent-metadata', 'parent-lib-list', 'parent-client'}
    require(set(failed['phases']) == failure_phases - {'parent-client'}
            and set(failed['raw']) == {'sources-base.json', 'sources-before.json', 'sources-after.json'}
            | {name + suffix for name in failure_phases for suffix in H.SUFFIXES}, 'failed eighteen-member raw census')
    failure_records = {name: H.read(failed_dir / name, H.filepin(pin)) for name, pin in failed['raw'].items()}
    for name in failure_phases:
        result = H.parse(failure_records[name + '-result.json'])
        require(result['exit_code'] == (101 if name == 'parent-client' else 0)
                and result['reason'] is None and result['group_absent'] is True, 'natural failed-attempt exit and reap')
        require(result['stdout_sha256'] == failed['raw'][name + '-stdout']['sha256']
                and result['stderr_sha256'] == failed['raw'][name + '-stderr']['sha256'], 'failed phase stream joins')
    failed_log = failure_records['parent-client-stdout'].decode()
    require('test result: FAILED. 146 passed; 1 failed; 0 ignored;' in failed_log
            and 'prefix_device_parent_checks_actual_control_for_every_forward_and_stage' in failed_log,
            'actual failed mutation fixture, not a production success')
    cpu = H.document(cpu_dir / 'complete.json', sha=a.cpu_sha)
    pure = H.document(pure_dir / 'complete.json', sha=a.pure_sha)
    prior = H.document(prior_dir / 'complete.json', sha=PRIOR_SHA)
    require(cpu['schema'] == 'ferric-p228-device-parent-cpu-result-v1' and cpu['passed'] is True
            and cpu['error'] is None and cpu['postcheck_errors'] == [] and cpu['source_unchanged'] is True
            and cpu['tests_passed'] == 257 and cpu['tests_ignored'] == 0 and cpu['parent_rebuilt'] is True
            and cpu['empty_initial_target'] is True and cpu['external_cargo_cache_reused'] is True,
            'actual parent257 qualification')
    require(all(cpu[key] is False for key in ('worker_rebuilt', 'gpu_execution', 'numerical_acceptance',
        'performance_claim', 'timestamp_calibration', 'compiler_hsaco_reproduced', 'production_authority')),
        'parent-only CPU scope')
    require(prior['schema'] == 'ferric-p228-host-policy-cpu-result-v1' and prior['passed'] is True
            and prior['tests_passed'] == 633 and prior['tests_ignored'] == 4 and prior['source_unchanged'] is True,
            'historical CPU633 baseline, not a new worker qualification')
    H.verify(prior_dir / 'complete.json', H.filepin(cpu['prior_cpu_complete']))
    require(cpu['prior_cpu_complete']['sha256'] == PRIOR_SHA
            and cpu['source_commits'] == COMMITS and cpu['source_trees'] == TREES, 'source and prior lineage')

    manifest = H.document(package / 'manifest.json', H.filepin(cpu['package_manifest']))
    require(cpu['package_manifest']['sha256'] == PACKAGE_SHA
            and manifest['schema'] == 'ferric-p228-device-parent-cpu-package-v1'
            and manifest['pure_tests'] == 16 and manifest['test_census'] == {'test_run.py': 16}
            and len(manifest['files']) == 4
            and {row['path'] for row in manifest['files']} == {'run.py', 'test_run.py', 'README.md', 'overlay.json'},
            'frozen CPU controller package')
    package_data = {row['path']: H.read(package / row['path'], row) for row in manifest['files']}
    require({path.name for path in package.iterdir()} == set(package_data) | {'manifest.json'}, 'package census')
    overlay = H.parse(package_data['overlay.json'])
    H.verify(package / 'overlay.json', H.filepin(cpu['overlay']))
    require(overlay['schema'] == 'ferric-p228-device-parent-cpu-overlay-v1'
            and len(overlay['files']) == 7 and overlay['added_parent_tests'] == cpu['added_parent_tests'], 'source overlay join')
    additions = set(cpu['added_parent_tests']['lib'])
    new_bin_tests = set(cpu['added_parent_tests']['bin'])
    require(len(additions) == 21 and len(new_bin_tests) == 1
            and sum(name.startswith(DEVICE) for name in additions) == 9
            and sum(name.startswith(DATA) for name in additions) == 12, 'exact source-frozen additions')
    inputs = {row['path']: H.filepin(row) for row in cpu['inputs']}
    require(len(inputs) == len(cpu['inputs']) == 233, 'exact unique input census')
    for pin in (cpu['package_manifest'], cpu['overlay'], cpu['source_manifest'], cpu['prior_cpu_complete']):
        require(inputs.get(pin['path']) == pin, 'direct receipt input joins')
    package_remote = str(Path(cpu['package_manifest']['path']).parent)
    for row in manifest['files']:
        require(inputs.get(package_remote + '/' + row['path']) == dict(path=package_remote + '/' + row['path'],
                bytes=row['bytes'], sha256=row['sha256']), 'compiled package member input')
    for key in ('runner', 'helper', 'extractor', 'stable_environment'):
        pin = prior[key]
        require(inputs.get(pin['path']) == pin, 'historical helper input')
    for pin in prior['raw'].values():
        require(inputs.get(pin['path']) == pin, 'historical raw input pin')

    remote = str(Path(H.filepin(cpu['raw']['sources-base.json'])['path']).parent)
    old_remote = str(Path(prior['raw']['sources-base.json']['path']).parent)
    require(Path(remote).parent == E and re.fullmatch('device-parent-cpu-v228-v[1-9][0-9]*', Path(remote).name),
            'fresh parent qualification root')
    old_phases = set(prior['phases']) - {'worker-metadata', 'worker-list', 'worker-tests', 'worker-build'}
    phases = old_phases | {'parent-device-data', BIN + '-list', BIN + '-tests'}
    require(len(old_phases) == 38 and len(phases) == 41 and set(cpu['phases']) == phases, 'closed 41 parent leaves')
    raw_names = {name + suffix for name in phases for suffix in H.SUFFIXES}
    raw_names.update(('sources-base.json', 'sources-before.json', 'sources-after.json'))
    require(set(cpu['raw']) == raw_names and len(raw_names) == 208, '208 retained raw members')
    retained = {}
    for name, pin in cpu['raw'].items():
        require(H.filepin(pin)['path'] == remote + '/' + name, 'direct raw member')
        retained[name] = H.read(cpu_dir / name, pin)
    commands = {}
    for name in old_phases:
        old_command = H.document(prior_dir / (name + '-command.json'), prior['raw'][name + '-command.json'])
        commands[name] = H.relocated(old_command, old_remote, remote)
    commands['parent-builds']['argv'][-1:-1] = ['--bin', BIN]
    commands['parent-device-data'] = H.parse(json.dumps(commands['parent-host-data-v2']))
    argv = commands['parent-device-data']['argv']
    require(argv[argv.index('--lib') + 1] == 'prefix_decode_host_observation_v2::', 'old data selector')
    argv[argv.index('--lib') + 1] = DATA
    old_bin = 'ferric-qwen3-finite-prefix-decode-host-policy-engineering'
    for suffix in ('-list', '-tests'):
        command = H.parse(json.dumps(commands[old_bin + suffix]))
        require(command['argv'][command['argv'].index('--bin') + 1] == old_bin, 'old bin selector')
        command['argv'][command['argv'].index('--bin') + 1] = BIN
        commands[BIN + suffix] = command
    for name in phases:
        require(H.parse(retained[name + '-command.json']) == commands[name], 'exact command/environment: ' + name)
        started = H.parse(retained[name + '-started.json'])
        result = H.parse(retained[name + '-result.json'])
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int and started['pid'] > 0
                and started['pid'] == started['pgid'], 'owned process group')
        require(result == cpu['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and 0 <= result['cache_bytes'] <= 6 << 30,
                'natural zero and reaped group')
        require(result['stdout_sha256'] == cpu['raw'][name + '-stdout']['sha256']
                and result['stderr_sha256'] == cpu['raw'][name + '-stderr']['sha256'], 'stream joins')

    library = H.inventory(retained['parent-lib-list-stdout'].decode())
    old_library = H.inventory(H.read(prior_dir / 'parent-lib-list-stdout', prior['raw']['parent-lib-list-stdout']).decode())
    require(not old_library.intersection(additions) and library == old_library | additions, 'exact full library inventory')
    old_tests = set(prior['tests']) - {'worker'}
    require(len(old_tests) == 23 and set(cpu['tests']) == old_tests | {'parent-device-data', BIN}, 'test target census')
    selected, passed = set(), 0
    for name, result in cpu['tests'].items():
        binary = name.startswith('ferric-qwen3-finite-')
        raw = retained[name + ('-tests-stdout' if binary else '-stdout')].decode()
        require(not H.outcomes(raw, result) and result['ignored'] == 0, 'actual nonignored outcomes')
        names = set(result['names'])
        if name == BIN:
            expected_names = new_bin_tests
        elif name == 'parent-device-data':
            expected_names = {test for test in additions if test.startswith(DATA)}
        else:
            previous = prior['tests'][name]
            previous_raw = H.read(prior_dir / (name + ('-tests-stdout' if binary else '-stdout')),
                                  prior['raw'][name + ('-tests-stdout' if binary else '-stdout')]).decode()
            require(not H.outcomes(previous_raw, previous), 'old selected tests really passed')
            expected_names = set(previous['names'])
            if name == 'parent-client':
                expected_names |= {test for test in additions if test.startswith(DEVICE)}
        require(names == expected_names, 'preserved old and exact new names')
        if binary:
            require(H.inventory(retained[name + '-list-stdout'].decode()) == names, 'actual bin inventory')
        else:
            require(names <= library and not selected.intersection(names), 'disjoint selected library cases')
            selected.update(names)
        passed += result['passed']
    require(len(selected) == 245 and additions <= selected and passed == 257, 'parent-only actual totals')

    sources = H.document(Path(a.source_inputs), H.filepin(cpu['source_manifest']), sha=SOURCE_SHA)
    require(overlay['source_manifest'] == cpu['source_manifest']
            and sources['schema'] == 'ferric-p228-clean-worker-sources-v1'
            and set(sources['archives']) == {'ferric', 'fe2o3'}, 'paired source manifest')
    original = {}
    for project, row in sources['archives'].items():
        require(row['commit'] == COMMITS[project] and row['tree'] == TREES[project], 'pushed archive generation')
        pin = {key: row[key] for key in ('path', 'bytes', 'sha256')}
        require(inputs.get(pin['path']) == pin, 'source archive input')
        archive = Path(a.archives) / Path(pin['path']).name
        H.verify(archive, H.filepin(pin))
        members, census = H.archive_map(archive, project)
        require(census == cpu['extraction'][project], 'extraction census')
        original.update(members)
    require(original == H.parse(retained['sources-base.json']), 'archive map equals compiled base')
    expected, changed, seen = dict(original), [], set()
    for row in overlay['files']:
        require(row['path'].startswith(PARENT) and row['path'] not in seen
                and '..' not in Path(row['path']).parts, 'unique parent destination')
        seen.add(row['path'])
        key = 'ferric/' + row['path']
        require(expected.get(key) == row['before'], 'compiled source preimage')
        H.content(row['after'])
        require(inputs.get(str(E / row['source'])) == dict(path=str(E / row['source']), **row['after']), 'overlay input body')
        live = H.verify(repo / row['path'], row['after'])
        expected[key] = row['after']
        changed.append(dict(path=row['path'], before=row['before'], after=row['after'], live=live))
    require(len(seen) == 7 and expected == H.parse(retained['sources-before.json'])
            == H.parse(retained['sources-after.json']), 'exact seven-file overlay and source stability')
    failed_sources = H.parse(failure_records['sources-before.json'])
    changed_fixture = 'ferric/' + PARENT + 'src/tp_finite_client/prefix_decode/device_v1_tests.rs'
    require(failed_sources == H.parse(failure_records['sources-after.json'])
            and set(failed_sources) == set(expected)
            and {name for name in expected if expected[name] != failed_sources[name]} == {changed_fixture},
            'only the intended test fixture changed since the retained failed source generation')

    metadata = H.parse(retained['parent-metadata-stdout'])
    require(len(metadata['packages']) == len({row['id'] for row in metadata['packages']})
            == cpu['metadata']['package_count'] == 209 and metadata['target_directory'] == remote + '/target', 'parent Cargo graph')
    old_metadata = H.relocated(prior['metadata']['parent'], old_remote, remote)
    require(cpu['metadata'] == old_metadata and len(old_metadata['local']) == 28
            and len(old_metadata['external']) == 181, 'same relocated parent dependency generation')
    local, external = {}, []
    for row in metadata['packages']:
        path = Path(row['manifest_path'])
        require(path.is_absolute() and '..' not in path.parts and str(path) == row['manifest_path'], 'metadata path')
        if row['source'] is None:
            require(row['name'] not in local, 'unique local package')
            local[row['name']] = str(path)
        else:
            require(path.is_relative_to('/home/harmenon/ferric-asrock-42/toolchain/cargo'), 'external dependency boundary')
            external.append({key: row[key] for key in ('name', 'version', 'source')} | {'manifest': str(path)})
    require(local == old_metadata['local'] and external == [{key: row[key] for key in ('name', 'version', 'source', 'manifest')}
            for row in old_metadata['external']], 'raw metadata manifest joins')

    old_bins = {name for name in prior['binaries'] if name != 'ferric-tp-peer-finite-engineering-worker-v1'}
    require(set(cpu['binaries']) == old_bins | {BIN} and len(cpu['binaries']) == 12, 'all old and new parent binaries')
    records = [H.parse(line) for line in retained['parent-builds-stdout'].splitlines() if line.startswith(b'{')]
    require([row['success'] for row in records if row.get('reason') == 'build-finished'] == [True], 'actual build success')
    artifacts = [row for row in records if row.get('reason') == 'compiler-artifact' and row.get('executable')]
    require(len(artifacts) == 12 and {row['target']['name'] for row in artifacts} == set(cpu['binaries']), 'actual twelve artifacts')
    for row in artifacts:
        name = row['target']['name']
        binary = cpu['binaries'][name]
        require(row == binary['artifact'] and H.filepin(binary['binary'])['path'] == row['executable']
                == remote + '/target/debug/' + name and row['target']['kind'] == ['bin']
                and row['profile']['test'] is False and row['profile']['opt_level'] == '2'
                and row['manifest_path'] == remote + '/sources/ferric/' + PARENT + 'Cargo.toml', 'actual optimized parent artifact')
    parent = H.verify(Path(a.parent), cpu['binaries'][BIN]['binary'])
    with Path(a.parent).open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'retained selected parent ELF')

    require(pure['schema'] == 'ferric-p228-device-parent-pure-v1' and pure['passed'] is True
            and pure['tests'] == 16 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True
            and pure['manifest_sha256'] == PACKAGE_SHA and pure['controller_sha256'] == PURE_CONTROLLER_SHA, 'actual pure16')
    require(all(pure[key] is False for key in ('native_execution', 'gpu_execution', 'numerical_acceptance',
            'full_model_acceptance', 'production_authority', 'performance_claim')), 'pure-only scope')
    pure_records = {name: H.read(pure_dir / name, H.filepin(pure[key])) for key, name in (
        ('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'), ('transcript', 'tests.log'))}
    before = H.parse(pure_records['sources-before.json'])
    require(before == H.parse(pure_records['sources-after.json']) and set(before) == set(package_data)
            and pure['source_sha256'] == pure['sources_before']['sha256'], 'pure source custody')
    for row in manifest['files']:
        require({key: before[row['path']][key] for key in ('bytes', 'sha256')}
                == {key: row[key] for key in ('bytes', 'sha256')}, 'same pure and CPU package')
    declared = {(method.name, 'test_run.' + node.name) for node in ast.parse(package_data['test_run.py']).body
                if isinstance(node, ast.ClassDef) for method in node.body
                if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')}
    log = pure_records['tests.log'].decode()
    observed = re.findall(r'^(test_[A-Za-z0-9_]+) \((test_run\.[A-Za-z0-9_]+)\) \.\.\. ok$', log, re.MULTILINE)
    require(len(declared) == len(observed) == 16 and set(observed) == declared
            and re.search(r'^Ran 16 tests in [^\n]+\n\nOK\s*$', log, re.MULTILINE), 'sixteen actual named pure outcomes')
    pure_controller = H.verify(Path(a.pure_controller), sha=PURE_CONTROLLER_SHA)
    publisher = H.verify(Path(__file__).resolve())
    readme = H.read(Path(__file__).resolve().with_name('README.md'))
    for path, pin in H.CHECKED.items():
        require(H.fingerprint(Path(path)) == pin, 'retained input prepublication recheck')

    summary = dict(schema='ferric-p228-device-parent-publication-v2', passed=True,
        cpu=H.CHECKED[str(cpu_dir / 'complete.json')], pure=H.CHECKED[str(pure_dir / 'complete.json')],
        prior_cpu=H.CHECKED[str(prior_dir / 'complete.json')], publisher=publisher,
        replay_dependency=H.CHECKED[str(helper_path)], replay_dependency_repository_path='qualification/native-device-routing-v1/publish.py',
        pure_controller=pure_controller, parent=parent, original_parent=cpu['binaries'][BIN]['binary'],
        prior_failed_attempt=H.CHECKED[str(failed_dir / 'failed.json')],
        prior_failed_raw_records_rehashed=18,
        fixture_repair='Test-only u64::MAX += 1 overflow changed to ^= 1; production behavior and named test roster unchanged.',
        source_commits=COMMITS, source_trees=TREES, compiled_overlay=changed, source_files=len(expected),
        cpu_tests_passed=257, cpu_tests_ignored=0, pure_tests_passed=16, phase_count=41, raw_cpu_records=208,
        source_archive_maps_replayed=True, local_retained_bytes_audited=True, original_cpu_controller_rerun=False,
        all233_cpu_input_bodies_locally_rehashed=False, external_dependency_manifest_bodies_locally_rehashed=False,
        all12_parent_binary_bodies_locally_rehashed=False, parent_rebuilt=True, worker_rebuilt=False,
        gpu_execution=False, timestamp_calibration=False, numerical_acceptance=False,
        full_model_acceptance=False, production_authority=False, performance_claim=False)
    output = {'README.md': readme, 'publish.py': Path(__file__).read_bytes(),
              'cpu/complete.json': (cpu_dir / 'complete.json').read_bytes(),
              'pure/complete.json': (pure_dir / 'complete.json').read_bytes(),
              'pure/controller.py': Path(a.pure_controller).read_bytes(),
              'controller/manifest.json': (package / 'manifest.json').read_bytes(),
              'history/failed-parent-v1.json': (failed_dir / 'failed.json').read_bytes(),
              'source-inputs.json': Path(a.source_inputs).read_bytes(),
              'result.json': (json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()}
    output.update({'cpu/' + name: raw for name, raw in retained.items()})
    output.update({'pure/' + name: raw for name, raw in pure_records.items()})
    output.update({'controller/' + name: raw for name, raw in package_data.items()})
    output.update({'history/' + name: raw for name, raw in failure_records.items() if name.startswith('parent-client-')})
    require(sum(map(len, output.values())) <= 64 << 20, 'bounded publication payload')
    target = Path(a.publish_to)
    require(target.parent == repo / 'qualification' and target.parent.resolve(strict=True) == target.parent
            and not os.path.lexists(target), 'fresh qualification destination')
    target.mkdir(mode=0o755)
    for name, raw in output.items():
        path = target / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
    for path, pin in H.CHECKED.items():
        require(H.fingerprint(Path(path)) == pin, 'publication input postcheck')
    print(json.dumps(dict(published=str(target), result=H.fingerprint(target / 'result.json'))), flush=True)


if __name__ == '__main__':
    main()
