import hashlib
import struct
import types
import unittest
from unittest import mock

import fixtures
import k16_component as driver
from test_component_inputs import metadata


class Worker:
    def __init__(self, fixture):
        self.fixture, self.buffers, self.events = fixture, {}, []
        self.corrupt = False
        self.corrupt_arm = None
        self.corrupt_offset = None
        self.loaded_mutation = None

    def allocate(self, data):
        handle = len(self.buffers) + 1
        self.buffers[handle] = data
        self.events.append(('allocate', handle))
        return handle

    def read(self, handle, size):
        self.events.append(('read', handle))
        assert len(self.buffers[handle]) == size
        return self.buffers[handle]

    def command(self, command, payload=b'', expected=None):
        operation = command['op']
        self.events.append((operation, command.get('buffer')))
        if operation == 'load_kernel':
            split = command['symbol'] == driver.SYMBOLS['paired']
            info = metadata(3)
            info.update(symbol=command['symbol'], object_sha256=command['object_sha256'])
            for argument in info['explicit_arguments']:
                if argument['global_buffer']:
                    argument['pointee_alignment'] = argument['access'] = None
            loaded = {'kernel': 2 if split else 1, 'metadata': info}
            if self.loaded_mutation is not None:
                self.loaded_mutation(loaded)
            return loaded, b''
        if operation == 'write':
            handle, offset = command['buffer'], command['offset']
            old = self.buffers[handle]
            assert command['payload_bytes'] == len(payload) and expected == 'written'
            self.buffers[handle] = old[:offset] + payload + old[offset + len(payload):]
            return {'op': 'written'}, b''
        if operation == 'free':
            del self.buffers[command['buffer']]
            return {'op': 'freed'}, b''
        raise AssertionError(operation)


class Session:
    def __init__(self, worker, mode, device):
        assert mode == 'latency' and device == 123
        self.worker, self.frontier, self.epoch = worker, 0, 0

    def configure(self):
        self.worker.events.append(('configure', None))

    def dispatch(self, plans, *, post_warmup, clock):
        assert len(plans) == 1
        plan = plans[0]
        split = plan['symbol'] == driver.SYMBOLS['paired']
        assert plan['command']['kernel'] == (2 if split else 1)
        assert plan['command']['grid'] == [98304, 1, 1]
        assert plan['command']['workgroup'] == [64, 1, 1]
        count, offset = 3, 48
        tag = 5 if self.frontier < 2 else 4
        assert struct.unpack_from('<IIIII', plan['encoded'], offset) == (32, 12288, 4096, 1, tag)
        pointers = plan['command']['pointers']
        assert len(pointers) == count
        assert [p['buffer'] for p in pointers] == [1, 2, 3]
        assert [p['buffer_offset'] for p in pointers] == [64, 64, 64]
        assert [p['extent_bytes'] for p in pointers] == [262144, 100663296, 786432]
        assert len(plan['encoded']) == 328
        self.worker.events.append(('dispatch', plan['symbol']))
        assert post_warmup is (self.frontier >= 4)
        self.frontier += 1
        data = self.worker.fixture.output_bf16
        if self.worker.corrupt:
            data = data[:393216] * 2
        if self.worker.corrupt_arm == plan['symbol']:
            offset = self.worker.corrupt_offset
            data = data[:offset] + bytes([data[offset] ^ 1]) + data[offset + 1:]
        self.worker.buffers[3] = driver.GUARD + data + driver.GUARD
        return {'host_dispatch_wall_ns': 123, 'ordered_worker_elapsed_ns': 100, 'dispatch_count': 1}


class K16Driver(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = fixtures.build_fixture()

    def images(self):
        return {arm: (b'matched-image', hashlib.sha256(b'matched-image').hexdigest()) for arm in driver.ARMS}

    def run_driver(self, worker, retain, alive):
        images = self.images()
        with mock.patch.dict(driver.IMAGES, {arm: item[1] for arm, item in images.items()}):
            return driver.run_component(types.SimpleNamespace(OrderedSession=Session), worker,
                self.fixture, images, 123, retain, alive)

    def test_schedule_tags_and_abba_order(self):
        schedule = list(driver.schedule())
        self.assertEqual(len(schedule), 16)
        self.assertEqual([row['projection'] for row in schedule[:4]], [5, 5, 4, 4])
        self.assertEqual([row['arm'] for row in schedule[4:]], ['control', 'paired', 'paired', 'control'] * 3)
        self.assertTrue(all(row['projection'] == 4 for row in schedule[4:]))

    def test_full_driver_lifecycle_and_output_checks(self):
        worker, retained, calls = Worker(self.fixture), [], []
        result = self.run_driver(worker, retained.append, lambda: calls.append(True))
        self.assertEqual(len(calls), 32)
        self.assertEqual(result['dispatch_packets'], 16)
        self.assertTrue(result['control_compiler_matches_candidate'])
        self.assertEqual(result['unique_code_objects'], 1)
        self.assertEqual(result['allocation_ids'], {'a': 1, 'kn': 2, 'output': 3})
        self.assertFalse(worker.buffers)
        self.assertEqual(len(retained), 32)
        self.assertEqual([row['event'] for row in retained], ['completed_unchecked', 'verified_sample'] * 16)
        self.assertEqual(result['inputs_before'], result['inputs_after'])
        self.assertTrue(all(row['output_check']['sha256'] == hashlib.sha256(self.fixture.output_bf16).hexdigest()
                            for row in result['samples']))
        self.assertEqual([operation for operation, _ in worker.events].count('allocate'), 3)
        first = next(index for index, row in enumerate(worker.events) if row[0] == 'write')
        self.assertEqual([name for name, _ in worker.events[first:first + 48]], ['write', 'dispatch', 'read'] * 16)

    def test_bad_candidate_bytes_rejected_before_allocation(self):
        worker = Worker(self.fixture)
        images = self.images()
        with self.assertRaises(ValueError):
            driver.run_component(types.SimpleNamespace(OrderedSession=Session), worker,
                self.fixture, images, 123, lambda _: None, lambda: None)
        self.assertEqual(worker.events, [])
        mutations = (
            lambda loaded: loaded['metadata'].update(symbol='stale-root'),
            lambda loaded: loaded['metadata'].update(object_sha256=[0] * 32),
            lambda loaded: loaded.update(kernel=0),
            lambda loaded: loaded.update(kernel=1),
            lambda loaded: loaded.pop('metadata'),
        )
        for mutation in mutations:
            worker = Worker(self.fixture)
            worker.loaded_mutation = mutation
            with self.subTest(mutation=mutation), self.assertRaises((ValueError, KeyError)):
                self.run_driver(worker, lambda _: None, lambda: None)
            self.assertEqual([name for name, _ in worker.events].count('dispatch'), 0)

    def test_incorrect_half_stops_before_admission(self):
        worker, retained = Worker(self.fixture), []
        worker.corrupt = True
        with self.assertRaises(ValueError):
            self.run_driver(worker, retained.append, lambda: None)
        self.assertEqual([row['event'] for row in retained], ['completed_unchecked'])
        self.assertEqual([name for name, _ in worker.events].count('dispatch'), 1)

    def test_each_arm_each_output_half_is_checked(self):
        for arm in driver.ARMS:
            for offset in (0, 393216, 786431):
                worker, retained = Worker(self.fixture), []
                worker.corrupt_arm = driver.SYMBOLS[arm]
                worker.corrupt_offset = offset
                with self.subTest(arm=arm, offset=offset), self.assertRaises(ValueError):
                    self.run_driver(worker, retained.append, lambda: None)
                expected = 1 if arm == 'control' else 2
                self.assertEqual([name for name, _ in worker.events].count('dispatch'), expected)
                self.assertEqual(retained[-1]['event'], 'completed_unchecked')

    def test_different_image_pair_rejected_even_when_individually_hashed(self):
        images = {arm: (arm.encode(), hashlib.sha256(arm.encode()).hexdigest()) for arm in driver.ARMS}
        with mock.patch.dict(driver.IMAGES, {arm: item[1] for arm, item in images.items()}):
            with self.assertRaisesRegex(ValueError, 'same emitted image'):
                driver.validate_images(images)

    def test_incomplete_schedule_cannot_report_complete_campaign(self):
        worker = Worker(self.fixture)
        cells = list(driver.schedule())[:-1]
        with mock.patch.object(driver, 'schedule', return_value=iter(cells)):
            with self.assertRaisesRegex(ValueError, 'complete16-cell16-packet'):
                self.run_driver(worker, lambda _: None, lambda: None)

    def test_lifetime_failure_stops_before_next_reset(self):
        worker, calls = Worker(self.fixture), []

        def alive():
            calls.append(True)
            if len(calls) == 3:
                raise ValueError('lost supervisor')

        with self.assertRaisesRegex(ValueError, 'lost supervisor'):
            self.run_driver(worker, lambda _: None, alive)
        self.assertEqual([name for name, _ in worker.events].count('dispatch'), 1)
        self.assertEqual([name for name, _ in worker.events].count('write'), 1)

    def test_guard_corruption_rejected(self):
        worker = Worker(self.fixture)
        record = {'id': 1, 'data': b'\0\0', 'element_bytes': 2}
        worker.buffers[1] = b'\0' * 64 + record['data'] + driver.GUARD
        with self.assertRaises(ValueError):
            driver.checked_read(worker, record, record['data'], 'bad leading guard')


if __name__ == '__main__':
    unittest.main()
