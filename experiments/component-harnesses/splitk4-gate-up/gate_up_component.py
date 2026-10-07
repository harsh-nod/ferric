"""C1 gate/up component driver; requires an externally owned native supervisor."""
import hashlib
import time

import fixtures
import slice_views

IMAGES = {
    'wave': '98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502',
    'splitk4': 'd28610d291eeec0589afbf269e26d21b7111c96f08106e1f661d6a66f024bf03',
}
SYMBOLS = {'wave': slice_views.WAVE, 'partial': slice_views.PARTIAL, 'merge': slice_views.MERGE}
GUARD = b'\xd3\x6a\x95\x2c' * 16
SCRATCH_POISON = b'\xad\xde\xc1\x7f' * 49152
ARMS = ('wave', 'splitk4')
WARMUPS, BLOCKS, CELLS, PACKETS = 2, 3, 16, 24
require = slice_views.require


def digest(data):
    return hashlib.sha256(data).hexdigest()


def schedule():
    for index, tag in enumerate((5, 4)):
        for arm in ARMS:
            yield {'phase': 'warmup', 'block': index, 'position': None, 'arm': arm, 'projection': tag}
    for block in range(BLOCKS):
        for position, arm in enumerate(('wave', 'splitk4', 'splitk4', 'wave')):
            yield {'phase': 'sample', 'block': block, 'position': position, 'arm': arm, 'projection': 4}


def validate_images(images):
    require(type(images) is dict and set(images) == set(ARMS), 'exact Wave/split-K4 images')
    for arm in ARMS:
        require(type(images[arm]) is tuple and len(images[arm]) == 2
                and type(images[arm][0]) is bytes and images[arm][1] == IMAGES[arm]
                and digest(images[arm][0]) == IMAGES[arm], 'exact actual component image bytes')


def plans_for(worker, records, images):
    validate_images(images)
    require(set(records) == {'a', 'nk', 'kn', 'scratch', 'output'}
            and len({record['id'] for record in records.values()}) == 5,
            'five distinct complete allocations')
    plans, metadata, loaded_ids = {}, {}, set()
    for role in ('wave', 'partial', 'merge'):
        image_arm = 'wave' if role == 'wave' else 'splitk4'
        raw, image_hash = images[image_arm]
        loaded, payload = worker.command({'op': 'load_kernel', 'payload_bytes': len(raw),
            'object_sha256': list(bytes.fromhex(image_hash)), 'symbol': SYMBOLS[role]}, raw, 'loaded_kernel')
        require(not payload and type(loaded.get('kernel')) is int and loaded['kernel'] > 0
                and loaded['kernel'] not in loaded_ids, 'distinct positive loaded kernel handles')
        loaded_ids.add(loaded['kernel'])
        meta = loaded['metadata']
        require(meta['symbol'] == SYMBOLS[role]
                and meta['object_sha256'] == list(bytes.fromhex(image_hash)), 'loaded image/root identity')
        metadata[role] = meta
        keys = {'wave': ('a', 'nk', 'output'), 'partial': ('a', 'kn', 'scratch'),
                'merge': ('scratch', 'output')}[role]
        views = [slice_views.view(records[key], 'write' if index == len(keys) - 1 else 'read')
                 for index, key in enumerate(keys)]
        groups = {'wave': 12288, 'partial': 3072, 'merge': 192}[role]
        for tag in (4, 5):
            scalars = () if role == 'merge' else (1, 12288, 4096, 1, tag)
            encoded, pointers = slice_views.encode(meta, views, scalars, groups)
            plans[(role, tag)] = {'symbol': SYMBOLS[role], 'metadata': meta, 'encoded': encoded,
                'command': {'op': 'dispatch', 'kernel': loaded['kernel'], 'payload_bytes': len(encoded),
                    'workgroup': [64, 1, 1], 'grid': [groups * 64, 1, 1],
                    'pointers': pointers, 'timeout_ms': 30000}}
    frames = {}
    for tag in (4, 5):
        frames[('wave', tag)] = [plans[('wave', tag)]]
        frames[('splitk4', tag)] = [plans[('partial', tag)], plans[('merge', tag)]]
    return frames, metadata


def checked_read(worker, record, expected, name):
    data = worker.read(record['id'], len(record['data']) + 2 * len(GUARD))
    require(type(data) is bytes and len(data) == len(expected) + 2 * len(GUARD)
            and data[:len(GUARD)] == data[-len(GUARD):] == GUARD,
            name + ': complete unchanged outer guards')
    actual = data[len(GUARD):-len(GUARD)]
    fixtures.validate_output(actual, expected, name)
    return {'bytes': len(actual), 'sha256': digest(actual), 'guards_unchanged': True}


def check_inputs(worker, records):
    return {name: checked_read(worker, records[name], records[name]['data'], name)
            for name in ('a', 'nk', 'kn')}


def run_component(adapter, worker, fixture, images, device, retain, check_alive,
                  clock=time.perf_counter_ns):
    validate_images(images)
    require(type(fixture) is fixtures.Fixture
            and tuple(map(len, (fixture.input_bf16, fixture.weights_nk_bf16,
                               fixture.weights_kn_bf16, fixture.partials_f32, fixture.output_bf16)))
            == (8192, 100663296, 100663296, 196608, 24576), 'complete fixed C1 gate/up fixture')
    session = adapter.OrderedSession(worker, 'latency', device)
    session.configure()
    records = {name: {'data': data, 'element_bytes': width} for name, data, width in (
        ('a', fixture.input_bf16, 2), ('nk', fixture.weights_nk_bf16, 2),
        ('kn', fixture.weights_kn_bf16, 2), ('scratch', SCRATCH_POISON, 4),
        ('output', b'\xa5' * fixtures.OUTPUT_CAPACITY_BYTES, 2))}
    ids = set()
    for record in records.values():
        identifier = worker.allocate(GUARD + record['data'] + GUARD)
        require(type(identifier) is int and identifier > 0 and identifier not in ids, 'distinct allocation handles')
        record['id'] = identifier
        ids.add(identifier)
    frames, metadata = plans_for(worker, records, images)
    before = check_inputs(worker, records)
    expected_output = fixture.output_bf16 + records['output']['data'][len(fixture.output_bf16):]
    samples = []
    for cell in schedule():
        check_alive()
        for name in ('output', 'scratch'):
            record = records[name]
            header, payload = worker.command({'op': 'write', 'buffer': record['id'], 'offset': len(GUARD),
                'payload_bytes': len(record['data'])}, record['data'], 'written')
            require(not payload and header.get('op') == 'written', 'complete ' + name + ' reset')
        timing = {**cell, **session.dispatch(frames[(cell['arm'], cell['projection'])],
            post_warmup=cell['phase'] == 'sample', clock=clock)}
        retain({'event': 'completed_unchecked', **timing})
        check_alive()
        output_check = checked_read(worker, records['output'], expected_output, 'output with untouched tail')
        expected_scratch = SCRATCH_POISON if cell['arm'] == 'wave' else fixture.partials_f32
        scratch_check = checked_read(worker, records['scratch'], expected_scratch, 'scratch')
        row = {**timing, 'output_check': output_check, 'scratch_check': scratch_check}
        retain({'event': 'verified_sample', **row})
        samples.append(row)
    require(len(samples) == CELLS and session.frontier == PACKETS, 'complete16-cell24-packet campaign')
    after = check_inputs(worker, records)
    require(before == after, 'input bytes changed')
    for record in records.values():
        header, payload = worker.command({'op': 'free', 'buffer': record['id']}, expected='freed')
        require(not payload and header.get('op') == 'freed', 'allocation released')
    return {'mode': 'latency', 'inputs_before': before, 'inputs_after': after, 'samples': samples,
        'metadata': metadata, 'allocation_ids': {name: record['id'] for name, record in records.items()},
        'allocation_reuse': 'one A, distinct NK/KN weights, one FP32 scratch and one BF16 output',
        'scratch_bytes': 196608, 'output_active_bytes': 24576, 'output_untouched_tail_bytes': 761856,
        'dispatch_packets': session.frontier, 'queue_epoch': session.epoch,
        'warmups_per_arm': WARMUPS, 'warmup_projection_tags': [5, 4], 'timed_projection_tag': 4,
        'blocks': BLOCKS, 'packets_per_cell': {'wave': 1, 'splitk4': 2},
        'timed_dispatches_per_arm': {'wave': 6, 'splitk4': 12},
        'host_timing_scope': 'encoding, process check, trace and synchronous ordered IPC; excludes reset and parity reads',
        'worker_timing_scope': 'ordered publication, completion and exit fence; not GPU-only',
        'control_compiler_matches_candidate': False, 'model_performance_qualified': False}
