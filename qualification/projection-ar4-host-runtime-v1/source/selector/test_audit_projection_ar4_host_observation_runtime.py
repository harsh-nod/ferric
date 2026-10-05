"""Synthetic selector tests; real frozen contract, no audit or native process."""
import contextlib
import copy
import hashlib
import json
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch

import audit_projection_ar4_host_observation_runtime as A


class SelectorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        base = Path(__file__).resolve().parent.parent / A.GPU_PACKAGE[0]
        missing = object(); previous = sys.modules.get('layer_validation', missing)
        try:
            for name, digest in (('layer_validation', A.VALIDATOR_SHA), ('observer_cpu', A.OBSERVER_CPU_SHA)):
                path = base / (name + '.py'); raw = path.read_bytes()
                if hashlib.sha256(raw).hexdigest() != digest:
                    raise AssertionError('exact frozen test dependency: ' + name)
                module = types.ModuleType(name); module.__file__ = str(path)
                exec(compile(raw, str(path), 'exec'), module.__dict__)
                if name == 'layer_validation': sys.modules[name] = module
                else: cls.O = module
        finally:
            if previous is missing: sys.modules.pop('layer_validation', None)
            else: sys.modules['layer_validation'] = previous

    def fixture(self, role='worker'):
        root = A.E / A.CPU_LABEL; bodies, calls = {}, []
        def pin(path, raw):
            value = dict(path=str(path), bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
            bodies[str(path)] = raw
            return value
        def document(path, value):
            return pin(path, json.dumps(value, sort_keys=True).encode())
        natural = dict(exit_code=0, reason=None, group_absent=True)
        phases = {name: dict(natural) for name in ['worker-build', 'parent-builds'] +
                  ['phase-' + str(i) for i in range(59)]}
        raw = {name: dict(path=str(root / name), bytes=1, sha256='a' * 64) for name in
            self.O.SNAPSHOTS | {name + suffix for name in phases for suffix in
                ('-command.json', '-started.json', '-stdout', '-stderr', '-result.json')}}
        binaries = {}
        elf = bytes([127, 69, 76, 70, 2, 1]) + bytes(12) + bytes([62, 0])
        self.assertEqual(len(elf), 20)
        for name in (self.O.PARENT, self.O.PLAIN, self.O.WORKER):
            selected_role = 'worker' if name == self.O.WORKER else 'parent'
            source = root / 'sources/ferric/adapters' / (
                'tp-peer-finite-engineering-worker-v1' if selected_role == 'worker' else 'm1-engineering-execution-v1')
            binary = pin(root / 'target' / selected_role / 'debug' / name, elf)
            artifact = dict(reason='compiler-artifact', executable=binary['path'], filenames=[binary['path']],
                manifest_path=str(source / 'Cargo.toml'), features=[] if selected_role == 'worker' else ['tp-batch-engineering'],
                target=dict(name=name, kind=['bin'], crate_types=['bin'], src_path=str(source / 'src' /
                    ('main.rs' if selected_role == 'worker' else 'bin/' + name + '.rs'))),
                profile=dict(test=False, opt_level='2', debug_assertions=True, overflow_checks=True, debuginfo=0))
            binaries[name] = dict(binary=binary, artifact=artifact)
        for phase, names in (('worker-build', [self.O.WORKER]), ('parent-builds', [self.O.PLAIN, self.O.PARENT])):
            rows = [binaries[name]['artifact'] for name in names] + [dict(reason='build-finished', success=True)]
            stream = pin(root / (phase + '-stdout'), b''.join(json.dumps(row).encode() + b'\n' for row in rows))
            raw[phase + '-stdout'] = stream; phases[phase]['stdout_sha256'] = stream['sha256']
        paths = ['adapters/Cargo.toml'] + ['adapters/file' + str(i) + '.rs' for i in range(16)]
        base = {'ferric/' + name: dict(bytes=10, sha256='c' * 64) for name in paths[:11]}
        proposal = dict(schema='ferric-p228-projection-ar4-host-observation-source-proposal-v1',
            base_cpu=dict(A.PRIOR_CPU), files=[dict(path=name, before=base.get('ferric/' + name),
                after=dict(bytes=11, sha256='d' * 64)) for name in paths])
        unformatted = {'ferric/' + row['path']: row['after'] for row in proposal['files']}
        formatted = copy.deepcopy(unformatted); formatted['ferric/adapters/file0.rs']['sha256'] = 'e' * 64
        for name, contents in [('base', base), ('unformatted', unformatted), ('before', formatted), ('after', formatted)]:
            raw['sources-' + name + '.json'] = document(root / ('sources-' + name + '.json'), contents)
        proposal_pin = document(A.E / 'p228-projection-ar4-host-observation-v1/source-manifest.json', proposal)
        controller = pin(A.E / 'p228-projection-ar4-host-observation-cpu-v1/run.py', b'# synthetic controller\n')
        outcomes = lambda prefix, passed, ignored, summaries: dict(passed=passed, ignored=ignored,
            names=[prefix + str(i) for i in range(passed + ignored)], summaries=summaries)
        value = dict(schema='ferric-p228-projection-ar4-host-observation-cpu-result-v1', passed=True,
            error=None, postcheck_errors=[], source_unchanged=True, empty_initial_target=True,
            prior_completion=dict(A.PRIOR_CPU), controller=controller, proposal=proposal_pin,
            metadata=dict(parent={}, worker={}), phases=phases, raw=raw, binaries=binaries,
            tests={'worker-tests': outcomes('worker::', 518, 4, [[505, 0, 4], [13, 0, 0]]),
                   'parent-client': outcomes('parent::', 337, 0, [[337, 0, 0]])},
            tests_passed=855, tests_ignored=4, historical_runtime_tests_not_repeated=208)
        value.update({key: False for key in ('gpu_execution', 'numerical_acceptance', 'performance_claim',
            'production_authority', 'full_cpu1037_cohort_requalified', 'compiler_requalified')})
        cpu = document(root / 'complete.json', value)
        binary = binaries[A.NAMES[role]]['binary']
        def read(path, expected=None, retain=False, maximum=1 << 30):
            body = bodies[str(path)]
            result = dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())
            A.require(len(body) <= maximum and (expected is None or expected == result['sha256']), 'synthetic pin check')
            calls.append(str(path))
            return result, body if retain else b''
        return types.SimpleNamespace(root=root, role=role, bodies=bodies, calls=calls, document=document, pin=pin,
            value=value, cpu=cpu, binary=binary, proposal=proposal, base=base, formatted=formatted,
            D=types.SimpleNamespace(parse=json.loads), pins=types.SimpleNamespace(read=read))

    @contextlib.contextmanager
    def bound(self, f):
        # Only synthetic data pins are substituted. The real frozen contract remains in use.
        f.cpu = f.document(f.root / 'complete.json', f.value)
        products = {role: (f.value['binaries'][name]['binary']['bytes'],
                          f.value['binaries'][name]['binary']['sha256']) for role, name in A.NAMES.items()}
        with patch.object(A, 'CPU', (f.cpu['bytes'], f.cpu['sha256'])), \
                patch.object(A, 'CONTROLLER_SHA', f.value['controller']['sha256']), \
                patch.object(A, 'PROPOSAL_SHA', f.value['proposal']['sha256']), \
                patch.object(A, 'BASE_SHA', f.value['raw']['sources-base.json']['sha256']), \
                patch.dict(A.BINARIES, products), patch.object(A, 'contract_module', return_value=self.O):
            yield

    def artifact(self, f):
        with self.bound(f): return A.cpu_artifact(f.D, f.pins, f.cpu, f.role)

    def args(self, f):
        return ['projection-ar4-host-observation-runtime-' + f.role + '-v228-v1', f.role,
                f.cpu['path'], f.cpu['sha256'], f.binary['path'], f.binary['sha256']]

    def test_both_roles_real_contract_nine_metadata_bodies_and_only_selected_elf(self):
        for role in A.NAMES:
            f = self.fixture(role)
            with self.bound(f), patch.object(A.os, 'access', return_value=True):
                self.assertEqual(A.selected(f.D, self.args(f), f.pins), (A.E / self.args(f)[0], f.binary))
            self.assertEqual(len(set(f.calls)), 10)
            for name in ('sources-base.json', 'sources-unformatted.json', 'sources-before.json',
                         'sources-after.json', 'parent-builds-stdout', 'worker-build-stdout'):
                self.assertIn(str(f.root / name), f.calls)
            for name, row in f.value['binaries'].items():
                self.assertEqual(row['binary']['path'] in f.calls, name == A.NAMES[role])

    def test_closed_role_namespace_digest_and_plain_parent_before_reads(self):
        f = self.fixture('parent'); args = self.args(f)
        bad = [args[:-1], args + ['extra']]
        for index, value in [(0, 'projection-ar4-decode-runtime-parent-v228-v1'), (1, 'plain'),
                (2, str(A.E / 'projection-ar4-cpu-v228-v1/complete.json')), (3, 'A' * 64),
                (4, str(f.root / 'target/parent/debug' / self.O.PLAIN)),
                (4, args[4].replace('/parent/', '/worker/')), (5, 'bad')]:
            row = list(args); row[index] = value; bad.append(row)
        for row in bad:
            with self.assertRaises(RuntimeError): A.selected(f.D, row, f.pins)
        self.assertEqual(f.calls, [])

    def test_actual_cpu_pin_is_required_before_contract_loading(self):
        f = self.fixture()
        with patch.object(A, 'contract_module') as loader:
            with self.assertRaises(RuntimeError): A.cpu_artifact(f.D, f.pins, f.cpu, f.role)
            loader.assert_not_called()
        self.assertEqual(f.calls, [])

    def test_old_generation_counts_nonnatural_and_false_claims_refuse(self):
        for key, bad in [('schema', 'ferric-projection-ar4-cpu-result-v1'), ('tests_passed', 1037),
                         ('passed', False), ('postcheck_errors', ['drift']), ('full_cpu1037_cohort_requalified', True)]:
            f = self.fixture(); f.value[key] = bad
            with self.subTest(key=key), self.assertRaises(RuntimeError): self.artifact(f)
        f = self.fixture(); f.value['phases']['worker-build']['group_absent'] = False
        with self.assertRaises(RuntimeError): self.artifact(f)

    def test_worker_split_and_named_inventory_are_not_collapsed(self):
        for change in ('collapsed', 'duplicate'):
            f = self.fixture(); row = f.value['tests']['worker-tests']
            if change == 'collapsed': row['summaries'] = [[518, 0, 4]]
            else: row['names'][1] = row['names'][0]
            with self.subTest(change=change), self.assertRaises(RuntimeError): self.artifact(f)

    def test_original_controller_proposal_and_predecessor_required(self):
        for change in ('controller', 'proposal', 'prior'):
            f = self.fixture()
            if change == 'prior': f.value['prior_completion']['bytes'] += 1
            else: f.value[change]['path'] = str(A.E / 'elsewhere.json')
            with self.subTest(change=change), self.assertRaises(RuntimeError): self.artifact(f)

    def test_overlay_preimage_and_unformatted_body_must_match(self):
        for change in ('preimage', 'duplicate', 'unformatted'):
            f = self.fixture()
            if change == 'unformatted':
                name = 'sources-unformatted.json'; f.value['raw'][name] = f.document(f.root / name, f.base)
            else:
                if change == 'preimage': f.proposal['files'][0]['before']['bytes'] = 99
                else: f.proposal['files'][1]['path'] = f.proposal['files'][0]['path']
                f.value['proposal'] = f.document(Path(f.value['proposal']['path']), f.proposal)
            with self.subTest(change=change), self.assertRaises(RuntimeError): self.artifact(f)

    def test_cargo_not_formattable_and_final_source_drift_refuse(self):
        for change in ('cargo', 'after', 'path'):
            f = self.fixture(); bad = copy.deepcopy(f.formatted)
            if change == 'path': f.value['raw']['sources-after.json']['path'] = str(f.root / 'elsewhere.json')
            else:
                bad['ferric/adapters/Cargo.toml']['bytes'] += 1
                f.value['raw']['sources-after.json'] = f.document(f.root / 'sources-after.json', bad)
                if change == 'cargo': f.value['raw']['sources-before.json'] = f.document(f.root / 'sources-before.json', bad)
            with self.subTest(change=change), self.assertRaises(RuntimeError): self.artifact(f)

    def test_build_finished_and_unique_selected_artifacts_required(self):
        for change in ('duplicate', 'missing', 'failed', 'stream-hash'):
            f = self.fixture(); name = 'parent-builds-stdout'
            rows = [json.loads(line) for line in f.bodies[str(f.root / name)].splitlines()]
            if change == 'duplicate': rows.insert(0, rows[0])
            elif change == 'missing': rows.pop(0)
            elif change == 'failed': rows[-1]['success'] = False
            if change == 'stream-hash': f.value['phases']['parent-builds']['stdout_sha256'] = 'e' * 64
            else:
                record = f.pin(f.root / name, b''.join(json.dumps(row).encode() + b'\n' for row in rows))
                f.value['raw'][name] = record; f.value['phases']['parent-builds']['stdout_sha256'] = record['sha256']
            with self.subTest(change=change), self.assertRaises(RuntimeError): self.artifact(f)

    def test_cargo_harness_or_wrong_role_path_is_not_selected(self):
        for change in ('harness', 'role', 'features'):
            f = self.fixture('parent'); row = f.value['binaries'][self.O.PARENT]
            if change == 'harness': row['artifact']['profile']['test'] = True
            elif change == 'role': row['binary']['path'] = row['binary']['path'].replace('/parent/', '/worker/')
            else: row['artifact']['features'] = []
            with self.subTest(change=change), self.assertRaises(RuntimeError): self.artifact(f)

    def test_exact_two_observed_elf_pins_are_required(self):
        f = self.fixture()
        with self.bound(f), patch.dict(A.BINARIES, {'parent': (1, 'a' * 64)}), self.assertRaises(RuntimeError):
            A.cpu_artifact(f.D, f.pins, f.cpu, f.role)

    def test_real_elf_header_and_executable_mode_required(self):
        for change in ('mode', 'magic', 'machine', 'sha'):
            f = self.fixture()
            if change in ('magic', 'machine'):
                body = bytearray(f.bodies[f.binary['path']]); body[0 if change == 'magic' else 18] = 0
                f.binary.update(f.pin(Path(f.binary['path']), bytes(body)))
            with self.bound(f), patch.object(A.os, 'access', return_value=change != 'mode'):
                args = self.args(f)
                if change == 'sha': args[-1] = 'e' * 64
                with self.subTest(change=change), self.assertRaises(RuntimeError): A.selected(f.D, args, f.pins)

    def test_filepin_extent_digest_and_exact_shape_refuse(self):
        f = self.fixture()
        for change in ('bool', 'extent', 'sha', 'extra'):
            row = dict(f.cpu)
            if change == 'bool': row['bytes'] = True
            elif change == 'extent': row['bytes'] += 1
            elif change == 'sha': row['sha256'] = 'A' * 64
            else: row['accepted'] = True
            with self.subTest(change=change), self.assertRaises(RuntimeError): A.read_pin(f.pins, row)

    def test_contract_loader_pins_package_and_restores_alias_even_on_error(self):
        for prior_present in (False, True):
            for fail in (False, True):
                sentinel = object(); calls = []; pins = object()
                def package(observed, name, digest):
                    self.assertIs(observed, pins); self.assertEqual((name, digest), A.GPU_PACKAGE)
                    return A.E / name
                def load(observed, path, digest, name):
                    self.assertIs(observed, pins); calls.append((path.name, digest))
                    if path.name == 'layer_validation.py': return self.O.V
                    self.assertIs(sys.modules['layer_validation'], self.O.V)
                    if fail: raise RuntimeError('synthetic loader failure')
                    return self.O
                with patch.dict(sys.modules):
                    if prior_present: sys.modules['layer_validation'] = sentinel
                    else: sys.modules.pop('layer_validation', None)
                    D = types.SimpleNamespace(package=package, load_module=load)
                    if fail:
                        with self.assertRaises(RuntimeError): A.contract_module(D, pins)
                    else: self.assertIs(A.contract_module(D, pins), self.O)
                    if prior_present: self.assertIs(sys.modules['layer_validation'], sentinel)
                    else: self.assertNotIn('layer_validation', sys.modules)
                self.assertEqual(calls, [('layer_validation.py', A.VALIDATOR_SHA), ('observer_cpu.py', A.OBSERVER_CPU_SHA)])


if __name__ == '__main__':
    unittest.main(verbosity=2)
