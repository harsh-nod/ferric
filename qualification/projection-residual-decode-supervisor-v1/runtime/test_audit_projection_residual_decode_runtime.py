"""Synthetic data/selection tests only; no audit process or native execution."""
import copy
import hashlib
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import audit_projection_residual_decode_runtime as A


class SelectorTests(unittest.TestCase):
    def fixture(self, role='worker'):
        root = A.E / A.CPU_LABEL
        bodies = {}
        def pin(path, raw):
            record = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
            bodies[str(path)] = raw
            return record
        def document(path, value):
            return pin(path, json.dumps(value, sort_keys=True).encode())
        name = A.NAMES[role]
        elf = bytes([0x7f, 69, 76, 70, 2, 1]) + bytes(12) + bytes([0x3e, 0])
        self.assertEqual(len(elf), 20)
        binary = pin(root / 'target' / role / 'debug' / name, elf)
        adapter = 'm1-engineering-execution-v1' if role == 'parent' else 'tp-peer-finite-engineering-worker-v1'
        source = root / 'sources/ferric/adapters' / adapter
        artifact = dict(reason='compiler-artifact', executable=binary['path'], filenames=[binary['path']],
            manifest_path=str(source / 'Cargo.toml'),
            target=dict(name=name, kind=['bin'], crate_types=['bin'],
                src_path=str(source / 'src' / ('main.rs' if role == 'worker' else 'bin/' + name + '.rs'))),
            profile=dict(debug_assertions=True, debuginfo=0, opt_level='2', overflow_checks=True, test=False))
        phase = 'parent-builds' if role == 'parent' else 'worker-build'
        stream = pin(root / (phase + '-stdout'), json.dumps(artifact).encode() + b'\n')
        snap = {'ferric/a.rs': {'bytes': 1, 'sha256': 'd' * 64}}
        before = document(root / 'sources-before.json', snap)
        after = document(root / 'sources-after.json', snap)
        controller = pin(A.E / 'p228-projection-residual-decode-cpu-v1/run.py', b'# synthetic\n')
        natural = dict(exit_code=0, reason=None, group_absent=True)
        phases = {phase: dict(natural, stdout_sha256=stream['sha256'])}
        phases.update({f'other-{n}': dict(natural) for n in range(86)})
        bins = {name: dict(artifact=artifact, binary=binary)}
        bins.update({f'other-{n}': {} for n in range(16)})
        value = dict(schema=A.CPU_SCHEMA, passed=True, error=None, postcheck_errors=[],
            source_unchanged=True, empty_initial_target=True, gpu_execution=False,
            numerical_acceptance=False, performance_claim=False, production_authority=False,
            prior_completion=dict(path=str(A.E / 'projection-residual-runtime-cpu-v228-v1/complete.json'),
                bytes=569967, sha256=A.PRIOR_CPU_SHA), controller=controller,
            metadata={'parent': {}, 'worker': {}}, phases=phases, binaries=bins,
            tests={'synthetic': dict(passed=1, ignored=4, summaries=[[1, 0, 4]],
                names=['pass', 'skip0', 'skip1', 'skip2', 'skip3'])},
            tests_passed=1, tests_ignored=4, raw={'sources-before.json': before,
                'sources-after.json': after, phase + '-stdout': stream})
        cpu = document(root / 'complete.json', value)
        calls = []
        def read(path, expected=None, retain=False, maximum=1 << 30):
            raw = bodies[str(path)]
            result = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
            A.require(len(raw) <= maximum and (expected is None or expected == result['sha256']),
                      'synthetic actual reader pin mismatch')
            calls.append(str(path))
            return result, raw if retain else b''
        pins = SimpleNamespace(read=read)
        args = ['projection-residual-decode-runtime-' + role + '-v228-v1', role,
                cpu['path'], cpu['sha256'], binary['path'], binary['sha256']]
        return SimpleNamespace(root=root, value=value, binary=binary, cpu=cpu, pins=pins,
            D=SimpleNamespace(parse=json.loads), args=args, phase=phase, artifact=artifact,
            bodies=bodies, calls=calls, document=document)

    def selected(self, f):
        with patch.object(A.os, 'access', return_value=True):
            return A.selected(f.D, f.args, f.pins)

    def test_both_roles_join_real_shaped_cpu_stream_sources_and_elf(self):
        for role in A.NAMES:
            f = self.fixture(role)
            self.assertEqual(self.selected(f), (A.E / f.args[0], f.binary))
            for name in ('sources-before.json', 'sources-after.json', f.phase + '-stdout'):
                self.assertIn(str(f.root / name), f.calls)
            self.assertIn(f.value['controller']['path'], f.calls)

    def test_closed_args_role_namespace_and_digest_before_reads(self):
        f = self.fixture()
        bad = [f.args[:-1], f.args + ['extent']]
        for index, value in [(0, 'projection-residual-decode-runtime-parent-v228-v1'),
                (1, 'image'), (2, '/tmp/complete.json'), (3, 'x' * 64),
                (4, f.args[4].replace('/worker/', '/parent/')), (5, 'A' * 64),
                (2, str(f.root / '../complete.json'))]:
            row = list(f.args); row[index] = value; bad.append(row)
        for row in bad:
            with self.assertRaises(RuntimeError): A.selected(f.D, row, f.pins)
        self.assertEqual(f.calls, [])

    def test_failed_or_nonnatural_or_authoritative_cpu_refuses(self):
        f = self.fixture()
        for change in ('passed', 'error', 'post', 'source', 'empty', 'gpu', 'phase', 'group'):
            v = copy.deepcopy(f.value)
            if change == 'passed': v['passed'] = False
            elif change == 'error': v['error'] = 'failed'
            elif change == 'post': v['postcheck_errors'] = ['drift']
            elif change == 'source': v['source_unchanged'] = False
            elif change == 'empty': v['empty_initial_target'] = False
            elif change == 'gpu': v['gpu_execution'] = True
            elif change == 'phase': v['phases'][f.phase]['exit_code'] = False
            else: v['phases'][f.phase]['group_absent'] = False
            with self.subTest(change=change), self.assertRaises(RuntimeError): A.qualified(v)

    def test_actual_test_accounting_cannot_be_count_only(self):
        f = self.fixture()
        for change in ('duplicate', 'failed', 'passed', 'ignored', 'missing-phase', 'missing-bin'):
            v = copy.deepcopy(f.value); t = v['tests']['synthetic']
            if change == 'duplicate': t['names'][1] = t['names'][0]
            elif change == 'failed': t['summaries'][0][1] = 1
            elif change == 'passed': v['tests_passed'] += 1
            elif change == 'ignored': t['ignored'] = 3
            elif change == 'missing-phase': v['phases'].pop('other-0')
            else: v['binaries'].pop('other-0')
            with self.subTest(change=change), self.assertRaises(RuntimeError): A.qualified(v)

    def test_selected_cargo_role_profile_and_exact_stream_are_required(self):
        for change in ('profile', 'path', 'manifest', 'duplicate', 'missing', 'stream-hash'):
            f = self.fixture()
            if change == 'profile': f.artifact['profile']['test'] = True
            elif change == 'path': f.artifact['executable'] = '/elsewhere/bin'
            elif change == 'manifest': f.artifact['manifest_path'] = '/elsewhere/Cargo.toml'
            elif change in ('duplicate', 'missing'):
                raw = json.dumps(f.artifact).encode() + b'\n'
                record = f.value['raw'][f.phase + '-stdout']
                raw = raw * 2 if change == 'duplicate' else b'{}\n'
                record.update(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
                f.bodies[record['path']] = raw
                f.value['phases'][f.phase]['stdout_sha256'] = record['sha256']
            else: f.value['phases'][f.phase]['stdout_sha256'] = 'e' * 64
            f.cpu = f.document(f.root / 'complete.json', f.value)
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.cpu_artifact(f.D, f.pins, f.cpu, 'worker')

    def test_source_snapshot_changes_or_forged_extent_refuse(self):
        for change in ('body', 'namespace', 'extent'):
            f = self.fixture(); record = f.value['raw']['sources-after.json']
            if change == 'body':
                record.update(f.document(Path(record['path']), {'changed': True}))
            elif change == 'namespace': record['path'] = str(f.root / 'other.json')
            else: record['bytes'] += 1
            f.cpu = f.document(f.root / 'complete.json', f.value)
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.cpu_artifact(f.D, f.pins, f.cpu, 'worker')

    def test_binary_digest_mode_and_nonelf_never_reach_auditor(self):
        for change in ('digest', 'mode', 'elf'):
            f = self.fixture()
            if change == 'digest': f.args[5] = 'e' * 64
            elif change == 'elf':
                raw = bytes(f.binary['bytes'])
                f.binary.update(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
                f.bodies[f.binary['path']] = raw
                f.cpu = f.document(f.root / 'complete.json', f.value)
                f.args[3], f.args[5] = f.cpu['sha256'], f.binary['sha256']
            with self.subTest(change=change), patch.object(A.os, 'access', return_value=change != 'mode'), \
                    self.assertRaises(RuntimeError):
                A.selected(f.D, f.args, f.pins)

    def test_exact_prior_and_controller_namespace_cannot_change(self):
        for change in ('prior', 'controller'):
            f = self.fixture()
            if change == 'prior': f.value['prior_completion']['sha256'] = 'e' * 64
            else: f.value['controller']['path'] = str(A.E / 'other.py')
            f.cpu = f.document(f.root / 'complete.json', f.value)
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.cpu_artifact(f.D, f.pins, f.cpu, 'worker')

    def test_filepin_is_strict_and_does_not_accept_boolean_extent(self):
        f = self.fixture()
        for change in ('bool', 'extra', 'digest'):
            record = dict(f.cpu)
            if change == 'bool': record['bytes'] = True
            elif change == 'extra': record['accepted'] = True
            else: record['sha256'] = 'X' * 64
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                A.read_pin(f.pins, record)
        self.assertEqual(f.calls, [])


if __name__ == '__main__':
    unittest.main()
