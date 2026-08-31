#!/usr/bin/env python3
"""Exercise canonical and hostile M1 r29 differential evidence intake."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


HERE = Path(__file__).resolve().parent
VALIDATOR = HERE / "validate-r29-differential-evidence.py"
PRODUCER = HERE.parents[1] / "m1-qualification" / "produce-r29-differential-evidence.py"


def fail(message: str) -> NoReturn:
    raise SystemExit(f"r29 differential evidence policy: {message}")


def load_validator() -> Any:
    spec = importlib.util.spec_from_file_location("ferric_m1_r29_policy", VALIDATOR)
    if spec is None or spec.loader is None:
        fail("cannot load r29 differential validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_json(path: Path, value: Any, module: Any) -> bytes:
    raw = module.canonical_bytes(value)
    path.write_bytes(raw)
    return raw


def make_sparse(path: Path, length: int) -> bytes:
    with path.open("wb") as output:
        output.truncate(length)
    return path.read_bytes()


def zero_digest(length: int) -> str:
    hasher = hashlib.sha256()
    block = bytes(1024 * 1024)
    remaining = length
    while remaining:
        chunk = block[: min(remaining, len(block))]
        hasher.update(chunk)
        remaining -= len(chunk)
    return hasher.hexdigest()


def companion(path: str, raw: bytes, module: Any) -> dict[str, Any]:
    return {"bytes": len(raw), "path": path, "sha256": module.digest_bytes(raw)}


def make_fixture(root: Path, module: Any) -> Path:
    intake_root = root / "intake"
    intake_root.mkdir(mode=0o700)
    captures = intake_root / "captures"
    references = intake_root / "references"
    comparison = intake_root / "comparison.bundle"
    raw_root = comparison / "raw"
    captures.mkdir(mode=0o700)
    references.mkdir(mode=0o700)
    comparison.mkdir(mode=0o700)
    raw_root.mkdir(mode=0o700)

    identities = {
        name: module.digest_bytes(f"plan:{name}".encode("ascii"))
        for name in module.PLAN_IDENTITIES
    }
    sources = [
        {
            "base_commit": module.digest_bytes(b"fe2o3 base")[:40],
            "commit": module.digest_bytes(b"fe2o3 commit")[:40],
            "id": "source.fe2o3",
            "repository": "fe2o3",
            "source_closure_sha256": identities["fe2o3-source-closure"],
            "tree": module.digest_bytes(b"fe2o3 tree")[:40],
        },
        {
            "base_commit": module.digest_bytes(b"ferric base")[:40],
            "commit": module.digest_bytes(b"ferric commit")[:40],
            "id": "source.ferric",
            "repository": "ferric",
            "source_closure_sha256": identities["ferric-source-closure"],
            "tree": module.digest_bytes(b"ferric tree")[:40],
        },
    ]
    tcb = [
        {
            "id": identifier,
            "identity_sha256": module.digest_bytes(identifier.encode("ascii")),
            "kind": module.TCB_KINDS[identifier],
        }
        for identifier in module.TCB_IDS
    ]
    toolchain = {
        key: module.digest_bytes(f"toolchain:{key}".encode("ascii"))
        for key in module.TOOLCHAIN_KEYS
    }
    toolchain["benchmark_executable_sha256"] = identities["benchmark-executable"]
    toolchain["benchmark_protocol_sha256"] = identities["benchmark-protocol"]
    toolchain["qualification_protocol_sha256"] = identities["benchmark-protocol"]
    toolchain["reference_implementation_sha256"] = identities[
        "reference-implementation"
    ]
    toolchain["reference_protocol_sha256"] = identities["reference-protocol"]

    policy = {
        "authority": "externally-admitted-differential-threshold-policy-only",
        "cases": [
            {
                "kind": kind,
                "maximum_logit_ulp_error": 0,
                "maximum_token_mismatches": 0,
            }
            for kind in module.CASE_KINDS
        ],
        "finite_logits_required": True,
        "format": module.POLICY_FORMAT,
        "logit_metric": "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
        "nonclaim": module.POLICY_NONCLAIM,
        "obligation_id": module.OBLIGATION_ID,
        "path_id": module.PATH_ID,
        "suite": module.SUITE,
        "target": module.TARGET,
        "token_metric": "ferric-reference-greedy-token-mismatch-count",
        "token_selection": "lowest-token-id-bf16-argmax",
    }
    policy_raw = write_json(intake_root / "acceptance-policy.json", policy, module)
    identities["differential-acceptance-policy"] = module.digest_bytes(policy_raw)
    review = {
        "authority": module.REVIEW_AUTHORITY,
        "format": module.REVIEW_FORMAT,
        "independence": "not-validated-by-ferric",
        "nonclaim": module.REVIEW_NONCLAIM,
        "organization": "external-review.example",
        "policy_sha256": module.digest_bytes(policy_raw),
        "review_identity_sha256": module.digest_bytes(b"review identity"),
        "reviewer": "reviewer.example",
        "status": "reviewed-declared",
        "target": module.TARGET,
    }
    review_raw = write_json(intake_root / "policy-review.json", review, module)
    intake = {
        "authority": module.INTAKE_AUTHORITY,
        "format": module.INTAKE_FORMAT,
        "milestone": "M1",
        "nonclaim": module.INTAKE_NONCLAIM,
        "obligation_id": module.OBLIGATION_ID,
        "path_id": module.PATH_ID,
        "policy_review_sha256": module.digest_bytes(review_raw),
        "sources": sources,
        "status": "external-input-not-independent-evidence",
        "target": module.TARGET,
        "tcb": tcb,
        "toolchain": toolchain,
    }
    write_json(intake_root / "intake.json", intake, module)
    cases = [
        {
            "id": f"{kind}.001",
            "input_sha256": module.digest_bytes(f"input:{kind}".encode("ascii")),
            "kind": kind,
            "workload_sha256": module.digest_bytes(f"workload:{kind}".encode("ascii")),
        }
        for kind in module.CASE_KINDS
    ]
    plan = {
        "authority": "benchmark-run-plan-only",
        "cases": cases,
        "format": module.PLAN_FORMAT,
        "identities": identities,
        "input_sha256": module.digest_bytes(b"benchmark input"),
        "milestone": "M1",
        "nonclaim": module.DIFFERENTIAL_NONCLAIM,
        "obligation_id": module.OBLIGATION_ID,
        "path_id": module.PATH_ID,
        "source_path": "benches/m1/differential.rs",
        "suite": module.SUITE,
        "target": module.TARGET,
    }
    plan_raw = write_json(intake_root / "plan.json", plan, module)
    plan_sha256 = module.digest_bytes(plan_raw)

    pairs: list[dict[str, Any]] = []
    case_outputs: list[dict[str, Any]] = []
    for case in cases:
        kind = case["kind"]
        rows = module.ROWS[kind]
        logits_bytes = rows * module.VOCABULARY_SIZE * 2
        token_bytes = rows * 4
        logits_sha256 = zero_digest(logits_bytes)
        tokens_sha256 = zero_digest(token_bytes)
        mode = "decode" if kind.startswith("decode-") else "prefill"
        dispatch_generation = 8_202 if mode == "decode" else 7
        execution = (
            {
                "context_plan_sha256": identities[f"dispatch-graph-{kind}"],
                "declared_workload_binding_sha256": module.digest_bytes(
                    f"workload-binding:{case['id']}".encode("ascii")
                ),
                "first_dispatch_generation": 11,
                "first_epoch": 17,
                "mode": "teacher-forced-c8192",
                "ordered_lane_bindings": [
                    {
                        "lane_identity_sha256": module.digest_bytes(
                            f"lane:{case['id']}:{lane}".encode("ascii")
                        ),
                        "lane_ordinal": lane,
                        "token_sequence_identity_sha256": module.digest_bytes(
                            f"tokens:{case['id']}:{lane}".encode("ascii")
                        ),
                    }
                    for lane in range(rows)
                ],
                "round_count": 8192,
                "round_history_sha256": module.digest_bytes(
                    f"history:{case['id']}".encode("ascii")
                ),
                "terminal_dispatch_generation": dispatch_generation,
                "terminal_epoch": 8_208,
                "terminal_ordinal": 8_191,
            }
            if mode == "decode"
            else {
                "dispatch_generation": dispatch_generation,
                "epoch": 11,
                "mode": "one-shot-prefill",
                "round_count": 1,
            }
        )
        runner = {
            "authority": "observed-target-only-qualification-capture",
            "benchmark_executable_sha256": identities["benchmark-executable"],
            "benchmark_protocol_sha256": identities["benchmark-protocol"],
            "case_id": case["id"],
            "compact_sha256": module.digest_bytes(f"compact:{kind}".encode("ascii")),
            "device_identity_sha256": module.digest_bytes(b"device identity"),
            "dispatch_generation": dispatch_generation,
            "environment_sha256": identities["environment"],
            "execution": execution,
            "format": module.CAPTURE_FORMAT,
            "gpu_unique_id": 23,
            "input_sha256": case["input_sha256"],
            "kernel_artifact_manifest_sha256": module.digest_bytes(b"kernel manifest"),
            "kind": kind,
            "logits_row_sha256": [
                zero_digest(module.VOCABULARY_SIZE * 2) for _ in range(rows)
            ],
            "logits_sha256": logits_sha256,
            "nonclaim": module.CAPTURE_NONCLAIM,
            "plan_sha256": plan_sha256,
            "program_catalog_sha256": module.digest_bytes(b"program catalog"),
            "runner_declaration_sha256": identities["generated-plan"],
            "selection": {"bucket": kind, "mode": mode, "role": "target-8b"},
            "status": "OBSERVED",
            "target": module.TARGET,
            "tokens_sha256": tokens_sha256,
            "workload_sha256": case["workload_sha256"],
        }
        runner_raw = module.canonical_bytes(runner)
        runner_sha256 = module.digest_bytes(runner_raw)
        producer_rows: dict[str, tuple[dict[str, Any], bytes]] = {}
        for producer, root_name, suffix in (
            ("ferric", captures, "capture.bundle"),
            ("reference", references, "reference.bundle"),
        ):
            bundle = root_name / f"{kind}.{suffix}"
            bundle.mkdir(mode=0o700)
            (bundle / "runner.json").write_bytes(runner_raw)
            logits = make_sparse(bundle / "logits.bf16le", logits_bytes)
            tokens = make_sparse(bundle / "tokens.u32le", token_bytes)
            producer_identity, protocol_identity = (
                ("benchmark-executable", "benchmark-protocol")
                if producer == "ferric"
                else ("reference-implementation", "reference-protocol")
            )
            manifest = {
                "authority": "externally-collected-model-output-only",
                "case_id": case["id"],
                "environment_sha256": identities["environment"],
                "format": module.OUTPUT_FORMAT,
                "input_sha256": case["input_sha256"],
                "kind": kind,
                "logits": {
                    "bytes": len(logits),
                    "encoding": "bf16-le",
                    "path": "logits.bf16le",
                    "sha256": module.digest_bytes(logits),
                },
                "plan_sha256": plan_sha256,
                "producer": producer,
                "producer_sha256": identities[producer_identity],
                "protocol_sha256": identities[protocol_identity],
                "runner_transcript_sha256": runner_sha256,
                "shape": {"rows": rows, "vocabulary_size": module.VOCABULARY_SIZE},
                "tokens": {
                    "bytes": len(tokens),
                    "encoding": "u32-le",
                    "path": "tokens.u32le",
                    "sha256": module.digest_bytes(tokens),
                },
                "workload_sha256": case["workload_sha256"],
            }
            manifest_raw = write_json(bundle / "output.json", manifest, module)
            producer_rows[producer] = (manifest, manifest_raw)
        capture_path = f"captures/{kind}.capture.bundle/output.json"
        reference_path = f"references/{kind}.reference.bundle/output.json"
        runner_path = f"captures/{kind}.capture.bundle/runner.json"
        pairs.append(
            {
                "case_id": case["id"],
                "ferric_output_manifest": companion(
                    capture_path, producer_rows["ferric"][1], module
                ),
                "kind": kind,
                "reference_output_manifest": companion(
                    reference_path, producer_rows["reference"][1], module
                ),
                "runner_transcript": companion(runner_path, runner_raw, module),
            }
        )
        case_outputs.append(
            {
                "case": case,
                "comparison": {
                    "compared_logits": module.ROWS[kind] * module.VOCABULARY_SIZE,
                    "compared_tokens": module.ROWS[kind],
                    "maximum_logit_ulp_error": 0,
                    "token_mismatches": 0,
                },
                "ferric": module.output_record(*producer_rows["ferric"]),
                "reference": module.output_record(*producer_rows["reference"]),
                "runner_sha256": runner_sha256,
            }
        )
    pairs_document = {
        "authority": "externally-collected-differential-pairs-only",
        "format": module.PAIRS_FORMAT,
        "pairs": pairs,
        "plan_sha256": plan_sha256,
        "suite": module.SUITE,
    }
    pairs_raw = write_json(intake_root / "pairs.json", pairs_document, module)
    pairs_sha256 = module.digest_bytes(pairs_raw)

    acceptance_cases: list[dict[str, Any]] = []
    observations: list[dict[str, Any]] = []
    for row in case_outputs:
        case = row["case"]
        kind = case["kind"]
        acceptance_case = {
            "case_id": case["id"],
            "comparison": row["comparison"],
            "ferric_output": row["ferric"],
            "kind": kind,
            "reference_output": row["reference"],
            "runner_transcript_sha256": row["runner_sha256"],
            "status": "within-policy",
            "threshold": {
                "maximum_logit_ulp_error": 0,
                "maximum_token_mismatches": 0,
            },
        }
        acceptance_cases.append(acceptance_case)
        raw_record = {
            "authority": "computed-differential-comparison-only",
            "case_id": case["id"],
            "comparison": row["comparison"],
            "ferric_output": row["ferric"],
            "format": module.RAW_FORMAT,
            "kind": kind,
            "nonclaim": module.DIFFERENTIAL_NONCLAIM,
            "pairs_sha256": pairs_sha256,
            "plan_sha256": plan_sha256,
            "reference_output": row["reference"],
            "runner_transcript_sha256": row["runner_sha256"],
            "shape": {
                "rows": module.ROWS[kind],
                "vocabulary_size": module.VOCABULARY_SIZE,
            },
            "status": "compared",
        }
        raw_bytes = write_json(
            raw_root / f"{case['id']}.differential.raw.json", raw_record, module
        )
        observations.append(
            {
                "attributes": {
                    "ferric-output-sha256": row["ferric"]["manifest_sha256"],
                    "raw-record-sha256": module.digest_bytes(raw_bytes),
                    "reference-output-sha256": row["reference"]["manifest_sha256"],
                    "runner-transcript-sha256": row["runner_sha256"],
                },
                "case_id": case["id"],
                "kind": kind,
                "measurements": {
                    "compared-logits": [row["comparison"]["compared_logits"]],
                    "compared-tokens": [row["comparison"]["compared_tokens"]],
                    "maximum-logit-ulp-error": [0],
                    "token-mismatches": [0],
                },
                "recorded_samples": 1,
                "status": "completed",
                "warmups": 0,
            }
        )
    write_json(
        comparison / "records.json",
        {
            "format": module.RECORDS_FORMAT,
            "observations": observations,
            "plan_sha256": plan_sha256,
            "suite": module.SUITE,
        },
        module,
    )
    write_json(
        intake_root / "acceptance.json",
        {
            "authority": "checked-differential-policy-conformance-only",
            "cases": acceptance_cases,
            "format": module.ACCEPTANCE_FORMAT,
            "nonclaim": module.ACCEPTANCE_NONCLAIM,
            "obligation_id": module.OBLIGATION_ID,
            "pairs_sha256": pairs_sha256,
            "path_id": module.PATH_ID,
            "plan_sha256": plan_sha256,
            "policy_sha256": module.digest_bytes(policy_raw),
            "status": "POLICY_CONFORMING",
            "suite": module.SUITE,
            "target": module.TARGET,
        },
        module,
    )
    return intake_root


def reject(operation: Callable[[], None], description: str) -> None:
    try:
        operation()
    except SystemExit as error:
        if error.code in (None, 0):
            fail(f"hostile case exited successfully: {description}")
        return
    fail(f"hostile r29 intake was accepted: {description}")


def run_policy() -> None:
    module = load_validator()
    with tempfile.TemporaryDirectory(prefix="ferric-m1-r29-evidence-") as temporary:
        root = Path(temporary)
        root.chmod(0o700)
        intake = make_fixture(root, module)
        roster, report = module.build_documents(intake)
        roster_value = module.parse_canonical(roster, "fixture roster")
        report_value = module.parse_canonical(report, "fixture report")
        if (
            len(roster_value["cases"]) != 7
            or report_value["status"] != "partial-non-evidence"
            or report_value["qualification_evidence"] is not False
            or report_value["independent_validation"] is not False
            or report_value["r29_closed"] is not False
        ):
            fail("canonical report promoted its authority")

        plan_value = json.loads((intake / "plan.json").read_bytes())
        plan_cases, plan_identities = module.validate_plan(plan_value)
        plan_sha256 = module.digest_bytes((intake / "plan.json").read_bytes())

        def runner_case(kind: str) -> tuple[Path, dict[str, Any], dict[str, Any]]:
            case = next(row for row in plan_cases if row["kind"] == kind)
            path = intake / "captures" / f"{kind}.capture.bundle" / "runner.json"
            return path, json.loads(path.read_bytes()), case

        runner_mutations: list[tuple[str, Callable[[dict[str, Any]], None], str]] = [
            (
                "decode-s1-c8192",
                lambda value: value["selection"].__setitem__("role", "draft-1b"),
                "runner selection role",
            ),
            (
                "prefill-s1-t128",
                lambda value: value["execution"].__setitem__(
                    "dispatch_generation", value["dispatch_generation"] + 1
                ),
                "prefill execution generation",
            ),
            (
                "decode-s1-c8192",
                lambda value: value["execution"].__setitem__(
                    "terminal_dispatch_generation", value["dispatch_generation"] + 1
                ),
                "decode terminal generation",
            ),
            (
                "decode-s1-c8192",
                lambda value: value["execution"].__setitem__("terminal_ordinal", 8190),
                "decode terminal ordinal",
            ),
            (
                "decode-s8-c8192",
                lambda value: value["execution"]["ordered_lane_bindings"].pop(),
                "decode lane roster",
            ),
            (
                "decode-s8-c8192",
                lambda value: value["execution"]["ordered_lane_bindings"][
                    0
                ].__setitem__("lane_ordinal", 1),
                "decode lane ordinal",
            ),
            (
                "prefill-s1-t128",
                lambda value: value.__setitem__(
                    "dispatch_generation", module.MAX_U64 + 1
                ),
                "runner u64 overflow",
            ),
            (
                "prefill-s1-t128",
                lambda value: value.__setitem__(
                    "runner_declaration_sha256",
                    module.digest_bytes(b"substituted generated plan"),
                ),
                "runner generated-plan identity",
            ),
            (
                "decode-s1-c8192",
                lambda value: value["execution"].__setitem__(
                    "context_plan_sha256",
                    module.digest_bytes(b"substituted context plan"),
                ),
                "runner decode context-plan identity",
            ),
        ]
        for kind, mutation, description in runner_mutations:
            _, runner_value, case = runner_case(kind)
            mutation(runner_value)
            reject(
                lambda value=runner_value, row=case: module.validate_runner(
                    value, row, plan_identities, plan_sha256
                ),
                description,
            )

        row_kind = "decode-s1-c8192"
        _, row_runner, row_case = runner_case(row_kind)
        ferric_bundle = intake / "captures" / f"{row_kind}.capture.bundle"
        ferric_manifest = json.loads((ferric_bundle / "output.json").read_bytes())
        logits = (ferric_bundle / "logits.bf16le").read_bytes()
        row_bytes = module.VOCABULARY_SIZE * 2
        row_identities = [
            module.digest_bytes(logits[offset : offset + row_bytes])
            for offset in range(0, len(logits), row_bytes)
        ]
        row_runner["logits_row_sha256"][0] = module.digest_bytes(
            b"substituted logit row"
        )
        reject(
            lambda: module.validate_runner_payload_bindings(
                row_runner, ferric_manifest, row_identities, row_case
            ),
            "runner logit-row identity",
        )

        output = root / "output"
        subprocess.run(
            [sys.executable, "-I", str(PRODUCER), str(intake), str(output)],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env={
                "PATH": os.environ.get("PATH", ""),
                "PYTHONDONTWRITEBYTECODE": "1",
            },
        )
        module.validate(intake, output)
        subprocess.run(
            [
                sys.executable,
                "-I",
                "-B",
                str(VALIDATOR),
                "validate",
                str(intake),
                str(output),
            ],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env={"PATH": os.environ.get("PATH", "")},
        )
        reject(lambda: module.produce(intake, output), "replace existing output")

        intake_link = root / "intake-link"
        intake_link.symlink_to(intake, target_is_directory=True)
        reject(lambda: module.build_documents(intake_link), "symlinked intake root")

        original_read_json = module.SecureRoot.read_json
        original_intake = (intake / "intake.json").read_bytes()
        file_race_triggered = False

        def mutate_previously_read_file(
            held_root: Any, name: str, description: str
        ) -> tuple[Any, bytes]:
            nonlocal file_race_triggered
            result = original_read_json(held_root, name, description)
            if name == "plan.json" and not file_race_triggered:
                file_race_triggered = True
                value = json.loads(original_intake)
                value["toolchain"]["compiler_sha256"] = module.digest_bytes(
                    b"raced compiler"
                )
                (intake / "intake.json").write_bytes(module.canonical_bytes(value))
            return result

        module.SecureRoot.read_json = mutate_previously_read_file
        try:
            reject(
                lambda: module.build_documents(intake),
                "previously read input mutation",
            )
        finally:
            module.SecureRoot.read_json = original_read_json
            (intake / "intake.json").write_bytes(original_intake)

        capture_bundle = intake / "captures" / "decode-s1-c8192.capture.bundle"
        capture_aside = intake / "captures" / ".decode-s1-c8192.capture.aside"
        directory_race_triggered = False

        def replace_previously_opened_bundle(
            held_root: Any, name: str, description: str
        ) -> tuple[Any, bytes]:
            nonlocal directory_race_triggered
            result = original_read_json(held_root, name, description)
            if (
                name == "references/decode-s1-c8192.reference.bundle/runner.json"
                and not directory_race_triggered
            ):
                directory_race_triggered = True
                capture_bundle.rename(capture_aside)
                capture_bundle.symlink_to(capture_aside.name, target_is_directory=True)
            return result

        module.SecureRoot.read_json = replace_previously_opened_bundle
        try:
            reject(
                lambda: module.build_documents(intake),
                "previously opened bundle substitution",
            )
        finally:
            module.SecureRoot.read_json = original_read_json
            if capture_bundle.is_symlink():
                capture_bundle.unlink()
            if capture_aside.exists():
                capture_aside.rename(capture_bundle)

        real_rename = module._rename_noreplace
        pre_race_output = root / "pre-race-output"

        def substitute_staging(parent_fd: int, source: str, destination: str) -> None:
            os.rename(
                source,
                f"{source}.aside",
                src_dir_fd=parent_fd,
                dst_dir_fd=parent_fd,
            )
            os.mkdir(source, 0o700, dir_fd=parent_fd)
            real_rename(parent_fd, source, destination)

        module._rename_noreplace = substitute_staging
        try:
            reject(
                lambda: module.produce(intake, pre_race_output),
                "staging-name substitution during publication",
            )
            if not pre_race_output.is_dir():
                fail("publication race removed the substituted output")
        finally:
            module._rename_noreplace = real_rename

        file_race_output = root / "file-race-output"

        def substitute_staged_file(
            parent_fd: int, source: str, destination: str
        ) -> None:
            source_fd = os.open(
                source,
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                dir_fd=parent_fd,
            )
            try:
                original_fd = os.open(
                    "report.json",
                    os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC,
                    dir_fd=source_fd,
                )
                try:
                    size = os.fstat(original_fd).st_size
                    original = module.SecureRoot._read_fd(
                        original_fd, size, "staged report race fixture"
                    )
                finally:
                    os.close(original_fd)
                os.unlink("report.json", dir_fd=source_fd)
                replacement_fd = os.open(
                    "report.json",
                    os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                    0o600,
                    dir_fd=source_fd,
                )
                try:
                    offset = 0
                    while offset != len(original):
                        offset += os.write(replacement_fd, original[offset:])
                    os.fsync(replacement_fd)
                finally:
                    os.close(replacement_fd)
                os.fsync(source_fd)
            finally:
                os.close(source_fd)
            real_rename(parent_fd, source, destination)

        module._rename_noreplace = substitute_staged_file
        try:
            reject(
                lambda: module.produce(intake, file_race_output),
                "identical-byte staged-file substitution during publication",
            )
            if not file_race_output.is_dir():
                fail("staged-file race removed the published output")
        finally:
            module._rename_noreplace = real_rename

        post_race_output = root / "post-race-output"

        def substitute_published(parent_fd: int, source: str, destination: str) -> None:
            real_rename(parent_fd, source, destination)
            os.rename(
                destination,
                f"{destination}.aside",
                src_dir_fd=parent_fd,
                dst_dir_fd=parent_fd,
            )
            os.mkdir(destination, 0o700, dir_fd=parent_fd)

        module._rename_noreplace = substitute_published
        try:
            reject(
                lambda: module.produce(intake, post_race_output),
                "published-name substitution before return",
            )
            if not post_race_output.is_dir():
                fail("post-rename race removed the substituted output")
        finally:
            module._rename_noreplace = real_rename

        mutations: list[tuple[Path, Callable[[dict[str, Any]], None], str]] = [
            (
                intake / "intake.json",
                lambda value: value["sources"][1].__setitem__(
                    "source_closure_sha256", module.digest_bytes(b"wrong source")
                ),
                "Ferric source closure",
            ),
            (
                intake / "plan.json",
                lambda value: value.__setitem__("target", "gfx942:xnack+"),
                "plan target",
            ),
            (
                intake / "acceptance-policy.json",
                lambda value: value["cases"][0].__setitem__(
                    "maximum_token_mismatches", 2
                ),
                "policy threshold",
            ),
            (
                intake / "policy-review.json",
                lambda value: value.__setitem__(
                    "independence", "independently-validated"
                ),
                "review independence promotion",
            ),
            (
                intake / "pairs.json",
                lambda value: value.__setitem__(
                    "plan_sha256", module.digest_bytes(b"wrong plan")
                ),
                "pairs plan identity",
            ),
            (
                intake / "acceptance.json",
                lambda value: value.__setitem__("status", "QUALIFIED"),
                "acceptance status promotion",
            ),
            (
                intake / "acceptance.json",
                lambda value: value["cases"][0]["comparison"].__setitem__(
                    "token_mismatches", 1
                ),
                "acceptance arithmetic",
            ),
            (
                intake / "comparison.bundle" / "records.json",
                lambda value: value["observations"][0].__setitem__("status", "failed"),
                "comparison observation",
            ),
        ]
        for path, mutation, description in mutations:
            original = path.read_bytes()
            value = json.loads(original)
            mutation(value)
            path.write_bytes(module.canonical_bytes(value))
            try:
                reject(lambda: module.build_documents(intake), description)
            finally:
                path.write_bytes(original)

        intake_path = intake / "intake.json"
        original_intake = intake_path.read_bytes()
        for description, mutation in (
            (
                "compiler identity replay",
                lambda value: value["toolchain"].__setitem__(
                    "compiler_sha256", module.digest_bytes(b"wrong compiler")
                ),
            ),
            (
                "TCB identity replay",
                lambda value: value["tcb"][0].__setitem__(
                    "identity_sha256", module.digest_bytes(b"wrong TCB")
                ),
            ),
        ):
            value = json.loads(original_intake)
            mutation(value)
            intake_path.write_bytes(module.canonical_bytes(value))
            try:
                reject(lambda: module.validate(intake, output), description)
            finally:
                intake_path.write_bytes(original_intake)

        payload = (
            intake / "captures" / "decode-s1-c8192.capture.bundle" / "logits.bf16le"
        )
        with payload.open("r+b") as output_payload:
            output_payload.write(b"\x01")
        try:
            reject(lambda: module.build_documents(intake), "Ferric capture payload")
        finally:
            with payload.open("r+b") as output_payload:
                output_payload.write(b"\x00")

        raw_path = (
            intake
            / "comparison.bundle"
            / "raw"
            / "decode-s1-c8192.001.differential.raw.json"
        )
        original_raw = raw_path.read_bytes()
        raw_value = json.loads(original_raw)
        raw_value["comparison"]["maximum_logit_ulp_error"] = 1
        raw_path.write_bytes(module.canonical_bytes(raw_value))
        try:
            reject(lambda: module.build_documents(intake), "raw comparison")
        finally:
            raw_path.write_bytes(original_raw)

        report_path = output / "report.json"
        original_report = report_path.read_bytes()
        for field in ("independent_validation", "qualification_evidence", "r29_closed"):
            promoted = json.loads(original_report)
            promoted[field] = True
            report_path.write_bytes(module.canonical_bytes(promoted))
            try:
                reject(
                    lambda: module.validate(intake, output),
                    f"published {field} promotion",
                )
            finally:
                report_path.write_bytes(original_report)

    print("PASS: r29 differential evidence policy (canonical plus 32 hostile cases)")


if __name__ == "__main__":
    run_policy()
