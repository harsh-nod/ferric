"""Synthetic identity joins only; no native artifact admission is fabricated."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

from artifact import handoff_binding
from common import digest
import draft
from test_draft import materialize


def encoded(value):
    return json.dumps(value, separators=(",", ":")).encode() + b"\n"


class HandoffIdentityTests(unittest.TestCase):
    def test_manifest_identity_mode_is_explicit_without_a_handoff_file(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            _, definition = materialize(base)
            (base / "handoff.fixture").unlink()
            definition["files"]["handoff"] = {"mode": "manifest-identity"}
            result, _, _ = draft.load_manifest(definition, base)
            binding = result["handoff_provenance"]
            self.assertEqual(binding["mode"], "manifest-identity")
            self.assertFalse(binding["raw_handoff_bytes_retained_and_revalidated"])
            self.assertEqual(binding["sha256"], result["provenance_sha256"]["handoff"])
            self.assertEqual(binding["observation_manifest_sha256"], definition["files"]["artifact_manifest"]["sha256"])

    def test_retained_bytes_join_manifest_length_and_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            _, definition = materialize(base)
            result, _, _ = draft.load_manifest(definition, base)
            self.assertTrue(result["handoff_provenance"]["raw_handoff_bytes_retained_and_revalidated"])
            raw = (base / "artifact_manifest.fixture").read_bytes()
            image = (base / "hsaco.fixture").read_bytes()
            with self.assertRaisesRegex(ValueError, "retained handoff"):
                handoff_binding(raw, image, b"wrong retained handoff")

    def test_profile_and_identity_mutations_reject_even_when_manifest_is_rehashed(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            materialize(base)
            raw = (base / "artifact_manifest.fixture").read_bytes()
            original = json.loads(raw)
            image = (base / "hsaco.fixture").read_bytes()
            mutations = [
                lambda value: value.update(schema="unknown"),
                lambda value: value.update(crate_name="target-not-draft"),
                lambda value: value.update(target="gfx942:xnack-"),
                lambda value: value.update(code_object_version=True),
                lambda value: value.update(authority="protected"),
                lambda value: value["grants"].update(launch=True),
                lambda value: value["grants"].update(load=0),
                lambda value: value["compiler_handoff"].update(byte_len=True),
                lambda value: value["compiler_handoff"].update(byte_len=0),
                lambda value: value["compiler_handoff"].update(byte_len=64 * 1024 * 1024 + 1),
                lambda value: value["compiler_handoff"].update(sha256="bad"),
                lambda value: value["compiler_handoff"].update(extra=True),
                lambda value: value["hsaco"]["identity"].update(byte_len=len(image) + 1),
                lambda value: value["hsaco"]["identity"].update(sha256="f" * 64),
                lambda value: value.update(extra=True),
            ]
            for mutate in mutations:
                value = copy.deepcopy(original)
                mutate(value)
                with self.subTest(value=value), self.assertRaises(ValueError):
                    handoff_binding(encoded(value), image)

    def test_noncanonical_encoding_and_field_order_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            materialize(base)
            raw = (base / "artifact_manifest.fixture").read_bytes()
            image = (base / "hsaco.fixture").read_bytes()
            value = json.loads(raw)
            for other in (raw[:-1], b" " + raw, json.dumps(value, indent=2).encode() + b"\n",
                          encoded(dict(reversed(list(value.items()))))):
                with self.subTest(other=other), self.assertRaises(ValueError):
                    handoff_binding(other, image)

    def test_mode_confusion_and_setup_substitution_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            _, definition = materialize(base)
            for replacement in ({"mode": "retained-bytes"}, {"mode": "manifest-identity", "sha256": "a" * 64},
                                {"mode": "manifest-identity", "path": "handoff.fixture"}):
                altered = copy.deepcopy(definition)
                altered["files"]["handoff"] = replacement
                with self.assertRaises(ValueError):
                    draft.load_manifest(altered, base)
            definition["files"]["handoff"] = {"mode": "manifest-identity"}
            raw = (base / "artifact_manifest.fixture").read_bytes()
            value = json.loads(raw)
            value["compiler_handoff"]["sha256"] = "b" * 64
            raw = encoded(value)
            (base / "artifact_manifest.fixture").write_bytes(raw)
            definition["files"]["artifact_manifest"]["sha256"] = digest(raw)
            # Keep the setup's actual old handoff, but update the manifest byte pin.
            capture = (base / "capture.fixture").read_bytes().splitlines()
            setup = json.loads(capture[0])
            setup["artifact_manifest_id"] = digest(raw)
            capture[0] = json.dumps(setup, separators=(",", ":")).encode()
            capture = b"\n".join(capture) + b"\n"
            (base / "capture.fixture").write_bytes(capture)
            definition["files"]["capture"]["sha256"] = digest(capture)
            with self.assertRaisesRegex(ValueError, "pinned binary/reference/artifact mismatch"):
                draft.load_manifest(definition, base)


if __name__ == "__main__":
    unittest.main()
