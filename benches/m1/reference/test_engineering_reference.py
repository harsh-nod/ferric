#!/usr/bin/env python3
"""CPU-only selected engineering tests, with no GPU or authority fixtures."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest import mock


def load_module(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


engineering = load_module("ferric_m1_engineering_reference", "engineering_run.py")
fixtures = load_module("ferric_m1_reference_test_fixtures", "test_reference.py")
core = engineering.core


def selected_fixture():
    plan, workload, files = fixtures.prefill_capture_fixture()
    runner = core.parse_canonical(files["runner.json"], "fixture runner")
    runner.update({
        "artifact_authority": "none",
        "authority": "aggregate-engineering-observation-only",
        "format": engineering.CAPTURE_FORMAT,
        "nonclaim": engineering.CAPTURE_NONCLAIM,
        "status": "OBSERVED-NON-AUTHORITATIVE",
    })
    files["runner.json"] = core.canonical_bytes(runner)
    output = core.parse_canonical(files["output.json"], "fixture output")
    output["runner_transcript_sha256"] = core.sha256_bytes(files["runner.json"])
    files["output.json"] = core.canonical_bytes(output)
    return plan, workload, files


def write_capture(root, files):
    bundle = root / f"{engineering.KIND}.capture.bundle"
    bundle.mkdir(parents=True)
    for name, data in files.items():
        (bundle / name).write_bytes(data)
    return bundle


def parsed_transcript(plan, workload, files):
    data = files["runner.json"]
    return engineering.parse_capture(core.parse_canonical(data, "fixture"), data, plan, workload, 7)


def coordinate_fixture(plan, files=None):
    observation = {
        "canonical_descriptor_sha256": "1" * 64, "compiler_handoff_sha256": "2" * 64,
        "hsaco_sha256": "3" * 64, "manifest_sha256": "4" * 64, "program_catalog_sha256": "5" * 64,
    }
    if files is not None:
        runner = core.parse_canonical(files["runner.json"], "fixture runner")
        observation["manifest_sha256"] = runner["kernel_artifact_manifest_sha256"]
        observation["program_catalog_sha256"] = runner["program_catalog_sha256"]
    return {
        "authority": "none", "derivation": engineering.DERIVATION, "format": engineering.CLOSURE_FORMAT,
        "model": {
            "admission_record_sha256": "6" * 64, "bundle_sha256": plan.identities["model"],
            "draft_prepacked_sha256": "8" * 64, "plan_catalog_sha256": "9" * 64,
            "target_prepacked_sha256": "a" * 64,
        },
        "nonclaim": engineering.CLOSURE_NONCLAIM, "observation": observation,
        "qualification": False, "target": core.TARGET,
    }


def engineering_common_fixture():
    plan, files = fixtures.common_fixture()
    value = core.parse_canonical(plan.data, "fixture plan")
    closure = coordinate_fixture(plan)
    files["closure.json"] = core.canonical_bytes(closure)
    files["acceptance-policy.json"] = core.canonical_bytes(engineering.diagnostic_policy())
    identities = value["identities"]
    identities["differential-acceptance-policy"] = core.sha256_bytes(files["acceptance-policy.json"])
    for name, label in (("ferric-source-closure", "ferric-source-coordinate"), ("fe2o3-source-closure", "fe2o3-source-coordinate"), ("benchmark-protocol", "protocol-coordinate")):
        identities[name] = engineering.coordinate_identity(closure, label)
    benchmark = core.parse_canonical(files["benchmark-input.json"], "fixture benchmark")
    benchmark["identities"] = identities
    files["benchmark-input.json"] = core.canonical_bytes(benchmark)
    value["input_sha256"] = core.sha256_bytes(files["benchmark-input.json"])
    data = core.canonical_bytes(value)
    return core.parse_plan(value, data), files


def engineering_workload_fixture():
    plan, files = engineering_common_fixture()
    value = core.parse_canonical(plan.data, "fixture plan")
    cases = []
    for kind in core.CASE_KINDS:
        rows, width, mode = core.CASE_GEOMETRY[kind]
        case_id = f"{kind}.001"
        tokens = tuple(index % 1000 for index in range(rows * width))
        payload = struct.pack(f"<{len(tokens)}I", *tokens)
        workload = {
            "case_id": case_id, "completion_wait_policy": core.COMPLETION_WAIT_POLICY,
            "format": core.WORKLOAD_FORMAT,
            "input": {"bytes": len(payload), "encoding": "u32-le", "path": f"{case_id}.tokens.u32le", "sha256": core.sha256_bytes(payload)},
            "kind": kind,
            "lanes": [{"active_length": 1 if mode == "decode" else width, "context_length": 8191 if mode == "decode" else 0} for _ in range(rows)],
            "selection": {"bucket": kind, "mode": mode, "role": "target-8b"},
        }
        data = core.canonical_bytes(workload)
        files[f"{case_id}.tokens.u32le"] = payload
        files[f"{case_id}.workload.json"] = data
        cases.append({"id": case_id, "input_sha256": core.sha256_bytes(payload), "kind": kind, "workload_sha256": core.sha256_bytes(data)})
    roster = {"cases": cases, "format": core.ROSTER_FORMAT, "suite": "differential"}
    files["roster.json"] = core.canonical_bytes(roster)
    value["cases"] = cases
    value["identities"]["workload-roster"] = core.sha256_bytes(files["roster.json"])
    benchmark = {"cases": cases, "format": core.BENCHMARK_INPUT_FORMAT, "identities": value["identities"], "suite": "differential", "target": core.TARGET}
    files["benchmark-input.json"] = core.canonical_bytes(benchmark)
    value["input_sha256"] = core.sha256_bytes(files["benchmark-input.json"])
    files["plan.json"] = core.canonical_bytes(value)
    return core.parse_plan(value, files["plan.json"]), files


class EngineeringReferenceTests(unittest.TestCase):
    def test_actual_independent_measurement_and_legacy_files(self):
        root = Path(__file__).parent
        measured = engineering.authenticate_artifacts(
            root / "engineering-implementation.json", root / "engineering-protocol.json"
        )
        self.assertEqual(measured, tuple(core.sha256_bytes((root / name).read_bytes()) for name in (
            "engineering-implementation.json", "engineering-protocol.json"
        )))
        self.assertNotEqual(measured[0], core.sha256_bytes((root / "implementation.json").read_bytes()))
        legacy = core.parse_canonical((root / "implementation.json").read_bytes(), "legacy measurement")
        for entry in legacy["files"]:
            data = (root / entry["path"]).read_bytes()
            self.assertEqual((len(data), core.sha256_bytes(data)), (entry["bytes"], entry["sha256"]))

    def test_engineering_and_qualification_transcripts_remain_disjoint(self):
        plan, workload, files = selected_fixture()
        parsed_transcript(plan, workload, files)
        with self.assertRaises(core.ReferenceFailure):
            core.parse_capture_transcript(core.parse_canonical(files["runner.json"], "fixture"), files["runner.json"], plan, workload, 7)
        _, _, qualified = fixtures.prefill_capture_fixture()
        with self.assertRaises(core.ReferenceFailure):
            parsed_transcript(plan, workload, qualified)

    def test_canonical_hostile_identity_selection_and_execution_rejected(self):
        plan, workload, files = selected_fixture()
        base = core.parse_canonical(files["runner.json"], "fixture")
        mutations = [
            ("artifact_authority", "qualification"), ("authority", "observed-target-only-qualification-capture"),
            ("benchmark_executable_sha256", "f" * 64), ("benchmark_protocol_sha256", "f" * 64),
            ("case_id", "prefill-s1-t512.001"), ("environment_sha256", "f" * 64),
            ("format", core.CAPTURE_FORMAT), ("gpu_unique_id", 8), ("input_sha256", "f" * 64),
            ("kind", "prefill-s1-t512"), ("plan_sha256", "f" * 64),
            ("runner_declaration_sha256", "f" * 64), ("status", "OBSERVED"),
            ("target", "gfx950:xnack-"), ("workload_sha256", "f" * 64),
            ("logits_row_sha256", []), ("compact_sha256", "z" * 64),
            ("dispatch_generation", 2), ("selection", {"bucket": engineering.KIND, "mode": "decode", "role": "target-8b"}),
            ("execution", {"dispatch_generation": 1, "epoch": 1, "mode": "one-shot-prefill", "round_count": 2}),
            ("unexpected", True),
        ]
        for key, value in mutations:
            with self.subTest(key=key):
                changed = dict(base)
                changed[key] = value
                data = core.canonical_bytes(changed)
                with self.assertRaises(core.ReferenceFailure):
                    engineering.parse_capture(changed, data, plan, workload, 7)

    def test_capture_full_payload_binding_and_roster_rejected(self):
        plan, workload, files = selected_fixture()
        for change in ("logits", "tokens", "row", "extra", "truncated", "symlink"):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                bundle = write_capture(root, files)
                if change == "logits":
                    (bundle / "logits.bf16le").write_bytes(b"\1\0" + files["logits.bf16le"][2:])
                elif change == "tokens":
                    (bundle / "tokens.u32le").write_bytes(struct.pack("<I", 1))
                elif change == "row":
                    runner = core.parse_canonical(files["runner.json"], "runner")
                    runner["logits_row_sha256"] = ["f" * 64]
                    (bundle / "runner.json").write_bytes(core.canonical_bytes(runner))
                elif change == "extra":
                    (bundle / "trailing").write_bytes(b"extra")
                elif change == "truncated":
                    (bundle / "logits.bf16le").write_bytes(files["logits.bf16le"][:-2])
                else:
                    (bundle / "tokens.u32le").unlink()
                    (root / "other").write_bytes(files["tokens.u32le"])
                    (bundle / "tokens.u32le").symlink_to(root / "other")
                with core.SecureDirectory.open(root, "fixture captures") as captures:
                    with self.assertRaises((core.ReferenceFailure, OSError)):
                        engineering.load_capture(captures, plan, workload, 7, coordinate_fixture(plan, files))

    def test_selected_manifest_is_nonqualifying_and_checks_finite_argmax(self):
        plan, workload, files = selected_fixture()
        transcript = parsed_transcript(plan, workload, files)
        logits, tokens = files["logits.bf16le"], files["tokens.u32le"]
        document = core.parse_canonical(engineering.reference_manifest(plan, workload, transcript, logits, tokens), "engineering output")
        self.assertEqual(document["authority"], "none")
        self.assertFalse(document["qualification"])
        self.assertEqual(document["format"], engineering.OUTPUT_FORMAT)
        self.assertNotEqual(document["format"], core.OUTPUT_FORMAT)
        for bad_logits, bad_tokens in (
            (logits[:-2], tokens), (logits, tokens + b"\0"),
            (logits, struct.pack("<I", 1)),
            (struct.pack("<H", 0x7F80) + logits[2:], tokens),
            (struct.pack("<H", 0x7FC1) + logits[2:], tokens),
        ):
            with self.assertRaises(core.ReferenceFailure):
                engineering.reference_manifest(plan, workload, transcript, bad_logits, bad_tokens)

    def test_selected_publisher_exact_roster_no_overwrite_and_mutation(self):
        plan, workload, files = selected_fixture()
        transcript = parsed_transcript(plan, workload, files)
        bundle = core.ReferenceBundle(
            case=workload.case, logits=files["logits.bf16le"], tokens=files["tokens.u32le"], runner=transcript.data,
            manifest=engineering.reference_manifest(plan, workload, transcript, files["logits.bf16le"], files["tokens.u32le"]),
        )
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "reference"
            with engineering.SelectedPublisher(output) as publisher:
                with self.assertRaises(core.ReferenceFailure):
                    publisher.publish()
                publisher.add(bundle)
                with self.assertRaises(core.ReferenceFailure):
                    publisher.add(bundle)
                publisher.publish()
            self.assertEqual({path.name for path in output.iterdir()}, {f"{engineering.KIND}.reference.bundle"})
            with engineering.SelectedPublisher(output) as publisher:
                publisher.add(bundle)
                with self.assertRaises(OSError):
                    publisher.publish()
            with engineering.SelectedPublisher(Path(temporary) / "mutated") as publisher:
                publisher.add(bundle)
                path = Path(temporary) / publisher.staging_name / publisher.bundle_names[0] / "tokens.u32le"
                path.write_bytes(struct.pack("<I", 1))
                with self.assertRaises(core.ReferenceFailure):
                    publisher.publish()

    def test_actual_selected_run_orchestration_two_executions_and_no_partial_result(self):
        plan, workload, files = selected_fixture()
        for mismatch in (False, True):
            with self.subTest(mismatch=mismatch), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                capture = root / "capture"
                write_capture(capture, files)
                plan_path = root / "plan.json"
                plan_path.write_bytes(plan.data)
                output = root / "reference"
                pair = (files["logits.bf16le"], files["tokens.u32le"])
                second = (pair[0], struct.pack("<I", 1)) if mismatch else pair
                with mock.patch.object(core, "require_isolated_python"), mock.patch.object(core, "require_virtual_environment"), \
                     mock.patch.object(engineering, "authenticate_artifacts", return_value=(plan.identities["reference-implementation"], plan.identities["reference-protocol"])), \
                     mock.patch.object(engineering, "load_engineering_workloads", return_value=((workload,), 7, coordinate_fixture(plan, files))), \
                     mock.patch.object(core, "authenticate_model_source") as source, \
                     mock.patch.object(core, "load_dependencies"), mock.patch.object(core, "load_model"), \
                     mock.patch.object(core, "execute_workload", side_effect=(pair, second)) as execute:
                    arguments = [engineering.CASE_ID, "implementation", "protocol", str(plan_path), "inputs", "model", str(capture), str(output)]
                    if mismatch:
                        with self.assertRaises(core.ReferenceFailure):
                            engineering.run(arguments)
                    else:
                        engineering.run(arguments)
                        source.return_value.__enter__.return_value.validate.assert_called_once()
                    self.assertEqual(execute.call_count, 2)
                    self.assertTrue(all(call.args[2] is workload for call in execute.call_args_list))
                self.assertEqual(output.exists(), not mismatch)

    def test_protocol_measurement_and_selected_case_fail_closed(self):
        root = Path(__file__).parent
        for case in ("", "decode-s1-c8192.001", "prefill-s1-t128", "prefill-s1-t512.001"):
            with self.assertRaises(core.ReferenceFailure):
                engineering.require_selected_case(case)
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary)
            (target / "protocol.json").write_bytes((root / "protocol.json").read_bytes())
            protocol = json.loads((root / "engineering-protocol.json").read_bytes())
            for key, value in (("qualification", True), ("case_id", "prefill-s1-t512.001"), ("authority", "qualified"), ("extra", 0)):
                changed = dict(protocol)
                changed[key] = value
                path = target / "engineering-protocol.json"
                path.write_bytes(core.canonical_bytes(changed))
                with self.assertRaises(core.ReferenceFailure):
                    engineering.authenticate_artifacts(root / "engineering-implementation.json", path)

    def test_engineering_common_documents_rederive_coordinates_and_disclaim_acceptance(self):
        plan, files = engineering_common_fixture()
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)
            for name, data in files.items():
                (path / name).write_bytes(data)
            with core.SecureDirectory.open(path, "fixture inputs") as inputs:
                gpu, closure = engineering.validate_engineering_common_documents(inputs, plan)
                self.assertEqual(gpu, 7)
                self.assertEqual(closure, coordinate_fixture(plan))
                with self.assertRaises(core.ReferenceFailure):
                    core.validate_common_documents(inputs, plan)
            original = core.parse_canonical(files["closure.json"], "fixture closure")
            for group in ("model", "observation"):
                for key in original[group]:
                    with self.subTest(group=group, key=key):
                        changed = json.loads(json.dumps(original))
                        changed[group][key] = "f" * 64
                        (path / "closure.json").write_bytes(core.canonical_bytes(changed))
                        with core.SecureDirectory.open(path, "mutated inputs") as inputs:
                            with self.assertRaises(core.ReferenceFailure):
                                engineering.validate_engineering_common_documents(inputs, plan)
            (path / "closure.json").write_bytes(files["closure.json"])
            policy = engineering.diagnostic_policy()
            policy["maximum_logit_ulp_error"] = 0
            (path / "acceptance-policy.json").write_bytes(core.canonical_bytes(policy))
            with core.SecureDirectory.open(path, "mutated inputs") as inputs:
                with self.assertRaises(core.ReferenceFailure):
                    engineering.validate_engineering_common_documents(inputs, plan)

    def test_qualification_closure_and_policy_are_not_engineering_coordinates(self):
        _, files = fixtures.common_fixture()
        with self.assertRaises(core.ReferenceFailure):
            engineering.parse_engineering_coordinates(core.parse_canonical(files["closure.json"], "qualification closure"))
        plan, _ = engineering_common_fixture()
        closure = coordinate_fixture(plan)
        for key, value in (("qualification", True), ("authority", "qualified"), ("kernel_proof_set", "f" * 64), ("derivation", "other")):
            changed = dict(closure)
            changed[key] = value
            with self.assertRaises(core.ReferenceFailure):
                engineering.parse_engineering_coordinates(changed)

    def test_exact_twenty_file_engineering_bundle_uses_all_seven_core_workload_decoders(self):
        plan, files = engineering_workload_fixture()
        self.assertEqual(len(files), 20)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, data in files.items():
                (root / name).write_bytes(data)
            workloads, gpu, _ = engineering.load_engineering_workloads(root, plan)
            self.assertEqual(gpu, 7)
            self.assertEqual(tuple(workload.case.kind for workload in workloads), core.CASE_KINDS)
            selected = next(workload for workload in workloads if workload.case.case_id == engineering.CASE_ID)
            self.assertEqual(selected.tokens, (tuple(range(128)),))
            with self.assertRaises(core.ReferenceFailure):
                core.load_workloads(root, plan)
            path = root / f"{engineering.CASE_ID}.tokens.u32le"
            path.write_bytes(path.read_bytes()[:-4])
            with self.assertRaises(core.ReferenceFailure):
                engineering.load_engineering_workloads(root, plan)
            path.write_bytes(files[path.name])
            (root / "trailing.json").write_bytes(b"{}\n")
            with self.assertRaises(core.ReferenceFailure):
                engineering.load_engineering_workloads(root, plan)


if __name__ == "__main__":
    unittest.main()
