import argparse
import copy
from fractions import Fraction
import json
import math
from pathlib import Path
import struct
import tempfile
import unittest

import reference as ref


class RoundingTests(unittest.TestCase):
    def test_bf16_ties_and_off_ties_are_not_double_rounded(self):
        for word in (0x3F80, 0x3F81, 0xBF80, 0xBF81, 0x3900):
            first = ref.bf16(word)
            second = ref.adjacent(first, 1, True)
            midpoint = (Fraction(first) + Fraction(second)) / 2
            expected = word if not word & 1 else ref.float_bits(second) >> 16
            self.assertEqual(ref.bf16_word(midpoint), expected)
            self.assertEqual(ref.bf16_word(midpoint - Fraction(1, 2**70)), ref.float_bits(first) >> 16)
            self.assertEqual(ref.bf16_word(midpoint + Fraction(1, 2**70)), ref.float_bits(second) >> 16)
        self.assertEqual(ref.bf16_word(-0.0), 0x8000)

    def test_f32_rounding_matches_exact_ties(self):
        halfway = Fraction(1) + Fraction(1, 2**24)
        self.assertEqual(ref.rounded(halfway), 1.0)
        self.assertEqual(ref.rounded(halfway + Fraction(1, 2**80)), ref.adjacent(1.0, 1))
        self.assertEqual(ref.rounded(halfway - Fraction(1, 2**80)), 1.0)

    def test_sqrt_midpoint_selection_is_exact(self):
        for value in (ref.EPSILON, 1.0, 2.0, 3.5, 2**-40, 2**40):
            value = ref.rounded(value)
            result = ref.sqrt_rn(value)
            low = (Fraction(ref.adjacent(result, -1)) + Fraction(result)) / 2
            high = (Fraction(result) + Fraction(ref.adjacent(result, 1))) / 2
            self.assertLessEqual(low * low, Fraction(value))
            self.assertGreaterEqual(high * high, Fraction(value))
        self.assertEqual(ref.sqrt_rn(4.0), 2.0)

    def test_bf16_projection_midpoint_interval_has_both_images(self):
        midpoint = Fraction(1) + Fraction(1, 256)
        low = ref.rounded(midpoint - Fraction(1, 2**40), True)
        high = ref.rounded(midpoint + Fraction(1, 2**40), True)
        self.assertEqual((low, high), (1.0, ref.bf16(0x3F81)))


class NumericalTests(unittest.TestCase):
    def test_signed_zero_and_strict_policy(self):
        self.assertEqual(ref.POLICY["sqrt_ulp_acceptance"], 0)
        self.assertEqual(ref.POLICY["reciprocal_ulp_acceptance"], 0)
        self.assertEqual(ref.float_bits(ref.multiply(-0.0, 1.0)), 0x80000000)
        intervals = ref.norm_intervals([(-0.0, -0.0)] * 1024, [0x3F80] * 128)
        self.assertTrue(all(ref.bf16_word(left) == ref.bf16_word(right) == 0x8000 for left, right in intervals))
        values = ([0.0] * 1024, [0.0] * 1024, [(0.0, 0.0)] * 1024,
                  [0x3F80] * 128, [(0.0, 0.0)] * 1024, [0] * 1024)
        with self.assertRaisesRegex(ValueError, "conditional"):
            ref.compare_outputs(bytes(4096), bytes(2048), struct.pack("<1024H", *([0x8000] * 1024)), values, [])

    @staticmethod
    def fixture():
        keys = [0x3F00 + (index * 37) % 256 for index in range(ref.K)]
        gamma = [0x4022] * ref.HEAD  # A non-unit value present in the real gamma tensor.
        values = [ref.bf16(word) for word in keys]
        intervals = ref.norm_intervals([(value, value) for value in values], gamma)
        output = [ref.bf16_word((left + right) / 2) for left, right in intervals]
        center = ref.model_center(keys, gamma)
        reference = (values, [0.0] * ref.K, [(value, value) for value in values], gamma, intervals, center)
        raw = (struct.pack("<1024f", *values), struct.pack("<1024H", *keys), struct.pack("<1024H", *output))
        return keys, gamma, reference, raw

    def test_conditional_and_composed_checks_keep_each_stage(self):
        _, _, values, raw = self.fixture()
        report = ref.compare_outputs(*raw, values, list(range(ref.K)))
        self.assertEqual(report["projection_values_checked"], 1024)
        self.assertEqual(report["quantized_values_checked"], 1024)
        self.assertEqual(report["conditional_norm_values_checked"], 1024)
        self.assertGreater(report["conditional_singleton_intervals"], 1000)

    def test_wrong_pre_gamma_cast_is_detected(self):
        keys, gamma, values, raw = self.fixture()
        wrong = struct.pack("<1024H", *ref.model_center(keys, gamma, wrong_cast=True))
        self.assertNotEqual(raw[2], wrong)
        with self.assertRaisesRegex(ValueError, "conditional"):
            ref.compare_outputs(raw[0], raw[1], wrong, values, list(range(ref.K)))

    def test_projection_quantization_output_and_extent_corruption(self):
        _, _, values, raw = self.fixture()
        for target in range(3):
            changed = list(raw)
            if target == 0:
                changed[target] = struct.pack("<f", 100.0) + changed[target][4:]
            else:
                changed[target] = struct.pack("<H", 0x4200) + changed[target][2:]
            with self.subTest(target=target), self.assertRaises(ValueError):
                ref.compare_outputs(*changed, values, list(range(ref.K)))
            changed = list(raw)
            changed[target] = changed[target][:-1]
            with self.subTest(short=target), self.assertRaises(ValueError):
                ref.compare_outputs(*changed, values, list(range(ref.K)))
        changed = struct.pack("<f", math.nan) + raw[0][4:]
        with self.assertRaisesRegex(ValueError, "nonfinite"):
            ref.compare_outputs(changed, raw[1], raw[2], values, [])

    def test_zero_and_epsilon_dominant_norm(self):
        gamma = [0x4022] * 128
        self.assertEqual(ref.norm_intervals([(0.0, 0.0)] * 1024, gamma), [(0.0, 0.0)] * 1024)
        value = ref.bf16(0x3580)
        intervals = ref.norm_intervals([(value, value)] * 1024, gamma)
        expected = ref.model_center([0x3580] * 1024, gamma)
        self.assertTrue(all(left <= ref.bf16(word) <= right for word, (left, right) in zip(expected, intervals)))
        self.assertTrue(all(right < 0.01 for _, right in intervals))

    def test_nonfinite_and_overflow_domains_reject(self):
        for value in (math.inf, math.nan, 2**21):
            with self.subTest(value=value), self.assertRaises(ValueError):
                ref.norm_intervals([(value, value)] * 1024, [0x3F80] * 128)


class EvidenceTests(unittest.TestCase):
    def test_corrupt_artifact_source_and_graph_reject(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            args = argparse.Namespace(artifact=root / "artifact.json")
            artifact = {"schema": "ferric-qwen3-knorm-chain-artifact-v1", "graph": ref.GRAPH}
            for role in ("producer", "consumer"):
                artifact[role] = {}
                for kind in ("source", "object"):
                    path = root / (role + kind)
                    path.write_bytes((role + kind).encode())
                    setattr(args, role + "_" + kind, path)
                    artifact[role][kind + "_sha256"] = ref.file_hash(path)
            for name in ("probe", "worker"):
                path = root / name
                path.write_bytes(name.encode())
                setattr(args, name, path)
            args.artifact.write_bytes(ref.json_bytes(artifact))
            ref.identities(args)
            mutated = copy.deepcopy(artifact)
            mutated["graph"]["intermediate_reupload"] = True
            args.artifact.write_bytes(ref.json_bytes(mutated))
            with self.assertRaisesRegex(ValueError, "graph"):
                ref.identities(args)
            args.artifact.write_bytes(ref.json_bytes(artifact))
            args.consumer_object.write_bytes(b"corrupt")
            with self.assertRaisesRegex(ValueError, "consumer object"):
                ref.identities(args)

    def test_reference_record_and_bytes_are_recomputed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            files = {"projection.f64le": b"expected"}
            (root / "projection.f64le").write_bytes(b"expected")
            record = {"policy": ref.POLICY}
            ref.check_frozen(copy.deepcopy(record), record, files, root)
            changed = copy.deepcopy(record)
            changed["policy"]["sqrt_ulp_acceptance"] += 1
            with self.assertRaisesRegex(ValueError, "identity"):
                ref.check_frozen(changed, record, files, root)
            (root / "projection.f64le").write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "bytes"):
                ref.check_frozen(record, record, files, root)

    def test_report_hash_lifecycle_and_authority_mutants(self):
        frozen = {"probe_sha256": "probe", "worker_sha256": "worker"}
        artifact = {"fixed": True}
        record = {"files": {"inputs.bf16le": "input"}}
        raw = {"projection": b"p", "quantized": b"q", "output": b"o"}
        report = {"schema": "ferric-qwen3-knorm-chain-probe-v1", "authority": "none",
                  "artifact": artifact, "completed_dispatches": 2, **frozen,
                  "input_sha256": "input", "weights_sha256": ref.WEIGHTS_HASH,
                  "norm_weights_sha256": ref.GAMMA_HASH,
                  **{key + "_sha256": ref.digest(value) for key, value in raw.items()}}
        flags = ("input_immutability_and_all_allocation_guards_passed", "projection_unchanged_after_consumer",
                 "producer_completion_before_consumer", "intermediate_allocation_reused_without_host_write",
                 "free_close_and_worker_exit_passed")
        report.update({key: True for key in flags})
        ref.check_report(report, frozen, artifact, record, raw)
        for key in (*flags, "output_sha256", "artifact", "completed_dispatches", "authority"):
            changed = dict(report)
            changed[key] = False if key in flags else "corrupt"
            with self.subTest(key=key), self.assertRaises(ValueError):
                ref.check_report(changed, frozen, artifact, record, raw)

    def test_duplicate_metadata_and_truncated_checkpoint_reject(self):
        with self.assertRaisesRegex(ValueError, "duplicate"):
            json.loads('{"shape":[],"shape":[128]}', object_pairs_hook=ref.extractor.unique_object)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkpoint = root / "model.safetensors"
            checkpoint.write_bytes(b"broken")
            with self.assertRaisesRegex(ValueError, "truncated"):
                ref.gamma_extract(checkpoint, root / "gamma")
            self.assertFalse((root / "gamma").exists())


if __name__ == "__main__":
    unittest.main()
