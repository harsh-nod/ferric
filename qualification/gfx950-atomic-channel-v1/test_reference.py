import json
from pathlib import Path
import struct
import tempfile
import unittest

import reference


class ReferenceTests(unittest.TestCase):
    def fixture(self, root, case_name="alternating"):
        case = root / "case"
        expected = reference.generate(case_name, case)
        run = root / "run"
        run.mkdir()
        probe = root / "probe"
        worker = root / "worker"
        probe.write_bytes(b"synthetic probe identity")
        worker.write_bytes(b"synthetic worker identity")
        artifact = {
            "schema": "ferric-atomic-channel-artifact-v1",
            "object_sha256": "11" * 32,
            "source_sha256": "22" * 32,
        }
        artifact_path = root / "artifact.json"
        artifact_path.write_text(json.dumps(artifact))
        data = (case / "inputs.u32le").read_bytes()
        (run / "channels.u32le").write_bytes(data)
        (run / "output.u32le").write_bytes(data)
        report = {
            "schema": "ferric-atomic-channel-probe-v1",
            "authority": "none",
            "artifact": artifact,
            "probe_sha256": reference.digest(probe.read_bytes()),
            "worker_sha256": reference.digest(worker.read_bytes()),
            "input_sha256": reference.digest(data),
            "channels_sha256": reference.digest(data),
            "output_sha256": reference.digest(data),
            "completed_dispatches": 1,
            "input_immutability_and_all_guards_passed": True,
            "free_close_and_worker_exit_passed": True,
        }
        (run / "report.json").write_text(json.dumps(report))
        return (case, run, artifact_path, probe, worker), report, expected

    def test_cases_are_deterministic_and_domain_is_exact(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in reference.CASES:
                first, second = root / (name + "-a"), root / (name + "-b")
                self.assertEqual(reference.generate(name, first),
                                 reference.generate(name, second))
                data = (first / "inputs.u32le").read_bytes()
                self.assertEqual(reference.decode_words(data),
                                 reference.reference(reference.decode_words(data)))
                with self.assertRaises(FileExistsError):
                    reference.generate(name, first)
        for values in ([0] * 255, [-1] * 256, [2**32] * 256, [True] * 256):
            with self.assertRaises(ValueError):
                reference.reference(values)
        for size in (0, 1023, 1025):
            with self.assertRaises(ValueError):
                reference.decode_words(bytes(size))
        self.assertEqual(reference.decode_words(struct.pack("<256I", *([0xFFFFFFFF] * 256))),
                         [0xFFFFFFFF] * 256)

    def test_complete_synthetic_reports_reproduce_exact_reference(self):
        for name in reference.CASES:
            with self.subTest(case=name), tempfile.TemporaryDirectory() as temporary:
                arguments, _, _ = self.fixture(Path(temporary), name)
                evidence = reference.check(*arguments)
                self.assertEqual(evidence["exact_word_comparisons"], 512)
                self.assertEqual(evidence["authority"], "none")
                self.assertIn("not cross-workgroup", evidence["scope"])
                with self.assertRaises(FileExistsError):
                    reference.check(*arguments)

    def test_raw_result_corruption_is_rejected_even_with_updated_hash(self):
        for filename, field in (("channels.u32le", "channels_sha256"),
                                ("output.u32le", "output_sha256")):
            for rehash in (False, True):
                with self.subTest(file=filename, rehash=rehash), tempfile.TemporaryDirectory() as temporary:
                    arguments, report, _ = self.fixture(Path(temporary))
                    run = arguments[1]
                    data = bytearray((run / filename).read_bytes())
                    data[-1] ^= 1
                    (run / filename).write_bytes(data)
                    if rehash:
                        report[field] = reference.digest(data)
                        (run / "report.json").write_text(json.dumps(report))
                    with self.assertRaises(ValueError):
                        reference.check(*arguments)
                    self.assertFalse((run / "numerical.json").exists())

    def test_report_and_reference_mutations_cannot_create_evidence(self):
        mutations = [
            ("authority", "production"),
            ("completed_dispatches", 0),
            ("completed_dispatches", 2),
            ("input_immutability_and_all_guards_passed", False),
            ("free_close_and_worker_exit_passed", False),
            ("input_sha256", "33" * 32),
            ("probe_sha256", "33" * 32),
            ("worker_sha256", "33" * 32),
        ]
        for field, value in mutations:
            with self.subTest(field=field, value=value), tempfile.TemporaryDirectory() as temporary:
                arguments, report, _ = self.fixture(Path(temporary))
                report[field] = value
                (arguments[1] / "report.json").write_text(json.dumps(report))
                with self.assertRaises(ValueError):
                    reference.check(*arguments)
                self.assertFalse((arguments[1] / "numerical.json").exists())
        with tempfile.TemporaryDirectory() as temporary:
            arguments, _, expected = self.fixture(Path(temporary))
            expected["expected_words"][0] ^= 1
            (arguments[0] / "expected.json").write_text(json.dumps(expected))
            with self.assertRaises(ValueError):
                reference.check(*arguments)
        with tempfile.TemporaryDirectory() as temporary:
            arguments, report, _ = self.fixture(Path(temporary))
            report["artifact"]["source_sha256"] = "44" * 32
            (arguments[1] / "report.json").write_text(json.dumps(report))
            with self.assertRaises(ValueError):
                reference.check(*arguments)


if __name__ == "__main__":
    unittest.main()
