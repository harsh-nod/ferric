import copy
import json
from pathlib import Path
import struct
import tempfile
import unittest
import reference as ref


class ReferenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.directory = self.root / "evidence"
        ref.prepare(self.directory)
        self.probe, self.worker, self.object, self.source = [self.root / name for name in ("probe", "worker", "object", "source")]
        for path in (self.probe, self.worker, self.object, self.source):
            path.write_bytes(path.name.encode())
        self.artifact = self.root / "artifact.json"
        self.artifact_data = {"schema": "ferric-static-publication-artifact-v1",
            "source_sha256": ref.digest(self.source.read_bytes()), "object_sha256": ref.digest(self.object.read_bytes()),
            "workgroup": [128, 1, 1], "grid": [256, 1, 1], "allocation_bytes": list(ref.SIZES),
            "metadata": {"symbol": "ferric_gfx950_static_publication_v1", "wavefront_size": 64}}
        self.artifact.write_bytes(ref.encoded(self.artifact_data))
        self.paths = (self.artifact, self.probe, self.worker, self.object, self.source)

    def run_record(self, name, case, ready=(0, 64)):
        run = self.directory / name / "run"
        run.mkdir(exist_ok=True)
        inputs, initial = ref.case_bytes(case)
        if case["shape"] != "valid":
            raw = [bytes([0xA7]) * 512, initial, inputs,
                   struct.pack("<256I", *([3] * 256)), bytes(1024)]
        else:
            flags = [2 if cell in ready else 1 for cell in range(128)]
            statuses = [1] * 128 + [2 if cell in ready else 0 for cell in range(128)]
            bits = ref.words(inputs, 128)
            values = [0] * 128 + [bits[cell] if cell in ready else 0 for cell in range(128)]
            raw = [inputs, struct.pack("<128I", *flags), inputs,
                   struct.pack("<256I", *statuses), struct.pack("<256I", *values)]
        for filename, data in zip(ref.FILES, raw):
            (run / filename).write_bytes(data)
        report = {"schema": "ferric-static-publication-probe-v1", "authority": "none",
            "artifact": copy.deepcopy(self.artifact_data), "case": case,
            "case_sha256": ref.digest(ref.encoded(case)), "probe_sha256": ref.digest(self.probe.read_bytes()),
            "worker_sha256": ref.digest(self.worker.read_bytes()), "input_sha256": ref.digest(inputs),
            "initial_flags_sha256": ref.digest(initial), "buffer_sha256": [ref.digest(data) for data in raw],
            "completed_dispatches": 1, "fresh_worker_and_allocations": True,
            "input_immutability_and_all_guards_passed": True, "free_close_and_worker_exit_passed": True}
        (run / "report.json").write_bytes(ref.encoded(report))
        return run, report

    def evaluate(self, name):
        return ref.evaluate(self.directory / name / "case", self.directory / name / "run", *self.paths)

    def test_frozen_matrix_tagged_inputs_and_arbitrary_initial_bits(self):
        self.assertEqual(len(ref.cases()), 44)
        all_inputs = []
        for _, case in ref.cases()[:40]:
            inputs, _ = ref.case_bytes(case)
            self.assertEqual(len(set(ref.words(inputs, 128))), 128)
            all_inputs.append(inputs)
        self.assertEqual(len(set(all_inputs)), 40)
        ready = next(case for _, case in ref.cases() if case["pattern"] == "ready")
        self.assertEqual(ref.words(ref.case_bytes(ready)[1], 128), [2] * 128)

    def test_positive_exact_bits_and_not_ready_are_distinct(self):
        invalid = dict(ref.cases()[0][1], repetition=True)
        with self.assertRaises(ValueError):
            ref.case_bytes(invalid)

        name, case = ref.cases()[0]
        self.run_record(name, case)
        result = self.evaluate(name)
        self.assertEqual(result["ready_cells"], [0, 64])
        self.assertEqual(result["not_ready_count"], 126)
        self.run_record(name, case, ready=())
        self.assertEqual(self.evaluate(name)["status"], "no_ready_observed")

    def test_coverage_never_turns_all_not_ready_into_pass(self):
        for name, case in ref.cases():
            self.run_record(name, case, ready=())
        self.assertEqual(ref.summarize(self.directory, self.paths)["status"], "inconclusive")
        for name, case in ref.cases():
            self.run_record(name, case)
        summary = ref.summarize(self.directory, self.paths)
        self.assertEqual(summary["status"], "pass")
        self.assertEqual(summary["ready_count"], 80)
        for name, case in ref.cases():
            if case["pattern"] == "ready":
                self.run_record(name, case, ready=(0,))
        self.assertEqual(ref.summarize(self.directory, self.paths)["status"], "inconclusive")
        for name, case in ref.cases():
            self.run_record(name, case, ready=range(128))
        self.assertEqual(ref.summarize(self.directory, self.paths)["status"], "inconclusive")

    def test_invalid_shapes_preserve_all_protocol_bytes(self):
        for name, case in ref.cases()[40:]:
            run, report = self.run_record(name, case)
            self.assertEqual(self.evaluate(name)["ready_count"], 0)
            data = bytearray((run / ref.FILES[0]).read_bytes()); data[0] ^= 1
            (run / ref.FILES[0]).write_bytes(data); report["buffer_sha256"][0] = ref.digest(data)
            (run / "report.json").write_bytes(ref.encoded(report))
            with self.assertRaises(ValueError): self.evaluate(name)

    def test_numerical_mutants_fail_even_with_resealed_raw_hashes(self):
        name, case = ref.cases()[0]
        # Wrong Ready data/cell, fabricated status, negative-zero NotReady, stale flag,
        # changed producer payload, and changed immutable input all fail independently.
        for buffer, word, bits in [(4, 128, 0), (3, 128, 1), (4, 129, 0x80000000),
                                   (1, 0, 1), (1, 1, 3), (0, 0, 0), (2, 0, 0)]:
            run, report = self.run_record(name, case)
            data = bytearray((run / ref.FILES[buffer]).read_bytes())
            data[word * 4:word * 4 + 4] = struct.pack("<I", bits)
            (run / ref.FILES[buffer]).write_bytes(data)
            report["buffer_sha256"][buffer] = ref.digest(data)
            (run / "report.json").write_bytes(ref.encoded(report))
            with self.assertRaises(ValueError): self.evaluate(name)

    def test_lifecycle_artifact_reference_and_raw_corruption_fail(self):
        name, case = ref.cases()[0]
        for key, value in [("authority", "protected"), ("completed_dispatches", 0),
                           ("completed_dispatches", True), ("fresh_worker_and_allocations", False),
                           ("input_immutability_and_all_guards_passed", False),
                           ("free_close_and_worker_exit_passed", False), ("worker_sha256", "0" * 64)]:
            run, report = self.run_record(name, case); report[key] = value
            (run / "report.json").write_bytes(ref.encoded(report))
            with self.assertRaises(ValueError): self.evaluate(name)
        run, _ = self.run_record(name, case)
        (run / ref.FILES[4]).write_bytes(bytes(1024))
        with self.assertRaises(ValueError): self.evaluate(name)
        self.run_record(name, case)
        self.object.write_bytes(b"corrupt object")
        with self.assertRaises(ValueError): self.evaluate(name)
        self.object.write_bytes(b"object")
        expected = self.directory / name / "case" / "expected.json"
        record = json.loads(expected.read_bytes()); record["reference_sha256"] = "0" * 64
        expected.write_bytes(ref.encoded(record))
        with self.assertRaises(ValueError): self.evaluate(name)

    def test_artifact_mutation_rejected_even_when_report_is_changed_to_match(self):
        name, case = ref.cases()[0]
        run, report = self.run_record(name, case)
        forged = copy.deepcopy(self.artifact_data)
        forged["grid"] = [128, 1, 1]
        self.artifact.write_bytes(ref.encoded(forged))
        report["artifact"] = forged
        (run / "report.json").write_bytes(ref.encoded(report))
        with self.assertRaises(ValueError):
            self.evaluate(name)

if __name__ == "__main__":
    unittest.main()
