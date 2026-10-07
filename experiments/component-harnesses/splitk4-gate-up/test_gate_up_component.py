import hashlib
import struct
import types
import unittest
from unittest import mock

import fixtures
import gate_up_component as driver
from test_component_inputs import metadata


class Worker:
    def __init__(self, fixture):
        self.fixture, self.buffers, self.events = fixture, {}, []
        self.loaded_mutation = None
        self.after_dispatch = None
        self.duplicate_allocation = False
        self.bad_reset = False

    def allocate(self, data):
        handle = len(self.buffers) + 1
        self.buffers[handle] = data
        self.events.append(('allocate', handle))
        return 1 if self.duplicate_allocation else handle

    def read(self, handle, size):
        self.events.append(('read', handle))
        assert len(self.buffers[handle]) == size
        return self.buffers[handle]

    def command(self, command, payload=b'', expected=None):
        operation = command['op']
        self.events.append((operation, command.get('buffer')))
        if operation == 'load_kernel':
            role = next(role for role, symbol in driver.SYMBOLS.items() if symbol == command['symbol'])
            info = metadata(command['symbol'])
            info.update(object_sha256=command['object_sha256'])
            loaded = {'kernel': {'wave': 1, 'partial': 2, 'merge': 3}[role], 'metadata': info}
            assert expected == 'loaded_kernel'
            if self.loaded_mutation is not None:
                self.loaded_mutation(loaded)
            return loaded, b''
        if operation == 'write':
            handle, offset = command['buffer'], command['offset']
            old = self.buffers[handle]
            assert command['payload_bytes'] == len(payload) and expected == 'written'
            assert handle in (4, 5) and offset == 64
            self.buffers[handle] = old[:offset] + payload + old[offset + len(payload):]
            return {'op': 'bad-reset' if self.bad_reset else 'written'}, b''
        if operation == 'free':
            del self.buffers[command['buffer']]
            return {'op': 'freed'}, b''
        raise AssertionError(operation)


class Session:
    def __init__(self, worker, mode, device):
        assert mode == 'latency' and device == 123
        self.worker, self.frontier, self.epoch, self.frames = worker, 0, 0, 0

    def configure(self):
        self.worker.events.append(('configure', None))

    def dispatch(self, plans, *, post_warmup, clock):
        roles = [next(role for role, symbol in driver.SYMBOLS.items() if symbol == plan['symbol'])
                 for plan in plans]
        assert roles in (['wave'], ['partial', 'merge'])
        for role, plan in zip(roles, plans, strict=True):
            command = plan['command']
            assert command['kernel'] == {'wave': 1, 'partial': 2, 'merge': 3}[role]
            assert command['grid'] == [{'wave': 12288, 'partial': 3072, 'merge': 192}[role] * 64, 1, 1]
            assert command['workgroup'] == [64, 1, 1]
            ids = {'wave': [1, 2, 5], 'partial': [1, 3, 4], 'merge': [4, 5]}[role]
            extents = {'wave': [8192, 100663296, 786432], 'partial': [8192, 100663296, 196608],
                       'merge': [196608, 786432]}[role]
            assert command['pointers'] == [
                {'kernarg_offset': index * 16, 'buffer': handle, 'buffer_offset': 64,
                 'extent_bytes': extent, 'access': 'write' if index == len(ids) - 1 else 'read'}
                for index, (handle, extent) in enumerate(zip(ids, extents, strict=True))]
            assert len(plan['encoded']) == (288 if role == 'merge' else 328)
            assert plan['encoded'][-256:] == bytes(256)
            if role != 'merge':
                tag = 5 if self.frames < 2 else 4
                assert struct.unpack_from('<5I', plan['encoded'], 48) == (1, 12288, 4096, 1, tag)
        assert post_warmup is (self.frontier >= 6)
        assert self.worker.buffers[4] == driver.GUARD + driver.SCRATCH_POISON + driver.GUARD
        assert self.worker.buffers[5] == driver.GUARD + b'\xa5' * fixtures.OUTPUT_CAPACITY_BYTES + driver.GUARD
        self.worker.events.append(('dispatch', tuple(roles)))
        output = self.worker.fixture.output_bf16
        self.worker.buffers[5] = (driver.GUARD + output
            + b'\xa5' * (fixtures.OUTPUT_CAPACITY_BYTES - len(output)) + driver.GUARD)
        if roles == ['partial', 'merge']:
            self.worker.buffers[4] = driver.GUARD + self.worker.fixture.partials_f32 + driver.GUARD
        self.frontier += len(plans)
        self.frames += 1
        if self.worker.after_dispatch is not None:
            self.worker.after_dispatch(self.worker, self.frames)
        return {'host_dispatch_wall_ns': 123, 'ordered_worker_elapsed_ns': 100, 'dispatch_count': len(plans)}


def corrupt_byte(worker, handle, offset):
    old = worker.buffers[handle]
    worker.buffers[handle] = old[:offset] + bytes([old[offset] ^ 1]) + old[offset + 1:]


class GateUpDriver(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = fixtures.build_fixture()

    def images(self):
        return {arm: (arm.encode(), hashlib.sha256(arm.encode()).hexdigest()) for arm in driver.ARMS}

    def run_driver(self, worker, retain=lambda _: None, alive=lambda: None):
        images = self.images()
        with mock.patch.dict(driver.IMAGES, {arm: item[1] for arm, item in images.items()}):
            return driver.run_component(types.SimpleNamespace(OrderedSession=Session), worker,
                self.fixture, images, 123, retain, alive)

    def test_schedule_tags_and_abba_order(self):
        schedule = list(driver.schedule())
        self.assertEqual(len(schedule), 16)
        self.assertEqual([row['projection'] for row in schedule[:4]], [5, 5, 4, 4])
        self.assertEqual([row['arm'] for row in schedule[4:]], ['wave', 'splitk4', 'splitk4', 'wave'] * 3)
        self.assertTrue(all(row['projection'] == 4 for row in schedule[4:]))

    def test_full_driver_lifecycle_ordering_resets_and_checks(self):
        worker, retained, calls = Worker(self.fixture), [], []
        result = self.run_driver(worker, retained.append, lambda: calls.append(True))
        self.assertEqual(len(calls), 32)
        self.assertEqual(result['dispatch_packets'], 24)
        self.assertEqual(result['packets_per_cell'], {'wave': 1, 'splitk4': 2})
        self.assertEqual(result['timed_dispatches_per_arm'], {'wave': 6, 'splitk4': 12})
        self.assertEqual(result['allocation_ids'], {'a': 1, 'nk': 2, 'kn': 3, 'scratch': 4, 'output': 5})
        self.assertFalse(worker.buffers)
        self.assertEqual(len(retained), 32)
        self.assertEqual([row['event'] for row in retained], ['completed_unchecked', 'verified_sample'] * 16)
        self.assertEqual(result['inputs_before'], result['inputs_after'])
        expected_output = self.fixture.output_bf16 + b'\xa5' * 761856
        for row in result['samples']:
            self.assertEqual(row['output_check']['sha256'], hashlib.sha256(expected_output).hexdigest())
            scratch = driver.SCRATCH_POISON if row['arm'] == 'wave' else self.fixture.partials_f32
            self.assertEqual(row['scratch_check']['sha256'], hashlib.sha256(scratch).hexdigest())
        first = next(index for index, row in enumerate(worker.events) if row[0] == 'write')
        self.assertEqual([name for name, _ in worker.events[first:first + 80]],
                         ['write', 'write', 'dispatch', 'read', 'read'] * 16)
        self.assertEqual([name for name, _ in worker.events].count('free'), 5)

    def test_bad_image_roster_or_bytes_rejected_before_allocation(self):
        for images in (self.images(), {}, {'wave': (b'x', '0' * 64)}):
            worker = Worker(self.fixture)
            with self.assertRaises(ValueError):
                driver.run_component(types.SimpleNamespace(OrderedSession=Session), worker,
                    self.fixture, images, 123, lambda _: None, lambda: None)
            self.assertEqual(worker.events, [])

    def test_bad_loaded_identity_or_abi_rejected_before_dispatch(self):
        mutations = (
            lambda loaded: loaded['metadata'].update(symbol='stale-root'),
            lambda loaded: loaded['metadata'].update(object_sha256=[0] * 32),
            lambda loaded: loaded['metadata'].update(kernarg_bytes=327),
            lambda loaded: loaded.update(kernel=0),
            lambda loaded: loaded.update(kernel=1),
            lambda loaded: loaded.pop('metadata'),
        )
        for mutation in mutations:
            worker = Worker(self.fixture)
            worker.loaded_mutation = mutation
            with self.subTest(mutation=mutation), self.assertRaises((ValueError, KeyError)):
                self.run_driver(worker)
            self.assertEqual([name for name, _ in worker.events].count('dispatch'), 0)

    def test_duplicate_allocations_rejected_before_loading(self):
        worker = Worker(self.fixture)
        worker.duplicate_allocation = True
        with self.assertRaisesRegex(ValueError, 'distinct allocation'):
            self.run_driver(worker)
        self.assertNotIn('load_kernel', [name for name, _ in worker.events])

    def test_output_active_tail_and_guard_corruption_rejected(self):
        for offset in (0, 64, 64 + 24576, 64 + fixtures.OUTPUT_CAPACITY_BYTES):
            worker, retained = Worker(self.fixture), []
            worker.after_dispatch = lambda worker, frame: corrupt_byte(worker, 5, offset)
            with self.subTest(offset=offset), self.assertRaises(ValueError):
                self.run_driver(worker, retained.append)
            self.assertEqual([row['event'] for row in retained], ['completed_unchecked'])

    def test_wave_scratch_mutation_and_candidate_partial_corruption_rejected(self):
        for bad_frame in (1, 2):
            worker, retained = Worker(self.fixture), []

            def corrupt(worker, frame):
                if frame == bad_frame:
                    corrupt_byte(worker, 4, 64 + 3 * 49152)

            worker.after_dispatch = corrupt
            with self.subTest(frame=bad_frame), self.assertRaises(ValueError):
                self.run_driver(worker, retained.append)
            self.assertEqual(retained[-1]['event'], 'completed_unchecked')
            self.assertEqual(len(retained), bad_frame * 2 - 1)

    def test_lifetime_loss_prevents_verification_or_next_reset(self):
        for failure_call in (2, 3):
            worker, calls, retained = Worker(self.fixture), [], []

            def alive():
                calls.append(True)
                if len(calls) == failure_call:
                    raise ValueError('lost supervisor')

            with self.subTest(call=failure_call), self.assertRaisesRegex(ValueError, 'lost supervisor'):
                self.run_driver(worker, retained.append, alive)
            self.assertEqual([name for name, _ in worker.events].count('dispatch'), 1)
            self.assertEqual([name for name, _ in worker.events].count('write'), 2)
            self.assertEqual(len(retained), failure_call - 1)

    def test_rejected_reset_prevents_dispatch(self):
        worker = Worker(self.fixture)
        worker.bad_reset = True
        with self.assertRaisesRegex(ValueError, 'reset'):
            self.run_driver(worker)
        self.assertNotIn('dispatch', [name for name, _ in worker.events])

    def test_input_mutation_fails_final_readback_before_free(self):
        for handle in (1, 2, 3):
            worker = Worker(self.fixture)

            def corrupt(worker, frame):
                if frame == 16:
                    corrupt_byte(worker, handle, 64)

            worker.after_dispatch = corrupt
            with self.subTest(handle=handle), self.assertRaises(ValueError):
                self.run_driver(worker)
            self.assertEqual([name for name, _ in worker.events].count('dispatch'), 16)
            self.assertNotIn('free', [name for name, _ in worker.events])


if __name__ == '__main__':
    unittest.main()
