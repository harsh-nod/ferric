#!/usr/bin/env python3
"""Synthetic, CPU-only contract fixtures. No GPU or model result is produced."""

from __future__ import annotations

import copy
from fractions import Fraction
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


PATH = Path(__file__).with_name("validate.py")
SPEC = importlib.util.spec_from_file_location("gfx950_measurements", PATH)
assert SPEC and SPEC.loader
v = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(v)


class ObservationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ferric-synthetic-qualification-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.document = self.fixture()

    def pin(self, name, value):
        raw = value if isinstance(value, bytes) else v.canonical(value)
        (self.root / name).write_bytes(raw)
        return {"path": name, "sha256": hashlib.sha256(raw).hexdigest(), "size_bytes": len(raw)}

    def fixture(self):
        provenance = {name: self.pin(name + ".txt", ("SYNTHETIC TEST ONLY: " + name).encode())
                      for name in v.PROVENANCE}
        case = {"model": "Qwen/Qwen3-0.6B", "batch": 1, "context": 1024, "precision": "bf16",
                "accumulation": "fp32", "metric": "decode-step-latency-ns", "output_tokens": 1}
        provenance["workload"] = self.pin("workload.json", {
            "scope": "device-smoke", "case": case, "seed": 42,
            "measurement_boundary": "submit-through-exact-completion",
            "prompt_trace": self.pin("prompts.txt", b"SYNTHETIC TEST PROMPT")})
        provenance["environment"] = self.pin("environment.json", {
            "target": "gfx950", "device_uuid": "SYNTHETIC-NOT-A-DEVICE", "device_count": 1,
            "host": "SYNTHETIC-NOT-A-HOST",
            **{name: self.pin(name + ".txt", ("SYNTHETIC: " + name).encode())
               for name in ("software", "hardware", "policy")}})
        reference = self.pin("reference.txt", b"SYNTHETIC INDEPENDENT REFERENCE TEST FIXTURE")
        variants = []
        for identifier, role, optimizations in [
            ("control", "ferric-command-batch", []),
            ("candidate", "megakernel", ["dispatch-consolidation"]),
        ]:
            executable = self.pin(identifier + ".hsaco", b"SYNTHETIC NOT HSACO: " + identifier.encode())
            numerical = self.pin(identifier + ".numerical.json", {
                "format": v.NUMERICAL_FORMAT, "scope": "device-smoke", "target": "gfx950",
                "passed": True, "tests": 3, "failed_tests": [], "comparison": "independent-reference",
                "executable_sha256": executable["sha256"], "reference_implementation": reference,
                **{name + "_sha256": provenance[name]["sha256"]
                   for name in ("workload", "model", "weights", "numerical_policy")}})
            variants.append({"id": identifier, "role": role, "scope": "device-smoke",
                             "optimizations": optimizations, "executable": executable,
                             "build_manifest": self.pin(identifier + ".build.txt", b"SYNTHETIC BUILD"),
                             "numerical_report": numerical,
                             **{name + "_sha256": provenance[name]["sha256"]
                                for name in ("environment", "workload", "tuning_budget")}})
        rows = []
        for phase, count in (("warmup", 10), ("recorded", 30)):
            for i in range(count):
                order = [variant["id"] for variant in variants]
                offset = i % len(order)
                rows.append({"phase": phase, "ordinal": i, "engine_order": order[offset:] + order[:offset],
                             "values": {variant["id"]: {
                                 "latency_ns": 1200 if variant["id"] == "control" else 1000,
                                 "clock_khz": 1_700_000, "temperature_millicelsius": 65_000,
                                 "passed": True, "faults": [],
                                 "numerical_report_sha256": variant["numerical_report"]["sha256"],
                                 "executable_sha256": variant["executable"]["sha256"]} for variant in variants}})
        return {"format": v.FORMAT, "scope": "device-smoke", "target": "gfx950",
                "authority": "engineering-observation-only", "qualification": False, "public_faster_claim": False,
                "case": case, "provenance": provenance, "variants": variants, "runs": rows,
                "ablations": [{"control": "control", "candidate": "candidate", "changed_optimization": "dispatch-consolidation"}],
                "interactions": [], "bound": None}

    def reject(self, mutate):
        document = copy.deepcopy(self.document)
        mutate(document)
        with self.assertRaises((v.Rejected, OSError)):
            v.validate(document, self.root)

    def edit_numerical(self, document, key, value):
        variant = document["variants"][1]
        pin = variant["numerical_report"]
        numerical = json.loads((self.root / pin["path"]).read_bytes())
        numerical[key] = value
        variant["numerical_report"] = self.pin(pin["path"], numerical)

    def add_variant(self, identifier, role, optimizations, latency):
        variant = copy.deepcopy(self.document["variants"][0])
        variant.update(id=identifier, role=role, optimizations=optimizations)
        self.document["variants"].append(variant)
        for row in self.document["runs"]:
            value = copy.deepcopy(row["values"]["control"])
            value["latency_ns"] = latency
            row["values"][identifier] = value
            order = [item["id"] for item in self.document["variants"]]
            offset = row["ordinal"] % len(order)
            row["engine_order"] = order[offset:] + order[:offset]

    def test_synthetic_arithmetic_never_authorizes_qualification(self):
        result = v.validate(self.document, self.root)
        self.assertFalse(result["qualification"])
        self.assertFalse(result["public_faster_claim"])
        self.assertEqual(result["scope"], "device-smoke")
        self.assertEqual(result["relative_to_command_batch"]["candidate"]["median_speedup"], {"numerator": 6, "denominator": 5})
        self.assertIsNone(result["fastest_measured_external_baseline"])
        self.assertEqual(result["counts"], {"warmup": 10, "recorded": 30})

    def test_cli_round_trip(self):
        self.pin("measurements.json", self.document)
        result = subprocess.run([sys.executable, "-I", str(PATH), str(self.root), "measurements.json"],
                                capture_output=True, check=True)
        self.assertFalse(json.loads(result.stdout)["qualification"])

    def test_cli_rejects_claim(self):
        self.document["qualification"] = True
        self.pin("measurements.json", self.document)
        result = subprocess.run([sys.executable, "-I", str(PATH), str(self.root), "measurements.json"], capture_output=True)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, b"")

    def test_duplicate_key(self):
        with self.assertRaises(v.Rejected):
            v.parse(b'{"a":1,"a":2}', "test")

    def test_float_nonfinite_and_noncanonical(self):
        for raw in [b'{"a":1.0}', b'{"a":NaN}', b'{"a":Infinity}', b'{"a":1}', b'{"a": 1e999}']:
            with self.subTest(raw=raw), self.assertRaises(v.Rejected):
                v.parse(raw, "test")

    def test_missing_and_extra_fields(self):
        self.reject(lambda d: d.pop("bound"))
        self.reject(lambda d: d.update(sota=True))
        self.reject(lambda d: d["runs"][0]["values"]["candidate"].update(summary_ns=10))

    def test_malformed_container_fields_are_contract_rejections(self):
        mutations = [
            lambda d: d.update(scope=[]),
            lambda d: d["case"].update(model={}),
            lambda d: d["variants"][0].update(role=[]),
            lambda d: d["runs"][0].update(phase={}),
            lambda d: d.update(case=[]),
            lambda d: d.update(provenance=[]),
            lambda d: d.update(variants={}),
            lambda d: d["variants"][0].update(optimizations={}),
            lambda d: d["variants"][0].update(id=[]),
            lambda d: d["runs"][0].update(values=[]),
            lambda d: d["ablations"][0].update(control=[]),
            lambda d: d["ablations"][0].update(changed_optimization=[]),
            lambda d: d.update(interactions={}),
            lambda d: d.update(bound=[]),
        ]
        for index, mutation in enumerate(mutations):
            with self.subTest(index=index):
                document = copy.deepcopy(self.document)
                mutation(document)
                with self.assertRaises(v.Rejected):
                    v.validate(document, self.root)

    def test_authority_escalation(self):
        for key, value in [("qualification", True), ("public_faster_claim", True), ("authority", "proved")]:
            with self.subTest(key=key):
                self.reject(lambda d: d.update({key: value}))

    def test_target_and_scope_drift(self):
        self.reject(lambda d: d.update(target="gfx942"))
        self.reject(lambda d: d.update(scope="full-decode"))
        self.reject(lambda d: d["variants"][1].update(scope="full-decode"))

    def test_semantic_workload_mismatch(self):
        self.reject(lambda d: d["case"].update(batch=8))
        self.reject(lambda d: d["variants"][1].update(workload_sha256="0" * 64))
        self.reject(lambda d: d["variants"][1].update(tuning_budget_sha256="0" * 64))

    def test_boolean_is_not_integer(self):
        self.reject(lambda d: d["case"].update(batch=True))
        self.reject(lambda d: d["runs"][0]["values"]["candidate"].update(latency_ns=True))

    def test_file_digest_and_size(self):
        self.reject(lambda d: d["provenance"]["ferric_source"].update(sha256="0" * 64))
        self.reject(lambda d: d["provenance"]["ferric_source"].update(size_bytes=1))

    def test_missing_file(self):
        self.reject(lambda d: d["provenance"]["ferric_source"].update(path="missing"))

    def test_path_traversal(self):
        for path in ("../escape", "/etc/passwd", "./model.txt", "a//b"):
            with self.subTest(path=path):
                self.reject(lambda d: d["provenance"]["model"].update(path=path))

    def test_symlink_and_hardlink(self):
        source = self.root / "model.txt"
        (self.root / "symlink.txt").symlink_to(source)
        self.reject(lambda d: d["provenance"]["model"].update(path="symlink.txt"))
        os.link(source, self.root / "hardlink.txt")
        self.reject(lambda d: None)

    def test_numerical_failure_and_wrong_reference(self):
        pin = self.document["variants"][1]["numerical_report"]
        original = (self.root / pin["path"]).read_bytes()
        for key, value in [("passed", False), ("failed_tests", ["logit"]), ("comparison", "self-reference"),
                           ("scope", "full-decode"), ("executable_sha256", "0" * 64),
                           ("workload_sha256", "0" * 64), ("tests", 0)]:
            with self.subTest(key=key):
                (self.root / pin["path"]).write_bytes(original)
                self.reject(lambda d: self.edit_numerical(d, key, value))

    def test_failed_or_faulted_sample(self):
        self.reject(lambda d: d["runs"][20]["values"]["candidate"].update(passed=False))
        self.reject(lambda d: d["runs"][20]["values"]["candidate"].update(faults=["timeout"]))

    def test_sample_identity_drift(self):
        self.reject(lambda d: d["runs"][20]["values"]["candidate"].update(executable_sha256="0" * 64))
        self.reject(lambda d: d["runs"][20]["values"]["candidate"].update(numerical_report_sha256="0" * 64))

    def test_sample_loss_order_and_warmup_policy(self):
        self.reject(lambda d: d["runs"].pop())
        self.reject(lambda d: d["runs"].pop(15))
        self.reject(lambda d: d["runs"][11].update(engine_order=["control", "candidate"]))
        self.reject(lambda d: d["runs"].append(d["runs"][0]))

    def test_noise_and_clock_drift(self):
        self.reject(lambda d: d["runs"][20]["values"]["candidate"].update(latency_ns=1100))
        self.reject(lambda d: d["runs"][20]["values"]["candidate"].update(clock_khz=2_000_000))
        self.reject(lambda d: d["runs"][20]["values"]["candidate"].update(temperature_millicelsius=80_000))

    def test_fractional_ppm_cannot_hide_variance_failure(self):
        for row in self.document["runs"]:
            row["values"]["candidate"]["latency_ns"] = 100_000_000
        self.reject(lambda d: d["runs"][20]["values"]["candidate"].update(latency_ns=102_000_001))

    def test_duplicate_variants_and_absent_control(self):
        self.reject(lambda d: d["variants"][1].update(id="control"))
        self.reject(lambda d: d["variants"][0].update(role="vllm"))

    def test_ablation_must_change_exactly_one_optimization(self):
        self.reject(lambda d: d["variants"][1]["optimizations"].append("software-pipeline"))
        self.reject(lambda d: d["ablations"][0].update(changed_optimization="software-pipeline"))
        self.reject(lambda d: d["ablations"].append(d["ablations"][0]))

    def test_interaction_rejects_missing_factor(self):
        self.reject(lambda d: d["interactions"].append({"control": "control", "a": "candidate", "b": "missing", "combined": "candidate"}))

    def test_factorial_interaction_arithmetic(self):
        self.document["variants"][1]["optimizations"] = ["dispatch-consolidation", "software-pipeline"]
        self.add_variant("factor-a", "ablation", ["dispatch-consolidation"], 1100)
        self.add_variant("factor-b", "ablation", ["software-pipeline"], 1000)
        for row in self.document["runs"]:
            row["values"]["candidate"]["latency_ns"] = 800
        self.document["ablations"] = [{"control": "control", "candidate": "factor-a", "changed_optimization": "dispatch-consolidation"},
                                       {"control": "factor-a", "candidate": "candidate", "changed_optimization": "software-pipeline"}]
        self.document["interactions"] = [{"control": "control", "a": "factor-a", "b": "factor-b", "combined": "candidate"}]
        result = v.validate(self.document, self.root)
        self.assertEqual(result["interactions"][0]["additional_saving_ns"], {"numerator": 100, "denominator": 1})
        self.reject(lambda d: d["variants"][3]["optimizations"].append("wrong-factor"))

    def test_fastest_supplied_baseline_not_sota(self):
        self.add_variant("vllm", "vllm", [], 1100)
        self.add_variant("sglang", "sglang", [], 900)
        result = v.validate(self.document, self.root)
        self.assertEqual(result["fastest_measured_external_baseline"], "sglang")
        self.assertEqual(result["candidate_vs_fastest_measured_external"]["median_speedup"], {"numerator": 9, "denominator": 10})
        self.assertFalse(result["public_faster_claim"])

    def test_bound_is_modeled_not_claimed_physical(self):
        self.document["bound"] = {
            "minimum_hbm_bytes": 1000, "operations": 200, "sustained_bytes_per_second": 2_000_000_000,
            "sustained_flops_per_second": 1_000_000_000, "critical_path_ns": 100,
            "calibration": self.pin("calibration.txt", b"SYNTHETIC CALIBRATION"),
            "derivation": self.pin("derivation.txt", b"SYNTHETIC DERIVATION")}
        bound = v.validate(self.document, self.root)["bound"]
        self.assertEqual(bound["modeled_floor_ns"], {"numerator": 500, "denominator": 1})
        self.assertEqual(bound["floor_over_median"]["candidate"], {"numerator": 1, "denominator": 2})
        self.assertFalse(bound["physical_lower_bound_proved"])
        self.assertFalse(bound["inconsistent_bound_requires_audit"])
        self.document["bound"]["minimum_hbm_bytes"] = 10_000
        self.assertTrue(v.validate(self.document, self.root)["bound"]["inconsistent_bound_requires_audit"])
        self.reject(lambda d: d["bound"].update(sustained_bytes_per_second=0))

    def test_bootstrap_is_paired_and_deterministic(self):
        control = [1000 + i for i in range(30)]
        candidate = [2 * value for value in control]
        self.assertEqual(v.paired_interval(control, candidate), (Fraction(1, 2), Fraction(1, 2)))
        unpaired = v.paired_interval(control, list(reversed(candidate)))
        self.assertLess(unpaired[0], Fraction(1, 2))
        self.assertGreater(unpaired[1], Fraction(1, 2))
        self.assertEqual(unpaired, v.paired_interval(control, list(reversed(candidate))))

    def test_power_of_two_sample_count_has_nondegenerate_bootstrap(self):
        control = [1000 + 5 * index for index in range(32)]
        candidate = [1000] * 32
        interval = v.paired_interval(control, candidate)
        self.assertLess(interval[0], interval[1])
        self.assertLess(interval[0], v.median(control) / v.median(candidate))
        self.assertGreater(interval[1], v.median(control) / v.median(candidate))
        self.assertEqual(interval, v.paired_interval(control, candidate))

    def test_strict_faster_threshold_and_nonregression(self):
        comparison = v.comparison([1050] * 30, [1000] * 30)
        self.assertFalse(comparison["engineering_faster_threshold_105"])
        self.assertTrue(comparison["nonregression_095"])
        self.assertFalse(v.comparison([900] * 30, [1000] * 30)["nonregression_095"])


if __name__ == "__main__":
    unittest.main()
