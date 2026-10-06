"""Authenticate and retain an actual host-observation outcome as data; no native launch or project imports."""
import hashlib
import gzip
import io
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import sys
import struct
import tarfile
import time

REMOTE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-model-host-observation-gpu-v228-v1'
TERMINAL = dict(bytes=614572, sha256='91a20d628644eb784d651e4d4d87571cb55cdb367971092fcea40d259ee38ec2')
TERMINAL_NAME = 'complete.json'
DOCUMENTATION = dict(bytes=4802, sha256='118d549ad3fb8a43a27b5d03391dc5b0a8d01835575e6918b1d670207465c86a')
ROOT_PINS = {
    'ar4-input.json': dict(bytes=1575, sha256='a2ef6a2660ddfb0fd59313455e321ed7a87a3395f28ca1ca71bc109c2e72f34e'),
    'ar4-request.json': dict(bytes=9339, sha256='dd4ee75e6a8f09cd65846a55542248b6f7cdba96813a00f38af4c30fcd827f21'),
    'frozen_owned.py': dict(bytes=30433, sha256='ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'),
    'library_audit.py': dict(bytes=25872, sha256='b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'),
    'prepare_model_inputs.py': dict(bytes=13728, sha256='c97a80ba8fac2683934578c67b9387ee17e047cf457fb76f59c3a7d6c2473569'),
    'prepared-inputs.json': dict(bytes=12511, sha256='4f40202f904d31606233efcd7902a9e86760e6a87e2e95d951b134265eb1f9b8'),
    'run_model_gpu.py': dict(bytes=38453, sha256='4c49f8053fdb5b676338b45b257d37280ec068a3f98cd37efb1aa0cc50c4f156'),
    'validate_observation.py': dict(bytes=13779, sha256='367d1904f15741138db708c1f10e5a777905b62d9440a961e952a6cc9125b055'),
    'guarded_announcement.py': dict(bytes=3251, sha256='96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80'),
}
ADMISSION = {
    'parent_cpu': dict(bytes=3815416, sha256='ab72e2316315ec6b0766284e0b4c26e834b2502cd4a6fc29c304a875950e3c8f'),
    'parent': dict(bytes=13836368, sha256='abf358b7fdcce0e2c432aa906d06c8790a3727d5471993ee01879d74a851f5c9'),
    'worker_cpu': dict(bytes=1625743, sha256='fd4b55f53e2a61aafb49bb901796ca6c5d72a2e34d725aabc16cd4d14cef082e'),
    'worker': dict(bytes=5873056, sha256='9221a902a52a07b96b6855c8b802d4e634cbc0970e3f1e252d4130cf6bc1753d'),
}
PRIOR_ROOT = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-model-gpu-v228-v3/ar4'
REFERENCE_PINS = {
    'reference/ordinary-ar4/complete.json': dict(path=PRIOR_ROOT + '/complete.json', bytes=93497,
        sha256='edf05cf2dd19a9934dab9762b328ea3e6160cf01c15606a3df3299ce967e4991'),
    'reference/ordinary-ar4/observation-0.bin': dict(path=PRIOR_ROOT + '/native/observation-0.bin', bytes=606976,
        sha256='1324a6056ea6c0308e912c63ed1b6a6d27e103c915c6dde11ff600d97cf9b1b0'),
    'reference/ordinary-ar4/observation-1.bin': dict(path=PRIOR_ROOT + '/native/observation-1.bin', bytes=606976,
        sha256='61f56a5b97e4f35922bf7733a74b36b8dec9544a15d66980757bf9ef5aefc6e4'),
    'reference/ordinary-ar4/observation-2.bin': dict(path=PRIOR_ROOT + '/native/observation-2.bin', bytes=606976,
        sha256='87d3799724693386909e97fd3ea31fab065de021824dea276218614f2a0fc85a'),
    'reference/ordinary-ar4/observation-3.bin': dict(path=PRIOR_ROOT + '/native/observation-3.bin', bytes=606976,
        sha256='7c40126b76a7eeb7a64ef67950636359ba951cafcdce2ddc42636388de3c5588'),
}
IDS = [16366993098680759275, 10838076764495710945]
LABELS = ('parent-readelf', 'parent-ldd', 'worker-readelf', 'worker-ldd',
          'before-0', 'before-1', 'before-2', 'parent', 'after-0', 'after-1', 'after-2')
FALSE_FLAGS = ('numerical_acceptance', 'full_model_acceptance', 'independent_full_model_reference',
               'full_long_workload', 'performance_claim', 'production_authority')
MAX_FILES, MAX_BODY, MAX_TOTAL = 274, 8 << 20, 72 << 20


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
    names |= {'initial-topology.json', 'observation.json', 'host-observation.json', 'instrumentation-comparison.json', 'native/complete.json', 'native/child-stderr.bin'}
    names |= {side + '-' + str(index) + '-topology.json' for side in ('before', 'after') for index in range(3)}
    names |= {'native/' + role + '-' + str(index) + '.' + suffix for index in range(4)
              for role, suffix in (('request', 'json'), ('control', 'bin'), ('observation', 'bin'))}
    require(len(names) == 79, 'fixed successful raw census')
    return names


def validate(bodies):
    terminal = parse(bodies['ar4/' + TERMINAL_NAME])
    require(terminal['schema'] == 'ferric-guarded-mlp-model-host-observation-gpu-v1' and terminal['mode'] == 'ar4'
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
    for name, row in REFERENCE_PINS.items():
        require(pin(bodies[name]) == compact(row) and terminal['readset'].get(row['path']) == row,
                'exact selected ordinary-AR4 reference body and original readset path')
    plan, request, prepared = (parse(bodies[name]) for name in
                              ('ar4-input.json', 'ar4-request.json', 'prepared-inputs.json'))
    require(compact(terminal['plan']) == ROOT_PINS['ar4-input.json']
            and terminal['plan']['path'] == REMOTE + '/ar4-input.json'
            and plan['schema'] == 'ferric-guarded-mlp-model-host-observation-gpu-input-v1' and plan['mode'] == 'ar4'
            and plan['request']['path'] == REMOTE + '/ar4-request.json'
            and compact(plan['request']) == ROOT_PINS['ar4-request.json']
            and request['decode']['mode'] == 'autoregressive'
            and request['decode']['evidence_directory'] == REMOTE + '/ar4/native', 'actual AR4 plan/request')
    require(prepared['schema'] == 'ferric-guarded-mlp-model-host-observation-input-preparation-v1'
            and prepared['passed'] is True and set(prepared['generated']) == {'ar4'}
            and prepared['native_execution'] is False and prepared['gpu_execution'] is False
            and prepared['controller']['path'] == REMOTE + '/prepare_model_inputs.py'
            and compact(prepared['controller']) == ROOT_PINS['prepare_model_inputs.py'], 'actual data preparation')
    for role, suffix in (('plan', 'input'), ('request', 'request')):
        row = prepared['generated']['ar4'][role]
        require(row['path'] == REMOTE + '/ar4-' + suffix + '.json'
                and compact(row) == ROOT_PINS['ar4-' + suffix + '.json'], 'prepared body join')
    for role in ('parent', 'parent_cpu', 'worker', 'worker_cpu'):
        require(compact(plan[role]) == ADMISSION[role]
                and plan[role] == terminal['admission'][role]
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
        require(same(parse(bodies['ar4/' + label + '/result.json']),
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
            and terminal['partial_gpu_execution_possible'] is False
            and terminal['host_observation_requested'] is True and terminal['host_observation_verified'] is True, 'full successful AR4 closure')
    for phase in terminal['phases']:
        label = phase['label']
        command = parse(bodies['ar4/' + label + '/command.json'])
        started = parse(bodies['ar4/' + label + '/started.json'])
        require(phase['exit_code'] == 0 and phase['reason'] is None
                and phase['cleanup_signalled'] is False and phase['owned_groups_absent'] is True
                and phase['owned_processes_reaped'] is True
                and phase['gpu_execution_requested'] is (label == 'parent')
                and command['gpu_execution_requested'] is (label == 'parent')
                and started['command_sha256'] == phase['command']['sha256']
                and phase['lineage'][0]['event'] == 'owned'
                and phase['lineage'][0]['reason'] == 'spawned-parent'
                and phase['lineage'][0]['identity'] == started['parent'], 'natural owned phase')
    checked = parse(bodies['ar4/observation.json'])
    require(same(checked, terminal['observation']) and checked['mode'] == 'autoregressive'
            and checked['local_bank_generations'] == [1, 1, 2, 2]
            and checked['retained_parent_files'] == 14 and checked['captured_bf16_words'] == 1213952
            and checked['layer_observations'] == 144 and checked['rank_guard_observations'] == 288
            and checked['actual_lowest_index_argmax_checked'] is True
            and all(checked[key] is False for key in FALSE_FLAGS if key != 'full_long_workload'),
            'original qualified validator observation, not an independent model reference')
    native = parse(bodies['ar4/native/complete.json'])
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
    started = parse(bodies['ar4/parent/started.json'])
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
    require(bodies['ar4/parent/stderr'] == expected_stderr, 'exact guarded announcement')
    validate_host(bodies, terminal)
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
        require(parse(bodies['ar4/' + label + '/stdout']) == [dict(gpu=index,
            process_list=[dict(process_info='No running processes detected')]) for index in range(8)],
            'recorded all-eight-GPU idle observation')
        require(same(parse(bodies['ar4/' + label + '-topology.json']), terminal['platform']), 'recorded stable platform')
    require(same(parse(bodies['ar4/initial-topology.json']), terminal['platform']), 'initial platform join')
    return terminal



def parse(raw):
    def pairs(rows):
        value = {}
        for key, item in rows:
            require(key not in value, 'duplicate JSON member')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


class HostFields:
    PAYLOAD_BYTES = 606976
    parse = staticmethod(parse)

    @staticmethod
    def keys(value, names):
        require(type(value) is dict and set(value) == set(names.split()), 'closed host fields')

    @staticmethod
    def uint(value, maximum=(1 << 64) - 1):
        require(type(value) is int and 0 <= value <= maximum, 'unsigned host integer')
        return value

    @staticmethod
    def octets(value, count=32):
        require(type(value) is list and len(value) == count, 'exact host octet extent')
        return bytes(HostFields.uint(v, 255) for v in value)

    @staticmethod
    def rust_pin(value):
        HostFields.keys(value, 'path bytes sha256')
        return dict(path=value['path'], bytes=HostFields.uint(value['bytes']),
                    sha256=HostFields.octets(value['sha256']).hex())


def host_admission(summary_raw, read_body, validator):
    """Additional host-only checks after ordinary model validation and owned-lineage admission."""
    summary = validator.parse(summary_raw)
    pin = validator.rust_pin(summary['files']['child_stderr'])
    raw = read_body(pin)
    require(0 < len(raw) <= 2 << 20 and len(raw) == pin['bytes']
            and hashlib.sha256(raw).hexdigest() == pin['sha256'], 'bounded exact host report')
    report = validator.parse(raw)
    validator.keys(report, 'schema bootstrap worker_sha256 child_pid profile_sha256 snapshots intervals '
                   'forward_host_ns close_host_ns completions native_closed inclusive_nested_host_scopes '
                   'paired_generic_dispatch_timers_complete tensor_stage_capture gpu_time gpu_overlap '
                   'numerical_acceptance full_model_acceptance performance_claim production_authority')
    same = lambda a, b: json.dumps(a, sort_keys=True, separators=(',', ':'), allow_nan=False) == json.dumps(
        b, sort_keys=True, separators=(',', ':'), allow_nan=False)
    require(report['schema'] == 'FerricGuardedMlpHostObservationV1'
            and same(report['bootstrap'], summary['bootstrap'])
            and validator.octets(report['worker_sha256']) == validator.octets(summary['request']['decode']['worker']['sha256'])
            and validator.uint(report['child_pid'], 0xffffffff) == summary['child_pid']
            and validator.octets(report['profile_sha256']) == validator.octets(summary['profile_sha256'])
            and report['native_closed'] is True and report['inclusive_nested_host_scopes'] is True
            and all(report[k] is False for k in ('paired_generic_dispatch_timers_complete', 'tensor_stage_capture',
                'gpu_time', 'gpu_overlap', 'numerical_acceptance', 'full_model_acceptance',
                'performance_claim', 'production_authority')), 'host identity, Close, policy and nonclaims')
    completions = [{k: v for k, v in row['response']['event'].items() if k != 'status'}
                   for row in summary['files']['frames']]
    require(type(report['completions']) is list and len(report['completions']) == 4
            and same(report['completions'], completions), 'host actual four wire completions')
    snapshots, intervals = report['snapshots'], report['intervals']
    require(type(snapshots) is list and len(snapshots) == 587
            and type(intervals) is list and len(intervals) == 586, 'closed host observation census')
    def vector(value, count):
        require(type(value) is list and len(value) == count, 'host counter vector extent')
        return [validator.uint(v) for v in value]
    def phase(index):
        if index == 0: return 'fresh_enabled'
        if index == 1: return 'setup_sealed'
        if index == 586: return 'before_close'
        forward, at = divmod(index - 2, 146)
        if at == 0: return 'forward_%d/begin' % forward
        if at == 145: return 'forward_%d/done' % forward
        layer, step = divmod(at - 1, 4)
        return 'forward_%d/layer_%02d/%s' % (forward, layer, ('begin', 'prefix', 'paired', 'hidden')[step])
    group = None; epochs = None
    for index, current in enumerate(snapshots):
        validator.keys(current, 'phase group_incarnation shared_full_currentness ranks shared')
        incarnation = validator.uint(current['group_incarnation'])
        require(incarnation > 0 and current['phase'] == phase(index)
                and current['shared_full_currentness'] is False
                and type(current['ranks']) is list and len(current['ranks']) == 2,
                'host group, fixed phase and conservative policy')
        shared = vector(current['shared'], 4)
        current_epochs = []
        for rank, row in enumerate(current['ranks']):
            validator.keys(row, 'rank unique_id queue_epoch cache_kernel_admission raw_timestamp_queue counters')
            counters = vector(row['counters'], 19)
            require(validator.uint(row['rank']) == rank and validator.uint(row['unique_id']) == IDS[rank]
                    and row['cache_kernel_admission'] is False and row['raw_timestamp_queue'] is False
                    and counters[4:6] == [0, 0], 'host rank and unchanged runtime policy')
            current_epochs.append(validator.uint(row['queue_epoch']))
        if index == 0:
            group, epochs = incarnation, current_epochs
            require(shared == [0] * 4 and all(row['counters'] == [0] * 19 for row in current['ranks']),
                    'host freshly enabled zero baseline')
        require(incarnation == group and current_epochs == epochs, 'host group or queue epoch drift')
        if index:
            delta = intervals[index - 1]
            validator.keys(delta, 'host_elapsed_ns ranks shared')
            validator.uint(delta['host_elapsed_ns'])
            require(type(delta['ranks']) is list and len(delta['ranks']) == 2, 'two host rank deltas')
            previous = snapshots[index - 1]
            expected_shared = [a - b for a, b in zip(shared, previous['shared'])]
            require(vector(delta['shared'], 4) == expected_shared, 'host shared subtraction or counter decrease')
            for rank in range(2):
                expected_rank = [a - b for a, b in zip(current['ranks'][rank]['counters'],
                                                     previous['ranks'][rank]['counters'])]
                require(vector(delta['ranks'][rank], 19) == expected_rank, 'host rank subtraction or counter decrease')
    forwards = vector(report['forward_host_ns'], 4)
    close_ns = validator.uint(report['close_host_ns'])
    rows = []
    for forward in range(4):
        start = 2 + forward * 146
        elapsed = sum(v['host_elapsed_ns'] for v in intervals[start:start + 145])
        require(elapsed <= (1 << 64) - 1 and forwards[forward] <= elapsed, 'host forward elapsed bound')
        layers = []
        for layer in range(36):
            begin = start + 1 + 4 * layer
            layers.append(dict(layer=layer, prefix=intervals[begin], paired=intervals[begin + 1],
                               hidden_read=intervals[begin + 2]))
        rows.append(dict(position=forward, forward_host_ns=forwards[forward],
                         bracket_host_ns=elapsed, layers=layers))
    counter_names = ['commands', 'command_ns', 'full_currentness_checks', 'full_currentness_ns',
        'operational_currentness_checks', 'operational_currentness_ns', 'kernel_admissions',
        'kernel_admission_ns', 'dispatches', 'dispatch_prepare_ns', 'dispatch_publish_ns',
        'dispatch_wait_ns', 'completion_polls', 'reads', 'read_bytes', 'read_ns', 'writes',
        'write_bytes', 'write_ns']
    shared_names = ['group_full_checks', 'group_full_ns', 'publication_full_checks', 'publication_full_ns']
    return dict(schema='ferric-guarded-mlp-model-host-observation-checked-v1', source=pin,
        snapshots=587, intervals=586, counter_names=counter_names, shared_counter_names=shared_names,
        forward_rows=rows, close_host_ns=close_ns,
        final_rank_counters=[row['counters'] for row in snapshots[-1]['ranks']],
        final_shared_counters=snapshots[-1]['shared'], same_run_completions_joined=True,
        native_close_confirmed=True, inclusive_nested_host_scopes=True,
        interval_wall_includes_host_gpu_waits_and_process_gaps=True,
        paired_generic_dispatch_timers_complete=False, tensor_stage_capture=False,
        gpu_time=False, gpu_overlap=False, throughput=False, numerical_acceptance=False,
        full_model_acceptance=False, performance_claim=False, production_authority=False)


def compare_payloads(summary_raw, read_body, validator, prior):
    summary = validator.parse(summary_raw)
    prior_pin, prior_inputs, values = prior
    rows = []
    for index, (old_pin, old) in enumerate(values):
        current = validator.rust_pin(summary['files']['frames'][index]['observation'])
        raw = read_body(current)
        require(len(raw) == current['bytes'] == len(old) == validator.PAYLOAD_BYTES
                and hashlib.sha256(raw).hexdigest() == current['sha256'], 'current observation body')
        rows.append(dict(position=index, current=current, ordinary=old_pin, byte_equal=raw == old,
                         same_history=summary['input_tokens'][:index + 1] == prior_inputs[:index + 1]))
    return dict(schema='ferric-guarded-mlp-host-instrumentation-comparison-v1', ordinary_terminal=prior_pin,
        frames=rows, all_payloads_equal=all(row['byte_equal'] for row in rows),
        all_histories_equal=all(row['same_history'] for row in rows),
        observed_payload_difference=any(not row['byte_equal'] for row in rows),
        causal_effect_established=False,
        comparison_completed=True, independent_accuracy_reference=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)


def validate_host(bodies, terminal):
    def raw_read(expected):
        path = expected['path']
        require(path.startswith(REMOTE + '/ar4/native/'), 'host native path custody')
        name = path[len(REMOTE) + 1:]
        require(ordinary(name) and name in bodies and pin(bodies[name]) == compact(expected),
                'host original retained body')
        return bodies[name]
    checked = host_admission(bodies['ar4/native/complete.json'], raw_read, HostFields)
    require(same(checked, terminal['host_observation'])
            and same(checked, parse(bodies['ar4/host-observation.json'])),
            'same original and retained host observation')
    prior_name = 'reference/ordinary-ar4/complete.json'
    prior = parse(bodies[prior_name])
    require(prior['passed'] is True and prior['errors'] == [] and prior['mode'] == 'ar4'
            and prior['native_attempts'] == 1 and prior['retries'] == 0, 'actual earlier ordinary AR4')
    values = []
    for index in range(4):
        name = 'reference/ordinary-ar4/observation-%d.bin' % index
        value = REFERENCE_PINS[name]
        require(value == prior['raw']['native/observation-%d.bin' % index],
                'prior retained payload and terminal pin join')
        values.append((value, bodies[name]))
    comparison = compare_payloads(bodies['ar4/native/complete.json'], raw_read, HostFields,
        (REFERENCE_PINS[prior_name], prior['observation']['input_tokens'], values))
    require(same(comparison, terminal['instrumentation_comparison'])
            and same(comparison, parse(bodies['ar4/instrumentation-comparison.json'])),
            'actual payload/history comparison recomputed, equality is not presumed')


def bindings():
    require(type(TERMINAL) is dict and set(TERMINAL) == {'bytes', 'sha256'}
            and type(TERMINAL['bytes']) is int and 0 < TERMINAL['bytes'] <= MAX_BODY
            and type(TERMINAL['sha256']) is str and re.fullmatch('[0-9a-f]{64}', TERMINAL['sha256'])
            and TERMINAL_NAME in ('complete.json', 'failed.json'), 'actual terminal remains unbound')
    require(type(DOCUMENTATION) is dict and set(DOCUMENTATION) == {'bytes', 'sha256'},
            'known retention documentation binding')
    require(all(type(row) is dict and set(row) == {'bytes', 'sha256'}
                and type(row['bytes']) is int and 0 < row['bytes'] <= MAX_BODY
                and type(row['sha256']) is str and re.fullmatch('[0-9a-f]{64}', row['sha256'])
                for row in ROOT_PINS.values()), 'actual current root input bindings remain pending')


def limits():
    os.umask(0o077)
    for kind, cap in ((resource.RLIMIT_AS, 256 << 20), (resource.RLIMIT_FSIZE, MAX_TOTAL),
                      (resource.RLIMIT_CPU, 120), (resource.RLIMIT_CORE, 0)):
        soft, hard = resource.getrlimit(kind)
        limit = min([cap] + [v for v in (soft, hard) if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    def expired(_number, _frame):
        raise RuntimeError('data retention120s deadline')
    signal.signal(signal.SIGALRM, expired)
    signal.setitimer(signal.ITIMER_REAL, 120)


def expected_inputs(terminal):
    require(type(terminal['raw']) is dict and len(terminal['raw']) <= 256
            and all(ordinary(name) and name not in ('complete.json', 'failed.json') for name in terminal['raw']),
            'bounded original terminal raw closure')
    expected = dict(ROOT_PINS)
    expected.update({name: compact(row) for name, row in REFERENCE_PINS.items()})
    expected.update({'ar4/' + name: compact(row) for name, row in terminal['raw'].items()})
    expected['ar4/' + TERMINAL_NAME] = TERMINAL
    require(len(expected) <= MAX_FILES - 3 and sum(row['bytes'] for row in expected.values()) <= MAX_TOTAL,
            'bounded declared source extent')
    return expected


def archive_pin(path, expected=None):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_nlink == 1 and 0 < before.st_size <= MAX_TOTAL, 'bounded ordinary archive')
    digest = hashlib.sha256(); size = 0
    with path.open('rb') as stream:
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'archive open identity')
        while True:
            chunk = stream.read(1 << 20)
            if not chunk: break
            size += len(chunk)
            require(size <= MAX_TOTAL, 'growing archive bound')
            digest.update(chunk)
        require(stamp(os.fstat(stream.fileno())) == stamp(before), 'archive changed while read')
    actual = dict(bytes=size, sha256=digest.hexdigest())
    require(stamp(path.lstat()) == stamp(before) and size == before.st_size
            and (expected is None or actual == expected), 'archive exact bytes and final identity')
    return actual


def manifest_for(bodies, terminal, original_count):
    return dict(schema='ferric-guarded-mlp-model-host-observation-retained-v1',
        files={name: pin(body) for name, body in sorted(bodies.items())}, original_terminal=TERMINAL,
        retained_original_source_files=original_count - len(REFERENCE_PINS), root_inputs=len(ROOT_PINS),
        selected_ordinary_reference_bodies=len(REFERENCE_PINS), ordinary_reference_pins=REFERENCE_PINS,
        case_files=len(terminal['raw']) + 1, raw_count=len(terminal['raw']), retention_verified=True,
        passed=terminal['passed'], original_errors=terminal['errors'], mode='ar4',
        native_attempts=terminal['native_attempts'], retries=terminal['retries'],
        native_completed_frames_observed=4 if terminal['passed'] else None,
        native_spawn_observed=terminal['native_spawn_observed'],
        gpu_execution_observed=terminal['gpu_execution_confirmed'], original_outcome_preserved=True,
        host_observation_requested=terminal['host_observation_requested'],
        host_observation_verified_observed=terminal['host_observation_verified'],
        host_structure_rechecked=terminal['passed'], instrumentation_comparison_rechecked=terminal['passed'],
        inclusive_nested_host_scopes=True, gpu_time=False, gpu_overlap=False, throughput=False,
        selected_external_readset_pin_metadata_retained=True, all_external_readset_bodies_retained=False,
        numerical_acceptance=False, full_model_acceptance=False, independent_full_model_reference=False,
        full_long_workload=False, performance_claim=False, production_authority=False,
        native_execution_by_retention=False, original_model_validator_rerun_by_retention=False)


ARCHIVE = Path('/mnt/c/Users/harmenon/ferric-session-evidence/20261004/guarded-mlp-model-host-observation-gpu-evidence-v228-v1.tar.gz')
ARCHIVE_PIN = dict(bytes=3929492, sha256='1bebcaec4eb2dd74600193981173d25b2da39ac0df861c48d36b04e3ca001d31')
EXPORTER = dict(bytes=40075, sha256='f1b8e5989eb425345949951b8ac4c2514e158980bdd9d088d8707698507795f3')
DESTINATION = Path('/home/harsh/ferric-p227-integration/qualification/guarded-mlp-model-host-observation-v1/gpu-attempt-v1')


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B retain_evidence.py')
    bindings(); limits()
    require(type(ARCHIVE_PIN) is dict and set(ARCHIVE_PIN) == {'bytes', 'sha256'}
            and type(EXPORTER) is dict and set(EXPORTER) == {'bytes', 'sha256'},
            'actual archive and bound exporter remain pending')
    require(not os.path.lexists(DESTINATION), 'fresh canonical evidence destination required')
    archive_pin(ARCHIVE, ARCHIVE_PIN)
    self_path = Path(__file__).resolve(); self_pin = pin(read(self_path))
    bodies = {}; total = 0
    with tarfile.open(ARCHIVE, mode='r:gz') as archive:
        for member in archive:
            require(len(bodies) < MAX_FILES and ordinary(member.name) and member.name not in bodies
                    and member.isfile() and not member.issym() and not member.islnk()
                    and not member.pax_headers and 0 <= member.size <= MAX_BODY,
                    'closed ordinary bounded archive member')
            total += member.size
            require(total <= MAX_TOTAL, 'archive expanded body bound')
            stream = archive.extractfile(member)
            require(stream is not None, 'ordinary archive body')
            body = stream.read(MAX_BODY + 1)
            require(len(body) == member.size, 'exact archive member extent')
            bodies[member.name] = body
    archive_pin(ARCHIVE, ARCHIVE_PIN)
    require('manifest.json' in bodies and len(bodies) <= MAX_FILES, 'retention manifest present')
    manifest_body = bodies.pop('manifest.json'); manifest = parse(manifest_body)
    require(set(manifest['files']) == set(bodies) and all(pin(body) == manifest['files'][name]
            for name, body in bodies.items()), 'complete manifest body closure')
    require(pin(bodies['export_evidence.py']) == EXPORTER and pin(bodies['RETENTION.md']) == DOCUMENTATION,
            'actual exporter and fixed retention documentation')
    terminal = parse(bodies['ar4/' + TERMINAL_NAME])
    expected = expected_inputs(terminal)
    require(set(bodies) == set(expected) | {'export_evidence.py', 'RETENTION.md'}
            and all(pin(bodies[name]) == row for name, row in expected.items()),
            'exact original sources, raw prefix and terminal')
    terminal = validate({name: bodies[name] for name in expected})
    require(same(manifest, manifest_for(bodies, terminal, len(expected))), 'exact manifest outcome and nonclaims')
    bodies['manifest.json'] = manifest_body
    if terminal['passed']:
        require(len(expected) == 94 and len(bodies) == 97, 'conditional successful97-member closure')
    require(len(bodies) == len(expected) + 3 and sum(map(len, bodies.values())) <= MAX_TOTAL,
            'closed retained capsule extent')
    read(self_path, self_pin); archive_pin(ARCHIVE, ARCHIVE_PIN)
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
    require(roster(DESTINATION) == set(bodies), 'complete retained roster')
    for name, body in bodies.items(): read(DESTINATION / name, pin(body))
    read(self_path, self_pin); archive_pin(ARCHIVE, ARCHIVE_PIN)
    print(json.dumps(dict(retention_verified=True, original_attempt_passed=terminal['passed'],
        destination=str(DESTINATION), members=len(bodies), pinned_members=len(manifest['files']),
        raw_count=len(terminal['raw']), expanded_bytes=sum(map(len, bodies.values())),
        manifest=pin(manifest_body), original_terminal=TERMINAL, archive=ARCHIVE_PIN,
        gpu_execution_performed_by_retainer=False), sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()
