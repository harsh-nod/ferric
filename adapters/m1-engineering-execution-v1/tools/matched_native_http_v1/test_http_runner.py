from contextlib import nullcontext
import importlib.util
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

definition = importlib.util.spec_from_file_location('http_runner_tests', Path(__file__).with_name('run_matched128.py'))
m = importlib.util.module_from_spec(definition)
definition.loader.exec_module(m)


class SupervisorTests(unittest.TestCase):
    def test_absent_outer_cache_requires_no_deletion(self):
        name = 'ferric-matched128-vllm-' + 'a' * 16
        intent = {'name': name, 'image': 'sha256:' + 'b' * 64,
                  'label_key': 'ferric.matched128', 'label_value': name}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(m, 'TMPFS_ROOT', root), patch.object(m.Container, 'retire_cache') as retire:
                result = m.retire_outer_cache(intent, root, root, None)
            self.assertTrue(result['absent'])
            self.assertFalse(result['retired_by_outer'])
            retire.assert_not_called()

    def test_outer_cache_without_exact_creation_receipt_is_preserved(self):
        name = 'ferric-matched128-vllm-' + 'a' * 16
        intent = {'name': name, 'image': 'sha256:' + 'b' * 64,
                  'label_key': 'ferric.matched128', 'label_value': name}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cache = root / name
            cache.mkdir()
            with patch.object(m, 'TMPFS_ROOT', root), patch.object(m.Container, 'retire_cache') as retire:
                with self.assertRaises(FileNotFoundError):
                    m.retire_outer_cache(intent, root, root, None)
                (root / 'cache-identity.json').write_text(json.dumps({
                    'schema': 'FerricMatched128TmpfsCacheV1', 'identity': {'path': '/foreign/cache'}}))
                with self.assertRaisesRegex(ValueError, 'retained cache creation identity'):
                    m.retire_outer_cache(intent, root, root, None)
            retire.assert_not_called()
            self.assertTrue(cache.is_dir())

    def test_server_thread_start_failure_never_shuts_down_unstarted_loop(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            clean = {'cleanup_ok': True, 'child_reaped': True, 'owned_descendants_absent': True,
                'threads_joined': True, 'closed_receipt': True, 'returncode': 0,
                'signals': [], 'errors': [], 'privileged_members_observed': []}
            class Backend:
                def __init__(self, config, *, owner, output):
                    self.setup = {}
                    owner.backend = self
                def wait_ready(self):
                    pass
                def close(self):
                    return clean
            class NeverStarted:
                ident = None
                def __init__(self, **kwargs):
                    pass
                def start(self):
                    raise RuntimeError('injected server thread start failure')
                def is_alive(self):
                    return False
                def join(self, **kwargs):
                    raise AssertionError('must not join an unstarted thread')
            server = SimpleNamespace(server_close=lambda: None,
                shutdown=lambda: (_ for _ in ()).throw(AssertionError('unstarted shutdown hangs')))
            serving = SimpleNamespace(Config=lambda **kwargs: kwargs, Server=lambda *_: server)
            binding = {'path': '/qualified/binary', 'sha256': 'a' * 64}
            plan = {'ferric': {'controller': binding, 'worker': binding, 'argv': ['/qualified/binary']},
                    'driver_sources': {}}
            owner = m.Ferric(serving, plan, output, 'prompt')
            with patch.object(m, 'digest', return_value='a' * 64), \
                 patch.object(m.private, 'backend_class', return_value=Backend), \
                 patch.object(m.contract, 'selection_adapter', return_value=SimpleNamespace(validate_setup=lambda *_: None)), \
                 patch.object(m.threading, 'Thread', NeverStarted):
                with self.assertRaisesRegex(RuntimeError, 'server thread'):
                    owner.start()
                owner.close()
            self.assertTrue(owner.closed)

    def exercise(self, *, engine='ferric', outer=None, error=None, fallback=False,
                 inner_change=None, postflight_error=False):
        outer = outer or {'status': 0, 'cleanup_ok': True, 'child_reaped': True,
                         'term_sent': False, 'kill_sent': False, 'errors': []}
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            plan = {'ferric': {'selected': True}, 'reference': {'sha256': 'f' * 64}}
            inner = {'schema': 'FerricNativeMatched128EngineeringReceiptV1', 'engine': engine,
                     'plan_sha256': 'a' * 64, 'cleanup_completed': True, 'errors': [], 'timing_admitted': True}
            inner.update(inner_change or {})
            (output / 'receipt.json').write_text(json.dumps(inner))
            observer = SimpleNamespace(container_identity=None,
                sample=lambda phase, *_: (_ for _ in ()).throw(ValueError('postflight'))
                    if postflight_error and phase == 'postflight' else {'accepted': True})
            retirement = {'absent': True, 'stop_sent': fallback, 'remove_sent': fallback, 'retained_identity': None}
            with patch.object(m, 'gpu_lease', return_value=nullcontext()), \
                 patch.object(m, 'ProbeObserver', return_value=observer), \
                 patch.object(m.life, 'supervise_group', side_effect=error, return_value=outer) as group, \
                 patch.object(m.custody, 'retire', return_value=retirement) as retire, \
                 patch.object(m, 'settle_gpu_idle', return_value={'idle': True}):
                status = m.supervise(plan, Path('/input/plan.json'), 'a' * 64, engine, output)
                result = json.loads((output / 'outer-receipt.json').read_text())
                return status, result, group.call_args, retire.call_count

    def test_normal_outer_and_inner_completion_admits_timing(self):
        status, result, call, _ = self.exercise()
        self.assertEqual(status, 0)
        self.assertTrue(result['timing_admitted'])
        self.assertIn('--execute', call.args[1])
        self.assertEqual(call.kwargs['grace'], 25)

    def test_outer_term_invalidates_clean_inner(self):
        status, result, _, _ = self.exercise(outer={'status': 0, 'cleanup_ok': True,
            'child_reaped': True, 'term_sent': True, 'kill_sent': False, 'errors': []})
        self.assertEqual(status, 125)
        self.assertFalse(result['timing_admitted'])

    def test_outer_kill_invalidates_clean_inner(self):
        status, result, _, _ = self.exercise(outer={'status': 125, 'cleanup_ok': False,
            'child_reaped': True, 'term_sent': True, 'kill_sent': True, 'errors': []})
        self.assertEqual(status, 125)
        self.assertFalse(result['timing_admitted'])

    def test_container_fallback_runs_even_when_group_wrapper_raises(self):
        status, result, _, calls = self.exercise(engine='vllm', error=ValueError('group failed'))
        self.assertEqual(calls, 1)
        self.assertEqual(status, 125)
        self.assertTrue(result['postflight']['accepted'])

    def test_outer_container_stop_cannot_be_normal_timing_completion(self):
        status, result, _, calls = self.exercise(engine='vllm', fallback=True)
        self.assertEqual(calls, 1)
        self.assertEqual(status, 125)
        self.assertFalse(result['timing_admitted'])

    def test_inner_failed_cleanup_or_wrong_plan_never_admitted(self):
        for field, value in (('cleanup_completed', False), ('plan_sha256', 'b' * 64),
                             ('errors', ['failure'])):
            with self.subTest(field=field):
                status, result, _, _ = self.exercise(inner_change={field: value})
                self.assertEqual(status, 125)
                self.assertFalse(result['timing_admitted'])

    def test_postflight_failure_invalidates_complete_request(self):
        status, result, _, _ = self.exercise(postflight_error=True)
        self.assertEqual(status, 125)
        self.assertFalse(result['timing_admitted'])

    def test_numerically_unadmitted_inner_remains_unadmitted(self):
        status, result, _, _ = self.exercise(inner_change={'timing_admitted': False})
        self.assertEqual(status, 0)
        self.assertFalse(result['timing_admitted'])


if __name__ == '__main__':
    unittest.main()
