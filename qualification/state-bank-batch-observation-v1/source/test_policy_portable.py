"""Data-only synthetic custody checks; no test runs a build, process or GPU."""
import copy
import hashlib
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import intake as I
import policy_portable as P


class PortableTests(unittest.TestCase):
    def store(self, directory, body=b'actual retained bytes'):
        objects = directory / 'objects'
        objects.mkdir()
        sha = hashlib.sha256(body).hexdigest()
        path = objects / sha
        path.write_bytes(body)
        original = dict(path='/build-host/not-present/source', bytes=len(body), sha256=sha)
        relocated = I.D.read_file(path)[0]
        aliases = {original['path']: dict(original=original, relocated=relocated)}
        return P.Store(I.D, I.D.Pins(), aliases, directory), original

    def test_relocated_bytes_never_need_original_build_host_path(self):
        with tempfile.TemporaryDirectory() as name:
            store, original = self.store(Path(name))
            self.assertEqual(store.get(original), b'actual retained bytes')
            self.assertEqual(store.used, {original['path']})

    def test_zero_length_raw_stderr_is_retained(self):
        with tempfile.TemporaryDirectory() as name:
            store, original = self.store(Path(name), b'')
            self.assertEqual(store.get(original), b'')

    def test_missing_alias_or_changed_original_refuses(self):
        with tempfile.TemporaryDirectory() as name:
            store, original = self.store(Path(name))
            for changed in (dict(original, path='/other'), dict(original, bytes=1),
                            dict(original, sha256='0' * 64)):
                with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                    store.get(changed)

    def test_relocated_body_mutation_refuses(self):
        with tempfile.TemporaryDirectory() as name:
            store, original = self.store(Path(name))
            Path(store.aliases[original['path']]['relocated']['path']).write_bytes(b'changed')
            with self.assertRaises(RuntimeError):
                store.get(original)

    def test_relocated_symlink_refuses(self):
        with tempfile.TemporaryDirectory() as name:
            store, original = self.store(Path(name))
            path = Path(store.aliases[original['path']]['relocated']['path'])
            destination = path.parent.parent / 'foreign'
            destination.write_bytes(path.read_bytes())
            path.unlink()
            path.symlink_to(destination)
            with self.assertRaises((RuntimeError, OSError)):
                store.get(original)

    def test_alias_roster_requires_exact_content_address(self):
        with tempfile.TemporaryDirectory() as name:
            directory = Path(name)
            store, original = self.store(directory)
            for kind in ('path', 'sha', 'extent', 'extra'):
                aliases = copy.deepcopy(store.aliases)
                row = aliases[original['path']]
                if kind == 'path': row['relocated']['path'] = '/elsewhere/object'
                elif kind == 'sha': row['relocated']['sha256'] = 'a' * 64
                elif kind == 'extent': row['relocated']['bytes'] += 1
                else: row['unreviewed'] = True
                with self.subTest(kind=kind), self.assertRaises(RuntimeError):
                    P.Store(I.D, I.D.Pins(), aliases, directory)

    def test_file_pins_reject_boolean_extent_and_traversal(self):
        value = dict(path='/actual/file', bytes=3, sha256='a' * 64)
        P.pin(value)
        for changed in (dict(value, bytes=True), dict(value, bytes=-1), dict(value, path='/a/../file'),
                        dict(value, path='relative'), dict(value, sha256='not-a-sha')):
            with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                P.pin(changed)

    def test_exact_cpu_recipes_are_cpu_only_and_keep_existing_limits(self):
        directory = Path('/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/host-policy-cpu-v228-v1')
        tools = dict(worker=dict(root='/nightly'), parent=dict(root='/stable'))
        recipes, worker, parent, target = P.recipes(directory, tools)
        self.assertEqual(len(recipes), 42)
        self.assertEqual(len({row[0] for row in recipes}), 42)
        self.assertEqual(target, directory / 'target')
        self.assertEqual(worker.parent.name, 'tp-peer-finite-engineering-worker-v1')
        self.assertEqual(parent.parent.name, 'm1-engineering-execution-v1')
        for name, role, argv, deadline in recipes:
            self.assertIn('--offline', argv)
            self.assertIn('--locked', argv)
            self.assertNotIn('--observe-host-policy', argv)
            self.assertIn(deadline, (120, 1200))
            self.assertIn(role, ('worker', 'parent'))
            if 'metadata' not in name:
                self.assertEqual(argv[argv.index('--jobs') + 1], '2')

    def test_closed_compiler_environments_route_exact_tools_and_disable_gpu(self):
        root = Path('/home/harmenon/ferric-asrock-42')
        directory = root / 'evidence/finite-resident-integration-v220/host-policy-cpu-v228-v1'
        tools = {role: dict(root=str(root / 'toolchain/rustup/toolchains' / name)) for role, name in (
            ('worker', 'nightly-2026-04-03-x86_64-unknown-linux-gnu'),
            ('parent', '1.97.1-x86_64-unknown-linux-gnu'))}
        worker = P.cpu_environment(directory, tools, 'worker')
        parent = P.cpu_environment(directory, tools, 'parent')
        for role, env in (('worker', worker), ('parent', parent)):
            compiler = Path(tools[role]['root'])
            self.assertEqual(env['PATH'], str(compiler / 'bin') + ':/usr/bin:/bin')
            self.assertEqual(env['RUSTC'], str(compiler / 'bin/rustc'))
            self.assertEqual(env['RUSTDOC'], str(compiler / 'bin/rustdoc'))
            self.assertEqual(env['CARGO_HOME'], str(root / 'toolchain/cargo'))
            self.assertEqual(env['CARGO_TARGET_DIR'], str(directory / 'target'))
            self.assertEqual(env['TMPDIR'], str(directory / 'tmp'))
            for key in ('HIP_VISIBLE_DEVICES', 'ROCR_VISIBLE_DEVICES', 'CUDA_VISIBLE_DEVICES'):
                self.assertEqual(env[key], '')
        self.assertNotIn('RUSTC_BOOTSTRAP', worker)
        self.assertNotIn('RUST_TEST_THREADS', worker)
        self.assertNotIn('LD_LIBRARY_PATH', parent)
        self.assertEqual(parent['RUSTC_BOOTSTRAP'], 'fe2o3_device,fe2o3_macros')
        self.assertEqual(parent['RUSTUP_TOOLCHAIN'], '1.97.1-x86_64-unknown-linux-gnu')
        self.assertEqual(worker['RUSTUP_TOOLCHAIN'], 'nightly-2026-04-03')
        self.assertEqual(worker['LD_LIBRARY_PATH'], str(directory / 'target/debug/deps') + ':'
                         + str(Path(tools['worker']['root']) / 'lib'))
        with self.assertRaises((RuntimeError, KeyError)):
            P.cpu_environment(directory, tools, 'unknown')

    def test_actual_named_test_inventory_and_outcomes_must_agree(self):
        names = P.test_inventory(b'first: test\nsecond: test\n')
        raw = b'test first ... ok\ntest second ... ignored\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 filtered out;\n'
        result = P.test_results(raw, names, [[1, 0, 1]])
        self.assertEqual((result['passed'], result['ignored']), (1, 1))
        for changed in (raw.replace(b'second', b'first'), raw.replace(b'... ok', b'... FAILED'),
                        raw.replace(b'1 passed', b'2 passed'), raw.replace(b'test second ... ignored\n', b'')):
            with self.assertRaises(RuntimeError):
                P.test_results(changed, names, [[1, 0, 1]])

    def test_duplicate_and_benchmark_inventories_refuse(self):
        for raw in (b'', b'a: test\na: test\n', b'a: test\nb: benchmark\n'):
            with self.assertRaises(RuntimeError):
                P.test_inventory(raw)

    def test_archive_source_maps_preserve_both_original_prefixes(self):
        bodies, rows = {}, {}
        for prefix in ('ferric', 'fe2o3'):
            stream = io.BytesIO()
            with tarfile.open(fileobj=stream, mode='w:gz') as tar:
                member = tarfile.TarInfo(prefix + '/Cargo.toml')
                member.size = 3
                tar.addfile(member, io.BytesIO(b'abc'))
            bodies[prefix] = stream.getvalue()
            rows[prefix] = dict(path='/' + prefix, bytes=len(bodies[prefix]), sha256=hashlib.sha256(bodies[prefix]).hexdigest(),
                                commit='a' * 40, tree='b' * 40)
        class Store:
            def get(self, record): return bodies[Path(record['path']).name]
        result = P.git_source_map(Store(), rows)
        expected = dict(bytes=3, sha256=hashlib.sha256(b'abc').hexdigest())
        self.assertEqual(result, {'ferric/Cargo.toml': expected, 'fe2o3/Cargo.toml': expected})

    def test_archive_traversal_and_links_refuse_without_extraction(self):
        for kind in ('traversal', 'link'):
            stream = io.BytesIO()
            with tarfile.open(fileobj=stream, mode='w:gz') as tar:
                member = tarfile.TarInfo('ferric/../outside' if kind == 'traversal' else 'ferric/link')
                if kind == 'link':
                    member.type = tarfile.SYMTYPE
                    member.linkname = '/outside'
                tar.addfile(member)
            raw = stream.getvalue()
            row = dict(path='/ferric', bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest(), commit='a' * 40, tree='b' * 40)
            class Store:
                def get(self, record): return raw
            with self.subTest(kind=kind), self.assertRaises(RuntimeError):
                P.git_source_map(Store(), dict(ferric=row, fe2o3=row))

    def test_old_cpu605_or_failed_candidate_cannot_be_relabelled(self):
        values = [dict(schema='ferric-p227-prefix-decode-host-cpu-v1', passed=True),
                  dict(schema=P.CPU_SCHEMA, passed=False)]
        class Store:
            def doc(self, _): return value
        for value in values:
            with self.assertRaises(RuntimeError):
                P.cpu_evidence(Store(), {})

    def test_unfrozen_cpu_controller_prevents_intake(self):
        class Store:
            def doc(self, _): return {}
        with patch.object(P, 'CPU_RUNNER_SHA', None), self.assertRaises(RuntimeError):
            P.cpu_evidence(Store(), {})


if __name__ == '__main__':
    unittest.main()
