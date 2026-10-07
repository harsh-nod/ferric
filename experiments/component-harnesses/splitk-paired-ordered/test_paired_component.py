import types
import unittest
from unittest.mock import patch

import paired_component as driver


def require(condition, message):
    if not condition:
        raise ValueError(message)


def image_fixture():
    # Synthetic unit inputs, never native image provenance.
    images = {arm: (arm.encode('ascii'), driver.IMAGE_PINS[arm]) for arm in driver.ARMS}
    return images, {value[0]: value[1] for value in images.values()}


class DriverTests(unittest.TestCase):
    def exercise(self, mode='latency', fault=None):
        events, retained, sessions = [], [], []
        images, digests = image_fixture()
        if fault == 'image':
            images['paired'] = images['unpaired']
        records = {name: {'id': index + 1, 'data': b'zero'}
                   for index, name in enumerate(('a', 'nk', 'kn', 'output', 'partials'))}
        plans = {arm: [{'metadata': {}, 'tag': part} for part in ('partial', 'merge')]
                 for arm in driver.ARMS}
        if fault == 'arm-count':
            plans['paired'].pop()

        class Session:
            def __init__(self, _worker, selected_mode, device):
                self.frontier, self.epoch = 0, 0
                self.mode, self.device = selected_mode, device
                sessions.append(self)

            def configure(self):
                events.append('configure')

            def dispatch(self, selected, *, post_warmup, clock):
                events.append(('batch', tuple(item['tag'] for item in selected), post_warmup))
                if fault == 'dispatch':
                    raise ValueError('dispatch failure')
                self.frontier += len(selected) + (1 if fault == 'frontier' else 0)
                return {'mode': self.mode, 'dispatch_count': len(selected),
                        'host_dispatch_wall_ns': 90, 'ordered_worker_elapsed_ns': 40}

        def read(_worker, record, expected, name):
            events.append(('read', name, expected))
            if fault == name + '-parity':
                raise ValueError('parity failure')
            return {'guards_unchanged': True}

        checks = []

        def inputs(*_):
            checks.append('input')
            events.append('inputs')
            return {'stable': not (fault == 'input-drift' and len(checks) == 2)}

        def command(value, *, expected):
            self.assertEqual(expected, 'freed')
            events.append(('free', value['buffer']))
            return {'op': 'freed'}, b'x' if fault == 'free' else b''

        component = types.SimpleNamespace(
            fixtures=types.SimpleNamespace(K=12288, N=4096, SPLITS=8),
            digest=lambda raw: digests[raw], records_for=lambda _: records,
            allocate_shared=lambda *_: events.append('allocate'), check_inputs=inputs,
            require=require, reset_outputs=lambda *_: events.append('reset'),
            checked_read=read, TAIL=b'tail')
        with patch.object(driver, 'plans_for', side_effect=lambda *_: events.append('load') or plans):
            try:
                result = driver.run_component(component, types.SimpleNamespace(OrderedSession=Session),
                    types.SimpleNamespace(command=command),
                    types.SimpleNamespace(output_f32=b'output', partials_f32=b'partials'),
                    images['unpaired'], images['paired'], mode, 31,
                    retained.append, lambda: events.append('alive'))
            except ValueError:
                self.assertIsNotNone(fault)
                return events, retained, None
        self.assertIsNone(fault)
        return events, retained, result

    def test_exact_two_warmups_then_three_abba_blocks(self):
        cells = list(driver.schedule())
        self.assertEqual(len(cells), 16)
        self.assertEqual([row['arm'] for row in cells[:4]], list(driver.ARMS) * 2)
        for block in range(3):
            selected = cells[4 + block * 4:8 + block * 4]
            self.assertEqual([row['arm'] for row in selected], ['unpaired', 'paired', 'paired', 'unpaired'])
            self.assertEqual([row['position'] for row in selected], list(range(4)))
            self.assertTrue(all(row['block'] == block and row['phase'] == 'sample'
                                and row['comparison'] == 'unpaired-vs-paired' for row in selected))
        self.assertTrue(all(row['phase'] == 'warmup' and row['position'] is None
                            and row['comparison'] is None for row in cells[:4]))

    def test_configuration_precedes_allocation_and_load(self):
        events, _, result = self.exercise()
        self.assertEqual(events[:4], ['configure', 'allocate', 'load', 'inputs'])
        self.assertEqual(result['dispatch_packets'], 32)

    def test_every_arm_is_one_complete_two_packet_batch(self):
        events, retained, result = self.exercise()
        batches = [item for item in events if type(item) is tuple and item[0] == 'batch']
        self.assertEqual(len(batches), 16)
        self.assertTrue(all(item[1] == ('partial', 'merge') for item in batches))
        self.assertEqual([item[2] for item in batches], [False] * 4 + [True] * 12)
        self.assertEqual(events.count('alive'), 32)
        self.assertEqual(len(retained), 32)
        self.assertEqual(len(result['samples']), 16)

    def test_both_exact_references_checked_outside_dispatch(self):
        events, _, _ = self.exercise()
        start = events.index('alive')
        self.assertEqual(events[start:start + 6], ['alive', 'reset',
            ('batch', ('partial', 'merge'), False), 'alive',
            ('read', 'output', b'outputtail'), ('read', 'partials', b'partials')])
        self.assertEqual(events.count(('read', 'partials', b'partials')), 16)

    def test_modes_preserve_inputs_and_keep_non_gpu_timing_labels(self):
        for mode in ('latency', 'counters', 'ticks'):
            with self.subTest(mode=mode):
                _, _, result = self.exercise(mode)
                self.assertEqual(result['mode'], mode)
                self.assertEqual(result['performance_configuration']['profile'], mode != 'latency')
                self.assertEqual(result['inputs_before'], result['inputs_after'])
                self.assertNotIn('gpu_ns', result)
                self.assertIn('not GPU-only', result['worker_timing_scope'])
                self.assertEqual(result['timed_kernels_per_arm'], ['partial', 'merge'])

    def test_swapped_image_fails_before_configuration_or_allocation(self):
        events, retained, result = self.exercise(fault='image')
        self.assertEqual((events, retained, result), ([], [], None))

    def test_bad_arm_cannot_dispatch(self):
        events, _, _ = self.exercise(fault='arm-count')
        self.assertFalse(any(type(item) is tuple and item[0] == 'batch' for item in events))

    def test_dispatch_or_parity_failure_cannot_publish_verified_sample(self):
        for fault in ('dispatch', 'output-parity', 'partials-parity'):
            with self.subTest(fault=fault):
                events, retained, _ = self.exercise(fault=fault)
                self.assertFalse(any(type(item) is tuple and item[0] == 'free' for item in events))
                self.assertFalse(any(item['event'] == 'verified_sample' for item in retained))

    def test_input_drift_or_wrong_frontier_is_terminal_before_free(self):
        for fault in ('input-drift', 'frontier'):
            with self.subTest(fault=fault):
                events, _, _ = self.exercise(fault=fault)
                self.assertFalse(any(type(item) is tuple and item[0] == 'free' for item in events))

    def test_shared_allocations_are_freed_and_bad_free_is_terminal(self):
        events, _, _ = self.exercise()
        self.assertEqual([item for item in events if type(item) is tuple and item[0] == 'free'],
                         [('free', index) for index in range(1, 6)])
        self.exercise(fault='free')


class PlanTests(unittest.TestCase):
    def test_exact_images_symbols_geometry_and_pointer_access(self):
        images, digests = image_fixture()
        records = {name: {'id': index + 1} for index, name in enumerate(('a', 'kn', 'partials', 'output'))}
        calls = []

        def load(worker, raw, digest, symbol, buffers, scalars, groups):
            calls.append((worker, raw, digest, symbol, buffers, scalars, groups))
            return {'symbol': symbol}

        component = types.SimpleNamespace(require=require, digest=lambda raw: digests[raw],
            fixtures=types.SimpleNamespace(K=12288, N=4096, SPLITS=8),
            load_plan=load, PARTIAL='partial', MERGE='merge')
        worker = object()
        result = driver.plans_for(component, worker, records, images)
        self.assertEqual(set(result), set(driver.ARMS))
        self.assertEqual(len(calls), 4)
        for index, arm in enumerate(driver.ARMS):
            self.assertEqual(calls[index * 2], (worker, *images[arm], 'partial',
                [(records['a'], 'read'), (records['kn'], 'read'), (records['partials'], 'write')],
                (1, 4096, 12288, 1, 2), 2048))
            self.assertEqual(calls[index * 2 + 1], (worker, *images[arm], 'merge',
                [(records['partials'], 'read'), (records['output'], 'write')], (), 64))

    def test_image_keys_bytes_hashes_and_fixed_shape_are_closed(self):
        images, digests = image_fixture()
        component = types.SimpleNamespace(require=require, digest=lambda raw: digests.get(raw, 'bad'),
            fixtures=types.SimpleNamespace(K=12288, N=4096, SPLITS=8))
        invalid = [dict(images, extra=images['paired']), {'unpaired': images['unpaired']},
            dict(images, paired=(b'altered', driver.IMAGE_PINS['paired'])),
            dict(images, paired=(bytearray(b'paired'), driver.IMAGE_PINS['paired'])),
            dict(images, paired=list(images['paired'])),
            dict(images, paired=images['unpaired'])]
        for candidate in invalid:
            with self.subTest(candidate=candidate), self.assertRaises(ValueError):
                driver.validate_images(component, candidate)
        component.fixtures.K = 8192
        with self.assertRaises(ValueError):
            driver.validate_images(component, images)
