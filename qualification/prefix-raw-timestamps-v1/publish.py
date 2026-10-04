"""Local CPU publication from retained archives and receipts; never extract/run."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tarfile

L = Path('/home/harsh/ferric-session-evidence/20261001/wave-output-lowering-v216')
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
R = Path('/home/harmenon/ferric-asrock-42')
PL = L / 'proposals'
P = PL / 'p228-prefix-raw-timestamps-cpu-v1'
C = L / 'prefix-raw-timestamps-cpu-v228-v1'
PURE = L / 'prefix-raw-timestamps-pure-v228-v1'
WORKER = L / 'prefix-raw-timestamps-worker-v228-v1'
PRIOR = L / 'state-bank-batch-cpu-v228-v1'
FERRIC = Path('/home/harsh/ferric-p227-integration')
RUNTIME = Path('/home/harsh/fe2o3-p228-runtime')
Q = FERRIC / 'qualification/prefix-raw-timestamps-v1'
BIN = 'ferric-tp-peer-finite-engineering-worker-v1'
PACKAGE_SHA = 'fb938584a0457bc9f0a82007990bb17eb82bd11c999b9fdd24e9bbfb1432c76c'
PRIOR_SHA = '14dab6e776cdc2184457b2865cf7d48b91a71008d49bea040deca68bba75838f'
SOURCE_SHA = '55cca48de457a203ac5b29277920a5ac2c5181387b1c2d06ccd2ccae091a8674'
PURE_RUNNER_SHA = '81d4f76d2fb6174eabd869606c6e0de6a37ce5d4ca9a2936332e41046a75b2e2'
COMMITS = {'ferric': '44e308d72815405f9127473368671e9696c5ebaa',
           'fe2o3': '3d217aabc3c48e9c767fa28a05bd596987c29f9d'}
TREES = {'ferric': 'fb509bfe4adb1afb98a8abb788f2ac74a7b3dc29',
         'fe2o3': 'e549d1ef8c8c2abec85d52838eebfe0f5a2ff46d'}
SOURCE_FILES = (
    'engineering_gfx950_peer_wave_qkv_attention_output_tiles_v6.rs',
    'engineering_gfx950_peer_wave_qkv_attention_output_tiles_v6_tests.rs',
    'engineering_gfx950_peer_wave_qkv_attention_output_tiles_timestamp_tests.rs',
)
PREFIX = 'engineering_gfx950::peer::wave_qkv_attention_output_tiles_v6::'
NEW_TESTS = {PREFIX + 'tests::' + name for name in (
    'resident_v6_raw_entry_invalid_deadline_poison_blocks_both_modes_and_rearm',
    'resident_v6_raw_entry_routes_to_native_validation_before_activation',
)} | {PREFIX + 'timestamp_tests::timestamp_prefix_join_' + name for name in (
    'checks_every_terminal_word_on_both_ranks', 'failure_enters_existing_group_poison_path',
    'preserves_real_state_host_and_raw_observations', 'rejects_foreign_group_or_duplicate_device',
    'rejects_host_timer_substitution', 'rejects_missing_or_extra_observations',
    'rejects_wrong_or_duplicate_rank_order',
)}
HELPERS = {
    str(E / 'p228-host-policy-cpu-v1/run.py'): '35abf1ed7a8fab8c4b540dfb3c0d48a3fa08480809fab4827f35dd5b7fa37494',
    str(E / 'p228-resident-state-fence-cpu-v2/run.py'): 'c16fce5d5e431f893bfccc5f1124ba20cbfca2544e26501e221c3ba452fdbedc',
    str(E / 'p228-state-bank-batch-cpu-v1/run.py'): 'd5c2237645915cd965aeb46fadf70130486abfc894fffafc9fc3513f277925da',
    str(E / 'run_clean_worker_p228_v1.py'): '2f3ef5c80e4483ac1c1a1ebb2bbacf18af7e7b263d13d965fa991525d85e6d2a',
    str(R / 'evidence/wave-output-lowering-v216/bounded.py'): 'e634e1b3be3b122b2231ad134c770f25d7d71d10ec767d61a16d03e726bdf4f1',
}
LEDGER = {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def content(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def body(path, expected=None, sha=None):
    require(path.resolve(strict=True) == path and path.is_file() and not path.is_symlink(), 'canonical retained file')
    require(path.stat().st_size <= 64 << 20, 'bounded retained file')
    raw = path.read_bytes()
    actual = content(raw)
    require(len(raw) <= 64 << 20, 'bounded actual bytes')
    if expected is not None:
        require(actual == {key: expected[key] for key in ('bytes', 'sha256')}, 'retained hash/extent: ' + str(path))
    if sha is not None:
        require(actual['sha256'] == sha, 'retained digest: ' + str(path))
    require(path not in LEDGER or LEDGER[path] == actual, 'conflicting reads')
    LEDGER[path] = actual
    return raw


def original_pin(path, remote):
    raw = body(path)
    return dict(path=str(remote), **content(raw))


def local_for(remote):
    path = Path(remote)
    require(path.is_absolute() and '..' not in path.parts and str(path) == remote, 'canonical original path')
    if path == R / 'evidence/wave-output-lowering-v216/bounded.py':
        return L / 'bounded.py'
    require(path.is_relative_to(E), 'retained original root')
    relative = path.relative_to(E)
    require(relative.parts, 'retained relative path')
    if path == E / C.name / 'target/debug' / BIN:
        return WORKER
    return (PL if relative.parts[0].startswith('p228-') else L) / relative


def read_pin(pin):
    require(isinstance(pin, dict) and set(pin) == {'path', 'bytes', 'sha256'}
            and type(pin['bytes']) is int and pin['bytes'] >= 0
            and isinstance(pin['sha256'], str) and re.fullmatch('[0-9a-f]{64}', pin['sha256']), 'FilePin shape')
    return body(local_for(pin['path']), pin)


def literal(source, name):
    tree = ast.parse(source)
    rows = [node.value for node in tree.body if isinstance(node, ast.Assign)
            and any(isinstance(target, ast.Name) and target.id == name for target in node.targets)]
    require(len(rows) == 1, 'one source literal: ' + name)
    return ast.literal_eval(rows[0])


def inventory(raw):
    names = re.findall(r'^([A-Za-z0-9_:]+): test$', raw, re.MULTILINE)
    require(names and len(names) == len(set(names)) and ': benchmark' not in raw, 'compiled test inventory')
    return set(names)


def outcomes(raw, names, summaries):
    rows = re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|ignored(?:, [^\n]*)?)$', raw, re.MULTILINE)
    require(len(rows) == len({name for name, _ in rows}) and {name for name, _ in rows} == names,
            'actual named test outcomes')
    found = [list(map(int, row)) for row in re.findall(
        r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw)]
    require(found == summaries, 'actual suite summaries')
    passed = sum(status == 'ok' for _, status in rows)
    ignored = sum(status.startswith('ignored') for _, status in rows)
    require(passed == sum(row[0] for row in found) and ignored == sum(row[2] for row in found), 'named totals')
    return dict(passed=passed, ignored=ignored, names=sorted(names), summaries=found)


def archive_map(path, prefix):
    result, seen, total, count = {}, set(), 0, 0
    with tarfile.open(path, 'r:gz') as archive:
        for member in archive:
            relative = Path(member.name)
            require(relative.parts and relative.parts[0] == prefix and not relative.is_absolute()
                    and '..' not in relative.parts and relative.as_posix() == member.name.rstrip('/'), 'archive path')
            require(relative.as_posix() not in seen and len(seen) < 12000, 'archive member census')
            seen.add(relative.as_posix())
            require(member.isdir() or member.isfile(), 'archive refuses links or special files')
            if member.isdir():
                continue
            require(0 <= member.size <= 16 << 20, 'archive member extent')
            total += member.size
            require(total <= 256 << 20, 'archive aggregate bound')
            with archive.extractfile(member) as stream:
                raw = stream.read((16 << 20) + 1)
            require(len(raw) == member.size, 'archive exact member body')
            result[relative.as_posix()] = content(raw)
            count += 1
    return result, dict(files=count, bytes=total)


def environment(out):
    toolchain = R / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu'
    target = out / 'target'
    return dict(PATH=str(toolchain / 'bin') + ':/usr/bin:/bin', HOME='/home/harmenon', LANG='C.UTF-8',
        LC_ALL='C.UTF-8', TZ='UTC', CARGO_HOME=str(R / 'toolchain/cargo'), RUSTUP_HOME=str(R / 'toolchain/rustup'),
        RUSTUP_TOOLCHAIN='nightly-2026-04-03', RUSTC=str(toolchain / 'bin/rustc'), RUSTDOC=str(toolchain / 'bin/rustdoc'),
        LD_LIBRARY_PATH=str(target / 'debug/deps') + ':' + str(toolchain / 'lib'), CARGO_BUILD_JOBS='2',
        CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL='0', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0',
        CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_TEST_OPT_LEVEL='2', TMPDIR=str(out / 'tmp'),
        OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1', MKL_NUM_THREADS='1', HIP_VISIBLE_DEVICES='',
        ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')


def recipes(out, filters):
    cargo = str(R / 'toolchain/rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/bin/cargo')
    manifest = str(out / 'sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml')
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', manifest]
    runtime = [cargo, 'test', *common, '-p', 'fe2o3-kfd', '--lib']
    worker = [cargo, 'test', *common, '--lib', '--test', 'shared_wire']
    rows = [('metadata', [cargo, 'metadata', '--offline', '--locked', '--manifest-path', manifest,
                          '--format-version', '1'], 120),
            ('runtime-list', [*runtime, '--', '--list', '--format', 'terse'], 1200)]
    rows.extend((name, [*runtime, selector, '--', '--test-threads=2'], 1200) for name, selector, _ in filters)
    rows.extend((('worker-list', [*worker, '--', '--list', '--format', 'terse'], 1200),
                 ('worker-tests', [*worker, '--', '--test-threads=2'], 1200),
                 ('worker-build', [cargo, 'build', '--profile', 'test', *common, '--bin', BIN,
                                   '--message-format=json'], 1200)))
    return rows


def main():
    require(not sys.flags.optimize and 'PYTHONOPTIMIZE' not in os.environ, 'ordinary unoptimized Python')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('cpu_sha')
    parser.add_argument('pure_sha')
    args = parser.parse_args()
    require(all(re.fullmatch('[0-9a-f]{64}', value) for value in (args.cpu_sha, args.pure_sha)),
            'actual completed CPU and pure receipt digests required')
    require(not os.path.lexists(Q), 'fresh qualification directory')
    records = {'publish.py': body(Path(__file__).resolve())}
    raw = body(C / 'complete.json', sha=args.cpu_sha)
    value = json.loads(raw)
    records['cpu/complete.json'] = raw
    require(value['schema'] == 'ferric-p228-prefix-raw-timestamps-cpu-result-v1'
            and value['passed'] is True and value['error'] is None and value['postcheck_errors'] == []
            and value['source_unchanged'] is True and value['empty_initial_target'] is True
            and value['external_cargo_cache_reused'] is True, 'successful fresh-source CPU receipt')
    require(value['tests_passed'] == 578 and value['tests_ignored'] == 4, 'actual ordinary census')
    require(all(value[key] is False for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
        'timestamp_calibration', 'parent_rebuilt', 'production_authority')), 'CPU-only qualification')
    require(value['source_commits'] == COMMITS and value['source_trees'] == TREES, 'actual clean source generations')
    manifest_raw = body(P / 'manifest.json', sha=PACKAGE_SHA)
    records['controller/manifest.json'] = manifest_raw
    require(value['package_manifest'] == dict(path=str(E / P.name / 'manifest.json'), **content(manifest_raw)),
            'executed frozen package')
    manifest = json.loads(manifest_raw)
    require(manifest['schema'] == 'ferric-p228-prefix-raw-timestamps-cpu-package-v1'
            and manifest['test_census'] == {'test_run.py': 16} and manifest['pure_tests'] == 16
            and len(manifest['files']) == 4 and {row['path'] for row in manifest['files']}
            == {'run.py', 'test_run.py', 'overlay.json', 'README.md'}, 'closed frozen package')
    for row in manifest['files']:
        records['controller/' + row['path']] = body(P / row['path'], row)
    overlay = json.loads(records['controller/overlay.json'])
    require(value['overlay'] == dict(path=str(E / P.name / 'overlay.json'),
            **content(records['controller/overlay.json'])), 'actual overlay input')
    require(overlay['schema'] == 'ferric-p228-prefix-raw-timestamps-cpu-overlay-v1'
            and len(overlay['files']) == 3 and set(overlay['added_runtime_tests']) == NEW_TESTS, 'typed timestamp overlay')
    source_raw = read_pin(overlay['source_manifest'])
    require(content(source_raw)['sha256'] == SOURCE_SHA and value['source_manifest'] == overlay['source_manifest'],
            'clean source manifest identity')
    records['implementation/source-inputs.json'] = source_raw
    sources = json.loads(source_raw)
    require(sources['schema'] == 'ferric-p228-clean-worker-sources-v1'
            and set(sources['archives']) == {'ferric', 'fe2o3'}, 'paired source archives')
    expected, extraction = {}, {}
    for project, row in sources['archives'].items():
        require(row['commit'] == COMMITS[project] and row['tree'] == TREES[project], 'archive generation')
        read_pin({key: row[key] for key in ('path', 'bytes', 'sha256')})
        files, extraction[project] = archive_map(local_for(row['path']), project)
        require(not set(expected).intersection(files), 'disjoint paired archives')
        expected.update(files)
    require(value['extraction'] == extraction, 'actual extraction census')
    base_map = dict(expected)
    preimages_raw = read_pin(overlay['runtime_preimages'])
    records['implementation/preimages.json'] = preimages_raw
    preimages = json.loads(preimages_raw)
    require(preimages['schema'] == 'ferric-p228-prefix-raw-timestamps-source-preimages-v1'
            and preimages['base_commit'] == COMMITS['fe2o3'], 'runtime proposal baseline')
    require(preimages['files'] == {row['path']: row['before']['sha256'] for row in overlay['files']
                                  if row['before'] is not None}, 'exact old body preimages')
    require(preimages['new_files'] == ['crates/fe2o3-kfd/src/' + SOURCE_FILES[-1]], 'exact new test module')
    require({row['path'] for row in overlay['files']} == {'crates/fe2o3-kfd/src/' + name for name in SOURCE_FILES},
            'exact three source destinations')
    for row in overlay['files']:
        require(row['source'] == 'p228-prefix-raw-timestamps-runtime-v2/source/' + Path(row['path']).name,
                'formatted source generation')
        key = 'fe2o3/' + row['path']
        require(expected.get(key) == row['before'], 'clean archive matches overlay preimage')
        replacement = body(PL / row['source'], row['after'])
        require(body(RUNTIME / row['path'], row['after']) == replacement, 'integrated runtime source identity')
        records['implementation/' + key] = replacement
        expected[key] = row['after']
    require(len(expected) == len(base_map) + 1, 'one new source file')

    helper_sources = {}
    for remote, sha in HELPERS.items():
        helper_sources[remote] = body(local_for(remote), sha=sha)
    base_filters = literal(helper_sources[str(E / 'p228-resident-state-fence-cpu-v2/run.py')], 'FILTERS')
    bank_source = helper_sources[str(E / 'p228-state-bank-batch-cpu-v1/run.py')]
    memory_filters = literal(bank_source, 'MEMORY_FILTERS')
    bank_selector = literal(bank_source, 'BANK_SELECTOR')
    old_filters = (*base_filters, ('state-bank', bank_selector, 10), *memory_filters)
    filters = tuple((name, selector, count + (2 if selector == PREFIX + 'tests::' else 0))
                    for name, selector, count in base_filters) + (
        ('state-bank', bank_selector, 10), *memory_filters,
        ('prefix-timestamps', PREFIX + 'timestamp_tests::', 7),
        ('raw-timestamps', 'engineering_gfx950::raw_timestamps::tests::', 8),
        ('memory-raw-timestamps', 'memory_linux::raw_timestamps_tests::', 6),
        ('queue-raw-timestamps', 'queue::submit::tests::raw_timestamp_control_', 2))
    commands = recipes(E / C.name, filters)
    require(len(commands) == 25 and set(value['phases']) == {name for name, _, _ in commands}, 'closed phase roster')
    raw_names = {'sources-base.json', 'sources-before.json', 'sources-after.json'} | {
        name + suffix for name, _, _ in commands for suffix in (
            '-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}
    require(len(raw_names) == 128 and set(value['raw']) == raw_names, 'exact 128 raw files')
    for name, pin in value['raw'].items():
        require(pin['path'] == str(E / C.name / name), 'raw original identity')
        records['cpu/' + name] = read_pin(pin)
    require(json.loads(records['cpu/sources-base.json']) == base_map, 'clean archives reconstruct source baseline')
    require(json.loads(records['cpu/sources-before.json']) == expected
            and json.loads(records['cpu/sources-after.json']) == expected, 'compiled source inventories match reconstructed map')
    tools = literal(helper_sources[str(R / 'evidence/wave-output-lowering-v216/bounded.py')], 'PINS')
    for name, argv, deadline in commands:
        command = json.loads(records['cpu/' + name + '-command.json'])
        require(command == dict(argv=argv, env=environment(E / C.name), tools=tools,
            deadline_seconds=deadline, cache_cap_bytes=6 << 30, affinity=[8, 9], nice=10,
            gpu_execution=False, expected_exit=0), 'exact command/environment/resources')
        started = json.loads(records['cpu/' + name + '-started.json'])
        require(set(started) == {'pid', 'pgid'} and type(started['pid']) is int
                and started['pid'] > 0 and started['pid'] == started['pgid'], 'owned process group')
        result = json.loads(records['cpu/' + name + '-result.json'])
        require(result == value['phases'][name] and result['exit_code'] == 0
                and result['reason'] is None and result['group_absent'] is True
                and result['cache_bytes'] <= 6 << 30, 'natural successful reaped command')
        require(result['stdout_sha256'] == value['raw'][name + '-stdout']['sha256']
                and result['stderr_sha256'] == value['raw'][name + '-stderr']['sha256'], 'owned raw streams')

    prior_raw = body(PRIOR / 'complete.json', sha=PRIOR_SHA)
    prior = json.loads(prior_raw)
    require(value['prior_cpu_complete'] == dict(path=str(E / PRIOR.name / 'complete.json'), **content(prior_raw)),
            'qualified regression baseline')
    require(prior['passed'] is True and prior['tests_passed'] == 553 and prior['tests_ignored'] == 4,
            'actual historical totals')
    prior_runtime = inventory(read_pin(prior['raw']['runtime-list-stdout']).decode())
    prior_worker = inventory(read_pin(prior['raw']['worker-list-stdout']).decode())
    prior_worker_raw = read_pin(prior['raw']['worker-tests-stdout']).decode()
    require(outcomes(prior_worker_raw, prior_worker, [[396, 0, 4], [13, 0, 0]]) == prior['tests']['worker'],
            'historical worker outcomes')
    runtime = inventory(records['cpu/runtime-list-stdout'].decode())
    require(not prior_runtime.intersection(NEW_TESTS) and runtime == prior_runtime | NEW_TESTS,
            'exact compiled runtime extension')
    selected, actual_tests = set(), {}
    for name, selector, count in filters:
        names = {test for test in runtime if selector in test}
        require(len(names) == count and not selected.intersection(names), 'nonoverlapping actual runtime selection')
        selected.update(names)
        actual_tests[name] = outcomes(records['cpu/' + name + '-stdout'].decode(), names, [[count, 0, 0]])
    require(len(selected) == 169 and NEW_TESTS <= selected, 'every new typed test actually passes')
    old_selected = {test for _, selector, _ in old_filters for test in prior_runtime if selector in test}
    require(len(old_selected) == 144 and old_selected <= selected, 'every old runtime selection preserved')
    worker_names = inventory(records['cpu/worker-list-stdout'].decode())
    require(worker_names == prior_worker and len(worker_names) == 413, 'full unchanged worker inventory')
    worker_stdout = records['cpu/worker-tests-stdout'].decode()
    actual_tests['worker'] = outcomes(worker_stdout, worker_names, [[396, 0, 4], [13, 0, 0]])
    ignore = lambda raw: set(re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\. ignored(?:, [^\n]*)?$', raw, re.MULTILINE))
    require(ignore(worker_stdout) == ignore(prior_worker_raw), 'same four named worker ignores')
    require(actual_tests == value['tests'] and sum(row['passed'] for row in actual_tests.values()) == 578,
            'receipt matches every actual test outcome')

    metadata = json.loads(records['cpu/metadata-stdout'])
    require(metadata['target_directory'] == str(E / C.name / 'target')
            and len(metadata['packages']) == 39
            and len({row['id'] for row in metadata['packages']}) == 39, 'actual metadata package/target census')
    runtime_crates = {'fe2o3-amd-target', 'fe2o3-amdhsa-loader', 'fe2o3-aql', 'fe2o3-drm-uapi',
        'fe2o3-hsaco', 'fe2o3-kfd', 'fe2o3-kfd-uapi', 'fe2o3-runtime-model', 'fe2o3-target-spec'}
    local = {row['name']: row['manifest_path'] for row in metadata['packages'] if row['source'] is None}
    wanted_local = {name: str(E / C.name / 'sources/fe2o3/crates' / name / 'Cargo.toml') for name in runtime_crates}
    wanted_local[BIN] = str(E / C.name / 'sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml')
    require(local == wanted_local and sum(row['source'] is None for row in metadata['packages']) == 10,
            'every local Cargo dependency comes from the clean pair')
    external = [row for row in metadata['packages'] if row['source'] is not None]
    reported = value['metadata']
    require(set(reported) == {'package_count', 'local', 'external'} and reported['package_count'] == 39
            and reported['local'] == local and len(reported['external']) == len(external), 'CPU metadata summary')
    for package, row in zip(external, reported['external']):
        require(set(row) == {'name', 'version', 'source', 'manifest', 'manifest_sha256'}
                and {key: row[key] for key in ('name', 'version', 'source')}
                == {key: package[key] for key in ('name', 'version', 'source')}
                and row['manifest'] == package['manifest_path']
                and Path(row['manifest']).is_relative_to(R / 'toolchain/cargo')
                and package['source'].startswith('registry+')
                and re.fullmatch('[0-9a-f]{64}', row['manifest_sha256']), 'recorded external dependency postcheck')
    runtime_package = next(row for row in metadata['packages'] if row['name'] == 'fe2o3-kfd')
    features = next(row['features'] for row in metadata['resolve']['nodes'] if row['id'] == runtime_package['id'])
    require('engineering-gfx950' in features and 'live-validation' not in features, 'CPU-only runtime features')

    artifact_rows = [json.loads(line) for line in records['cpu/worker-build-stdout'].decode().splitlines()
                     if line.startswith('{')]
    require([row['success'] for row in artifact_rows if row.get('reason') == 'build-finished'] == [True],
            'actual Cargo build finish')
    executable_rows = [row for row in artifact_rows if row.get('reason') == 'compiler-artifact' and row.get('executable')]
    require(len(executable_rows) == 1 and set(value['binaries']) == {BIN}, 'one selected worker artifact')
    artifact = executable_rows[0]
    worker = value['binaries'][BIN]['binary']
    require(artifact == value['binaries'][BIN]['artifact'] and artifact['target']['name'] == BIN
            and artifact['target']['kind'] == ['bin'] and artifact['profile']['test'] is False
            and artifact['profile']['opt_level'] == '2' and artifact['manifest_path'] == str(
                E / C.name / 'sources/ferric/adapters/tp-peer-finite-engineering-worker-v1/Cargo.toml')
            and artifact['executable'] == worker['path'] == str(E / C.name / 'target/debug' / BIN),
            'exact built worker selection')
    require(read_pin(worker)[:4] == b'\x7fELF', 'retained worker ELF')

    expected_inputs = [dict(path=str(E / P.name / 'manifest.json'), **content(manifest_raw)),
        *(dict(path=str(E / P.name / row['path']), **content(records['controller/' + row['path']]))
          for row in manifest['files']),
        *(dict(path=remote, **content(raw)) for remote, raw in helper_sources.items()),
        value['prior_cpu_complete'], *prior['raw'].values(), overlay['source_manifest'], overlay['runtime_preimages'],
        *(dict(path=str(E / row['source']), **row['after']) for row in overlay['files']),
        *({key: row[key] for key in ('path', 'bytes', 'sha256')} for row in sources['archives'].values())]
    require(len(expected_inputs) == 127 and value['inputs'] == expected_inputs, 'complete exact consumed input roster')
    for pin in expected_inputs:
        read_pin(pin)

    pure_raw = body(PURE / 'complete.json', sha=args.pure_sha)
    pure = json.loads(pure_raw)
    require(pure['schema'] == 'ferric-p228-prefix-raw-timestamps-pure-v1' and pure['passed'] is True
            and pure['tests'] == 16 and pure['errors'] == pure['failures'] == pure['skipped'] == 0
            and pure['manifest_sha256'] == PACKAGE_SHA and pure['controller_sha256'] == PURE_RUNNER_SHA
            and pure['source_postchecks_passed'] is True and pure['synthetic_policy_tests_only'] is True,
            'actual pure qualification')
    require(all(pure[key] is False for key in ('native_execution', 'gpu_execution', 'numerical_acceptance',
        'full_model_acceptance', 'production_authority', 'performance_claim')), 'pure-only scope')
    records['pure/complete.json'] = pure_raw
    for key in ('sources_before', 'sources_after', 'transcript'):
        pin = pure[key]
        require(Path(pin['path']).parent == E / PURE.name, 'pure raw original directory')
        records['pure/' + Path(pin['path']).name] = read_pin(pin)
    pure_sources = {row['path']: dict(path=str(E / P.name / row['path']),
                     **content(records['controller/' + row['path']])) for row in manifest['files']}
    require(json.loads(records['pure/sources-before.json']) == pure_sources
            and json.loads(records['pure/sources-after.json']) == pure_sources
            and pure['source_sha256'] == pure['sources_before']['sha256'], 'tested source postchecks')
    names = set()
    for cls in ast.parse(records['controller/test_run.py']).body:
        if isinstance(cls, ast.ClassDef):
            names.update((method.name, 'test_run.' + cls.name) for method in cls.body
                         if isinstance(method, ast.FunctionDef) and method.name.startswith('test_'))
    transcript = records['pure/tests.log'].decode()
    actual = re.findall(r'^(test_[A-Za-z0-9_]+) \((test_run\.[A-Za-z0-9_]+)\) \.\.\. ok$', transcript, re.MULTILINE)
    require(len(names) == len(actual) == 16 and set(actual) == names
            and re.search(r'^Ran 16 tests in [0-9.]+s$', transcript, re.MULTILINE)
            and transcript.rstrip().endswith('OK'), 'all actual named pure tests')
    runner = L / 'run_prefix_raw_timestamps_pure_p228_v1.py'
    records['run_pure.py'] = body(runner, sha=PURE_RUNNER_SHA)

    # Source reconstruction is in memory only; old trees and archives are not copied.
    for path, pin in tuple(LEDGER.items()):
        body(path, pin)
    result = dict(schema='ferric-p228-prefix-raw-timestamps-public-cpu-v1', passed=True,
        cpu_receipt=dict(path=str(E / C.name / 'complete.json'), **content(records['cpu/complete.json'])),
        pure_receipt=dict(path=str(E / PURE.name / 'complete.json'), **content(pure_raw)),
        package_manifest=value['package_manifest'], source_manifest=value['source_manifest'],
        source_commits=COMMITS, source_trees=TREES, source_files=len(expected), source_base_files=len(base_map),
        source_map_reconstructed_from_archives=True, fresh_compiled_source_copy_required=False,
        integrated_runtime_files=3, overlay=value['overlay'], prior_cpu_complete=value['prior_cpu_complete'],
        tests_passed=578, tests_ignored=4, runtime_tests=169, worker_tests=409,
        new_runtime_tests=9, newly_selected_existing_raw_timestamp_tests=16, typed_mlp_timestamp_tests=7,
        controller_tests=16, bounded_commands=25, raw_files_rehashed=128, input_pins_rehashed=len(expected_inputs),
        metadata_packages=39, local_runtime_crates=9, external_dependency_postchecks_from_cpu_receipt=True,
        worker=worker, worker_copied_into_publication=False, archives_copied_into_publication=False,
        parent_rebuilt=False, gpu_execution=False, numerical_acceptance=False,
        timestamp_calibration=False, performance_claim=False, sustained_2048_256=False, production_authority=False)
    records['result.json'] = (json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(sum(len(raw) for raw in records.values()) <= 256 << 20, 'bounded publication bytes')
    Q.mkdir(mode=0o755)
    for name, raw in records.items():
        destination = Q / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as stream:
            stream.write(raw)
    print(json.dumps(dict(published_files=len(records), result=dict(path=str(Q / 'result.json'),
        **content(records['result.json'])), **result), indent=2, sort_keys=True), flush=True)


if __name__ == '__main__':
    main()
