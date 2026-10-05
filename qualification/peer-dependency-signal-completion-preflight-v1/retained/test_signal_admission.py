"""Synthetic byte-backed admission tests; no compiler or native invocation."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    'signal_gpu_admission', Path(__file__).with_name('run_gpu.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)


def encoded(value):
    return (json.dumps(value, sort_keys=True) + '\n').encode()


def file_pin(path, raw):
    return dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


class CpuFixture:
    def __enter__(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.evidence = self.root / 'evidence'
        self.evidence.mkdir()
        sources = {}
        for name in sorted(R.FORMATTED | {'run_cpu.py', 'Cargo.toml', 'Cargo.lock'}):
            sources[name] = self.put(name, ('synthetic source: ' + name + '\n').encode())
        self.controller = sources['run_cpu.py']
        self.cpu = dict(
            schema='ferric-peer-dependency-signal-completion-cpu-v1', passed=True,
            failure=None, postcheck_errors=[], source_unchanged=True,
            host=R.HOST, boot=R.BOOT, controller=self.controller,
            phases=[], raw={}, tests={}, cargo_lock=sources['Cargo.lock'],
            tool_pins={'synthetic-tool': self.put('tool', b'synthetic tool\n')})
        self.artifacts = {}
        for key, label, target in [
                ('compatibility_binary', 'probe-build', R.COMPATIBILITY_EXAMPLE),
                ('terminal_binary', 'terminal-probe-build', R.TERMINAL_EXAMPLE),
                ('binary', 'signal-probe-build', R.EXAMPLE)]:
            body = bytearray(64)
            body[:6] = b'\x7fELF\x02\x01'
            body[18:20] = b'\x3e\x00'
            body[24] = len(self.artifacts) + 1
            pin = self.put('target/debug/examples/' + target, bytes(body))
            Path(pin['path']).chmod(0o700)
            artifact = dict(reason='compiler-artifact', target=dict(name=target, kind=['example']),
                            profile=dict(test=False), executable=pin['path'])
            self.cpu[key] = dict(pin=pin, cargo_artifact=artifact)
            self.artifacts[label] = artifact
        for index, label in enumerate(R.PHASES):
            argv = ['/synthetic-tool', label]
            pid = 1000 + index
            command = self.raw(label + '.command.json', encoded(dict(argv=argv, cwd=str(self.root))))
            self.raw(label + '.started.json', encoded(dict(argv=argv, pid=pid, pgid=pid)))
            if label in {'aql-tests', 'kfd-tests', 'probe-tests', 'terminal-probe-tests', 'signal-probe-tests'}:
                name = label.replace('-', '_') + '::synthetic_case'
                stdout = ('running 1 test\ntest ' + name + ' ... ok\n'
                          'test result: ok. 1 passed; 0 failed; 0 ignored; '
                          '0 measured; 0 filtered out; finished in 0.00s\n').encode()
                self.cpu['tests'][label] = dict(
                    summaries=[dict(status='ok', passed=1, failed=0, ignored=0,
                                    measured=0, filtered_out=0)],
                    named=[dict(name=name, outcome='ok')], passed=1, failed=0, ignored=0)
            elif label in self.artifacts:
                stdout = encoded(self.artifacts[label]) + encoded(dict(reason='build-finished', success=True))
            else:
                stdout = b''
            row = dict(label=label, argv=argv, pid=pid, pgid=pid, exit_code=0,
                       natural_exit=True, reaped=True, process_group_absent=True,
                       forced_cleanup=False, timed_out=False, exception=None, command=command,
                       stdout=self.raw(label + '.stdout', stdout),
                       stderr=self.raw(label + '.stderr', b''))
            self.cpu['phases'].append(row)
            self.raw(label + '.result.json', encoded(row))
        for scope, field in [('input', 'input_sources'), ('formatted', 'formatted_sources'),
                             ('tested', 'tested_sources'), ('after', 'final_sources')]:
            self.cpu[field] = self.raw('sources-' + scope + '.json', encoded(sources))
        return self

    def __exit__(self, *_):
        self.temp.cleanup()

    def put(self, relative, raw):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        return file_pin(path, raw)

    def raw(self, name, body):
        pin = self.put('evidence/' + name, body)
        self.cpu['raw'][name] = pin
        return pin

    def phase(self, label):
        return next(row for row in self.cpu['phases'] if row['label'] == label)

    def phase_stdout(self, label, raw):
        row = self.phase(label)
        row['stdout'] = self.raw(label + '.stdout', raw)
        self.raw(label + '.result.json', encoded(row))

    def receipt(self):
        return self.put('evidence/complete.json', encoded(self.cpu))


def admit(fixture, supplied_sha=None, supplied_path=None):
    pin = fixture.receipt()
    with patch.multiple(R, CPU_ROOT=fixture.root,
                        CPU_RUNNER_SHA=fixture.controller['sha256'], INPUTS={},
                        HARD_DEADLINE=float('inf')):
        return R.admit_cpu(Path(supplied_path or pin['path']),
                           pin['sha256'] if supplied_sha is None else supplied_sha)


def native_fixture():
    buffers = []
    for rank in range(2):
        for role, owner, iteration in [('seed0', rank, 0), ('seed1', rank, 1),
                                       ('intermediate', rank, 1), ('output0', 1-rank, 0),
                                       ('output1', 1-rank, 1)]:
            payload = b''.join((0x3c00 | ((index * 13 + owner * 257 + iteration * 521) % 1024))
                               .to_bytes(2, 'little') for index in range(4096))
            body = b'\xa5' * 64 + payload + b'\xa5' * 64
            buffers.append(dict(owner_rank=rank, role=role, bytes=8320,
                                sha256=hashlib.sha256(body).hexdigest()))
    return dict(
        schema='ferric-peer-dependency-native-observation-v228-v3',
        request_sha256='1' * 64, device_unique_ids=[16366993098680759275, 10838076764495710945],
        artifact_path=str(R.IMAGE), artifact_bytes=14624, artifact_sha256=R.IMAGE_SHA,
        kernel_symbol='ferric_qwen3_tp_peer_copy_bf16_v4', timeout_ms=60000,
        witness_requested=True, witness_observed=True, completion_count=16,
        completion_contract='all signals complete; actual read retained for ring capacity',
        observed_queue_frontiers=[[8, 3], [8, 3]],
        elapsed_scope='native aggregate host interval; not GPU duration or throughput',
        device_iterations=2, kernel_packets=8, barrier_packets=8,
        intermediate_reused_between_iterations=True, completion_slots_reset_between_iterations=False,
        repeated_host_calls_tested=False, all_data_and_guards_verified=True,
        input_files_unchanged=True, healthy_close=True, performance_claim=False,
        production_authority=False, buffers=buffers, elapsed_ns=1)


class SignalAdmissionTests(unittest.TestCase):
    def test_synthetic_thirteen_phase_sixty_nine_raw_admission_selects_three_elfs(self):
        with CpuFixture() as fixture:
            self.assertEqual(len(fixture.cpu['phases']), 13)
            self.assertEqual(len(fixture.cpu['raw']), 69)
            self.assertEqual(len(fixture.cpu['tests']), 5)
            self.assertEqual(len(R.FORMATTED), 10)
            actual, signal, terminal, compatibility = admit(fixture)
            self.assertEqual(actual, fixture.receipt())
            self.assertEqual(signal, fixture.cpu['binary']['pin'])
            self.assertEqual(terminal, fixture.cpu['terminal_binary']['pin'])
            self.assertEqual(compatibility, fixture.cpu['compatibility_binary']['pin'])
            self.assertEqual(len({signal['path'], terminal['path'], compatibility['path']}), 3)

    def test_cpu_requires_actual_caller_hash_and_exact_new_completion_path(self):
        with CpuFixture() as fixture:
            for sha in ['', 'not-a-sha', 'A' * 64, '0' * 64]:
                with self.subTest(sha=sha), self.assertRaises(RuntimeError):
                    admit(fixture, supplied_sha=sha)
            with self.assertRaisesRegex(RuntimeError, 'exact CPU completion'):
                admit(fixture, supplied_path=fixture.root / 'old/evidence/complete.json')

    def test_cpu_refuses_old_schema_and_wrong_controller_generation(self):
        for field in ['native-schema', 'terminal-schema', 'controller']:
            with self.subTest(field=field), CpuFixture() as fixture:
                if field == 'native-schema':
                    fixture.cpu['schema'] = 'ferric-peer-dependency-native-cpu-v1'
                elif field == 'terminal-schema':
                    fixture.cpu['schema'] = 'ferric-peer-dependency-terminal-join-cpu-v1'
                else:
                    fixture.cpu['controller'] = dict(fixture.controller, sha256='0' * 64)
                with self.assertRaisesRegex(RuntimeError, 'CPU completion contract|CPU controller generation'):
                    admit(fixture)

    def test_cpu_requires_closed_phase_raw_and_test_rosters(self):
        for field in ['phases', 'raw', 'tests']:
            with self.subTest(field=field), CpuFixture() as fixture:
                if field == 'phases':
                    fixture.cpu[field].pop()
                elif field == 'raw':
                    del fixture.cpu[field]['signal-probe-tests.stderr']
                else:
                    del fixture.cpu[field]['signal-probe-tests']
                with self.assertRaisesRegex(RuntimeError, 'CPU thirteen-phase roster|CPU raw roster|CPU test scopes'):
                    admit(fixture)

    def test_cpu_refuses_failed_leaf_or_named_test_corruption(self):
        for case in ['exit', 'cleanup', 'test-log']:
            with self.subTest(case=case), CpuFixture() as fixture:
                if case == 'test-log':
                    fixture.phase_stdout('signal-probe-tests', b'test synthetic ... FAILED\n')
                else:
                    row = fixture.phase('default-check')
                    row['exit_code' if case == 'exit' else 'forced_cleanup'] = 1 if case == 'exit' else True
                    fixture.raw('default-check.result.json', encoded(row))
                with self.assertRaisesRegex(RuntimeError, 'non-natural|failed or absent'):
                    admit(fixture)

    def test_cpu_refuses_formatter_scope_drift_or_changed_final_snapshot(self):
        for scope, field, name in [('input', 'input_sources', 'Cargo.toml'),
                                    ('formatted', 'formatted_sources', 'run_cpu.py'),
                                    ('after', 'final_sources', 'Cargo.lock')]:
            with self.subTest(scope=scope), CpuFixture() as fixture:
                original = json.loads(Path(fixture.cpu[field]['path']).read_bytes())
                original[name]['sha256'] = '0' * 64
                fixture.cpu[field] = fixture.raw('sources-' + scope + '.json', encoded(original))
                with self.assertRaisesRegex(RuntimeError, 'CPU ten-file formatter|CPU source postcheck'):
                    admit(fixture)

    def test_cpu_replays_each_cargo_target_and_build_finished_record(self):
        for key, label in [('compatibility_binary', 'probe-build'),
                           ('terminal_binary', 'terminal-probe-build'), ('binary', 'signal-probe-build')]:
            for mutation in ['wrong-target', 'failed-build', 'duplicate-artifact']:
                with self.subTest(key=key, mutation=mutation), CpuFixture() as fixture:
                    artifact = copy.deepcopy(fixture.cpu[key]['cargo_artifact'])
                    if mutation == 'wrong-target':
                        artifact['target']['name'] = 'wrong-example'
                        fixture.cpu[key]['cargo_artifact'] = artifact
                    raw = encoded(artifact)
                    if mutation == 'duplicate-artifact':
                        raw += encoded(artifact)
                    raw += encoded(dict(reason='build-finished', success=mutation != 'failed-build'))
                    fixture.phase_stdout(label, raw)
                    with self.assertRaisesRegex(RuntimeError, 'Cargo build completion|actual Cargo example selection'):
                        admit(fixture)

    def test_cpu_rehashes_all_three_elfs_and_checks_each_architecture(self):
        for key in ['compatibility_binary', 'terminal_binary', 'binary']:
            for repin in [False, True]:
                with self.subTest(key=key, repin=repin), CpuFixture() as fixture:
                    selected = fixture.cpu[key]
                    path = Path(selected['pin']['path'])
                    raw = bytearray(path.read_bytes())
                    raw[18:20] = b'\xb7\x00'
                    path.write_bytes(raw)
                    if repin:
                        selected['pin'] = file_pin(path, raw)
                    with self.assertRaisesRegex(RuntimeError, 'FilePin mismatch|x86_64 little-endian ELF'):
                        admit(fixture)

    def test_native_accepts_only_closed_v3_sixteen_completion_eight_barrier_shape(self):
        value = native_fixture()
        self.assertEqual(R.observation(encoded(value), {'sha256': '1' * 64}), value)
        for field, old in [('schema', 'ferric-peer-dependency-native-observation-v228-v1'),
                           ('schema', 'ferric-peer-dependency-native-observation-v228-v2'),
                           ('completion_count', 14), ('barrier_packets', 6),
                           ('completion_contract', 'all queue read frontiers equal write')]:
            bad = copy.deepcopy(value)
            bad[field] = old
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                R.observation(encoded(bad), {'sha256': '1' * 64})
        for mutation in ['healthy_close', 'completion_contract', 'observed_queue_frontiers', 'extra']:
            bad = copy.deepcopy(value)
            if mutation != 'extra':
                del bad[mutation]
            else:
                bad['gpu_duration_ns'] = 1
            with self.subTest(mutation=mutation), self.assertRaisesRegex(RuntimeError, 'closed fields'):
                R.observation(encoded(bad), {'sha256': '1' * 64})

    def test_native_frontiers_retain_lagging_reads_and_reject_malformed_counters(self):
        value = native_fixture()
        for frontiers in [[[8, 3], [8, 3]], [[8, 0], [8, 8]], [[8, 3], [8, 5]]]:
            expected = copy.deepcopy(frontiers)
            raw = encoded(dict(value, observed_queue_frontiers=frontiers))
            observed = R.observation(raw, {'sha256': '1' * 64})
            self.assertEqual(observed['observed_queue_frontiers'], expected)
            self.assertEqual(frontiers, expected)
        malformed = [
            None, {}, [], [[8, 3]], [[8, 3], [8, 3], [8, 3]],
            [[8], [8, 3]], [[8, 3, 0], [8, 3]], [None, [8, 3]],
            [[True, 3], [8, 3]], [[8, False], [8, 3]],
            [[8.0, 3], [8, 3]], [[8, 3.0], [8, 3]],
            [[-1, 3], [8, 3]], [[8, -1], [8, 3]],
            [[7, 3], [8, 3]], [[9, 3], [8, 3]], [[8, 9], [8, 3]],
            [[8, 3], [8, True]], [[8, 3], [8, 1 << 64]], [[8, '3'], [8, 3]],
        ]
        for frontiers in malformed:
            with self.subTest(frontiers=frontiers), self.assertRaisesRegex(RuntimeError, 'queue frontiers'):
                R.observation(encoded(dict(value, observed_queue_frontiers=frontiers)), {'sha256': '1' * 64})

    def test_native_checks_every_guarded_buffer_hash_and_exact_u64_ids(self):
        value = native_fixture()
        for index in range(10):
            bad = copy.deepcopy(value)
            bad['buffers'][index]['sha256'] = '0' * 64
            with self.subTest(index=index), self.assertRaisesRegex(RuntimeError, 'guarded-buffer hash mismatch'):
                R.observation(encoded(bad), {'sha256': '1' * 64})
        for ids in [list(reversed(value['device_unique_ids'])),
                    [float(number) for number in value['device_unique_ids']]]:
            bad = copy.deepcopy(value)
            bad['device_unique_ids'] = ids
            with self.assertRaises(RuntimeError):
                R.observation(encoded(bad), {'sha256': '1' * 64})

    def test_native_refuses_witness_reset_close_or_request_identity_drift(self):
        value = native_fixture()
        for field in ['witness_observed', 'completion_slots_reset_between_iterations', 'healthy_close']:
            bad = copy.deepcopy(value)
            bad[field] = not bad[field]
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                R.observation(encoded(bad), {'sha256': '1' * 64})
        with self.assertRaises(RuntimeError):
            R.observation(encoded(value), {'sha256': '2' * 64})

    def test_native_elapsed_requires_positive_bounded_integer_not_boolean(self):
        value = native_fixture()
        for elapsed in [False, True, 0, -1, 1.0, 60_000_000_001]:
            bad = dict(value, elapsed_ns=elapsed)
            with self.subTest(elapsed=elapsed), self.assertRaisesRegex(RuntimeError, 'elapsed bound'):
                R.observation(encoded(bad), {'sha256': '1' * 64})
        value['elapsed_ns'] = 60_000_000_000
        self.assertEqual(R.observation(encoded(value), {'sha256': '1' * 64}), value)


if __name__ == '__main__':
    unittest.main()
