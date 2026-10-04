"""Pure/file-backed contracts only; subprocesses and platform probes are mocked."""
import ast
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tempfile
import types
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('runtime_audit_under_test', Path(__file__).with_name('audit.py'))
A = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(A)


def outcome(**updates):
    value = dict(exit_code=0, reason=None, owned_groups_absent=True,
                 owned_processes_reaped=True, cleanup_signalled=False, elapsed_seconds=.1,
                 deadline_seconds=30, owned_groups=[], lineage=[])
    value.update(updates); return value


def topology():
    return dict(monotonic=1, wall_time=2, devices=[dict(unique_id=uid, node=rank + 2,
        render_minor=128 + rank * 8, properties={'unique_id': uid}, direct_link={'node_to': 3 - rank},
        device_path='/sys/devices/test-%d' % rank, compute_partition='SPX', memory_partition='NPS1',
        gpu_busy=0, memory_busy=0, vram_used=17, vram_total=100) for rank, uid in enumerate(A.DEVICES)])


class AuditTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.binary = self.file('binary', b'\x7fELF\x02\x01test')
        self.lib = self.file('lib.so', b'actual library fixture')
        self.loader = self.file('ld-linux-x86-64.so.2', b'actual loader fixture')
        self.alias = self.base / 'alias.so'; self.alias.symlink_to(self.lib['path'])
        self.elf = ' 0x01 (NEEDED) Shared library: [libexample.so]\n'
        self.ldd = ('linux-vdso.so.1 (0x123)\nlibexample.so => %s (0x456)\n%s (0x789)\n'
                    % (self.alias, self.loader['path']))

    def file(self, name, raw):
        path = self.base / name; path.write_bytes(raw)
        return A.D.read_file(path)[0]

    def fake_runner(self, directory, argv, cwd, env, owned, save, deadline):
        self.assertEqual(argv[:7], ['/usr/bin/prlimit', '--as=2147483648', '--cpu=30',
            '--fsize=1048576', '--core=0', '--', '/usr/bin/readelf'] if argv[6] == '/usr/bin/readelf'
            else ['/usr/bin/prlimit', '--as=2147483648', '--cpu=30', '--fsize=1048576',
                  '--core=0', '--', '/usr/bin/ldd'])
        self.assertEqual(deadline, 30); self.assertEqual(env, A.ENV)
        raw = self.elf if argv[6] == '/usr/bin/readelf' else self.ldd
        (directory / 'stdout').write_text(raw); (directory / 'stderr').write_bytes(b'')
        save(directory / 'started.json', {'parent': {'pid': 999}, 'supervisor_pid': 888})
        return outcome()

    def test_closed_ldd_and_needed_roster(self):
        self.assertEqual(A.library_paths(self.elf, self.ldd), [str(self.alias), self.loader['path']])
        for elf, ldd in [(self.elf * 2, self.ldd), ('', self.ldd), (self.elf, self.ldd * 2),
                (self.elf, self.ldd.replace('libexample.so', 'different.so')),
                (self.elf, self.ldd.replace('linux-vdso.so.1 (0x123)\n', '')),
                (self.elf, self.ldd + '\n'), (self.elf, self.ldd + 'libbad.so => not found\n')]:
            with self.subTest(elf=elf, ldd=ldd), self.assertRaises(RuntimeError):
                A.library_paths(elf, ldd)

    def test_actual_retained_direct_loader_satisfies_its_needed_identity(self):
        base = Path(__file__).with_name('fixtures')
        elf = (base / 'actual-readelf.stdout').read_bytes()
        ldd = (base / 'actual-ldd.stdout').read_bytes()
        self.assertEqual((len(elf), hashlib.sha256(elf).hexdigest()),
            (1615, 'c4c154492d1bc30d10731adb430915f5a251e3369e379b0c525fa25d52deb48b'))
        self.assertEqual((len(ldd), hashlib.sha256(ldd).hexdigest()),
            (230, '8614b3c271fa6997a2d0096563f3d82c040021118e4ac64a9466870cbee23940'))
        self.assertEqual(A.library_paths(elf.decode(), ldd.decode()), [
            '/lib/x86_64-linux-gnu/libgcc_s.so.1', '/lib/x86_64-linux-gnu/libc.so.6',
            '/lib64/ld-linux-x86-64.so.2'])

    def test_direct_loader_does_not_allow_missing_unknown_or_duplicate_dependencies(self):
        base = Path(__file__).with_name('fixtures')
        elf = (base / 'actual-readelf.stdout').read_text()
        ldd = (base / 'actual-ldd.stdout').read_text()
        missing = '\n'.join(line for line in ldd.splitlines() if '/lib64/' not in line) + '\n'
        variants = [missing, ldd.replace('/lib64/ld-linux-x86-64.so.2', '/lib64/unknown-loader.so'),
            ldd + 'ld-linux-x86-64.so.2 => /different/loader (0x111)\n',
            ldd + '/other/ld-linux-x86-64.so.2 (0x111)\n',
            ldd + 'libother.so => /lib/x86_64-linux-gnu/libc.so.6 (0x111)\n',
            ldd + 'libc.so.6 => /different/libc.so.6 (0x111)\n',
            ldd.replace('libgcc_s.so.1 =>', 'libgcc_missing.so =>')]
        for text in variants:
            with self.subTest(text=text), self.assertRaises(RuntimeError):
                A.library_paths(elf, text)

    def test_library_aliases_pin_canonical_bytes_and_refuse_drift(self):
        pins = A.D.Pins(); rows = A.canonical_libraries(pins, [str(self.alias)])
        self.assertEqual(rows, [dict(path=str(self.alias), resolved=self.lib)])
        A.alias_guard(rows); pins.recheck()
        self.alias.unlink(); self.alias.symlink_to(self.loader['path'])
        with self.assertRaises(RuntimeError): A.alias_guard(rows)
        Path(self.lib['path']).write_bytes(b'changed')
        with self.assertRaises(RuntimeError): pins.recheck()

    def test_pin_extent_digest_and_types_are_exact(self):
        self.assertEqual(A.read_pin(A.D.Pins(), self.binary, 1024, True), b'\x7fELF\x02\x01test')
        for updates in ({'bytes': True}, {'bytes': self.binary['bytes'] + 1}, {'sha256': '0' * 64},
                        {'path': str(self.alias)}, {'extra': 1}):
            with self.subTest(updates=updates), self.assertRaises((RuntimeError, FileNotFoundError)):
                A.read_pin(A.D.Pins(), dict(self.binary, **updates), 1024)

    def test_topology_identity_matches_existing_observer_projection(self):
        sample = topology(); identity = A.topology_identity(sample)
        altered = copy.deepcopy(sample); altered['monotonic'] = 3
        for row in altered['devices']: row.update(gpu_busy=1, memory_busy=2, vram_used=20)
        self.assertEqual(A.topology_identity(altered), identity)
        altered['devices'][0]['vram_total'] = 200
        self.assertNotEqual(A.topology_identity(altered), identity)
        altered['devices'].reverse()
        with self.assertRaises(RuntimeError): A.topology_identity(altered)

    def test_software_reads_are_observations_not_file_identity_or_review(self):
        row = A.observed_file(Path(self.lib['path']))
        self.assertEqual(row['observed_sha256'], self.lib['sha256'])
        self.assertNotIn('sha256', row)
        self.assertEqual(A.observed_file(self.base / 'absent'),
                         dict(path=str(self.base / 'absent'), present=False))
        dangling = self.base / 'dangling'; dangling.symlink_to(self.base / 'absent')
        with self.assertRaises(RuntimeError): A.observed_file(dangling)
        with patch.object(A, 'observed_file', side_effect=lambda p: dict(path=str(p), present=False)), \
                patch.object(Path, 'glob', return_value=[]):
            value = A.software_snapshot()
        self.assertFalse(value['reviewed']); self.assertFalse(value['compiler_queried'])
        self.assertFalse(value['gpu_execution']); self.assertNotIn('notes', value)

    def test_leaf_records_exact_logical_and_actual_argv_separately(self):
        with patch.object(A.D, 'run_coordinator', side_effect=self.fake_runner):
            value, raw = A.leaf(self.base, 'readelf', ['/usr/bin/readelf', '-d', self.binary['path']],
                                object(), lambda: None)
        audit = json.loads(Path(value['audit']['path']).read_bytes())
        self.assertEqual(set(audit), {'argv', 'exit_code', 'deadline_seconds', 'stdout', 'stderr'})
        self.assertEqual(audit['argv'], ['/usr/bin/readelf', '-d', self.binary['path']])
        self.assertEqual(raw, self.elf); self.assertEqual(audit['stderr']['bytes'], 0)
        owner = json.loads(Path(value['owner']['path']).read_bytes())
        self.assertEqual(owner['command'], value['command']); self.assertFalse(owner['gpu_execution'])
        command = json.loads(Path(value['command']['path']).read_bytes())
        self.assertEqual(command['argv'][6:], audit['argv']); self.assertEqual(command['stream_cap_bytes'], 1 << 20)

    def test_owner_failures_retained_before_audit_publication(self):
        for index, updates in enumerate((dict(exit_code=1), dict(reason='deadline'),
                dict(cleanup_signalled=True), dict(owned_groups_absent=False),
                dict(owned_processes_reaped=False))):
            out = self.base / str(index); out.mkdir()
            def failed(*args, **kwargs):
                self.fake_runner(*args, **kwargs); return outcome(**updates)
            with patch.object(A.D, 'run_coordinator', side_effect=failed), self.assertRaises(RuntimeError):
                A.leaf(out, 'ldd', ['/usr/bin/ldd', self.binary['path']], object(), lambda: None)
            self.assertTrue((out / 'ldd/owner.json').is_file())
            self.assertFalse((out / 'ldd/audit.json').exists())

    def test_stream_stderr_or_extent_failure_never_publishes_audit(self):
        for index, bad in enumerate(('stderr', 'stdout')):
            out = self.base / ('stream-%d' % index); out.mkdir()
            def stream_error(directory, *args, **kwargs):
                result = self.fake_runner(directory, *args, **kwargs)
                (directory / bad).write_bytes(b'x' if bad == 'stderr' else b'x' * ((1 << 20) + 1))
                return result
            with patch.object(A.D, 'run_coordinator', side_effect=stream_error), self.assertRaises(RuntimeError):
                A.leaf(out, 'ldd', ['/usr/bin/ldd', self.binary['path']], object(), lambda: None)
            self.assertTrue((out / 'ldd/owner.json').is_file())
            self.assertFalse((out / 'ldd/audit.json').exists())

    def execute_fixture(self, after=None, fail_second=False):
        pins = A.D.Pins(); A.read_pin(pins, self.binary, 1024)
        out = self.base / 'observations'
        topo = types.SimpleNamespace(topology_sample=unittest.mock.Mock(side_effect=[topology(), after or topology()]))
        real_canonical = A.canonical_libraries
        def libraries(p, paths):
            return [] if paths == ['/usr/bin/prlimit', '/usr/bin/readelf', '/usr/bin/ldd'] else real_canonical(p, paths)
        def runner(directory, *args, **kwargs):
            value = self.fake_runner(directory, *args, **kwargs)
            return outcome(exit_code=1) if fail_second and directory.name == 'ldd' else value
        with patch.object(A, 'host_identity', return_value=dict(host=A.HOST, boot_id='1' * 36)), \
                patch.object(A.shutil, 'disk_usage', return_value=types.SimpleNamespace(free=50 << 30)), \
                patch.object(A, 'canonical_libraries', side_effect=libraries), \
                patch.object(A, 'software_snapshot', return_value=dict(schema='synthetic-test-only', **A.FLAGS)), \
                patch.object(A.D, 'run_coordinator', side_effect=runner):
            return A.execute(out, self.binary, pins, object(), topo)

    def test_full_observation_does_not_mint_review_or_gpu_authority(self):
        record = self.execute_fixture(); value = json.loads(Path(record['path']).read_bytes())
        self.assertEqual(value['binary'], self.binary); self.assertEqual(len(value['libraries']), 2)
        self.assertEqual(len(value['owners']), 2); self.assertEqual(value['devices'], A.DEVICES)
        for key in A.FLAGS: self.assertEqual(value[key], A.FLAGS[key])
        self.assertNotIn('notes', value); self.assertNotIn('engineering_reviewed', value)
        self.assertEqual(value['topology_identity'], A.topology_identity(topology()))

    def test_late_topology_failure_retains_owned_records_without_complete(self):
        after = topology(); after['devices'][0]['compute_partition'] = 'CPX'
        with self.assertRaises(RuntimeError): self.execute_fixture(after=after)
        out = self.base / 'observations'
        self.assertTrue((out / 'readelf/owner.json').is_file()); self.assertTrue((out / 'ldd/owner.json').is_file())
        self.assertTrue((out / 'failed.json').is_file()); self.assertFalse((out / 'complete.json').exists())

    def test_second_leaf_failure_retains_failure_without_complete(self):
        with self.assertRaises(RuntimeError): self.execute_fixture(fail_second=True)
        out = self.base / 'observations'
        self.assertTrue((out / 'readelf/audit.json').is_file()); self.assertTrue((out / 'ldd/owner.json').is_file())
        self.assertFalse((out / 'ldd/audit.json').exists()); self.assertFalse((out / 'complete.json').exists())

    def test_actual_frozen_runtime_review_accepts_observation_fields_only_after_explicit_review(self):
        # Execute the exact pinned function, not a duplicate of its schema checks.
        parent = A.E / 'p227-prefix-parity-observation-v2'
        pin, raw = A.D.read_file(parent / 'prepare.py', retain=True, maximum=1 << 20)
        self.assertEqual(pin['sha256'], 'ed4b8770dc90d48f393bbdc292d3eb91ea441c8f3420f0b39f013db4f962dcf4')
        V = A.D.load_module(A.D.Pins(), parent / 'validation.py',
            'cdaf6dc53208bbca8f23b2a3fa3eca9d28fe00ca4cfc74b36d225f3ebe8d28d1', 'runtime_fixture_validation')
        node, = [n for n in ast.parse(raw).body if isinstance(n, ast.FunctionDef) and n.name == 'runtime_review']
        def read(pins, record, retain=False, maximum=64 << 20):
            return A.read_pin(pins, record, maximum, retain)
        scope = dict(V=V, Path=Path, re=re, read=read,
                     document=lambda p, r, maximum: A.D.parse(read(p, r, True, maximum)))
        exec(compile(ast.Module(body=[node], type_ignores=[]), str(parent / 'prepare.py'), 'exec'), scope)
        complete = json.loads(Path(self.execute_fixture()['path']).read_bytes())
        review = {k: complete[k] for k in ('host', 'boot_id', 'binary', 'readelf', 'ldd', 'libraries')}
        review.update(schema='ferric-p227-prefix-parity-runtime-review-v1', authority='none', reviewed=True,
                      notes='Synthetic test-only explicit review; not an actual engineering judgment.',
                      production_authority=False, gpu_execution=False)
        scope['runtime_review'](review, self.binary, review, A.D.Pins())
        review['reviewed'] = False
        with self.assertRaises(RuntimeError): scope['runtime_review'](review, self.binary, review, A.D.Pins())

    def test_independent_namespace_accepts_explicit_future_binary_pin_without_minting_evidence(self):
        label = 'prefix-independent-runtime-audit-v228-v1'
        path = A.E / 'prefix-independent-deployment-v228-v1' / A.BINARY
        parsed, binary = A.arguments([label, str(path), '1234', 'a' * 64])
        self.assertEqual(parsed, label)
        self.assertEqual(binary, dict(path=str(path), bytes=1234, sha256='a' * 64))
        self.assertFalse(A.FLAGS['reviewed'])
        self.assertFalse(A.FLAGS['gpu_execution'])
        self.assertFalse(A.FLAGS['numerical_acceptance'])

    def test_legacy_or_malformed_namespaces_and_extents_are_refused_before_any_host_probe(self):
        label = 'prefix-independent-runtime-audit-v228-v1'
        path = A.E / 'prefix-independent-deployment-v228-v1' / A.BINARY
        good = [label, str(path), '1234', 'a' * 64]
        variants = [good[:-1], good + ['extra'],
            ['prefix-runtime-audit-v227-v1', *good[1:]],
            ['prefix-independent-runtime-audit-v228-v0', *good[1:]],
            ['prefix-independent-runtime-audit-v228-v01', *good[1:]],
            [label, str(A.E / 'prefix-parity-deployment-v227-v1' / A.BINARY), *good[2:]],
            [label, str(path.with_name('wrong-binary')), *good[2:]],
            [label, str(A.E.parent / 'prefix-independent-deployment-v228-v1' / A.BINARY), *good[2:]],
            [*good[:2], '0', good[3]], [*good[:2], '01234', good[3]]]
        for args in variants:
            with patch.object(A, 'host_identity') as host, self.assertRaises(RuntimeError):
                A.main(args)
            host.assert_not_called()

    def test_main_uses_the_same_namespace_parser_tested_above(self):
        with patch.object(A, 'arguments', side_effect=RuntimeError('synthetic parser sentinel')) as parser:
            with self.assertRaisesRegex(RuntimeError, 'synthetic parser sentinel'):
                A.main(['synthetic'])
        parser.assert_called_once_with(['synthetic'])

    def test_all_non_namespace_functions_and_frozen_helpers_remain_unchanged(self):
        root = Path(__file__).resolve().parent
        original = (root / 'preimage/audit.py').read_bytes()
        self.assertEqual(hashlib.sha256(original).hexdigest(),
                         'def16c2f69c082fa273596bef0caf7a2af195d07d2ac6f96d944706d5f44a514')
        before = {row.name: ast.dump(row, include_attributes=False) for row in ast.parse(original).body
                  if isinstance(row, ast.FunctionDef) and row.name != 'main'}
        after = {row.name: ast.dump(row, include_attributes=False)
                 for row in ast.parse(Path(A.__file__).read_bytes()).body
                 if isinstance(row, ast.FunctionDef) and row.name not in ('main', 'arguments')}
        self.assertEqual(before, after)
        for name, digest in (('run_row_facts_v2.py', A.HELPER_SHA), ('frozen_owned.py', A.OWNED_SHA)):
            self.assertEqual(hashlib.sha256((root / name).read_bytes()).hexdigest(), digest)


if __name__ == '__main__':
    unittest.main()
