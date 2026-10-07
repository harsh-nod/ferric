#!/usr/bin/env python3
"""CPU-only retained-reference admission and publication regressions."""

from contextlib import contextmanager
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch


def load(name, filename):
    specification = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


subject = load("retained_reference_test_subject", "engineering_tp1_retained_final_stage_reference.py")
fixtures = load("retained_reference_native_fixtures", "test_engineering_tp1_final_stage_reference.py")
reference = subject.reference
encoded = fixtures.encoded
TEST_RECORDS = {"argv.txt": b"original isolated reference command\n",
                "started.txt": b"2026-09-18T01:31:35Z\n", "finished.txt": b"2026-09-18T01:32:14Z\n"}


def fixture(mode=None):
    mode = reference.ARITHMETIC_MODES[0] if mode is None else mode
    native = fixtures.fixture(arithmetic=mode)
    old = fixtures.fixture()
    history = {**TEST_RECORDS, "native-pins.json": encoded(old.pins)}
    history.update({f"pass{ordinal}-{name}": data for ordinal in (1, 2)
                    for name, data in old.data.items() if name.endswith(".bf16")})
    comparison = {
        "schema": "FerricTpFinalStageReferenceComparisonV1", "authority": "none",
        "qualification": False, "benchmark_comparable": False, "numerical_pass_claimed": False,
        "tolerance_reviewed": False, "cause_established": False, "process_absence_independently_proven": False,
        "reference_execution": "two-independent-full-sequence-executions-use-cache-false",
        "reference_byte_identical": True,
        "reference_model": {"repository": subject.core.PINNED_REPOSITORY, "revision": subject.core.PINNED_REVISION},
        "native_target": "gfx950:xnack-", "reference_target": subject.core.TARGET,
        "input_tokens": old.manifest["input_tokens"], "positions": old.manifest["positions"],
        "rows": [], "nonclaim": reference.NONCLAIM,
        "implementation_sha256": {name: "1" * 64 for name in subject.IMPLEMENTATION_FILES},
        "native_pins_sha256": subject.digest(history["native-pins.json"]),
        "native_files_sha256": {name: subject.digest(raw) for name, raw in old.data.items()},
        "native_witnesses_sha256": {name: subject.digest(raw) for name, raw in old.witness.items()},
        "reference_files_sha256": {name: subject.digest(history[name]) for name in subject.PASS_FILES},
    }
    history["comparison.json"] = encoded(comparison)
    return native, history


@contextmanager
def admitted_fixture(history):
    # Synthetic fixtures have their own trusted root; production constants are not changed.
    sha = subject.digest(history["comparison.json"])
    with patch.object(subject, "HISTORICAL_COMPARISON_SHA256", sha), \
            patch.object(subject, "HISTORICAL_RECORD_HASHES", {key: subject.digest(raw) for key, raw in TEST_RECORDS.items()}):
        yield sha


def change_comparison(history, mutate):
    value = json.loads(history["comparison.json"])
    mutate(value)
    history["comparison.json"] = encoded(value)


class AdmissionTests(unittest.TestCase):
    def test_both_modes_reuse_only_identical_workload_and_original_producer(self):
        for mode in reference.ARITHMETIC_MODES:
            native, history = fixture(mode)
            with admitted_fixture(history) as sha:
                original, pins, passes = subject.validate_history(history, sha, native.manifest)
            self.assertEqual(original["implementation_sha256"], {name: "1" * 64 for name in subject.IMPLEMENTATION_FILES})
            self.assertEqual(pins["model_bundle_id"], subject.core.PINNED_MODEL_IDENTITY)
            self.assertEqual(passes[0], passes[1])
            self.assertEqual(set(passes[0]), {f"{name}.bf16" for name in reference.WIDTHS})

    def test_external_comparison_hash_and_unadmitted_history_are_rejected(self):
        native, history = fixture()
        with self.assertRaises(subject.Failure):
            subject.validate_history(history, subject.digest(history["comparison.json"]), native.manifest)
        with admitted_fixture(history) as sha:
            with self.assertRaises(subject.Failure):
                subject.validate_history(history, "2" * 64, native.manifest)
            history["comparison.json"] += b" "
            with self.assertRaises(subject.Failure):
                subject.validate_history(history, sha, native.manifest)

    def test_model_revision_target_execution_and_nonclaims_cannot_be_relabelled(self):
        mutations = [lambda doc: doc["reference_model"].update(repository="other"),
                     lambda doc: doc["reference_model"].update(revision="2" * 40),
                     lambda doc: doc.update(reference_target="gfx950:xnack-"),
                     lambda doc: doc.update(reference_execution="fresh"),
                     lambda doc: doc.update(qualification=True),
                     lambda doc: doc.update(numerical_pass_claimed=True),
                     lambda doc: doc.update(schema="FerricTpArithmeticFinalStageReferenceComparisonV1"),
                     lambda doc: doc["implementation_sha256"].pop("run.py"),
                     lambda doc: doc["implementation_sha256"].update({"run.py": "invalid"})]
        for mutation in mutations:
            native, history = fixture()
            change_comparison(history, mutation)
            with admitted_fixture(history) as sha, self.assertRaises(subject.Failure):
                subject.validate_history(history, sha, native.manifest)

    def test_model_content_identity_is_bound_through_historical_pins(self):
        for which in ("native", "historical"):
            native, history = fixture()
            if which == "native":
                native.manifest["setup"]["model_bundle_id"] = "2" * 64
            else:
                pins = json.loads(history["native-pins.json"])
                pins["model_bundle_id"] = "2" * 64
                history["native-pins.json"] = encoded(pins)
                change_comparison(history, lambda doc: doc.update(native_pins_sha256=subject.digest(history["native-pins.json"])))
            with admitted_fixture(history) as sha, self.assertRaises(subject.Failure):
                subject.validate_history(history, sha, native.manifest)

    def test_full_consumed_sequence_positions_and_workload_must_match(self):
        mutations = [lambda doc: doc["input_tokens"].__setitem__(5, 1),
                     lambda doc: doc["input_tokens"].__setitem__(0, 1),
                     lambda doc: doc.update(positions=[3, 5]),
                     lambda doc: doc["setup"].update(prompt="different text"),
                     lambda doc: doc["setup"].update(new_tokens=3)]
        for mutation in mutations:
            native, history = fixture()
            mutation(native.manifest)
            with admitted_fixture(history) as sha, self.assertRaises(subject.Failure):
                subject.validate_history(history, sha, native.manifest)

    def test_all_six_payload_hashes_extents_and_closed_roster_are_required(self):
        for filename in sorted(subject.PASS_FILES):
            native, history = fixture()
            with admitted_fixture(history) as sha:
                history[filename] = b"\x80\x3f" + history[filename][2:]
                with self.subTest(filename=filename), self.assertRaises(subject.Failure):
                    subject.validate_history(history, sha, native.manifest)
        for kind in ("missing", "extra", "short", "hash_roster", "repeat"):
            native, history = fixture()
            filename = "pass1-residual.bf16"
            if kind == "missing":
                history.pop(filename)
            elif kind == "extra":
                history["unbound.bin"] = b"x"
            elif kind == "short":
                history[filename] = history[filename][2:]
                change_comparison(history, lambda doc: doc["reference_files_sha256"].update({filename: subject.digest(history[filename])}))
            elif kind == "hash_roster":
                change_comparison(history, lambda doc: doc["reference_files_sha256"].pop(filename))
            else:
                change_comparison(history, lambda doc: doc.update(reference_byte_identical=False))
            with admitted_fixture(history) as sha, self.subTest(kind=kind), self.assertRaises(subject.Failure):
                subject.validate_history(history, sha, native.manifest)

    def test_original_command_timestamps_and_pins_bytes_are_authenticated(self):
        for name in (*TEST_RECORDS, "native-pins.json"):
            native, history = fixture()
            with admitted_fixture(history) as sha:
                history[name] += b" "
                with self.subTest(name=name), self.assertRaises(subject.Failure):
                    subject.validate_history(history, sha, native.manifest)


class RunTests(unittest.TestCase):
    def run_fixture(self, root, mode, *, existing=False, alias=False, mutate=False, drift=False, nonfinite=False):
        native, history = fixture(mode)
        if nonfinite:
            filename = "pass2-logits.bf16"
            history[filename] = b"\xc1\x7f" + history[filename][2:]
            change_comparison(history, lambda doc: doc.update(reference_byte_identical=False))
            change_comparison(history, lambda doc: doc["reference_files_sha256"].update({filename: subject.digest(history[filename])}))
        base = Path(root)
        for directory, files in (("capture", native.data), ("witness", native.witness), ("history", history),
                                  ("pins", {"native.json": encoded(native.pins)})):
            (base / directory).mkdir()
            for name, data in files.items():
                (base / directory / name).write_bytes(data)
        output = base / "history" / "output" if alias else base / "output"
        if existing:
            output.mkdir()
            (output / "sentinel").write_bytes(b"preserve")
        real_compare = reference.compare
        def compare(*args, **kwargs):
            result = real_compare(*args, **kwargs)
            if mutate:
                (base / "history" / "pass1-residual.bf16").write_bytes(b"changed")
            return result
        real_implementation = subject.implementation
        calls = []
        def implementation(arithmetic):
            result = real_implementation(arithmetic)
            calls.append(None)
            if drift and len(calls) > 1:
                result["engineering_tp1_retained_final_stage_reference.py"] = "2" * 64
            return result
        with admitted_fixture(history) as sha, patch.object(subject.core, "require_isolated_python"), \
                patch.object(subject.core, "load_dependencies", side_effect=AssertionError("must remain CPU-only")), \
                patch.object(subject.core, "load_model", side_effect=AssertionError("must not load model")), \
                patch.object(reference, "execute", side_effect=AssertionError("must not execute reference")), \
                patch.object(reference, "compare", side_effect=compare), \
                patch.object(subject, "implementation", side_effect=implementation):
            arguments = [mode, str(base / "capture"), str(base / "witness"), str(base / "pins/native.json"),
                         subject.digest(encoded(native.pins)), str(base / "history"), sha, str(output)]
            if existing or alias or mutate or drift:
                with self.assertRaises(subject.Failure):
                    subject.run(arguments)
                if existing:
                    self.assertEqual((output / "sentinel").read_bytes(), b"preserve")
                else:
                    self.assertFalse(output.exists())
                return
            subject.run(arguments)
        result = json.loads((output / "comparison.json").read_bytes())
        self.assertEqual(result["residual_arithmetic"], mode)
        self.assertEqual(result["schema"], "FerricTpRetainedArithmeticFinalStageReferenceComparisonV1")
        self.assertIs(result["fresh_reference_execution"], False)
        for name in ("qualification", "benchmark_comparable", "numerical_pass_claimed", "cause_established"):
            self.assertIs(result[name], False)
        self.assertEqual(result["retained_reference"]["producer_records"], {key: raw.decode() for key, raw in TEST_RECORDS.items()})
        self.assertEqual(result["retained_reference"]["implementation_sha256"], {name: "1" * 64 for name in subject.IMPLEMENTATION_FILES})
        self.assertIn("engineering_tp1_retained_final_stage_reference.py", result["comparator_implementation_sha256"])
        for name, raw in history.items():
            retained_name = name if name in subject.PASS_FILES else f"retained-{name}"
            self.assertEqual((output / retained_name).read_bytes(), raw)
        if nonfinite:
            self.assertIs(result["reference_byte_identical"], False)
            self.assertIsNone(result["rows"][0]["stages"]["logits"]["native_vs_pass2"]["rmse"])

    def test_both_modes_keep_original_raw_passes_and_new_comparator_identity(self):
        for mode in reference.ARITHMETIC_MODES:
            with tempfile.TemporaryDirectory() as root:
                self.run_fixture(root, mode)

    def test_nonfinite_reference_and_repeat_disagreement_remain_raw_observations(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root, reference.ARITHMETIC_MODES[1], nonfinite=True)

    def test_existing_output_alias_input_drift_and_implementation_drift_fail_closed(self):
        for options in ({"existing": True}, {"alias": True}, {"mutate": True}, {"drift": True}):
            with self.subTest(options=options), tempfile.TemporaryDirectory() as root:
                self.run_fixture(root, reference.ARITHMETIC_MODES[0], **options)

    def test_invalid_mode_or_argument_count_fails_before_opening_inputs(self):
        with patch.object(subject, "implementation") as implementation:
            for arguments in ([], [None] + ["unused"] * 7, ["auto"] + ["unused"] * 7):
                with self.assertRaises(subject.Failure):
                    subject.run(arguments)
            implementation.assert_not_called()


if __name__ == "__main__":
    unittest.main()
