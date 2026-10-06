"""Full KFD CPU qualification for the additive private combined-state owner."""

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
EXPECTED_ROOT = E / 'guarded-mlp-combined-state-owner-cpu-v228-v1'
SOURCE = ROOT / 'fe2o3'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
GENERATION = '5a500d63c29b78f8788356b20dfcbb5c41ec20c9'
BASE = E / 'peer-dependency-signal-completion-cpu-v228-v1'
RT_INPUT = E / 'guarded-mlp-segment-cpu-v228-v5/input-manifest.json'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
PROPOSAL_SHA = '323ef39a6c6ec1d9c91c179a3e28e6d48bc5f56e99e786df52ff30b68f8dd311'
LINEAGE = {
    'base_complete': (BASE / 'evidence/complete.json', 234095,
                      'b4d6edccd6232fc925305d8053b844c3bf072a3f2aade9a774f44d3e67db059f'),
    'base_sources': (BASE / 'evidence/sources-tested.json', 307490,
                     'e307d81cac3a96e7006a5aa1ffe0b62e2aab746ca5ebb735f3e7c5ecc8d623e1'),
    'base_stdout': (BASE / 'evidence/kfd-tests.stdout', 110290,
                    'e0d3b14fb03fe4aa56d7ffeb2fadbb4c39583befe4fe56364b17eff378837899'),
    'base_controller': (BASE / 'run_cpu.py', 27116,
                        'd66b641752986b09952dc73b28ee7984796359e39d1373046dfa2229ffa68406'),
    'rt_input': (RT_INPUT, 1012525,
                 '9c3cf80f76658c778bc708997858860f8089355a693fab7fad3a4ad706aa1c87'),
    'owner_proposal': (ROOT / 'inputs/owner-source.json', 6686, PROPOSAL_SHA),
}
CRATES = ('fe2o3-amd-target', 'fe2o3-amdhsa-loader', 'fe2o3-aql', 'fe2o3-drm-uapi',
          'fe2o3-hsaco', 'fe2o3-kfd', 'fe2o3-kfd-uapi', 'fe2o3-runtime-model', 'fe2o3-target-spec')
OWNER_PREFIX = 'engineering_gfx950::peer::combined_mlp_state_v1::tests::'
MEMORY_PREFIX = 'memory_linux::combined_mlp_state_v1::tests::'
COHORTS = {
    'combined-owner-tests': (OWNER_PREFIX, tuple(OWNER_PREFIX + name for name in (
        'combined_binding_matrix_admits_only_exact_owner_and_suffix_regions',
        'combined_borrowed_regions_preserve_one_real_allocation_identity',
        'combined_cleanup_selects_queue_first_and_keeps_existing_kinds_unchanged',
        'combined_generation_and_activation_rules_refuse_replay_skip_and_overflow',
        'combined_native_submit_refuses_bad_phase_before_any_context_operation',
        'combined_owner_does_not_change_legacy_token_or_public_byte_policy',
        'combined_snapshot_predicates_check_every_prefix_word_and_guard_component',
        'combined_terminal_failures_poison_without_publishing_generation_or_losing_custody',
    ))),
    'combined-memory-tests': (MEMORY_PREFIX, tuple(MEMORY_PREFIX + name for name in (
        'combined_memory_bad_mapping_and_alignment_refuse_before_any_atomic_access',
        'combined_memory_cannot_be_observed_or_rearmed_as_legacy_2192_owner',
        'combined_memory_constructs_exact_552_genuine_atomics_and_preserves_tail',
        'combined_memory_quiescent_rearm_preserves_objects_and_replaces_every_word',
        'combined_memory_wrong_request_refuses_before_construction_or_stores',
        'combined_memory_zero_generation_does_not_publish_or_mutate',
    ))),
}
IGNORED = tuple(sorted((
    'queue::dispatch_binding::tests::real_gfx950_kernel_rejects_before_fixed_dispatch_data_preparation',
    'shared_memory::gfx950_observed::queue::finite_join::tests::retained_fixed_image_passes_same_engine_intake',
    'shared_memory::gfx950_observed::queue::multiwave_join::tests::actual_multiwave_image_passes_distinct_same_engine_intake',
)))
OVERLAY = tuple('crates/fe2o3-kfd/src/' + name for name in (
    'engineering_gfx950_peer.rs', 'engineering_gfx950_peer_combined_mlp_state_v1.rs',
    'engineering_gfx950_peer_combined_mlp_state_v1_tests.rs', 'engineering_gfx950_peer_tests.rs',
    'memory_linux.rs', 'memory_linux_combined_mlp_state_v1.rs',
    'memory_linux_combined_mlp_state_v1_tests.rs',
))
TARGETS = {
    'kfd-lib': ('fe2o3_kfd', 'lib', 'src/lib.rs'),
    'engineering-worker-test': ('fe2o3-gfx950-engineering-worker', 'bin', 'src/bin/gfx950_engineering_worker.rs'),
    'debug-trap-test': ('kfd_debug_trap_live', 'test', 'tests/kfd_debug_trap_live.rs'),
    'telemetry-env-test': ('target_debug_telemetry_env_v1', 'test', 'tests/target_debug_telemetry_env_v1.rs'),
    'telemetry-test': ('target_debug_telemetry_v1', 'test', 'tests/target_debug_telemetry_v1.rs'),
}
WHOLE_WALL, LEAF_WALL, CLEANUP_RESERVE = 3600, 1800, 50
PHASES = ('rustc-version', 'metadata', 'default-check', 'kfd-tests-build', 'kfd-list',
          'kfd-ignored', 'kfd-tests', 'combined-owner-tests', 'combined-memory-tests')


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


def normalized_libtest(text):
    names = (
        'queue_linux::tests::payload_release_failure_after_event_destroy_is_process_terminal',
        'queue_linux::tests::unpublished_custody_cleanup_failure_is_process_terminal',
    )
    for name in names:
        block = 'test ' + name + ' ... \nrunning 1 test\nok'
        text, count = re.subn('^' + re.escape(block) + r'(?=\n|\Z)',
                              'test ' + name + ' ... ok', text, flags=re.M)
        require(count <= 1, 'duplicate known abort-child parent block: ' + name)
    return text


def test_outcomes(path):
    text = normalized_libtest(path.read_text())
    summaries = [dict(status=status, passed=int(passed), failed=int(failed),
                      ignored=int(ignored), measured=int(measured), filtered_out=int(filtered))
                 for status, passed, failed, ignored, measured, filtered in re.findall(
                     r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; '
                     r'(\d+) measured; (\d+) filtered out;', text, re.M)]
    named = [dict(name=name, outcome=outcome) for name, outcome in re.findall(
        r'^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, [^\n]*)?$', text, re.M)]
    require(summaries, 'no actual libtest summaries: ' + str(path))
    for status, field in (('ok', 'passed'), ('FAILED', 'failed'), ('ignored', 'ignored')):
        require(sum(row['outcome'] == status for row in named)
                == sum(row[field] for row in summaries), 'named libtest census differs from summaries')
    return dict(summaries=summaries, named=named,
                passed=sum(row['passed'] for row in summaries),
                failed=sum(row['failed'] for row in summaries),
                ignored=sum(row['ignored'] for row in summaries))


def named_statuses(value):
    rows = value['named']
    result = {row['name']: row['outcome'] for row in rows}
    require(len(result) == len(rows) and all(re.fullmatch(r'[A-Za-z0-9_:]+', name)
            for name in result), 'closed unique test names required')
    return result


def full_outcomes(path, expected, old_summaries):
    value = test_outcomes(path)
    summaries = [dict(row) for row in old_summaries]
    summaries[0]['passed'] += 14
    require(value['summaries'] == summaries and named_statuses(value) == expected
            and (value['passed'], value['failed'], value['ignored']) == (1008, 0, 3),
            'full KFD five-target outcomes differ from exact old-plus-fourteen roster')
    return value


def lineage_contract(h, inputs, before, readset):
    lineage = inputs['source_lineage']
    require(set(lineage) == set(LINEAGE) | {'owner_overlay'}, 'closed source lineage')
    bodies = {}
    for name, (path, size, digest) in LINEAGE.items():
        require(path.resolve(strict=True) == path and size <= 16 << 20, 'lineage path/size')
        row = h.pin(path)
        require(row == lineage[name] and row['bytes'] == size and row['sha256'] == digest,
                'literal source lineage differs: ' + name)
        readset[name] = row
        bodies[name] = path.read_bytes()
        require(h.pin(path) == row, 'lineage changed while reading')
    base = json.loads(bodies['base_complete'])
    base_sources = json.loads(bodies['base_sources'])
    rt = json.loads(bodies['rt_input'])
    proposal = json.loads(bodies['owner_proposal'])
    require(base['schema'] == 'ferric-peer-dependency-signal-completion-cpu-v1'
            and base['passed'] is True and base['failure'] is None and base['postcheck_errors'] == []
            and base['source_unchanged'] is True and base['gpu_execution'] is False
            and base['native_probe_invoked'] is False
            and base['tested_sources'] == base['raw']['sources-tested.json'] == readset['base_sources']
            and base['controller'] == readset['base_controller']
            and base['raw']['kfd-tests.stdout'] == readset['base_stdout'], 'qualified KFD baseline')
    require(len(base['phases']) == 13 and all(row['exit_code'] == 0 and row['natural_exit'] is True
            and row['reaped'] is True and row['process_group_absent'] is True
            and row['forced_cleanup'] is False and row['timed_out'] is False
            and row['exception'] is None for row in base['phases']), 'baseline natural lifecycle')
    require(len(base_sources) == 785 and base_sources['run_cpu.py'] == readset['base_controller']
            and all(row['path'] == str(BASE / name) and Path(name).as_posix() == name
                    and not Path(name).is_absolute() and '..' not in Path(name).parts
                    for name, row in base_sources.items()), 'baseline closed source roster')
    require(rt['dependency_commit'] == GENERATION and len(rt['files']) == 5307,
            'exact RT5a full source input')
    crates = {name: row for name, row in base_sources.items() if name.startswith('crates/')}
    require(len(crates) == 779 and {Path(name).parts[1] for name in crates} == set(CRATES)
            and sum(name.startswith('crates/fe2o3-kfd/') for name in crates) == 157
            and compact(crates) == {name: rt['files']['fe2o3/' + name] for name in crates},
            'all nine-crate and KFD baseline bodies must equal RT5a')
    for old, current in (('Cargo.toml.original', 'Cargo.toml'), ('Cargo.lock.input', 'Cargo.lock')):
        require(compact({old: base_sources[old]})[old] == rt['files']['fe2o3/' + current],
                'thin workspace retained full RT5a manifest/lock differs')
    require(set(base_sources) - set(crates) == {'Cargo.toml', 'Cargo.toml.original', 'Cargo.lock',
            'Cargo.lock.input', 'rust-toolchain.toml', 'run_cpu.py'}, 'thin workspace top-level roster')
    require(proposal['schema'] == 'ferric-guarded-mlp-combined-state-owner-source-v1'
            and proposal['base_dependency_commit'] == GENERATION
            and proposal['base_input'] == compact({'rt': readset['rt_input']})['rt']
            and proposal['requested_bytes'] == 2208 and proposal['prefix_bytes'] == 2192
            and proposal['guard_offset_bytes'] == 2192 and proposal['guard_bytes'] == 16
            and proposal['expected_new_focused_tests'] == 14
            and proposal['additive_private_owner'] is True
            and all(proposal[name] is False for name in ('legacy_state_v2_changed',
                    'legacy_profiles_changed', 'compiler_admission_changed', 'public_api_added',
                    'coordinator_implemented', 'worker_integrated')), 'owner authority/scope differs')
    require(proposal['test_cohorts'] == {prefix: {'filter': prefix, 'names': sorted(names)}
            for prefix, names in COHORTS.values()}, 'exact owner source test cohorts')
    require(set(proposal['files']) == set(OVERLAY) == set(lineage['owner_overlay'])
            and sum(row['before'] is None for row in proposal['files'].values()) == 4,
            'closed seven-file, four-addition owner overlay')
    expected = {'fe2o3/' + name: row for name, row in compact(base_sources).items()
                if name != 'run_cpu.py'}
    for name, row in proposal['files'].items():
        require(expected.get('fe2o3/' + name) == row['before'], 'owner preimage differs: ' + name)
        expected['fe2o3/' + name] = row['after']
        actual = h.pin(SOURCE / name)
        require(actual == lineage['owner_overlay'][name] == before['fe2o3/' + name]
                and compact({name: actual})[name] == row['after'], 'owner postimage differs: ' + name)
        readset['owner_overlay/' + name] = actual
    for name, row in proposal['unchanged_dependencies'].items():
        require(expected['fe2o3/' + name] == rt['files']['fe2o3/' + name] == row,
                'unchanged legacy owner/profile dependency differs: ' + name)
    require(len(expected) == 788 and compact({name: row for name, row in before.items()
            if name.startswith('fe2o3/')}) == expected, 'fresh closed owner source map differs')
    old_tests = test_outcomes(Path(readset['base_stdout']['path']))
    require(old_tests == base['tests']['kfd-tests'] and len(old_tests['summaries']) == 5
            and (old_tests['passed'], old_tests['failed'], old_tests['ignored']) == (994, 0, 3)
            and [(row['passed'], row['ignored'], row['filtered_out'], row['measured'])
                 for row in old_tests['summaries']] == [(974, 3, 0, 0), (0, 0, 0, 0),
                        (0, 0, 0, 0), (9, 0, 0, 0), (11, 0, 0, 0)], 'exact baseline raw KFD census')
    names = named_statuses(old_tests)
    require(len(names) == 997 and sorted(name for name, status in names.items() if status == 'ignored')
            == list(IGNORED) and all(status in ('ok', 'ignored') for status in names.values()),
            'exact baseline old names/ignored roster')
    for _, selected in COHORTS.values():
        require(not set(selected) & set(names), 'new owner names collide with old roster')
        names.update({name: 'ok' for name in selected})
    return base, old_tests, names


def selected_tests(h, path):
    rows = h.build_records(path)
    test_rows = [row for row in rows if row.get('reason') == 'compiler-artifact'
                 and row.get('manifest_path') == str(SOURCE / 'crates/fe2o3-kfd/Cargo.toml')
                 and row.get('profile', {}).get('test') is True]
    require(len(test_rows) == 5, 'exact five KFD test artifacts required')
    artifacts = {}
    for role, (name, kind, source) in TARGETS.items():
        artifact = h.select_artifact(rows, 'fe2o3-kfd', name, kind, True)
        row = artifact['cargo_artifact']
        require(row['target']['kind'] == [kind]
                and row['target']['src_path'] == str(SOURCE / 'crates/fe2o3-kfd' / source)
                and row['filenames'].count(artifact['pin']['path']) == 1
                and set(row['features']) == {'default', 'engineering-gfx950'},
                'closed selected KFD test target/features')
        artifacts[role] = artifact
    require(len({row['pin']['path'] for row in artifacts.values()}) == 5, 'distinct selected test ELFs')
    return artifacts


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
    h = load_exact('owner_owned_supervisor', ROOT / 'supervisor.py', SUPERVISOR_SHA)
    h.ROOT, h.SOURCE, h.OUT, h.TARGET, h.TMP = ROOT, SOURCE, OUT, TARGET, TMP
    h.ROLES = {role: ('fe2o3-kfd', name, kind) for role, (name, kind, _) in TARGETS.items()}
    original_sources = h.sources
    def sources():
        return dict(original_sources(), **{'supervisor.py': h.pin(ROOT / 'supervisor.py')})
    h.sources = sources
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - CLEANUP_RESERVE - time.monotonic()))
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    before = final = input_pin = config = old_tests = None
    readset, tool_pins, external, external_after, local, artifacts, tests = {}, {}, {}, {}, {}, {}, {}
    inventory, ignored, phases, errors, failure = [], [], [], [], None
    try:
        input_pin = h.pin(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'literal input mismatch')
        inputs = json.loads((ROOT / 'input-manifest.json').read_bytes())
        require(set(inputs) == {'schema', 'source_generation', 'files', 'tool_pins', 'source_lineage'}
                and inputs['schema'] == 'ferric-guarded-mlp-combined-state-owner-cpu-input-v1'
                and inputs['source_generation'] == GENERATION, 'input generation/fields')
        before = sources()
        require(len(before) == 790 and inputs['files'] == compact(before), 'closed exact source map')
        h.save('sources-before.json', before)
        base, old_tests, expected_names = lineage_contract(h, inputs, before, readset)
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
                   CARGO_PROFILE_DEV_OPT_LEVEL='2', CARGO_PROFILE_DEV_DEBUG='0',
                   CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true', CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',
                   CARGO_PROFILE_TEST_OPT_LEVEL='2', CARGO_PROFILE_TEST_DEBUG='0',
                   CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true', CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',
                   ROCR_VISIBLE_DEVICES='', HIP_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
        def leaf(label, argv, seconds=LEAF_WALL):
            h.run(label, argv, env, phases, deadline, before, seconds, SOURCE)
        cargo = str(h.TOOLCHAIN / 'cargo')
        common = ['--offline', '--locked', '--jobs', '2', '-p', 'fe2o3-kfd']
        selected = [*common, '--features', 'engineering-gfx950', '--lib', '--tests']
        leaf('rustc-version', [str(h.TOOLCHAIN / 'rustc'), '--version', '--verbose'], 60)
        leaf('metadata', [cargo, 'metadata', '--offline', '--locked', '--format-version', '1'], 120)
        external, local = h.metadata_contract()
        require(set(local) == set(CRATES), 'exact nine-crate metadata closure')
        h.save('dependencies-before.json', external)
        leaf('default-check', [cargo, 'check', *common, '--no-default-features'])
        leaf('kfd-tests-build', [cargo, 'test', *selected, '--no-run', '--message-format=json'])
        artifacts = selected_tests(h, OUT / 'kfd-tests-build.stdout')
        leaf('kfd-list', [cargo, 'test', *selected, '--', '--list', '--format=terse'], 120)
        leaf('kfd-ignored', [cargo, 'test', *selected, '--', '--ignored', '--list', '--format=terse'], 120)
        inventory = h.inventory(OUT / 'kfd-list.stdout')
        ignored = h.inventory(OUT / 'kfd-ignored.stdout')
        require(inventory == sorted(expected_names) and len(inventory) == 1011
                and ignored == list(IGNORED), 'compiled full/ignored KFD inventory differs')
        for prefix, names in COHORTS.values():
            require(sorted(name for name in inventory if name.startswith(prefix)) == sorted(names)
                    and not set(names) & set(ignored), 'compiled focused inventory differs')
        leaf('kfd-tests', [cargo, 'test', *selected, '--', '--test-threads=1'])
        tests['kfd-tests'] = full_outcomes(OUT / 'kfd-tests.stdout', expected_names, old_tests['summaries'])
        for label, (prefix, names) in COHORTS.items():
            leaf(label, [cargo, 'test', *common, '--features', 'engineering-gfx950',
                         '--lib', prefix, '--', '--test-threads=1'])
            tests[label] = h.outcomes(OUT / (label + '.stdout'), names, 991)
        require([row['label'] for row in phases] == list(PHASES), 'closed nine phases')
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
        require(before is not None and final == before, 'source/lock mutation')
    check('sources', source_check)
    if input_pin is not None:
        check('input', lambda: require(h.pin(ROOT / 'input-manifest.json') == input_pin, 'input drift'))
    if config is not None:
        check('configuration', lambda: require(h.configurations() == config, 'configuration drift'))
    for label, expected in dict(tool_pins, **readset).items():
        check('readset ' + label, lambda expected=expected: require(
            h.pin(Path(expected['path'])) == expected, 'authenticated input/tool drift'))
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
            h.pin(Path(artifact['pin']['path'])) == artifact['pin'], 'selected test artifact drift'))
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
    result = dict(schema='ferric-guarded-mlp-combined-state-owner-cpu-v1', passed=failure is None,
                  failure=failure, postcheck_errors=errors, source_generation=GENERATION,
                  controller=h.pin(ROOT / 'run_cpu.py'), supervisor=h.pin(ROOT / 'supervisor.py'),
                  input_manifest=input_pin, source_lineage=inputs['source_lineage'] if input_pin and before else None,
                  readset=readset, baseline_tests=old_tests,
                  input_sources=before, final_sources=final, source_unchanged=before is not None and final == before,
                  phases=phases, artifacts=artifacts, tests=tests, inventory=inventory, ignored=ignored,
                  owner_cohorts={label: {'filter': prefix, 'names': sorted(names)}
                                for label, (prefix, names) in COHORTS.items()},
                  full_kfd_tests_executed='kfd-tests' in tests,
                  tool_pins=tool_pins, raw=raw, local_dependencies=local, configurations=config,
                  elapsed_seconds=time.monotonic() - started, test_artifact_phase='kfd-tests-build',
                  limits=dict(whole_seconds=WHOLE_WALL, leaf_seconds=LEAF_WALL,
                              cleanup_reserve_seconds=CLEANUP_RESERVE, cpu_seconds=h.CPU_LIMIT,
                              address_space_bytes=h.AS_LIMIT, file_bytes=h.FILE_LIMIT,
                              cache_bytes=h.CACHE_LIMIT, stream_bytes=h.STREAM_LIMIT,
                              initial_free_bytes=h.START_FREE, live_free_bytes=h.LIVE_FREE,
                              affinity=[8, 9], nice=10, cargo_jobs=2),
                  additive_private_owner=True, legacy_state_v2_changed=False, legacy_profiles_changed=False,
                  gpu_execution=False, worker_integrated=False, coordinator_implemented=False,
                  native_peer_ordering_qualified=False, production_authority=False, performance_claim=False,
                  doctests_executed=False, live_validation_enabled=False)
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
