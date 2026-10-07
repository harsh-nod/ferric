import types
import unittest

import ordered_component as driver
import component_contract as contract


class DriverTests(unittest.TestCase):
    def exercise(self, mode='latency', fault=None):
        frozen = contract.load_component(contract.D)
        events, retained, sessions = [], [], []
        records = {name: {'id': index + 1, 'data': b'zero'}
                   for index, name in enumerate(('a', 'nk', 'kn', 'output', 'partials'))}
        plans = {name: [{'metadata': {}, 'tag': part} for part in parts]
                 for name, parts in {'wave': ('wave',), 'mfma': ('mfma',),
                                    'splitk': ('partial', 'merge')}.items()}
        if fault == 'arm-count':
            plans['splitk'].pop()

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
                self.frontier += len(selected)
                return {'mode': self.mode, 'dispatch_count': len(selected),
                        'host_dispatch_wall_ns': 90, 'ordered_worker_elapsed_ns': 40}

        def read(_worker, record, expected, name):
            events.append(('read', name, expected))
            if fault == 'parity':
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
            records_for=lambda _: records,
            allocate_shared=lambda *_: events.append('allocate'),
            component_plans=lambda *_: events.append('load') or plans,
            check_inputs=inputs, schedule=frozen.schedule, require=frozen.require,
            reset_outputs=lambda *_: events.append('reset'), checked_read=read, TAIL=b'tail')
        try:
            result = driver.run_component(component, types.SimpleNamespace(OrderedSession=Session),
                types.SimpleNamespace(command=command),
                types.SimpleNamespace(output_f32=b'output', partials_f32=b'partials'),
                (b'v5', 'v5hash'), (b'candidate', 'candidatehash'), mode, 31,
                retained.append, lambda: events.append('alive'))
        except ValueError:
            self.assertIsNotNone(fault)
            return events, retained, None
        self.assertIsNone(fault)
        return events, retained, result

    def test_configuration_precedes_allocation_and_load(self):
        events, _, result = self.exercise()
        self.assertEqual(events[:4], ['configure', 'allocate', 'load', 'inputs'])
        self.assertEqual(result['dispatch_packets'], 44)

    def test_same_fixed_schedule_and_split_single_batch(self):
        events, retained, result = self.exercise()
        batches = [item for item in events if type(item) is tuple and item[0] == 'batch']
        self.assertEqual(len(batches), 30)
        self.assertEqual(sum(item[1] == ('partial', 'merge') for item in batches), 14)
        self.assertEqual([item[2] for item in batches], [False] * 6 + [True] * 24)
        self.assertEqual(events.count('alive'), 60)
        self.assertEqual(len(retained), 60)
        self.assertEqual(len(result['samples']), 30)

    def test_reset_and_parity_outside_dispatch_and_endpoints(self):
        events, _, _ = self.exercise()
        start = events.index('alive')
        self.assertEqual(events[start:start + 4], ['alive', 'reset', ('batch', ('wave',), False), 'alive'])
        self.assertEqual(events[start + 4:start + 6],
                         [('read', 'output', b'outputtail'), ('read', 'partials', b'zero')])
        self.assertIn(('read', 'partials', b'partials'), events)

    def test_modes_keep_identical_inputs_and_no_gpu_ns_label(self):
        for mode in ('latency', 'counters', 'ticks'):
            with self.subTest(mode=mode):
                _, _, result = self.exercise(mode)
                self.assertEqual(result['mode'], mode)
                self.assertEqual(result['performance_configuration']['profile'], mode != 'latency')
                self.assertEqual(result['inputs_before'], result['inputs_after'])
                self.assertNotIn('gpu_ns', result)
                self.assertIn('excludes preparation/staging', result['worker_timing_scope'])

    def test_bad_arm_cannot_dispatch(self):
        events, _, _ = self.exercise(fault='arm-count')
        self.assertFalse(any(type(item) is tuple and item[0] == 'batch' for item in events))

    def test_dispatch_and_parity_failures_do_not_free_or_publish_verified(self):
        for fault in ('dispatch', 'parity'):
            events, retained, _ = self.exercise(fault=fault)
            self.assertFalse(any(type(item) is tuple and item[0] == 'free' for item in events))
            self.assertFalse(any(item['event'] == 'verified_sample' for item in retained))

    def test_input_drift_is_terminal_before_free(self):
        events, _, _ = self.exercise(fault='input-drift')
        self.assertFalse(any(type(item) is tuple and item[0] == 'free' for item in events))

    def test_all_allocations_freed_and_bad_free_is_terminal(self):
        events, _, _ = self.exercise()
        self.assertEqual([item for item in events if type(item) is tuple and item[0] == 'free'],
                         [('free', index) for index in range(1, 6)])
        self.exercise(fault='free')
