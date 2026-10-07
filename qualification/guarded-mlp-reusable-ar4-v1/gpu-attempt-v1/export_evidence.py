"""Authenticate and retain an actual explicit reusable AR4 outcome as data; no native launch or project imports."""
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

REMOTE = '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/guarded-mlp-reusable-ar4-gpu-v228-v1'
TERMINAL = dict(bytes=105254, sha256='05d695b62b76c00a6598ff69a08d9d3290710ffdc189f63d3ef2b738d521afbd')
TERMINAL_NAME = 'complete.json'
DOCUMENTATION = dict(bytes=5875, sha256='71c2a19ae3b0d96d18e19e45b81f5b9a038220ac1dd1ee79597ce5011a3c59ee')
ROOT_PINS = {
    'ar4-input.json': dict(bytes=1539, sha256='3b1db3249727bcd363ce585efeace612f46c703cf5caf264a5e3384eed09a00e'),
    'ar4-request.json': dict(bytes=9332, sha256='a90d75ed02d7f69802917651615a7903ccbd56c76a6ab1cef3b0f596b930302e'),
    'frozen_owned.py': dict(bytes=30433, sha256='ab41af455396322303bb8ebd4132e680affc3bdc5830608fad36736c92024583'),
    'library_audit.py': dict(bytes=25872, sha256='b26d60ef6dbe24f2182b52b2e0eb44632848061b7a4a1735fc4e1d26b8ee638d'),
    'prepare_model_inputs.py': dict(bytes=14493, sha256='9406be145e0dcd3cfbab8ce587d8014ea8626067fdabc0f23890af65564d0060'),
    'prepared-inputs.json': dict(bytes=12322, sha256='020db9d8e0528e1ff80989d5282b901dd631ab1b4b1a88b790c91c72f41d336d'),
    'run_model_gpu.py': dict(bytes=34836, sha256='c506df18aa71978fdf81eb93b422ebc39c03f832c935c10f4698b5fa25121fa3'),
    'validate_observation.py': dict(bytes=15574, sha256='9d0f860d0174ef5574c3023253b4e220ff16bfba845ad13c7e0ce210cdc66ed7'),
    'guarded_announcement.py': dict(bytes=3251, sha256='96f84389071ef2c1a9e354705ebb7144e9ff97e30c242bd9b494f03d2aea6b80'),
}
ADMISSION = {
    'parent_cpu': dict(bytes=3796001, sha256='f24fab8b524afc87726d6aec918abce01dc39643f56c7ac7f323af4c4ce953f2'),
    'parent': dict(bytes=13859224, sha256='3d02886f0d36ebcd73d34797e0046eeb850eff0c49a1cff0e7ac16dd46c6fa91'),
    'worker_cpu': dict(bytes=1631185, sha256='24feb83a0bde60dc9c6b8db28f2ce8252379b40d8e0d40483690f58038881b2a'),
    'worker': dict(bytes=5912952, sha256='a5c5c8b323e2d88ec27df1065709171fb9bd46114aff99b083b63d3644743fb8'),
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
SEED = [9112, 2190, 3772, 220]
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
    names |= {'initial-topology.json', 'observation.json', 'ordinary-comparison.json', 'native/complete.json', 'native/child-stderr.bin'}
    names |= {side + '-' + str(index) + '-topology.json' for side in ('before', 'after') for index in range(3)}
    names |= {'native/' + role + '-' + str(index) + '.' + suffix for index in range(4)
              for role, suffix in (('request', 'json'), ('control', 'bin'), ('observation', 'bin'))}
    require(len(names) == 78, 'fixed successful raw census')
    return names


def validate(bodies):
    terminal = parse(bodies['ar4/' + TERMINAL_NAME])
    require(terminal['schema'] == 'ferric-guarded-mlp-reusable-ar4-gpu-v1' and terminal['mode'] == 'ar4'
            and type(terminal['passed']) is bool and terminal['retries'] == 0
            and type(terminal['native_attempts']) is int and terminal['native_attempts'] in (0, 1)
            and all(terminal[key] is False for key in FALSE_FLAGS)
            and terminal['reusable_arena_requested'] is True
            and type(terminal['actual_arena_plateau_verified']) is bool
            and terminal['host_observation_requested'] is False
            and terminal['shared_full_currentness_requested'] is False
            and terminal['default_full_currentness_requested'] is True
            and terminal['performance_policy_changed'] is False
            and terminal['baseline_payload_equality_required'] is True
            and terminal['timing_comparison_performed'] is False and terminal['speedup_claim'] is False,
            'original bounded reusable outcome and authority')
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
            and plan['schema'] == 'ferric-guarded-mlp-reusable-ar4-gpu-input-v1' and plan['mode'] == 'ar4'
            and plan['request']['path'] == REMOTE + '/ar4-request.json'
            and compact(plan['request']) == ROOT_PINS['ar4-request.json']
            and request['schema'] == 'FerricFiniteGuardedMlpReusableAr4RequestV1'
            and request['decode']['mode'] == 'autoregressive'
            and request['decode']['evidence_directory'] == REMOTE + '/ar4/native', 'actual AR4 plan/request')
    require(prepared['schema'] == 'ferric-guarded-mlp-reusable-ar4-input-preparation-v1'
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
            ('tf4_data_revalidation', dict(bytes=79327, sha256='d0551f310a57f63dfe98af8c887b5996752e3b62128a3087c4a03849082e6a3f')),
            ('census_cpu', dict(bytes=1513, sha256='a1f048f2a32ccb44f5c2d7f6263faa3620a4eaf130a2c55222c8e2c32461f5a8'))):
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
            and terminal['actual_arena_plateau_verified'] is True, 'full successful reusable AR4 closure')
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
            and native['schema'] == 'FerricFiniteGuardedMlpReusableAr4ObservationV1'
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
    validate_reuse(bodies, terminal)
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


def keys(value, names):
    require(type(value) is dict and set(value) == set(names.split()), 'closed object fields')


def uint(value, maximum=(1 << 64) - 1):
    require(type(value) is int and 0 <= value <= maximum, 'unsigned integer')
    return value


def octets(value, count=32):
    require(type(value) is list and len(value) == count, 'byte-array extent')
    return bytes(uint(v, 255) for v in value)


def profile(bootstrap):
    require(bootstrap['schema'] == 'FerricGuardedMlpReusableAr4BootstrapV1',
            'explicit reusable bootstrap')
    b = bootstrap['decode']; s = b['scope']
    h = hashlib.sha256(b'ferric-prefix284-combined2208-four-decode-v1\0')
    for value in (s['bundle_id'], s['model_id'], s['session'], b['registration'],
                  b['prefix_image']['sha256'], b['tiles_image']['sha256'],
                  bootstrap['projection_image']['sha256'], bootstrap['guarded_image']['sha256']):
        h.update(octets(value))
    for value in (s['pool_identity'], s['group_id']):
        h.update(struct.pack('<Q', uint(value)))
    h.update(struct.pack('<II', uint(s['child_identity'], 0xffffffff), uint(b['timeout_ms'], 10000)))
    require(b['mode'] == 'autoregressive', 'reusable route requires autoregressive input')
    ar = b['mode'] == 'autoregressive'
    require(b['input_tokens'] == (SEED[:1] if ar else SEED), 'profile seeds')
    h.update(bytes([ar]))
    for value in b['device_ids']:
        h.update(struct.pack('<Q', uint(value)))
    for value in ([SEED[0], 0, 0, 0] if ar else SEED):
        h.update(struct.pack('<I', value))
    return hashlib.sha256(b'ferric-guarded-mlp-reusable-ar4-profile-v1\0' + h.digest()).digest()


def arena_census(raw, bootstrap):
    require(0 < len(raw) <= 4096, 'bounded reusable arena census')
    value = parse(raw)
    keys(value, 'schema profile_sha256 registration session device_ids allocation_counts '
                'completed_forwards native_closed performance_claim')
    decode = bootstrap['decode']
    require(value['schema'] == 'FerricGuardedMlpReusableAr4ArenaCensusV1'
            and octets(value['profile_sha256']) == profile(bootstrap)
            and octets(value['registration']) == octets(decode['registration'])
            and octets(value['session']) == octets(decode['scope']['session'])
            and value['device_ids'] == decode['device_ids'] == IDS
            and all(type(item) is int for item in value['device_ids'])
            and uint(value['completed_forwards']) == 4
            and value['native_closed'] is True and value['performance_claim'] is False,
            'reusable census profile, identity and healthy Close')
    counts = value['allocation_counts']
    require(type(counts) is list and len(counts) == 5, 'five actual census samples')
    for row in counts:
        require(type(row) is list and len(row) == 2, 'two-rank census extent')
        for count in row:
            uint(count, 2048)
    require(counts == [[715, 711], [751, 747], [787, 783], [787, 783], [787, 783]],
            'actual arena first uses and plateau')
    return value


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
    return dict(schema='ferric-guarded-mlp-reusable-ar4-comparison-v1', ordinary_terminal=prior_pin,
        frames=rows, all_payloads_equal=all(row['byte_equal'] for row in rows),
        all_histories_equal=all(row['same_history'] for row in rows),
        observed_payload_difference=any(not row['byte_equal'] for row in rows),
        causal_effect_established=False,
        comparison_completed=True, independent_accuracy_reference=False, numerical_acceptance=False,
        performance_claim=False, production_authority=False)


def validate_reuse(bodies, terminal):
    def raw_read(expected):
        path = expected['path']
        require(path.startswith(REMOTE + '/ar4/native/'), 'reusable native path custody')
        name = path[len(REMOTE) + 1:]
        require(ordinary(name) and name in bodies and pin(bodies[name]) == compact(expected),
                'reusable original retained body')
        return bodies[name]
    native = parse(bodies['ar4/native/complete.json'])
    census_pin = HostFields.rust_pin(native['files']['child_stderr'])
    census = arena_census(raw_read(census_pin), native['bootstrap'])
    checked = terminal['observation']
    require(octets(native['profile_sha256']) == profile(native['bootstrap'])
            and checked['actual_arena_plateau_verified'] is True
            and same(census, checked['actual_arena_census'])
            and same(terminal['actual_allocation_counts'], census['allocation_counts']),
            'independently rechecked actual census, bootstrap profile and plateau')
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
    require(same(comparison, terminal['ordinary_ar4_comparison'])
            and same(comparison, parse(bodies['ar4/ordinary-comparison.json']))
            and comparison['all_payloads_equal'] is True and comparison['all_histories_equal'] is True,
            'actual all-four payload/history equality independently rechecked')


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
    return dict(schema='ferric-guarded-mlp-reusable-ar4-retained-v1',
        files={name: pin(body) for name, body in sorted(bodies.items())}, original_terminal=TERMINAL,
        retained_original_source_files=original_count - len(REFERENCE_PINS), root_inputs=len(ROOT_PINS),
        selected_ordinary_reference_bodies=len(REFERENCE_PINS), ordinary_reference_pins=REFERENCE_PINS,
        case_files=len(terminal['raw']) + 1, raw_count=len(terminal['raw']), retention_verified=True,
        passed=terminal['passed'], original_errors=terminal['errors'], mode='ar4',
        native_attempts=terminal['native_attempts'], retries=terminal['retries'],
        native_completed_frames_observed=4 if terminal['passed'] else None,
        native_spawn_observed=terminal['native_spawn_observed'],
        gpu_execution_observed=terminal['gpu_execution_confirmed'], original_outcome_preserved=True,
        reusable_arena_requested=terminal['reusable_arena_requested'],
        actual_arena_plateau_verified_observed=terminal['actual_arena_plateau_verified'],
        actual_allocation_counts=terminal['actual_allocation_counts'],
        host_observation_requested=False, shared_full_currentness_requested=False,
        default_full_currentness_requested=True, performance_policy_changed=False,
        census_rechecked=terminal['passed'], ordinary_payload_comparison_rechecked=terminal['passed'],
        baseline_payload_equality_required=True, gpu_time=False, gpu_overlap=False, throughput=False,
        timing_comparison_performed=False, speedup_claim=False,
        selected_external_readset_pin_metadata_retained=True, all_external_readset_bodies_retained=False,
        numerical_acceptance=False, full_model_acceptance=False, independent_full_model_reference=False,
        full_long_workload=False, performance_claim=False, production_authority=False,
        native_execution_by_retention=False, original_model_validator_rerun_by_retention=False)


SOURCE = Path(REMOTE)
ARCHIVE_OUT = SOURCE.parent / 'guarded-mlp-reusable-ar4-gpu-evidence-v228-v1.tar.gz'


def source_path(name):
    return Path(REFERENCE_PINS[name]['path']) if name in REFERENCE_PINS else SOURCE / name


def main():
    require(__debug__ and sys.dont_write_bytecode and len(sys.argv) == 1, 'python3 -B export_evidence.py')
    bindings(); limits()
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == 'smci350-rck-g03-b19-03',
            'exact unprivileged data export host')
    require(ARCHIVE_OUT.parent.resolve(strict=True) == ARCHIVE_OUT.parent
            and not os.path.lexists(ARCHIVE_OUT), 'fresh external archive destination')
    terminal_body = read(SOURCE / 'ar4' / TERMINAL_NAME, TERMINAL)
    expected = expected_inputs(parse(terminal_body))
    require(roster(SOURCE) == set(expected) - set(REFERENCE_PINS), 'closed nine-root and original case source roster')
    bodies = {name: read(source_path(name), row) for name, row in sorted(expected.items())}
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
        require(len(expected) == 93 and len(bodies) == 96, 'conditional successful96-member closure')
    require(roster(SOURCE) == set(expected) - set(REFERENCE_PINS), 'source roster before archive')
    for name, row in expected.items(): read(source_path(name), row)
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
    require(roster(SOURCE) == set(expected) - set(REFERENCE_PINS), 'source roster after archive')
    for name, row in expected.items(): read(source_path(name), row)
    read(self_path, pin(bodies['export_evidence.py'])); read(documentation, DOCUMENTATION)
    print(json.dumps(dict(export_verified=True, original_attempt_passed=terminal['passed'],
        archive=dict(path=str(ARCHIVE_OUT), **actual_archive), members=len(bodies),
        pinned_members=len(manifest['files']), raw_count=len(terminal['raw']),
        expanded_bytes=sum(map(len, bodies.values())), original_terminal=TERMINAL,
        gpu_execution_performed_by_exporter=False), sort_keys=True))
    signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()
