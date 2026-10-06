"""Fresh driver-only CPU qualification using the unchanged owned supervisor."""

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
EXPECTED_ROOT = E / 'guarded-mlp-driver-cpu-v228-v1'
SOURCE = ROOT / 'fe2o3'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
BASE_CPU = E / 'guarded-mlp-segment-cpu-v228-v5'
BASE_COMPLETE_SHA = '34df5002c9ca586b5c6dd7e6fc212341b857eedc7ce64bdd4ce0871ccfd3f34f'
BASE_SOURCES_SHA = '955e37e41e54df808221a7d3459f3f782ac7d0b6115ec35e48e14d5f834d4d78'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 3600, 1800, 50
PHASES = ('rustc-version', 'metadata', 'driver-tests-build', 'driver-list',
          'driver-ignored', 'driver-tests', 'driver-build')
ENGINEERING = tuple('engineering_hsaco::tests::' + name for name in (
    'parses_only_exact_target_profiles_cov_and_namespace',
    'rejects_hostile_identities_providers_limits_and_cargo_overrides',
    'manifest_profile_guard_rejects_both_cross_target_directions',
    'extraction_flags_follow_the_requested_profile_without_cargo_overrides',
    'optimized_inline_normalization_is_explicit_and_has_fixed_flags',
    'rejects_loader_and_cargo_selection_environment',
    'isolated_build_std_command_clears_caller_channel_and_owns_fixed_arguments',
    'complete_versioned_build_std_vendor_closure_is_admitted',
    'incomplete_build_std_vendor_fails_before_extraction_command_preparation',
    'build_std_vendor_checksum_substitution_is_rejected',
    'empty_build_std_registry_closure_is_rejected',
    'vendor_configuration_is_closed_and_injection_resistant',
    'claimed_host_linker_path_swap_cannot_change_executed_bytes',
    'substituted_scratch_path_is_never_removed',
    'output_namespace_is_fresh_and_never_uses_production_names',
    'publication_failure_retains_partial_output_without_cleanup',
    'source_has_no_production_or_supervisor_adoption_path',
    'content_namespace_binds_manifest_and_hsaco',
))
IGNORED = tuple(sorted((
    'profile_command::tests::unauthorised_pc_probe_never_executes_caller_selected_avail',
    'profile_command::tests::real_pc_plan_probes_capability_without_running_beta_capture',
    'profile_command::tests::real_pc_plan_and_fake_collector_publish_the_exact_query_tuple',
    'production_source_isa_unit_matrix_v1::ordinary_source_units_round_trip_through_the_production_observer_on_both_targets',
    'production_source_isa_characteristic_matrix_v2::production_adapter_v2::ordinary_source_units_preserve_characteristic_facts_on_both_targets_v2',
)))


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def load_exact(name, path, digest):
    require(path.resolve(strict=True) == path and path.is_file() and not path.is_symlink(),
            'helper must be canonical ordinary source')
    before = path.stat()
    require(before.st_size <= 1 << 20, 'helper size bound')
    body = path.read_bytes()
    after = path.stat()
    require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
            == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
            and hashlib.sha256(body).hexdigest() == digest, 'helper source pin mismatch')
    module = types.ModuleType(name)
    module.__file__ = str(path)
    exec(compile(body, str(path), 'exec'), module.__dict__)
    return module


def compact(rows):
    return {name: {key: row[key] for key in ('bytes', 'sha256')} for name, row in rows.items()}


def selected_driver(h, path, testing):
    rows = h.build_records(path)
    artifact = h.select_artifact(rows, 'cargo-fe2o3', 'cargo-fe2o3', 'bin', testing)
    row = artifact['cargo_artifact']
    require(row['target']['kind'] == ['bin'] and row['target']['crate_types'] == ['bin']
            and row['target']['src_path'] == str(SOURCE / 'crates/cargo-fe2o3/src/main.rs')
            and row['filenames'].count(artifact['pin']['path']) == 1,
            'closed driver Cargo target/filename required')
    if not testing:
        require(artifact['pin']['path'] == str(TARGET / 'debug/cargo-fe2o3'),
                'final production driver path differs')
    return artifact


def main():
    started = time.monotonic()
    deadline = started + WHOLE_WALL
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]), 'python3 -B run_cpu.py INPUT_SHA')
    require(ROOT == EXPECTED_ROOT and ROOT.resolve(strict=True) == ROOT
            and not any(os.path.lexists(p) for p in (OUT, TARGET, TMP)), 'fresh exact outputs required')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID mismatch')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, '40 GiB setup floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice level')
    if priority == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30),
                      (resource.RLIMIT_FSIZE, 1 << 30)):
        soft, hard = resource.getrlimit(kind)
        value = min([cap] + [x for x in (soft, hard) if x != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (value, value))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    h = load_exact('driver_owned_supervisor', ROOT / 'supervisor.py', SUPERVISOR_SHA)
    h.ROOT, h.SOURCE, h.OUT, h.TARGET, h.TMP = ROOT, SOURCE, OUT, TARGET, TMP
    h.ROLES = {'driver': ('cargo-fe2o3', 'cargo-fe2o3', 'bin')}
    original_sources = h.sources
    def sources():
        return dict(original_sources(), **{name: h.pin(ROOT / name)
                    for name in ('supervisor.py',)})
    h.sources = sources
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - CLEANUP_RESERVE - time.monotonic()))
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    before = final = input_pin = base_complete_pin = base_source_pin = config = None
    tool_pins, external, external_after, local, artifacts, tests = {}, {}, {}, {}, {}, {}
    inventory, ignored, phases, errors, failure = [], [], [], [], None
    try:
        input_pin = h.pin(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'literal input mismatch')
        inputs = json.loads((ROOT / 'input-manifest.json').read_bytes())
        require(set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins'}
                and inputs['schema'] == 'ferric-guarded-mlp-driver-cpu-input-v1'
                and inputs['source_generation'] == GENERATION, 'input generation/fields')
        before = sources()
        require(len(before) == 5299 and inputs['files'] == compact(before), 'closed exact source map')
        h.save('sources-before.json', before)
        base_complete_pin = h.pin(BASE_CPU / 'evidence/complete.json')
        base_source_pin = h.pin(BASE_CPU / 'evidence/sources-after.json')
        require(base_complete_pin['sha256'] == BASE_COMPLETE_SHA
                and base_source_pin['sha256'] == BASE_SOURCES_SHA, 'actual base CPU/source pins')
        base = json.loads((BASE_CPU / 'evidence/complete.json').read_bytes())
        base_sources = json.loads((BASE_CPU / 'evidence/sources-after.json').read_bytes())
        require(base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
                and base['dependency_commit'] == GENERATION and base['source_unchanged'] is True
                and base['raw']['sources-after.json'] == base_source_pin,
                'actual source-generation receipt join')
        expected = {name: row for name, row in base_sources.items() if name.startswith('fe2o3/')}
        require(len(expected) == 5297 and compact(expected)
                == compact({name: row for name, row in before.items() if name.startswith('fe2o3/')}),
                'fresh workspace differs from qualified RT source bytes')
        config = h.configurations()
        tool_pins = {name: h.pin(h.TOOLCHAIN / name) for name in ('rustc', 'rustdoc', 'rustfmt', 'cargo')}
        tool_pins['prlimit'] = h.pin(Path('/usr/bin/prlimit'))
        for name, size in h.SHARED_LIBRARIES.items():
            path = h.TOOLCHAIN_LIB / name
            require(path.resolve(strict=True) == path, 'tool library alias')
            tool_pins[name] = h.pin(path)
            require(tool_pins[name]['bytes'] == size, 'tool library extent')
        require(tool_pins == inputs['tool_pins'] == base['tool_pins'], 'exact qualified tool pins')
        env = dict(HOME='/home/harmenon', PATH=str(h.TOOLCHAIN) + ':/usr/bin:/bin',
                   CARGO_HOME='/home/harmenon/.cargo', CARGO_TARGET_DIR=str(TARGET),
                   LD_LIBRARY_PATH=str(TARGET / 'debug/deps') + ':' + str(h.TOOLCHAIN_LIB),
                   TMPDIR=str(TMP), RUSTC=str(h.TOOLCHAIN / 'rustc'), RUSTDOC=str(h.TOOLCHAIN / 'rustdoc'),
                   CARGO_BUILD_JOBS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
                   CARGO_CACHE_AUTO_CLEAN_FREQUENCY='never', MALLOC_ARENA_MAX='2', RUST_BACKTRACE='1',
                   CARGO_PROFILE_DEV_OPT_LEVEL='0', CARGO_PROFILE_DEV_DEBUG='0',
                   CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
                   CARGO_PROFILE_TEST_OPT_LEVEL='0', CARGO_PROFILE_TEST_DEBUG='0',
                   CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
                   ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        def leaf(label, argv, seconds=LEAF_WALL, package=False):
            cwd = SOURCE / 'crates/cargo-fe2o3' if package else ROOT
            selected_env = dict(env)
            if package:
                selected_env['CARGO_MANIFEST_DIR'] = str(cwd)
            h.run(label, argv, selected_env, phases, deadline, before, seconds, cwd)
        cargo = str(h.TOOLCHAIN / 'cargo')
        common = ['--offline', '--locked', '--manifest-path', str(SOURCE / 'Cargo.toml')]
        leaf('rustc-version', [str(h.TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60)
        leaf('metadata', [cargo, 'metadata', *common, '--format-version', '1'], 120)
        external, local = h.metadata_contract()
        h.save('dependencies-before.json', external)
        leaf('driver-tests-build', [cargo, 'test', *common, '--jobs', '2', '-p', 'cargo-fe2o3',
                                  '--bin', 'cargo-fe2o3', '--no-run', '--message-format=json'])
        artifacts['test'] = selected_driver(h, OUT / 'driver-tests-build.stdout', True)
        executable = artifacts['test']['pin']['path']
        leaf('driver-list', [executable, '--list', '--format=terse'], 120, True)
        leaf('driver-ignored', [executable, '--ignored', '--list', '--format=terse'], 120, True)
        inventory = h.inventory(OUT / 'driver-list.stdout')
        ignored = h.inventory(OUT / 'driver-ignored.stdout')
        require(inventory and ignored == list(IGNORED)
                and sorted(name for name in inventory if name.startswith('engineering_hsaco::tests::'))
                    == sorted(ENGINEERING), 'compiled full/ignored/engineering inventory mismatch')
        require(not set(ENGINEERING) & set(ignored), 'engineering tests cannot be ignored')
        leaf('driver-tests', [executable, 'engineering_hsaco::tests::', '--test-threads=1'], package=True)
        tests['driver'] = h.outcomes(OUT / 'driver-tests.stdout', ENGINEERING, len(inventory))
        leaf('driver-build', [cargo, 'build', *common, '--jobs', '2', '-p', 'cargo-fe2o3',
                             '--bin', 'cargo-fe2o3', '--message-format=json'])
        artifacts['driver'] = selected_driver(h, OUT / 'driver-build.stdout', False)
        require([row['label'] for row in phases] == list(PHASES), 'closed seven phases')
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
        final = sources()
        h.save('sources-after.json', final)
        require(final == before, 'source/lock mutation')
    check('sources', source_check)
    if input_pin is not None:
        check('input', lambda: require(h.pin(ROOT / 'input-manifest.json') == input_pin, 'input drift'))
    if config is not None:
        check('configuration', lambda: require(h.configurations() == config, 'configuration drift'))
    for label, expected in tool_pins.items():
        check('tool ' + label, lambda expected=expected: require(h.pin(Path(expected['path'])) == expected, 'tool drift'))
    for label, expected in [('base complete', base_complete_pin), ('base sources', base_source_pin)]:
        if expected is not None:
            check(label, lambda expected=expected: require(h.pin(Path(expected['path'])) == expected, 'base drift'))
    for directory, expected in external.items():
        def dependency_check(directory=directory, expected=expected):
            path = Path(directory)
            actual = {str(p.relative_to(path)): h.pin(p) for p in h.files_below(path, packed=False)}
            external_after[directory] = actual
            require(actual == expected, 'external dependency drift')
        check('dependency ' + directory, dependency_check)
    check('dependency ledger', lambda: h.save('dependencies-after.json', external_after))
    for label, artifact in artifacts.items():
        check('artifact ' + label, lambda artifact=artifact: require(
            h.pin(Path(artifact['pin']['path'])) == artifact['pin'], 'selected artifact drift'))
    raw = {}
    def raw_check():
        nonlocal raw
        raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
        for row in phases:
            for key in ('command', 'stdout', 'stderr'):
                require(raw[Path(row[key]['path']).name] == row[key], 'raw phase join')
    check('raw evidence', raw_check)
    failure = failure or ('postcheck failed' if errors else None)
    if time.monotonic() >= deadline:
        failure = failure or 'whole deadline exceeded'
    result = dict(schema='ferric-guarded-mlp-driver-cpu-v1', passed=failure is None,
                  failure=failure, postcheck_errors=errors, source_generation=GENERATION,
                  controller=h.pin(ROOT / 'run_cpu.py'), supervisor=h.pin(ROOT / 'supervisor.py'),
                  input_manifest=input_pin,
                  base_cpu_complete=base_complete_pin, base_cpu_sources=base_source_pin,
                  input_sources=before, final_sources=final, source_unchanged=before is not None and final == before,
                  phases=phases, artifacts=artifacts, tests=tests, inventory=inventory, ignored=ignored,
                  engineering_tests=sorted(ENGINEERING), test_scope='engineering_hsaco::tests::',
                  full_bin_tests_executed=False, tool_pins=tool_pins, raw=raw,
                  local_dependencies=local, configurations=config, elapsed_seconds=time.monotonic() - started,
                  final_artifact_phase='driver-build', test_artifact_phase='driver-tests-build',
                  limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL,
                              cleanup_reserve_seconds=CLEANUP_RESERVE, cpu_seconds=h.CPU_LIMIT,
                              address_space_bytes=h.AS_LIMIT, file_bytes=h.FILE_LIMIT,
                              cache_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT,
                              initial_free_bytes=h.START_FREE, live_free_bytes=h.LIVE_FREE,
                              affinity=[8, 9], nice=10, cargo_jobs=2),
                  gpu_execution=False, compiler_hsaco_reproduced=False, production_authority=False)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
    try:
        h.save('complete.json' if failure is None else 'failed.json', result)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(dict(passed=failure is None, failure=failure, phases=len(phases),
                          tests={k: {n: v[n] for n in ('passed', 'failed', 'ignored')}
                                 for k, v in tests.items()}, output=str(OUT)), sort_keys=True))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
