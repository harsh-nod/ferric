"""Source-only paired/unpaired split-K driver using the frozen component ABI."""
import time


IMAGE_PINS = {
    'unpaired': '1b16379c91c945883bfc9aecdbde896853573a92546acd228eba6e232d746cae',
    'paired': '950618ad101779973b4051190266cfdd601329405691cb215296fe61b84278ec',
}
ARMS = ('unpaired', 'paired')
WARMUPS = 2
BLOCKS = 3
CELLS = 16
PACKETS = 32


def schedule():
    for index in range(WARMUPS):
        for arm in ARMS:
            yield {'phase': 'warmup', 'block': index, 'comparison': None,
                   'position': None, 'arm': arm}
    for block in range(BLOCKS):
        for position, arm in enumerate(('unpaired', 'paired', 'paired', 'unpaired')):
            yield {'phase': 'sample', 'block': block, 'comparison': 'unpaired-vs-paired',
                   'position': position, 'arm': arm}


def validate_images(component, images):
    component.require(type(images) is dict and set(images) == set(ARMS), 'exact split-K image arms')
    component.require((component.fixtures.K, component.fixtures.N, component.fixtures.SPLITS)
                      == (12288, 4096, 8), 'unchanged fixed down fixture')
    for arm in ARMS:
        image = images[arm]
        component.require(type(image) is tuple and len(image) == 2 and type(image[0]) is bytes
                          and image[1] == IMAGE_PINS[arm], 'exact historical split-K image binding')
        component.require(component.digest(image[0]) == image[1], 'actual split-K image bytes')


def plans_for(component, worker, records, images):
    validate_images(component, images)
    scalars = (1, component.fixtures.N, component.fixtures.K, 1, 2)
    plans = {}
    for arm in ARMS:
        image = images[arm]
        plans[arm] = [
            component.load_plan(worker, *image, component.PARTIAL,
                [(records['a'], 'read'), (records['kn'], 'read'), (records['partials'], 'write')],
                scalars, 2048),
            component.load_plan(worker, *image, component.MERGE,
                [(records['partials'], 'read'), (records['output'], 'write')], (), 64),
        ]
    return plans


def run_component(component, adapter, worker, fixture, unpaired, paired, mode, device,
                  retain=lambda _row: None, check_alive=lambda: None,
                  clock=time.perf_counter_ns):
    images = {'unpaired': unpaired, 'paired': paired}
    validate_images(component, images)
    session = adapter.OrderedSession(worker, mode, device)
    session.configure()
    records = component.records_for(fixture)
    component.allocate_shared(worker, records)
    plans = plans_for(component, worker, records, images)
    component.require(set(plans) == set(ARMS) and all(len(plans[arm]) == 2 for arm in ARMS),
                      'exact two complete partial-plus-merge arms')
    before = component.check_inputs(worker, records)
    samples = []
    for cell in schedule():
        check_alive()
        component.reset_outputs(worker, records)
        timing = {**cell, **session.dispatch(plans[cell['arm']],
            post_warmup=cell['phase'] == 'sample', clock=clock)}
        retain({'event': 'completed_unchecked', **timing})
        check_alive()
        output = component.checked_read(worker, records['output'],
            fixture.output_f32 + component.TAIL, 'output')
        partials = component.checked_read(worker, records['partials'], fixture.partials_f32, 'partials')
        row = {**timing, 'output_check': output, 'partial_check': partials}
        retain({'event': 'verified_sample', **row})
        samples.append(row)
    component.require(len(samples) == CELLS and session.frontier == PACKETS,
                      'fixed16-cell32-packet ABBA campaign')
    after = component.check_inputs(worker, records)
    component.require(before == after, 'input custody changed')
    for record in records.values():
        _, payload = worker.command({'op': 'free', 'buffer': record['id']}, expected='freed')
        component.require(not payload, 'free response payload')
    return {'mode': mode, 'inputs_before': before, 'inputs_after': after, 'samples': samples,
        'metadata': {arm: [plan['metadata'] for plan in entries] for arm, entries in plans.items()},
        'allocation_ids': {name: record['id'] for name, record in records.items()},
        'allocation_reuse': 'one fixed shared set; no allocation within timed samples',
        'dispatch_packets': session.frontier, 'queue_epoch': session.epoch,
        'performance_configuration': {'cache_kernel_admission': True,
            'operational_currentness': True, 'profile': mode != 'latency'},
        'counter_scope': (None if mode == 'latency' else
            'previous snapshot plus batch; overlapping durations are not additive'),
        'host_timing_scope': 'encoding, process check, trace and synchronous ordered IPC; excludes snapshots and parity reads',
        'worker_timing_scope': 'ordered publication, completion and exit fence; excludes preparation/staging; not GPU-only',
        'timed_kernels_per_arm': ['partial', 'merge'], 'warmups_per_arm': WARMUPS, 'blocks': BLOCKS}
