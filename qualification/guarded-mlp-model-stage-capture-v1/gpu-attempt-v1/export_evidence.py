"""Authenticate and retain an actual capture outcome as data; no native launch or project imports."""
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

REMOTE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-model-stage-capture-gpu-v228-v1'
TERMINAL = dict(bytes=97034, sha256='324b40f7dcee931c856aec8b353bd35176e02570290aca5376c0075cfdf3be12')
TERMINAL_NAME = 'complete.json'
DOCUMENTATION = dict(bytes=3864, sha256='461130c0c311b8b70f0335d689bfad0cefabf03f9a1ebb51811ca213a072971a')
ROOT_PINS = {
    'ar4-input.json': dict(bytes=1557, sha256='43123ddac94fed45726e4b977b43ec53b038b79bf11b23bc973c9357e8d0fff1'),
    'ar4-request.json': dict(bytes=9335, sha256='69c299f5f8c33353817d4515d065447152a9268b6e139e92fededb2d96caf7ef'),
    'frozen_owned.py': dict(bytes=30433, sha256='ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'),
    'library_audit.py': dict(bytes=25872, sha256='b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'),
    'prepare_model_inputs.py': dict(bytes=13416, sha256='d701a3c772fe4e5086d44109c1d098b7948d7033a0a742295f6710563448f4e1'),
    'prepared-inputs.json': dict(bytes=12409, sha256='140d42509085e890e012b213b56f3d3b4876405c96b63356abc6aa713d16f992'),
    'run_model_gpu.py': dict(bytes=34531, sha256='afa4d007153179533136ae7a5be4de860407522cb6b92ebcde12441b2eabe175'),
    'validate_observation.py': dict(bytes=13779, sha256='367d1904f15741138db708c1f10e5a777905b62d9440a961e952a6cc9125b055'),
    'guarded_announcement.py': dict(bytes=3251, sha256='96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80'),
}
ADMISSION = {
    'parent_cpu': dict(bytes=3787634, sha256='f1dd4cf50c704bc1f39472df3a8299872de5490fcf073160077888629192ee91'),
    'parent': dict(bytes=13673168, sha256='f77b78a963d20fac64d4ce0c7ffd6f0d028d2d147aa4957dc47160abb4cfe25d'),
    'worker_cpu': dict(bytes=1607204, sha256='64855967eb15f5b5b5fadb98cbc30e8e382968099ec439090562cacc174eec47'),
    'worker': dict(bytes=5785096, sha256='c747212d88ef54532194792bdcc2d2dbe35610abdd54d39ce2affe6ec57653e7'),
}
IDS = [16366993098680759275, 10838076764495710945]
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
    names |= {'initial-topology.json', 'observation.json', 'capture-observation.json', 'native/complete.json', 'native/child-stderr.bin'}
    names |= {side + '-' + str(index) + '-topology.json' for side in ('before', 'after') for index in range(3)}
    names |= {'native/' + role + '-' + str(index) + '.' + suffix for index in range(4)
              for role, suffix in (('request', 'json'), ('control', 'bin'), ('observation', 'bin'))}
    require(len(names) == 78, 'fixed successful raw census')
    return names


def validate(bodies):
    terminal = parse(bodies['ar4/' + TERMINAL_NAME])
    require(terminal['schema'] == 'ferric-guarded-mlp-model-stage-capture-gpu-v1' and terminal['mode'] == 'ar4'
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
    plan, request, prepared = (parse(bodies[name]) for name in
                              ('ar4-input.json', 'ar4-request.json', 'prepared-inputs.json'))
    require(compact(terminal['plan']) == ROOT_PINS['ar4-input.json']
            and terminal['plan']['path'] == REMOTE + '/ar4-input.json'
            and plan['schema'] == 'ferric-guarded-mlp-model-stage-capture-gpu-input-v1' and plan['mode'] == 'ar4'
            and plan['request']['path'] == REMOTE + '/ar4-request.json'
            and compact(plan['request']) == ROOT_PINS['ar4-request.json']
            and request['decode']['mode'] == 'autoregressive'
            and request['decode']['evidence_directory'] == REMOTE + '/ar4/native', 'actual AR4 plan/request')
    require(prepared['schema'] == 'ferric-guarded-mlp-model-stage-capture-input-preparation-v1'
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
            and terminal['capture_requested'] is True and terminal['capture_verified'] is True, 'full successful AR4 closure')
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
    validate_capture(bodies, terminal)
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


class CaptureFields:
    PAYLOAD_BYTES = 606976
    parse = staticmethod(parse)

    @staticmethod
    def keys(value, names):
        require(type(value) is dict and set(value) == set(names.split()), 'closed capture fields')

    @staticmethod
    def uint(value, maximum=(1 << 64) - 1):
        require(type(value) is int and 0 <= value <= maximum, 'unsigned capture integer')
        return value

    @staticmethod
    def octets(value, count=32):
        require(type(value) is list and len(value) == count, 'exact capture octet extent')
        return bytes(CaptureFields.uint(v, 255) for v in value)

    @staticmethod
    def rust_pin(value):
        CaptureFields.keys(value, 'path bytes sha256')
        return dict(path=value['path'], bytes=CaptureFields.uint(value['bytes']),
                    sha256=CaptureFields.octets(value['sha256']).hex())


def capture_admission(summary_raw, read_body, validator):
    """Additional capture checks after the ordinary model validator accepts this summary."""
    summary = validator.parse(summary_raw)
    pin = validator.rust_pin(summary['files']['child_stderr'])
    raw = read_body(pin)
    require(0 < len(raw) <= 1100000 and len(raw) == pin['bytes']
            and hashlib.sha256(raw).hexdigest() == pin['sha256'], 'nonempty exact bounded capture stderr')
    envelope = validator.parse(raw)
    validator.keys(envelope, 'schema profile_sha256 registration_sha256 session device_ids '
                   'completed_forwards native_closed sampling capture numerical_acceptance '
                   'performance_claim production_authority')
    require(envelope['schema'] == 'FerricFiniteGuardedMlpLayerZeroCaptureV1'
            and validator.octets(envelope['profile_sha256']) == validator.octets(summary['profile_sha256'])
            and validator.octets(envelope['registration_sha256']) == validator.octets(summary['registration_sha256'])
            and validator.octets(envelope['session']) == validator.octets(summary['request']['decode']['session'])
            and envelope['device_ids'] == IDS and all(type(v) is int for v in envelope['device_ids'])
            and validator.uint(envelope['completed_forwards']) == 4 and envelope['native_closed'] is True
            and envelope['sampling'] == 'prefix-boundaries-and-post-paired-retained'
            and all(envelope[k] is False for k in ('numerical_acceptance', 'performance_claim', 'production_authority')),
            'closed capture identity, completed workload and nonclaims')
    capture = envelope['capture']
    validator.keys(capture, 'schema generation position layer parts payload_bytes payload_sha256 payload '
                   'full_cache_capture native_close_confirmed numerical_acceptance performance_claim production_authority')
    require(capture['schema'] == 'FerricFiniteLayerZeroCaptureV1'
            and [validator.uint(capture[k]) for k in ('generation', 'position', 'layer')] == [1, 0, 0]
            and type(capture['parts']) is list and len(capture['parts']) == 34
            and validator.uint(capture['payload_bytes']) == 256136
            and capture['native_close_confirmed'] is True
            and all(capture[k] is False for k in ('full_cache_capture', 'numerical_acceptance',
                                                'performance_claim', 'production_authority')),
            'exact capture layout and lifetime scope')
    payload = validator.octets(capture['payload'], 256136)
    require(hashlib.sha256(payload).digest() == validator.octets(capture['payload_sha256']), 'whole capture digest')
    first = summary['files']['frames'][0]
    request = validator.parse(read_body(validator.rust_pin(first['request'])))
    observation = read_body(validator.rust_pin(first['observation']))
    command = request['command']
    metadata = struct.pack('<145I', *command['cache_metadata'])
    rotary = struct.pack('<128I', *command['rotary_bits'])
    require(request['id'] == command['generation'] == 1 and command['cache_metadata'][0] == 0
            and len(observation) == validator.PAYLOAD_BYTES, 'already validated first-forward capture join')
    stages = (
        ('before_prefix', (('input', 'bf16', 8192), ('cache_metadata', 'u32', 580), ('rotary', 'f32', 512))),
        ('after_prefix', (('input_normalized', 'bf16', 8192), ('raw_qkv', 'bf16', 6144),
                          ('query', 'bf16', 4096), ('current_key', 'bf16', 1024), ('current_value', 'bf16', 1024),
                          ('attention', 'bf16', 4096), ('output_partial', 'f32', 16384))),
        ('after_first_residual', (('first_residual', 'bf16', 8192),)),
        ('after_mlp', (('post_normalized', 'bf16', 8192), ('gate', 'bf16', 12288), ('up', 'bf16', 12288),
                      ('activation', 'bf16', 12288), ('down_partial', 'f32', 16384))),
        ('after_final_residual', (('final_hidden', 'bf16', 8192),)),
    )
    offset = ordinal = 0
    for boundary, specs in stages:
        for rank in (0, 1):
            for role, scalar, size in specs:
                part = capture['parts'][ordinal]
                validator.keys(part, 'boundary rank role scalar elements offset bytes source_byte_offset sha256')
                width = 2 if scalar == 'bf16' else 4
                source_offset = command['cache_metadata'][1] * 16384 if role in ('current_key', 'current_value') else 0
                require((part['boundary'], validator.uint(part['rank']), part['role'], part['scalar'],
                         validator.uint(part['elements']), validator.uint(part['offset']), validator.uint(part['bytes']),
                         validator.uint(part['source_byte_offset']))
                        == (boundary, rank, role, scalar, size // width, offset, size, source_offset),
                        'closed ordered capture part')
                data = payload[offset:offset + size]
                require(hashlib.sha256(data).digest() == validator.octets(part['sha256']), 'capture part digest')
                if scalar in ('bf16', 'f32'):
                    mask = 0x7f80 if width == 2 else 0x7f800000
                    require(all(int.from_bytes(data[i:i + width], 'little') & mask != mask
                                for i in range(0, size, width)), 'finite captured scalar')
                if role == 'cache_metadata':
                    require(data == metadata, 'capture actual first metadata')
                elif role == 'rotary':
                    require(data == rotary, 'capture actual first rotary')
                elif role == 'final_hidden':
                    require(data == observation[:8192], 'capture actual first layer output')
                offset += size
                ordinal += 1
    require(ordinal == 34 and offset == len(payload), 'complete capture coverage')
    return dict(schema='ferric-guarded-mlp-model-stage-capture-checked-v1', source=pin,
                generation=1, position=0, layer=0, parts=34, payload_bytes=len(payload),
                payload_sha256=hashlib.sha256(payload).hexdigest(), native_close_confirmed=True,
                same_run_request_and_layer_output_joined=True,
                sampling='prefix-boundaries-and-post-paired-retained',
                independent_framework_comparison=False, numerical_acceptance=False,
                performance_claim=False, production_authority=False)


def validate_capture(bodies, terminal):
    def raw_read(expected):
        path = expected['path']
        require(path.startswith(REMOTE + '/ar4/native/'), 'capture native path custody')
        name = path[len(REMOTE) + 1:]
        require(ordinary(name) and name in bodies and pin(bodies[name]) == compact(expected),
                'capture original retained body')
        return bodies[name]
    checked = capture_admission(bodies['ar4/native/complete.json'], raw_read, CaptureFields)
    require(same(checked, terminal['capture_observation'])
            and same(checked, parse(bodies['ar4/capture-observation.json'])),
            'same original and retained capture observation')


def bindings():
    require(type(TERMINAL) is dict and set(TERMINAL) == {'bytes', 'sha256'}
            and type(TERMINAL['bytes']) is int and 0 < TERMINAL['bytes'] <= MAX_BODY
            and type(TERMINAL['sha256']) is str and re.fullmatch('[0-9a-f]{64}', TERMINAL['sha256'])
            and TERMINAL_NAME in ('complete.json', 'failed.json'), 'actual terminal remains unbound')
    require(type(DOCUMENTATION) is dict and set(DOCUMENTATION) == {'bytes', 'sha256'},
            'known retention documentation binding')


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
    return dict(schema='ferric-guarded-mlp-model-stage-capture-retained-v1',
        files={name: pin(body) for name, body in sorted(bodies.items())}, original_terminal=TERMINAL,
        retained_original_source_files=original_count, root_inputs=len(ROOT_PINS),
        case_files=len(terminal['raw']) + 1, raw_count=len(terminal['raw']), retention_verified=True,
        passed=terminal['passed'], original_errors=terminal['errors'], mode='ar4',
        native_attempts=terminal['native_attempts'], retries=terminal['retries'],
        native_completed_frames_observed=4 if terminal['passed'] else None,
        native_spawn_observed=terminal['native_spawn_observed'],
        gpu_execution_observed=terminal['gpu_execution_confirmed'], original_outcome_preserved=True,
        capture_requested=terminal['capture_requested'], capture_verified_observed=terminal['capture_verified'],
        capture_structure_rechecked=terminal['passed'],
        selected_external_readset_pin_metadata_retained=True, all_external_readset_bodies_retained=False,
        numerical_acceptance=False, full_model_acceptance=False, independent_full_model_reference=False,
        full_long_workload=False, performance_claim=False, production_authority=False,
        native_execution_by_retention=False, original_model_validator_rerun_by_retention=False)


SOURCE = Path(REMOTE)
ARCHIVE_OUT = SOURCE.parent / 'guarded-mlp-model-stage-capture-gpu-evidence-v228-v1.tar.gz'


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B export_evidence.py')
    bindings(); limits()
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged data export host')
    require(ARCHIVE_OUT.parent.resolve(strict=True) == ARCHIVE_OUT.parent
            and not os.path.lexists(ARCHIVE_OUT), 'fresh external archive destination')
    terminal_body = read(SOURCE / 'ar4' / TERMINAL_NAME, TERMINAL)
    expected = expected_inputs(parse(terminal_body))
    require(roster(SOURCE) == set(expected), 'closed nine-root and original case source roster')
    bodies = {name: read(SOURCE / name, row) for name, row in sorted(expected.items())}
    require(sum(map(len, bodies.values())) <= MAX_TOTAL, 'bounded actual source extent')
    terminal = validate(bodies)
    self_path = Path(__file__).resolve()
    require(not self_path.is_relative_to(SOURCE), 'export tooling stays outside immutable case root')
    bodies['export_evidence.py'] = read(self_path)
    documentation = self_path.parent / 'RETENTION.md'
    bodies['RETENTION.md'] = read(documentation, DOCUMENTATION)
    manifest = manifest_for(bodies, terminal, len(expected))
    bodies['manifest.json'] = (json.dumps(manifest, indent=2, sort_keys=True, allow_nan=False) + '\n').encode()
    require(len(bodies) == len(expected) + 3 and len(bodies) <= MAX_FILES
            and sum(map(len, bodies.values())) <= MAX_TOTAL, 'bounded exact capsule closure')
    if terminal['passed']:
        require(len(expected) == 88 and len(bodies) == 91, 'conditional successful91-member closure')
    require(roster(SOURCE) == set(expected), 'source roster before archive')
    for name, row in expected.items(): read(SOURCE / name, row)
    read(self_path, pin(bodies['export_evidence.py'])); read(documentation, DOCUMENTATION)
    with ARCHIVE_OUT.open('xb') as outer:
        with gzip.GzipFile(filename='', fileobj=outer, mode='wb', mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode='w|', format=tarfile.USTAR_FORMAT) as archive:
                for name, body in sorted(bodies.items()):
                    require(ordinary(name), 'ordinary archive member')
                    info = tarfile.TarInfo(name); info.size = len(body); info.mode = 0o600
                    info.uid = info.gid = info.mtime = 0
                    archive.addfile(info, io.BytesIO(body))
        outer.flush(); os.fsync(outer.fileno())
    actual_archive = archive_pin(ARCHIVE_OUT)
    require(roster(SOURCE) == set(expected), 'source roster after archive')
    for name, row in expected.items(): read(SOURCE / name, row)
    read(self_path, pin(bodies['export_evidence.py'])); read(documentation, DOCUMENTATION)
    print(json.dumps(dict(export_verified=True, original_attempt_passed=terminal['passed'],
        archive=dict(path=str(ARCHIVE_OUT), **actual_archive), members=len(bodies),
        pinned_members=len(manifest['files']), raw_count=len(terminal['raw']),
        expanded_bytes=sum(map(len, bodies.values())), original_terminal=TERMINAL,
        gpu_execution_performed_by_exporter=False), sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()
