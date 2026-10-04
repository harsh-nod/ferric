"""Publish retained CPU qualification only; never import/run tested controllers."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import tarfile

PRIOR_SHA = '2d53e512b2ab882b97333c1d1889780c8fed0ab2763b411dc167b8f1735ee5da'
PACKAGE_SHA = 'dbe0c89910178f1c83039a320a629351428c360df21ea269e2541b4f62b29c3b'
PURE_CONTROLLER_SHA = '950c91d092b60cb36c522ac31a53d6f1f9626ec27180b0565a0786052770260b'
COMMITS = {'ferric': '82b0fe5850f38c3ff8d2cbe3640a9880d722467e',
           'fe2o3': '9a321f3f98e597a75e8ebeafdda169ec10e12e9e'}
TREES = {'ferric': '3ab6b3a9bcbc521bbf3b65fa1b4615018c442ce4',
         'fe2o3': '7d7701d5a453f2d6b5f5c336c8e83e5a836b5b84'}
WORKER = 'ferric-tp-peer-finite-engineering-worker-v1'
WORKER_SRC = 'adapters/tp-peer-finite-engineering-worker-v1/src/'
SUFFIXES = ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')
CHECKED = {}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def unique(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, 'duplicate JSON field')
        value[key] = item
    return value


def parse(raw):
    return json.loads(raw, object_pairs_hook=unique,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


def content(value):
    require(type(value) is dict and set(value) == {'bytes', 'sha256'}
            and type(value['bytes']) is int and 0 <= value['bytes'] <= 256 << 20
            and type(value['sha256']) is str and re.fullmatch('[0-9a-f]{64}', value['sha256']), 'content pin')
    return value


def filepin(value):
    require(type(value) is dict and set(value) == {'path', 'bytes', 'sha256'}, 'file pin')
    path = Path(value['path'])
    require(path.is_absolute() and '..' not in path.parts and str(path) == value['path'], 'original file path')
    content({key: value[key] for key in ('bytes', 'sha256')})
    return value


def fingerprint(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, 'canonical local path')
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
            and before.st_size <= 256 << 20, 'bounded unaliased regular file')
    h = hashlib.sha256()
    size = 0
    with path.open('rb') as stream:
        while block := stream.read(1 << 20):
            size += len(block)
            require(size <= before.st_size, 'file grew during hash')
            h.update(block)
        opened = os.fstat(stream.fileno())
    after = path.lstat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns, s.st_nlink)
    require(stamp(before) == stamp(opened) == stamp(after) and size == before.st_size, 'file identity drift')
    return dict(path=str(path), bytes=size, sha256=h.hexdigest())


def verify(path, expected=None, sha=None):
    path = Path(path)
    actual = fingerprint(path)
    if expected is not None:
        wanted = content({key: expected[key] for key in ('bytes', 'sha256')})
        require(all(actual[key] == wanted[key] for key in wanted), 'local retained bytes: ' + str(path))
    if sha is not None:
        require(re.fullmatch('[0-9a-f]{64}', sha) and actual['sha256'] == sha, 'supplied receipt digest')
    require(str(path) not in CHECKED or CHECKED[str(path)] == actual, 'repeated pin drift')
    CHECKED[str(path)] = actual
    return actual


def read(path, expected=None, sha=None):
    actual = verify(path, expected, sha)
    require(actual['bytes'] <= 16 << 20, 'bounded JSON/text source')
    raw = Path(path).read_bytes()
    require(len(raw) == actual['bytes'] and hashlib.sha256(raw).hexdigest() == actual['sha256'], 'read/hash drift')
    return raw


def document(path, expected=None, sha=None):
    return parse(read(path, expected, sha))


def relocated(value, old, new):
    if isinstance(value, str):
        return value.replace(old + '/', new + '/')
    if isinstance(value, list):
        return [relocated(v, old, new) for v in value]
    if isinstance(value, dict):
        return {k: relocated(v, old, new) for k, v in value.items()}
    return value


def inventory(raw):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', raw, re.MULTILINE)
    require(names and len(names) == len(set(names)) and ': benchmark' not in raw, 'test inventory')
    return set(names)


def outcomes(raw, expected):
    rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored(?:, [^\n]*)?)$', raw, re.MULTILINE)
    require(len(rows) == len({n for n, _ in rows}) and {n for n, _ in rows} == set(expected['names']), 'named actual test outcomes')
    counts = [list(map(int, row)) for row in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw)]
    passed = sum(status == 'ok' for _, status in rows)
    ignored = sum(status.startswith('ignored') for _, status in rows)
    require(counts == expected['summaries'] and all(row[1] == 0 for row in counts)
            and passed == expected['passed'] == sum(row[0] for row in counts)
            and ignored == expected['ignored'] == sum(row[2] for row in counts), 'actual test summary')
    return {name for name, status in rows if status.startswith('ignored')}


def archive_map(path, project):
    result, seen, total = {}, set(), 0
    with tarfile.open(path, 'r:gz') as archive:
        for member in archive:
            p = Path(member.name)
            require(not p.is_absolute() and p.parts and p.parts[0] == project
                    and '..' not in p.parts and p.as_posix() == member.name.rstrip('/')
                    and member.name not in seen and len(seen) < 12000, 'archive member path/census')
            seen.add(member.name)
            require(member.isdir() or member.isfile(), 'archive special/link refused')
            if member.isdir():
                continue
            require(0 <= member.size <= 16 << 20 and p.as_posix() not in result, 'archive file extent/alias')
            total += member.size
            require(total <= 256 << 20, 'archive expanded bound')
            with archive.extractfile(member) as stream:
                raw = stream.read((16 << 20) + 1)
            require(len(raw) == member.size, 'archive body extent')
            result[p.as_posix()] = dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
    return result, dict(files=len(result), bytes=total)


def main():
    require(not sys.flags.optimize, 'unoptimized verifier required')
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ('cpu', 'cpu-sha', 'pure', 'pure-sha', 'prior', 'worker', 'repo', 'package',
                'source-inputs', 'archives', 'pure-controller', 'publish-to'):
        parser.add_argument('--' + key, required=True)
    a = parser.parse_args()
    cpu_dir, pure_dir, prior_dir = Path(a.cpu), Path(a.pure), Path(a.prior)
    repo, package = Path(a.repo), Path(a.package)
    cpu = document(cpu_dir / 'complete.json', sha=a.cpu_sha)
    pure = document(pure_dir / 'complete.json', sha=a.pure_sha)
    prior = document(prior_dir / 'complete.json', sha=PRIOR_SHA)
    require(cpu['schema'] == 'ferric-p228-device-routing-cpu-result-v1' and cpu['passed'] is True
            and cpu['error'] is None and cpu['postcheck_errors'] == [] and cpu['source_unchanged'] is True
            and cpu['tests_passed'] == 609 and cpu['tests_ignored'] == 4 and cpu['empty_initial_target'] is True
            and cpu['external_cargo_cache_reused'] is True, 'actual CPU609 qualification')
    require(all(cpu[k] is False for k in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
                'timestamp_calibration', 'parent_rebuilt', 'production_authority')), 'CPU-only authority')
    require(cpu['prior_cpu_complete']['sha256'] == PRIOR_SHA and prior['tests_passed'] == 578
            and prior['passed'] is True and prior['tests_ignored'] == 4, 'qualified prior578')
    verify(prior_dir / 'complete.json', filepin(cpu['prior_cpu_complete']))
    require(cpu['source_commits'] == COMMITS and cpu['source_trees'] == TREES, 'exact paired source lineage')
    remote = str(Path(filepin(cpu['raw']['sources-base.json'])['path']).parent)
    old_remote = str(Path(prior['raw']['sources-base.json']['path']).parent)
    phases = set(cpu['phases'])
    require(phases == set(prior['phases']) and len(phases) == 25, 'all25 phases')
    raw_names = {name + suffix for name in phases for suffix in SUFFIXES} | {'sources-base.json', 'sources-before.json', 'sources-after.json'}
    require(set(cpu['raw']) == raw_names and len(raw_names) == 128, 'all128 raw records')
    retained = {}
    for name, pin in cpu['raw'].items():
        require(filepin(pin)['path'] == remote + '/' + name, 'direct retained phase member')
        retained[name] = read(cpu_dir / name, pin)
    for name in sorted(phases):
        command = parse(retained[name + '-command.json'])
        old_pin = prior['raw'][name + '-command.json']
        old_command = document(prior_dir / (name + '-command.json'), filepin(old_pin))
        require(command == relocated(old_command, old_remote, remote), 'unchanged command/environment recipe: ' + name)
        started = parse(retained[name + '-started.json'])
        result = parse(retained[name + '-result.json'])
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pid'] == started['pgid'], 'owned group start')
        require(result == cpu['phases'][name] and result['exit_code'] == 0 and result['reason'] is None
                and result['group_absent'] is True and 0 <= result['cache_bytes'] <= 6 << 30, 'natural zero/reaped group')
        require(result['stdout_sha256'] == cpu['raw'][name + '-stdout']['sha256']
                and result['stderr_sha256'] == cpu['raw'][name + '-stderr']['sha256'], 'phase stream digest join')

    manifest = document(package / 'manifest.json', filepin(cpu['package_manifest']))
    require(cpu['package_manifest']['sha256'] == PACKAGE_SHA
            and manifest['schema'] == 'ferric-p228-device-routing-cpu-package-v1'
            and manifest['pure_tests'] == 13 and manifest['test_census'] == {'test_run.py': 13}
            and len(manifest['files']) == 4
            and {r['path'] for r in manifest['files']} == {'run.py', 'test_run.py', 'overlay.json', 'README.md'}, 'frozen CPU controller package')
    package_data = {}
    for row in manifest['files']:
        require(set(row) == {'path', 'bytes', 'sha256'}, 'package member')
        package_data[row['path']] = read(package / row['path'], row)
    require({p.name for p in package.iterdir()} == set(package_data) | {'manifest.json'}, 'exact package directory')
    overlay = parse(package_data['overlay.json'])
    verify(package / 'overlay.json', filepin(cpu['overlay']))
    require(overlay['schema'] == 'ferric-p228-device-routing-cpu-overlay-v1' and len(overlay['files']) == 18
            and overlay['added_worker_tests'] == cpu['added_worker_tests']
            and len(cpu['added_worker_tests']) == len(set(cpu['added_worker_tests'])) == 31, 'compiled18 sources and31 additions')
    inputs = {row['path']: row for row in cpu['inputs']}
    require(len(inputs) == len(cpu['inputs']) == 161, 'exact unique CPU input pins')
    for pin in inputs.values(): filepin(pin)
    require(all(filepin(pin) == inputs.get(pin['path']) for pin in (cpu['package_manifest'], cpu['overlay'], cpu['source_manifest'])), 'receipt input custody')
    runtime_list = inventory(retained['runtime-list-stdout'].decode())
    prior_runtime = inventory(read(prior_dir / 'runtime-list-stdout', prior['raw']['runtime-list-stdout']).decode())
    require(runtime_list == prior_runtime, 'complete runtime inventory unchanged')
    worker_list = inventory(retained['worker-list-stdout'].decode())
    prior_worker = set(prior['tests']['worker']['names'])
    require(worker_list == prior_worker | set(cpu['added_worker_tests']) and len(worker_list) == 444
            and not prior_worker.intersection(cpu['added_worker_tests']), 'complete worker regression+31 inventory')
    require(set(cpu['tests']) == set(prior['tests']), 'test cohort roster')
    selected, passed, ignored = set(), 0, 0
    for name, result in cpu['tests'].items():
        raw = retained[('worker-tests' if name == 'worker' else name) + '-stdout'].decode()
        ignored_names = outcomes(raw, result)
        if name == 'worker':
            require(set(result['names']) == worker_list and result['passed'] == 440 and result['ignored'] == 4,
                    'actual worker440 plus4 ignored')
            old_ignored = outcomes(read(prior_dir / 'worker-tests-stdout', prior['raw']['worker-tests-stdout']).decode(), prior['tests']['worker'])
            require(ignored_names == old_ignored and not ignored_names.intersection(cpu['added_worker_tests']), 'no new ignored tests')
        else:
            names = set(result['names'])
            require(names == set(prior['tests'][name]['names']) and names <= runtime_list
                    and not selected.intersection(names) and not ignored_names, 'disjoint unchanged runtime cohort')
            selected.update(names)
        passed += result['passed']; ignored += result['ignored']
    require(len(selected) == 169 and (passed, ignored) == (609, 4), 'replayed totals')

    source_inputs = document(Path(a.source_inputs), filepin(cpu['source_manifest']))
    require(overlay['source_manifest'] == cpu['source_manifest']
            and source_inputs['schema'] == 'ferric-p228-clean-worker-sources-v1'
            and set(source_inputs['archives']) == {'ferric', 'fe2o3'}, 'source manifest join')
    original = {}
    for project, row in source_inputs['archives'].items():
        require(row['commit'] == COMMITS[project] and row['tree'] == TREES[project], 'archive source generation')
        pin = {key: row[key] for key in ('path', 'bytes', 'sha256')}
        require(inputs.get(pin['path']) == pin, 'actual archive input pin')
        path = Path(a.archives) / Path(pin['path']).name
        verify(path, filepin(pin))
        members, census = archive_map(path, project)
        require(census == cpu['extraction'][project], 'actual archive extraction census')
        original.update(members)
    require(original == parse(retained['sources-base.json']), 'full archive map equals actual source base')
    expected = dict(original)
    changed = []
    seen = set()
    for row in overlay['files']:
        path = Path(row['path'])
        require(not path.is_absolute() and '..' not in path.parts and path.as_posix() == row['path']
                and row['path'].startswith(WORKER_SRC) and path.suffix == '.rs' and row['path'] not in seen,
                'unique worker source destination')
        seen.add(row['path'])
        key = 'ferric/' + row['path']
        require(expected.get(key) == row['before'], 'compiled source preimage')
        content(row['after'])
        require(any(p['path'].endswith('/' + row['source'])
                    and {k: p[k] for k in ('bytes', 'sha256')} == row['after'] for p in cpu['inputs']), 'overlay input body pin')
        actual = verify(repo / path, row['after'])
        expected[key] = row['after']
        changed.append(dict(path=row['path'], before=row['before'], after=row['after'], live=actual))
    require(expected == parse(retained['sources-before.json']) == parse(retained['sources-after.json']), 'exact18 overlay and unchanged source maps')
    metadata = parse(retained['metadata-stdout'])
    require(len(metadata['packages']) == cpu['metadata']['package_count'] == 39
            and metadata['target_directory'] == remote + '/target', 'actual Cargo metadata closure')
    local, external = {}, []
    require(len({p['id'] for p in metadata['packages']}) == 39, 'unique Cargo package identities')
    for p in metadata['packages']:
        path = Path(p['manifest_path'])
        require(path.is_absolute() and '..' not in path.parts and str(path) == p['manifest_path'], 'Cargo manifest path')
        if p['source'] is None:
            require(p['name'] not in local, 'unique local Cargo package')
            local[p['name']] = str(path)
        else:
            require(p['source'].startswith('registry+')
                    and path.is_relative_to('/home/harmenon/ferric-asrock-42/toolchain/cargo'), 'external registry package boundary')
            external.append({key: p[key] for key in ('name', 'version', 'source')} | {'manifest': str(path)})
    runtime_names = {'fe2o3-amd-target', 'fe2o3-amdhsa-loader', 'fe2o3-aql', 'fe2o3-drm-uapi',
        'fe2o3-hsaco', 'fe2o3-kfd', 'fe2o3-kfd-uapi', 'fe2o3-runtime-model', 'fe2o3-target-spec'}
    expected_local = {name: remote + '/sources/fe2o3/crates/' + name + '/Cargo.toml' for name in runtime_names}
    expected_local[WORKER] = remote + '/sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml'
    require(local == expected_local == cpu['metadata']['local'] and len(external) == 29, 'exact ten local Cargo manifests')
    require(external == [{k: row[k] for k in ('name', 'version', 'source', 'manifest')}
                         for row in cpu['metadata']['external']], 'retained external metadata join')
    require(all(re.fullmatch('[0-9a-f]{64}', row['manifest_sha256']) for row in cpu['metadata']['external']), 'external manifest digest shape')
    runtime = next(p for p in metadata['packages'] if p['name'] == 'fe2o3-kfd')
    features = next(p['features'] for p in metadata['resolve']['nodes'] if p['id'] == runtime['id'])
    require('engineering-gfx950' in features and 'live-validation' not in features, 'no live GPU feature')

    require(set(cpu['binaries']) == {WORKER}, 'only native worker built')
    binary = cpu['binaries'][WORKER]
    records = [parse(line) for line in retained['worker-build-stdout'].splitlines() if line.startswith(b'{')]
    require([r['success'] for r in records if r.get('reason') == 'build-finished'] == [True], 'actual build success')
    artifacts = [r for r in records if r.get('reason') == 'compiler-artifact' and r.get('executable')]
    require(artifacts == [binary['artifact']], 'actual compiler artifact')
    artifact = artifacts[0]
    require(artifact['executable'] == filepin(binary['binary'])['path'] == remote + '/target/debug/' + WORKER
            and artifact['target']['name'] == WORKER and artifact['target']['kind'] == ['bin']
            and artifact['profile']['test'] is False and artifact['profile']['opt_level'] == '2'
            and artifact['manifest_path'] == remote + '/sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml', 'optimized worker target')
    worker = verify(Path(a.worker), binary['binary'])
    with Path(a.worker).open('rb') as stream:
        require(stream.read(4) == b'\x7fELF', 'retained worker ELF')

    require(pure['schema'] == 'ferric-p228-device-routing-pure-v1' and pure['passed'] is True
            and pure['tests'] == 13 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True
            and pure['manifest_sha256'] == cpu['package_manifest']['sha256'], 'actual pure13 gate')
    require(all(pure[k] is False for k in ('native_execution', 'gpu_execution', 'numerical_acceptance',
                'full_model_acceptance', 'production_authority', 'performance_claim')), 'pure-only scope')
    pure_records = {}
    for key, name in (('sources_before', 'sources-before.json'), ('sources_after', 'sources-after.json'), ('transcript', 'tests.log')):
        pure_records[name] = read(pure_dir / name, filepin(pure[key]))
    before = parse(pure_records['sources-before.json'])
    require(before == parse(pure_records['sources-after.json']) and set(before) == set(package_data)
            and pure['source_sha256'] == pure['sources_before']['sha256'], 'actual pure unchanged four files')
    for row in manifest['files']:
        require({k: before[row['path']][k] for k in ('bytes', 'sha256')} == {k: row[k] for k in ('bytes', 'sha256')}, 'pure/compiled package identity')
    log = pure_records['tests.log'].decode()
    declared = set()
    for node in ast.parse(package_data['test_run.py']).body:
        if isinstance(node, ast.ClassDef):
            for method in node.body:
                if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'):
                    require((method.name, 'test_run.' + node.name) not in declared, 'unique authored pure test')
                    declared.add((method.name, 'test_run.' + node.name))
    observed = re.findall(r'^(test_[A-Za-z0-9_]+) \((test_run\.[A-Za-z0-9_]+)\) \.\.\. ok$', log, re.MULTILINE)
    require(len(declared) == len(observed) == 13 and set(observed) == declared
            and re.search(r'^Ran 13 tests in [^\n]+\n\nOK\s*$', log, re.MULTILINE), 'actual pure unittest transcript')
    require(pure['controller_sha256'] == PURE_CONTROLLER_SHA, 'frozen successor pure controller')
    pure_controller = verify(Path(a.pure_controller), sha=PURE_CONTROLLER_SHA)
    publisher = verify(Path(__file__).resolve())
    readme = read(Path(__file__).resolve().with_name('README.md'))
    for path, pin in CHECKED.items():
        require(fingerprint(Path(path)) == pin, 'final retained input recheck')

    target = Path(a.publish_to)
    require(target.parent == repo / 'qualification' and target.parent.resolve(strict=True) == target.parent
            and not os.path.lexists(target), 'fresh qualification destination')
    target.mkdir(mode=0o755)
    def publish(name, raw):
        path = target / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(raw)
    publish('README.md', readme)
    publish('publish.py', Path(__file__).read_bytes())
    publish('cpu/complete.json', (cpu_dir / 'complete.json').read_bytes())
    for name, raw in retained.items(): publish('cpu/' + name, raw)
    publish('pure/complete.json', (pure_dir / 'complete.json').read_bytes())
    for name, raw in pure_records.items(): publish('pure/' + name, raw)
    publish('pure/controller.py', Path(a.pure_controller).read_bytes())
    publish('controller/manifest.json', (package / 'manifest.json').read_bytes())
    for name, raw in package_data.items(): publish('controller/' + name, raw)
    publish('source-inputs.json', Path(a.source_inputs).read_bytes())
    summary = dict(schema='ferric-p228-device-routing-publication-v1', passed=True,
        cpu=CHECKED[str(cpu_dir / 'complete.json')], pure=CHECKED[str(pure_dir / 'complete.json')],
        publisher=publisher, pure_controller=pure_controller, worker=worker,
        original_worker=binary['binary'], source_commits=COMMITS, source_trees=TREES,
        compiled_overlay=changed, source_files=len(expected), cpu_tests_passed=609, cpu_tests_ignored=4,
        pure_tests_passed=13, phase_count=25, raw_cpu_records=128, source_archive_maps_replayed=True,
        local_retained_bytes_audited=True, original_cpu_controller_rerun=False,
        all161_cpu_input_bodies_locally_rehashed=False, registry_manifest_bodies_locally_rehashed=False,
        gpu_execution=False, timestamp_calibration=False, numerical_acceptance=False,
        parent_rebuilt=False, full_model_acceptance=False, production_authority=False, performance_claim=False)
    publish('result.json', (json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + '\n').encode())
    for path, pin in CHECKED.items():
        require(fingerprint(Path(path)) == pin, 'publication input postcheck')
    print(json.dumps(dict(published=str(target), result=fingerprint(target / 'result.json'))), flush=True)


if __name__ == '__main__':
    main()
