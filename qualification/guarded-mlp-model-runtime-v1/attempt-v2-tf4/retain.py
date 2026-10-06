"""Retain the exact failed TF4 attempt as data; import or execute no project code."""
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import stat
import sys

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/guarded-model-gpu-result-v2')
DESTINATION = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-model-runtime-v1/attempt-v2-tf4')
REMOTE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-model-gpu-v228-v2'
TERMINAL = dict(bytes=91510, sha256='8ced1ef7f167348fbcb68ce56cca8fc8e2238273ebe18fd737127563779871ce')
ROOT_PINS = {
    'ar4-input.json': dict(bytes=1473, sha256='028c0c426724bd1bdd8c567edb236fd7eed89054361b886f8fc30cc4edfee78e'),
    'ar4-request.json': dict(bytes=9302, sha256='f46ccb56dabbfe492594706ae558f6edd359d948f6e689f1ca78fe69acadc2c3'),
    'frozen_owned.py': dict(bytes=30433, sha256='ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'),
    'library_audit.py': dict(bytes=25872, sha256='b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'),
    'prepare_model_inputs.py': dict(bytes=12601, sha256='e0834c53036cc2852dfa26a89c26f255dbdbc3279402df3b221df8c65ae7dfbf'),
    'prepared-inputs.json': dict(bytes=13713, sha256='955058fabfaad784f999f08c8026876bd51689f485a756e16b8733b200555a08'),
    'run_model_gpu.py': dict(bytes=25291, sha256='865d00d994a7bcb2ff62f3bef6a33ee133fad03766f611c4b6833f11c96d0c09'),
    'tf4-input.json': dict(bytes=1473, sha256='51f8a860efe329940e6fc825121b91f6b2d1dbdd382b2f7cb231854a71cceba8'),
    'tf4-request.json': dict(bytes=9302, sha256='c126bca4d314bbae35fbf45a4ecc30a5d182b150d19e3d3025612c8eb79f7bd0'),
    'validate_observation.py': dict(bytes=13779, sha256='367d1904f15741138db708c1f10e5a777905b62d9440a961e952a6cc9125b055'),
}
LABELS = ('parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd',
          'before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
FALSE_FLAGS = ('numerical_acceptance', 'full_model_acceptance', 'independent_full_model_reference',
               'full_long_workload', 'performance_claim', 'production_authority')
MAX_FILES, MAX_BODY, MAX_TOTAL = 96, 1 << 20, 8 << 20


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def pin(body):
    return dict(bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def compact(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def ordinary(name):
    return (type(name) is str and Path(name).as_posix() == name and name not in ('', '.')
            and not Path(name).is_absolute() and '..' not in Path(name).parts)


def stamp(row):
    return (row.st_dev, row.st_ino, row.st_mode, row.st_nlink,
            row.st_size, row.st_mtime_ns, row.st_ctime_ns)


def read(path, expected=None):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_nlink == 1 and 0 <= before.st_size <= MAX_BODY,
            'ordinary bounded unlinked body: ' + str(path))
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'opened file changed')
        body = stream.read(MAX_BODY + 1)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'open file changed while reading')
    require(stamp(path.lstat()) == stamp(before) and len(body) == before.st_size
            and (expected is None or pin(body) == compact(expected)), 'file pin or identity changed: ' + str(path))
    return body


def roster(root):
    require(root.resolve(strict=True) == root and root.is_dir(), 'ordinary evidence directory')
    result = set()
    for directory, dirs, names in os.walk(root, followlinks=False,
            onerror=lambda error: (_ for _ in ()).throw(error)):
        require(all(not (Path(directory) / name).is_symlink() for name in dirs), 'directory symlink')
        for name in names:
            path = Path(directory) / name
            relative = str(path.relative_to(root))
            require(ordinary(relative) and relative not in result, 'ordinary unique source member')
            require(path.resolve(strict=True) == path and stat.S_ISREG(path.lstat().st_mode), 'nonregular member')
            result.add(relative)
        require(len(result) <= MAX_FILES, 'bounded member census')
    return result


def wire_pin(row):
    require(type(row['sha256']) is list and len(row['sha256']) == 32
            and all(type(value) is int and 0 <= value <= 255 for value in row['sha256']), 'exact wire digest bytes')
    return dict(bytes=row['bytes'], sha256=bytes(row['sha256']).hex())


def validate(bodies):
    failed = json.loads(bodies['tf4/failed.json'])
    require(pin(bodies['tf4/failed.json']) == TERMINAL and failed['schema'] == 'ferric-guarded-mlp-model-gpu-v1'
            and failed['passed'] is False and failed['mode'] == 'tf4'
            and failed['errors'] == ['RuntimeError: one actual worker announcement']
            and all(failed[key] is False for key in FALSE_FLAGS)
            and all(failed[key] is True for key in ('gpu_execution', 'gpu_execution_requested',
                'gpu_execution_confirmed', 'native_spawn_observed'))
            and failed['native_attempts'] == 1 and failed['retries'] == 0, 'preserve the exact failed outer outcome')
    require(pin(bodies['run_model_gpu.py']) == compact(failed['controller'])
            and failed['controller']['path'] == REMOTE + '/run_model_gpu.py', 'original failed controller identity')
    require(set(failed['raw']) == {name[4:] for name in bodies if name.startswith('tf4/') and name != 'tf4/failed.json'}
            and len(failed['raw']) == 76, 'all76 raw files retained, no omissions or extras')
    for name, row in failed['raw'].items():
        require(ordinary(name) and row['path'] == REMOTE + '/tf4/' + name
                and pin(bodies['tf4/' + name]) == compact(row), 'raw receipt pin join: ' + name)
    root_readset = {path[len(REMOTE) + 1:]: row for path, row in failed['readset'].items()
                   if path.startswith(REMOTE + '/')}
    require(set(root_readset) == {'frozen_owned.py', 'library_audit.py', 'run_model_gpu.py',
        'tf4-input.json', 'tf4-request.json', 'validate_observation.py'}, 'six actual local TF4 readset inputs')
    for name, row in root_readset.items():
        require(row['path'] == REMOTE + '/' + name and compact(row) == ROOT_PINS[name], 'actual readset/input join')
    plan, request, prepared = (json.loads(bodies[name]) for name in
                              ('tf4-input.json', 'tf4-request.json', 'prepared-inputs.json'))
    require(compact(failed['plan']) == ROOT_PINS['tf4-input.json'] and plan['mode'] == 'tf4'
            and compact(plan['request']) == ROOT_PINS['tf4-request.json'], 'actual TF4 request and plan')
    for mode in ('tf4', 'ar4'):
        for role, suffix in (('plan', 'input'), ('request', 'request')):
            row = prepared['generated'][mode][role]
            require(row['path'] == REMOTE + '/' + mode + '-' + suffix + '.json'
                    and compact(row) == ROOT_PINS[mode + '-' + suffix + '.json'], 'prepared context body pin')
    for role in ('parent', 'parent_cpu', 'worker', 'worker_cpu'):
        require(plan[role] == failed['admission'][role], 'admitted product/CPU ancestry pin')
    require([row['label'] for row in failed['phases']] == list(LABELS), 'exact11 phase roster')
    for phase in failed['phases']:
        label = phase['label']
        result = json.loads(bodies['tf4/' + label + '/result.json'])
        command = json.loads(bodies['tf4/' + label + '/command.json'])
        started = json.loads(bodies['tf4/' + label + '/started.json'])
        require(result == {key: value for key, value in phase.items() if key != 'label'}
                and phase['exit_code'] == 0 and phase['reason'] is None
                and phase['cleanup_signalled'] is False and phase['owned_groups_absent'] is True
                and phase['owned_processes_reaped'] is True
                and phase['gpu_execution_requested'] is (label == 'parent'), 'actual clean phase, not outer acceptance')
        for field in ('command', 'started', 'stdout', 'stderr'):
            require(phase[field] == failed['raw'][label + '/' + ('command.json' if field == 'command'
                else 'started.json' if field == 'started' else field)], 'phase raw file join')
        require(started['command_sha256'] == phase['command']['sha256']
                and command['gpu_execution_requested'] is (label == 'parent')
                and phase['lineage'][0]['event'] == 'owned'
                and phase['lineage'][0]['reason'] == 'spawned-parent'
                and phase['lineage'][0]['identity'] == started['parent'], 'owned process spawn and command join')
    native = json.loads(bodies['tf4/native/complete.json'])
    require(bodies['tf4/native/complete.json'] == bodies['tf4/parent/stdout']
            and native['schema'] == 'FerricFiniteGuardedMlpDecodeObservationV1'
            and native['request'] == request and native['completed_forwards'] == 4
            and native['child_pid'] == failed['observation']['child_pid'] == 2809580
            and native['native_attempts'] == 1 and native['retries'] == 0
            and all(native[key] is True for key in ('child_exit_zero', 'process_group_absent',
                'native_closed', 'gpu_execution'))
            and all(native[key] is False for key in ('numerical_acceptance', 'full_model_acceptance',
                'performance_claim', 'production_authority', 'full_long_workload')), 'four completed native frames without outer qualification')
    expected_stderr = ('finite guarded owned child pid=2809580 pgid=2809580; no native setup acknowledged\n'
        + ''.join('finite guarded completed position=' + str(index) + '\n' for index in range(4))).encode()
    require(bodies['tf4/parent/stderr'] == expected_stderr, 'exact failed announcement and completion stream')
    parent_phase = next(row for row in failed['phases'] if row['label'] == 'parent')
    require(any(row['event'] == 'owned' and row['identity']['pid'] == row['identity']['pgid'] == 2809580
            and row['reason'] == 'ancestry' for row in parent_phase['lineage']), 'actual child ownership observation')
    require(native['input_tokens'] == failed['observation']['input_tokens'] == [9112, 2190, 3772, 220]
            and native['observed_output_tokens'] == failed['observation']['output_tokens'] == [67, 198, 25, 16]
            and failed['observation']['local_bank_generations'] == [1, 1, 2, 2], 'recorded TF4 tokens and bank generations')
    require(len(native['files']['frames']) == 4, 'four original native file records')
    for index, frame in enumerate(native['files']['frames']):
        event = frame['response']['event']
        require(event['status'] == 'completed' and event['position'] == index and event['generation'] == index + 1
                and frame['response']['id'] == index + 1, 'four ordered completed frame records')
        for role, suffix in (('control', 'bin'), ('observation', 'bin'), ('request', 'json')):
            name = 'native/' + role + '-' + str(index) + '.' + suffix
            require(frame[role]['path'] == REMOTE + '/tf4/' + name
                    and wire_pin(frame[role]) == compact(failed['raw'][name]), 'original native frame payload digest')
            if role != 'request':
                require(wire_pin(event[role]) == wire_pin(frame[role]), 'response and retained payload digest join')
    require(wire_pin(native['files']['child_stderr']) == compact(failed['raw']['native/child-stderr.bin'])
            and native['files']['summary_bytes'] == len(bodies['tf4/native/complete.json'])
            and native['files']['total_bytes'] == sum(len(body) for name, body in bodies.items()
                if name.startswith('tf4/native/')), 'complete small native evidence extent')
    for label in ('before-0', 'before-1', 'before-2', 'after-0', 'after-1', 'after-2'):
        idle = json.loads(bodies['tf4/' + label + '/stdout'])
        require(idle == [dict(gpu=index, process_list=[dict(process_info='No running processes detected')])
                        for index in range(8)], 'retained eight-GPU idle observation')
        require(json.loads(bodies['tf4/' + label + '-topology.json']) == failed['platform'], 'stable recorded platform')
    require(json.loads(bodies['tf4/initial-topology.json']) == failed['platform'], 'initial platform join')
    return failed


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B retain.py')
    require(not os.path.lexists(DESTINATION), 'fresh canonical evidence destination required')
    resource.setrlimit(resource.RLIMIT_AS, (256 << 20, 256 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (8 << 20, 8 << 20))
    signal.alarm(120)
    os.umask(0o077)
    failed_body = read(SOURCE / 'tf4/failed.json', TERMINAL)
    failed = json.loads(failed_body)
    expected = dict(ROOT_PINS)
    expected.update({'tf4/' + name: compact(row) for name, row in failed['raw'].items()})
    expected['tf4/failed.json'] = TERMINAL
    require(len(expected) == 87 and roster(SOURCE) == set(expected), 'closed actual10-root/77-case source roster')
    bodies = {name: read(SOURCE / name, row) for name, row in sorted(expected.items())}
    require(sum(map(len, bodies.values())) == 3791177 and sum(map(len, bodies.values())) <= MAX_TOTAL,
            'exact original copied evidence extent')
    failed = validate(bodies)
    self_path = Path(__file__).resolve()
    bodies['retain.py'] = read(self_path)
    bodies['README.md'] = read(self_path.parent / 'README.md')
    manifest = dict(schema='ferric-guarded-mlp-model-gpu-retained-v1',
        files={name: pin(body) for name, body in sorted(bodies.items())},
        original_terminal=TERMINAL, retained_original_source_files=87, root_inputs=10,
        case_files=77, raw_count=76, retention_verified=True, passed=False,
        original_errors=failed['errors'], mode='tf4', native_attempts=1, retries=0,
        native_completed_frames_observed=4, native_parent_exit_code=0,
        gpu_execution_observed=True, outer_failed_receipt_preserved=True,
        selected_external_readset_pin_metadata_retained=True, all_external_readset_bodies_retained=False,
        ar4_execution_observed=False, numerical_acceptance=False, full_model_acceptance=False,
        independent_full_model_reference=False, full_long_workload=False,
        performance_claim=False, production_authority=False, project_code_executed_by_retainer=False)
    bodies['manifest.json'] = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()
    require(len(bodies) == 90 and sum(map(len, bodies.values())) <= MAX_TOTAL, 'bounded90-member retained closure')
    require(roster(SOURCE) == set(expected), 'source roster changed before retention')
    for name, row in expected.items():
        read(SOURCE / name, row)
    DESTINATION.parent.mkdir(parents=True, exist_ok=True)
    require(DESTINATION.parent.resolve(strict=True) == DESTINATION.parent, 'ordinary destination parent')
    DESTINATION.mkdir(mode=0o700)
    for name, body in sorted(bodies.items()):
        require(ordinary(name), 'ordinary retained path')
        path = DESTINATION / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('xb') as stream:
            stream.write(body)
        read(path, pin(body))
    require(roster(DESTINATION) == set(bodies) and roster(SOURCE) == set(expected), 'complete retained and original rosters')
    for name, body in bodies.items():
        read(DESTINATION / name, pin(body))
    for name, row in expected.items():
        read(SOURCE / name, row)
    print(json.dumps(dict(retention_verified=True, original_attempt_passed=False,
        destination=str(DESTINATION), members=len(bodies), pinned_members=len(manifest['files']),
        raw_count=76, expanded_bytes=sum(map(len, bodies.values())), manifest=pin(bodies['manifest.json']),
        original_terminal=TERMINAL, gpu_execution_performed_by_retainer=False), sort_keys=True))


if __name__ == '__main__':
    main()
