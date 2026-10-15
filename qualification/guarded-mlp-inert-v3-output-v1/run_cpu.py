"""Qualify exact selected-input V3 transport; no worker compilation or GPU calls."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import sys
import time
import types

ROOT = Path(__file__).resolve().parent
E = ROOT.parent
SOURCE = ROOT / 'fe2o3'
OUT, TARGET, TMP = (ROOT / name for name in ('evidence', 'target', 'tmp'))
Q = E / 'guarded-mlp-finite-predicate-inline-cpu-v228-v3'
F = E / 'guarded-mlp-semantic-capture-driver-cpu-v228-v1'
B = E / 'guarded-mlp-inert-v3-invocation-cpu-v228-v1'
S = E / 'guarded-mlp-inert-v3-selector-cpu-v228-v1'
RECOVERY = E / 'guarded-mlp-inert-v3-selector-recovery-cpu-v228-v1'
C = E / 'guarded-mlp-inert-v3-consumer-cpu-v228-v1'
FIRST = E / 'guarded-mlp-inert-v3-output-cpu-v228-v1'
UNIT = 'ferric-inert-v3-output-cpu-v228-v2.service'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
GATES = (
    'production_pipeline::conditional_formal_tests::conditional_formal_pipeline_keeps_v2_and_protected_handoffs_closed',
    'production_pipeline::tests::extraction_custody_cannot_enter_protected_publication',
)

def require(value, message):
    if not value:
        raise RuntimeError(message)

def compact(rows):
    return {name: {key: row[key] for key in ('bytes', 'sha256')} for name, row in rows.items()}

def main():
    start = time.monotonic()
    deadline = start + 1800
    require(ROOT.name == 'guarded-mlp-inert-v3-output-cpu-v228-v2'
            and ROOT.resolve(strict=True) == ROOT and len(sys.argv) == 2
            and __debug__ and sys.dont_write_bytecode, 'exact invocation/root')
    require(not any(os.path.lexists(p) for p in (OUT, TARGET, TMP)), 'fresh outputs')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/uid')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, 'initial disk floor')
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    require(os.getpriority(os.PRIO_PROCESS, 0) == 10, 'nice10')
    cg = Path('/proc/self/cgroup').read_text().strip().split(':', 2)[2]
    scope = Path('/sys/fs/cgroup') / cg.lstrip('/')
    require(scope.name == UNIT and (scope / 'memory.max').read_text().strip() == str(12 << 30)
            and (scope / 'memory.swap.max').read_text().strip() == '0', 'bounded service')
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30)):
        _, hard = resource.getrlimit(kind)
        limit = cap if hard == resource.RLIM_INFINITY else min(cap, hard)
        resource.setrlimit(kind, (limit, limit))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    body = (ROOT / 'supervisor.py').read_bytes()
    require(hashlib.sha256(body).hexdigest() == SUPERVISOR_SHA, 'supervisor pin')
    h = types.ModuleType('owned_supervisor')
    h.__file__ = str(ROOT / 'supervisor.py')
    exec(compile(body, h.__file__, 'exec'), h.__dict__)
    h.ROOT, h.SOURCE, h.OUT, h.TARGET, h.TMP = ROOT, SOURCE, OUT, TARGET, TMP
    h.ROLES = {'invocation': ('fe2o3-rustc-invocation', 'fe2o3_rustc_invocation', 'lib'),
               'driver': ('cargo-fe2o3', 'cargo-fe2o3', 'bin')}
    def sources():
        paths = [ROOT / name for name in ('run_cpu.py', 'supervisor.py', 'prepare_inputs.py')]
        paths += h.files_below(SOURCE)
        return {str(p.relative_to(ROOT)): h.pin(p) for p in sorted(paths)}
    h.sources = sources
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    OUT.mkdir(mode=0o700)
    TMP.mkdir(mode=0o700)
    phases, artifacts, tests, inventories, ignored, errors = [], {}, {}, {}, {}, []
    before = after = config = None
    external, tool_pins, final_external, parents, source_deltas = {}, {}, {}, {}, {}
    failure = None
    try:
        manifest_pin = h.pin(ROOT / 'input-manifest.json')
        require(manifest_pin['sha256'] == sys.argv[1], 'input manifest pin')
        manifest = json.loads((ROOT / 'input-manifest.json').read_bytes())
        before = sources()
        require(compact(before) == manifest['files'], 'exact input roster')
        h.save('sources-before.json', before)
        for label, root in [('backend', Q), ('frontend', F), ('transport', B), ('selector-recovery', RECOVERY), ('consumer', C)]:
            pin = h.pin(root / 'evidence/complete.json')
            require(pin == manifest['parents'][label], 'parent pin')
            value = json.loads((root / 'evidence/complete.json').read_bytes())
            require(value['passed'] and value['source_unchanged'] and not value['postcheck_errors'], 'qualified parent')
            parents[label] = pin
        failed_pin = h.pin(S / 'evidence/failed.json')
        require(failed_pin == manifest['parents']['selector-source-failed'], 'exact preserved selector failure')
        selector_failed = json.loads((S / 'evidence/failed.json').read_bytes())
        recovery = json.loads((RECOVERY / 'evidence/complete.json').read_bytes())
        require(selector_failed['failure'] == "RuntimeError('active cohort')"
                and selector_failed['source_unchanged'] and not selector_failed['postcheck_errors']
                and recovery['first_failed_receipt'] == failed_pin
                and not recovery['rust_build_repeated'] and not recovery['completed_cohort_repeated'],
                'selector failure/recovery exact source lineage')
        parents['selector-source-failed'] = failed_pin
        first_pin = h.pin(FIRST / 'evidence/failed.json')
        require(first_pin == manifest['parents']['output-first-failed'], 'original output failure pin')
        first = json.loads((FIRST / 'evidence/failed.json').read_bytes())
        require(first['source_unchanged'] and not first['postcheck_errors'] and not first['passed']
                and first['failure'] == "RuntimeError('output-tests did not finish naturally/reaped/successfully')",
                'preserved actual same-length-write negative-control failure')
        parents['output-first-failed'] = first_pin
        base = json.loads((C / 'evidence/complete.json').read_bytes())['sources_after']
        old = {k: v for k, v in base.items() if k.startswith('fe2o3/')}
        new = {k: v for k, v in before.items() if k.startswith('fe2o3/')}
        source_deltas = {k: dict(before=compact(old).get(k), after=compact(new).get(k))
                         for k in sorted(set(old) | set(new)) if compact(old).get(k) != compact(new).get(k)}
        require(set(source_deltas) == {
            'fe2o3/crates/cargo-fe2o3/src/protected_compiler_handoff_v3.rs',
            'fe2o3/crates/cargo-fe2o3/src/protected_compiler_handoff_v3/inert_semantic_completion_v1.rs',
            'fe2o3/crates/cargo-fe2o3/src/protected_compiler_handoff_v3/inert_semantic_output_v1.rs',
        }, 'closed three-file source delta')
        for name in ('engineering_hsaco.rs', 'engineering_hsaco/execution.rs', 'engineering_hsaco/support.rs'):
            rel = Path('crates/cargo-fe2o3/src') / name
            require((SOURCE / rel).read_bytes() == (F / 'fe2o3' / rel).read_bytes(), 'preserved qualified frontend')
        config = h.configurations()
        tool_pins = manifest['tool_pins']
        for row in tool_pins.values():
            require(h.pin(Path(row['path'])) == row, 'tool prepin')
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
        def leaf(label, argv, cwd=ROOT, seconds=900):
            localenv = dict(env)
            if cwd != ROOT:
                localenv['CARGO_MANIFEST_DIR'] = str(cwd)
            h.run(label, argv, localenv, phases, deadline, before, seconds, cwd)
        common = ['--offline', '--locked', '--manifest-path', str(SOURCE / 'Cargo.toml')]
        cargo = str(h.TOOLCHAIN / 'cargo')
        leaf('metadata', [cargo, 'metadata', *common, '--format-version=1'], seconds=120)
        external, local = h.metadata_contract()
        h.save('dependencies-before.json', external)
        for label, package, name, kind, flag in (
            ('invocation', 'fe2o3-rustc-invocation', 'fe2o3_rustc_invocation', 'lib', '--lib'),
            ('driver', 'cargo-fe2o3', 'cargo-fe2o3', 'bin', '--bins'),
        ):
            leaf(label + '-build', [cargo, 'test', *common, '--jobs=2', '-p', package, flag, '--no-run', '--message-format=json'])
            artifact = h.select_artifact(h.build_records(OUT / (label + '-build.stdout')), package, name, kind, True)
            artifacts[label] = artifact
            binary = artifact['pin']['path']
            cwd = SOURCE / 'crates' / package
            leaf(label + '-list', [binary, '--list', '--format=terse'], cwd, 120)
            leaf(label + '-ignored', [binary, '--ignored', '--list', '--format=terse'], cwd, 120)
            inventories[label] = h.inventory(OUT / (label + '-list.stdout'))
            ignored[label] = h.inventory(OUT / (label + '-ignored.stdout'))
            if label == 'invocation':
                require(not ignored[label], 'invocation has no ignored tests')
                selected = inventories[label]
                require(len([n for n in selected if n.startswith('selected_transport_v3::tests::')]) == 6, 'six new tests')
                leaf(label + '-tests', [binary, '--test-threads=1'], cwd)
                tests[label] = h.outcomes(OUT / (label + '-tests.stdout'), selected, len(inventories[label]))
            else:
                parent = json.loads((F / 'evidence/complete.json').read_bytes())
                require(ignored[label] == parent['ignored'], 'unchanged driver ignored roster')
                for cohort, prefix in [('engineering', 'engineering_hsaco::tests::'),
                                       ('capture', 'inert_rustc_invocation_capture::tests::'),
                                       ('selector', 'protected_compiler_handoff_v3::inert_semantic_extraction_v1::tests::'),
                                       ('binding-wrapper', 'binding_wrapper::lifecycle_tests::'),
                                       ('consumer', 'protected_compiler_handoff_v3::inert_semantic_completion_v1::tests::'),
                                       ('output', 'protected_compiler_handoff_v3::inert_semantic_output_v1::tests::')]:
                    selected = [n for n in inventories[label] if n.startswith(prefix)]
                    require(selected and not set(selected) & set(ignored[label]), 'active cohort')
                    if cohort == 'engineering':
                        require(selected == parent['engineering_tests'], 'qualified frontend cohort')
                    if cohort == 'selector':
                        require(len(selected) == 10, 'ten explicit selector tests')
                    if cohort == 'consumer':
                        require(len(selected) == 9, 'nine explicit inert consumer tests')
                    if cohort == 'output':
                        require(len(selected) == 8, 'eight explicit pinned output tests')
                    leaf(cohort + '-tests', [binary, prefix, '--test-threads=1'], cwd)
                    tests[cohort] = h.outcomes(OUT / (cohort + '-tests.stdout'), selected, len(inventories[label]))
        # The pre-existing compiler binary is NOT rebuilt from this candidate.
        qreceipt = json.loads((Q / 'evidence/complete.json').read_bytes())
        gate_artifact = qreceipt['artifacts']['test']
        require(h.pin(Path(gate_artifact['pin']['path'])) == gate_artifact['pin'], 'qualified gate artifact')
        artifacts['prior-compiler-gates'] = gate_artifact
        for index, name in enumerate(GATES):
            label = 'prior-gate-' + str(index)
            leaf(label, [gate_artifact['pin']['path'], name, '--exact', '--test-threads=1'], Q / 'fe2o3/crates/rustc-codegen-fe2o3')
            tests[label] = h.outcomes(OUT / (label + '.stdout'), [name], len(qreceipt['inventory']))
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    def check(label, action):
        try:
            signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
            action()
        except BaseException as error:
            errors.append(label + ': ' + repr(error))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    def sourcecheck():
        nonlocal after
        after = sources()
        h.save('sources-after.json', after)
        require(before == after, 'source drift')
    check('sources', sourcecheck)
    if config is not None:
        check('config', lambda: require(h.configurations() == config, 'config drift'))
    for label, row in {**tool_pins, **parents}.items():
        check(label, lambda row=row: require(h.pin(Path(row['path'])) == row, 'pin drift'))
    for label, artifact in artifacts.items():
        check(label, lambda artifact=artifact: require(h.pin(Path(artifact['pin']['path'])) == artifact['pin'], 'artifact drift'))
    for directory, expected in external.items():
        def dependencycheck(directory=directory, expected=expected):
            root = Path(directory)
            actual = {str(p.relative_to(root)): h.pin(p) for p in h.files_below(root, packed=False)}
            final_external[directory] = actual
            require(actual == expected, 'dependency drift')
        check(directory, dependencycheck)
    check('dependencies', lambda: h.save('dependencies-after.json', final_external))
    failure = failure or ('postcheck failed' if errors else None)
    raw = {p.name: h.pin(p) for p in sorted(OUT.iterdir()) if p.is_file()}
    result = dict(schema='ferric-inert-semantic-output-component-cpu-v1', passed=failure is None,
                  failure=failure, postcheck_errors=errors, source_unchanged=before == after,
                  input_manifest=h.pin(ROOT / 'input-manifest.json'), source_delta=source_deltas,
                  sources_before=before, sources_after=after, parents=parents, tool_pins=tool_pins,
                  artifacts=artifacts, inventories=inventories, ignored=ignored, tests=tests,
                  phases=phases, raw=raw, boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                  controller_pid=os.getpid(), controller_starttime_ticks=int(Path('/proc/self/stat').read_text().rsplit(') ', 1)[1].split()[19]),
                  cgroup=cg, affinity=sorted(os.sched_getaffinity(0)), elapsed_seconds=time.monotonic()-start,
                  production_transport_integrated=False, protected_capture_producer_integrated=True,
                  selector_preflight_integrated=True, extraction_only_consumer_qualified=False,
                  inert_content_consumer_implemented=True, live_parent_completion_executed=False,
                  pinned_output_intake_implemented=True,
                  same_length_content_recheck_implemented=True,
                  actual_conditional_formal_archive_replayed=False,
                  engineering_process_schema_compatible=False, worker_compiled=False,
                  gpu_execution=False, production_authority=False)
    h.save('complete.json' if failure is None else 'failed.json', result)
    print(json.dumps(dict(passed=failure is None, failure=failure, errors=errors, phases=len(phases),
                          tests={k: v['passed'] for k,v in tests.items()}), sort_keys=True))
    return 0 if failure is None else 1

if __name__ == '__main__':
    raise SystemExit(main())
