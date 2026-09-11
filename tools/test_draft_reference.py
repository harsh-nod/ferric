"""CPU-only policy fixtures, never canonical model outputs or a GPU emulator."""

import copy
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch

import draft_reference as reference


PRODUCER = "11" * 32
PAYLOAD = "22" * 32


def fixture():
    prompt = [1, 2, 3, 4, 5]
    choices = [11, 12, 13, 14, 6, 7]
    steps = []
    for position, choice in enumerate(choices):
        steps.append({"position": position, "input_token": prompt[position] if position < 5 else 6,
                      "choice": choice, "cache_tokens": position + 1,
                      "logits_dtype": "torch.bfloat16", "finite_count": reference.VOCABULARY,
                      "top2": {"ids": [choice, 99], "values": [2.0, 1.0], "gap": 1.0}})
    observed = {"steps": steps, "generated_tokens": [6, 7], "generated_utf8_bytes": list(b"fixture")}
    return {"schema": reference.RAW_SCHEMA, "authority": "independent-draft-reference-only",
            "performance_qualified": False, "model": reference.MODEL, "model_revision": reference.REVISION,
            "source": "/synthetic/cpu/fixture", "checkpoint": {name: {"bytes": size, "sha256": sha} for name, (size, sha) in reference.FILES.items()},
            "draft_payload_sha256": PAYLOAD, "producer_sha256": PRODUCER,
            "externally_pinned_image": reference.IMAGE, "externally_pinned_image_id": reference.IMAGE_ID,
            "versions": dict(reference.VERSIONS), "implementation_sources": dict(reference.SOURCES),
            "policy": copy.deepcopy(reference.POLICY), "prompt": reference.PROMPT, "prompt_tokens": prompt,
            "device": {"index": 0, "name": "fixture only", "gcn_arch": "gfx950", "torch_hip": "fixture"},
            "loading_info": {name: [] for name in ("missing_keys", "unexpected_keys", "mismatched_keys", "error_msgs")},
            "tied_weights": True, "passes": [copy.deepcopy(observed), copy.deepcopy(observed)], "repeated_exact": True}


def identity(raw):
    return {"schema": reference.IDENTITY_SCHEMA, "checkpoint": copy.deepcopy(raw["checkpoint"]),
            "identity": {"model_bundle_id": "33" * 32, "draft_model_id": reference.MODEL_ID,
                         "draft_config_id": "44" * 32, "draft_weights_sha256": PAYLOAD}}


class DraftReferenceTests(unittest.TestCase):
    def test_cpu_import_does_not_load_model_or_torch(self):
        self.assertNotIn("torch", sys.modules)
        self.assertNotIn("transformers", sys.modules)

    def test_closed_raw_and_separate_adapter(self):
        raw = fixture()
        reference.validate_raw(raw, PRODUCER)
        result = reference.adapt(raw, identity(raw), PRODUCER, "55" * 32)
        self.assertEqual(result["schema"], "FerricDraftCanaryReferenceV1")
        self.assertEqual(result["generated_tokens"], [6, 7])
        self.assertEqual(result["generated_utf8_bytes"], list(b"fixture"))
        self.assertEqual(set(result), {"schema", "model", "model_revision", "identity", "prompt_tokens", "generated_tokens", "generated_utf8_bytes", "producer"})
        self.assertIn(PRODUCER, result["producer"])
        self.assertNotIn("identity", raw)
        self.assertNotIn("throughput", result)

    def test_parser_rejects_duplicate_nonfinite_and_oversized_documents(self):
        for data in [b"", b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":Infinity}', b" " * (reference.MAX_DOCUMENT + 1)]:
            with self.subTest(data=data[:30]), self.assertRaises(ValueError):
                reference.parse(data)
        self.assertEqual(reference.parse(reference.encoded(fixture())), fixture())

    def test_external_hash_and_source_checked_before_input_or_gpu_import(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "raw.json"
            data = reference.encoded(fixture())
            path.write_bytes(data)
            self.assertEqual(reference.read_pinned(path, reference.digest(data)), fixture())
            with self.assertRaises(ValueError):
                reference.read_pinned(path, "00" * 32)
            with self.assertRaises(ValueError):
                reference.main(["--producer-sha256", "00" * 32, "produce", "--source", "/not-read", "--output", str(Path(directory) / "out.json"), "--image", reference.IMAGE, "--image-id", reference.IMAGE_ID])
        self.assertNotIn("torch", sys.modules)

    def test_raw_pins_authority_and_policy_are_exact(self):
        mutations = [
            ("schema", "FerricQwen3TpBatchObservationV1"), ("authority", "qualified"),
            ("performance_qualified", True), ("model", "Qwen/Qwen3-8B"),
            ("model_revision", "changed"), ("producer_sha256", "00" * 32),
            ("externally_pinned_image", "latest"), ("externally_pinned_image_id", "changed"),
            ("tied_weights", 1), ("repeated_exact", 1), ("extra", False),
        ]
        for key, value in mutations:
            raw = fixture()
            raw[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                reference.validate_raw(raw, PRODUCER)
        for group, key, value in [
            ("versions", "torch", "different"), ("implementation_sources", "models/qwen3/modeling_qwen3.py", "00" * 32),
            ("policy", "dtype", "float32"), ("policy", "eos_stopping", 0),
            ("policy", "prompt_tokens", 5.0), ("policy", "trust_remote_code", True),
            ("policy", "attention", "eager"), ("loading_info", "missing_keys", ["lm_head.weight"]),
            ("device", "index", False), ("device", "gcn_arch", "gfx942"),
        ]:
            raw = fixture()
            raw[group][key] = value
            with self.subTest(group=group, key=key), self.assertRaises(ValueError):
                reference.validate_raw(raw, PRODUCER)

    def test_integer_arrays_and_ordinary_output_canaries_are_strict(self):
        for token in [True, 6.0, -1, reference.VOCABULARY]:
            raw = fixture()
            raw["prompt_tokens"][0] = token
            with self.assertRaises(ValueError):
                reference.validate_raw(raw, PRODUCER)
        for field, value in [("generated_tokens", [7, 6]), ("generated_utf8_bytes", [256]), ("generated_utf8_bytes", [False])]:
            raw = fixture()
            raw["passes"][0][field] = value
            with self.assertRaises(ValueError):
                reference.validate_raw(raw, PRODUCER)

    def test_each_forward_schedule_cache_dtype_finite_extent_and_top_gap(self):
        for field, value in [("position", 1), ("position", 0.0), ("input_token", 2), ("choice", False),
                             ("cache_tokens", 0), ("cache_tokens", True), ("logits_dtype", "torch.float32"),
                             ("finite_count", reference.VOCABULARY - 1), ("finite_count", float(reference.VOCABULARY))]:
            raw = fixture()
            raw["passes"][0]["steps"][0][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                reference.validate_raw(raw, PRODUCER)
        for top in [{"ids": [11, 11], "values": [2, 1], "gap": 1},
                    {"ids": [11, 1], "values": [2, 2], "gap": 0},
                    {"ids": [11, 99], "values": [1, 2], "gap": -1},
                    {"ids": [11, 99], "values": [2, 1], "gap": 2},
                    {"ids": [11, 99], "values": [float("nan"), 1], "gap": 1}]:
            raw = fixture()
            raw["passes"][0]["steps"][0]["top2"] = top
            with self.assertRaises(ValueError):
                reference.validate_raw(raw, PRODUCER)
        raw = fixture()
        raw["passes"][0]["steps"][5]["input_token"] = 7
        with self.assertRaises(ValueError):
            reference.validate_raw(raw, PRODUCER)

    def test_repeated_all_six_observations_not_only_two_outputs(self):
        raw = fixture()
        raw["passes"][1]["steps"][0]["top2"]["values"] = [3, 2]
        with self.assertRaisesRegex(ValueError, "repeated complete"):
            reference.validate_raw(raw, PRODUCER)
        raw = fixture()
        raw["passes"] = raw["passes"][:1]
        with self.assertRaises(ValueError):
            reference.validate_raw(raw, PRODUCER)

    def test_full_vocabulary_argmax_ties_nonfinite_and_tail(self):
        values = [-1.0] * reference.VOCABULARY
        values[7] = values[19] = 2.0
        self.assertEqual(reference.top_two(values), {"ids": [7, 19], "values": [2.0, 2.0], "gap": 0.0})
        values[-1] = 2.5
        self.assertEqual(reference.top_two(values), {"ids": [reference.VOCABULARY - 1, 7], "values": [2.5, 2.0], "gap": 0.5})
        for bad in [float("nan"), float("inf"), float("-inf"), True]:
            values[10] = bad
            with self.assertRaises(ValueError):
                reference.top_two(values)
        with self.assertRaises(ValueError):
            reference.top_two(values[:-1])

    def test_adapter_requires_separate_admitted_draft_identity(self):
        raw = fixture()
        for key in ("draft_model_id", "draft_weights_sha256", "draft_config_id", "model_bundle_id"):
            bound = identity(raw)
            bound["identity"][key] = "bad" if key in ("draft_config_id", "model_bundle_id") else "00" * 32
            with self.subTest(key=key), self.assertRaises(ValueError):
                reference.adapt(raw, bound, PRODUCER, "55" * 32)
        bound = identity(raw)
        bound["checkpoint"]["config.json"]["sha256"] = "00" * 32
        with self.assertRaises(ValueError):
            reference.adapt(raw, bound, PRODUCER, "55" * 32)
        bound = identity(raw)
        bound["identity"]["generated_tokens"] = [6, 7]
        with self.assertRaises(ValueError):
            reference.adapt(raw, bound, PRODUCER, "55" * 32)

    def test_checkpoint_complete_hash_payload_and_changes_without_large_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = struct.pack("<Q", 2) + b"{}" + b"abc"
            (root / "model.safetensors").write_bytes(data)
            pins = {"model.safetensors": (len(data), reference.digest(data))}
            with patch.object(reference, "FILES", pins), patch.object(reference, "PAYLOAD_BYTES", 3):
                _, payload = reference.checkpoint(root)
                self.assertEqual(payload, reference.digest(b"abc"))
                (root / "model.safetensors").write_bytes(data[:-1] + b"d")
                with self.assertRaisesRegex(ValueError, "checkpoint mismatch"):
                    reference.checkpoint(root)
                (root / "model.safetensors").write_bytes(data + b"x")
                with self.assertRaisesRegex(ValueError, "byte length"):
                    reference.checkpoint(root)
            with patch.object(reference, "FILES", pins), patch.object(reference, "PAYLOAD_BYTES", 4):
                (root / "model.safetensors").write_bytes(data)
                with self.assertRaisesRegex(ValueError, "boundary"):
                    reference.checkpoint(root)

    def test_cached_transformers_loading_info_exact_empty_sets(self):
        value = {key: set() for key in ("missing_keys", "unexpected_keys", "mismatched_keys")}
        value["error_msgs"] = []
        self.assertEqual(reference.normalize_loading_info(value), {key: [] for key in value})
        for key in value:
            for wrong in (["failure"], ["lm_head.weight"], {}, None, {"failure"}):
                bad = copy.deepcopy(value)
                bad[key] = wrong
                with self.assertRaises(ValueError):
                    reference.normalize_loading_info(bad)
        bad = copy.deepcopy(value)
        bad["missing_keys"] = []
        with self.assertRaises(ValueError):
            reference.normalize_loading_info(bad)
        bad = copy.deepcopy(value)
        bad["conversion_errors"] = {}
        with self.assertRaises(ValueError):
            reference.normalize_loading_info(bad)

    def test_fresh_private_publication_and_no_overwrite_or_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "result.json"
            reference.publish(target, {"fixture": True})
            self.assertEqual(target.stat().st_mode & 0o777, 0o600)
            self.assertEqual(json.loads(target.read_text()), {"fixture": True})
            with self.assertRaises(ValueError):
                reference.publish(target, {})
            link = root / "link.json"
            link.symlink_to(root / "missing.json")
            with self.assertRaises(ValueError):
                reference.publish(link, {})
            os.chmod(root, 0o755)
            with self.assertRaises(ValueError):
                reference.publish(root / "new.json", {})
            os.chmod(root, 0o700)

    def test_adapt_cli_only_reads_pinned_documents_and_never_model(self):
        producer_sha = reference.file_digest(Path(reference.__file__))
        raw = fixture()
        raw["producer_sha256"] = producer_sha
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            raw_bytes = reference.encoded(raw)
            identity_bytes = reference.encoded(identity(raw))
            (root / "raw.json").write_bytes(raw_bytes)
            (root / "identity.json").write_bytes(identity_bytes)
            argv = ["--producer-sha256", producer_sha, "adapt", "--raw", str(root / "raw.json"), "--raw-sha256", reference.digest(raw_bytes), "--identity", str(root / "identity.json"), "--identity-sha256", reference.digest(identity_bytes), "--output", str(root / "adapter.json")]
            self.assertEqual(reference.main(argv), 0)
            result = json.loads((root / "adapter.json").read_text())
            self.assertEqual(result["generated_tokens"], [6, 7])
            self.assertIn(reference.digest(raw_bytes), result["producer"])
        self.assertNotIn("torch", sys.modules)


if __name__ == "__main__":
    unittest.main()
