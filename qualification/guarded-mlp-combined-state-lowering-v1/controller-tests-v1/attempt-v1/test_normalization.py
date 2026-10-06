"""Synthetic command/observation checks for the future bound lowering successor."""

import copy
import hashlib
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import lowering as L


FLAGS = ('-Zalways-encode-mir -Zinline-mir=yes '
         '-Zmir-enable-passes=-JumpThreading -Copt-level=3 -Ctarget-cpu=gfx950 '
         '-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,-xnack')


def pin(path):
    body = Path(path).read_bytes()
    return dict(path=str(path), bytes=len(body), sha256=hashlib.sha256(body).hexdigest())


def tool_rows():
    rows = {name: dict(path='/synthetic-tools/' + name, bytes=17, sha256='a' * 64)
            for name in L.NAMES}
    rows.update({name: dict(path=str(L.TOOLCHAIN / 'bin' / name), bytes=19, sha256='b' * 64)
                 for name in ('cargo', 'rustc')})
    return rows


def observation(tools, image):
    identity = lambda row: dict(sha256=row['sha256'], byte_len=row['bytes'])
    selected = {key: identity(tools[name]) for key, name in (
        ('cargo', 'cargo'), ('rustc', 'rustc'), ('host_linker', 'clang-22'),
        ('host_lld', 'lld'), ('host_lld_proxy', 'fe2o3-engineering-lld-proxy'),
        ('extractor', 'fe2o3-rustc-extract'), ('extractor_backend', 'librustc_codegen_fe2o3.so'))}
    selected.update(rustc_lib_tree_sha256='c' * 64,
                    cargo_vendor=dict(tree_sha256='d' * 64, git_sources=[]),
                    worker=dict(executable=identity(tools['fe2o3-llvm-link-worker']),
                                worker_build_identity=L.WORKER_BUILD,
                                llvm_build_identity=L.LLVM_BUILD))
    return dict(schema='EngineeringHsacoObservationV1', namespace='fe2o3-engineering-v1',
                authority='none', crate_name=L.CRATE, target='gfx950:xnack-',
                code_object_version=6, providers=[], tools=selected,
                grants=dict(publication=False, load=False, launch=False),
                options=dict(mir_normalization='optimized-inline-v1', extraction_rustflags=FLAGS,
                             optimization='O2', strip_debug=True, verify_each=True,
                             timeout_seconds=L.LEAF_SECONDS, maximum_output_bytes=L.STREAM_LIMIT),
                execution=dict(exact_output_replay=True),
                hsaco=dict(kernel_names=list(L.KERNELS),
                           identity=dict(sha256=hashlib.sha256(image).hexdigest(), byte_len=len(image))))


def run_output(value, tools):
    image = b'\x7fELFsynthetic-only-not-a-real-hsaco'
    manifest = json.dumps(value, sort_keys=True).encode()
    content = hashlib.sha256(b'FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0'
        + len(manifest).to_bytes(8, 'little') + manifest
        + len(image).to_bytes(8, 'little') + image).hexdigest()
    with tempfile.TemporaryDirectory() as temporary:
        out = Path(temporary).resolve()
        folder = out / 'fe2o3-engineering-v1' / content
        folder.mkdir(parents=True)
        (folder / 'observation.json').write_bytes(manifest)
        (folder / 'observation.hsaco').write_bytes(image)
        (out / 'compile.stdout').write_text(str(folder) + '\n')
        with patch.object(L, 'OUT', out):
            return L.output(SimpleNamespace(pin=pin), tools, [])


class NormalizationTests(unittest.TestCase):
    def test_command_selects_exact_profile_before_cargo_delimiter(self):
        tools = tool_rows()
        cpu = dict(tool_pins={name: tools[name] for name in ('cargo', 'rustc')})
        audit = dict(tools={name: dict(deployed=tools[name]) for name in L.NAMES})
        argv, env, selected = L.command(cpu, audit, [])
        self.assertEqual(argv.count('--mir-normalization'), 1)
        index = argv.index('--mir-normalization')
        self.assertEqual(argv[index + 1], 'optimized-inline-v1')
        self.assertLess(index, argv.index('--'))
        self.assertEqual(argv[argv.index('--') + 1:], [
            '--manifest-path', str(L.CPU / 'candidate/Cargo.toml'),
            '--lib', '--no-default-features', '--features', 'gfx950'])
        self.assertEqual(env, dict(PATH='/usr/bin:/bin', HOME=str(L.OUT), LANG='C',
                                  LC_ALL='C', TZ='UTC', TMPDIR=str(L.SCRATCH),
                                  CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='2'))
        self.assertEqual(selected, tools)

    def test_output_accepts_exact_seven_field_options(self):
        tools = tool_rows()
        value = observation(tools, b'\x7fELFsynthetic-only-not-a-real-hsaco')
        result = run_output(value, tools)
        self.assertEqual(result['value']['options'], value['options'])

    def test_output_refuses_old_missing_extra_or_minimal_options(self):
        tools = tool_rows()
        original = observation(tools, b'\x7fELFsynthetic-only-not-a-real-hsaco')
        variants = []
        for missing in ('mir_normalization', 'extraction_rustflags'):
            value = copy.deepcopy(original)
            del value['options'][missing]
            variants.append(value)
        value = copy.deepcopy(original)
        del value['options']['mir_normalization']
        del value['options']['extraction_rustflags']
        variants.append(value)
        for key, replacement in (('mir_normalization', 'minimal-v1'), ('extra', True)):
            value = copy.deepcopy(original)
            value['options'][key] = replacement
            variants.append(value)
        for value in variants:
            with self.subTest(options=value['options']), self.assertRaises(RuntimeError):
                run_output(value, tools)

    def test_output_refuses_flags_or_worker_policy_drift(self):
        tools = tool_rows()
        original = observation(tools, b'\x7fELFsynthetic-only-not-a-real-hsaco')
        variants = [("extraction_rustflags", FLAGS.replace(old, new)) for old, new in (
            ('inline-mir=yes', 'inline-mir=no'), ('opt-level=3', 'opt-level=0'),
            ('gfx950', 'gfx942'), ('-xnack', '+xnack'))]
        variants.extend([('optimization', 'O3'), ('verify_each', False),
                         ('strip_debug', False), ('timeout_seconds', L.LEAF_SECONDS + 1)])
        for key, replacement in variants:
            value = copy.deepcopy(original)
            value['options'][key] = replacement
            with self.subTest(key=key, replacement=replacement), self.assertRaises(RuntimeError):
                run_output(value, tools)


if __name__ == '__main__':
    unittest.main()
