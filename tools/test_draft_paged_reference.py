"""CPU-only policy/custody tests; no PyTorch import or GPU execution."""

import ast
import copy
import importlib.util
import os
from pathlib import Path
import struct
import tempfile
from types import SimpleNamespace
import unittest


ROOT = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("paged_reference", ROOT / "draft_paged_reference.py")
tool = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(tool)
HELPER_PATH = Path(os.environ.get("FERRIC_DRAFT_REFERENCE_HELPER", ROOT / "draft_reference.py"))
helper = tool.load_helper(HELPER_PATH, tool.HELPER_SHA)
PRODUCER = "a" * 64


def fixture():
    values = [0.0] * tool.VOCABULARY
    values[10], values[11] = 2.0, 1.0
    data = struct.pack(f"<{tool.VOCABULARY}f", *values)
    prompt = [1, 2, 3, 4, 5]
    payloads, schedules = {}, {}
    for mode, schedule in tool.POLICY["schedules"].items():
        passes = []
        for repetition in range(2):
            steps, previous = [], None
            for ordinal in range(len(schedule)):
                expected = tool.plan(helper, mode, prompt, ordinal, previous)
                name = tool.payload_name(mode, repetition, ordinal)
                payloads[name] = data
                steps.append({**expected, "choice": 10, "cache_tokens": expected["positions"][-1] + 1,
                              "logits_dtype": "torch.float32", "finite_count": tool.VOCABULARY,
                              "top2": {"ids": [10, 11], "values": [2.0, 1.0], "gap": 1.0},
                              "logits": {"file": name, "bytes": len(data), "sha256": helper.digest(data)}})
                previous = 10
            passes.append({"steps": steps, "generated_tokens": [10, 10], "generated_utf8_bytes": [120]})
        schedules[mode] = passes
    raw = {"schema": tool.RAW_SCHEMA, "authority": "independent-draft-reference-only", "performance_qualified": False,
           "model": helper.MODEL, "model_revision": helper.REVISION, "source": "/canonical/model",
           "checkpoint": {name: {"bytes": size, "sha256": sha} for name, (size, sha) in helper.FILES.items()},
           "draft_payload_sha256": "b" * 64, "producer_sha256": PRODUCER, "legacy_helper_sha256": tool.HELPER_SHA,
           "externally_pinned_image": helper.IMAGE, "externally_pinned_image_id": helper.IMAGE_ID,
           "versions": helper.VERSIONS.copy(), "implementation_sources": helper.SOURCES.copy(), "policy": copy.deepcopy(tool.POLICY),
           "prompt": helper.PROMPT, "prompt_tokens": prompt,
           "device": {"index": 0, "name": "test-only", "gcn_arch": "gfx950", "torch_hip": "test-only"},
           "loading_info": {name: [] for name in ("missing_keys", "unexpected_keys", "mismatched_keys", "error_msgs")},
           "tied_weights": True, "schedules": schedules, "repeated_exact": True, "logit_payload_bytes": 16 * len(data)}
    return raw, payloads


class PagedReferenceTests(unittest.TestCase):
    def test_full_logit_custody_and_both_schedules_pass(self):
        raw, payloads = fixture()
        tool.validate_raw(helper, raw, PRODUCER, payloads.__getitem__)
        self.assertEqual(len(payloads), 16)
        self.assertEqual(sum(map(len, payloads.values())), 9_723_904)
        self.assertNotIn("torch", tool.__dict__)

    def test_schedule_requires_previous_choice_and_exact_ordinal(self):
        prompt = [1, 2, 3, 4, 5]
        self.assertEqual(tool.plan(helper, "full", prompt, 0, None), {"inputs": prompt, "positions": [0, 1, 2, 3, 4], "selected_row": 4})
        for mode, ordinal, previous in [("full", 1, None), ("tokenwise", 5, None), ("full", 2, 10), ("tokenwise", True, 10), ("unknown", 0, None), ("full", 1, -1)]:
            with self.subTest(mode=mode, ordinal=ordinal), self.assertRaises(ValueError):
                tool.plan(helper, mode, prompt, ordinal, previous)

    def test_frozen_source_and_external_helper_pin_are_required(self):
        with self.assertRaises(ValueError):
            tool.load_helper(HELPER_PATH, "0" * 64)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "helper.py"
            path.write_bytes(HELPER_PATH.read_bytes() + b"\n")
            with self.assertRaises(ValueError):
                tool.load_helper(path, tool.HELPER_SHA)

    def test_cli_self_source_drift_rejects_before_raw_or_model_access(self):
        with self.assertRaisesRegex(ValueError, "producer source drift"):
            tool.main(["--producer-sha256", "0" * 64, "--legacy-helper", str(HELPER_PATH),
                       "--legacy-helper-sha256", tool.HELPER_SHA, "produce", "--source", "/absent",
                       "--output", "/absent", "--image", helper.IMAGE, "--image-id", helper.IMAGE_ID])

    def test_empty_display_name_and_string_subclasses_preserve_strict_architecture(self):
        class RuntimeString(str):
            pass

        properties = SimpleNamespace(name=RuntimeString(""), gcnArchName=RuntimeString("gfx950:sramecc+:xnack-"))
        device = helper.device_metadata(properties, RuntimeString("7.2.fixture"))
        self.assertEqual(device["name"], "")
        self.assertTrue(all(type(device[key]) is str for key in ("name", "gcn_arch", "torch_hip")))
        raw, payloads = fixture()
        raw["device"] = device
        tool.validate_raw(helper, raw, PRODUCER, payloads.__getitem__)
        for key, value in [("gcn_arch", ""), ("gcn_arch", "gfx942"), ("torch_hip", ""), ("name", None)]:
            changed = copy.deepcopy(raw)
            changed["device"][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                tool.validate_raw(helper, changed, PRODUCER, payloads.__getitem__)

    def test_fp32_controls_are_explicit_and_bound_in_policy(self):
        tree = ast.parse((ROOT / "draft_paged_reference.py").read_text())
        calls = [node for node in ast.walk(tree) if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)]
        precision = [node for node in calls if node.func.attr == "set_float32_matmul_precision"]
        self.assertEqual(len(precision), 1)
        self.assertEqual(ast.literal_eval(precision[0].args[0]), "highest")
        autocast = [node for node in calls if node.func.attr == "autocast"]
        self.assertEqual(len(autocast), 1)
        self.assertEqual({item.arg: ast.literal_eval(item.value) for item in autocast[0].keywords}, {"device_type": "cuda", "enabled": False})
        self.assertEqual(tool.POLICY["float32_matmul_precision"], "highest")
        self.assertIs(tool.POLICY["autocast_enabled"], False)
        raw, payloads = fixture()
        for key, value in [("float32_matmul_precision", "high"), ("autocast_enabled", True), ("tf32", True)]:
            changed = copy.deepcopy(raw)
            changed["policy"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                tool.validate_raw(helper, changed, PRODUCER, payloads.__getitem__)

    def test_raw_profile_identity_and_policy_mutations_reject(self):
        mutations = [
            lambda raw: raw.update(schema=helper.RAW_SCHEMA),
            lambda raw: raw.update(performance_qualified=True),
            lambda raw: raw.update(authority="protected"),
            lambda raw: raw.update(extra=True),
            lambda raw: raw.update(producer_sha256="c" * 64),
            lambda raw: raw.update(legacy_helper_sha256="c" * 64),
            lambda raw: raw.update(model_revision="changed"),
            lambda raw: raw.update(logit_payload_bytes=0),
            lambda raw: raw["policy"].update(head_dtype="bfloat16"),
            lambda raw: raw["policy"]["schedules"].update(full=[[0], [1]]),
            lambda raw: raw["device"].update(index=True),
            lambda raw: raw["device"].update(gcn_arch="gfx942"),
            lambda raw: raw["checkpoint"]["config.json"].update(bytes=1),
            lambda raw: raw["loading_info"].update(missing_keys=["x"]),
            lambda raw: raw.update(tied_weights=False),
            lambda raw: raw.update(repeated_exact=False),
        ]
        raw, payloads = fixture()
        for index, mutation in enumerate(mutations):
            changed = copy.deepcopy(raw)
            mutation(changed)
            with self.subTest(index=index), self.assertRaises(ValueError):
                tool.validate_raw(helper, changed, PRODUCER, payloads.__getitem__)

    def test_step_geometry_causality_precision_and_descriptor_mutations_reject(self):
        mutations = [lambda step: step.update(inputs=[5] * 5), lambda step: step.update(positions=[0]),
                     lambda step: step.update(selected_row=0), lambda step: step.update(selected_row=True),
                     lambda step: step.update(choice=11), lambda step: step.update(cache_tokens=4),
                     lambda step: step.update(finite_count=1), lambda step: step.update(logits_dtype="torch.bfloat16"),
                     lambda step: step["logits"].update(file="../escape"), lambda step: step["logits"].update(bytes=True),
                     lambda step: step["logits"].update(sha256="c" * 64), lambda step: step.update(extra=True),
                     lambda step: step["top2"].update(gap=2.0)]
        raw, payloads = fixture()
        for index, mutation in enumerate(mutations):
            changed = copy.deepcopy(raw)
            mutation(changed["schedules"]["full"][0]["steps"][0])
            with self.subTest(index=index), self.assertRaises(ValueError):
                tool.validate_raw(helper, changed, PRODUCER, payloads.__getitem__)

    def test_nan_full_vector_and_rehashed_wrong_argmax_reject(self):
        raw, payloads = fixture()
        for value in (float("nan"), float("inf"), 4.0):
            changed = copy.deepcopy(raw)
            step = changed["schedules"]["full"][0]["steps"][0]
            name = step["logits"]["file"]
            data = bytearray(payloads[name])
            struct.pack_into("<f", data, 100 * 4, value)
            step["logits"]["sha256"] = helper.digest(data)
            with self.subTest(value=value), self.assertRaises(ValueError):
                tool.validate_raw(helper, changed, PRODUCER, lambda key: bytes(data) if key == name else payloads[key])

    def test_nonmax_logit_change_breaks_repetition_even_with_correct_top2(self):
        raw, payloads = fixture()
        step = raw["schedules"]["full"][1]["steps"][0]
        name = step["logits"]["file"]
        data = bytearray(payloads[name])
        struct.pack_into("<f", data, 100 * 4, 0.5)
        payloads[name] = bytes(data)
        step["logits"]["sha256"] = helper.digest(data)
        with self.assertRaisesRegex(ValueError, "repeated complete"):
            tool.validate_raw(helper, raw, PRODUCER, payloads.__getitem__)

    def test_missing_duplicate_extra_steps_and_decode_input_reject(self):
        raw, payloads = fixture()
        mutations = [lambda raw: raw["schedules"].pop("full"),
                     lambda raw: raw["schedules"]["full"].pop(),
                     lambda raw: raw["schedules"]["full"][0]["steps"].pop(),
                     lambda raw: raw["schedules"]["full"][0]["steps"].append(raw["schedules"]["full"][0]["steps"][0]),
                     lambda raw: raw["schedules"]["tokenwise"][0]["steps"][5].update(inputs=[11]),
                     lambda raw: raw["schedules"]["full"][0].update(generated_tokens=[11, 10]),
                     lambda raw: raw["schedules"]["full"][0].update(generated_utf8_bytes=[255])]
        for index, mutation in enumerate(mutations):
            changed = copy.deepcopy(raw)
            mutation(changed)
            with self.subTest(index=index), self.assertRaises((ValueError, UnicodeError)):
                tool.validate_raw(helper, changed, PRODUCER, payloads.__getitem__)

    def test_lowest_id_tie_is_recomputed_from_full_vector(self):
        values = [0.0] * tool.VOCABULARY
        values[10] = values[11] = 2.0
        self.assertEqual(helper.top_two(values), {"ids": [10, 11], "values": [2.0, 2.0], "gap": 0.0})

    def test_directory_roster_pin_symlink_and_hardlink_reject(self):
        raw, payloads = fixture()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            data = helper.encoded(raw)
            (path / "raw.json").write_bytes(data)
            for name, value in payloads.items():
                (path / name).write_bytes(value)
            self.assertEqual(tool.load_raw(helper, path, helper.digest(data), PRODUCER), raw)
            with self.assertRaises(ValueError):
                tool.load_raw(helper, path, "c" * 64, PRODUCER)
            extra = path / "extra"
            extra.write_bytes(b"x")
            with self.assertRaises(ValueError):
                tool.load_raw(helper, path, helper.digest(data), PRODUCER)
            extra.unlink()
            name = next(iter(payloads))
            original = path / name
            original.unlink()
            with self.assertRaises(ValueError):
                tool.load_raw(helper, path, helper.digest(data), PRODUCER)
            other = path / next(key for key in payloads if key != name)
            original.symlink_to(other)
            with self.assertRaises(ValueError):
                tool.load_raw(helper, path, helper.digest(data), PRODUCER)
            original.unlink()
            os.link(other, original)
            with self.assertRaises(ValueError):
                tool.load_raw(helper, path, helper.digest(data), PRODUCER)

    def test_adapters_bind_identity_and_remain_distinct_without_performance_fields(self):
        raw, payloads = fixture()
        tool.validate_raw(helper, raw, PRODUCER, payloads.__getitem__)
        identity = {"schema": helper.IDENTITY_SCHEMA, "checkpoint": raw["checkpoint"],
                    "identity": {"model_bundle_id": "c" * 64, "draft_model_id": helper.MODEL_ID,
                                 "draft_config_id": "d" * 64, "draft_weights_sha256": raw["draft_payload_sha256"]}}
        for mode, count in [("full", 2), ("tokenwise", 6)]:
            result = tool.adapt(helper, raw, identity, PRODUCER, "e" * 64, mode)
            self.assertEqual(result["schema"], tool.REFERENCE_SCHEMA)
            self.assertEqual(result["head_precision"], "fp32-v10")
            self.assertEqual(result["prefill"], mode)
            self.assertEqual(len(result["steps"]), count)
            self.assertNotIn("metrics", result)
            self.assertNotEqual(result["schema"], helper.ADAPTER_SCHEMA)
        for key in identity["identity"]:
            changed = copy.deepcopy(identity)
            changed["identity"][key] = "invalid"
            with self.subTest(key=key), self.assertRaises(ValueError):
                tool.adapt(helper, raw, changed, PRODUCER, "e" * 64, "full")
        changed = copy.deepcopy(identity)
        changed["identity"]["draft_weights_sha256"] = "f" * 64
        with self.assertRaises(ValueError):
            tool.adapt(helper, raw, changed, PRODUCER, "e" * 64, "full")

    def test_payload_writer_is_exclusive_and_full_extent(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "logits.f32le"
            descriptor = tool.write_payload(helper, path, [0.0] * tool.VOCABULARY)
            self.assertEqual(descriptor["bytes"], tool.LOGIT_BYTES)
            self.assertEqual(descriptor["sha256"], helper.digest(path.read_bytes()))
            with self.assertRaises(ValueError):
                tool.write_payload(helper, path, [0.0] * tool.VOCABULARY)


if __name__ == "__main__":
    unittest.main()
