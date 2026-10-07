"""One bounded, task-owned offline container; never a model-serving endpoint."""
import hashlib
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

from common import compact, encoded, load, parse, read, require, save

E = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220')
ROOT = E / 'guarded-mlp-readiness40-position5-reference-v228-v1'
NAME = 'ferric-readiness40-position5-reference-v228-v1'
IMAGE = 'sha256:c5f9efa2623d9c8b2e483d2a139202e496baf5a04900a4f32907149f41a42dba'
MODEL = Path('/home/harmenon/ferric-mi350-model-b968-c189-r1.kUjwIKs0/model/target')
OWNED_SHA = 'ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'
SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGALRM)
CLI_CLEANUP_SECONDS = 130
CPU_ENV = dict(PATH='/usr/bin:/bin', HOME='/home/harmenon', USER='harmenon', LOGNAME='harmenon',
               LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC', PYTHONDONTWRITEBYTECODE='1',
               HIP_VISIBLE_DEVICES='', ROCR_VISIBLE_DEVICES='', CUDA_VISIBLE_DEVICES='')
TEST_NAMES = (
    'test_authentic_prompt_roles_and_full_count',
    'test_prompt_mutation_and_workload_join_refused',
    'test_boolean_ids_and_wrong_model_lineage_refused',
    'test_forty_selected_positions_and_repeat_gate',
    'test_input_history_never_uses_previous_prediction',
    'test_selected_byte_hash_or_role_mismatch_refused',
    'test_capture_missing_duplicate_and_norm_join_refused',
    'test_capture_hooks_removed_even_when_removal_fails',
    'test_nonfinite_shape_and_argmax_refused',
    'test_repeat_payload_logit_and_cache_drift_refused',
    'test_metric_byte_difference_does_not_create_acceptance',
    'test_wrong_selection_and_trailing_bytes_refused',
    'test_container_exact_readonly_mounts_and_no_network',
    'test_container_named_user_offline_and_single_gpu',
    'test_wrong_container_owner_or_image_refused',
    'test_retirement_uses_separate_deadline',
    'test_retirement_ignores_failed_workload_storage',
    'test_retirement_signal_deferred_and_recorded',
    'test_work_signal_raises_between_cli_calls',
    'test_unittest_census_accepts_both_supported_spellings',
)


class OwnerSignals:
    def __init__(self):
        self.retiring, self.observed = False, []

    def __call__(self, number, _frame):
        self.observed.append(number)
        if not self.retiring:
            raise InterruptedError('owned container signal ' + str(number))


def leaf_guard(retiring, signals, now, started, seconds, deadline, storage):
    require((retiring or not signals) and now - started < seconds and now + CLI_CLEANUP_SECONDS < deadline,
            'owned CLI deadline/signal')
    if not retiring:
        storage()


def test_census(raw):
    text = raw.decode('utf-8')
    names = re.findall(r'^(test_[a-z0-9_]+) \(test_reference\.ReferenceTests(?:\.\1)?\) \.\.\. ok$', text, re.M)
    require(len(names) == len(TEST_NAMES) == 20 and set(names) == set(TEST_NAMES)
            and re.search(r'\nRan 20 tests in [0-9.]+s\n\nOK\s*$', text), 'twenty exact pure tests')


def container_environment():
    return dict(USER='harmenon', LOGNAME='harmenon', HOME='/scratch/home', TMPDIR='/scratch/tmp',
        LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC', PYTHONDONTWRITEBYTECODE='1', PYTHONUNBUFFERED='1',
        HF_HUB_OFFLINE='1', TRANSFORMERS_OFFLINE='1', HF_DATASETS_OFFLINE='1', HF_HUB_DISABLE_TELEMETRY='1',
        TOKENIZERS_PARALLELISM='false', CUBLAS_WORKSPACE_CONFIG=':4096:8', HIP_VISIBLE_DEVICES='0',
        XDG_CACHE_HOME='/scratch/cache', HF_HOME='/scratch/huggingface', TORCH_HOME='/scratch/torch',
        TRITON_CACHE_DIR='/scratch/triton', TORCHINDUCTOR_CACHE_DIR='/scratch/inductor',
        TORCH_EXTENSIONS_DIR='/scratch/extensions', MIOPEN_USER_DB_PATH='/scratch/miopen',
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1', MAX_JOBS='2',
        TORCHINDUCTOR_COMPILE_THREADS='2', DO_NOT_TRACK='1')


def build_command(overlay):
    argv = ['/usr/bin/docker', 'create', '--pull=never', '--name', NAME,
        '--label', 'ferric.readiness.reference.owner=' + str(ROOT), '--network=none', '--read-only',
        '--user', '9661:9661', '--group-add', '993', '--device', '/dev/kfd',
        '--device', '/dev/dri/renderD128', '--cap-drop=ALL', '--security-opt', 'no-new-privileges',
        '--cpuset-cpus', '8,9', '--cpus', '2', '--memory', '64g', '--memory-swap', '64g',
        '--pids-limit', '256', '--shm-size', '1g', '--ulimit', 'core=0:0',
        '--ulimit', 'fsize=1073741824:1073741824', '--log-driver=none', '--workdir', '/source',
        '--entrypoint', '/usr/bin/timeout']
    for source, target, readonly in ((MODEL, '/model', True), (ROOT / 'source', '/source', True),
            (ROOT / 'inputs', '/inputs', True), (overlay, '/packages', True),
            (ROOT / 'output', '/output', False), (ROOT / 'scratch', '/scratch', False),
            (ROOT / 'scratch/tmp', '/tmp', False)):
        argv.extend(['--mount', 'type=bind,src=%s,dst=%s%s' % (source, target, ',readonly' if readonly else '')])
    for key, value in container_environment().items():
        argv.extend(['--env', key + '=' + value])
    return argv + [IMAGE, '--signal=TERM', '--kill-after=15s', '900s',
                   '/usr/bin/python3', '-I', '-B', '/source/run.py']


def idle(raw):
    rows = parse(raw)
    require(isinstance(rows, list) and len(rows) == 8, 'expected exactly eight GPU audit rows')
    require(all(set(row) == {'gpu', 'process_list'} and type(row['gpu']) is int for row in rows)
            and {row['gpu'] for row in rows} == set(range(8))
            and all(row['process_list'] == [{'process_info': 'No running processes detected'}]
                    for row in rows), 'GPU process roster is not exact-empty')
    return rows


def topology():
    device = Path('/sys/class/drm/renderD128/device')
    require(int((device / 'unique_id').read_text().strip(), 16) == 16366993098680759275,
            'selected physical gfx950 identity')
    return dict(host=os.uname().nodename, boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                unique_id=16366993098680759275, pci_path=str(device.resolve(strict=True)))


def tree(root, expected, file_cap=64 << 20, total_cap=512 << 20):
    require(root.resolve(strict=True) == root and not root.is_symlink(), 'canonical immutable tree')
    found, total = {}, 0
    for directory, dirs, files in os.walk(root, followlinks=False):
        for name in dirs:
            require(not (Path(directory) / name).is_symlink(), 'tree directory link')
        for name in files:
            path = Path(directory) / name
            relative = str(path.relative_to(root))
            require(relative in expected and len(found) < 20000, 'tree extra file/count')
            raw, row = read(path, expected[relative], file_cap)
            total += len(raw)
            require(total <= total_cap, 'immutable tree aggregate cap')
            found[relative] = {key: row[key] for key in ('bytes', 'sha256')}
    require(set(found) == set(expected), 'tree missing file')
    return found


def writable_bounds():
    for name, cap, file_cap in (('output', 32 << 20, 8 << 20), ('scratch', 2 << 30, 1 << 30),
                              ('evidence', 64 << 20, 8 << 20)):
        total = count = 0
        for directory, dirs, files in os.walk(ROOT / name, followlinks=False):
            for item in dirs:
                require(not (Path(directory) / item).is_symlink(), 'owned directory link')
            for item in files:
                path = Path(directory) / item
                status = path.lstat()
                require(path.is_file() and not path.is_symlink() and status.st_nlink == 1
                        and status.st_size <= file_cap, 'owned file bound/type')
                total += status.st_size; count += 1
        require(count <= 32768 and total <= cap, 'owned tree aggregate bound')
    require(shutil.disk_usage(ROOT).free >= 38 << 30, 'live free disk floor')


def command(owned, label, argv, deadline, seconds=30, cwd=None, owner_signals=None, retiring=False):
    require(time.monotonic() + CLI_CLEANUP_SECONDS < deadline, 'pre-spawn owned CLI cleanup reserve')
    directory = ROOT / 'evidence' / label
    directory.mkdir(mode=0o700)
    save(directory / 'command.json', encoded(dict(argv=argv, env=CPU_ENV, seconds=seconds)))
    tracker, child, result, reason = owned.OwnedProcesses(), None, None, None
    signals = []
    previous = {s: signal.getsignal(s) for s in SIGNALS}
    def defer(number, _frame):
        signals.append(number)
    started = time.monotonic()
    try:
        for s in previous:
            signal.signal(s, defer)
        with (directory / 'stdout').open('xb') as stdout, (directory / 'stderr').open('xb') as stderr:
            child = subprocess.Popen(argv, cwd=cwd or ROOT, env=CPU_ENV, stdin=subprocess.DEVNULL,
                stdout=stdout, stderr=stderr, start_new_session=True)
            tracker.attach(child)
            save(directory / 'started.json', encoded(dict(parent=tracker.root, supervisor_pid=os.getpid())))
            while True:
                tracker.discover()
                leaf_guard(retiring, signals, time.monotonic(), started, seconds, deadline, writable_bounds)
                require(max(os.fstat(stdout.fileno()).st_size, os.fstat(stderr.fileno()).st_size) <= 8 << 20,
                        'owned CLI stream bound')
                if os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT):
                    break
                time.sleep(.2)
    except BaseException as error:
        reason = type(error).__name__ + ': ' + str(error)
    finally:
        try:
            if child is not None:
                try:
                    result = tracker.cleanup()
                except BaseException as error:
                    reason = reason or str(error)
                    result = owned.emergency_reap(child, tracker.events)
        finally:
            try:
                tracker.close_fds()
            finally:
                for s, handler in previous.items():
                    signal.signal(s, handler)
    if owner_signals is not None:
        owner_signals.observed.extend(signals)
    require(result is not None, 'CLI spawn/registration failed: ' + str(reason))
    stdout, outpin = read(directory / 'stdout')
    stderr, errpin = read(directory / 'stderr')
    row = dict(**result, reason=reason, observed_signals=signals, stdout=outpin, stderr=errpin,
               elapsed_seconds=time.monotonic() - started)
    save(directory / 'result.json', encoded(row))
    require(result['exit_code'] == 0 and result['cleanup_signalled'] is False
            and result['owned_groups_absent'] is result['owned_processes_reaped'] is True
            and reason is None and (retiring or not signals), label + ' failed naturally or during owned cleanup')
    return stdout


def owned_container(value):
    require(type(value) is list and len(value) == 1, 'one exact owned container')
    info = value[0]
    require(info['Name'] == '/' + NAME and info['Config']['Image'] == IMAGE
            and info['Config']['Labels'].get('ferric.readiness.reference.owner') == str(ROOT)
            and info['HostConfig']['NetworkMode'] == 'none', 'exact container ownership/image/network')
    return info


def main(plan_sha):
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03'
            and Path(__file__).resolve() == ROOT / 'source/launch.py', 'fixed host/controller identity')
    plan_raw, plan_pin = read(ROOT / 'launch-plan.json')
    require(plan_pin['sha256'] == plan_sha, 'root-pinned fresh launch plan')
    plan = parse(plan_raw)
    require(plan['schema'] == 'ferric-readiness40-position5-reference-launch-v1'
            and plan['image'] == IMAGE and plan['topology'] == topology(), 'fresh topology/image plan')
    source_manifest_raw, source_pin = read(ROOT / 'source/source-manifest.json', plan['source_manifest'])
    source_manifest = parse(source_manifest_raw)
    source_files = dict(source_manifest['files'], **{'source-manifest.json': compact(source_manifest_raw)})
    source_before = tree(ROOT / 'source', source_files)
    overlay = Path(plan['overlay_root'])
    overlay_raw, overlay_pin = read(plan['overlay_manifest']['path'], plan['overlay_manifest'])
    overlay_files = parse(overlay_raw)['files']
    overlay_before = tree(overlay, overlay_files)
    contract = parse((ROOT / 'source/inputs.json').read_bytes())
    input_files = {contract['locations'][name]: {k: row[k] for k in ('bytes', 'sha256')}
                   for name, row in contract['files'].items()}
    input_files['environment.json'] = plan['environment']
    inputs_before = tree(ROOT / 'inputs', input_files)
    for name in ('output', 'scratch', 'evidence'):
        path = ROOT / name
        require(not os.path.lexists(path), 'fresh owner output')
        path.mkdir(mode=0o700)
    for name in ('home', 'tmp'):
        (ROOT / 'scratch' / name).mkdir(mode=0o700)
    owned = load(ROOT / 'source/owned.py', dict(bytes=source_files['owned.py']['bytes'], sha256=OWNED_SHA), 'readiness_owned')
    owned.subreaper()
    started, cid, attempted, errors, posterrors = time.monotonic(), None, 0, [], []
    work_deadline, retirement_deadline = started + 1200, started + 1800
    result = dict(schema='ferric-readiness40-position5-reference-owned-v1', passed=False,
                  plan=plan_pin, native_execution=False, numerical_acceptance=False, acceptance_threshold=None,
                  full_model_acceptance=False, performance_claim=False, production_authority=False)
    owner_signals = OwnerSignals()
    original_handlers = {s: signal.getsignal(s) for s in SIGNALS}
    for s in original_handlers:
        signal.signal(s, owner_signals)
    def cli(label, argv, seconds=30):
        return command(owned, label, argv, retirement_deadline if owner_signals.retiring else work_deadline,
                       seconds, owner_signals=owner_signals, retiring=owner_signals.retiring)
    try:
        require(shutil.disk_usage(ROOT).free >= 40 << 30, 'initial disk floor')
        require(cli('image', ['/usr/bin/docker', 'image', 'inspect', '--format', '{{.Id}}', IMAGE]).decode().strip() == IMAGE,
                'actual immutable cached image')
        require(not cli('name-before', ['/usr/bin/docker', 'ps', '-aq', '--no-trunc', '--filter', 'name=^/' + NAME + '$']).strip(),
                'fresh exact container name')
        command(owned, 'cpu-tests', ['/usr/bin/python3', '-B', '-m', 'unittest', '-v', 'test_reference'],
                work_deadline, 120, ROOT / 'source', owner_signals=owner_signals)
        test_census(read(ROOT / 'evidence/cpu-tests/stderr')[0])
        for i in range(3):
            idle(cli('before-%d' % i, ['/usr/bin/amd-smi', 'process', '--json']))
        require(topology() == plan['topology'], 'prelaunch topology unchanged')
        require(time.monotonic() + 930 + CLI_CLEANUP_SECONDS < work_deadline, 'native leaf plus CLI cleanup reserve')
        cid = cli('create', build_command(overlay)).decode().strip()
        require(re.fullmatch('[0-9a-f]{64}', cid), 'owned immutable container ID')
        info = owned_container(parse(cli('created', ['/usr/bin/docker', 'inspect', cid])))
        require(info['Id'] == cid and not info['State']['Running'], 'created but not executed container')
        attempted = 1
        cli('execute', ['/usr/bin/docker', 'start', '-a', cid], 930)
        info = owned_container(parse(cli('exited', ['/usr/bin/docker', 'inspect', cid])))
        require(info['State']['Status'] == 'exited' and info['State']['ExitCode'] == 0
                and info['State']['OOMKilled'] is False, 'natural framework container exit')
        child_raw, child_pin = read(ROOT / 'output/complete.json')
        child = parse(child_raw)
        require(child['passed'] is True and child['error'] is None and child['postcheck_errors'] == []
                and child['schema'] == 'ferric-readiness40-position5-framework-reference-v1'
                and child['full_model_forward_calls'] == 80 and child['generated_tokens'] == 0
                and child['prompt_tokens_authenticated'] == 2048 and child['repeat_gate_passed'] is True
                and child['selected_positions'] == [0, 5, 16, 39]
                and child['native_intermediates_used'] is False and child['candidate_receipt'] is None
                and child['numerical_acceptance'] is False and child['full_model_acceptance'] is False
                and child['performance_claim'] is False, 'two-pass forty-position reference report')
        import reference
        input_bodies = {role: read(ROOT / 'inputs' / contract['locations'][role])[0] for role in contract['files']}
        tokens, _, _ = reference.prompt_inputs(contract, input_bodies)
        payloads = [{position: read(ROOT / 'output' / ('pass%d-pos%d.bf16' % (ordinal, position)))[0]
                     for position in reference.SELECTED} for ordinal in (1, 2)]
        require(child['full_prompt_tokens'] == tokens and child['input_tokens'] == tokens[:40]
                and reference.repeat_gate(child['passes'], tokens, payloads), 'independent owner repeat/capture join')
        for ordinal, value in enumerate(child['passes'], 1):
            require(parse(read(ROOT / 'output' / ('pass%d.json' % ordinal))[0]) == value, 'saved pass identity')
        output_names = set()
        for row in child['output_pins']:
            path = Path(row['path'])
            require(path.parent == Path('/output') and path.name not in output_names, 'closed child output pin')
            read(ROOT / 'output' / path.name, {key: row[key] for key in ('bytes', 'sha256')})
            output_names.add(path.name)
        expected_outputs = {'pass%d-pos%d.bf16' % (ordinal, position) for ordinal in (1, 2)
                            for position in reference.SELECTED}
        expected_outputs |= {'pass1.json', 'pass2.json'} | {'implementation-' + n + '.py' for n in
            ('modeling_qwen3', 'activation', 'sdpa', 'torch_functional')}
        require(output_names == expected_outputs
                and {p.name for p in (ROOT / 'output').iterdir()} == output_names | {'complete.json'},
                'full child output closure')
        result['framework_report'] = child_pin
    except BaseException as error:
        errors.append(type(error).__name__ + ': ' + str(error))
    finally:
        owner_signals.retiring = True
        try:
            # Recover only the exact name/label/image after a possibly partial docker-create.
            if cid is None:
                found = cli('recover', ['/usr/bin/docker', 'ps', '-aq', '--no-trunc', '--filter', 'name=^/' + NAME + '$']).decode().split()
                require(len(found) <= 1, 'unique owned recovery')
                cid = found[0] if found else None
            if cid is not None:
                info = owned_container(parse(cli('cleanup-inspect', ['/usr/bin/docker', 'inspect', cid])))
                if info['State']['Running']:
                    cli('stop', ['/usr/bin/docker', 'stop', '--time', '15', cid], 30)
                final = owned_container(parse(cli('final-inspect', ['/usr/bin/docker', 'inspect', cid])))
                require(not final['State']['Running'], 'owned container stopped before removal')
                result['final_container_state'] = final['State']
                cli('remove', ['/usr/bin/docker', 'rm', cid])
            require(not cli('name-after', ['/usr/bin/docker', 'ps', '-aq', '--filter', 'name=^/' + NAME + '$']).strip(),
                    'owned container name absent')
            result['container_removed'] = True
        except BaseException as error:
            posterrors.append('container cleanup: ' + str(error))
        for i in range(3):
            try:
                idle(cli('after-%d' % i, ['/usr/bin/amd-smi', 'process', '--json']))
            except BaseException as error:
                posterrors.append('idle: ' + str(error))
        for label, action in (
                ('sources', lambda: tree(ROOT / 'source', source_files) == source_before),
                ('inputs', lambda: tree(ROOT / 'inputs', input_files) == inputs_before),
                ('packages', lambda: tree(overlay, overlay_files) == overlay_before),
                ('overlay-manifest', lambda: read(overlay_pin['path'], overlay_pin)[1] == overlay_pin),
                ('plan', lambda: read(plan_pin['path'], plan_pin)[1] == plan_pin),
                ('topology', lambda: topology() == plan['topology'])):
            try:
                require(action(), label + ' changed')
            except BaseException as error:
                posterrors.append(label + ': ' + str(error))
        try:
            writable_bounds()
        except BaseException as error:
            posterrors.append('final writable tree bounds: ' + str(error))
        result.update(errors=errors, postcheck_errors=posterrors, framework_attempts=attempted,
            retries=0, elapsed_seconds=time.monotonic() - started, source_files=source_before,
            input_files=inputs_before, overlay_manifest=overlay_pin,
            observed_signals=owner_signals.observed, whole_deadline_seconds=1800,
            operational_cutoff_seconds=1200, retirement_reserve_seconds=600,
            passed=attempted == 1 and not errors and not posterrors and not owner_signals.observed
                and time.monotonic() < retirement_deadline and result.get('container_removed') is True)
        try:
            save(ROOT / ('complete.json' if result['passed'] else 'failed.json'), encoded(result))
        finally:
            for s, handler in original_handlers.items():
                signal.signal(s, handler)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    require(len(sys.argv) == 2 and re.fullmatch('[0-9a-f]{64}', sys.argv[1]), 'launch PLAN_SHA')
    os.umask(0o077)
    raise SystemExit(main(sys.argv[1]))
