"""Run one exact, existing retained-capture formal test; no worker replay or GPU."""
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
import tarfile
import time
import types

ROOT = Path(__file__).resolve().parent
E = ROOT.parent
CPU = E / 'guarded-mlp-finite-predicate-inline-cpu-v228-v3'
PAIR = E / 'guarded-mlp-early-stop-compile-v228-v18'
ARCHIVE = E / 'guarded-mlp-finite-predicate-inline-publication-v228-v1/originals.tar.gz'
SOURCE = CPU / 'fe2o3'
PACKAGE = SOURCE / 'crates/rustc-codegen-fe2o3'
CPU_SHA = '41d0425beff3ebef1c98910bc904402452c83cddbb8c72229cb9c05e42787a1a'
FIXED_SHA = '65f856d211c78cdbc4322060fbfea200f78299b8504032ff4f55db8c4e897017'
SUPERVISOR_SHA = '8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc'
TEST_SOURCE = PACKAGE / 'src/production_pipeline/wave_mlp_tile_capture_v2_tests.rs'


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def main():
    started = time.monotonic()
    deadline = started + 660
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 2
            and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'python3 -B run.py INPUT_SHA')
    require(ROOT == E / 'guarded-mlp-fixed-formal-replay-cpu-v228-v1'
            and Path(__file__).resolve() == ROOT / 'run.py'
            and ROOT.resolve(strict=True) == ROOT, 'owned root')
    require(os.getuid() == os.geteuid() == 9661
            and os.uname().nodename == 'smci350-rck-g03-b19-03', 'host/UID')
    require(not any((ROOT / name).exists() for name in ('evidence', 'tmp', 'unused-target')),
            'fresh output namespace')
    os.umask(0o077)
    require(os.sched_getaffinity(0) == {8, 9} and os.getpriority(os.PRIO_PROCESS, 0) == 10,
            'CPU8/9 nice10')
    cgroup = Path('/proc/self/cgroup').read_text().strip().split(':', 2)[2]
    scope = Path('/sys/fs/cgroup') / cgroup.lstrip('/')
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    require(boot == '2dbbbc5a-d8b7-46ac-bd1f-236e5847e24a'
            and scope.name == 'ferric-mlp-fixed-formal-replay-cpu-v228-v1.service'
            and (scope / 'memory.max').read_text().strip() == str(12 << 30)
            and (scope / 'memory.swap.max').read_text().strip() == '0', 'bounded reconciled service')
    require(shutil.disk_usage(ROOT).free >= 40 << 30, '40 GiB initial free floor')
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30),
                      (resource.RLIMIT_FSIZE, 1 << 30)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, 'subreaper')
    supervisor_path = CPU / 'supervisor.py'
    raw = supervisor_path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == SUPERVISOR_SHA, 'qualified supervisor source')
    h = types.ModuleType('retained_capture_supervisor')
    h.__file__ = str(supervisor_path)
    exec(compile(raw, str(supervisor_path), 'exec'), h.__dict__)
    h.ROOT, h.SOURCE, h.OUT = ROOT, SOURCE, ROOT / 'evidence'
    h.TMP, h.TARGET = ROOT / 'tmp', ROOT / 'unused-target'
    h.CPU_LIMIT = 600
    h.OUT.mkdir(mode=0o700)
    h.TMP.mkdir(mode=0o700)
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)
    signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - 50 - time.monotonic()))
    failure, errors, phases, tests, stages = None, [], [], {}, []
    before = after = input_pin = config = None
    selected = None
    inventory, ignored = [], []
    lineage_source = None
    try:
        input_pin = h.pin(ROOT / 'input-manifest.json')
        require(input_pin['sha256'] == sys.argv[1], 'literal manifest hash')
        inputs = json.loads((ROOT / 'input-manifest.json').read_bytes())
        require(set(inputs) == {'schema', 'cpu_receipt_sha256', 'fixed_receipt_sha256',
                                'selected_test', 'files'}
                and inputs['schema'] == 'ferric-fixed-formal-replay-input-v1'
                and inputs['cpu_receipt_sha256'] == CPU_SHA
                and inputs['fixed_receipt_sha256'] == FIXED_SHA, 'closed input contract')
        paths = [Path(name) for name in inputs['files']]
        require(5800 < len(paths) < 5900 and all(
                path.resolve(strict=True) == path and inputs['files'][str(path)]['path'] == str(path)
                for path in paths), 'bounded canonical source/input paths')
        def sources():
            return {str(path): h.pin(path) for path in paths}
        h.sources = sources
        before = sources()
        require(before == inputs['files'], 'closed input identities')
        h.save('sources-before.json', before)
        require(before[str(CPU / 'evidence/complete.json')]['sha256'] == CPU_SHA
                and before[str(PAIR / 'fixed-run/evidence/result.json')]['sha256'] == FIXED_SHA,
                'actual original receipts')
        cpu = json.loads((CPU / 'evidence/complete.json').read_bytes())
        require(cpu['passed'] and cpu['source_unchanged'] and cpu['postcheck_errors'] == []
                and len(cpu['phases']) == 19 and len(cpu['ignored']) == 24
                and cpu['tests']['compiler']['passed'] == 1341
                and cpu['tests']['compiler']['failed'] == 0, 'qualified original CPU scope')
        for row in cpu['final_sources'].values():
            require(before[row['path']] == row, 'qualified source pin')
        for row in cpu['tool_pins'].values():
            require(before[row['path']] == row, 'qualified tool/library pin')
        for row in cpu['artifacts'].values():
            require(before[row['pin']['path']] == row['pin'], 'qualified artifact pin')
        project = {str(path) for path in h.files_below(SOURCE)}
        expected_project = {row['path'] for name, row in cpu['final_sources'].items()
                            if name.startswith('fe2o3/')}
        require(project == expected_project and len(project) == 5808, 'actual source roster')
        require(h.pin(CPU / 'evidence/sources-after.json') == cpu['raw']['sources-after.json'],
                'qualified source snapshot')
        names = [name for name in cpu['inventory'] if name.endswith(
            '::actual_mlp_v39_reaches_formal_archive_and_outer_lineage_replay')]
        require(len(names) == 1 and names[0] in cpu['ignored']
                and inputs['selected_test'] == names[0], 'unique explicitly ignored exact test')
        selected = names[0]
        fixed = json.loads((PAIR / 'fixed-run/evidence/result.json').read_bytes())
        require(fixed['compile_accepted'] is False and not fixed['postcheck_errors']
                and len(fixed['phases']) == 1, 'retained actual failed compile')
        phase = fixed['phases'][0]
        require(phase['exit_code'] == 1 and phase['natural_exit'] and phase['reaped']
                and phase['process_group_absent'] and not phase['timed_out']
                and not phase['forced_cleanup'] and phase['exception'] is None
                and phase['storage_failure'] is None, 'clean original compiler terminal')
        for pid in (fixed['execution_scope']['pid'], phase['pid'], phase['pgid']):
            require(not Path('/proc/' + str(pid)).exists(), 'original job retired')
        require(before[str(PAIR / 'fixed-run/evidence/compile.stderr')] == phase['stderr'],
                'original compiler stream')
        capture_dir = PAIR / 'fixed-run/fe2o3-engineering-diagnostics-v1'
        capture = capture_dir / 'semantic-mir-v1.bin'
        for name, row in fixed['diagnostic_files'].items():
            require(before[str(capture_dir / name)] == row, 'actual capture/terminal join')
        require(before[str(capture)]['sha256'] ==
                '7a149f67fd9fa233c6e72f038c6462016475e585bc14172a9b70cc94ed958bed'
                and before[str(capture)]['bytes'] == 957157, 'original fixed capture')
        require(before[str(ARCHIVE)]['sha256'] ==
                '65008a7eb7aeb5435beac87a56a3138568ca6fdba18552a282b91894f6e4c68d',
                'published original archive')
        with tarfile.open(ARCHIVE, mode='r:gz') as archive:
            entry = archive.getmember('originals.json')
            require(entry.isfile() and entry.size <= 1 << 20, 'bounded original ledger')
            ledger = json.loads(archive.extractfile(entry).read())['originals']
        for name in ('sources-before.json', 'sources-after.json'):
            path = PAIR / ('fixed-run/evidence/' + name)
            records = [row for row in ledger.values() if row['path'] == str(path)]
            require(len(records) == 1 and before[str(path)] == records[0],
                    'published original source snapshot pin')
        old_sources = (PAIR / 'fixed-run/evidence/sources-before.json').read_bytes()
        require(old_sources == (PAIR / 'fixed-run/evidence/sources-after.json').read_bytes(),
                'original compile source stability')
        compile_sources = json.loads(old_sources)
        for row in cpu['final_sources'].values():
            require(compile_sources[row['path']] == row, 'original provider/backend source join')
        fixture = list(h.files_below(PAIR / 'fixed'))
        require(len(fixture) == 7 and all(before[str(path)] == compile_sources[str(path)]
                                       for path in fixture), 'exact original caller fixture')
        baseline = next(row for row in cpu['phases'] if row['label'] == 'compiler-tests')
        require(before[str(CPU / 'evidence/compiler-tests.command.json')] == baseline['command'],
                'original qualified test environment')
        command = json.loads((CPU / 'evidence/compiler-tests.command.json').read_bytes())
        require(command['cwd'] == str(PACKAGE), 'original package test cwd')
        env = dict(command['env'], TMPDIR=str(h.TMP), CARGO_TARGET_DIR=str(h.TARGET),
                   FE2O3_MLP_TILE_V2_SEMANTIC_CAPTURE=str(capture),
                   FE2O3_MLP_TILE_V2_SEMANTIC_CAPTURE_SHA256=before[str(capture)]['sha256'])
        require(env['CARGO_MANIFEST_DIR'] == str(PACKAGE)
                and all(env[key] == '' for key in (
                    'ROCR_VISIBLE_DEVICES', 'HIP_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES')),
                'qualified package environment with GPU hidden')
        executable = cpu['artifacts']['test']['pin']['path']
        require(before[executable]['sha256'] ==
                'b6b03b519e08c0ffb842508263fb24aadde2e450bd0d0d93e19171a80928ecf3',
                'qualified compiler test binary')
        config = h.configurations()
        h.run('test-list', [executable, '--list', '--format=terse'],
              env, phases, deadline, before, 30, PACKAGE)
        h.run('test-ignored-list', [executable, '--ignored', '--list', '--format=terse'],
              env, phases, deadline, before, 30, PACKAGE)
        inventory = h.inventory(h.OUT / 'test-list.stdout')
        ignored = h.inventory(h.OUT / 'test-ignored-list.stdout')
        require(inventory == cpu['inventory'] and ignored == cpu['ignored'],
                'exact original compiled inventories and unchanged ignores')
        h.run('fixed-formal-replay',
              [executable, '--ignored', '--exact', selected, '--nocapture', '--test-threads=1'],
              env, phases, deadline, before, 600, PACKAGE)
        tests = h.outcomes(h.OUT / 'fixed-formal-replay.stdout', [selected], len(inventory))
        require([row['label'] for row in phases] ==
                ['test-list', 'test-ignored-list', 'fixed-formal-replay'], 'three closed phases')
        stderr = (h.OUT / 'fixed-formal-replay.stderr').read_text()
        stages = [line for line in stderr.splitlines() if line.startswith('stage=')]
        require('stage=formal status=complete roots=11 unresolved=8' in stages
                and any(line.startswith('stage=outer-lineage-replay status=complete formal_bytes=')
                        for line in stages), 'actual complete formal and outer replay markers')
    except BaseException as error:
        failure = repr(error)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    try:
        signal.setitimer(signal.ITIMER_REAL, max(0.001, deadline - time.monotonic()))
        if before is not None:
            after = {name: h.pin(Path(name)) for name in before}
            h.save('sources-after.json', after)
            require(after == before, 'input/source/tool/artifact mutation')
        if input_pin is not None:
            require(h.pin(ROOT / 'input-manifest.json') == input_pin, 'input manifest mutation')
        if config is not None:
            require(h.configurations() == config, 'configuration mutation')
        if before is not None:
            require({str(path) for path in h.files_below(SOURCE)} == expected_project, 'source roster mutation')
    except BaseException as error:
        errors.append(repr(error))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    replay_stderr = h.OUT / 'fixed-formal-replay.stderr'
    if replay_stderr.exists():
        stages = [line for line in replay_stderr.read_text().splitlines() if line.startswith('stage=')]
    failure = failure or ('postcheck failure' if errors else None)
    raw = {path.name: h.pin(path) for path in sorted(h.OUT.iterdir()) if path.is_file()}
    result = dict(schema='ferric-fixed-formal-replay-cpu-v1', passed=failure is None,
                  failure=failure, postcheck_errors=errors, phases=phases, tests=tests, stages=stages,
                  selected_test=selected, inventory=inventory, default_ignored=ignored,
                  original_cpu_receipt_sha256=CPU_SHA, original_compile_receipt_sha256=FIXED_SHA,
                  input_manifest=input_pin, inputs_before=before, inputs_after=after,
                  source_unchanged=before is not None and after == before,
                  controller=h.pin(ROOT / 'run.py'), raw=raw,
                  execution_scope=dict(boot_id=boot, pid=os.getpid(), cgroup=cgroup, affinity=[8, 9],
                                       start_ticks=Path('/proc/self/stat').read_text().rsplit(')', 1)[1].split()[19],
                                       memory_max=str(12 << 30), memory_swap_max='0'),
                  elapsed_seconds=time.monotonic() - started,
                  production_source_changed=False, worker_compilation_replayed=False,
                  full_compiler_suite_rerun=False, gpu_execution=False, production_authority=False,
                  numerical_acceptance=False, performance_claim=False)
    h.save('complete.json' if failure is None else 'failed.json', result)
    print(json.dumps(dict(passed=failure is None, failure=failure, phases=len(phases),
                          tests=tests, stages=stages), sort_keys=True))
    return 0 if failure is None else 1


if __name__ == '__main__':
    raise SystemExit(main())
