"""Retain a clean terminal worker CPU run without copying host executables."""

import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys
import tarfile
import time


E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-stage-capture-worker-cpu-v228-v1'
WORKER = ROOT / 'ferric/adapters/tp-peer-finite-engineering-worker-v1'
PREFIX = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
TARGET = ROOT / 'target'
INTERFACE = E / 'guarded-mlp-model-interface-cpu-v228-v1'
ARCHIVE = E / 'guarded-mlp-stage-capture-worker-cpu-evidence-v228-v1.tar.gz'
STAGER = E / 'guarded-stage-capture-stage-worker-v1.py'
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
PACKAGE = 'ferric-tp-peer-finite-engineering-worker-v1'
INPUT_PIN = dict(bytes=205000, sha256='91564acc34bcf0008a93cba01102eac7be8fe256a4c217a2e5c1e6fe5f66afdb')
CONTROLLER_PIN = dict(bytes=36610, sha256='4b290e97c620cd69c924778a43c3172e3198d6a64cb0d61c1c157b7aa54c6726')
SUPERVISOR_PIN = dict(bytes=41485, sha256='8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc')
STAGER_PIN = dict(bytes=13084, sha256='686e3d82ae485e573cf9631f7ab63da34e25f80979843f6c5db017575cbafe5d')
CACHE_MANIFEST_PIN = dict(bytes=22831, sha256='6c8b4eb4a9cf407dc0909dde3c574145ede603bdbe9abe8d4ff5bbc56a196ba1')
CACHE_ARCHIVE_PIN = dict(bytes=8411032, sha256='fc997f95853f1ea94d383ee5755bcc3dbc0c78a5c933ed21cafc3c8385bc3155')
CACHE_STAGE_PIN = dict(bytes=9560, sha256='69fe8d715887c3797e8ba7aa5933b5775f6aeefa5ae986c5324509d8c67183a5')
CACHE_PACK_PIN = dict(bytes=7100, sha256='6ff15399b0f6200c8fe7bece46c8f5e8914488e9c78702e9fcc952b8e13a61cd')
CACHE_LOCK_PIN = dict(bytes=7470, sha256='df2a4e0b9cf96687328a5b1e41937eb940e62144314d7f903c8c961da947f7aa')
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'metadata', 'worker-tests-build',
          'worker-list', 'worker-ignored', 'worker-tests', 'worker-build')
RUNTIME_SELECTED = tuple('fe2o3/' + name for name in
    ('Cargo.toml', 'Cargo.toml.original', 'Cargo.lock', 'Cargo.lock.input', 'rust-toolchain.toml'))
PRLIMIT = ['/usr/bin/prlimit', '--as=12884901888', '--cpu=1200', '--fsize=1073741824', '--core=0', '--']
DEADLINE = time.monotonic() + 180
MAX_MEMBERS, MAX_BODY = 320, 64 << 20


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def pin_bytes(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
            value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def file_pin(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and 0 <= before.st_size <= 1 << 30, 'bounded ordinary file: ' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed before pin')
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'file changed during pin')
    require(stamp(path.lstat()) == stamp(before), 'file changed after pin')
    return dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())


def read(path):
    before = file_pin(path)
    require(before['bytes'] <= MAX_BODY, 'retained body extent')
    body = path.read_bytes()
    require(pin_bytes(body) == compact(before) and file_pin(path) == before, 'retained body changed')
    return body


def ordinary_name(name):
    return (type(name) is str and Path(name).as_posix() == name and not Path(name).is_absolute()
            and '..' not in Path(name).parts and name not in ('', '.'))


def paths_below(root):
    require(root.resolve(strict=True) == root and root.is_dir(), 'ordinary source directory')
    result = []
    for directory, directories, names in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(directory) / name).is_symlink() for name in directories), 'source directory alias')
        result.extend(Path(directory) / name for name in names)
    require(len(result) <= 20000, 'bounded source/dependency file census')
    return sorted(result)


def parse_tests(body):
    summaries, named, active, progress = [], [], [], set()
    for line in body.decode('utf-8').splitlines():
        notice = re.fullmatch(r'test ([A-Za-z0-9_:]+) has been running for over 60 seconds', line)
        if notice:
            name = notice.group(1)
            require(name not in progress and name not in {row['name'] for row in named}, 'invalid test progress')
            progress.add(name)
            continue
        result = re.fullmatch(r'test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?', line)
        if result:
            name, status = result.groups()
            require(name not in {row['name'] for row in named}
                    and (status != 'ignored' or name not in progress), 'duplicate/ignored test progress')
            row = dict(name=name, outcome=status)
            named.append(row)
            active.append(row)
            continue
        summary = re.fullmatch(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; '
            r'(\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s', line)
        if summary:
            status, passed, failed, ignored, measured, filtered = summary.groups()
            row = dict(status=status, passed=int(passed), failed=int(failed), ignored=int(ignored),
                       measured=int(measured), filtered_out=int(filtered))
            require(all(sum(item['outcome'] == outcome for item in active) == row[key]
                for outcome, key in (('ok', 'passed'), ('FAILED', 'failed'), ('ignored', 'ignored'))),
                'test target summary differs from named outcomes')
            summaries.append(row)
            active = []
        else:
            require(not line.startswith('test '), 'malformed named libtest output')
    require(not active and progress <= {row['name'] for row in named}, 'incomplete named test output')
    return dict(summaries=summaries, named=named,
        passed=sum(row['passed'] for row in summaries), failed=sum(row['failed'] for row in summaries),
        ignored=sum(row['ignored'] for row in summaries))


def expected_environment(inputs):
    toolchain = Path(inputs['tool_pins']['cargo']['path']).parent
    library = toolchain.parent / 'lib'
    return dict(HOME='/home/harmenon', PATH=str(toolchain) + ':/usr/bin:/bin',
        CARGO_HOME=str(ROOT / 'cargo-home'), CARGO_TARGET_DIR=str(TARGET),
        LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(library),
        TMPDIR=str(ROOT / 'tmp'), RUSTC=str(toolchain / 'rustc'), RUSTDOC=str(toolchain / 'rustdoc'),
        CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
        CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never', MALLOC_ARENA_MAX='2', RUST_BACKTRACE='1',
        CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0',
        CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
        CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
        CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
        ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')


def recipes(inputs):
    toolchain = Path(inputs['tool_pins']['cargo']['path']).parent
    cargo = str(toolchain / 'cargo')
    fmt = [str(toolchain / 'rustfmt'), '--edition', '2024', '--config', 'skip_children=true']
    overlay = [str(ROOT / name) for name in inputs['worker_overlay']]
    common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(WORKER / 'Cargo.toml')]
    selected = [*common, '--lib', '--bin', PACKAGE, '--test', 'shared_wire']
    return {
        'rustfmt': ([*fmt, *overlay], 120),
        'rustfmt-check': ([*fmt, '--check', *overlay], 120),
        'rustc-version': ([str(toolchain / 'rustc'), '--version', '--verbose'], 60),
        'metadata': ([cargo, 'metadata', '--offline', '--locked', '--format-version', '1',
                      '--manifest-path', str(WORKER / 'Cargo.toml')], 120),
        'worker-tests-build': ([cargo, 'test', *selected, '--no-run', '--message-format=json'], 1800),
        'worker-list': ([cargo, 'test', *selected, '--', '--list', '--format=terse'], 120),
        'worker-ignored': ([cargo, 'test', *selected, '--', '--ignored', '--list', '--format=terse'], 120),
        'worker-tests': ([cargo, 'test', *selected, '--', '--test-threads=1'], 1800),
        'worker-build': ([cargo, 'build', *common, '--bin', PACKAGE, '--message-format=json'], 1800),
    }


def verify_cache(retain, result):
    home = ROOT / 'cargo-home'
    manifest_path = ROOT / 'cargo-cache-manifest.json'
    stage_path = ROOT / 'cargo-cache-stage-complete.json'
    manifest = json.loads(retain('cargo-cache-manifest.json', manifest_path,
        dict(path=str(manifest_path), **CACHE_MANIFEST_PIN)))
    stage_body = retain('cargo-cache-stage-complete.json', stage_path)
    stage_pin = dict(path=str(stage_path), **pin_bytes(stage_body))
    stage = json.loads(stage_body)
    for name, remote, expected in (
        ('stage_worker_cache.py', E / 'guarded-stage-capture-stage-cache-v1.py', CACHE_STAGE_PIN),
        ('pack_worker_cache.py', E / 'pack_guarded_worker_cache_v228_v3.py', CACHE_PACK_PIN),
    ):
        retain(name, remote, dict(path=str(remote), **expected))
    require(manifest['schema'] == 'ferric-guarded-mlp-worker-cache-v1'
            and manifest['lock'] == CACHE_LOCK_PIN and len(manifest['packages']) == 29
            and len(manifest['files']) == 59 and manifest['cargo_execution'] is False
            and stage['schema'] == 'ferric-guarded-mlp-worker-cache-stage-v1' and stage['passed'] is True
            and stage['manifest'] == CACHE_MANIFEST_PIN and stage['cargo_home'] == str(home)
            and stage['lock'] == CACHE_LOCK_PIN and stage['files'] == manifest['files']
            and stage['packages'] == 29 and stage['cache_files'] == 59
            and stage['inner_members'] == manifest['inner_members'] == 2266
            and stage['inner_expanded_bytes'] == manifest['inner_expanded_bytes'] == 50677903
            and all(stage[key] is False for key in ('shared_cache_changed', 'lock_changed',
                         'project_code_executed', 'crate_sources_extracted')),
            'independent private cache staging contract')
    archive_path = E / 'guarded-mlp-worker-cache-v228-v3.tar.gz'
    require(stage['archive'] == dict(path=str(archive_path), **CACHE_ARCHIVE_PIN)
            and file_pin(archive_path) == stage['archive'], 'original cache archive identity without body retention')
    lock = file_pin(WORKER / 'Cargo.lock')
    require(compact(lock) == CACHE_LOCK_PIN, 'worker dependency lock remains unchanged')
    files = {}
    for name, expected in manifest['files'].items():
        require(ordinary_name(name) and name.startswith('registry/'), 'closed private cache input name')
        row = file_pin(home / name)
        require(compact(row) == expected, 'immutable private cache input changed')
        files[name] = row
    config = 'registry/index/index.crates.io-1949cf8c6b5b557f/config.json'
    allowed = {config}
    for package in manifest['packages']:
        require(package['archive'] not in allowed and package['index'] not in allowed
                and files[package['archive']]['sha256'] == package['checksum'],
                'locked checksum and distinct cache package rows')
        allowed.update((package['archive'], package['index']))
    require(allowed == set(files), 'exact 29 archives, 29 index rows, one config')
    provenance = dict(manifest=dict(path=str(manifest_path), **CACHE_MANIFEST_PIN), stage=stage_pin,
        lock=lock, files=files, packages=manifest['packages'], inner_members=manifest['inner_members'],
        inner_expanded_bytes=manifest['inner_expanded_bytes'], cargo_home=str(home))
    if result['cache_provenance'] is not None:
        require(result['cache_provenance'] == provenance and result['cache_inputs_unchanged'] is True,
                'actual CPU cache proof and final input postcheck join')
    else:
        require(result['passed'] is False and result['cache_inputs_unchanged'] is False,
                'a run without cache admission cannot be qualified')
    return provenance, stage['archive']


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]), 'python3 -B export_worker_cpu.py TERMINAL_SHA')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'export host/UID')
    require(ROOT.resolve(strict=True) == ROOT and not os.path.lexists(ARCHIVE), 'fresh terminal archive')
    os.umask(0o077)
    bodies, observed = {}, {}
    def retain(name, path, expected=None):
        require(ordinary_name(name) and name not in bodies, 'unique ordinary retained name')
        body = read(path)
        row = dict(path=str(path), **pin_bytes(body))
        require(expected is None or row == expected, 'retained source/raw pin mismatch: ' + name)
        bodies[name] = body
        observed[path] = row
        require(len(bodies) < MAX_MEMBERS and sum(map(len, bodies.values())) < MAX_BODY,
                'expanded capsule bound exceeded; preserve remote originals')
        return body
    manifest_body = retain('input-manifest.json', ROOT / 'input-manifest.json',
        dict(path=str(ROOT / 'input-manifest.json'), **INPUT_PIN))
    inputs = json.loads(manifest_body)
    require(inputs['schema'] == 'ferric-guarded-mlp-stage-capture-worker-cpu-input-v1'
            and inputs['source_generation'] == GENERATION and len(inputs['files']) == 990,
            'fixed worker input contract')
    for name, expected in (('run_cpu.py', CONTROLLER_PIN), ('supervisor.py', SUPERVISOR_PIN)):
        retain(name, ROOT / name, dict(path=str(ROOT / name), **expected))
    retain('stage_worker_cpu.py', STAGER, dict(path=str(STAGER), **STAGER_PIN))
    retain('export_worker_cpu.py', Path(__file__).resolve())
    stages = [ROOT / name for name in ('stage-complete.json', 'stage-failed.json') if os.path.lexists(ROOT / name)]
    require(len(stages) == 1, 'one exact staging terminal')
    stage = json.loads(retain(stages[0].name, stages[0]))
    require(stage['passed'] is True and stage['input_manifest'] == dict(path=str(ROOT / 'input-manifest.json'), **INPUT_PIN)
            and stage['source_files'] == 990 and stage['runtime_files'] == 807 and stage['worker_files'] == 181
            and stage['archive_members'] == 190 and stage['qualified_parent_unchanged'] is True
            and all(stage[key] is False for key in ('formatting_executed', 'compiler_executed',
                                                  'project_code_executed', 'gpu_execution')),
            'CPU run must descend from a successful data-only stage')
    terminals = [ROOT / 'evidence' / name for name in ('complete.json', 'failed.json')
                 if os.path.lexists(ROOT / 'evidence' / name)]
    require(len(terminals) == 1, 'one exact worker CPU terminal')
    terminal_body = retain('evidence/' + terminals[0].name, terminals[0])
    require(pin_bytes(terminal_body)['sha256'] == sys.argv[1], 'actual terminal SHA')
    result = json.loads(terminal_body)
    require(result['schema'] == 'ferric-guarded-mlp-stage-capture-worker-cpu-v1' and type(result['passed']) is bool
            and (terminals[0].name == 'complete.json') == result['passed']
            and (result['failure'] is None) == result['passed']
            and result['postcheck_errors'] == [] and result['source_generation'] == GENERATION
            and result['controller'] == dict(path=str(ROOT / 'run_cpu.py'), **CONTROLLER_PIN)
            and result['supervisor'] == dict(path=str(ROOT / 'supervisor.py'), **SUPERVISOR_PIN)
            and result['input_manifest'] == dict(path=str(ROOT / 'input-manifest.json'), **INPUT_PIN),
            'clean passed/failed worker terminal identity')
    require(all(result[key] is False for key in ('runtime_suite_rerun', 'gpu_execution',
        'native_guarded_worker_qualified', 'whole_model_guarded_execution', 'numerical_acceptance',
        'production_authority', 'performance_claim', 'capture_native_execution')), 'CPU-only outcome authority')
    require(result['capture_source_added'] is True, 'explicit capture source qualification')
    require(result['source_lineage'] == inputs['source_lineage'], 'terminal input lineage join')
    cache_provenance, cache_archive = verify_cache(retain, result)
    expected_lineage = {
        'interface_complete': ('lineage/interface-complete.json', INTERFACE / 'evidence/complete.json'),
        'interface_sources': ('lineage/interface-sources.json', INTERFACE / 'evidence/sources-after.json'),
        'worker_complete': ('inputs/worker-complete.json', ROOT / 'inputs/worker-complete.json'),
        'worker_tests': ('inputs/worker-tests.stdout', ROOT / 'inputs/worker-tests.stdout'),
        'worker_list': ('inputs/worker-list.stdout', ROOT / 'inputs/worker-list.stdout'),
        'worker_snapshot': ('inputs/worker-source.json', ROOT / 'inputs/worker-source.json'),
        'worker_sources': ('inputs/worker-sources.json', ROOT / 'inputs/worker-sources.json'),
        'capture_proposal': ('inputs/capture-source-manifest.json', ROOT / 'inputs/capture-source-manifest.json'),
    }
    require(set(inputs['source_lineage']) == set(expected_lineage), 'closed eight selected lineage bodies')
    for key, (name, path) in expected_lineage.items():
        retain(name, path, inputs['source_lineage'][key])
    require(set(result['readset']) <= set(expected_lineage)
            and all(row == inputs['source_lineage'][key] for key, row in result['readset'].items()),
            'actual controller readset remains a subset of selected lineage')
    if result['passed']:
        require(result['readset'] == inputs['source_lineage']
                and result['runtime_source_qualified_by_interface_cpu'] is True,
                'successful runtime lineage proof')
    raw = result['raw']
    require(type(raw) is dict and len(raw) <= 50 and all(ordinary_name(name)
                and '/' not in name and row['path'] == str(ROOT / 'evidence' / name)
                for name, row in raw.items()), 'closed bounded raw evidence names')
    actual_raw = {path.name for path in (ROOT / 'evidence').iterdir() if path.is_file()}
    require(actual_raw == set(raw) | {terminals[0].name}, 'terminal/raw evidence directory closure')
    for name, row in raw.items():
        retain('evidence/' + name, Path(row['path']), row)
    phases = result['phases']
    labels = [row['label'] for row in phases]
    require(labels == list(PHASES[:len(phases)]) and len(labels) <= 9,
            'actual phases must be a closed recipe prefix')
    recipe, env = recipes(inputs), expected_environment(inputs)
    phase_raw = set()
    for index, phase in enumerate(phases):
        label = phase['label']
        names = {suffix: label + '.' + suffix for suffix in
                 ('command.json', 'started.json', 'result.json', 'stdout', 'stderr')}
        phase_raw.update(names.values())
        require(set(names.values()) <= set(raw), 'every registered phase retains all five raw files')
        command = json.loads(bodies['evidence/' + names['command.json']])
        started = json.loads(bodies['evidence/' + names['started.json']])
        returned = json.loads(bodies['evidence/' + names['result.json']])
        argv, cap = recipe[label]
        require(phase == returned and phase['argv'] == command['argv'] == started['argv'] == PRLIMIT + argv
                and command['cwd'] == str(WORKER) and command['env'] == env
                and 0 < command['wall_timeout_seconds'] <= cap
                and phase['pid'] == phase['pgid'] == started['pid'] == started['pgid']
                and type(phase['pid']) is int and phase['pid'] > 0, 'phase recipe/owned child join')
        require(phase['natural_exit'] is True and phase['reaped'] is True
                and phase['process_group_absent'] is True and phase['forced_cleanup'] is False
                and phase['timed_out'] is False and phase['exception'] is None
                and phase['storage_failure'] is None and phase['observed_signals'] == []
                and type(phase['exit_code']) is int
                and (phase['exit_code'] == 0 or (not result['passed'] and index == len(phases) - 1)),
                'only clean natural terminal prefixes are exportable by this recipe')
        require(phase['command'] == raw[names['command.json']]
                and phase['stdout'] == raw[names['stdout']] and phase['stderr'] == raw[names['stderr']],
                'receipt raw stream joins')
    maps = {'sources-preformat.json', 'sources-before.json', 'sources-after.json',
            'dependencies-before.json', 'dependencies-after.json'}
    require(set(raw) == phase_raw | (set(raw) & maps) and 'sources-after.json' in raw
            and 'dependencies-after.json' in raw, 'closed raw phase/snapshot roster')
    final = result['final_sources']
    require(type(final) is dict and len(final) == 990 and set(final) == set(inputs['files'])
            and all(ordinary_name(name) and row['path'] == str(ROOT / name) for name, row in final.items())
            and json.loads(bodies['evidence/sources-after.json']) == final, 'actual full final source map')
    preformat, formatted = result['preformat_sources'], result['input_sources']
    if preformat is not None:
        require(json.loads(bodies['evidence/sources-preformat.json']) == preformat
                and {name: compact(row) for name, row in preformat.items()} == inputs['files']
                and set(preformat) == set(final), 'preformat source identity')
    if formatted is not None:
        require(json.loads(bodies['evidence/sources-before.json']) == formatted and final == formatted
                and result['source_unchanged'] is True and preformat is not None,
                'postformat source unchanged')
        changes = sorted(name for name in formatted if formatted[name] != preformat[name])
        require(changes == result['format_changed_paths'] and set(changes) <= set(inputs['worker_overlay']),
                'formatter only changed authorized worker sources')
    else:
        require(result['passed'] is False and result['source_unchanged'] is False,
                'uncompleted formatting cannot authorize a pass')
    require(all(compact(row) == inputs['files'][name] for name, row in final.items()
                if name not in inputs['worker_overlay']), 'immutable runtime/nonoverlay final source join')
    actual_source_paths = {str(path.relative_to(ROOT))
        for directory in (ROOT / 'ferric', ROOT / 'fe2o3') for path in paths_below(directory)}
    require(actual_source_paths | {'run_cpu.py', 'supervisor.py'} == set(final), 'full source directory closure')
    for name, row in final.items():
        require(file_pin(ROOT / name) == row, 'actual final source body mismatch: ' + name)
    worker_names = sorted(name for name in final if name.startswith(PREFIX))
    require(len(worker_names) == 181, 'full worker source retention')
    for name in [*worker_names, *RUNTIME_SELECTED]:
        retain(name, ROOT / name, final[name])
    if result['tool_pins']:
        require(result['tool_pins'] == inputs['tool_pins'], 'current qualified toolchain closure')
        for row in result['tool_pins'].values():
            require(file_pin(Path(row['path'])) == row, 'toolchain changed before export')
    dependencies_after = json.loads(bodies['evidence/dependencies-after.json'])
    if 'dependencies-before.json' in raw:
        dependencies_before = json.loads(bodies['evidence/dependencies-before.json'])
        require(dependencies_after == dependencies_before, 'external dependency snapshots differ')
        for directory, expected in dependencies_before.items():
            path = Path(directory)
            require(path.is_relative_to(ROOT / 'cargo-home'), 'external dependency path')
            require({str(p.relative_to(path)): file_pin(p) for p in paths_below(path)} == expected,
                    'external dependency changed before export')
    else:
        require(dependencies_after == {}, 'dependency postmap without admitted premap')
    artifacts = result['artifacts']
    allowed = {
        'worker-lib': ('worker-tests-build', 'ferric_tp_peer_finite_engineering_worker_v1', 'lib', 'src/lib.rs', True),
        'worker-bin-test': ('worker-tests-build', PACKAGE, 'bin', 'src/main.rs', True),
        'worker-wire-test': ('worker-tests-build', 'shared_wire', 'test', 'tests/shared_wire.rs', True),
        'worker': ('worker-build', PACKAGE, 'bin', 'src/main.rs', False),
    }
    require(set(artifacts) <= set(allowed), 'closed worker artifact roles')
    artifact_presence = {}
    for role, artifact in artifacts.items():
        label, name, kind, source, testing = allowed[role]
        require(label in labels, 'artifact must have an actual build phase')
        records = [json.loads(line) for line in bodies['evidence/' + label + '.stdout'].decode().splitlines()
                   if line.startswith('{')]
        row = artifact['cargo_artifact']
        require(sum(value == row for value in records) == 1
                and row['reason'] == 'compiler-artifact' and row['manifest_path'] == str(WORKER / 'Cargo.toml')
                and row['target']['name'] == name and row['target']['kind'] == [kind]
                and row['target']['src_path'] == str(WORKER / source) and row['features'] == []
                and row['profile']['test'] is testing and row['executable'] == artifact['pin']['path']
                and row['filenames'].count(artifact['pin']['path']) == 1,
                'actual Cargo artifact metadata join')
        path = Path(artifact['pin']['path'])
        require(path.is_relative_to(TARGET), 'artifact must remain within fresh target')
        present = os.path.lexists(path)
        if present:
            require(file_pin(path) == artifact['pin'], 'present host artifact pin changed')
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'present host artifact ELF magic')
        artifact_presence[role] = present
    require(len({row['pin']['path'] for row in artifacts.values()}) == len(artifacts), 'distinct actual products')
    capture = json.loads(bodies['inputs/capture-source-manifest.json'])
    require(pin_bytes(bodies['inputs/capture-source-manifest.json']) == dict(bytes=7739,
                sha256='2dc554b35f227180b1367f926d38e3332305f99666d9f9bd52b941240d1d899e')
            and capture['schema'] == 'ferric-guarded-mlp-model-stage-capture-source-v1'
            and capture['runtime_sources_changed'] is False and capture['default_wire_changed'] is False
            and capture['kernel_images_changed'] is False
            and inputs['new_tests'] == {'worker-lib': capture['new_tests']['worker_library'],
                                       'worker-bin-test': [], 'worker-wire-test': []},
            'exact capture proposal and five added worker tests')
    previous = json.loads(bodies['inputs/worker-complete.json'])
    previous_sources = json.loads(bodies['inputs/worker-sources.json'])
    require(pin_bytes(bodies['inputs/worker-complete.json']) == compact(capture['base']['worker_complete'])
            and pin_bytes(bodies['inputs/worker-sources.json']) == compact(capture['base']['worker_sources'])
            and previous['passed'] is True and previous['source_unchanged'] is True
            and previous['postcheck_errors'] == []
            and previous['input_sources'] == previous['final_sources'] == previous_sources
            and len(previous_sources) == 990, 'actual V6 source/outcome lineage')
    baseline = parse_tests(bodies['inputs/worker-tests.stdout'])
    require(baseline == previous['tests']['worker-tests'], 'actual V6 raw baseline tests')
    prior_names = {row['name']: row['outcome'] for row in baseline['named']}
    expected_names = dict(prior_names)
    for names in inputs['new_tests'].values():
        require(not set(names) & set(expected_names), 'declared test additions collide')
        expected_names.update({name: 'ok' for name in names})
    require(len(prior_names) == 590 and baseline['passed'] == 586 and baseline['ignored'] == 4
            and len(expected_names) == 595, 'closed historical-plus-added worker census')
    if result['baseline_tests'] is not None:
        require(result['baseline_tests'] == baseline, 'parsed historical test receipt join')
    expected_ignored = sorted(name for name, status in expected_names.items() if status == 'ignored')
    for label, wanted in (('worker-list', sorted(expected_names)), ('worker-ignored', expected_ignored)):
        if label in labels and phases[labels.index(label)]['exit_code'] == 0:
            text = bodies['evidence/' + label + '.stdout'].decode()
            names = re.findall(r'^([A-Za-z0-9_:]+): test$', text, re.M)
            require(len(names) == len(set(names)) and sorted(names) == wanted and ': benchmark' not in text,
                    'actual listed worker inventory differs')
    raw_tests = None
    if 'worker-tests' in labels:
        raw_tests = parse_tests(bodies['evidence/worker-tests.stdout'])
        actual = {row['name']: row['outcome'] for row in raw_tests['named']}
        require(set(actual) <= set(expected_names) and all(status in ('ok', 'FAILED')
                if expected_names[name] == 'ok' else status == 'ignored' for name, status in actual.items()),
                'failed or successful raw outcomes must remain within authenticated test roster')
    require(set(result['tests']) <= {'worker-tests'}
            and result['full_worker_tests_executed'] is ('worker-tests' in result['tests']),
            'controller test-admission flag')
    if 'worker-tests' in result['tests']:
        require(result['tests']['worker-tests'] == raw_tests, 'actual parsed worker test receipt')
    if result['passed']:
        require(labels == list(PHASES) and len(raw) == 50 and set(raw) == phase_raw | maps
                and set(artifacts) == set(allowed) and all(artifact_presence.values())
                and result['inventory'] == sorted(expected_names) and result['ignored'] == expected_ignored
                and raw_tests == result['tests']['worker-tests']
                and {row['name']: row['outcome'] for row in raw_tests['named']} == expected_names
                and (raw_tests['passed'], raw_tests['failed'], raw_tests['ignored']) == (591, 0, 4)
                and [[row[key] for key in ('passed', 'failed', 'ignored')] for row in raw_tests['summaries']]
                    == [[578, 0, 4], [0, 0, 0], [13, 0, 0]], 'full successful worker CPU closure')
    for path, expected in observed.items():
        require(file_pin(path) == expected, 'retained readset drift before archive write')
    for row in cache_provenance['files'].values():
        require(file_pin(Path(row['path'])) == row, 'immutable cache input drift during export')
    require(time.monotonic() < DEADLINE, 'export deadline')
    ledger = dict(schema='ferric-guarded-mlp-stage-capture-worker-cpu-retained-v1',
        passed=result['passed'], failure=result['failure'], terminal_name=terminals[0].name,
        terminal=dict(path=str(terminals[0]), **pin_bytes(terminal_body)), input=INPUT_PIN,
        files={name: pin_bytes(body) for name, body in sorted(bodies.items())},
        raw_names=sorted(raw), raw_count=len(raw), phases=len(phases),
        source_map_rows=len(final), retained_worker_source_files=181, retained_runtime_manifest_files=5,
        actual_artifact_metadata=artifacts, artifact_bodies_present_at_export=artifact_presence,
        host_executable_bodies_retained=False, actual_raw_worker_tests=raw_tests,
        runtime_ancestry=inputs['source_lineage']['interface_complete'],
        full_runtime_bodies_retained=False, full_source_pin_maps_retained=True,
        cache_provenance=cache_provenance, cache_archive=cache_archive,
        cache_package_bodies_retained=False, cache_cpu_admitted=result['cache_provenance'] is not None,
        worker_cpu_qualified=result['passed'], capture_source_added=True, capture_native_execution=False,
        gpu_execution=False, numerical_acceptance=False,
        whole_model_guarded_execution=False, production_authority=False, performance_claim=False)
    bodies['manifest.json'] = (json.dumps(ledger, sort_keys=True, indent=2) + '\n').encode('utf-8')
    total = sum(map(len, bodies.values()))
    require(len(bodies) <= MAX_MEMBERS and total <= MAX_BODY,
            'final expanded capsule bound exceeded; preserve remote originals')
    if result['passed']:
        require(len(bodies) == 256, 'exact successful selected capsule member census')
    with ARCHIVE.open('xb') as output:
        with tarfile.open(fileobj=output, mode='w:gz', format=tarfile.PAX_FORMAT) as tar:
            for name, body in sorted(bodies.items()):
                info = tarfile.TarInfo(name)
                info.size, info.mode, info.mtime = len(body), 0o600, 0
                info.uid = info.gid = 0
                info.uname = info.gname = ''
                tar.addfile(info, io.BytesIO(body))
    require(time.monotonic() < DEADLINE, 'export deadline after archive write')
    print(json.dumps(dict(archive=file_pin(ARCHIVE), members=len(bodies), pins=len(ledger['files']),
        raw=len(raw), expanded_bytes=total, passed=result['passed'], failure=result['failure'],
        host_executable_bodies_retained=False, gpu_execution=False), sort_keys=True))


if __name__ == '__main__':
    def interrupted(signum, _frame):
        raise RuntimeError('worker CPU evidence export interrupted: ' + str(signum))
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(signum, interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, DEADLINE - time.monotonic()))
    try:
        main()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
