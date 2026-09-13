"""Synthetic profile/lifecycle tests only; never starts a worker."""
import copy
import hashlib
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import probe


class ProfileTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        proofs = Path(__file__).resolve().parents[1]
        cls.fixture_path = Path(os.environ.get("FERRIC_ATTENTION_FIXTURES",
            proofs / "tp1-attention-kernels-v1/probe.py"))
        cls.helper_path = Path(os.environ.get("FERRIC_ATTENTION_HELPER",
            proofs / "tensor-parallel-kernels-v1/probe.py"))
        cls.fixtures = probe.load_fixture(cls.fixture_path)
        cls.core = cls.fixtures.load_helper(cls.helper_path)
        cls.specs = tuple(probe.specifications())
        cls.cases = {spec: probe.make_case(cls.core, cls.fixtures, spec) for spec in cls.specs}

    def test_eight_ordered_cases_reuse_exact_frozen_inputs(self):
        self.assertEqual(self.specs, tuple((family, rows, role) for family, rows in
            (("uniform", 1), ("uniform", 17), ("uniform", 31), ("selector", 32))
            for role in ("resident", "candidate")))
        for left, right in zip(self.specs[::2], self.specs[1::2], strict=True):
            resident, candidate = self.cases[left], self.cases[right]
            old = self.fixtures.make_case(self.core, (*left[:2], "wave"))
            for case in (resident, candidate):
                probe.validate_case(self.fixtures, case)
                for key in ("groups", "scalars", "buffers"):
                    self.assertEqual(case[key], old[key])
            self.assertEqual(resident["buffers"], candidate["buffers"])
            self.assertNotEqual(resident["symbol"], candidate["symbol"])
        self.assertEqual(tuple(probe.ROOTS), tuple(record["root"] for record in probe.IMAGES.values()))

    def test_closed_specs_geometry_roots_and_tails(self):
        for spec in (("uniform", True, "resident"), ("uniform", 2, "candidate"),
                     ("selector", 32, "baseline"), ["uniform", 1, "resident"]):
            with self.subTest(spec=spec), self.assertRaises(ValueError):
                probe.make_case(self.core, self.fixtures, spec)
        original = self.cases[self.specs[1]]
        for mutation in ("root", "name", "extra", "rows", "context", "tail", "nonfinite"):
            case = copy.deepcopy(original)
            if mutation == "root":
                case["symbol"] = probe.ROOTS[0]
            elif mutation == "name":
                case["name"] += "_extra"
            elif mutation == "extra":
                case["extra"] = True
            elif mutation in ("rows", "context"):
                case["scalars"][0 if mutation == "rows" else 4] += 1
            elif mutation == "tail":
                case["buffers"][-1]["expected"] = case["buffers"][-1]["expected"][:-2] + b"\0\0"
            else:
                case["buffers"][0]["data"] = b"\x80\x7f" + case["buffers"][0]["data"][2:]
            with self.subTest(mutation=mutation), self.assertRaises((ValueError, RuntimeError)):
                probe.validate_case(self.fixtures, case)

    def test_generator_and_core_are_source_pinned(self):
        self.assertEqual(hashlib.sha256(self.fixture_path.read_bytes()).hexdigest(), probe.FIXTURE_SHA)
        self.assertEqual(hashlib.sha256(self.helper_path.read_bytes()).hexdigest(), probe.HELPER_SHA)
        with tempfile.TemporaryDirectory() as directory:
            changed = Path(directory) / "changed.py"
            changed.write_bytes(self.fixture_path.read_bytes() + b"\n")
            with self.assertRaises(ValueError):
                probe.load_fixture(changed)
            alias = Path(directory) / "alias.py"
            alias.symlink_to(self.fixture_path)
            with self.assertRaises(OSError):
                probe.load_fixture(alias)

    def worker_case(self, *, dispatch_error=False, close_error=False, payload=b""):
        worker = SimpleNamespace(command=Mock(return_value=({}, payload)),
                                 finish=Mock(), abort=Mock())
        if close_error:
            worker.finish.side_effect = RuntimeError("synthetic close failure")
        calls = []

        def dispatch(actual_worker, case, artifact, artifact_sha):
            self.assertIs(actual_worker, worker)
            calls.append((case["name"], case["symbol"], artifact, artifact_sha))
            if dispatch_error:
                raise RuntimeError("synthetic dispatch failure")
            return dict(name=case["name"], symbol=case["symbol"])

        core = SimpleNamespace(probe=dispatch)
        artifacts = {"resident": b"synthetic-resident", "candidate": b"synthetic-candidate"}
        return worker, core, artifacts, calls

    def dispatch_cached(self, worker, core, artifacts):
        with patch.object(probe, "make_case", side_effect=lambda _core, _fixtures, spec: self.cases[spec]):
            return probe.run_cases(worker, core, self.fixtures, artifacts)

    def test_exact_two_image_dispatch_and_normal_close(self):
        worker, core, artifacts, calls = self.worker_case()
        results = self.dispatch_cached(worker, core, artifacts)
        self.assertEqual(len(calls), 8)
        for spec, call, result in zip(self.specs, calls, results, strict=True):
            role = spec[2]
            self.assertEqual(call, (self.cases[spec]["name"], probe.IMAGES[role]["root"],
                                    artifacts[role], probe.IMAGES[role]["sha256"]))
            self.assertEqual(result["role"], role)
            self.assertEqual(result["artifact_sha256"], probe.IMAGES[role]["sha256"])
        worker.command.assert_called_once_with({"op": "configure_performance", "cache_kernel_admission": False,
            "operational_currentness": True, "profile": False}, expected="performance_configured")
        worker.finish.assert_called_once_with()
        worker.abort.assert_not_called()

    def test_configuration_failure_aborts_before_dispatch(self):
        worker, core, artifacts, calls = self.worker_case(payload=b"unexpected")
        with self.assertRaises(ValueError):
            self.dispatch_cached(worker, core, artifacts)
        self.assertEqual(calls, [])
        worker.finish.assert_not_called()
        worker.abort.assert_called_once_with()

    def test_dispatch_failure_aborts_without_admitted_results(self):
        worker, core, artifacts, calls = self.worker_case(dispatch_error=True)
        with self.assertRaises(RuntimeError):
            self.dispatch_cached(worker, core, artifacts)
        self.assertEqual(len(calls), 1)
        worker.finish.assert_not_called()
        worker.abort.assert_called_once_with()

    def test_close_failure_never_returns_admitted_results(self):
        worker, core, artifacts, calls = self.worker_case(close_error=True)
        with self.assertRaises(RuntimeError):
            self.dispatch_cached(worker, core, artifacts)
        self.assertEqual(len(calls), 8)
        worker.finish.assert_called_once_with()
        worker.abort.assert_called_once_with()

    def test_native_requires_explicit_operational_arguments(self):
        with patch.object(self.fixtures, "load_helper", return_value=self.core), \
             patch.object(probe, "load_fixture", return_value=self.fixtures), \
             patch.object(self.core, "Worker") as worker:
            with self.assertRaisesRegex(ValueError, "explicit operational"):
                probe.main(["--run", "--fixtures", str(self.fixture_path), "--helper", str(self.helper_path)])
            worker.assert_not_called()


if __name__ == "__main__":
    unittest.main()
