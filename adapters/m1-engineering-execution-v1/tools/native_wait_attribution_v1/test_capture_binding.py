"""CPU-only capture binding fixtures; the original legacy replay stays independent."""
import copy
import importlib.util
from pathlib import Path
import types
import unittest


def load(name):
    path = Path(__file__).with_name(name + '.py')
    value = importlib.util.module_from_spec(importlib.util.spec_from_file_location(name, path))
    value.__spec__.loader.exec_module(value)
    return value


b = load('capture_binding')
f = load('test_wait_observation')
driver = load('run_wait')


def fixture():
    records = f.records()
    value = {'epoch': 0, 'executions': [{'frontier': row['next_write'], 'host_ns': 60000}
             for row in records[4:]], 'before': {'token': {'executions': 4, 'dispatches': 2596}},
             'token_counter_delta': {'executions': 127, 'dispatches': 127 * 652}}
    legacy_raw = b'{"start":true}\n{"breakdown":true}\n{"end":true}\n'
    def replay(raw, spec, setup, counter):
        if raw != legacy_raw:
            raise ValueError('legacy bytes changed')
        return {'accepted': True, 'breakdown': value}
    legacy = types.SimpleNamespace(SETUP={}, exact=lambda a, c: a == c,
                                   clean_exit=lambda _: None, replay=replay)
    return b.Replay(legacy, f.w), f.encode(records) + legacy_raw, value


def original():
    argv = ['controller', '--worker', '/old/worker', '--worker-sha256', 'f' * 64]
    return {'cell_id': 'counter-A', 'spec': {'arm': 'A', 'argv': argv[:],
            'worker': {'path': '/old/worker', 'sha256': 'f' * 64}}, 'common_args': argv[1:],
            'output': '/old/cells/counter-A'}


class CaptureBindingTests(unittest.TestCase):
    def test_derive_only_worker_and_output(self):
        row = original()
        before = copy.deepcopy(row)
        changed = b.derive(row, Path('/new'), 'e' * 64)
        self.assertEqual(row, before)
        self.assertEqual(changed['spec']['worker'], {'path': '/new/worker-candidate', 'sha256': 'e' * 64})
        for argv in (changed['spec']['argv'], changed['common_args']):
            self.assertEqual(argv[argv.index('--worker-sha256') + 1], 'e' * 64)
        self.assertEqual(changed['output'], '/new/cells/counter-A')

    def test_nonbaseline_rejected(self):
        for key in ('cell_id', 'arm'):
            row = original()
            if key == 'cell_id':
                row[key] = 'counter-B'
            else:
                row['spec'][key] = 'B'
            with self.assertRaises(ValueError):
                b.derive(row, Path('/new'), 'e' * 64)

    def test_ambiguous_worker_selector_rejected(self):
        for suffix in (['--worker', '/other'], ['--worker-sha256', '0' * 64]):
            row = original()
            row['common_args'] += suffix
            with self.assertRaises(ValueError):
                b.derive(row, Path('/new'), 'e' * 64)

    def test_positive_binding(self):
        check, raw, _ = fixture()
        result = check.replay(raw, {'arm': 'A'}, {}, None)
        self.assertTrue(result['accepted'])
        self.assertTrue(result['wait']['controller_spans_bound'])
        self.assertFalse(result['performance_qualified'])

    def test_legacy_failure_propagates(self):
        check, raw, _ = fixture()
        with self.assertRaises(ValueError):
            check.replay(raw.replace(b'"start":true', b'"start":false'), {'arm': 'A'}, {}, None)
        check.legacy.replay = lambda *_: {'accepted': False}
        with self.assertRaises(ValueError):
            check.replay(raw, {'arm': 'A'}, {}, None)

    def test_epoch_and_frontier_binding(self):
        for change in ('epoch', 'frontier'):
            check, raw, value = fixture()
            if change == 'epoch':
                value['epoch'] = 1
            else:
                value['executions'][20]['frontier'] += 1
            with self.assertRaises(ValueError):
                check.replay(raw, {'arm': 'A'}, {}, None)

    def test_nested_native_duration(self):
        check, raw, value = fixture()
        value['executions'][3]['host_ns'] = 50199
        with self.assertRaises(ValueError):
            check.replay(raw, {'arm': 'A'}, {}, None)

    def test_aggregate_binding(self):
        for scope in ('before', 'after'):
            check, raw, value = fixture()
            row = value['before']['token'] if scope == 'before' else value['token_counter_delta']
            row['dispatches'] -= 1
            with self.assertRaises(ValueError):
                check.replay(raw, {'arm': 'A'}, {}, None)

    def test_missing_execution_and_wrong_arm(self):
        check, raw, value = fixture()
        with self.assertRaises(ValueError):
            check.replay(raw, {'arm': 'B'}, {}, None)
        value['executions'].pop()
        with self.assertRaises(ValueError):
            check.replay(raw, {'arm': 'A'}, {}, None)

    def test_cpu_closure(self):
        value = {'status': 0, 'returncode': 0, 'cleanup_ok': True, 'child_reaped': True,
                 'term_sent': False, 'kill_sent': False, 'errors': [],
                 'profile': 'FerricCpuFourCore45GiBEmitterV1'}
        driver.clean_cpu(value)
        for field, wrong in (('status', 1), ('returncode', 1), ('cleanup_ok', False),
                             ('child_reaped', False), ('term_sent', True), ('kill_sent', True),
                             ('errors', ['failure']), ('profile', 'other')):
            with self.assertRaises(ValueError):
                driver.clean_cpu({**value, field: wrong})


if __name__ == '__main__':
    suite = unittest.TestSuite([unittest.defaultTestLoader.loadTestsFromTestCase(f.WaitObservationTests),
                               unittest.defaultTestLoader.loadTestsFromTestCase(CaptureBindingTests)])
    raise SystemExit(0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1)
