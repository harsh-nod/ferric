"""Publish completed parent clock-mode CPU evidence without launching tested code."""
import ast
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import tarfile
import tomllib

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
F = Path('/home/harsh/ferric-p227-integration')
Q = F / 'qualification/gfx950-clock-parent-v1'
PACKAGE = L / 'proposals/p228-gfx950-clock-parent-cpu-v1'
CPU = L / 'gfx950-clock-parent-cpu-v228-v1'
PURE = L / 'gfx950-clock-parent-pure-v228-v1'
FORMAT = L / 'clock-and-down2-format-v228-v3'
PRIOR = L / 'device-parent-cpu-v228-v2'
PARENT = 'adapters/m1-engineering-execution-v1/'
WORKER = 'adapters/tp-peer-finite-engineering-worker-v1/'
OLD_BIN = 'ferric-qwen3-finite-prefix-decode-device-engineering'
NEW_BIN = 'ferric-qwen3-finite-prefix-decode-device-clock-engineering'
DEVICE = 'tp_finite_client::prefix_decode::device_clock_v2::tests::'
DATA = 'prefix_decode_device_clock_observation_v2::tests::'
OLD_DATA = 'prefix_decode_device_observation_v1::tests::'
PACKAGE_SHA = 'b81aee702d68a07d83bd7efa5174b09d4f079b5e51898979330b4f9224fcb5d5'
PRIOR_SHA = '3b51d61268bee492dd6ded7623742153513c46cd3d246ebc60a6bc9a2555077b'
PURE_SHA = 'cb970375dd13b4d2ce9ff2ceb65d8317aedebd7d871a1ced9f41a3b77a1bd6b3'
PURE_CONTROLLER_SHA = '23d125957230549a9ec729e148239195d7ecdd92b274973c24b678492c144e2d'
FORMAT_SHA = '824e4437d430b9a9165aa462e101e7d61d54905a8229528be8359de6ebef0e8f'
COMMITS = {'ferric': '924703865d9cfcb1dc791af211b7558ff7c58894',
           'fe2o3': '27b53d2b74c1f239988a891a4aed39e089b05663'}
TREES = {'ferric': '66b302c5ee7a71993685df884229514d2196b785',
         'fe2o3': '3c2fa509b7328c4bf7d8e2ef2aab3a0e0c4a422e'}
GIT_RUNTIME = ('git+https://github.com/harsh-nod/fe2o3.git?'
               'rev=faaaf15d68eff996b22951758b1a9fa83317d6d2#faaaf15d68eff996b22951758b1a9fa83317d6d2')
SUFFIXES = ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ
            and sys.dont_write_bytecode, 'unoptimized data replay with bytecode disabled')
    require(len(sys.argv) == 2 and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'CPU_SHA')
    helper = F / 'qualification/native-device-routing-v1/publish.py'
    require(helper.resolve(strict=True) == helper and hashlib.sha256(helper.read_bytes()).hexdigest()
            == 'af21f9226556d79e9a8b3ae9a96b759808ccaca573f348eaa575eb0c8049d9b9', 'retained data helper')
    spec = importlib.util.spec_from_file_location('retained_parent_clock_data_helpers', helper)
    h = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(h)
    h.verify(helper, sha='af21f9226556d79e9a8b3ae9a96b759808ccaca573f348eaa575eb0c8049d9b9')
    cpu = h.document(CPU / 'complete.json', sha=sys.argv[1])
    prior = h.document(PRIOR / 'complete.json', sha=PRIOR_SHA)
    require(cpu['schema'] == 'ferric-p228-gfx950-clock-parent-cpu-result-v1'
            and cpu['passed'] is True and cpu['error'] is None and cpu['postcheck_errors'] == []
            and cpu['source_unchanged'] is True and cpu['tests_passed'] == 275 and cpu['tests_ignored'] == 0
            and cpu['parent_rebuilt'] is True and cpu['empty_initial_target'] is True
            and cpu['external_cargo_cache_reused'] is True, 'actual completed parent275')
    require(prior['schema'] == 'ferric-p228-device-parent-cpu-result-v1'
            and prior['passed'] is True and prior['error'] is None and prior['postcheck_errors'] == []
            and prior['source_unchanged'] is True and prior['tests_passed'] == 257
            and prior['tests_ignored'] == 0 and prior['parent_rebuilt'] is True, 'actual prior parent257')
    for value in (cpu, prior):
        require(all(value[key] is False for key in ('worker_rebuilt', 'gpu_execution', 'numerical_acceptance',
            'performance_claim', 'timestamp_calibration', 'compiler_hsaco_reproduced', 'production_authority')),
            'parent-only CPU claim boundary')
    require(cpu['sibling_runtime_rebuilt'] is False, 'no sibling runtime rebuild claim')
    require(cpu['prior_cpu_complete']['path'] == str(E / PRIOR.name / 'complete.json'), 'prior identity')
    h.verify(PRIOR / 'complete.json', h.filepin(cpu['prior_cpu_complete']), sha=PRIOR_SHA)
    require(cpu['source_commits'] == COMMITS and cpu['source_trees'] == TREES, 'paired archive generation')
    remote, old_remote = E / CPU.name, E / PRIOR.name
    copies = [(CPU / 'complete.json', 'cpu/complete.json'), (PRIOR / 'complete.json', 'prior/complete.json')]
    phases = set(prior['phases']) | {'parent-clock-data', NEW_BIN + '-list', NEW_BIN + '-tests'}
    require(len(prior['phases']) == 41 and len(phases) == 44 and set(cpu['phases']) == phases,
            'closed extended parent phase roster')
    raw, old_raw = {}, {}
    for directory, value, root, found in ((CPU, cpu, remote, raw), (PRIOR, prior, old_remote, old_raw)):
        names = {name + suffix for name in value['phases'] for suffix in SUFFIXES} | {
            'sources-base.json', 'sources-before.json', 'sources-after.json'}
        require(set(value['raw']) == names and len(names) == (223 if directory == CPU else 208), 'exact raw census')
        for name, pin in value['raw'].items():
            require(h.filepin(pin)['path'] == str(root / name), 'direct retained record')
            found[name] = h.read(directory / name, pin)
            if directory == CPU:
                copies.append((directory / name, 'cpu/' + name))
    commands = {name: h.relocated(h.parse(old_raw[name + '-command.json']), str(old_remote), str(remote))
                for name in prior['phases']}
    require(commands['parent-builds']['argv'][-1] == '--message-format=json', 'build recipe terminator')
    commands['parent-builds']['argv'][-1:-1] = ['--bin', NEW_BIN]
    commands['parent-clock-data'] = h.parse(json.dumps(commands['parent-device-data']))
    argv = commands['parent-clock-data']['argv']
    require(argv[argv.index('--lib') + 1] == OLD_DATA, 'old shared schema selector')
    argv[argv.index('--lib') + 1] = DATA
    for suffix in ('-list', '-tests'):
        command = h.parse(json.dumps(commands[OLD_BIN + suffix]))
        require(command['argv'][command['argv'].index('--bin') + 1] == OLD_BIN, 'old binary selector')
        command['argv'][command['argv'].index('--bin') + 1] = NEW_BIN
        commands[NEW_BIN + suffix] = command
    for name in sorted(phases):
        command = h.parse(raw[name + '-command.json'])
        require(command == commands[name], 'exact recipe/environment: ' + name)
        require(command['affinity'] == [8, 9] and command['nice'] == 10 and command['gpu_execution'] is False
                and command['expected_exit'] == 0 and command['cache_cap_bytes'] == 6 << 30, 'bounded CPU leaf')
        require(all(command['env'][key] == '' for key in
                ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')), 'no visible GPU')
    for value, found in ((cpu, raw), (prior, old_raw)):
        for name in value['phases']:
            started, result = h.parse(found[name + '-started.json']), h.parse(found[name + '-result.json'])
            require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                    and started['pid'] > 0 and started['pid'] == started['pgid'], 'owned process group')
            require(result == value['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                    and result['group_absent'] is True and 0 <= result['cache_bytes'] <= 6 << 30,
                    'natural zero and reaped process group')
            require(result['stdout_sha256'] == value['raw'][name + '-stdout']['sha256']
                    and result['stderr_sha256'] == value['raw'][name + '-stderr']['sha256'], 'actual stream joins')

    manifest = h.document(PACKAGE / 'manifest.json', h.filepin(cpu['package_manifest']), sha=PACKAGE_SHA)
    require(manifest['schema'] == 'ferric-p228-gfx950-clock-parent-cpu-package-v1'
            and manifest['pure_tests'] == 20 and manifest['test_census'] == {'test_run.py': 20}
            and len(manifest['files']) == 4 and {row['path'] for row in manifest['files']}
            == {'run.py', 'test_run.py', 'overlay.json', 'README.md'}, 'frozen four-file package')
    require({path.name for path in PACKAGE.iterdir()} == {row['path'] for row in manifest['files']} | {'manifest.json'},
            'closed package directory')
    package_data = {row['path']: h.read(PACKAGE / row['path'], row) for row in manifest['files']}
    overlay = h.document(PACKAGE / 'overlay.json', h.filepin(cpu['overlay']))
    require(overlay['schema'] == 'ferric-p228-gfx950-clock-parent-cpu-overlay-v1'
            and len(overlay['files']) == 20 and overlay['added_parent_tests'] == cpu['added_parent_tests'],
            'exact source/test overlay')
    added = set(cpu['added_parent_tests']['lib'])
    bin_added = set(cpu['added_parent_tests']['bin'])
    require(cpu['added_parent_tests']['lib'] == sorted(added) and len(added) == 17
            and sum(name.startswith(DEVICE) for name in added) == 10
            and sum(name.startswith(DATA) for name in added) == 7 and len(bin_added) == 1, '17 library plus one bin additions')
    inputs = {row['path']: h.filepin(row) for row in cpu['inputs']}
    require(len(inputs) == len(cpu['inputs']) == 242, 'closed unique CPU input roster')
    for pin in (cpu['package_manifest'], cpu['overlay'], cpu['source_manifest'], cpu['prior_cpu_complete'],
                *prior['raw'].values()):
        require(inputs.get(pin['path']) == pin, 'actual consumed input join')
    for row in manifest['files']:
        pin = dict(row, path=str(E / PACKAGE.name / row['path']))
        require(inputs.get(pin['path']) == pin, 'compiled package member input')

    library = h.inventory(raw['parent-lib-list-stdout'].decode())
    old_library = h.inventory(old_raw['parent-lib-list-stdout'].decode())
    require(not old_library & added and library == old_library | added, 'whole library inventory extension')
    require(set(cpu['tests']) == set(prior['tests']) | {'parent-clock-data', NEW_BIN}, 'exact test record roster')
    old_bins = set(prior['binaries'])
    require(len(old_bins) == 12 and OLD_BIN in old_bins, 'historical twelve parent bins')
    selected, passed = set(), 0
    for name, result in cpu['tests'].items():
        is_bin = name in old_bins | {NEW_BIN}
        stream = name + ('-tests' if is_bin else '')
        require(not h.outcomes(raw[stream + '-stdout'].decode(), result) and result['ignored'] == 0,
                'actual passing nonignored outcomes')
        if name == NEW_BIN:
            expected_names = bin_added
        elif name == 'parent-clock-data':
            expected_names = {test for test in added if test.startswith(DATA)}
        else:
            require(not h.outcomes(old_raw[stream + '-stdout'].decode(), prior['tests'][name]), 'old outcomes really passed')
            expected_names = set(prior['tests'][name]['names'])
            if name == 'parent-client':
                expected_names |= {test for test in added if test.startswith(DEVICE)}
        names = set(result['names'])
        require(names == expected_names, 'exact old/new names')
        if is_bin:
            require(h.inventory(raw[name + '-list-stdout'].decode()) == names and len(names) == 1, 'actual bin listing')
        else:
            require(names <= library and not selected & names, 'disjoint library selections')
            selected.update(names)
        passed += result['passed']
    require(len(selected) == 262 and added <= selected and passed == 275, '262 library plus 13 binary passes')

    source_inputs_path = L / 'clock-recorder-source-inputs-v228-v1.json'
    sources = h.document(source_inputs_path, h.filepin(cpu['source_manifest']))
    require(overlay['source_manifest'] == cpu['source_manifest']
            and sources['schema'] == 'ferric-p228-clean-worker-sources-v1'
            and set(sources['archives']) == {'ferric', 'fe2o3'}, 'paired archive source manifest')
    original = {}
    for project, row in sources['archives'].items():
        require(row['commit'] == COMMITS[project] and row['tree'] == TREES[project], 'archive commit/tree')
        pin = {key: row[key] for key in ('path', 'bytes', 'sha256')}
        require(inputs.get(pin['path']) == pin and Path(pin['path']).parent == E, 'consumed archive')
        archive = L / Path(pin['path']).name
        h.verify(archive, h.filepin(pin))
        members, census = h.archive_map(archive, project)
        require(census == cpu['extraction'][project] and not set(original) & set(members), 'actual archive census')
        original.update(members)
    require(original == h.parse(raw['sources-base.json']), 'full archive maps equal actual source base')
    roots = {str(Path(path).parent.relative_to(old_remote / 'sources')) + '/' for path in prior['metadata']['local'].values()}
    require(roots and all(root.startswith('ferric/') for root in roots) and 'ferric/' + PARENT in roots,
            'historical local Ferric dependency roots')
    roots.add('ferric/' + WORKER)
    excluded = {'ferric/' + PARENT + 'README.md', 'ferric/' + WORKER + 'README.md'}
    root_files = {'ferric/Cargo.toml', 'ferric/Cargo.lock', 'ferric/rust-toolchain', 'ferric/rust-toolchain.toml'}
    def compiled_base(value):
        return {path: pin for path, pin in value.items() if path not in excluded and
                (path in root_files or path.startswith('ferric/.cargo/') or any(path.startswith(root) for root in roots))}
    require(compiled_base(original) == compiled_base(h.parse(old_raw['sources-after.json'])),
            'unchanged parent257 compiled base except two documented READMEs')
    expected, seen, parent_paths, worker_paths = dict(original), set(), set(), set()
    for row in overlay['files']:
        path = Path(row['path'])
        require(not path.is_absolute() and '..' not in path.parts and path.as_posix() == row['path']
                and row['path'].startswith((PARENT, WORKER)) and row['path'] not in seen, 'unique source destination')
        seen.add(row['path'])
        is_parent = row['path'].startswith(PARENT)
        directory = 'p228-gfx950-clock-parent-source-v2' if is_parent else 'p228-gfx950-clock-recorder-source-v2'
        require(row['source'] == directory + '/draft/' + row['path'], 'formatted source identity')
        (parent_paths if is_parent else worker_paths).add(row['path'])
        key = 'ferric/' + row['path']
        require(expected.get(key) == row['before'], 'compiled source preimage')
        h.content(row['after'])
        pin = dict(row['after'], path=str(E / row['source']))
        require(inputs.get(pin['path']) == pin, 'overlay consumed input')
        h.verify(L / row['source'], pin)
        h.verify(F / path, pin)
        copies.append((L / row['source'], ('parent-source/' if is_parent else 'shared-worker-source/') + row['path']))
        expected[key] = row['after']
    require(len(parent_paths) == 8 and len(worker_paths) == 12
            and expected == h.parse(raw['sources-before.json']) == h.parse(raw['sources-after.json']),
            'exact twenty-file source closure, immutable during qualification')
    cargo_key = 'ferric/' + PARENT + 'Cargo.toml'
    archive = L / Path(sources['archives']['ferric']['path']).name
    with tarfile.open(archive, 'r:gz') as packed:
        member = packed.getmember(cargo_key)
        require(member.isfile() and member.size == original[cargo_key]['bytes'] and member.size <= 1 << 20, 'original parent manifest')
        with packed.extractfile(member) as stream:
            cargo_before = stream.read((1 << 20) + 1)
    require(len(cargo_before) == original[cargo_key]['bytes']
            and hashlib.sha256(cargo_before).hexdigest() == original[cargo_key]['sha256'], 'original manifest bytes')
    before_toml, after_toml = tomllib.loads(cargo_before.decode()), tomllib.loads(h.read(F / PARENT / 'Cargo.toml').decode())
    bin_row = dict(name=NEW_BIN, path='src/bin/' + NEW_BIN + '.rs', **{'required-features': ['tp-batch-engineering']})
    require(after_toml.get('bin', []).count(bin_row) == 1 and bin_row not in before_toml.get('bin', [])
            and dict(after_toml, bin=[row for row in after_toml['bin'] if row != bin_row]) == before_toml,
            'manifest adds only the new opt-in parent binary')
    live_files = 0
    for key, pin in compiled_base(expected).items():
        path = Path(key)
        if path.suffix == '.rs' or path.name in ('Cargo.toml', 'Cargo.lock'):
            h.verify(F / key.removeprefix('ferric/'), pin)
            live_files += 1

    require(cpu['metadata'] == h.relocated(prior['metadata'], str(old_remote), str(remote)),
            'same locked parent metadata/dependency generation')
    metadata = h.parse(raw['parent-metadata-stdout'])
    require(metadata['target_directory'] == str(remote / 'target')
            and len(metadata['packages']) == cpu['metadata']['package_count'] == 209, 'actual parent metadata closure')
    local, external = {}, []
    for package in metadata['packages']:
        if package['source'] is None:
            require(package['name'] not in local, 'unique local package')
            local[package['name']] = package['manifest_path']
        else:
            external.append({key: package[key] for key in ('name', 'version', 'source')}
                            | {'manifest': package['manifest_path']})
    require(local == cpu['metadata']['local'] and external == [{key: row[key] for key in
            ('name', 'version', 'source', 'manifest')} for row in cpu['metadata']['external']], 'actual metadata summary join')
    runtime = [row for row in metadata['packages'] if row['name'] == 'fe2o3-kfd']
    require(len(runtime) == 1 and runtime[0]['source'] == GIT_RUNTIME
            and not Path(runtime[0]['manifest_path']).is_relative_to(remote / 'sources/fe2o3'),
            'locked Git runtime, not sibling runtime source')
    binary_names = old_bins | {NEW_BIN}
    require(set(cpu['binaries']) == binary_names and len(binary_names) == 13, 'thirteen actual parent artifacts')
    records = [h.parse(line) for line in raw['parent-builds-stdout'].splitlines() if line.startswith(b'{')]
    require([row['success'] for row in records if row.get('reason') == 'build-finished'] == [True], 'actual build completion')
    artifacts = [row for row in records if row.get('reason') == 'compiler-artifact' and row.get('executable')]
    require(len(artifacts) == 13 and {row['target']['name'] for row in artifacts} == binary_names, 'actual Cargo artifact roster')
    for artifact in artifacts:
        name = artifact['target']['name']
        binary = cpu['binaries'][name]
        require(artifact == binary['artifact'] and artifact['executable'] == h.filepin(binary['binary'])['path']
                == str(remote / 'target/debug' / name) and artifact['target']['kind'] == ['bin']
                and artifact['manifest_path'] == str(remote / 'sources/ferric' / PARENT / 'Cargo.toml')
                and artifact['profile'] == prior['binaries'][OLD_BIN]['artifact']['profile']
                and artifact['profile']['test'] is False and artifact['profile']['opt_level'] == '2'
                and artifact['profile']['debug_assertions'] is True and artifact['profile']['overflow_checks'] is True,
                'exact parent artifact and checked build profile')
    parent_path = L / 'gfx950-clock-parent-v228-v1'
    parent_pin = h.verify(parent_path, cpu['binaries'][NEW_BIN]['binary'])
    with parent_path.open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'retained new parent ELF')

    pure = h.document(PURE / 'complete.json', sha=PURE_SHA)
    require(pure['schema'] == 'ferric-p228-gfx950-clock-parent-pure-v1' and pure['passed'] is True
            and pure['tests'] == 20 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True
            and pure['manifest_sha256'] == PACKAGE_SHA and pure['controller_sha256'] == PURE_CONTROLLER_SHA,
            'actual pure20 qualification')
    require(all(pure[key] is False for key in ('native_execution', 'gpu_execution', 'numerical_acceptance',
                'full_model_acceptance', 'production_authority', 'performance_claim')), 'pure-only scope')
    before = h.document(PURE / 'sources-before.json', h.filepin(pure['sources_before']))
    require(before == h.document(PURE / 'sources-after.json', h.filepin(pure['sources_after']))
            and set(before) == set(package_data) and pure['source_sha256'] == pure['sources_before']['sha256'], 'unchanged pure sources')
    for row in manifest['files']:
        require(before[row['path']] == dict(row, path=str(E / PACKAGE.name / row['path'])), 'pure/CPU package identity')
    log = h.read(PURE / 'tests.log', h.filepin(pure['transcript'])).decode()
    declared = {(method.name, 'test_run.' + node.name) for node in ast.parse(package_data['test_run.py']).body
                if isinstance(node, ast.ClassDef) for method in node.body
                if isinstance(method, ast.FunctionDef) and method.name.startswith('test_')}
    observed = re.findall(r'^(test_\w+) \((test_run\.\w+)\) \.\.\. ok$', log, re.MULTILINE)
    require(len(declared) == len(observed) == 20 and set(observed) == declared
            and re.search(r'^Ran 20 tests in [^\n]+\n\nOK\s*$', log, re.MULTILINE), 'twenty actual named pure outcomes')
    pure_controller = L / 'run_gfx950_clock_parent_pure_p228_v1.py'
    h.verify(pure_controller, sha=PURE_CONTROLLER_SHA)

    formatted = h.document(FORMAT / 'complete.json', sha=FORMAT_SHA)
    require(formatted['schema'] == 'ferric-clock-and-down2-format-v1' and formatted['passed'] is True
            and formatted['gpu_execution'] is False and formatted['tests_executed'] is False
            and sum(len(rows) for rows in formatted['formatted'].values()) == 23, 'shared format-only receipt')
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
    formatted_sources = dict(formatted['formatted']['p228-gfx950-clock-parent-v1'],
                             **formatted['formatted']['p228-gfx950-clock-recorder-v1'])
    require(set(formatted_sources) == seen, 'twenty formatted overlay sources, excluding Down2')
    for row in overlay['files']:
        require(formatted_sources[row['path']] == dict(row['after'], path=str(E / row['source'])), 'format/CPU/live source identity')
    format_controller = L / Path(formatted['controller']['path']).name
    h.verify(format_controller, h.filepin(formatted['controller']))
    copies += [(FORMAT / 'complete.json', 'format/complete.json'), (source_inputs_path, 'source-inputs.json'),
               (pure_controller, 'tools/' + pure_controller.name), (format_controller, 'tools/' + format_controller.name),
               (Path(__file__).resolve(), 'publish.py')]
    copies += [(PURE / name, 'pure/' + name) for name in ('complete.json', 'sources-before.json', 'sources-after.json', 'tests.log')]
    copies += [(PACKAGE / name, 'controller/' + name) for name in ('manifest.json', *sorted(package_data))]

    payloads, ledger = {}, {}
    for source, name in copies:
        require(name not in payloads and not Path(name).is_absolute() and '..' not in Path(name).parts, 'unique publication member')
        payloads[name] = h.read(source)
        ledger[name] = {key: h.CHECKED[str(source)][key] for key in ('bytes', 'sha256')}
    require(Q.parent.resolve(strict=True) == Q.parent, 'canonical publication parent')
    readme_pin = None
    if os.path.lexists(Q):
        require(Q.resolve(strict=True) == Q and Q.is_dir() and {path.name for path in Q.iterdir()} == {'README.md'},
                'only root-authored README may precede publication')
        readme_pin = h.verify(Q / 'README.md')
    for path, pin in h.CHECKED.items():
        require(h.fingerprint(Path(path)) == pin, 'pre-publication input recheck')
    result = dict(schema='ferric-p228-gfx950-clock-parent-public-result-v1', passed=True,
        cpu_receipt=h.CHECKED[str(CPU / 'complete.json')], cpu_receipt_sha256=sys.argv[1],
        prior_cpu_receipt=cpu['prior_cpu_complete'], pure_receipt=h.CHECKED[str(PURE / 'complete.json')],
        format_receipt=h.CHECKED[str(FORMAT / 'complete.json')], package_manifest=cpu['package_manifest'],
        cpu_tests_passed=275, cpu_tests_ignored=0, library_tests_passed=262, binary_tests_passed=13,
        library_inventory=len(library), new_parent_tests=18, pure_tests_passed=20,
        phases=44, raw_records=223, parent_files=8, shared_worker_source_files=12,
        source_files=len(expected), live_compiled_files_joined=live_files,
        source_commits=COMMITS, source_trees=TREES, source_archive_maps_replayed=True,
        same_parent257_compiled_base=True, compiled_source_unchanged=True,
        manifest_only_new_bin_registration=True, parent_dependency_generation=cpu['metadata'],
        parent_runtime_git_source=GIT_RUNTIME, parent_runtime_from_sibling_archive=False,
        parent=cpu['binaries'][NEW_BIN]['binary'], locally_retained_parent=parent_pin,
        cargo_artifact_records=13, other_twelve_parent_binaries_locally_rehashed=False,
        parent_rebuilt=True, worker_rebuilt=False, sibling_runtime_rebuilt=False,
        retained=ledger, root_authored_readme=readme_pin, formatted_files=23, down2_qualified_here=False,
        native_clock_sampling=False, gpu_execution=False, timestamp_calibration=False,
        gpu_clock_domain_relationship_established=False, cross_device_clock_alignment=False,
        compiler_hsaco_reproduced=False, numerical_acceptance=False, full_model_acceptance=False,
        performance_claim=False, production_authority=False, sustained_2048_256=False,
        all_cpu_input_bodies_locally_rehashed=False, external_dependency_bodies_locally_rehashed=False,
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
