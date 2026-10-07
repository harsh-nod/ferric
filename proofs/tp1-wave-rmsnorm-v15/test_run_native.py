"""Synthetic wrapper admission and argv checks; no worker or GPU execution."""
import copy
import hashlib
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import run_native as wrapper


class WrapperTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.checker = wrapper.load_pinned(Path(os.environ["FERRIC_PAIRED_CHECKER"]),
                                          wrapper.CHECKER_SHA, "rmsnorm_frozen_checker_test")

    def binding(self):
        return dict(schema="FerricTp1WaveRmsnormV15BindingsV1", authority="none",
            native_probe_sha256="1" * 64, source=wrapper.IMAGE_SOURCE, tree=wrapper.IMAGE_TREE,
            compiler_source=wrapper.COMPILER_SOURCE, compiler_tree=wrapper.COMPILER_TREE,
            emission_receipt_sha256="2" * 64,
            worker=dict(path=str(wrapper.WORKER), sha256=wrapper.WORKER_SHA, byte_len=1591144,
                source=wrapper.WORKER_SOURCE, tree=wrapper.WORKER_TREE,
                host_receipt_sha256=wrapper.WORKER_RECEIPT),
            image=dict(path=str(wrapper.STAGE / "fe2o3-engineering-v1" / ("3" * 64)),
                sha256="4" * 64, byte_len=11240, observation_sha256="5" * 64,
                handoff_sha256="6" * 64, root=wrapper.ROOT))

    def observation(self):
        image = self.binding()["image"]
        return dict(schema="EngineeringHsacoObservationV1", namespace="fe2o3-engineering-v1",
            authority="none", artifact="observation.hsaco",
            crate_name="ferric_qwen3_tp_wave_rmsnorm_kernels_device_v15",
            target="gfx950:xnack-", code_object_version=6,
            hsaco=dict(identity=dict(sha256=image["sha256"], byte_len=image["byte_len"]),
                       kernel_names=[wrapper.ROOT]),
            compiler_handoff=dict(sha256=image["handoff_sha256"], byte_len=40406),
            tools=dict(cargo_vendor=dict(git_sources=[
                dict(url="https://github.com/harsh-nod/fe2o3.git", rev=wrapper.COMPILER_SOURCE),
                dict(url="https://github.com/harsh-nod/pliron.git", rev="9de42fc6ca7b8f3500ccf2346d69ebbb36e889cd")])),
            execution=dict(exact_output_replay=True), options=dict(optimization="O2", verify_each=True),
            providers=[], grants=dict(publication=False, load=False, launch=False))

    def arguments(self):
        return ["--bindings", str(wrapper.STAGE / "v15-wrapper/bindings.json"),
            "--bindings-sha256", "7" * 64, "--runner-sha256", "8" * 64, "--tag", "v15-finite-r1"]

    def test_explicit_cli_and_closed_options(self):
        args = wrapper.options(self.arguments())
        self.assertEqual(args.tag, "v15-finite-r1")
        self.assertEqual(args.bindings.name, "bindings.json")
        for extra in (["--tag", "other"], ["--tag=other"], ["--run"], ["--bindings-sh", "a"]):
            with self.subTest(extra=extra), self.assertRaises((ValueError, SystemExit)):
                wrapper.options(self.arguments() + extra)
        for index, value in ((3, "0" * 64), (5, "A" * 64), (7, "../other"), (7, "a" * 65)):
            args = self.arguments()
            args[index] = value
            with self.subTest(index=index, value=value), self.assertRaises(ValueError):
                wrapper.options(args)
        with self.assertRaises(SystemExit):
            wrapper.options([])

    def test_wrong_host_rejected_before_loading_or_effects(self):
        with patch.object(wrapper.socket, "gethostname", return_value="synthetic-cpu"), \
                patch.object(wrapper, "load_pinned") as loader, patch.object(wrapper, "require_identity") as owned:
            with self.assertRaisesRegex(ValueError, "root mi350 only"):
                wrapper.main(self.arguments())
            loader.assert_not_called()
            owned.assert_not_called()

    def test_complete_bindings_and_exact_current_provenance(self):
        binding = self.binding()
        self.assertEqual(wrapper.validate_bindings(binding), Path(binding["image"]["path"]))
        self.assertNotEqual(wrapper.WORKER_SHA, self.checker.WORKER_SHA)
        for key in binding:
            changed = copy.deepcopy(binding)
            changed[key] = None
            with self.subTest(key=key), self.assertRaises((ValueError, TypeError)):
                wrapper.validate_bindings(changed)
        for key in ("source", "tree", "compiler_source", "compiler_tree"):
            changed = dict(binding, **{key: "a" * 40})
            with self.subTest(key=key), self.assertRaises(ValueError):
                wrapper.validate_bindings(changed)
        with self.assertRaises(ValueError):
            wrapper.validate_bindings(dict(binding, publication=True))

    def test_old_worker_identity_or_unbound_receipt_rejected(self):
        for key, value in (("sha256", self.checker.WORKER_SHA), ("source", self.checker.WORKER_SOURCE),
                           ("path", str(wrapper.STAGE / "old-worker")), ("tree", "a" * 40),
                           ("host_receipt_sha256", "0" * 64), ("byte_len", True)):
            changed = self.binding()
            changed["worker"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                wrapper.validate_bindings(changed)
        for key in ("native_probe_sha256", "emission_receipt_sha256"):
            with self.subTest(key=key), self.assertRaises(ValueError):
                wrapper.validate_bindings(dict(self.binding(), **{key: "0" * 64}))

    def test_single_image_extent_root_and_canonical_namespace(self):
        for key, value in (("root", "other"), ("byte_len", True), ("byte_len", 0),
                           ("byte_len", 4 * 1024**2 + 1), ("sha256", "0" * 64),
                           ("observation_sha256", "x" * 64), ("handoff_sha256", "0" * 64),
                           ("path", str(wrapper.STAGE / "candidate")),
                           ("path", str(wrapper.STAGE / "fe2o3-engineering-v1/../candidate"))):
            changed = self.binding()
            changed["image"][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                wrapper.validate_bindings(changed)
        changed = self.binding()
        changed["image"]["second_root"] = wrapper.ROOT
        with self.assertRaises(ValueError):
            wrapper.validate_bindings(changed)

    def test_observation_exact_sole_root_bytes_handoff_and_nonclaims(self):
        image = self.binding()["image"]
        wrapper.validate_observation(self.observation(), image, self.checker)
        mutations = [(("hsaco", "kernel_names"), [wrapper.ROOT, "other"]),
            (("hsaco", "kernel_names"), []), (("hsaco", "identity", "sha256"), "0" * 64),
            (("hsaco", "identity", "byte_len"), 1), (("compiler_handoff", "sha256"), "0" * 64),
            (("execution", "exact_output_replay"), 1), (("grants", "launch"), True),
            (("providers",), ["extra"]), (("target",), "gfx942"), (("code_object_version",), True),
            (("authority",), "launch"), (("namespace",), "other"), (("artifact",), "other.hsaco"),
            (("crate_name",), "other"), (("options", "verify_each"), 1), (("options", "optimization"), "O0")]
        for path, value in mutations:
            changed = self.observation()
            cursor = changed
            for key in path[:-1]:
                cursor = cursor[key]
            cursor[path[-1]] = value
            with self.subTest(path=path), self.assertRaises(ValueError):
                wrapper.validate_observation(changed, image, self.checker)

    def test_observation_current_vendor_revision_is_unique(self):
        for mutation in ("old", "duplicate", "missing"):
            changed = self.observation()
            rows = changed["tools"]["cargo_vendor"]["git_sources"]
            if mutation == "old":
                rows[0]["rev"] = wrapper.WORKER_SOURCE
            elif mutation == "duplicate":
                rows.append(dict(rows[0]))
            else:
                rows.pop(0)
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                wrapper.validate_observation(changed, self.binding()["image"], self.checker)

    def test_held_image_content_identity_and_exact_two_file_roster(self):
        binary = b"synthetic-image-not-executable"
        image = self.binding()["image"]
        image.update(sha256=hashlib.sha256(binary).hexdigest(), byte_len=len(binary))
        observation = self.observation()
        observation["hsaco"]["identity"] = dict(sha256=image["sha256"], byte_len=len(binary))
        manifest = json.dumps(observation).encode()
        image["observation_sha256"] = hashlib.sha256(manifest).hexdigest()
        content = hashlib.sha256(b"FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0")
        for raw in (manifest, binary):
            content.update(len(raw).to_bytes(8, "little"))
            content.update(raw)
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve() / content.hexdigest()
            directory.mkdir()
            (directory / "observation.json").write_bytes(manifest)
            (directory / "observation.hsaco").write_bytes(binary)
            base = SimpleNamespace(read_bound=lambda path, _bound: path.read_bytes())
            wrapper.validate_image(directory, image, base, self.checker)
            (directory / "extra").write_bytes(b"not admitted")
            with self.assertRaisesRegex(ValueError, "two-file"):
                wrapper.validate_image(directory, image, base, self.checker)
            (directory / "extra").unlink()
            wrong = directory.with_name("a" * 64)
            directory.rename(wrong)
            with self.assertRaisesRegex(ValueError, "content directory"):
                wrapper.validate_image(wrong, image, base, self.checker)
            (wrong / "observation.hsaco").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "image bytes"):
                wrapper.validate_image(wrong, image, base, self.checker)

    def test_command_binds_current_worker_single_image_and_operational_probe(self):
        here = wrapper.STAGE / "v15-wrapper/run_native.py"
        fixture, core = here.parent / "probe.py", wrapper.STAGE / "large-kv-frozen-helper.py"
        image = self.binding()["image"]
        command = wrapper.command_for(here, fixture, core, image, 101, wrapper.STAGE / "v15-finite-r1")
        self.assertEqual(command, [wrapper.sys.executable, "-I", "-B", str(here.parent / "native_probe.py"),
            "--fixtures", str(fixture), "--helper", str(core), "--worker", str(wrapper.WORKER),
            "--worker-sha256", wrapper.WORKER_SHA,
            "--artifact", str(Path(image["path"]) / "observation.hsaco"),
            "--artifact-sha256", image["sha256"], "--device-unique-id", "101",
            "--output", str(wrapper.STAGE / "v15-finite-r1/probe"), "--run", "--operational"])
        self.assertNotIn(self.checker.WORKER_SHA, command)
        self.assertEqual(command.count("--artifact"), 1)

    def test_normal_exit_and_owned_identity_are_fail_closed(self):
        wrapper.require_normal_exit(0, False)
        for status, forced in ((True, False), (1, False), (-9, False), (0, True), (0, 0), (0, None)):
            with self.subTest(status=status, forced=forced), self.assertRaises(ValueError):
                wrapper.require_normal_exit(status, forced)
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve()
            expected = wrapper.identity(directory.stat())
            wrapper.require_identity(directory, expected, directory=True)
            with self.assertRaises(ValueError):
                wrapper.require_identity(directory, (*expected[:3], expected[3] + 1), directory=True)
            link = directory / "link"
            link.symlink_to(directory)
            with self.assertRaises(ValueError):
                wrapper.require_identity(link, expected, directory=True)


if __name__ == "__main__":
    unittest.main()
