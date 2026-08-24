#!/usr/bin/env python3
"""CPU-only unit tests for the Ferric M1 reference producer."""

from __future__ import annotations

import importlib.util
import os
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


MODULE_PATH = Path(__file__).with_name("run.py")
SPEC = importlib.util.spec_from_file_location("ferric_m1_reference", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
reference = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = reference
SPEC.loader.exec_module(reference)


def identities() -> dict[str, str]:
    result = {name: "1" * 64 for name in reference.PLAN_IDENTITIES}
    result["model"] = reference.PINNED_MODEL_IDENTITY
    return result


def plan_value() -> dict[str, object]:
    cases = [
        {
            "id": f"{kind}.001",
            "input_sha256": format(index + 2, "064x"),
            "kind": kind,
            "workload_sha256": format(index + 20, "064x"),
        }
        for index, kind in enumerate(reference.CASE_KINDS)
    ]
    return {
        "authority": "benchmark-run-plan-only",
        "cases": cases,
        "format": reference.PLAN_FORMAT,
        "identities": identities(),
        "input_sha256": "2" * 64,
        "milestone": "M1",
        "nonclaim": reference.DIFFERENTIAL_NONCLAIM,
        "obligation_id": "m1.r29",
        "path_id": "differential-bench",
        "source_path": "benches/m1/differential.rs",
        "suite": "differential",
        "target": reference.TARGET,
    }


def empty_bundle(case: reference.PlanCase) -> reference.ReferenceBundle:
    return reference.ReferenceBundle(
        case=case,
        logits=b"logits",
        tokens=b"tokens",
        runner=b"{}\n",
        manifest=b"{}\n",
    )


def common_fixture() -> tuple[reference.Plan, dict[str, bytes]]:
    value = plan_value()
    identities = value["identities"]
    assert isinstance(identities, dict)
    cases = value["cases"]
    closure = {
        "compiler": "3" * 64,
        "compiler_configuration": "4" * 64,
        "fe2o3_source": identities["fe2o3-source-closure"],
        "ferric_source": identities["ferric-source-closure"],
        "format": reference.QUALIFICATION_CLOSURE_FORMAT,
        "kernel_abi_catalog": "5" * 64,
        "kernel_proof_set": "6" * 64,
        "qualification_protocol": identities["benchmark-protocol"],
        "runtime_abi": "7" * 64,
        "runtime_contract": "8" * 64,
        "target_contract": "9" * 64,
        "tcb_report": "a" * 64,
        "validator_registry": "b" * 64,
    }
    environment = {
        "format": reference.ENVIRONMENT_FORMAT,
        "gpu_unique_id": 7,
        "target": reference.TARGET,
    }
    policy = {
        "authority": "externally-admitted-differential-threshold-policy-only",
        "cases": [
            {
                "kind": kind,
                "maximum_logit_ulp_error": 0,
                "maximum_token_mismatches": 0,
            }
            for kind in reference.CASE_KINDS
        ],
        "finite_logits_required": True,
        "format": reference.ACCEPTANCE_POLICY_FORMAT,
        "logit_metric": "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
        "nonclaim": reference.ACCEPTANCE_POLICY_NONCLAIM,
        "obligation_id": "m1.r29",
        "path_id": "differential-bench",
        "suite": "differential",
        "target": reference.TARGET,
        "token_metric": "ferric-reference-greedy-token-mismatch-count",
        "token_selection": "lowest-token-id-bf16-argmax",
    }
    roster = {
        "cases": cases,
        "format": reference.ROSTER_FORMAT,
        "suite": "differential",
    }
    documents = {
        "closure.json": reference.canonical_bytes(closure),
        "environment.json": reference.canonical_bytes(environment),
        "acceptance-policy.json": reference.canonical_bytes(policy),
        "roster.json": reference.canonical_bytes(roster),
    }
    identities["environment"] = reference.sha256_bytes(documents["environment.json"])
    identities["differential-acceptance-policy"] = reference.sha256_bytes(
        documents["acceptance-policy.json"]
    )
    identities["workload-roster"] = reference.sha256_bytes(documents["roster.json"])
    benchmark = {
        "cases": cases,
        "format": reference.BENCHMARK_INPUT_FORMAT,
        "identities": identities,
        "suite": "differential",
        "target": reference.TARGET,
    }
    documents["benchmark-input.json"] = reference.canonical_bytes(benchmark)
    value["input_sha256"] = reference.sha256_bytes(documents["benchmark-input.json"])
    plan_data = reference.canonical_bytes(value)
    return reference.parse_plan(value, plan_data), documents


def prefill_capture_fixture() -> tuple[
    reference.Plan, reference.Workload, dict[str, bytes]
]:
    plan, _ = common_fixture()
    case = next(case for case in plan.cases if case.kind == "prefill-s1-t128")
    workload = reference.Workload(
        data=b"workload",
        case=case,
        lanes=(reference.Lane(active_length=128, context_length=0),),
        tokens=(tuple(range(128)),),
    )
    logits = b"\0" * (reference.VOCABULARY_SIZE * reference.BF16_BYTES)
    tokens = b"\0" * reference.TOKEN_BYTES
    runner = {
        "authority": "observed-target-only-qualification-capture",
        "benchmark_executable_sha256": plan.identities["benchmark-executable"],
        "benchmark_protocol_sha256": plan.identities["benchmark-protocol"],
        "case_id": case.case_id,
        "compact_sha256": "2" * 64,
        "device_identity_sha256": "3" * 64,
        "dispatch_generation": 1,
        "environment_sha256": plan.identities["environment"],
        "execution": {
            "dispatch_generation": 1,
            "epoch": 1,
            "mode": "one-shot-prefill",
            "round_count": 1,
        },
        "format": reference.CAPTURE_FORMAT,
        "gpu_unique_id": 7,
        "input_sha256": case.input_sha256,
        "kernel_artifact_manifest_sha256": "4" * 64,
        "kind": case.kind,
        "logits_row_sha256": [reference.sha256_bytes(logits)],
        "logits_sha256": reference.sha256_bytes(logits),
        "nonclaim": reference.CAPTURE_NONCLAIM,
        "plan_sha256": plan.sha256,
        "program_catalog_sha256": "5" * 64,
        "runner_declaration_sha256": plan.identities["generated-plan"],
        "selection": {"bucket": case.kind, "mode": "prefill", "role": "target-8b"},
        "status": "OBSERVED",
        "target": reference.TARGET,
        "tokens_sha256": reference.sha256_bytes(tokens),
        "workload_sha256": case.workload_sha256,
    }
    runner_data = reference.canonical_bytes(runner)
    output = {
        "authority": "externally-collected-model-output-only",
        "case_id": case.case_id,
        "environment_sha256": plan.identities["environment"],
        "format": reference.OUTPUT_FORMAT,
        "input_sha256": case.input_sha256,
        "kind": case.kind,
        "logits": {
            "bytes": len(logits),
            "encoding": "bf16-le",
            "path": "logits.bf16le",
            "sha256": reference.sha256_bytes(logits),
        },
        "plan_sha256": plan.sha256,
        "producer": "ferric",
        "producer_sha256": plan.identities["benchmark-executable"],
        "protocol_sha256": plan.identities["benchmark-protocol"],
        "runner_transcript_sha256": reference.sha256_bytes(runner_data),
        "shape": {"rows": 1, "vocabulary_size": reference.VOCABULARY_SIZE},
        "tokens": {
            "bytes": len(tokens),
            "encoding": "u32-le",
            "path": "tokens.u32le",
            "sha256": reference.sha256_bytes(tokens),
        },
        "workload_sha256": case.workload_sha256,
    }
    return (
        plan,
        workload,
        {
            "logits.bf16le": logits,
            "tokens.u32le": tokens,
            "runner.json": runner_data,
            "output.json": reference.canonical_bytes(output),
        },
    )


class CanonicalTests(unittest.TestCase):
    def test_canonical_json_is_pretty_sorted_ascii_with_one_lf(self) -> None:
        value = {"z": [2, 1], "a": {"ok": True}}
        expected = (
            b'{\n  "a": {\n    "ok": true\n  },\n  "z": [\n    2,\n    1\n  ]\n}\n'
        )
        self.assertEqual(reference.canonical_bytes(value), expected)
        self.assertEqual(reference.parse_canonical(expected, "fixture"), value)

    def test_noncanonical_duplicate_float_and_nonascii_are_rejected(self) -> None:
        invalid = (
            b'{"a":1}\n',
            b'{\n  "a": 1,\n  "a": 2\n}\n',
            b'{\n  "a": 1.0\n}\n',
            '{\n  "a": "snowman \u2603"\n}\n'.encode(),
        )
        for data in invalid:
            with self.subTest(data=data), self.assertRaises(reference.ReferenceFailure):
                reference.parse_canonical(data, "invalid fixture")


class ParserTests(unittest.TestCase):
    def test_reference_protocol_requires_fixed_completion_wait_policy(self) -> None:
        data = MODULE_PATH.with_name("protocol.json").read_bytes()
        value = reference.parse_canonical(data, "reference protocol fixture")
        reference.validate_protocol(value)
        self.assertEqual(value["format"], "FERRIC-M1-REFERENCE-PROTOCOL-V2")

        replacements = (
            {**reference.COMPLETION_WAIT_POLICY, "unexpected": True},
            {
                **reference.COMPLETION_WAIT_POLICY,
                "max_consecutive_scans_without_progress": 8_191,
            },
        )
        for replacement in replacements:
            execution = {**value["execution"], "completion_wait_policy": replacement}
            mutated = {**value, "execution": execution}
            with self.subTest(replacement=replacement), self.assertRaises(
                reference.ReferenceFailure
            ):
                reference.validate_protocol(mutated)

        legacy_execution = dict(value["execution"])
        del legacy_execution["completion_wait_policy"]
        legacy_execution["workload_max_polls"] = 20_000_000
        with self.assertRaises(reference.ReferenceFailure):
            reference.validate_protocol({**value, "execution": legacy_execution})

    def test_capture_gpu_runner_and_row_identities_are_cross_bound(self) -> None:
        plan, workload, files = prefill_capture_fixture()
        with tempfile.TemporaryDirectory() as temporary:
            root_path = Path(temporary)
            bundle_path = root_path / f"{workload.case.kind}.capture.bundle"
            bundle_path.mkdir()
            for name, data in files.items():
                (bundle_path / name).write_bytes(data)
            with reference.SecureDirectory.open(root_path, "capture fixture") as root:
                reference.load_capture(root, plan, workload, 7)
                with self.assertRaises(reference.ReferenceFailure):
                    reference.load_capture(root, plan, workload, 8)

                runner = reference.parse_canonical(
                    files["runner.json"], "runner fixture"
                )
                runner["logits_row_sha256"] = ["f" * 64]
                runner_data = reference.canonical_bytes(runner)
                (bundle_path / "runner.json").write_bytes(runner_data)
                output = reference.parse_canonical(
                    files["output.json"], "output fixture"
                )
                output["runner_transcript_sha256"] = reference.sha256_bytes(runner_data)
                (bundle_path / "output.json").write_bytes(
                    reference.canonical_bytes(output)
                )
                with self.assertRaises(reference.ReferenceFailure):
                    reference.load_capture(root, plan, workload, 7)

    def test_common_documents_are_hash_and_semantically_bound(self) -> None:
        plan, documents = common_fixture()
        with tempfile.TemporaryDirectory() as temporary:
            root_path = Path(temporary)
            for name, data in documents.items():
                (root_path / name).write_bytes(data)
            with reference.SecureDirectory.open(root_path, "common fixture") as root:
                self.assertEqual(reference.validate_common_documents(root, plan), 7)
                environment = reference.parse_canonical(
                    documents["environment.json"], "environment fixture"
                )
                environment["gpu_unique_id"] = 8
                (root_path / "environment.json").write_bytes(
                    reference.canonical_bytes(environment)
                )
                with self.assertRaises(reference.ReferenceFailure):
                    reference.validate_common_documents(root, plan)

    def test_closure_semantic_substitution_is_rejected_without_a_document_hash(
        self,
    ) -> None:
        plan, documents = common_fixture()
        closure = reference.parse_canonical(
            documents["closure.json"], "closure fixture"
        )
        closure["ferric_source"] = "f" * 64
        documents["closure.json"] = reference.canonical_bytes(closure)
        with tempfile.TemporaryDirectory() as temporary:
            root_path = Path(temporary)
            for name, data in documents.items():
                (root_path / name).write_bytes(data)
            with reference.SecureDirectory.open(root_path, "common fixture") as root:
                with self.assertRaises(reference.ReferenceFailure):
                    reference.validate_common_documents(root, plan)

    def test_exact_seven_case_plan_and_pinned_model_are_required(self) -> None:
        value = plan_value()
        data = reference.canonical_bytes(value)
        plan = reference.parse_plan(value, data)
        self.assertEqual(tuple(case.kind for case in plan.cases), reference.CASE_KINDS)
        value["identities"]["model"] = "0" * 64  # type: ignore[index]
        with self.assertRaises(reference.ReferenceFailure):
            reference.parse_plan(value, reference.canonical_bytes(value))

    def test_workload_input_is_canonical_hash_bound_and_range_checked(self) -> None:
        self.assertEqual(reference.WORKLOAD_FORMAT, "FERRIC-M1-QUALIFICATION-WORKLOAD-V2")
        kind = "prefill-s1-t128"
        tokens = tuple(range(128))
        payload = struct.pack("<128I", *tokens)
        input_sha = reference.sha256_bytes(payload)
        value = {
            "case_id": f"{kind}.001",
            "completion_wait_policy": reference.COMPLETION_WAIT_POLICY,
            "format": reference.WORKLOAD_FORMAT,
            "input": {
                "bytes": len(payload),
                "encoding": "u32-le",
                "path": f"{kind}.001.tokens.u32le",
                "sha256": input_sha,
            },
            "kind": kind,
            "lanes": [{"active_length": 128, "context_length": 0}],
            "selection": {"bucket": kind, "mode": "prefill", "role": "target-8b"},
        }
        data = reference.canonical_bytes(value)
        case = reference.PlanCase(
            f"{kind}.001", input_sha, kind, reference.sha256_bytes(data)
        )
        with tempfile.TemporaryDirectory() as temporary:
            root_path = Path(temporary)
            payload_path = root_path / f"{kind}.001.tokens.u32le"
            payload_path.write_bytes(payload)
            with reference.SecureDirectory.open(root_path, "fixture root") as root:
                workload = reference.parse_workload(value, data, case, root)
                self.assertEqual(workload.tokens, (tokens,))
                replacements = (
                    {**reference.COMPLETION_WAIT_POLICY, "unexpected": True},
                    {
                        **reference.COMPLETION_WAIT_POLICY,
                        "timeout_basis": "wall-clock",
                    },
                )
                for replacement in replacements:
                    mutated = {**value, "completion_wait_policy": replacement}
                    mutated_data = reference.canonical_bytes(mutated)
                    mutated_case = reference.PlanCase(
                        f"{kind}.001",
                        input_sha,
                        kind,
                        reference.sha256_bytes(mutated_data),
                    )
                    with self.subTest(replacement=replacement), self.assertRaises(
                        reference.ReferenceFailure
                    ):
                        reference.parse_workload(
                            mutated, mutated_data, mutated_case, root
                        )

                legacy = dict(value)
                del legacy["completion_wait_policy"]
                legacy["max_polls"] = 20_000_000
                legacy_data = reference.canonical_bytes(legacy)
                legacy_case = reference.PlanCase(
                    f"{kind}.001",
                    input_sha,
                    kind,
                    reference.sha256_bytes(legacy_data),
                )
                with self.assertRaises(reference.ReferenceFailure):
                    reference.parse_workload(legacy, legacy_data, legacy_case, root)

                payload_path.write_bytes(payload[:-4] + struct.pack("<I", 151_643))
                with self.assertRaises(reference.ReferenceFailure):
                    reference.parse_workload(value, data, case, root)


class Bf16Tests(unittest.TestCase):
    def test_lowest_id_argmax_uses_exact_serialized_bf16(self) -> None:
        values = [0] * reference.VOCABULARY_SIZE
        values[4] = 0x3F80
        values[7] = 0x3F80
        row = struct.pack(f"<{len(values)}H", *values)
        self.assertEqual(reference.bf16_argmax(row), 4)

    def test_signed_zero_ties_and_nonfinite_rejection(self) -> None:
        values = [0x8000, 0] + [0xBF80] * (reference.VOCABULARY_SIZE - 2)
        row = struct.pack(f"<{len(values)}H", *values)
        self.assertEqual(reference.bf16_argmax(row), 0)
        values[11] = 0x7F80
        with self.assertRaises(reference.ReferenceFailure):
            reference.bf16_argmax(struct.pack(f"<{len(values)}H", *values))


class ManifestTests(unittest.TestCase):
    def test_reference_output_manifest_binds_reference_and_runner(self) -> None:
        value = plan_value()
        plan = reference.parse_plan(value, reference.canonical_bytes(value))
        case = plan.cases[3]
        workload = reference.Workload(b"workload", case, (), ())
        transcript = reference.CaptureTranscript(b"runner\n", "a" * 64, case)
        logits = b"\0" * (reference.VOCABULARY_SIZE * 2)
        tokens = b"\0" * 4
        data = reference.reference_manifest(plan, workload, transcript, logits, tokens)
        manifest = reference.parse_canonical(data, "reference output")
        self.assertEqual(manifest["producer"], "reference")
        self.assertEqual(
            manifest["producer_sha256"], plan.identities["reference-implementation"]
        )
        self.assertEqual(manifest["runner_transcript_sha256"], "a" * 64)
        self.assertEqual(manifest["logits"]["sha256"], reference.sha256_bytes(logits))

    def test_implementation_manifest_rejects_file_tamper(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root_path = Path(temporary)
            contents = {
                "pyproject.toml": b"project\n",
                "run.py": b"runner\n",
                "uv.lock": b"lock\n",
            }
            for name, data in contents.items():
                (root_path / name).write_bytes(data)
            value = {
                "authority": "source-reviewed-reference-implementation-closure-only",
                "files": [
                    {
                        "bytes": len(contents[name]),
                        "path": name,
                        "sha256": reference.sha256_bytes(contents[name]),
                    }
                    for name in sorted(contents)
                ],
                "format": reference.IMPLEMENTATION_FORMAT,
                "python": "3.12",
            }
            data = reference.canonical_bytes(value)
            with reference.SecureDirectory.open(
                root_path, "implementation fixture"
            ) as root:
                identity = reference.parse_implementation_manifest(
                    value, data, root, root_path / "run.py"
                )
                self.assertEqual(identity, reference.sha256_bytes(data))
                (root_path / "uv.lock").write_bytes(b"tampered\n")
                with self.assertRaises(reference.ReferenceFailure):
                    reference.parse_implementation_manifest(
                        value, data, root, root_path / "run.py"
                    )


class SecurityTests(unittest.TestCase):
    def test_python_isolation_flags_are_required(self) -> None:
        admitted = SimpleNamespace(
            ignore_environment=1, isolated=1, no_user_site=1, safe_path=True
        )
        reference.require_isolated_python(admitted)
        for name in ("ignore_environment", "isolated", "no_user_site", "safe_path"):
            rejected = SimpleNamespace(**vars(admitted))
            setattr(rejected, name, False if name == "safe_path" else 0)
            with self.subTest(flag=name), self.assertRaises(reference.ReferenceFailure):
                reference.require_isolated_python(rejected)

    def test_non_base_virtual_environment_is_required(self) -> None:
        reference.require_virtual_environment(
            Path("/venv"), Path("/usr"), Path("/venv/bin/python")
        )
        with self.assertRaises(reference.ReferenceFailure):
            reference.require_virtual_environment(
                Path("/usr"), Path("/usr"), Path("/usr/bin/python")
            )
        with self.assertRaises(reference.ReferenceFailure):
            reference.require_virtual_environment(
                Path("/venv"), Path("/usr"), Path("/usr/bin/python")
            )

    def test_visible_gpu_must_be_gfx942_xnack_minus(self) -> None:
        class Cuda:
            architecture = "gfx942:sramecc+:xnack-"

            @staticmethod
            def is_available() -> bool:
                return True

            @staticmethod
            def device_count() -> int:
                return 1

            @classmethod
            def get_device_properties(cls, _: int) -> SimpleNamespace:
                return SimpleNamespace(gcnArchName=cls.architecture)

        torch = SimpleNamespace(cuda=Cuda())
        reference.validate_gpu_target(torch)
        for architecture in ("gfx942:sramecc+:xnack+", "gfx90a:sramecc+:xnack-"):
            with self.subTest(architecture=architecture):
                Cuda.architecture = architecture
                with self.assertRaises(reference.ReferenceFailure):
                    reference.validate_gpu_target(torch)

    def test_same_name_inode_substitution_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root_path = Path(temporary)
            (root_path / "shard").write_bytes(b"authenticated")
            replacement = root_path.parent / f"{root_path.name}.replacement"
            replacement.write_bytes(b"replacement")
            try:
                with reference.SecureDirectory.open(root_path, "model fixture") as root:
                    held = root.open_file("shard", "held shard")
                    try:
                        os.replace(replacement, root_path / "shard")
                        with self.assertRaises(reference.ReferenceFailure):
                            reference.validate_bound_files(
                                root, {"shard": held}, "model fixture"
                            )
                    finally:
                        held.close()
            finally:
                replacement.unlink(missing_ok=True)

    def test_symlink_and_hardlink_inputs_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root_path = Path(temporary)
            original = root_path / "original"
            original.write_bytes(b"data")
            os.link(original, root_path / "hardlink")
            os.symlink("original", root_path / "symlink")
            with reference.SecureDirectory.open(root_path, "input fixture") as root:
                with self.assertRaises(reference.ReferenceFailure):
                    root.open_file("original", "hard-linked input")
                with self.assertRaises(reference.ReferenceFailure):
                    root.open_file("symlink", "symlink input")


class PublicationTests(unittest.TestCase):
    def test_partial_bundle_failure_is_removed(self) -> None:
        plan = reference.parse_plan(
            plan_value(), reference.canonical_bytes(plan_value())
        )
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "reference-output"
            original_write = reference.write_new
            writes = 0

            def fail_second_write(*args: object, **kwargs: object) -> None:
                nonlocal writes
                writes += 1
                if writes == 2:
                    raise OSError("injected staged write failure")
                original_write(*args, **kwargs)

            with self.assertRaises(OSError):
                with reference.OutputPublisher(output) as publisher:
                    with mock.patch.object(
                        reference, "write_new", side_effect=fail_second_write
                    ):
                        publisher.add(empty_bundle(plan.cases[0]))
            self.assertEqual(list(Path(temporary).iterdir()), [])

    def test_staging_name_inode_substitution_is_rejected(self) -> None:
        plan = reference.parse_plan(
            plan_value(), reference.canonical_bytes(plan_value())
        )
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "reference-output"
            publisher = reference.OutputPublisher(output)
            stolen = root / "stolen-staging"
            try:
                for case in plan.cases:
                    publisher.add(empty_bundle(case))
                os.rename(root / publisher.staging_name, stolen)
                os.mkdir(root / publisher.staging_name)
                with self.assertRaises(reference.ReferenceFailure):
                    publisher.publish()
                self.assertFalse(output.exists())
            finally:
                publisher.close()
                stolen.rmdir()

    def test_exact_named_bundles_publish_once_without_replacement(self) -> None:
        plan = reference.parse_plan(
            plan_value(), reference.canonical_bytes(plan_value())
        )
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "reference-output"
            with reference.OutputPublisher(output) as publisher:
                for case in plan.cases:
                    publisher.add(empty_bundle(case))
                publisher.publish()
            expected = {f"{kind}.reference.bundle" for kind in reference.CASE_KINDS}
            self.assertEqual({path.name for path in output.iterdir()}, expected)
            with self.assertRaises(FileExistsError):
                with reference.OutputPublisher(output) as publisher:
                    for case in plan.cases:
                        publisher.add(empty_bundle(case))
                    publisher.publish()
            self.assertEqual(
                {path.name for path in Path(temporary).iterdir()}, {"reference-output"}
            )


if __name__ == "__main__":
    unittest.main()
