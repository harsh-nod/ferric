"""Full KFD CPU qualification and unchanged-worker regression, using owned leaves."""
from collections import Counter
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
import types

ROOT = Path(__file__).resolve().parent
E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
EXPECTED_ROOT = E / 'guarded-mlp-peer-read-pair-cpu-v228-v2'
RUNTIME = ROOT / 'fe2o3'
WORKER_REL = 'ferric/adapters/tp-peer-finite-engineering-worker-v1/'
WORKER = ROOT / WORKER_REL
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
CARGO_HOME = ROOT / 'cargo-home'
SUPPORT_SHA = 'fa63ab2dac6557177a8f75d8bfa17d48feff728ca66943fbadc4ed6f4d313ebd'
CACHE_SHA = 'fefe68da1d811361c8a435649493a211376cae04c19528907292686513a1761e'
PROPOSAL_SHA = '23bad10d4f844eb500aee01f9d433e5882f2f40811920b5a85566cf2db089c37'
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 3600, 1800, 50
HISTORY = {
    'runtime-complete.json': (2187577, '032c4ac98bee0b6c551f1b98fb12d7966bd4f9fadc6b0b8570ab2488634c7814'),
    'runtime-sources.json': (401732, 'efe370a940d49aaeb173fdf9a245c6a19f5561852d2fb535d929fae93db30da7'),
    'runtime-tests.stdout': (127000, 'f402543405de9287cd3e9963b997af7466da20c2fe4d197bec8aa3f475aa8723'),
    'runtime-list.stdout': (118987, '43d705698f8fc768f266cd55f4635a61e7fe03e4b7fd7b86c62e10c1fbfa8c2b'),
    'worker-complete.json': (1631185, '24feb83a0bde60dc9c6b8db28f2ce8252379b40d8e0d40483690f58038881b2a'),
    'worker-sources.json': (406182, '4c49b12bd9d2dbcc7c504f72dd38818cec567317f2ffb3fda1f18f88f841a827'),
    'worker-tests.stdout': (73088, '97f3aabc5f4f3d477ffc9cd68f8bae628265c985dbf70da8ad7fe2a931ed738a'),
    'worker-list.stdout': (68746, 'c93f7870e8b17614f78f2fedb0b0bb7f153fe07324dacb9b6a195abb50965301'),
    'runtime-docs.stdout': (2610, 'ae232b746889e201c68b98493db857a97d110dabb9dd32cee7fdaa11a5d0b8d8'),
    'legacy-docs.stdout': (2161, '95b8958d401e6e40ab05df7310915287a68270d301e7164826349c1c068c52d5'),
}
KFD_TARGETS = {
    'kfd-lib': ('fe2o3_kfd', 'lib', 'src/lib.rs'),
    'engineering-worker-test': ('fe2o3-gfx950-engineering-worker', 'bin', 'src/bin/gfx950_engineering_worker.rs'),
    'guarded-facade-test': ('guarded_mlp_facade_v1', 'test', 'tests/guarded_mlp_facade_v1.rs'),
    'debug-trap-test': ('kfd_debug_trap_live', 'test', 'tests/kfd_debug_trap_live.rs'),
    'telemetry-env-test': ('target_debug_telemetry_env_v1', 'test', 'tests/target_debug_telemetry_env_v1.rs'),
    'telemetry-test': ('target_debug_telemetry_v1', 'test', 'tests/target_debug_telemetry_v1.rs'),
}
PAIRED = 'engineering_gfx950::peer::combined_mlp_state_v1::paired::'
FOCUS = {
    'peer-read-pair-tests': 'engineering_gfx950::peer::read_pair_v1::tests::',
    'arena-reuse-tests': PAIRED + 'arena::retired::reuse::tests::',
    'arena-memory-tests': 'memory_linux::paired_arena_reuse_v1::tests::',
    'retained-tests': PAIRED + 'retained::tests::',
    'mixed-bank-tests': PAIRED + 'retained::mixed_bank::tests::',
}
DOC_FILTER = 'engineering_gfx950_peer_combined_mlp_paired_facade_v1.rs'
PHASES = ('rustfmt', 'rustfmt-check', 'rustc-version', 'runtime-metadata', 'default-check',
    'kfd-tests-build', 'kfd-list', 'kfd-ignored', 'kfd-tests', *FOCUS,
    'interface-doc-list', 'interface-doc-tests', 'metadata', 'worker-tests-build',
    'worker-list', 'worker-ignored', 'worker-tests', 'worker-build')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def load_exact(name, filename, digest):
    path = ROOT / filename
    require(path.resolve(strict=True) == path and path.is_file(), 'canonical helper')
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    before = path.stat()
    require(before.st_size <= 1 << 20, 'helper extent')
    raw = path.read_bytes()
    require(stamp(before) == stamp(path.stat()) and hashlib.sha256(raw).hexdigest() == digest,
            'authenticated helper body')
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    return module


def normalized(text):
    for name in ('payload_release_failure_after_event_destroy_is_process_terminal',
                 'unpublished_custody_cleanup_failure_is_process_terminal'):
        full = 'queue_linux::tests::' + name
        text, count = re.subn('^' + re.escape('test ' + full + ' ... \nrunning 1 test\nok')
                             + r'(?=\n|\Z)', 'test ' + full + ' ... ok', text, flags=re.M)
        require(count <= 1, 'duplicate known nested child progress')
    return text


def sources(h):
    paths = [ROOT / name for name in ('run_cpu.py', 'supervisor.py', 'worker_support.py', 'cache.py')]
    paths += h.files_below(ROOT / 'ferric') + h.files_below(RUNTIME)
    require(len(paths) == len(set(paths)) == 1001, '813 runtime +184 worker +4 controller/helper sources')
    rows = {str(path.relative_to(ROOT)): h.pin(path) for path in sorted(paths)}
    require(sum(row['bytes'] for row in rows.values()) <= 64 << 20
            and sum(name.startswith('fe2o3/') for name in rows) == 813
            and sum(name.startswith(WORKER_REL) for name in rows) == 184, 'closed source tree')
    return rows


def lineage(h, w, c, inputs, before, readset):
    bodies = {}
    require(set(inputs['lineage']) == set(HISTORY) | {'runtime-proposal.json'}, 'closed current CPU lineage')
    for name in inputs['lineage']:
        path = ROOT / 'inputs' / name
        raw = c.read(path)
        actual = h.pin(path)
        require(w.compact({'x': actual})['x'] == inputs['lineage'][name], 'input lineage hash')
        if name in HISTORY:
            size, digest = HISTORY[name]
            require(c.pin(raw) == dict(bytes=size, sha256=digest), 'literal qualified CPU lineage')
        else:
            require(c.pin(raw)['sha256'] == PROPOSAL_SHA, 'frozen runtime-only proposal')
        readset[name], bodies[name] = actual, raw
    worker, runtime = (c.parse(bodies[name + '-complete.json']) for name in ('worker', 'runtime'))
    maps = {kind: c.parse(bodies[kind + '-sources.json']) for kind in ('worker', 'runtime')}
    for kind, receipt, count in (('worker', worker, 9), ('runtime', runtime, 21)):
        require(receipt['passed'] is True and receipt['failure'] is None and receipt['postcheck_errors'] == []
                and receipt['source_unchanged'] is True and receipt['gpu_execution'] is False
                and receipt['input_sources'] == receipt['final_sources'] == maps[kind]
                and w.compact({'x': receipt['raw']['sources-after.json']})
                    == w.compact({'x': readset[kind + '-sources.json']})
                and len(receipt['phases']) == count and all(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['reaped'] is True and row['process_group_absent'] is True
                    and row['forced_cleanup'] is False for row in receipt['phases']), 'qualified current CPU predecessor')
    require(worker['tool_pins'] == runtime['tool_pins'] == inputs['tool_pins'], 'same qualified toolchain')
    worker_map = w.compact(maps['worker'])
    runtime_map = {name: row for name, row in w.compact(maps['runtime']).items() if name.startswith('fe2o3/')}
    require(len(runtime_map) == 811 and runtime_map == {name: row for name, row in worker_map.items()
            if name.startswith('fe2o3/')}, 'current runtime source bodies identical in both predecessors')
    proposal = c.parse(bodies['runtime-proposal.json'])
    require(proposal['schema'] == 'ferric-engineering-peer-read-pair-runtime-source-v2'
            and len(proposal['files']) == 3 and len({row['path'] for row in proposal['files']}) == 3
            and proposal['worker_source_changed'] is False
            and proposal['default_fresh_arena_policy_changed'] is False
            and proposal['global_currentness_policy_changed'] is False
            and proposal['method_local_currentness_cadence_changed'] is True
            and proposal['ordinary_read_api_changed'] is False
            and proposal['public_runtime_limits_changed'] is False, 'runtime-only paired-read contract')
    for role in ('runtime', 'worker'):
        for suffix, name in (('complete', '-complete.json'), ('sources', '-sources.json')):
            require(proposal['base'][role + '_' + suffix] == c.pin(bodies[role + name]), 'direct proposal ancestry')
    overlay = []
    for row in proposal['files']:
        name = 'fe2o3/' + row['path']
        require(row['repository'] == 'runtime' and w.ordinary_relative(row['path'])
                and row['path'].startswith('crates/fe2o3-kfd/')
                and runtime_map.get(name) == row['before'], 'runtime replacement/addition preimage')
        runtime_map[name] = row['after']
        overlay.append(name)
    require(sum(row['before'] is None for row in proposal['files']) == 2
            and inputs['overlay'] == sorted(overlay), 'two new files and exact formatting boundary')
    expected = {name: row for name, row in worker_map.items() if name.startswith(WORKER_REL)} | runtime_map
    require({name: row for name, row in inputs['files'].items() if name.startswith(('fe2o3/', WORKER_REL))}
            == expected and w.compact(before) == inputs['files'], 'only three runtime overlay rows')
    baselines = {}
    for kind, receipt, label in (('runtime', runtime, 'kfd-tests'), ('worker', worker, 'worker-tests')):
        old = w.outcomes(normalized(bodies[kind + '-tests.stdout'].decode()))
        require(old == receipt['tests'][label] and w.inventory(bodies[kind + '-list.stdout'].decode())
                == sorted(w.statuses(old)), 'actual previous full named outcomes')
        for suffix, raw_label in (('-tests.stdout', label + '.stdout'),
                                  ('-list.stdout', label.replace('-tests', '-list') + '.stdout')):
            require(w.compact({'x': receipt['raw'][raw_label]})
                    == w.compact({'x': readset[kind + suffix]}), 'historical raw receipt join')
        baselines[kind] = old
    new = proposal['test_names']
    require(len(new) == len(set(new)) == 7 and all(re.fullmatch('[A-Za-z0-9_:]+', name) for name in new)
            and not set(new) & set(w.statuses(baselines['runtime']))
            and all(name.startswith(FOCUS['peer-read-pair-tests']) for name in new), 'seven explicit new read-pair names')
    require((baselines['runtime']['passed'], baselines['runtime']['ignored']) == (1101, 8)
            and (baselines['worker']['passed'], baselines['worker']['ignored']) == (615, 4), 'current qualified census')
    require(c.pin(bodies['runtime-docs.stdout'])
            == w.compact({'x': runtime['raw']['interface-doc-tests.stdout']})['x'], 'actual ten-doc raw join')
    return proposal, baselines, bodies['legacy-docs.stdout'].decode(), bodies['runtime-docs.stdout'].decode()

def cache_contract(h, w, c, inputs):
    raw = c.read(ROOT / 'cargo-cache-manifest.json')
    require(c.pin(raw) == inputs['cache_manifest'], 'actual two-lock cache manifest binding')
    manifest = c.parse(raw)
    stage = c.parse(c.read(ROOT / 'cargo-cache-stage-complete.json'))
    locks = {role: c.read(ROOT / row[0]) for role, row in c.LOCKS.items()}
    bodies = {name: c.read(CARGO_HOME / name) for name in manifest['files']}
    require(len(bodies) == 73 and all(c.relative(name) and name.startswith('registry/') for name in bodies),
            'exact immutable private cache inputs')
    c.validate(manifest, bodies, locks)
    require(stage['schema'] == 'ferric-guarded-mlp-reusable-arena-cache-stage-v1' and stage['passed'] is True
            and stage['manifest'] == inputs['cache_manifest'] and stage['locks'] == manifest['locks']
            and stage['cargo_home'] == str(CARGO_HOME) and stage['files'] == manifest['files']
            and stage['packages'] == 39 and stage['cache_files'] == 73
            and stage['controller'] == inputs['files']['cache.py']
            and all(stage[key] is False for key in ('shared_cache_changed', 'lock_changed',
                        'project_code_executed', 'crate_sources_extracted')), 'closed private cache staging')
    return dict(manifest=h.pin(ROOT / 'cargo-cache-manifest.json'),
        stage=h.pin(ROOT / 'cargo-cache-stage-complete.json'),
        files={name: h.pin(CARGO_HOME / name) for name in bodies}, locks=manifest['locks'],
        locked_packages=manifest['locked_packages'], archive=stage['archive'])


def runtime_metadata(h, w, c):
    value = c.parse(c.read(OUT / 'runtime-metadata.stdout'))
    require(value['workspace_root'] == str(RUNTIME) and value['target_directory'] == str(TARGET),
            'runtime workspace/target')
    packages = {row['id']: row for row in value['packages']}
    require(len(packages) == len(value['packages']) <= 256, 'unique bounded runtime package graph')
    expected_local = w.CRATES - {w.PACKAGE}
    local, external = {}, {}
    for row in packages.values():
        path = Path(row['manifest_path'])
        require(path.resolve(strict=True) == path, 'dependency alias')
        if row['source'] is None:
            require(row['name'] in expected_local and row['name'] not in local
                    and path == RUNTIME / 'crates' / row['name'] / 'Cargo.toml', 'closed runtime package')
            local[row['name']] = str(path)
        else:
            require(row['source'] == c.REGISTRY and path.is_relative_to(CARGO_HOME / 'registry/src'),
                    'locked private registry dependency')
            external[str(path.parent)] = {str(p.relative_to(path.parent)): h.pin(p)
                                          for p in h.files_below(path.parent, packed=False)}
    require(set(local) == expected_local and all(packages[key]['source'] is None
            for key in value['workspace_members']), 'nine runtime workspace crates')
    kfd = next(row for row in packages.values() if row['name'] == 'fe2o3-kfd')
    for name, kind, source in KFD_TARGETS.values():
        require(sum(row['name'] == name and row['kind'] == [kind]
                    and row['src_path'] == str(RUNTIME / 'crates/fe2o3-kfd' / source)
                    for row in kfd['targets']) == 1, 'selected KFD target')
    return external, local


def doc_results(text):
    rows, active, summaries = [], [], []
    # Rustdoc emits separate compiled-example and compile-fail sections.
    for line in text.splitlines():
        row = re.fullmatch(r'test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?', line)
        summary = re.fullmatch(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                              r'(\d+) measured; (\d+) filtered out; finished in [0-9.]+s', line)
        if row:
            name, status = row.groups()
            require(name not in {name for name, _ in rows} and status == 'ok', 'unique successful doc result')
            rows.append((name, status))
            active.append((name, status))
        elif summary:
            values = summary.groups()
            require(active and values[:5] == ('ok', str(len(active)), '0', '0', '0'), 'exact doc section census')
            summaries.append(values)
            active = []
        else:
            require(not line.startswith('test '), 'malformed or unsupported doc result')
    require(not active and 1 <= len(summaries) <= 2 and rows
            and sum(int(row[1]) for row in summaries) == len(rows), 'complete successful selected doc census')
    return dict(named=rows, summaries=summaries, passed=len(rows), failed=0, ignored=0)


def doc_regressions(actual, historical):
    cases = []
    def accepted(name, body, count, sections):
        value = doc_results(body)
        require(value['passed'] == count and len(value['summaries']) == sections, 'doc regression ' + name)
        cases.append(dict(name=name, outcome='ok'))
    def refused(name, body):
        require(body != actual, 'doc regression mutation changed bytes')
        try:
            doc_results(body)
        except RuntimeError:
            cases.append(dict(name=name, outcome='ok'))
        else:
            raise RuntimeError('doc regression accepted ' + name)
    accepted('actual_two_sections', actual, 10, 2)
    accepted('historical_one_section', historical, 9, 1)
    first = next(line for line in actual.splitlines() if line.startswith('test ') and ' ... ' in line)
    refused('missing_named_result', actual.replace(first + '\n', '', 1))
    refused('duplicate_named_result', actual.replace(first, first + '\n' + first, 1))
    refused('wrong_section_subtotal', actual.replace('ok. 1 passed;', 'ok. 2 passed;', 1))
    refused('failed_named_result', actual.replace(' ... ok', ' ... FAILED', 1))
    summary = next(line for line in actual.splitlines() if line.startswith('test result:'))
    refused('duplicate_section_summary', actual.replace(summary, summary + '\n' + summary, 1))
    final_summary = [line for line in actual.splitlines() if line.startswith('test result:')][-1]
    refused('missing_final_summary', actual.replace(final_summary + '\n', '', 1))
    require(len(cases) == 8, 'eight exact doc parser regressions')
    return dict(passed=8, failed=0, cases=cases, source='pinned current ten-doc and historical nine-doc raw stdout',
                project_execution=False, gpu_execution=False)


def main():
    started = time.monotonic()
    deadline = started + WHOLE_WALL
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'python3 -B run_cpu.py INPUT_SHA')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and not any(os.path.lexists(path) for path in (OUT, TARGET, TMP)), 'fresh CPU namespace')
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'qualified host and UID')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, '40 GiB initial free floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'nice level')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30)):
        bounds = resource.getrlimit(kind)
        value = min([cap] + [n for n in bounds if n != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    w = load_exact('qualified_worker_support', 'worker_support.py', SUPPORT_SHA)
    c = load_exact('private_cache_data', 'cache.py', CACHE_SHA)
    w.ROOT, w.FERRIC, w.RUNTIME, w.WORKER = ROOT, ROOT / 'ferric', RUNTIME, WORKER
    w.OUT, w.TARGET, w.TMP, w.CARGO_HOME = OUT, TARGET, TMP, CARGO_HOME
    h = w.load_supervisor()
    h.SOURCE = RUNTIME
    h.sources = lambda: sources(h)
    old_scratch = h.scratch_bytes
    def scratch():
        total = old_scratch()
        count = 0
        for directory, dirs, names in os.walk(CARGO_HOME, followlinks=False):
            require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'cache directory alias')
            for name in names:
                count += 1
                try:
                    total += (Path(directory) / name).lstat().st_size
                except FileNotFoundError:
                    pass
        require(count <= 15000, 'bounded generated cache roster')
        return total
    h.scratch_bytes = scratch
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, deadline - CLEANUP_RESERVE - time.monotonic())
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    inputs = input_pin = preformat = formatted = final = config = cache = proposal = baselines = doc_parser_tests = None
    readset, tool_pins, external, external_after, local, artifacts, tests, inventories = {}, {}, {}, {}, {}, {}, {}, {}
    phases, errors, changed = [], [], []
    failure = None
    try:
        input_pin = h.pin(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'literal input pin')
        inputs = c.parse(c.read(ROOT / 'input-manifest.json'))
        require(set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins', 'lineage', 'overlay', 'cache_manifest'}
                and inputs['schema'] == 'ferric-guarded-mlp-peer-read-pair-cpu-input-v2'
                and inputs['source_generation'] == GENERATION, 'closed input schema')
        preformat = sources(h)
        h.save('sources-preformat.json', preformat)
        require((WORKER / '../../../fe2o3').resolve(strict=True) == RUNTIME, 'unchanged worker dependency layout')
        proposal, baselines, old_docs, current_docs = lineage(h, w, c, inputs, preformat, readset)
        doc_parser_tests = doc_regressions(current_docs, old_docs)
        h.save('doc-parser-regression.json', doc_parser_tests)
        config = w.configurations(h)
        tool_pins = {name: h.pin(h.TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
        tool_pins['prlimit'] = h.pin(Path('/usr/bin/prlimit'))
        for name, size in h.SHARED_LIBRARIES.items():
            tool_pins[name] = h.pin(h.TOOLCHAIN_LIB / name)
            require(tool_pins[name]['bytes'] == size, 'qualified compiler library extent')
        require(tool_pins == inputs['tool_pins'], 'qualified immutable compiler toolchain')
        cache = cache_contract(h, w, c, inputs)
        require({str(path.relative_to(CARGO_HOME)) for path in h.files_below(CARGO_HOME, packed=False)}
                == set(cache['files']), 'fresh private Cargo home, no preexisting extracted sources')
        env = dict(HOME='/home/harmenon', PATH=str(h.TOOLCHAIN) + ':/usr/bin:/bin', CARGO_HOME=str(CARGO_HOME),
            CARGO_TARGET_DIR=str(TARGET), LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(h.TOOLCHAIN_LIB),
            TMPDIR=str(TMP), RUSTC=str(h.TOOLCHAIN / 'rustc'), RUSTDOC=str(h.TOOLCHAIN / 'rustdoc'),
            CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
            CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never', MALLOC_ARENA_MAX='2', RUST_BACKTRACE='1',
            CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true',
            CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true', CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
            CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
            ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        def leaf(label, argv, seconds=LEAF_WALL, tested=None, cwd=RUNTIME):
            h.run(label, argv, env, phases, deadline, tested, seconds, cwd)
        fmt = [str(h.TOOLCHAIN / 'rustfmt'), '--edition', '2024', '--config', 'skip_children=true']
        overlay = [str(ROOT / name) for name in inputs['overlay']]
        leaf('rustfmt', [*fmt, *overlay], 120)
        formatted = sources(h)
        changed = sorted(name for name in formatted if formatted[name] != preformat[name])
        require(set(formatted) == set(preformat) and set(changed) <= set(inputs['overlay']), 'format only three runtime files')
        h.save('sources-before.json', formatted)
        leaf('rustfmt-check', [*fmt, '--check', *overlay], 120, formatted)
        leaf('rustc-version', [str(h.TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60, formatted)
        cargo = str(h.TOOLCHAIN / 'cargo')
        common = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(RUNTIME / 'Cargo.toml'), '-p', 'fe2o3-kfd']
        selected = [*common, '--features', 'engineering-gfx950', '--lib', '--tests']
        leaf('runtime-metadata', [cargo, 'metadata', '--offline', '--locked', '--format-version', '1',
                                 '--manifest-path', str(RUNTIME / 'Cargo.toml')], 120, formatted)
        external, local['runtime'] = runtime_metadata(h, w, c)
        h.save('runtime-dependencies-before.json', external)
        leaf('default-check', [cargo, 'check', *common, '--no-default-features'], tested=formatted)
        leaf('kfd-tests-build', [cargo, 'test', *selected, '--no-run', '--message-format=json'], tested=formatted)
        records = h.build_records(OUT / 'kfd-tests-build.stdout')
        require(sum(row.get('reason') == 'compiler-artifact' and row.get('manifest_path')
                == str(RUNTIME / 'crates/fe2o3-kfd/Cargo.toml') and row.get('profile', {}).get('test') is True
                for row in records) == 6, 'exact six runtime test products')
        for role, (name, kind, source) in KFD_TARGETS.items():
            artifact = h.select_artifact(records, 'fe2o3-kfd', name, kind, True)
            row = artifact['cargo_artifact']
            require(row['target']['kind'] == [kind] and row['target']['src_path'] == str(RUNTIME / 'crates/fe2o3-kfd' / source)
                    and row['features'] == ['default', 'engineering-gfx950']
                    and row['filenames'].count(artifact['pin']['path']) == 1, 'runtime product source/features')
            artifacts[role] = artifact
        expected = w.statuses(baselines['runtime']) | {name: 'ok' for name in proposal['test_names']}
        summaries = [dict(row) for row in baselines['runtime']['summaries']]
        summaries[0]['passed'] += 7
        leaf('kfd-list', [cargo, 'test', *selected, '--', '--list', '--format=terse'], 120, formatted)
        leaf('kfd-ignored', [cargo, 'test', *selected, '--', '--ignored', '--list', '--format=terse'], 120, formatted)
        inventories['runtime'] = w.inventory((OUT / 'kfd-list.stdout').read_text())
        require(inventories['runtime'] == sorted(expected) and w.inventory((OUT / 'kfd-ignored.stdout').read_text())
                == sorted(name for name, status in expected.items() if status == 'ignored'), 'runtime exact named inventory')
        leaf('kfd-tests', [cargo, 'test', *selected, '--', '--test-threads=1'], tested=formatted)
        value = w.outcomes(normalized((OUT / 'kfd-tests.stdout').read_text()))
        require(value['summaries'] == summaries and w.statuses(value) == expected
                and (value['passed'], value['failed'], value['ignored']) == (1108, 0, 8), 'all old outcomes plus seven additions')
        tests['kfd-tests'] = value
        for label, prefix in FOCUS.items():
            leaf(label, [cargo, 'test', *common, '--features', 'engineering-gfx950', '--lib', prefix,
                         '--', '--test-threads=1'], 180, formatted)
            value = w.outcomes(normalized((OUT / (label + '.stdout')).read_text()))
            named = {name: status for name, status in expected.items() if name.startswith(prefix)}
            require(named and w.statuses(value) == named and len(value['summaries']) == 1
                    and value['summaries'][0]['filtered_out'] == 1093 - len(named)
                    and value['failed'] == value['ignored'] == 0, 'focused exact library scope')
            tests[label] = value
        docs = [*common, '--features', 'engineering-gfx950', '--doc', DOC_FILTER]
        leaf('interface-doc-list', [cargo, 'test', *docs, '--', '--list', '--format=terse'], 120, formatted)
        doc_names = [line[:-6] for line in (OUT / 'interface-doc-list.stdout').read_text().splitlines() if line.endswith(': test')]
        require(len(doc_names) == len(set(doc_names)) == 10 and all(DOC_FILTER in name for name in doc_names), 'ten selected facade docs')
        leaf('interface-doc-tests', [cargo, 'test', *docs, '--', '--test-threads=1'], 180, formatted)
        value = doc_results((OUT / 'interface-doc-tests.stdout').read_text())
        normalize = lambda name: re.sub(r' \(line \d+\)', '', name)
        old = doc_results(old_docs)
        require(value['passed'] == 10 and Counter(normalize(name) for name, _ in value['named'] if name.endswith(' - compile fail'))
                == Counter(normalize(name) for name, _ in old['named'])
                and sum('bind_guarded_mlp_pair_exact_own_residual_reusable_unchecked_v1' in name
                        and not name.endswith(' - compile fail') for name, _ in value['named']) == 1
                and {re.sub(r' - compile(?: fail)?$', '', name) for name, _ in value['named']} == set(doc_names),
                'nine inherited privacy docs and one new compiled public-signature example')
        tests['interface-doc-tests'] = dict(value, inventory=doc_names)
        wc = ['--offline', '--locked', '--jobs', '2', '--manifest-path', str(WORKER / 'Cargo.toml')]
        ws = [*wc, '--lib', '--bin', w.PACKAGE, '--test', 'shared_wire']
        leaf('metadata', [cargo, 'metadata', '--offline', '--locked', '--format-version', '1',
                         '--manifest-path', str(WORKER / 'Cargo.toml')], 120, formatted, WORKER)
        worker_external, local['worker'] = w.metadata_contract(h)
        for path, rows in worker_external.items():
            require(path not in external or external[path] == rows, 'shared dependency unchanged between resolutions')
            external[path] = rows
        h.save('dependencies-before.json', external)
        leaf('worker-tests-build', [cargo, 'test', *ws, '--no-run', '--message-format=json'], tested=formatted, cwd=WORKER)
        records = h.build_records(OUT / 'worker-tests-build.stdout')
        require(sum(row.get('reason') == 'compiler-artifact' and row.get('manifest_path') == str(WORKER / 'Cargo.toml')
                    and row.get('profile', {}).get('test') is True for row in records) == 3, 'three worker test products')
        for role, (name, kind, source) in w.TARGETS.items():
            artifacts[role] = w.selected_artifact(h, records, name, kind, source, True)
        leaf('worker-list', [cargo, 'test', *ws, '--', '--list', '--format=terse'], 120, formatted, WORKER)
        leaf('worker-ignored', [cargo, 'test', *ws, '--', '--ignored', '--list', '--format=terse'], 120, formatted, WORKER)
        expected_worker = w.statuses(baselines['worker'])
        inventories['worker'] = w.inventory((OUT / 'worker-list.stdout').read_text())
        require(inventories['worker'] == sorted(expected_worker) and w.inventory((OUT / 'worker-ignored.stdout').read_text())
                == sorted(name for name, status in expected_worker.items() if status == 'ignored'), 'unchanged worker inventory')
        leaf('worker-tests', [cargo, 'test', *ws, '--', '--test-threads=1'], tested=formatted, cwd=WORKER)
        value = w.outcomes((OUT / 'worker-tests.stdout').read_text())
        require(value == baselines['worker'], 'every unchanged worker outcome/target summary preserved')
        tests['worker-tests'] = value
        leaf('worker-build', [cargo, 'build', *wc, '--bin', w.PACKAGE, '--message-format=json'], tested=formatted, cwd=WORKER)
        artifacts['worker'] = w.selected_artifact(h, h.build_records(OUT / 'worker-build.stdout'),
                                                w.PACKAGE, 'bin', 'src/main.rs', False)
        require(len({row['pin']['path'] for row in artifacts.values()}) == 10
                and [row['label'] for row in phases] == list(PHASES), 'ten distinct products and exact twenty-two phases')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(label, action):
        try:
            remaining = deadline - 5 - time.monotonic()
            require(remaining > 0, 'postcheck deadline')
            signal.setitimer(signal.ITIMER_REAL, remaining)
            action()
        except BaseException as error:
            errors.append(label + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    def source_check():
        nonlocal final
        final = sources(h)
        h.save('sources-after.json', final)
        if formatted is not None:
            require(final == formatted, 'qualified source/lock drift')
        elif preformat is not None:
            require(set(final) == set(preformat) and all(final[name] == row for name, row in preformat.items()
                    if name not in (inputs or {}).get('overlay', [])), 'failed format changed outside overlay')
    check('sources', source_check)
    if input_pin:
        check('input', lambda: require(h.pin(ROOT / 'input-manifest.json') == input_pin, 'input drift'))
    if config is not None:
        check('configurations', lambda: require(w.configurations(h) == config, 'Cargo configuration drift'))
    if cache is not None:
        check('cache', lambda: require(cache_contract(h, w, c, inputs) == cache, 'immutable cache input drift'))
    for label, row in dict(tool_pins, **readset).items():
        check('readset ' + label, lambda row=row: require(h.pin(Path(row['path'])) == row, 'readset drift'))
    for path, expected in external.items():
        def dependency_check(path=path, expected=expected):
            root = Path(path)
            actual = {str(p.relative_to(root)): h.pin(p) for p in h.files_below(root, packed=False)}
            external_after[path] = actual
            require(actual == expected, 'resolved dependency drift')
        check('dependency ' + path, dependency_check)
    check('dependency ledger', lambda: h.save('dependencies-after.json', external_after))
    for label, artifact in artifacts.items():
        check('product ' + label, lambda artifact=artifact: require(h.pin(Path(artifact['pin']['path'])) == artifact['pin'], 'product drift'))
    raw = {}
    def raw_check():
        nonlocal raw
        raw = {path.name: h.pin(path) for path in sorted(OUT.iterdir()) if path.is_file()}
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw[Path(row[key]['path']).name] == row[key], 'original phase bytes')
    check('raw', raw_check)
    failure = failure or ('postcheck failed' if errors else None)
    if time.monotonic() >= deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-guarded-mlp-peer-read-pair-cpu-v2', passed=failure is None,
        failure=failure, postcheck_errors=errors, source_generation=GENERATION, input_manifest=input_pin,
        controller=h.pin(ROOT / 'run_cpu.py'), supervisor=h.pin(ROOT / 'supervisor.py'),
        support=h.pin(ROOT / 'worker_support.py'), cache_helper=h.pin(ROOT / 'cache.py'),
        readset=readset, baseline_tests=baselines, preformat_sources=preformat, input_sources=formatted,
        final_sources=final, source_unchanged=formatted is not None and formatted == final,
        format_changed_paths=changed, phases=phases, artifacts=artifacts, tests=tests, inventories=inventories,
        tool_pins=tool_pins, local_dependencies=local, cache_provenance=cache, configurations=config, raw=raw,
        elapsed_seconds=time.monotonic() - started,
        doc_parser_tests=doc_parser_tests,
        limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL, cleanup_reserve_seconds=CLEANUP_RESERVE,
            cpu_seconds=h.CPU_LIMIT, address_space_bytes=h.AS_LIMIT, file_bytes=h.FILE_LIMIT,
            cache_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT, initial_free_bytes=h.START_FREE,
            live_free_bytes=h.LIVE_FREE, affinity=[8, 9], nice=10, cargo_jobs=2),
        full_runtime_tests_executed='kfd-tests' in tests, full_worker_tests_executed='worker-tests' in tests,
        selected_facade_doctests_executed='interface-doc-tests' in tests, all_crate_doctests_executed=False,
        peer_read_pair_runtime_added=True, opt_in_retired_arena_reuse_preserved=True,
        default_fresh_arena_policy_changed=False, global_currentness_policy_changed=False,
        method_local_currentness_cadence_changed=True, ordinary_read_api_changed=False,
        public_runtime_limits_changed=False,
        worker_source_changed=False, lockfiles_changed=False, shared_cache_changed=False,
        full_model_long_request_enabled=False, gpu_execution=False, gpu_qualified=False,
        numerical_acceptance=False, performance_claim=False, production_authority=False)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
    try:
        h.save('complete.json' if failure is None else 'failed.json', result)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=failure is None, failure=failure, phases=len(phases),
                         output=str(OUT)), sort_keys=True))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
