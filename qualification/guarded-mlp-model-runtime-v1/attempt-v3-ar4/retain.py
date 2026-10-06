"""Retain an authenticated actual AR4 outcome as data; execute no project code."""
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys

SOURCE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/guarded-model-gpu-result-v3')
DESTINATION = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-model-runtime-v1/attempt-v3-ar4')
REMOTE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-model-gpu-v228-v3'
TERMINAL = dict(bytes=93497, sha256='edf05cf2dd19a9934dab9762b328ea3e6160cf01c15606a3df3299ce967e4991')
TERMINAL_NAME = 'complete.json'
ROOT_PINS = {
    'ar4-input.json': dict(bytes=1473, sha256='70e4e52153ddb9d8ca55fc3fb6dc4559ad22efa241b3cb6c18d071698866d8c7'),
    'ar4-request.json': dict(bytes=9296, sha256='c0bcbdc384d3b9dc5fafbbcb8d40434e646cd9377df8468d4e120a6a76de0049'),
    'frozen_owned.py': dict(bytes=30433, sha256='ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'),
    'library_audit.py': dict(bytes=25872, sha256='b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'),
    'prepare_model_inputs.py': dict(bytes=12571, sha256='d2ce7fa6d6afc0d2d2860a12e077d31c230b33f4934f0bc82f8738fca0cc2571'),
    'prepared-inputs.json': dict(bytes=11957, sha256='7d719e5e39a79a9390a5ab94f0659897f5124d25b9aa7fc4a174d5a8d9bc8bbc'),
    'run_model_gpu.py': dict(bytes=26891, sha256='032724ae531b9a7f9b081c257a5eeed943389df94867d8eff17ec333578c88d2'),
    'validate_observation.py': dict(bytes=13779, sha256='367d1904f15741138db708c1f10e5a777905b62d9440a961e952a6cc9125b055'),
    'guarded_announcement.py': dict(bytes=3251, sha256='96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80'),
}
LABELS = ('parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd',
          'before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
FALSE_FLAGS = ('numerical_acceptance', 'full_model_acceptance', 'independent_full_model_reference',
               'full_long_workload', 'performance_claim', 'production_authority')
MAX_FILES, MAX_BODY, MAX_TOTAL = 269, 8 << 20, 72 << 20


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


def same(left, right):
    return json.dumps(left, sort_keys=True, separators=(',', ':'), allow_nan=False) == json.dumps(
        right, sort_keys=True, separators=(',', ':'), allow_nan=False)


def success_names():
    names = {label + '/' + name for label in LABELS
             for name in ('command.json', 'started.json', 'stdout', 'stderr', 'result.json')}
    names |= {'initial-topology.json', 'observation.json', 'native/complete.json', 'native/child-stderr.bin'}
    names |= {side + '-' + str(index) + '-topology.json' for side in ('before', 'after') for index in range(3)}
    names |= {'native/' + role + '-' + str(index) + '.' + suffix for index in range(4)
              for role, suffix in (('request', 'json'), ('control', 'bin'), ('observation', 'bin'))}
    require(len(names) == 77, 'fixed successful raw census')
    return names


def validate(bodies):
    terminal = json.loads(bodies['ar4/' + TERMINAL_NAME])
    require(terminal['schema'] == 'ferric-guarded-mlp-model-gpu-v1' and terminal['mode'] == 'ar4'
            and type(terminal['passed']) is bool and terminal['retries'] == 0
            and type(terminal['native_attempts']) is int and terminal['native_attempts'] in (0, 1)
            and all(terminal[key] is False for key in FALSE_FLAGS), 'original bounded outcome and authority')
    require(TERMINAL_NAME == ('complete.json' if terminal['passed'] else 'failed.json')
            and type(terminal['errors']) is list
            and (terminal['errors'] == [] if terminal['passed'] else bool(terminal['errors'])), 'terminal status preserved')
    require(compact(terminal['controller']) == ROOT_PINS['run_model_gpu.py']
            and terminal['controller']['path'] == REMOTE + '/run_model_gpu.py', 'actual current controller')
    require(set(terminal['raw']) == {name[4:] for name in bodies
            if name.startswith('ar4/') and name != 'ar4/' + TERMINAL_NAME}, 'complete original raw closure')
    for name, row in terminal['raw'].items():
        require(ordinary(name) and row['path'] == REMOTE + '/ar4/' + name
                and pin(bodies['ar4/' + name]) == compact(row), 'raw pin join: ' + name)
    root_readset = {path[len(REMOTE) + 1:]: row for path, row in terminal['readset'].items()
                   if path.startswith(REMOTE + '/')}
    require(set(root_readset) == set(ROOT_PINS) - {'prepare_model_inputs.py', 'prepared-inputs.json'},
            'seven current controller readset bodies')
    for name, row in root_readset.items():
        require(row['path'] == REMOTE + '/' + name and compact(row) == ROOT_PINS[name], 'current readset join')
    plan, request, prepared = (json.loads(bodies[name]) for name in
                              ('ar4-input.json', 'ar4-request.json', 'prepared-inputs.json'))
    require(compact(terminal['plan']) == ROOT_PINS['ar4-input.json']
            and terminal['plan']['path'] == REMOTE + '/ar4-input.json'
            and plan['schema'] == 'ferric-guarded-mlp-model-gpu-input-v1' and plan['mode'] == 'ar4'
            and plan['request']['path'] == REMOTE + '/ar4-request.json'
            and compact(plan['request']) == ROOT_PINS['ar4-request.json']
            and request['decode']['mode'] == 'autoregressive'
            and request['decode']['evidence_directory'] == REMOTE + '/ar4/native', 'actual AR4 plan/request')
    require(prepared['passed'] is True and set(prepared['generated']) == {'ar4'}
            and prepared['controller']['path'] == REMOTE + '/prepare_model_inputs.py'
            and compact(prepared['controller']) == ROOT_PINS['prepare_model_inputs.py'], 'actual data preparation')
    for role, suffix in (('plan', 'input'), ('request', 'request')):
        row = prepared['generated']['ar4'][role]
        require(row['path'] == REMOTE + '/ar4-' + suffix + '.json'
                and compact(row) == ROOT_PINS['ar4-' + suffix + '.json'], 'prepared body join')
    for role in ('parent', 'parent_cpu', 'worker', 'worker_cpu'):
        require(plan[role] == terminal['admission'][role]
                and terminal['readset'][plan[role]['path']] == plan[role], 'admitted CPU/product pin metadata')
    for key, extent in (
            ('parser_cpu', dict(bytes=8872, sha256='170ba440cdf33cd88fc55c2ddff709624fd819d3d461d54daad239a719efdf5a')),
            ('tf4_data_revalidation', dict(bytes=79327, sha256='d0551f310a57f63dfe98af8c887b5996752e3b62128a3087c4a03849082e6a3f'))):
        row = terminal[key]
        require(compact(row) == extent and terminal['readset'][row['path']] == row, 'actual prior gate metadata')
    require(terminal['limits'] == dict(native_seconds=4000, whole_seconds=4300,
        address_space_bytes=32 << 30, stream_bytes=8 << 20, case_bytes=64 << 20,
        case_members=256, affinity=[8, 9]), 'unchanged execution limits')
    labels = [phase['label'] for phase in terminal['phases']]
    require(len(labels) == len(set(labels)) and all(label in LABELS for label in labels), 'unique known phases')
    for phase in terminal['phases']:
        label = phase['label']
        require(same(json.loads(bodies['ar4/' + label + '/result.json']),
                     {key: value for key, value in phase.items() if key != 'label'}), 'original phase result body')
        for key, filename in (('command', 'command.json'), ('started', 'started.json'), ('stdout', 'stdout'), ('stderr', 'stderr')):
            if phase[key] is not None:
                require(phase[key] == terminal['raw'][label + '/' + filename], 'phase raw body join')
    if not terminal['passed']:
        return terminal
    require(set(terminal['raw']) == success_names() and labels == list(LABELS)
            and terminal['native_attempts'] == 1
            and all(terminal[key] is True for key in ('gpu_execution_requested', 'gpu_execution',
                'gpu_execution_confirmed', 'native_spawn_observed'))
            and terminal['partial_gpu_execution_possible'] is False, 'full successful AR4 closure')
    for phase in terminal['phases']:
        label = phase['label']
        command = json.loads(bodies['ar4/' + label + '/command.json'])
        started = json.loads(bodies['ar4/' + label + '/started.json'])
        require(phase['exit_code'] == 0 and phase['reason'] is None
                and phase['cleanup_signalled'] is False and phase['owned_groups_absent'] is True
                and phase['owned_processes_reaped'] is True
                and phase['gpu_execution_requested'] is (label == 'parent')
                and command['gpu_execution_requested'] is (label == 'parent')
                and started['command_sha256'] == phase['command']['sha256']
                and phase['lineage'][0]['event'] == 'owned'
                and phase['lineage'][0]['reason'] == 'spawned-parent'
                and phase['lineage'][0]['identity'] == started['parent'], 'natural owned phase')
    checked = json.loads(bodies['ar4/observation.json'])
    require(same(checked, terminal['observation']) and checked['mode'] == 'autoregressive'
            and checked['local_bank_generations'] == [1, 1, 2, 2]
            and checked['retained_parent_files'] == 14 and checked['captured_bf16_words'] == 1213952
            and checked['layer_observations'] == 144 and checked['rank_guard_observations'] == 288
            and checked['actual_lowest_index_argmax_checked'] is True
            and all(checked[key] is False for key in FALSE_FLAGS if key != 'full_long_workload'),
            'original qualified validator observation, not an independent model reference')
    native = json.loads(bodies['ar4/native/complete.json'])
    require(bodies['ar4/native/complete.json'] == bodies['ar4/parent/stdout']
            and native['schema'] == 'FerricFiniteGuardedMlpDecodeObservationV1'
            and native['request'] == request and native['completed_forwards'] == 4
            and native['child_pid'] == checked['child_pid']
            and native['native_attempts'] == 1 and native['retries'] == 0
            and all(native[key] is True for key in ('child_exit_zero', 'process_group_absent', 'native_closed', 'gpu_execution'))
            and all(native[key] is False for key in FALSE_FLAGS if key != 'independent_full_model_reference'),
            'completed native summary and no authority promotion')
    inputs, outputs = native['input_tokens'], native['observed_output_tokens']
    require(inputs == checked['input_tokens'] and outputs == checked['output_tokens']
            and len(inputs) == len(outputs) == 4 and inputs[0] == 9112
            and all(type(value) is int and 0 <= value < 151936 for value in inputs + outputs)
            and inputs[1:] == outputs[:-1], 'own-output AR4 recurrence')
    close = native['close']
    require(close['id'] == 5 and close['event']['status'] == 'closed'
            and close['event']['completed_forwards'] == 4 and close['native_closed'] is True
            and close['event']['transcript_sha256'] == native['transcript_sha256'], 'completed Close transcript')
    phase = next(row for row in terminal['phases'] if row['label'] == 'parent')
    started = json.loads(bodies['ar4/parent/started.json'])
    owned = phase['lineage']
    require(len(owned) == 2 and [row['event'] for row in owned] == ['owned', 'owned']
            and owned[0]['reason'] == 'spawned-parent' and owned[1]['reason'] == 'ancestry',
            'exact actual two-owned-process lineage')
    parent, worker = (row['identity'] for row in owned)
    for identity in (parent, worker):
        require(set(identity) == {'pid', 'pgid', 'ppid', 'sid', 'starttime', 'state', 'uid'}
                and all(type(identity[key]) is int and identity[key] > 0 for key in ('pid', 'pgid', 'ppid', 'sid', 'starttime'))
                and type(identity['uid']) is int and identity['uid'] == 9661
                and identity['state'] in ('R', 'S', 'D', 'I', 'T', 't', 'Z'),
                'recorded process identity shape')
    require(set(started) == {'parent', 'supervisor_pid', 'command_sha256'}
            and type(started['supervisor_pid']) is int and started['supervisor_pid'] > 0
            and parent == started['parent'] and parent['pid'] == parent['pgid'] == parent['sid']
            and parent['ppid'] == started['supervisor_pid'] and worker['pid'] == worker['pgid'] == checked['child_pid']
            and worker['ppid'] == worker['sid'] == parent['pid'] and worker['starttime'] >= parent['starttime']
            and worker['pid'] != parent['pid'] and len(phase['owned_groups']) == 2
            and all(type(value) is int for value in phase['owned_groups'])
            and sorted(phase['owned_groups']) == sorted([parent['pgid'], worker['pgid']])
            and terminal['native_started'] == terminal['raw']['parent/started.json'], 'actual retired worker ancestry')
    expected_stderr = ('finite guarded owned child pid=' + str(worker['pid']) + ' pgid=' + str(worker['pid'])
        + '; no native setup acknowledged\n'
        + ''.join('finite guarded completed position=' + str(index) + '\n' for index in range(4))).encode()
    require(bodies['ar4/parent/stderr'] == expected_stderr and bodies['ar4/native/child-stderr.bin'] == b'',
            'exact guarded announcement and empty worker stderr')
    require(len(native['files']['frames']) == 4, 'four native file records')
    for index, frame in enumerate(native['files']['frames']):
        event = frame['response']['event']
        require(event['status'] == 'completed' and event['position'] == index
                and event['generation'] == index + 1 and frame['response']['id'] == index + 1, 'ordered frame metadata')
        for role, suffix in (('control', 'bin'), ('observation', 'bin'), ('request', 'json')):
            name = 'native/' + role + '-' + str(index) + '.' + suffix
            require(frame[role]['path'] == REMOTE + '/ar4/' + name
                    and wire_pin(frame[role]) == compact(terminal['raw'][name]), 'native original body digest')
            if role != 'request':
                require(wire_pin(event[role]) == wire_pin(frame[role]), 'response and payload digest join')
    require(wire_pin(native['files']['child_stderr']) == compact(terminal['raw']['native/child-stderr.bin'])
            and native['files']['summary_bytes'] == len(bodies['ar4/native/complete.json'])
            and native['files']['total_bytes'] == sum(len(body) for name, body in bodies.items()
                if name.startswith('ar4/native/')), 'complete native evidence extent')
    for label in ('before-0', 'before-1', 'before-2', 'after-0', 'after-1', 'after-2'):
        require(json.loads(bodies['ar4/' + label + '/stdout']) == [dict(gpu=index,
            process_list=[dict(process_info='No running processes detected')]) for index in range(8)],
            'recorded all-eight-GPU idle observation')
        require(same(json.loads(bodies['ar4/' + label + '-topology.json']), terminal['platform']), 'recorded stable platform')
    require(same(json.loads(bodies['ar4/initial-topology.json']), terminal['platform']), 'initial platform join')
    return terminal


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B retain.py')
    require(type(TERMINAL) is dict and set(TERMINAL) == {'bytes', 'sha256'}
            and type(TERMINAL['bytes']) is int and 0 < TERMINAL['bytes'] <= MAX_BODY
            and type(TERMINAL['sha256']) is str and re.fullmatch('[0-9a-f]{64}', TERMINAL['sha256'])
            and TERMINAL_NAME in ('complete.json', 'failed.json'), 'actual terminal remains unbound')
    require(not os.path.lexists(DESTINATION), 'fresh canonical evidence destination required')
    resource.setrlimit(resource.RLIMIT_AS, (256 << 20, 256 << 20))
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_BODY, MAX_BODY))
    signal.alarm(120)
    os.umask(0o077)
    terminal_body = read(SOURCE / 'ar4' / TERMINAL_NAME, TERMINAL)
    terminal = json.loads(terminal_body)
    require(type(terminal['raw']) is dict and len(terminal['raw']) <= 256
            and all(ordinary(name) and name not in ('complete.json', 'failed.json') for name in terminal['raw']),
            'bounded ordinary terminal raw map')
    expected = dict(ROOT_PINS)
    expected.update({'ar4/' + name: compact(row) for name, row in terminal['raw'].items()})
    expected['ar4/' + TERMINAL_NAME] = TERMINAL
    require(roster(SOURCE) == set(expected), 'closed actual nine-root and original case source roster')
    require(sum(row['bytes'] for row in expected.values()) <= MAX_TOTAL, 'bounded declared source extent')
    bodies = {name: read(SOURCE / name, row) for name, row in sorted(expected.items())}
    require(sum(map(len, bodies.values())) <= MAX_TOTAL, 'bounded actual source extent')
    terminal = validate(bodies)
    self_path = Path(__file__).resolve()
    bodies['retain.py'] = read(self_path)
    bodies['README.md'] = read(self_path.parent / 'README.md')
    manifest = dict(schema='ferric-guarded-mlp-model-gpu-retained-v1',
        files={name: pin(body) for name, body in sorted(bodies.items())}, original_terminal=TERMINAL,
        retained_original_source_files=len(expected), root_inputs=len(ROOT_PINS),
        case_files=len(terminal['raw']) + 1, raw_count=len(terminal['raw']), retention_verified=True,
        passed=terminal['passed'], original_errors=terminal['errors'], mode='ar4',
        native_attempts=terminal['native_attempts'], retries=terminal['retries'],
        native_completed_frames_observed=4 if terminal['passed'] else None,
        native_spawn_observed=terminal['native_spawn_observed'],
        gpu_execution_observed=terminal['gpu_execution_confirmed'], original_outcome_preserved=True,
        selected_external_readset_pin_metadata_retained=True, all_external_readset_bodies_retained=False,
        numerical_acceptance=False, full_model_acceptance=False, independent_full_model_reference=False,
        full_long_workload=False, performance_claim=False, production_authority=False,
        project_code_executed_by_retainer=False, original_validator_rerun_by_retainer=False)
    bodies['manifest.json'] = (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()
    require(len(bodies) == len(expected) + 3 and len(bodies) <= MAX_FILES
            and sum(map(len, bodies.values())) <= MAX_TOTAL, 'bounded exact retained closure')
    if terminal['passed']:
        require(len(expected) == 87 and len(bodies) == 90, 'conditional successful 90-member closure')
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
    print(json.dumps(dict(retention_verified=True, original_attempt_passed=terminal['passed'],
        destination=str(DESTINATION), members=len(bodies), pinned_members=len(manifest['files']),
        raw_count=len(terminal['raw']), expanded_bytes=sum(map(len, bodies.values())),
        manifest=pin(bodies['manifest.json']), original_terminal=TERMINAL,
        gpu_execution_performed_by_retainer=False), sort_keys=True))


if __name__ == '__main__':
    main()
