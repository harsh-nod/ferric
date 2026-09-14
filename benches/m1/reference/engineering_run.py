#!/usr/bin/env python3
"""Selected-case engineering reference bytes; never qualification acceptance."""

from __future__ import annotations

import importlib.util
import hashlib
import os
from pathlib import Path
import sys
from typing import Any


def _load_reference_core() -> Any:
    path = Path(__file__).with_name("run.py")
    spec = importlib.util.spec_from_file_location("ferric_m1_reference_core", path)
    if spec is None or spec.loader is None:
        raise ImportError("cannot load the exact sibling reference core")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


core = _load_reference_core()
Failure = core.ReferenceFailure
CASE_ID = "prefill-s1-t128.001"
KIND = "prefill-s1-t128"
IMPLEMENTATION_FORMAT = "FERRIC-M1-ENGINEERING-REFERENCE-IMPLEMENTATION-V1"
PROTOCOL_FORMAT = "FERRIC-M1-ENGINEERING-REFERENCE-PROTOCOL-V1"
OUTPUT_FORMAT = "FERRIC-M1-ENGINEERING-REFERENCE-OUTPUT-V1"
CAPTURE_FORMAT = "FERRIC-M1-TECHNICAL-PREQUALIFICATION-CAPTURE-V1"
CLOSURE_FORMAT = "FERRIC-M1-ENGINEERING-REPRODUCIBILITY-CLOSURE-V1"
POLICY_FORMAT = "FERRIC-M1-ENGINEERING-DIAGNOSTIC-POLICY-V1"
DERIVATION = "ferric.m1.r29-engineering-coordinates.v1"
CLOSURE_NONCLAIM = "Authority-none reproducibility coordinates derived from admitted engineering observation and model-plan bytes only. Runner identity slots are inert engineering coordinates, not compiler-origin, proof, runtime-contract, validator, TCB, or current-publication evidence. No qualification authority or M1 gate closure is supplied."
POLICY_NONCLAIM = "Engineering numerical diagnostics only. This policy specifies finite BF16 and token-comparison metrics without thresholds or acceptance, and supplies no qualification authority or M1 gate closure."
CAPTURE_NONCLAIM = (
    "Authority-none aggregate engineering observation only. This transcript "
    "authenticates no compiler origin or Worker V3 publication, selects no current "
    "protected publication, establishes no reference comparison, tolerance, "
    "numerical correctness, hardware correctness, performance, qualification, or "
    "m1.r29 closure."
)
NONCLAIM = (
    "Selected-case engineering reference bytes only. Authority is none; this is "
    "not a qualification result, reviewed tolerance, benchmark, protected "
    "publication, full seven-case R29 comparison, or M1 gate closure."
)


def require_selected_case(case_id: str) -> None:
    if case_id != CASE_ID:
        raise Failure("engineering reference supports only prefill-s1-t128.001")


def authenticate_artifacts(implementation: Path, protocol: Path) -> tuple[str, str]:
    parent, name = core.open_parent(implementation, "engineering implementation")
    with parent:
        value, data, _ = parent.read_canonical(name, "engineering implementation")
        document = core.exact_object(
            value, ("authority", "files", "format", "python"),
            "engineering implementation",
        )
        core.expect_string(document, "authority", "none", "engineering implementation")
        core.expect_string(document, "format", IMPLEMENTATION_FORMAT, "engineering implementation")
        core.expect_string(document, "python", "3.12", "engineering implementation")
        names = ("engineering_run.py", "pyproject.toml", "run.py", "uv.lock")
        files = core.exact_array(document["files"], len(names), "engineering implementation files")
        for expected, raw in zip(names, files, strict=True):
            entry = core.exact_object(raw, ("bytes", "path", "sha256"), "engineering source")
            core.expect_string(entry, "path", expected, "engineering source")
            length = core.integer_field(entry, "bytes", "engineering source")
            digest = core.require_sha256(entry["sha256"], "engineering source hash")
            if length == 0:
                raise Failure("engineering source must not be empty")
            with parent.open_file(expected, "engineering source") as source:
                source.digest(length, digest)
                if expected in ("engineering_run.py", "run.py"):
                    actual_path = Path(__file__) if expected == "engineering_run.py" else Path(core.__file__)
                    with core.open_parent(actual_path, "running engineering source")[0] as actual_parent:
                        with actual_parent.open_file(actual_path.name, "running engineering source") as actual:
                            actual.digest(length, digest)
                            if actual.identity != source.identity:
                                raise Failure("measured source is not the running engineering source")
        implementation_hash = core.sha256_bytes(data)
    protocol_parent, protocol_name = core.open_parent(protocol, "engineering protocol")
    with protocol_parent:
        value, protocol_data, _ = protocol_parent.read_canonical(protocol_name, "engineering protocol")
        document = core.exact_object(
            value,
            ("authority", "base_protocol", "case_id", "format", "input", "invocation", "nonclaim", "output_format", "qualification", "target"),
            "engineering protocol",
        )
        for key, expected in (
            ("authority", "none"), ("case_id", CASE_ID),
            ("format", PROTOCOL_FORMAT), ("nonclaim", NONCLAIM),
            ("output_format", OUTPUT_FORMAT), ("target", core.TARGET),
        ):
            core.expect_string(document, key, expected, "engineering protocol")
        if document["qualification"] is not False:
            raise Failure("engineering protocol must explicitly disclaim qualification")
        if document["input"] != {"closure_format": CLOSURE_FORMAT, "derivation": DERIVATION, "policy_format": POLICY_FORMAT}:
            raise Failure("engineering input protocol drifted")
        if document["invocation"] != {
            "arguments": ["CASE-ID", "IMPLEMENTATION-MANIFEST", "PROTOCOL", "PLAN", "INPUT-BUNDLE", "MODEL-SOURCE", "FERRIC-CAPTURE-ROOT", "OUTPUT-ROOT"],
            "command": "VENV/bin/python -I engineering_run.py",
            "mode": "selected-case-single-model-load-two-byte-identical-executions",
        }:
            raise Failure("engineering reference invocation drifted")
        base = core.exact_object(document["base_protocol"], ("bytes", "path", "sha256"), "base reference protocol")
        core.expect_string(base, "path", "protocol.json", "base reference protocol")
        with protocol_parent.open_file("protocol.json", "base reference protocol") as source:
            length = core.integer_field(base, "bytes", "base reference protocol")
            source.digest(length, core.require_sha256(base["sha256"], "base reference protocol hash"))
            core.validate_protocol(core.parse_canonical(source.read(exact=length), "base reference protocol"))
    return implementation_hash, core.sha256_bytes(protocol_data)


def parse_engineering_coordinates(value: Any) -> dict[str, Any]:
    closure = core.exact_object(value, ("authority", "derivation", "format", "model", "nonclaim", "observation", "qualification", "target"), "engineering closure")
    for key, expected in (("authority", "none"), ("derivation", DERIVATION), ("format", CLOSURE_FORMAT), ("nonclaim", CLOSURE_NONCLAIM), ("target", core.TARGET)):
        core.expect_string(closure, key, expected, "engineering closure")
    if closure["qualification"] is not False:
        raise Failure("engineering closure must disclaim qualification")
    groups = {
        "model": ("admission_record_sha256", "bundle_sha256", "draft_prepacked_sha256", "plan_catalog_sha256", "target_prepacked_sha256"),
        "observation": ("canonical_descriptor_sha256", "compiler_handoff_sha256", "hsaco_sha256", "manifest_sha256", "program_catalog_sha256"),
    }
    for group, fields in groups.items():
        values = core.exact_object(closure[group], fields, f"engineering {group} coordinates")
        for key in fields:
            core.require_sha256(values[key], f"engineering coordinate {key}")
    return closure


def coordinate_identity(closure: dict[str, Any], label: str) -> str:
    observation, model = closure["observation"], closure["model"]
    fields = [DERIVATION.encode("ascii"), label.encode("ascii")]
    fields.extend(bytes.fromhex(observation[key]) for key in (
        "manifest_sha256", "hsaco_sha256", "compiler_handoff_sha256", "canonical_descriptor_sha256", "program_catalog_sha256",
    ))
    fields.extend(bytes.fromhex(model[key]) for key in (
        "admission_record_sha256", "bundle_sha256", "target_prepacked_sha256", "draft_prepacked_sha256", "plan_catalog_sha256",
    ))
    digest = hashlib.sha256()
    for field in fields:
        digest.update(len(field).to_bytes(8, "little"))
        digest.update(field)
    return digest.hexdigest()


def diagnostic_policy() -> dict[str, Any]:
    return {
        "authority": "none", "case_kinds": list(core.CASE_KINDS), "finite_logits_required": True,
        "format": POLICY_FORMAT, "logit_metric": "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
        "nonclaim": POLICY_NONCLAIM, "qualification": False, "target": core.TARGET,
        "token_metric": "ferric-reference-greedy-token-mismatch-count", "token_selection": "lowest-token-id-bf16-argmax",
    }


def validate_engineering_common_documents(inputs: Any, plan: Any) -> tuple[int, dict[str, Any]]:
    benchmark, benchmark_data, _ = inputs.read_canonical("benchmark-input.json", "engineering benchmark input")
    benchmark = core.exact_object(benchmark, ("cases", "format", "identities", "suite", "target"), "engineering benchmark input")
    for key, expected in (("format", core.BENCHMARK_INPUT_FORMAT), ("suite", "differential"), ("target", core.TARGET)):
        core.expect_string(benchmark, key, expected, "engineering benchmark input")
    if core.sha256_bytes(benchmark_data) != plan.input_sha256 or benchmark["cases"] != core.plan_case_values(plan) or benchmark["identities"] != plan.identities:
        raise Failure("engineering benchmark input differs from the exact plan")
    roster, roster_data, _ = inputs.read_canonical("roster.json", "engineering workload roster")
    roster = core.exact_object(roster, ("cases", "format", "suite"), "engineering workload roster")
    core.expect_string(roster, "format", core.ROSTER_FORMAT, "engineering workload roster")
    core.expect_string(roster, "suite", "differential", "engineering workload roster")
    if core.sha256_bytes(roster_data) != plan.identities["workload-roster"] or roster["cases"] != core.plan_case_values(plan):
        raise Failure("engineering workload roster differs from the plan")
    environment, environment_data, _ = inputs.read_canonical("environment.json", "engineering environment")
    environment = core.exact_object(environment, ("format", "gpu_unique_id", "target"), "engineering environment")
    core.expect_string(environment, "format", core.ENVIRONMENT_FORMAT, "engineering environment")
    core.expect_string(environment, "target", core.TARGET, "engineering environment")
    gpu = core.integer_field(environment, "gpu_unique_id", "engineering environment")
    if gpu == 0 or core.sha256_bytes(environment_data) != plan.identities["environment"]:
        raise Failure("engineering environment differs from the plan")
    policy, policy_data, _ = inputs.read_canonical("acceptance-policy.json", "engineering diagnostic policy")
    if core.canonical_bytes(policy) != core.canonical_bytes(diagnostic_policy()) or core.sha256_bytes(policy_data) != plan.identities["differential-acceptance-policy"]:
        raise Failure("engineering diagnostic policy differs from its exact plan binding")
    closure, _, _ = inputs.read_canonical("closure.json", "engineering reproducibility closure")
    closure = parse_engineering_coordinates(closure)
    for identity_name, label in (
        ("ferric-source-closure", "ferric-source-coordinate"), ("fe2o3-source-closure", "fe2o3-source-coordinate"),
        ("benchmark-protocol", "protocol-coordinate"),
    ):
        if coordinate_identity(closure, label) != plan.identities[identity_name]:
            raise Failure(f"engineering coordinate differs from plan binding {identity_name}")
    if closure["model"]["bundle_sha256"] != plan.identities["model"]:
        raise Failure("engineering closure model differs from the plan")
    return gpu, closure


def load_engineering_workloads(input_path: Path, plan: Any) -> tuple[tuple[Any, ...], int, dict[str, Any]]:
    with core.SecureDirectory.open(input_path, "engineering input bundle") as inputs:
        if inputs.entries() != core.expected_input_entries(plan):
            raise Failure("engineering input bundle must contain the exact twenty files")
        _, bundled_plan, _ = inputs.read_canonical("plan.json", "engineering bundled plan")
        if bundled_plan != plan.data:
            raise Failure("engineering bundled plan differs from the supplied plan")
        gpu, closure = validate_engineering_common_documents(inputs, plan)
        workloads = []
        for case in plan.cases:
            value, data, _ = inputs.read_canonical(f"{case.case_id}.workload.json", "engineering workload")
            workloads.append(core.parse_workload(value, data, case, inputs))
        return tuple(workloads), gpu, closure


def parse_capture(value: Any, data: bytes, plan: Any, workload: Any, gpu_unique_id: int) -> Any:
    require_selected_case(workload.case.case_id)
    document = core.exact_object(
        value,
        ("artifact_authority", "authority", "benchmark_executable_sha256", "benchmark_protocol_sha256", "case_id", "compact_sha256", "device_identity_sha256", "dispatch_generation", "environment_sha256", "execution", "format", "gpu_unique_id", "input_sha256", "kernel_artifact_manifest_sha256", "kind", "logits_row_sha256", "logits_sha256", "nonclaim", "plan_sha256", "program_catalog_sha256", "runner_declaration_sha256", "selection", "status", "target", "tokens_sha256", "workload_sha256"),
        "engineering capture transcript",
    )
    bindings = {
        "artifact_authority": "none",
        "authority": "aggregate-engineering-observation-only",
        "benchmark_executable_sha256": plan.identities["benchmark-executable"],
        "benchmark_protocol_sha256": plan.identities["benchmark-protocol"],
        "case_id": CASE_ID,
        "environment_sha256": plan.identities["environment"],
        "format": CAPTURE_FORMAT,
        "input_sha256": workload.case.input_sha256,
        "kind": KIND,
        "nonclaim": CAPTURE_NONCLAIM,
        "plan_sha256": plan.sha256,
        "runner_declaration_sha256": plan.identities["generated-plan"],
        "status": "OBSERVED-NON-AUTHORITATIVE",
        "target": core.TARGET,
        "workload_sha256": workload.case.workload_sha256,
    }
    for key, expected in bindings.items():
        core.expect_string(document, key, expected, "engineering capture transcript")
    for key in ("compact_sha256", "device_identity_sha256", "kernel_artifact_manifest_sha256", "logits_sha256", "program_catalog_sha256", "runner_declaration_sha256", "tokens_sha256"):
        core.require_sha256(document[key], "engineering capture identity")
    core.expect_integer(document, "gpu_unique_id", gpu_unique_id, "engineering capture transcript")
    generation = core.integer_field(document, "dispatch_generation", "engineering capture transcript")
    if generation == 0:
        raise Failure("engineering capture generation must be nonzero")
    if document["selection"] != {"bucket": KIND, "mode": "prefill", "role": "target-8b"}:
        raise Failure("engineering capture selection drifted")
    hashes = core.exact_array(document["logits_row_sha256"], 1, "engineering capture row hashes")
    core.require_sha256(hashes[0], "engineering capture row hash")
    if hashes[0] != document["logits_sha256"]:
        raise Failure("engineering single-row identity differs from the logit payload")
    core._validate_capture_execution(document["execution"], plan, workload, generation)
    return core.CaptureTranscript(data=data, sha256=core.sha256_bytes(data), case=workload.case)


def load_capture(captures: Any, plan: Any, workload: Any, gpu_unique_id: int, coordinates: dict[str, Any]) -> Any:
    require_selected_case(workload.case.case_id)
    with captures.child(f"{KIND}.capture.bundle", "engineering capture bundle") as bundle:
        if bundle.entries() != {"logits.bf16le", "output.json", "runner.json", "tokens.u32le"}:
            raise Failure("engineering capture file roster drifted")
        value, data, _ = bundle.read_canonical("runner.json", "engineering capture transcript")
        transcript = parse_capture(value, data, plan, workload, gpu_unique_id)
        for field, key in (("kernel_artifact_manifest_sha256", "manifest_sha256"), ("program_catalog_sha256", "program_catalog_sha256")):
            core.expect_string(value, field, coordinates["observation"][key], "engineering capture artifact binding")
        output, _, _ = bundle.read_canonical("output.json", "engineering capture output")
        core.validate_ferric_output(output, plan, workload, transcript, bundle)
        return transcript


def reference_manifest(plan: Any, workload: Any, transcript: Any, logits: bytes, tokens: bytes) -> bytes:
    require_selected_case(workload.case.case_id)
    if len(logits) != core.VOCABULARY_SIZE * 2 or len(tokens) != 4:
        raise Failure("engineering reference output extent drifted")
    if core.bf16_argmax(logits) != int.from_bytes(tokens, "little"):
        raise Failure("engineering reference token differs from finite BF16 argmax")
    return core.canonical_bytes({
        "authority": "none",
        "case_id": CASE_ID,
        "environment_sha256": plan.identities["environment"],
        "format": OUTPUT_FORMAT,
        "input_sha256": workload.case.input_sha256,
        "kind": KIND,
        "logits": {"bytes": len(logits), "encoding": "bf16-le", "path": "logits.bf16le", "sha256": core.sha256_bytes(logits)},
        "nonclaim": NONCLAIM,
        "plan_sha256": plan.sha256,
        "producer": "reference",
        "producer_sha256": plan.identities["reference-implementation"],
        "protocol_sha256": plan.identities["reference-protocol"],
        "qualification": False,
        "runner_transcript_sha256": transcript.sha256,
        "shape": {"rows": 1, "vocabulary_size": core.VOCABULARY_SIZE},
        "tokens": {"bytes": len(tokens), "encoding": "u32-le", "path": "tokens.u32le", "sha256": core.sha256_bytes(tokens)},
        "workload_sha256": workload.case.workload_sha256,
    })


class SelectedPublisher(core.OutputPublisher):
    def add(self, bundle: Any) -> None:
        require_selected_case(bundle.case.case_id)
        value = core.parse_canonical(bundle.manifest, "engineering reference output")
        if value.get("format") != OUTPUT_FORMAT or value.get("authority") != "none" or value.get("qualification") is not False:
            raise Failure("engineering publisher rejects qualification output")
        super().add(bundle)

    def publish(self) -> None:
        if self.bundle_names != [f"{KIND}.reference.bundle"]:
            raise Failure("engineering publisher requires the exact selected case")
        self.validate_staging()
        os.fsync(self.staging.fd)
        with self.parent.child(self.staging_name, "rebound engineering staging root") as rebound:
            if rebound.identity != self.staging.identity or rebound.entries() != set(self.bundle_names):
                raise Failure("engineering staging binding drifted")
            core.rename_noreplace(self.parent.fd, self.staging_name, self.output_name)
        self.armed = False
        os.fsync(self.parent.fd)


def run(arguments: list[str]) -> None:
    core.require_isolated_python()
    core.require_virtual_environment()
    if len(arguments) != 8:
        raise Failure("usage: engineering_run.py CASE-ID IMPLEMENTATION-MANIFEST PROTOCOL PLAN INPUT-BUNDLE MODEL-SOURCE FERRIC-CAPTURE-ROOT OUTPUT-ROOT")
    require_selected_case(arguments[0])
    implementation, protocol, plan_path, inputs, model_path, capture_path, output = map(Path, arguments[1:])
    measured = authenticate_artifacts(implementation, protocol)
    value, data, _ = core.read_canonical_path(plan_path, "benchmark plan")
    plan = core.parse_plan(value, data)
    if measured != (plan.identities["reference-implementation"], plan.identities["reference-protocol"]):
        raise Failure("engineering reference implementation/protocol differs from the plan")
    workloads, gpu_unique_id, coordinates = load_engineering_workloads(inputs, plan)
    selected = tuple(workload for workload in workloads if workload.case.case_id == CASE_ID)
    if len(selected) != 1:
        raise Failure("plan must contain exactly one selected engineering case")
    workload = selected[0]
    with core.SecureDirectory.open(capture_path, "engineering capture root") as captures:
        if captures.entries() != {f"{KIND}.capture.bundle"}:
            raise Failure("engineering capture root must contain only the selected case")
        transcript = load_capture(captures, plan, workload, gpu_unique_id, coordinates)
    with core.authenticate_model_source(model_path) as model_source:
        dependencies = core.load_dependencies()
        model = core.load_model(dependencies, model_source)
        logits, tokens = core.execute_workload(model, dependencies.torch, workload)
        repeated = core.execute_workload(model, dependencies.torch, workload)
        if repeated != (logits, tokens):
            raise Failure("engineering reference repeated execution was not byte-identical")
        model_source.validate()
    if authenticate_artifacts(implementation, protocol) != measured:
        raise Failure("engineering reference implementation/protocol changed during execution")
    final_value, final_data, _ = core.read_canonical_path(plan_path, "benchmark plan")
    if final_data != data or core.parse_plan(final_value, final_data) != plan:
        raise Failure("engineering reference plan changed during execution")
    bundle = core.ReferenceBundle(
        case=workload.case, logits=logits, tokens=tokens, runner=transcript.data,
        manifest=reference_manifest(plan, workload, transcript, logits, tokens),
    )
    with SelectedPublisher(output) as publisher:
        publisher.add(bundle)
        publisher.publish()
    print(f"output={output}")
    print(f"plan_sha256={plan.sha256}")
    print("status=ENGINEERING_SELECTED_REFERENCE_PUBLISHED authority=none qualification=false")


def main() -> int:
    try:
        run(sys.argv[1:])
    except (Failure, OSError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
