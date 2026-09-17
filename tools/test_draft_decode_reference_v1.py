"""CPU-only contract tests; synthetic logits are never performance evidence."""

import copy
import importlib.util
from pathlib import Path
import struct
import unittest
from unittest.mock import patch


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ROOT = Path(__file__).resolve().parent
producer = load("decode_cpu_reference", ROOT / "draft_decode_reference_v1.py")
helper = producer.load_helper(ROOT / "draft_reference.py")


class DecodeReferenceTests(unittest.TestCase):
    def test_decode_supports_longer_generation(self):
        class Tokenizer:
            def decode(self, tokens, skip_special_tokens):
                self.tokens = tokens
                self.skip = skip_special_tokens
                return "reference text"

        tokenizer = Tokenizer()
        result = producer.decode_generated(helper, tokenizer, [1, 2, 3, 4], 4)
        self.assertEqual(bytes(result), b"reference text")
        self.assertEqual(tokenizer.tokens, [1, 2, 3, 4])
        self.assertIs(tokenizer.skip, True)
        with self.assertRaises(ValueError):
            producer.decode_generated(helper, tokenizer, [1, 2], 4)

    def test_every_admitted_schedule(self):
        prompt = [1, 2, 3, 4, 5]
        for mode in ("full", "tokenwise"):
            for count in range(2, 129):
                positions = []
                for ordinal in range(count + (4 if mode == "tokenwise" else 0)):
                    step = producer.schedule(mode, count, prompt, ordinal, 7)
                    positions.extend(step["positions"])
                    self.assertEqual(step["selected_row"], len(step["inputs"]) - 1)
                self.assertEqual(positions, list(range(count + 4)))

    def test_invalid_schedule_rejected(self):
        for mode, count, ordinal, previous in [("unknown", 2, 0, None), ("full", 1, 0, None), ("full", 129, 0, None), ("full", True, 0, None), ("full", 2, 2, 0), ("full", 2, 1, None), ("full", 2, 1, True)]:
            with self.subTest((mode, count, ordinal, previous)), self.assertRaises(ValueError):
                producer.schedule(mode, count, [1, 2, 3, 4, 5], ordinal, previous)

    def fixture(self):
        data = struct.pack("<8f", 0, 1, 0, 0, 0, 0, 0, 0)
        payloads, passes = {}, []
        for repetition in range(2):
            steps = []
            for ordinal in range(2):
                name = producer.payload_name(repetition, ordinal)
                payloads[name] = data
                expected = producer.schedule("full", 2, [1, 2, 3, 4, 5], ordinal, 1)
                steps.append({**expected, "choice": 1, "cache_tokens": ordinal + 5, "logits_sha256": helper.digest(data), "logits_file": name, "top2": helper.top_two(struct.unpack("<8f", data))})
            passes.append({"steps": steps, "generated_tokens": [1, 1], "generated_utf8_bytes": [65]})
        raw = {"schema": producer.RAW_SCHEMA, "performance_qualified": False, "model": helper.MODEL, "model_revision": helper.REVISION, "checkpoint": {name: {"bytes": size, "sha256": sha} for name, (size, sha) in helper.FILES.items()}, "draft_payload_sha256": "a" * 64, "producer_sha256": "b" * 64, "legacy_helper_sha256": producer.HELPER_SHA, "image_id": producer.IMAGE_ID, "versions": producer.VERSIONS, "implementation_sources": producer.SOURCES, "policy": producer.POLICY, "prompt_tokens": [1, 2, 3, 4, 5], "prefill": "full", "new_tokens": 2, "passes": passes}
        return raw, payloads

    def test_complete_logits_and_repeated_choices(self):
        with patch.object(helper, "VOCABULARY", 8):
            raw, payloads = self.fixture()
            producer.validate_raw(helper, raw, "b" * 64, payloads.__getitem__)
            mutations = [
                lambda r: r.update(performance_qualified=True),
                lambda r: r.update(image_id="unbound"),
                lambda r: r.update(model_revision="unbound"),
                lambda r: r["passes"][0]["steps"][1].update(inputs=[2]),
                lambda r: r["passes"][0]["steps"][0].update(choice=2),
                lambda r: r["passes"][0]["steps"][0].update(cache_tokens=4),
                lambda r: r["passes"][0]["steps"][0].update(logits_sha256="c" * 64),
                lambda r: r["passes"][0]["steps"][0].update(logits_file="../escape"),
                lambda r: r["passes"][1].update(generated_utf8_bytes=[66]),
                lambda r: r["passes"][0].update(generated_tokens=[2, 2]),
                lambda r: r.update(extra="unrecognized"),
            ]
            for mutate in mutations:
                altered = copy.deepcopy(raw)
                mutate(altered)
                with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                    producer.validate_raw(helper, altered, "b" * 64, payloads.__getitem__)

    def test_payload_corruption_rejected(self):
        with patch.object(helper, "VOCABULARY", 8):
            raw, payloads = self.fixture()
            payloads[producer.payload_name(0, 0)] = b"truncated"
            with self.assertRaises(ValueError):
                producer.validate_raw(helper, raw, "b" * 64, payloads.__getitem__)


if __name__ == "__main__":
    unittest.main()
