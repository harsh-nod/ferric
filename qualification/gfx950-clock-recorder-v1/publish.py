"""Publish retained clock-recorder CPU evidence, never run qualification or a GPU."""
import ast
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
R = Path('/home/harsh/fe2o3-p228-runtime')
Q = F / 'qualification/gfx950-clock-recorder-v1'
PACKAGE = L / 'proposals/p228-gfx950-clock-recorder-cpu-v1'
CPU = L / 'gfx950-clock-recorder-cpu-v228-v1'
PURE = L / 'gfx950-clock-recorder-pure-v228-v1'
FORMAT = L / 'clock-and-down2-format-v228-v3'
PRIOR = L / 'gfx950-clock-cpu-v228-v1'
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
WORKER_ROOT = 'adapters/tp-peer-finite-engineering-worker-v1/'
PACKAGE_SHA = 'db325121121fa198e51668fc6c25a111e858edb917fa50ed0dea82fd5f5ddda4'
PRIOR_SHA = '458732e9e0c67501f33584ff1c6c041f93c0b9021884b504c4a7dfb206610ad3'
PURE_SHA = '28200c1264d47f025f1c7c58381f9cf845ebd3a301cdcf618b673837671cc7ef'
PURE_CONTROLLER_SHA = '7e108806d4ff164e53ddf2959149531ca68fa0ac920ac03473cc3117841a59e3'
FORMAT_SHA = '824e4437d430b9a9165aa462e101e7d61d54905a8229528be8359de6ebef0e8f'
COMMITS = {'ferric': '924703865d9cfcb1dc791af211b7558ff7c58894',
           'fe2o3': '27b53d2b74c1f239988a891a4aed39e089b05663'}
TREES = {'ferric': '66b302c5ee7a71993685df884229514d2196b785',
         'fe2o3': '3c2fa509b7328c4bf7d8e2ef2aab3a0e0c4a422e'}
SUFFIXES = ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode, 'ordinary data replay with bytecode disabled')
    require(len(sys.argv) == 2 and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'CPU_SHA')
    helper = F / 'qualification/native-device-routing-v1/publish.py'
    require(helper.resolve(strict=True) == helper and hashlib.sha256(helper.read_bytes()).hexdigest()
            == 'af21f9226556d79e9a8b3ae9a96b759808ccaca573f348eaa575eb0c8049d9b9',
            'retained data helper, not a tested controller')
    spec = importlib.util.spec_from_file_location('retained_clock_recorder_data_helpers', helper)
    h = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(h)
    h.verify(helper, sha='af21f9226556d79e9a8b3ae9a96b759808ccaca573f348eaa575eb0c8049d9b9')
    cpu = h.document(CPU / 'complete.json', sha=sys.argv[1])
    prior = h.document(PRIOR / 'complete.json', sha=PRIOR_SHA)
    require(cpu['schema'] == 'ferric-p228-gfx950-clock-recorder-cpu-result-v1'
            and cpu['passed'] is True and cpu['error'] is None
            and cpu['postcheck_errors'] == [] and cpu['source_unchanged'] is True
            and cpu['tests_passed'] == 669 and cpu['tests_ignored'] == 4
            and cpu['empty_initial_target'] is True and cpu['external_cargo_cache_reused'] is True,
            'actual completed CPU669 qualification')
    require(prior['schema'] == 'ferric-p228-gfx950-clock-cpu-result-v1'
            and prior['passed'] is True and prior['error'] is None
            and prior['postcheck_errors'] == [] and prior['source_unchanged'] is True
            and prior['tests_passed'] == 648 and prior['tests_ignored'] == 4,
            'actual prior CPU648')
    for value in (cpu, prior):
        require(all(value[k] is False for k in ('gpu_execution', 'numerical_acceptance',
            'performance_claim', 'timestamp_calibration', 'parent_rebuilt', 'production_authority')),
            'CPU-only claim boundary')
    require(cpu['prior_cpu_complete']['path'] == str(E / PRIOR.name / 'complete.json'), 'prior identity')
    h.verify(PRIOR / 'complete.json', h.filepin(cpu['prior_cpu_complete']), sha=PRIOR_SHA)
    require(cpu['source_commits'] == COMMITS and cpu['source_trees'] == TREES, 'clean source generation')
    remote, old_remote = E / CPU.name, E / PRIOR.name
    copies = [(CPU / 'complete.json', 'cpu/complete.json'), (PRIOR / 'complete.json', 'prior/complete.json')]
    phases = set(cpu['phases'])
    require(phases == set(prior['phases']) and len(phases) == 33, 'exact unchanged 33-phase roster')
    raw_names = {name + suffix for name in phases for suffix in SUFFIXES} | {
        'sources-base.json', 'sources-before.json', 'sources-after.json'}
    require(set(cpu['raw']) == set(prior['raw']) == raw_names and len(raw_names) == 168,
            'exact actual raw rosters')
    raw, old_raw = {}, {}
    for directory, value, root, found in ((CPU, cpu, remote, raw), (PRIOR, prior, old_remote, old_raw)):
        for name, pin in value['raw'].items():
            require(h.filepin(pin)['path'] == str(root / name), 'direct raw record')
            found[name] = h.read(directory / name, pin)
            if directory == CPU:
                copies.append((directory / name, 'cpu/' + name))
    for name in sorted(phases):
        command = h.parse(raw[name + '-command.json'])
        require(command == h.relocated(h.parse(old_raw[name + '-command.json']), str(old_remote), str(remote)),
                'unchanged bounded command/environment: ' + name)
        require(command['affinity'] == [8, 9] and command['nice'] == 10
                and command['gpu_execution'] is False and command['expected_exit'] == 0
                and command['cache_cap_bytes'] == 6 << 30, 'owned CPU bounds')
        require(all(command['env'][k] == '' for k in
                    ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
        for value, found in ((cpu, raw), (prior, old_raw)):
            started, result = h.parse(found[name + '-started.json']), h.parse(found[name + '-result.json'])
            require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                    and started['pid'] > 0 and started['pid'] == started['pgid'], 'owned process group')
            require(result == value['phases'][name] and result['exit_code'] == 0
                    and result['reason'] is None and result['group_absent'] is True
                    and 0 <= result['cache_bytes'] <= 6 << 30, 'natural phase completion and reap')
            require(result['stdout_sha256'] == value['raw'][name + '-stdout']['sha256']
                    and result['stderr_sha256'] == value['raw'][name + '-stderr']['sha256'], 'phase streams')

    manifest = h.document(PACKAGE / 'manifest.json', h.filepin(cpu['package_manifest']), sha=PACKAGE_SHA)
    require(manifest['schema'] == 'ferric-p228-gfx950-clock-recorder-cpu-package-v1'
            and manifest['pure_tests'] == 15 and manifest['test_census'] == {'test_run.py': 15}
            and len(manifest['files']) == 4 and {row['path'] for row in manifest['files']}
            == {'run.py', 'test_run.py', 'overlay.json', 'README.md'}, 'frozen four-file package')
    require({path.name for path in PACKAGE.iterdir()} == {row['path'] for row in manifest['files']} | {'manifest.json'},
            'closed package directory')
    package_data = {row['path']: h.read(PACKAGE / row['path'], row) for row in manifest['files']}
    overlay = h.document(PACKAGE / 'overlay.json', h.filepin(cpu['overlay']))
    added = set(cpu['added_worker_tests'])
    require(overlay['schema'] == 'ferric-p228-gfx950-clock-recorder-cpu-overlay-v1'
            and len(overlay['files']) == 12 and overlay['added_worker_tests'] == cpu['added_worker_tests']
            and cpu['added_worker_tests'] == sorted(added) and len(added) == 21, 'twelve worker files and 21 tests')
    inputs = {row['path']: h.filepin(row) for row in cpu['inputs']}
    require(len(inputs) == len(cpu['inputs']) == 196, 'closed unique actual CPU input roster')
    for pin in (cpu['package_manifest'], cpu['overlay'], cpu['source_manifest'], cpu['prior_cpu_complete'],
                *prior['raw'].values()):
        require(inputs.get(pin['path']) == pin, 'actual consumed input join')
    for row in manifest['files']:
        expected_pin = dict(row, path=str(E / PACKAGE.name / row['path']))
        require(inputs.get(expected_pin['path']) == expected_pin, 'compiled package member input')

    runtime = h.inventory(raw['runtime-list-stdout'].decode())
    require(runtime == h.inventory(old_raw['runtime-list-stdout'].decode()) and len(runtime) == 900,
            'unchanged complete runtime inventory')
    old_worker = h.inventory(old_raw['worker-list-stdout'].decode())
    worker = h.inventory(raw['worker-list-stdout'].decode())
    require(len(old_worker) == 444 and old_worker == set(prior['tests']['worker']['names'])
            and not old_worker & added and worker == old_worker | added and len(worker) == 465,
            'exact full worker inventory extension')
    require(set(cpu['tests']) == set(prior['tests']), 'unchanged test cohort roster')
    selected, passed, ignored = set(), 0, 0
    for name, result in cpu['tests'].items():
        stream = 'worker-tests' if name == 'worker' else name
        ignored_names = h.outcomes(raw[stream + '-stdout'].decode(), result)
        old_ignored = h.outcomes(old_raw[stream + '-stdout'].decode(), prior['tests'][name])
        if name == 'worker':
            require(set(result['names']) == worker and result['summaries'] == [[448, 0, 4], [13, 0, 0]]
                    and ignored_names == old_ignored and not ignored_names & added, 'all 461 worker passes, old ignores')
        else:
            names = set(result['names'])
            require(names == set(prior['tests'][name]['names']) and names <= runtime
                    and not names & selected and not ignored_names and not old_ignored, 'unchanged disjoint runtime selection')
            selected.update(names)
        passed += result['passed']
        ignored += result['ignored']
    require(len(selected) == 208 and (passed, ignored) == (669, 4), 'actual named totals')

    source_inputs_path = L / 'clock-recorder-source-inputs-v228-v1.json'
    sources = h.document(source_inputs_path, h.filepin(cpu['source_manifest']))
    require(overlay['source_manifest'] == cpu['source_manifest']
            and sources['schema'] == 'ferric-p228-clean-worker-sources-v1'
            and set(sources['archives']) == {'ferric', 'fe2o3'}, 'paired source input manifest')
    original = {}
    for project, row in sources['archives'].items():
        require(row['commit'] == COMMITS[project] and row['tree'] == TREES[project], 'source commit and tree')
        pin = {key: row[key] for key in ('path', 'bytes', 'sha256')}
        require(inputs.get(pin['path']) == pin and Path(pin['path']).parent == E, 'archive consumed input')
        archive = L / Path(pin['path']).name
        h.verify(archive, h.filepin(pin))
        members, census = h.archive_map(archive, project)
        require(census == cpu['extraction'][project] and not set(original) & set(members), 'actual archive census')
        original.update(members)
    require(original == h.parse(raw['sources-base.json']), 'whole fresh source maps reproduced from archives')
    excluded = {'fe2o3/crates/fe2o3-kfd/README.md', 'ferric/' + WORKER_ROOT + 'README.md'}
    def compiled_base(value):
        return {path: pin for path, pin in value.items() if path not in excluded
                and (path.startswith('fe2o3/') or path.startswith('ferric/' + WORKER_ROOT))}
    require(compiled_base(original) == compiled_base(h.parse(old_raw['sources-after.json'])),
            'unchanged CPU648 runtime/worker base except the two documented READMEs')
    expected, seen = dict(original), set()
    for row in overlay['files']:
        path = Path(row['path'])
        require(not path.is_absolute() and '..' not in path.parts and path.as_posix() == row['path']
                and row['path'].startswith(WORKER_ROOT + 'src/') and path.suffix == '.rs'
                and row['path'] not in seen, 'unique worker-only source change')
        seen.add(row['path'])
        require(row['source'] == 'p228-gfx950-clock-recorder-source-v2/draft/' + row['path'], 'formatted source identity')
        key = 'ferric/' + row['path']
        require(expected.get(key) == row['before'], 'exact compiled preimage')
        h.content(row['after'])
        pin = dict(row['after'], path=str(E / row['source']))
        require(inputs.get(pin['path']) == pin, 'overlay body consumed input')
        h.verify(L / row['source'], pin)
        h.verify(F / path, row['after'])
        copies.append((L / row['source'], 'worker-source/' + row['path']))
        expected[key] = row['after']
    require(expected == h.parse(raw['sources-before.json']) == h.parse(raw['sources-after.json']),
            'only the twelve declared worker changes, unchanged throughout qualification')
    live_worker_files, live_runtime_files = 0, 0
    for key, pin in expected.items():
        path = Path(key)
        if path.suffix != '.rs' and path.name not in ('Cargo.toml', 'Cargo.lock'):
            continue
        if key.startswith('ferric/' + WORKER_ROOT):
            h.verify(F / key.removeprefix('ferric/'), pin)
            live_worker_files += 1
        elif key.startswith('fe2o3/crates/fe2o3-kfd/'):
            h.verify(R / key.removeprefix('fe2o3/'), pin)
            live_runtime_files += 1

    require(cpu['metadata'] == h.relocated(prior['metadata'], str(old_remote), str(remote)),
            'same qualified Cargo dependency/local manifest closure')
    metadata = h.parse(raw['metadata-stdout'])
    require(metadata['target_directory'] == str(remote / 'target')
            and len(metadata['packages']) == cpu['metadata']['package_count'] == 39, 'actual Cargo metadata')
    local, external = {}, []
    for package in metadata['packages']:
        if package['source'] is None:
            require(package['name'] not in local, 'unique local package')
            local[package['name']] = package['manifest_path']
        else:
            external.append({key: package[key] for key in ('name', 'version', 'source')}
                            | {'manifest': package['manifest_path']})
    require(local == cpu['metadata']['local'] and external == [{key: row[key] for key in
            ('name', 'version', 'source', 'manifest')} for row in cpu['metadata']['external']], 'actual metadata receipt join')
    runtime_package = next(row for row in metadata['packages'] if row['name'] == 'fe2o3-kfd')
    features = next(row['features'] for row in metadata['resolve']['nodes'] if row['id'] == runtime_package['id'])
    require('engineering-gfx950' in features and 'live-validation' not in features, 'CPU runtime feature closure')
    require(set(cpu['binaries']) == {WORKER}, 'only worker built')
    binary = cpu['binaries'][WORKER]
    records = [h.parse(line) for line in raw['worker-build-stdout'].splitlines() if line.startswith(b'{')]
    require([row['success'] for row in records if row.get('reason') == 'build-finished'] == [True], 'Cargo build success')
    artifacts = [row for row in records if row.get('reason') == 'compiler-artifact' and row.get('executable')]
    require(artifacts == [binary['artifact']], 'actual worker compiler artifact')
    artifact = binary['artifact']
    require(artifact['executable'] == h.filepin(binary['binary'])['path'] == str(remote / 'target/debug' / WORKER)
            and artifact['target']['name'] == WORKER and artifact['target']['kind'] == ['bin']
            and artifact['profile'] == prior['binaries'][WORKER]['artifact']['profile']
            and artifact['profile']['test'] is False and artifact['profile']['opt_level'] == '2'
            and artifact['profile']['debug_assertions'] is True and artifact['profile']['overflow_checks'] is True
            and artifact['manifest_path'] == str(remote / 'sources/ferric' / WORKER_ROOT / 'Cargo.toml'),
            'same checked optimized worker build profile')
    worker_path = L / 'gfx950-clock-recorder-worker-v228-v1'
    worker_pin = h.verify(worker_path, binary['binary'])
    with worker_path.open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'retained executable ELF')

    pure = h.document(PURE / 'complete.json', sha=PURE_SHA)
    require(pure['schema'] == 'ferric-p228-gfx950-clock-recorder-pure-v1' and pure['passed'] is True
            and pure['tests'] == 15 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True
            and pure['manifest_sha256'] == PACKAGE_SHA and pure['controller_sha256'] == PURE_CONTROLLER_SHA,
            'actual frozen pure15 qualification')
    require(all(pure[k] is False for k in ('native_execution', 'gpu_execution', 'numerical_acceptance',
                'full_model_acceptance', 'production_authority', 'performance_claim')), 'pure synthetic scope')
    before = h.document(PURE / 'sources-before.json', h.filepin(pure['sources_before']))
    after = h.document(PURE / 'sources-after.json', h.filepin(pure['sources_after']))
    require(before == after and set(before) == set(package_data)
            and pure['source_sha256'] == pure['sources_before']['sha256'], 'unchanged pure sources')
    for row in manifest['files']:
        require(before[row['path']] == dict(row, path=str(E / PACKAGE.name / row['path'])), 'pure/CPU package identity')
    log = h.read(PURE / 'tests.log', h.filepin(pure['transcript'])).decode()
    declared = {(method.name, 'test_run.' + node.name) for node in ast.parse(package_data['test_run.py']).body
                if isinstance(node, ast.ClassDef) for method in node.body
                if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')}
    observed = re.findall(r'^(test_\w+) \((test_run\.\w+)\) \.\.\. ok$', log, re.MULTILINE)
    require(len(declared) == len(observed) == 15 and set(observed) == declared
            and re.search(r'^Ran 15 tests in [^\n]+\n\nOK\s*$', log, re.MULTILINE), '15 actual named pure outcomes')
    pure_controller = L / 'run_gfx950_clock_recorder_pure_p228_v1.py'
    h.verify(pure_controller, sha=PURE_CONTROLLER_SHA)

    formatted = h.document(FORMAT / 'complete.json', sha=FORMAT_SHA)
    require(formatted['schema'] == 'ferric-clock-and-down2-format-v1' and formatted['passed'] is True
            and formatted['gpu_execution'] is False and formatted['tests_executed'] is False
            and sum(len(rows) for rows in formatted['formatted'].values()) == 23, 'format-only 23-file receipt')
    for name, pin in formatted['raw'].items():
        require(h.filepin(pin)['path'] == str(E / FORMAT.name / name), 'direct format record')
        h.verify(FORMAT / name, pin)
        copies.append((FORMAT / name, 'format/' + name))
    require(h.document(FORMAT / 'sources-after.json', formatted['raw']['sources-after.json']) == formatted['formatted'],
            'actual formatted map')
    for name, result in formatted['phases'].items():
        require(h.document(FORMAT / (name + '-result.json'), formatted['raw'][name + '-result.json']) == result
                and result['exit_code'] == 0 and result['reason'] is None and result['group_absent'] is True
                and result['stdout_sha256'] == formatted['raw'][name + '-stdout']['sha256']
                and result['stderr_sha256'] == formatted['raw'][name + '-stderr']['sha256'], 'actual formatting phases')
    formatted_worker = formatted['formatted']['p228-gfx950-clock-recorder-v1']
    require(set(formatted_worker) == seen, 'only the twelve worker bodies qualified here')
    for row in overlay['files']:
        require(formatted_worker[row['path']] == dict(row['after'], path=str(E / row['source'])), 'format/CPU/live source join')
    format_controller = L / Path(formatted['controller']['path']).name
    h.verify(format_controller, h.filepin(formatted['controller']))
    copies += [(FORMAT / 'complete.json', 'format/complete.json'), (source_inputs_path, 'source-inputs.json'),
               (pure_controller, 'tools/' + pure_controller.name), (format_controller, 'tools/' + format_controller.name),
               (Path(__file__).resolve(), 'publish.py')]
    copies += [(PURE / name, 'pure/' + name) for name in ('complete.json', 'sources-before.json', 'sources-after.json', 'tests.log')]
    copies += [(PACKAGE / name, 'controller/' + name) for name in ('manifest.json', *sorted(package_data))]

    # Authenticate every copied body before making any publication directory.
    payloads, ledger = {}, {}
    for source, name in copies:
        require(name not in payloads and not Path(name).is_absolute() and '..' not in Path(name).parts,
                'unique publication member')
        payloads[name] = h.read(source)
        ledger[name] = {key: h.CHECKED[str(source)][key] for key in ('bytes', 'sha256')}
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical qualification parent')
    readme_pin = None
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir()
                and {path.name for path in Q.iterdir()} == {'README.md'},
                'only the root-authored README may precede publication')
        readme_pin = h.verify(Q / 'README.md')
    for path, pin in h.CHECKED.items():
        require(h.fingerprint(Path(path)) == pin, 'pre-publication input recheck')
    result = dict(schema='ferric-p228-gfx950-clock-recorder-public-result-v1', passed=True,
        cpu_receipt=h.CHECKED[str(CPU / 'complete.json')], prior_cpu_receipt=cpu['prior_cpu_complete'],
        pure_receipt=h.CHECKED[str(PURE / 'complete.json')], format_receipt=h.CHECKED[str(FORMAT / 'complete.json')],
        package_manifest=cpu['package_manifest'], cpu_receipt_sha256=sys.argv[1],
        cpu_tests_passed=669, cpu_tests_ignored=4, runtime_tests_passed=208, runtime_inventory=900,
        worker_tests_passed=461, worker_inventory=465, new_worker_tests=21, pure_tests_passed=15,
        phases=33, raw_records=168, worker_files=12, source_files=len(expected),
        live_worker_files_joined=live_worker_files, live_runtime_files_joined=live_runtime_files,
        source_commits=COMMITS, source_trees=TREES, compiled_source_unchanged=True,
        source_archive_maps_replayed=True, same_cpu648_compiled_base=True,
        worker=binary['binary'], locally_retained_worker=worker_pin, retained=ledger,
        root_authored_readme=readme_pin,
        formatted_files=23, formatted_worker_files_qualified=12,
        formatted_parent_and_down2_qualified_here=False, parent_rebuilt=False,
        native_clock_sampling=False, gpu_execution=False, timestamp_calibration=False,
        gpu_clock_domain_relationship_established=False, cross_device_clock_alignment=False,
        numerical_acceptance=False, full_model_acceptance=False, performance_claim=False,
        production_authority=False, sustained_2048_256=False,
        all_cpu_input_bodies_locally_rehashed=False, registry_bodies_locally_rehashed=False,
        original_cpu_controller_rerun=False)
    Q.mkdir(mode=0o755, exist_ok=True)
    for name, body in payloads.items():
        path = Q / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(body)
        h.verify(path, ledger[name])
    for path, pin in h.CHECKED.items():
        require(h.fingerprint(Path(path)) == pin, 'publication input/output postcheck')
    with (Q / 'result.json').open('x', encoding='ascii') as stream:
        json.dump(result, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write('\n')
    print(json.dumps(dict(files=len(ledger), hashes_checked=len(h.CHECKED),
        result=h.fingerprint(Q / 'result.json'))), flush=True)


if __name__ == '__main__':
    main()
