"""Bounded seven-tool loader audit joined to an actual full S/RPO producer."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import subprocess
import sys
import time

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
TOOLS = E / 'guarded-mlp-ranked-cfg-cap-compiler-tools-v228-v2'
OUT = E / 'guarded-mlp-ranked-cfg-cap-compiler-tools-audit-v228-v2'
SCRIPT = E / 'audit_guarded_mlp_ranked_cfg_cap_compiler_tools_v228_v2.py'
AUDIT_INPUT = E / 'guarded-mlp-ranked-cfg-cap-compiler-tools-audit-inputs-v228-v2.json'
CPU_ROOT = E / 'guarded-mlp-s-rpo-qualification-cpu-v228-v1'
OLD_MANIFEST_SHA = 'e22e8654ff094cc449ad09a9457bcbe5c07e311e3eb46635e40cf7f3cd5888e3'
CPU_CONTROLLER_SHA = '083abf39c6a9d24ff379c238a7f41c56dfb71757a2678a08f65ca935fd2fc866'
CPU_HELPER_SHA = 'a77cc7f82ebdd9518185b30b837e250abc68cc32e4858ab1ad3dc83a0ec8efe6'
PROPOSAL_SHA = '88a30a490707881c73023b8359773ab271561f1442c51c15d314cea933ba94f2'
PREVIOUS_MANIFEST_SHA = '4d0b2f6220056573e2683e9057f7e716f6fcdf6ef35d70d30d9bfc212f2ca9f6'
DRIVER_ROOT = E / 'guarded-mlp-driver-cpu-v228-v1'
DRIVER_CONTROLLER_SHA = '8b72ba779a3df9125ecfd9170ce8964eefccd1b83f2e595fe09d21a2e44f50f5'
DRIVER_PHASES = ('rustc-version', 'metadata', 'driver-tests-build', 'driver-list',
                 'driver-ignored', 'driver-tests', 'driver-build')
DRIVER_GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
DRIVER_BASE_CPU_SHA = '34df5002c9ca586b5c6dd7e6fc212341b857eedc7ce64bdd4ce0871ccfd3f34f'
DRIVER_BASE_SOURCES_SHA = '955e37e41e54df808221a7d3459f3f782ac7d0b6115ec35e48e14d5f834d4d78'
DRIVER_SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
DRIVER_ENGINEERING = tuple(sorted('engineering_hsaco::tests::' + name for name in (
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
)))
DRIVER_IGNORED = tuple(sorted((
    'profile_command::tests::unauthorised_pc_probe_never_executes_caller_selected_avail',
    'profile_command::tests::real_pc_plan_probes_capability_without_running_beta_capture',
    'profile_command::tests::real_pc_plan_and_fake_collector_publish_the_exact_query_tuple',
    'production_source_isa_unit_matrix_v1::ordinary_source_units_round_trip_through_the_production_observer_on_both_targets',
    'production_source_isa_characteristic_matrix_v2::production_adapter_v2::ordinary_source_units_preserve_characteristic_facts_on_both_targets_v2',
)))
DRIVER_SOURCE_PINS = {
    'fe2o3/crates/cargo-fe2o3/src/engineering_hsaco.rs': '061c6b6e2f34c2f86ec4d37f987bd0a47c72968fc917d8f51899de03a495b94c',
    'fe2o3/crates/cargo-fe2o3/src/engineering_hsaco/execution.rs': 'c9deaaed38da78c4f95687d05f984feda2eac1924d2d9a3db8a45525d0c0be5c',
    'fe2o3/crates/cargo-fe2o3/src/engineering_hsaco/support.rs': 'abf7979e2988077f5aeddac10c5e4f561a7285bdd520736b61fab7971ef063d5',
}

CAPACITY_ROOT = E / 'guarded-mlp-ranked-cfg-cap-cpu-v228-v2'
CAPACITY_CONTROLLER_SHA = '8f7ec46eb8726c85c7cc5651d426d6003351546c1c865ebef7e410246069e3c7'
CAPACITY_COMPLETE_SHA = 'bd31abddedd2f7585d56e1e1807add8454e1047c914d1b9e6a9e63c036989e9c'  # Bind only an actual successful cap CPU receipt.
CAPACITY_PROPOSAL_SHA = '71915fa85ce1ff46c5dd4c1bb96e75f6b27505616923bfec4d657782b46a5d72'
BASE_COMPLETE_SHA = 'e30712ef17c7094efce9bd13bee2bb172823e94f4817c812cfb8b1b20cefb847'
BASE_SOURCES_SHA = 'fa4722605d557e1236809f610bc7af5ebb6e7f15a643ba736392be5b2177bb8a'
DRIVER_MANIFEST_SHA = 'b38ef216900209ea59fbd53a1cb571447f983eb8984ca4a7ce57c1fb10f521fb'
DRIVER_COMPLETE_SHA = 'a869f0f4aa0572c82492cb3d90bdb0dc9bb2625a7b441ff1f3e9a876a87079d9'
CFG_PREFIX = 'production_ranked_projection_v1::tests::cfg_block_limit_diagnostic_'
CFG_NAMES = tuple(sorted(CFG_PREFIX + name for name in (
    'zero_preserves_both_refusals',
    'exact_limit_preserves_graph_acceptance',
    'over_limit_preserves_both_refusals',
    'counts_unreachable_declared_blocks',
    'precedes_work_without_changing_budget',
    'callable_summary_has_no_fabricated_root',
    'root_is_bounded_and_escaped',
    'context_leaves_other_errors_unchanged',
)))
CFG_PATH = 'fe2o3/crates/rustc-codegen-fe2o3/src/'
CFG_FILES = {CFG_PATH + 'production_ranked_projection_v1.rs',
             CFG_PATH + 'production_ranked_projection_v1/loop_switch_domain_v1.rs',
             CFG_PATH + 'production_ranked_projection_v1/cfg_block_limit_diagnostic_v1.rs',
             CFG_PATH + 'production_ranked_projection_v1/cfg_block_limit_diagnostic_v1_tests.rs',
             CFG_PATH + 'production_ranked_projection_v1/analysis_multi_split_v1_tests.rs',
             CFG_PATH + 'production_ranked_projection_v1/projection_08_tests.rs',
             'fe2o3/crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds.rs',
             'fe2o3/crates/fe2o3-pliron/src/production_analysis/pliron_ranked_bounds/resource_tests.rs'}

CAPACITY_COHORTS = {
    'cfg-block-cap-compiler': dict(
        role='compiler-lib',
        prefix='production_ranked_projection_v1::tests::cfg_block_cap_',
        names=['production_ranked_projection_v1::tests::cfg_block_cap_preserves_fixed_graph_work_and_edge_caps']),
    'cfg-block-cap-pliron': dict(
        role='pliron-lib',
        prefix='production_analysis::pliron_ranked_bounds::resource_upper_bound_tests::memory_bounds_block_cap_',
        names=sorted('production_analysis::pliron_ranked_bounds::resource_upper_bound_tests::' + name for name in (
            'memory_bounds_block_cap_accepts_legacy_measured_and_2048',
            'memory_bounds_block_cap_does_not_expand_other_limits',
            'memory_bounds_block_cap_preserves_independent_work_storage_refusals'))),
}

CAPACITY_LIMITS = dict(previous_blocks=1024, blocks=2048, facts=1024, edges=2048,
                       projection_graph_work=3145728, operations=65536,
                       ranked_work=8388608, ranked_storage=131072, findings=4096)

NIGHTLY_LIB = Path('/home/harmenon/.rustup/toolchains/nightly-2026-04-03-x86_64-unknown-linux-gnu/lib')
NAMES = ('cargo-fe2o3', 'clang-22', 'fe2o3-engineering-lld-proxy',
         'fe2o3-llvm-link-worker', 'fe2o3-rustc-extract',
         'librustc_codegen_fe2o3.so', 'lld')
NIGHTLY = {
    'libLLVM.so.22.1-rust-1.96.0-nightly': (199520544, '8af284bb5ae923ac175ddb3f7b9ad16f1f733f7f5f2779e0c9e8c68ef9ba162b'),
    'librustc_driver-7bb70639c3ace5a4.so': (152936640, 'a0aa61a461841224222b0064f9d77a84fe6d3410745d3d96ceaade6ee679cade'),
    'libLLVM-22-rust-1.96.0-nightly.so': (43, 'd43c716e9a7f6e673b4021e1ced69208a21e2433f0f90b49a8701f8be7c056be'),
}
SIGNALS = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM)
AS_LIMIT, STREAM_LIMIT, FILE_LIMIT = 12 << 30, 1 << 20, 1 << 30
FREE_FLOOR, WHOLE_SECONDS, CLEANUP_RESERVE = 40 << 30, 600, 50
INPUTS, ALIASES = {}, {}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def stamp(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_uid,
            value.st_gid, value.st_nlink, value.st_size,
            value.st_mtime_ns, value.st_ctime_ns)


def pin(path, track=True, cap=FILE_LIMIT):
    path = Path(path)
    require(path.is_absolute() and path.resolve(strict=True) == path,
            'noncanonical input: ' + str(path))
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= cap,
            'not a bounded ordinary file: ' + str(path))
    digest = hashlib.sha256()
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    with os.fdopen(fd, 'rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed before read')
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'input changed during read')
    require(stamp(path.lstat()) == stamp(before), 'input changed after read')
    value = dict(path=str(path), bytes=before.st_size, sha256=digest.hexdigest())
    if track:
        require(str(path) not in INPUTS or INPUTS[str(path)] == value, 'conflicting input pin')
        INPUTS[str(path)] = value
    return value


def resolved(path):
    original = str(path)
    path = Path(path)
    require(path.is_absolute(), 'nonabsolute library/tool path')
    actual = path.resolve(strict=True)
    require(original not in ALIASES or ALIASES[original] == str(actual), 'resolution changed')
    ALIASES[original] = str(actual)
    return pin(actual)


def save(name, value):
    path = OUT / name
    with path.open('x') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
    return pin(path, track=False)


def group_exists(pgid):
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def reap_adopted(child):
    rows = []
    child.poll()
    if child.returncode is None:
        return rows
    while True:
        try:
            pid, status = os.waitpid(-child.pid, os.WNOHANG)
        except ChildProcessError:
            break
        if pid == 0:
            break
        rows.append(dict(pid=pid, wait_status=status))
    return rows


def interrupted(signum, _frame):
    raise RuntimeError('interrupted by signal ' + str(signum))


def owned(label, argv, env, leaves, hard_deadline):
    require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB live storage floor')
    require(time.monotonic() + 30 + CLEANUP_RESERVE < hard_deadline,
            'insufficient whole-run reserve for leaf and cleanup')
    command = ['/usr/bin/prlimit', '--as=' + str(AS_LIMIT), '--cpu=30',
               '--fsize=' + str(STREAM_LIMIT), '--core=0', '--', *argv]
    command_pin = save(label + '.command.json', dict(argv=command, cwd=str(OUT),
                                                    env=env, wall_seconds=30))
    stdout, stderr = OUT / (label + '.stdout'), OUT / (label + '.stderr')
    start, child, exception = time.monotonic_ns(), None, None
    timed_out = forced = False
    reaped, deferred = [], []
    group_absent = False
    with stdout.open('xb') as so, stderr.open('xb') as se:
        previous = {sig: signal.getsignal(sig) for sig in SIGNALS}
        def defer(signum, _frame):
            if len(deferred) < 16:
                deferred.append(signum)
        for sig in SIGNALS:
            signal.signal(sig, defer)
        try:
            require(not deferred, 'termination requested before spawn')
            child = subprocess.Popen(command, cwd=OUT, env=env, stdin=subprocess.DEVNULL,
                                     stdout=so, stderr=se, start_new_session=True)
            require(os.getpgid(child.pid) == child.pid, 'owned process group identity')
            save(label + '.started.json', dict(pid=child.pid, pgid=child.pid, argv=command))
            deadline = time.monotonic() + 30
            while child.poll() is None:
                require(not deferred, 'termination during owned leaf: ' + repr(deferred))
                require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB live storage floor')
                if time.monotonic() >= deadline:
                    timed_out = True
                    break
                time.sleep(0.05)
        except BaseException as error:
            exception = repr(error)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            try:
                if child is not None:
                    reaped.extend(reap_adopted(child))
                    if child.returncode is None or group_exists(child.pid):
                        forced = True
                        for sig, allowance in ((signal.SIGTERM, 10), (signal.SIGKILL, 20)):
                            try:
                                os.killpg(child.pid, sig)
                            except ProcessLookupError:
                                pass
                            deadline = min(hard_deadline - 5, time.monotonic() + allowance)
                            while time.monotonic() < deadline:
                                reaped.extend(reap_adopted(child))
                                if child.returncode is not None and not group_exists(child.pid):
                                    break
                                time.sleep(0.02)
                            if child.returncode is not None and not group_exists(child.pid):
                                break
                    try:
                        child.wait(timeout=max(0.001, min(5, hard_deadline - time.monotonic())))
                    except subprocess.TimeoutExpired:
                        exception = (exception or '') + ' leader not reaped'
                    reaped.extend(reap_adopted(child))
                    group_absent = not group_exists(child.pid)
            finally:
                for sig, handler in previous.items():
                    signal.signal(sig, handler)
    if deferred:
        exception = (exception or '') + ' deferred signals: ' + repr(deferred)
    row = dict(label=label, command=command_pin, pid=child.pid if child else None,
               pgid=child.pid if child else None, exit_code=child.returncode if child else None,
               timed_out=timed_out, forced_cleanup=forced, exception=exception,
               observed_signals=deferred, adopted_reaped=reaped,
               natural_exit=child is not None and child.returncode is not None
                   and not timed_out and not forced and exception is None,
               reaped=child is not None and child.returncode is not None,
               process_group_absent=group_absent, elapsed_ns=time.monotonic_ns() - start,
               stdout=pin(stdout, False, STREAM_LIMIT), stderr=pin(stderr, False, STREAM_LIMIT))
    leaves.append(row)
    save(label + '.result.json', row)
    require(row['natural_exit'] and row['exit_code'] == 0 and row['reaped'] and group_absent,
            label + ' did not exit naturally, successfully and fully reaped')
    require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB post-leaf storage floor')
    signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline - CLEANUP_RESERVE - time.monotonic()))
    return stdout.read_text()


def libraries(readelf, ldd, shared_object):
    needed = re.findall(r'\(NEEDED\).*Shared library: \[([^\]\n]+)\]', readelf)
    interpreters = re.findall(r'\[Requesting program interpreter: ([^\]\n]+)\]', readelf)
    require(needed and len(needed) == len(set(needed)), 'missing or duplicate DT_NEEDED')
    require(len(interpreters) == (0 if shared_object else 1), 'unexpected PT_INTERP count')
    require(ldd.strip() and 'not found' not in ldd and 'not a dynamic executable' not in ldd,
            'empty or unresolved ldd output')
    named, direct, virtual = {}, [], []
    for line in ldd.splitlines():
        if not line.strip():
            continue
        library = re.fullmatch(r'\s*(\S+) => (/\S+) \(0x[0-9a-fA-F]+\)\s*', line)
        absolute = re.fullmatch(r'\s*(/\S+) \(0x[0-9a-fA-F]+\)\s*', line)
        vdso = re.fullmatch(r'\s*linux-vdso\.so\.1 \(0x[0-9a-fA-F]+\)\s*', line)
        if library:
            name, path = library.groups()
            require(name not in named, 'duplicate resolved SONAME')
            named[name] = dict(reported_path=path, pin=resolved(path))
        elif absolute:
            direct.append(absolute.group(1))
        elif vdso:
            virtual.append('linux-vdso.so.1')
        else:
            raise RuntimeError('unrecognized ldd line: ' + line)
    require(len(virtual) == 1 and len(direct) <= 1, 'ambiguous direct loader/vDSO')
    loader = None
    if direct:
        path = direct[0]
        loader = dict(reported_path=path, pin=resolved(path))
        if Path(path).name in needed:
            name = Path(path).name
            require(name not in named or named[name]['pin'] == loader['pin'],
                    'named/direct loader disagreement')
            named[name] = loader
    interpreter = None
    if interpreters:
        path = interpreters[0]
        interpreter = dict(reported_path=path, pin=resolved(path))
        require(loader is not None and loader['pin'] == interpreter['pin'],
                'PT_INTERP does not match ldd direct loader')
    require(set(needed) <= set(named), 'unresolved DT_NEEDED closure')
    return dict(needed=needed, libraries=named, interpreter=interpreter,
                direct_loader=loader, virtual_objects=virtual, all_needed_resolved=True,
                shared_object_without_interpreter=shared_object)


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def read_json(row, digest=None):
    require(type(row) is dict and set(row) == {'path', 'bytes', 'sha256'},
            'closed input FilePin')
    path = Path(row['path'])
    require(pin(path, cap=64 << 20) == row and (digest is None or row['sha256'] == digest),
            'actual input digest/extent mismatch')
    raw = path.read_bytes()
    require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256']
            and pin(path, cap=64 << 20) == row, 'retained JSON changed during read')
    return json.loads(raw)


def producer_contract(value, inputs, sources, records, manifest, old):
    require(value['schema'] == 'ferric-guarded-mlp-s-rpo-qualification-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['diagnostic_build'] is False
            and value['controller']['sha256'] == CPU_CONTROLLER_SHA
            and value['helper']['sha256'] == CPU_HELPER_SHA,
            'actual successful reviewed full S/RPO producer required')
    require(inputs['schema'] == 'ferric-guarded-mlp-s-rpo-qualification-input-v1'
            and inputs['lineage'] == value['source_lineage']
            and inputs['lineage']['proposal']['sha256'] == PROPOSAL_SHA
            and value['final_compiler_product_phase'] == 'compiler-tests-build',
            'actual source lineage and final build phase')
    require(len(sources) == 5795 and {name: compact(row) for name, row in sources.items()} == inputs['files']
            and sources['run_cpu.py']['sha256'] == CPU_CONTROLLER_SHA
            and sources['qualification_helpers.py']['sha256'] == CPU_HELPER_SHA,
            'complete 5793-source/controller/helper map')
    phases = value['phases']
    require(len(phases) == len({row['label'] for row in phases}) == 32
            and all(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['reaped'] is True and row['process_group_absent'] is True
                    and row['forced_cleanup'] is False and row['timed_out'] is False
                    and row['exception'] is None and row['storage_failure'] is None
                    for row in phases), 'all 32 actual CPU leaves must pass naturally')
    require(len(value['tests']) == 19 and value['tests_passed'] == 2798
            and value['tests_ignored'] == 25
            and sum(row['passed'] for row in value['tests'].values()) == value['tests_passed']
            and sum(row['ignored'] for row in value['tests'].values()) == value['tests_ignored']
            and all(row['failed'] == 0 for row in value['tests'].values())
            and (value['tests']['compiler']['passed'], value['tests']['compiler']['ignored']) == (1239, 24)
            and (value['tests']['pliron']['passed'], value['tests']['pliron']['ignored']) == (1504, 1),
            'full named-suite and focused/extraction result census')
    require([row.get('success') for row in records if row.get('reason') == 'build-finished'] == [True],
            'actual final Cargo build-finished success')
    artifacts = value['artifacts']
    require(set(artifacts) == {'backend', 'backend-rlib', 'extractor', 'pliron-lib',
                              'compiler-lib', 'atomic-extraction', 'matrix-extraction'},
            'seven exact final CPU product roles')
    package = CPU_ROOT / 'fe2o3/crates/rustc-codegen-fe2o3'
    for role, name, kind, filename, source in (
            ('backend', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.so', 'src/lib.rs'),
            ('backend-rlib', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.rlib', 'src/lib.rs'),
            ('extractor', 'fe2o3-rustc-extract', ['bin'], 'fe2o3-rustc-extract', 'src/bin/fe2o3-rustc-extract.rs')):
        selected = artifacts[role]
        matches = [row for row in records if row.get('reason') == 'compiler-artifact'
                   and row.get('manifest_path') == str(package / 'Cargo.toml')
                   and row.get('target', {}).get('name') == name
                   and sorted(row['target'].get('kind', [])) == kind
                   and sorted(row['target'].get('crate_types', [])) == kind
                   and row['target'].get('src_path') == str(package / source)
                   and row.get('profile', {}).get('test') is False]
        selected_path = Path(selected['pin']['path'])
        require(len(matches) == 1 and selected['cargo_artifact'] == matches[0]
                and selected_path.is_absolute() and '..' not in selected_path.parts
                and selected_path.is_relative_to(CPU_ROOT / 'target')
                and selected_path.name == filename
                and selected['pin']['path'] in matches[0]['filenames'],
                'selected product must be from final checked Cargo records: ' + role)
        require(matches[0]['executable'] == (selected['pin']['path'] if role == 'extractor' else None),
                'selected library/executable Cargo role')
    require(artifacts['backend']['cargo_artifact'] == artifacts['backend-rlib']['cargo_artifact'],
            'backend ELF and rlib must share the final actual library record')
    require(set(manifest) == set(old) == {'bin/' + name for name in NAMES},
            'closed seven-tool deployment manifest')
    replacements = {'fe2o3-rustc-extract': 'extractor', 'librustc_codegen_fe2o3.so': 'backend'}
    for name in NAMES:
        row = manifest['bin/' + name]
        require(set(row) == {'source', 'bytes', 'sha256'}, 'closed deployment tool row')
        if name in replacements:
            product = artifacts[replacements[name]]['pin']
            require(row == dict(source=product['path'], **compact(product)),
                    'new extractor/backend must be actual final producer bodies')
        else:
            require(row == old['bin/' + name], 'legacy driver/linker/worker identity changed')
    return dict(rlib_cpu_provenance=artifacts['backend-rlib'],
                test_elf_cpu_provenance={role: artifacts[role] for role in
                    ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')},
                omitted_artifact_bodies_rehashed=False)


def qualified_producer(audit_input, manifest):
    for key, relative in (('producer_complete', 'evidence/complete.json'),
                          ('producer_input', 'input-manifest.json'),
                          ('producer_sources', 'evidence/sources-before.json'),
                          ('producer_final_build', 'evidence/compiler-tests-build.stdout')):
        require(audit_input[key]['path'] == str(CPU_ROOT / relative),
                'exact current producer readset path: ' + key)
    old = read_json(audit_input['old_tool_manifest'], OLD_MANIFEST_SHA)
    value = read_json(audit_input['producer_complete'])
    require(value['input_manifest'] == audit_input['producer_input']
            and value['input_sources'] == audit_input['producer_sources']
            and compact(value['final_sources']) == compact(audit_input['producer_sources'])
            and value['raw']['compiler-tests-build.stdout'] == audit_input['producer_final_build'],
            'producer receipt/readset identity join')
    inputs = read_json(audit_input['producer_input'])
    sources = read_json(audit_input['producer_sources'])
    build_pin = audit_input['producer_final_build']
    path = Path(build_pin['path'])
    require(pin(path, cap=64 << 20) == build_pin, 'actual final Cargo stdout pin')
    raw = path.read_bytes()
    require(len(raw) == build_pin['bytes'] and hashlib.sha256(raw).hexdigest() == build_pin['sha256']
            and pin(path, cap=64 << 20) == build_pin, 'final Cargo stdout changed during read')
    records = [json.loads(line) for line in raw.decode().splitlines() if line.startswith('{')]
    proof = producer_contract(value, inputs, sources, records, manifest, old)
    return dict(complete=audit_input['producer_complete'], input=audit_input['producer_input'],
                sources=audit_input['producer_sources'], final_build=build_pin,
                source_lineage=value['source_lineage'], **proof)



def driver_contract(value, inputs, sources, build, tests_build, manifest, previous):
    require(value['schema'] == 'ferric-guarded-mlp-driver-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['controller']['sha256'] == DRIVER_CONTROLLER_SHA
            and value['supervisor']['sha256'] == DRIVER_SUPERVISOR_SHA
            and value['source_generation'] == DRIVER_GENERATION
            and value['base_cpu_complete']['sha256'] == DRIVER_BASE_CPU_SHA
            and value['base_cpu_sources']['sha256'] == DRIVER_BASE_SOURCES_SHA
            and value['final_artifact_phase'] == 'driver-build'
            and value['test_artifact_phase'] == 'driver-tests-build',
            'actual successful reviewed driver producer required')
    require(set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins'}
            and inputs['schema'] == 'ferric-guarded-mlp-driver-cpu-input-v1'
            and inputs['source_generation'] == DRIVER_GENERATION
            and inputs['tool_pins'] == value['tool_pins']
            and len(sources) == 5299
            and {name: compact(row) for name, row in sources.items()} == inputs['files']
            and len([name for name in sources if name.startswith('fe2o3/')]) == 5297
            and sources['run_cpu.py'] == value['controller']
            and sources['supervisor.py'] == value['supervisor']
            and all(row['path'] == str(DRIVER_ROOT / name) for name, row in sources.items())
            and all(sources[name]['sha256'] == digest for name, digest in DRIVER_SOURCE_PINS.items()),
            'complete actual driver source map and normalized CLI bodies')
    require([row['label'] for row in value['phases']] == list(DRIVER_PHASES)
            and all(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['reaped'] is True and row['process_group_absent'] is True
                    and row['forced_cleanup'] is False and row['timed_out'] is False
                    and row['exception'] is None and row['storage_failure'] is None
                    for row in value['phases']), 'all seven actual driver leaves must pass naturally')
    require(set(value['tests']) == {'driver'} and value['full_bin_tests_executed'] is False
            and value['test_scope'] == 'engineering_hsaco::tests::'
            and value['engineering_tests'] == list(DRIVER_ENGINEERING),
            'only the exact engineering driver cohort is qualified')
    suite = value['tests']['driver']
    names, ignored = value['inventory'], value['ignored']
    require(names == sorted(set(names)) and ignored == list(DRIVER_IGNORED)
            and set(ignored) <= set(names) and not set(DRIVER_ENGINEERING) & set(ignored)
            and sorted(name for name in names if name.startswith('engineering_hsaco::tests::'))
                == list(DRIVER_ENGINEERING)
            and suite == dict(names=list(DRIVER_ENGINEERING), passed=18, failed=0, ignored=0,
                              filtered_out=len(names) - 18),
            'exact eighteen passing engineering names and compiled filtered census')
    require(set(value['artifacts']) == {'driver', 'test'}, 'exact driver/test artifact roles')
    package = DRIVER_ROOT / 'fe2o3/crates/cargo-fe2o3'
    for role, records, test in (('driver', build, False), ('test', tests_build, True)):
        require([row.get('success') for row in records if row.get('reason') == 'build-finished'] == [True],
                'actual successful final Cargo stream: ' + role)
        matches = [row for row in records if row.get('reason') == 'compiler-artifact'
                   and row.get('manifest_path') == str(package / 'Cargo.toml')
                   and row.get('target', {}).get('name') == 'cargo-fe2o3'
                   and row['target'].get('kind') == ['bin']
                   and row['target'].get('crate_types') == ['bin']
                   and row['target'].get('src_path') == str(package / 'src/main.rs')
                   and row.get('profile', {}).get('test') is test]
        artifact = value['artifacts'][role]
        path = Path(artifact['pin']['path'])
        require(len(matches) == 1 and artifact['cargo_artifact'] == matches[0]
                and path.is_absolute() and '..' not in path.parts
                and path.is_relative_to(DRIVER_ROOT / 'target')
                and str(path) == matches[0]['executable'] and str(path) in matches[0]['filenames']
                and (test or path == DRIVER_ROOT / 'target/debug/cargo-fe2o3'),
                'driver/test body must join its exact final Cargo product: ' + role)
    require(set(manifest) == set(previous) == {'bin/' + name for name in NAMES},
            'closed previous and new seven-tool rosters')
    product = value['artifacts']['driver']['pin']
    for name in NAMES:
        row = manifest['bin/' + name]
        require(set(row) == {'source', 'bytes', 'sha256'}, 'closed new deployment row')
        expected = dict(source=product['path'], **compact(product)) if name == 'cargo-fe2o3' else previous['bin/' + name]
        require(row == expected, 'only the exact final driver may replace a previous tool: ' + name)
    return dict(driver=value['artifacts']['driver'], test_elf_cpu_provenance=value['artifacts']['test'],
                omitted_test_elf_body_rehashed=False)


def qualified_driver(audit_input, manifest, previous):
    paths = (('driver_complete', 'evidence/complete.json'),
             ('driver_input', 'input-manifest.json'),
             ('driver_sources', 'evidence/sources-before.json'),
             ('driver_final_build', 'evidence/driver-build.stdout'),
             ('driver_test_build', 'evidence/driver-tests-build.stdout'))
    for key, relative in paths:
        require(audit_input[key]['path'] == str(DRIVER_ROOT / relative),
                'exact driver readset path: ' + key)
    value = read_json(audit_input['driver_complete'])
    require(value['input_manifest'] == audit_input['driver_input']
            and value['raw']['sources-before.json'] == audit_input['driver_sources']
            and compact(value['raw']['sources-after.json']) == compact(audit_input['driver_sources'])
            and value['raw']['driver-build.stdout'] == audit_input['driver_final_build']
            and value['raw']['driver-tests-build.stdout'] == audit_input['driver_test_build'],
            'actual driver receipt/readset join')
    require(pin(Path(value['controller']['path'])) == value['controller'],
            'actual reviewed driver controller body')
    inputs = read_json(audit_input['driver_input'])
    sources = read_json(audit_input['driver_sources'])
    require(value['input_sources'] == value['final_sources'] == sources,
            'actual initial/final driver source map equality')
    streams = []
    for key in ('driver_final_build', 'driver_test_build'):
        row = audit_input[key]
        path = Path(row['path'])
        require(pin(path, cap=64 << 20) == row, 'driver Cargo stdout pin')
        raw = path.read_bytes()
        require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256']
                and pin(path, cap=64 << 20) == row, 'driver Cargo stdout changed during read')
        streams.append([json.loads(line) for line in raw.decode().splitlines() if line.startswith('{')])
    proof = driver_contract(value, inputs, sources, *streams, manifest, previous)
    return dict(complete=audit_input['driver_complete'], input=audit_input['driver_input'],
                sources=audit_input['driver_sources'], final_build=audit_input['driver_final_build'],
                test_build=audit_input['driver_test_build'], **proof)



def capacity_contract(value, inputs, sources, records, manifest, previous,
                        base, base_sources, proposal):
    require(value['schema'] == 'ferric-guarded-mlp-ranked-cfg-cap-cpu-v1'
            and value['passed'] is True and value['failure'] is None
            and value['postcheck_errors'] == [] and value['source_unchanged'] is True
            and value['diagnostic_build'] is False
            and value['admission_changed'] is True
            and value['structural_capacity_expansion'] is True
            and value['cfg_diagnostics_retained'] is True
            and value['capacity_limits'] == CAPACITY_LIMITS
            and value['capacity_cohorts'] == CAPACITY_COHORTS
            and value['cfg_diagnostic_tests'] == list(CFG_NAMES)
            and value['controller']['sha256'] == CAPACITY_CONTROLLER_SHA
            and value['helper']['sha256'] == CPU_HELPER_SHA
            and value['final_compiler_product_phase'] == 'compiler-tests-build',
            'actual successful explicit structural-capacity producer required')
    require(inputs['schema'] == 'ferric-guarded-mlp-ranked-cfg-cap-cpu-input-v1'
            and inputs['lineage'] == value['source_lineage']
            and len(sources) == 5797
            and {name: compact(row) for name, row in sources.items()} == inputs['files']
            and sources['run_cpu.py'] == value['controller']
            and sources['qualification_helpers.py'] == value['helper']
            and all(row['path'] == str(CAPACITY_ROOT / name) for name, row in sources.items()),
            'complete capacity source/controller/helper map')
    lineage = inputs['lineage']
    cohorts = {'cfg-block-limit-diagnostic': dict(role='compiler-lib', prefix=CFG_PREFIX,
                                               names=list(CFG_NAMES)), **CAPACITY_COHORTS}
    proposal_cohorts = {label: dict(filter=row['prefix'], names=row['names'])
                        for label, row in cohorts.items()}
    require(set(lineage) == {'base_complete', 'base_sources', 'base_input', 'base_metadata',
                            'base_dependencies', 'base_streams', 'capacity_proposal',
                            'capacity_overlay'}
            and lineage['base_complete']['sha256'] == BASE_COMPLETE_SHA
            and lineage['base_sources']['sha256'] == BASE_SOURCES_SHA
            and lineage['capacity_proposal']['sha256'] == CAPACITY_PROPOSAL_SHA
            and proposal['schema'] == 'ferric-guarded-mlp-ranked-cfg-cap-source-v1'
            and proposal['admission_changed'] is True
            and proposal['structural_capacity_expansion'] is True
            and proposal['cfg_diagnostics_retained'] is True
            and proposal['runtime_or_kernel_source_changes'] is False
            and proposal['block_limit'] == dict(before=1024, after=2048)
            and proposal['preserved_limits'] == dict(
                facts=1024, edges=2048, operations=65536, findings=4096,
                projector_graph_work=3145728, bounds_work=8388608, bounds_storage_items=131072)
            and proposal['base_complete'] == compact(lineage['base_complete'])
            and proposal['base_sources'] == compact(lineage['base_sources'])
            and proposal['cohorts'] == proposal_cohorts,
            'exact qualified baseline and reviewed capacity proposal')
    expected = {name: compact(row) for name, row in base_sources.items()
                if name.startswith('fe2o3/')}
    overlay = {'fe2o3/' + name: row for name, row in proposal['files'].items()}
    require(len(expected) == 5793 and set(overlay) == CFG_FILES
            and lineage['capacity_overlay'] == overlay, 'closed eight-file capacity overlay')
    for name, row in overlay.items():
        require(set(row) == {'before', 'after'}
                and row['before'] == expected.get(name)
                and set(row['after']) == {'bytes', 'sha256'}, 'exact capacity preimage')
        expected[name] = row['after']
    require(len(expected) == 5795
            and {name: compact(row) for name, row in sources.items()
                 if name.startswith('fe2o3/')} == expected,
            'all source bodies outside capacity overlay must remain exact')
    phases = value['phases']
    labels = [row['label'] for row in phases]
    old_labels = [row['label'] for row in base['phases']]
    extra_labels = list(cohorts)
    require(len(labels) == len(set(labels)) == 35
            and [label for label in labels if label not in cohorts] == old_labels
            and labels[labels.index('atomic-extraction-0')-3:labels.index('atomic-extraction-0')]
                == extra_labels
            and all(row['exit_code'] == 0 and row['natural_exit'] is True
                    and row['reaped'] is True and row['process_group_absent'] is True
                    and row['forced_cleanup'] is False and row['timed_out'] is False
                    and row['exception'] is None and row['storage_failure'] is None
                    for row in phases), 'all inherited CPU leaves plus three exact repeats')
    inventories = dict(base['test_inventories'])
    additions = {role: [name for row in cohorts.values() if row['role'] == role
                        for name in row['names']] for role in ('compiler-lib', 'pliron-lib')}
    for role, names in additions.items():
        require(not set(names) & set(inventories[role]), 'capacity names must be new')
        inventories[role] = sorted(inventories[role] + names)
    require(value['test_inventories'] == inventories
            and value['ignored_inventories'] == base['ignored_inventories'],
            'exact old compiled inventories and ignore identities plus twelve names')
    tests = value['tests']
    require(set(tests) == set(base['tests']) | set(cohorts)
            and value['tests_passed'] == 2822 and value['tests_ignored'] == 25
            and sum(row['passed'] for row in tests.values()) == 2822
            and sum(row['ignored'] for row in tests.values()) == 25,
            'twenty-two scopes and actual passing/ignored census')
    for label, prior in base['tests'].items():
        wanted = dict(prior)
        if label in ('compiler', 'pliron'):
            role = label + '-lib'
            wanted['names'] = inventories[role]
            wanted['named_outcomes'] = dict(prior['named_outcomes'],
                                            **{name: 'ok' for name in additions[role]})
            wanted['passed'] += len(additions[role])
        elif label in base['compiler_cohorts']:
            wanted['filtered_out'] += len(additions['compiler-lib'])
        require(tests[label] == wanted, 'inherited named outcomes changed: ' + label)
    for label, row in cohorts.items():
        names = row['names']
        require(tests[label] == dict(
                    names=names, passed=len(names), failed=0, ignored=0,
                    filtered_out=len(inventories[row['role']]) - len(names)),
                'actual passing capacity/diagnostic names: ' + label)
    require([row.get('success') for row in records if row.get('reason') == 'build-finished'] == [True],
            'actual final Cargo build-finished success')
    artifacts = value['artifacts']
    require(set(artifacts) == {'backend', 'backend-rlib', 'extractor', 'pliron-lib',
                              'compiler-lib', 'atomic-extraction', 'matrix-extraction'},
            'seven exact final CPU product roles')
    package = CAPACITY_ROOT / 'fe2o3/crates/rustc-codegen-fe2o3'
    for role, name, kind, filename, source in (
            ('backend', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.so', 'src/lib.rs'),
            ('backend-rlib', 'rustc_codegen_fe2o3', ['dylib', 'rlib'], 'librustc_codegen_fe2o3.rlib', 'src/lib.rs'),
            ('extractor', 'fe2o3-rustc-extract', ['bin'], 'fe2o3-rustc-extract', 'src/bin/fe2o3-rustc-extract.rs')):
        selected = artifacts[role]
        matches = [row for row in records if row.get('reason') == 'compiler-artifact'
                   and row.get('manifest_path') == str(package / 'Cargo.toml')
                   and row.get('target', {}).get('name') == name
                   and sorted(row['target'].get('kind', [])) == kind
                   and sorted(row['target'].get('crate_types', [])) == kind
                   and row['target'].get('src_path') == str(package / source)
                   and row.get('profile', {}).get('test') is False]
        selected_path = Path(selected['pin']['path'])
        require(len(matches) == 1 and selected['cargo_artifact'] == matches[0]
                and selected_path.is_absolute() and '..' not in selected_path.parts
                and selected_path.is_relative_to(CAPACITY_ROOT / 'target')
                and selected_path.name == filename
                and selected['pin']['path'] in matches[0]['filenames'],
                'selected product must be from final checked Cargo records: ' + role)
        require(matches[0]['executable'] == (selected['pin']['path'] if role == 'extractor' else None),
                'selected library/executable Cargo role')
    require(artifacts['backend']['cargo_artifact'] == artifacts['backend-rlib']['cargo_artifact'],
            'backend ELF and rlib must share the final actual library record')
    require(set(manifest) == set(previous) == {'bin/' + name for name in NAMES},
            'closed seven-tool deployment manifest')
    replacements = {'fe2o3-rustc-extract': 'extractor', 'librustc_codegen_fe2o3.so': 'backend'}
    for name in NAMES:
        row = manifest['bin/' + name]
        require(set(row) == {'source', 'bytes', 'sha256'}, 'closed deployment tool row')
        if name in replacements:
            product = artifacts[replacements[name]]['pin']
            require(row == dict(source=product['path'], **compact(product)),
                    'new extractor/backend must be actual final producer bodies')
        else:
            require(row == previous['bin/' + name], 'unchanged qualified driver/linker/worker identity changed')
    return dict(rlib_cpu_provenance=artifacts['backend-rlib'],
                test_elf_cpu_provenance={role: artifacts[role] for role in
                    ('pliron-lib', 'compiler-lib', 'atomic-extraction', 'matrix-extraction')},
                omitted_artifact_bodies_rehashed=False)



def qualified_capacity_producer(audit_input, manifest, previous):
    for key, relative in (('capacity_complete', 'evidence/complete.json'),
                          ('capacity_input', 'input-manifest.json'),
                          ('capacity_sources', 'evidence/sources-before.json'),
                          ('capacity_final_build', 'evidence/compiler-tests-build.stdout')):
        require(audit_input[key]['path'] == str(CAPACITY_ROOT / relative),
                'exact capacity producer readset: ' + key)
    value = read_json(audit_input['capacity_complete'], CAPACITY_COMPLETE_SHA)
    require(value['input_manifest'] == audit_input['capacity_input']
            and value['input_sources'] == audit_input['capacity_sources']
            and compact(value['final_sources']) == compact(audit_input['capacity_sources'])
            and value['raw']['compiler-tests-build.stdout'] == audit_input['capacity_final_build'],
            'actual capacity receipt/readset join')
    inputs = read_json(audit_input['capacity_input'])
    sources = read_json(audit_input['capacity_sources'])
    lineage = inputs['lineage']
    base = read_json(lineage['base_complete'], BASE_COMPLETE_SHA)
    base_sources = read_json(lineage['base_sources'], BASE_SOURCES_SHA)
    require(compact(lineage['base_complete']) == compact(audit_input['producer_complete'])
            and compact(lineage['base_sources']) == compact(audit_input['producer_sources']),
            'capacity baseline must be the independently replayed qualified S producer')
    proposal = read_json(lineage['capacity_proposal'], CAPACITY_PROPOSAL_SHA)
    require(pin(Path(value['controller']['path'])) == value['controller'],
            'actual capacity producer controller body')
    row = audit_input['capacity_final_build']
    path = Path(row['path'])
    require(pin(path, cap=64 << 20) == row, 'actual final capacity Cargo stdout')
    raw = path.read_bytes()
    require(len(raw) == row['bytes'] and hashlib.sha256(raw).hexdigest() == row['sha256']
            and pin(path, cap=64 << 20) == row, 'capacity Cargo stdout changed')
    records = [json.loads(line) for line in raw.decode().splitlines() if line.startswith('{')]
    proof = capacity_contract(value, inputs, sources, records, manifest, previous,
                                base, base_sources, proposal)
    return dict(complete=audit_input['capacity_complete'], input=audit_input['capacity_input'],
                sources=audit_input['capacity_sources'],
                final_build=audit_input['capacity_final_build'],
                source_lineage=value['source_lineage'], diagnostic_build=False,
                admission_changed=True, structural_capacity_expansion=True,
                cfg_diagnostics_retained=True, capacity_limits=value['capacity_limits'],
                capacity_cohorts=value['capacity_cohorts'], **proof)


def main():
    started = time.monotonic()
    hard_deadline = started + WHOLE_SECONDS
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch(r'[0-9a-f]{64}', sys.argv[1]),
            'python3 -B audit_guarded_mlp_ranked_cfg_cap_compiler_tools_v228_v2.py ACTUAL_INPUT_SHA')
    require(all(type(value) is str and re.fullmatch(r'[0-9a-f]{64}', value)
                for value in (CAPACITY_CONTROLLER_SHA, CAPACITY_PROPOSAL_SHA, CAPACITY_COMPLETE_SHA)),
            'actual capacity producer bindings remain pending')
    require(type(DRIVER_CONTROLLER_SHA) is str and re.fullmatch(r'[0-9a-f]{64}', DRIVER_CONTROLLER_SHA),
            'reviewed driver producer controller binding is pending')
    require(Path(__file__).resolve() == SCRIPT and os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'unexpected controller host/path/UID')
    require(E.resolve(strict=True) == E and TOOLS.resolve(strict=True) == TOOLS
            and not os.path.lexists(OUT), 'canonical inputs and fresh output required')
    require(shutil.disk_usage(E).free >= FREE_FLOOR, '40 GiB setup storage floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    priority = os.getpriority(os.PRIO_PROCESS, 0)
    require(priority in (0, 10), 'unexpected nice value')
    if priority == 0:
        os.nice(10)
    require(os.sched_getaffinity(0) == {8, 9}
            and os.getpriority(os.PRIO_PROCESS, 0) == 10, 'CPU affinity/nice mismatch')
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, AS_LIMIT),
                      (resource.RLIMIT_FSIZE, STREAM_LIMIT)):
        soft, hard = resource.getrlimit(kind)
        effective = min([cap] + [x for x in (soft, hard) if x != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (effective, effective))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper setup')
    OUT.mkdir(mode=0o700)
    for sig in SIGNALS:
        signal.signal(sig, interrupted)
    signal.setitimer(signal.ITIMER_REAL, WHOLE_SECONDS - CLEANUP_RESERVE)
    env = dict(HOME='/home/harmenon', PATH='/usr/bin:/bin', LANG='C', LC_ALL='C', TZ='UTC',
               LD_LIBRARY_PATH=str(TOOLS / 'bin') + ':' + str(NIGHTLY_LIB), ROCR_VISIBLE_DEVICES='',
               HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
    leaves, audits, manifest, tool_rows, audit_tools = [], {}, None, {}, {}
    failure, postcheck_errors = None, []
    audit_input_pin, manifest_pin, producer, driver, previous_pin = None, None, None, None, None
    baseline_producer, driver_manifest_pin = None, None
    try:
        script_pin = pin(SCRIPT)
        audit_input_pin = pin(AUDIT_INPUT, cap=64 << 10)
        audit_input = read_json(audit_input_pin, sys.argv[1])
        require(set(audit_input) == {'schema', 'controller', 'tool_manifest', 'old_tool_manifest',
                    'producer_complete', 'producer_input', 'producer_sources', 'producer_final_build',
                    'previous_tool_manifest', 'driver_complete', 'driver_input', 'driver_sources',
                    'driver_final_build', 'driver_test_build', 'driver_tool_manifest',
                    'capacity_complete', 'capacity_input', 'capacity_sources',
                    'capacity_final_build'}
                and audit_input['schema'] == 'ferric-guarded-mlp-ranked-cfg-cap-tool-audit-input-v1'
                and audit_input['controller'] == script_pin,
                'closed caller-pinned audit admission')
        audit_tools = {name: resolved('/usr/bin/' + name) for name in ('readelf', 'ldd', 'prlimit')}
        audit_tools['bash'] = resolved('/bin/bash')
        require(NIGHTLY_LIB.resolve(strict=True) == NIGHTLY_LIB, 'canonical nightly lib directory')
        for name, (size, sha) in NIGHTLY.items():
            value = pin(NIGHTLY_LIB / name)
            require((value['bytes'], value['sha256']) == (size, sha), 'nightly compiler library identity')
        manifest_pin = pin(TOOLS / 'manifest.json', cap=64 << 10)
        require(manifest_pin == audit_input['tool_manifest'], 'actual new seven-tool manifest identity')
        manifest = read_json(manifest_pin)
        previous_pin = audit_input['previous_tool_manifest']
        previous = read_json(previous_pin, PREVIOUS_MANIFEST_SHA)
        require(audit_input['producer_complete']['sha256'] == BASE_COMPLETE_SHA
                and audit_input['driver_complete']['sha256'] == DRIVER_COMPLETE_SHA,
                'exact actual baseline and unchanged focused driver receipts')
        baseline_producer = qualified_producer(audit_input, previous)
        driver_manifest_pin = audit_input['driver_tool_manifest']
        driver_manifest = read_json(driver_manifest_pin, DRIVER_MANIFEST_SHA)
        driver = qualified_driver(audit_input, driver_manifest, previous)
        producer = qualified_capacity_producer(audit_input, manifest, driver_manifest)
        require(set(manifest) == {'bin/' + name for name in NAMES}, 'exact seven-tool roster')
        require({p.name for p in TOOLS.iterdir()} == {'bin', 'manifest.json'}
                and {p.name for p in (TOOLS / 'bin').iterdir()} == set(NAMES), 'closed transported tree')
        for name in NAMES:
            original = manifest['bin/' + name]
            require(set(original) == {'source', 'bytes', 'sha256'}
                    and Path(original['source']).is_absolute(), 'original tool record shape')
            path = TOOLS / 'bin' / name
            value = pin(path)
            require((value['bytes'], value['sha256']) == (original['bytes'], original['sha256']),
                    'transported tool identity differs')
            require(name == 'librustc_codegen_fe2o3.so' or os.access(path, os.X_OK), 'tool executable mode')
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'tool is not an ELF object')
            tool_rows[name] = dict(original=original, deployed=value)
        save('inputs-before.json', INPUTS)
        for name in NAMES:
            path = str(TOOLS / 'bin' / name)
            dynamic = owned(name + '-readelf', ['/usr/bin/readelf', '-l', '-d', path], env, leaves, hard_deadline)
            linkage = owned(name + '-ldd', ['/usr/bin/ldd', path], env, leaves, hard_deadline)
            audits[name] = libraries(dynamic, linkage, name == 'librustc_codegen_fe2o3.so')
            if name == 'fe2o3-rustc-extract':
                require(audits[name]['libraries']['librustc_codegen_fe2o3.so']['pin']
                        == tool_rows['librustc_codegen_fe2o3.so']['deployed'],
                        'extractor must resolve the exact deployed backend')
        require(len(leaves) == 14 and set(audits) == set(NAMES), 'fourteen successful audit leaves required')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    for name, expected in list(INPUTS.items()):
        if time.monotonic() >= hard_deadline:
            postcheck_errors.append('whole deadline reached before completing input postchecks')
            break
        try:
            signal.setitimer(signal.ITIMER_REAL, max(0.001, hard_deadline - time.monotonic()))
            require(pin(Path(name), track=False) == expected, 'input/provider posthash drift')
        except BaseException as error:
            postcheck_errors.append(name + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    for name, expected in ALIASES.items():
        try:
            require(str(Path(name).resolve(strict=True)) == expected, 'reported alias changed')
        except BaseException as error:
            postcheck_errors.append(name + ': ' + repr(error))
    try:
        require({p.name for p in TOOLS.iterdir()} == {'bin', 'manifest.json'}
                and {p.name for p in (TOOLS / 'bin').iterdir()} == set(NAMES),
                'transported tree roster changed')
    except BaseException as error:
        postcheck_errors.append('tool roster: ' + repr(error))
    if postcheck_errors:
        failure = failure or 'input/provider postcheck failed'
    if time.monotonic() >= hard_deadline:
        failure = failure or 'whole audit deadline exceeded'
    if shutil.disk_usage(E).free < FREE_FLOOR:
        failure = failure or '40 GiB final storage floor'
    save('inputs-after.json', dict(inputs=INPUTS, resolved_paths=ALIASES, errors=postcheck_errors))
    result = dict(schema='ferric-guarded-mlp-ranked-cfg-cap-compiler-tool-audit-v1', passed=failure is None,
                  failure=failure, postcheck_errors=postcheck_errors, tool_manifest=manifest_pin,
                  input_manifest=audit_input_pin, qualified_producer=producer,
                  previous_tool_manifest=previous_pin, qualified_driver=driver,
                  driver_tool_manifest=driver_manifest_pin,
                  qualified_baseline_producer=baseline_producer,
                  admission_changed=True, structural_capacity_expansion=True,
                  cfg_diagnostics_retained=True, capacity_limits=CAPACITY_LIMITS,
                  driver_receipt_source_and_final_cargo_joins_replayed=driver is not None,
                  producer_receipt_source_and_final_cargo_joins_replayed=producer is not None,
                  rlib_deployed=False, rlib_readelf_or_ldd_invoked=False,
                  tools=tool_rows, audit_tools=audit_tools, phases=leaves, audits=audits,
                  inputs=INPUTS, resolved_paths=ALIASES, environment=env,
                  host=os.uname().nodename, boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                  elapsed_seconds=time.monotonic() - started,
                  limits=dict(whole_wall_seconds=WHOLE_SECONDS, cleanup_reserve_seconds=CLEANUP_RESERVE,
                              leaf_wall_seconds=30, leaf_cpu_seconds=30, address_space_bytes=AS_LIMIT,
                              stream_bytes=STREAM_LIMIT, free_bytes=FREE_FLOOR, affinity=[8, 9], nice=10),
                  raw={p.name: pin(p, False) for p in sorted(OUT.iterdir()) if p.is_file()},
                  actual_library_audits_replayed=failure is None, tool_execution_limited_to_readelf_ldd=True,
                  compiler_invocation=False, vendor_created=False, gpu_execution=False,
                  production_authority=False, load_authority=False, launch_authority=False)
    receipt = save('complete.json' if failure is None else 'failed.json', result)
    print(json.dumps(receipt), flush=True)
    if failure is not None:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
