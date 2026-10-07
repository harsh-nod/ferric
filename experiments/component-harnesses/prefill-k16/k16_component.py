"""Unqualified component driver; caller must supply the owned native supervisor."""
import hashlib
import time

import fixtures
import slice_views

IMAGES = {
    'control': 'fdbf5f1999e709b695b2ef4399bfc44920df1a0b85a61c5fe1bbc41e719e5357',
    'paired': 'fdbf5f1999e709b695b2ef4399bfc44920df1a0b85a61c5fe1bbc41e719e5357',
}
SYMBOLS = {'control': 'ferric_qwen3_prefill_k16_control_bf16_r1',
           'paired': 'ferric_qwen3_prefill_k16_paired_bf16_r1'}
GUARD = b'\xd3\x6a\x95\x2c' * 16
ARMS = ('control', 'paired')
WARMUPS, BLOCKS, CELLS, PACKETS = 2, 3, 16, 16
require = slice_views.require


def digest(data):
    return hashlib.sha256(data).hexdigest()


def schedule():
    for index, tag in enumerate((5, 4)):
        for arm in ARMS:
            yield {'phase': 'warmup', 'block': index, 'position': None, 'arm': arm, 'projection': tag}
    for block in range(BLOCKS):
        for position, arm in enumerate(('control', 'paired', 'paired', 'control')):
            yield {'phase': 'sample', 'block': block, 'position': position, 'arm': arm, 'projection': 4}


def validate_images(images):
    require(type(images) is dict and set(images) == set(ARMS), 'exact matched image arms')
    for arm in ARMS:
        require(type(images[arm]) is tuple and len(images[arm]) == 2
                and type(images[arm][0]) is bytes and images[arm][1] == IMAGES[arm]
                and digest(images[arm][0]) == IMAGES[arm], 'exact actual component image bytes')
    require(images['control'] == images['paired'], 'both roots use the same emitted image')


def plans_for(worker, records, images):
    validate_images(images)
    plans = {}
    loaded_ids = set()
    for arm in ARMS:
        raw, image_hash = images[arm]
        loaded, payload = worker.command({'op': 'load_kernel', 'payload_bytes': len(raw),
            'object_sha256': list(bytes.fromhex(image_hash)), 'symbol': SYMBOLS[arm]}, raw, 'loaded_kernel')
        require(not payload and type(loaded.get('kernel')) is int and loaded['kernel'] > 0
                and loaded['kernel'] not in loaded_ids, 'distinct positive loaded kernel handles')
        loaded_ids.add(loaded['kernel'])
        metadata = loaded['metadata']
        require(metadata['symbol'] == SYMBOLS[arm]
                and metadata['object_sha256'] == list(bytes.fromhex(image_hash)), 'loaded image/root identity')
        out = records['output']
        views = [slice_views.view(records['a'], 'read'), slice_views.view(records['kn'], 'read')]
        views.append(slice_views.view(out, 'write'))
        groups = 1536
        for tag in (4, 5):
            encoded, pointers = slice_views.encode(metadata, views, (32, 12288, 4096, 1, tag), groups)
            plans[(arm, tag)] = {'symbol': SYMBOLS[arm], 'metadata': metadata, 'encoded': encoded,
                'command': {'op': 'dispatch', 'kernel': loaded['kernel'], 'payload_bytes': len(encoded),
                    'workgroup': [64, 1, 1], 'grid': [groups * 64, 1, 1],
                    'pointers': pointers, 'timeout_ms': 30000}}
    return plans


def checked_read(worker, record, expected, name):
    data = worker.read(record['id'], len(record['data']) + 2 * len(GUARD))
    require(type(data) is bytes and len(data) == len(expected) + 2 * len(GUARD)
            and data[:len(GUARD)] == data[-len(GUARD):] == GUARD,
            name + ': complete unchanged outer guards')
    actual = data[len(GUARD):-len(GUARD)]
    fixtures.validate_output(actual, expected, name)
    return {'bytes': len(actual), 'sha256': digest(actual), 'guards_unchanged': True}


def check_inputs(worker, records):
    return {name: checked_read(worker, records[name], records[name]['data'], name) for name in ('a', 'kn')}


def run_component(adapter, worker, fixture, images, device, retain, check_alive,
                  clock=time.perf_counter_ns):
    validate_images(images)
    require(type(fixture) is fixtures.Fixture
            and (len(fixture.input_bf16), len(fixture.weights_kn_bf16), len(fixture.output_bf16))
            == (262144, 100663296, 786432), 'complete fixed K16 fixture')
    session = adapter.OrderedSession(worker, 'latency', device)
    session.configure()
    records = {name: {'data': data, 'element_bytes': 2} for name, data in
               (('a', fixture.input_bf16), ('kn', fixture.weights_kn_bf16), ('output', b'\xa5' * 786432))}
    ids = set()
    for record in records.values():
        identifier = worker.allocate(GUARD + record['data'] + GUARD)
        require(type(identifier) is int and identifier > 0 and identifier not in ids, 'distinct allocation handles')
        record['id'] = identifier
        ids.add(identifier)
    plans = plans_for(worker, records, images)
    before = check_inputs(worker, records)
    samples = []
    for cell in schedule():
        check_alive()
        output = records['output']
        header, payload = worker.command({'op': 'write', 'buffer': output['id'], 'offset': len(GUARD),
            'payload_bytes': len(output['data'])}, output['data'], 'written')
        require(not payload and header.get('op') == 'written', 'complete output reset')
        timing = {**cell, **session.dispatch([plans[(cell['arm'], cell['projection'])]],
            post_warmup=cell['phase'] == 'sample', clock=clock)}
        retain({'event': 'completed_unchecked', **timing})
        check_alive()
        checked = checked_read(worker, output, fixture.output_bf16, 'output')
        row = {**timing, 'output_check': checked}
        retain({'event': 'verified_sample', **row})
        samples.append(row)
    require(len(samples) == CELLS and session.frontier == PACKETS, 'complete16-cell16-packet campaign')
    after = check_inputs(worker, records)
    require(before == after, 'input bytes changed')
    for record in records.values():
        header, payload = worker.command({'op': 'free', 'buffer': record['id']}, expected='freed')
        require(not payload and header.get('op') == 'freed', 'allocation released')
    return {'mode': 'latency', 'inputs_before': before, 'inputs_after': after, 'samples': samples,
        'metadata': {arm: plans[(arm, 4)]['metadata'] for arm in ARMS},
        'allocation_ids': {name: record['id'] for name, record in records.items()},
        'allocation_reuse': 'one A, one KN weight and one shared BF16 output allocation',
        'dispatch_packets': session.frontier, 'queue_epoch': session.epoch,
        'warmups_per_arm': WARMUPS, 'warmup_projection_tags': [5, 4], 'timed_projection_tag': 4,
        'blocks': BLOCKS, 'timed_kernels_per_arm': 1,
        'host_timing_scope': 'encoding, process check, trace and synchronous ordered IPC; excludes reset and parity reads',
        'worker_timing_scope': 'ordered publication, completion and exit fence; not GPU-only',
        'control_compiler_matches_candidate': True,
        'unique_code_objects': 1,
        'b_register_reuse_established': False,
        'model_performance_qualified': False}
