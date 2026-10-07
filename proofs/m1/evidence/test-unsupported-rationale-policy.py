#!/usr/bin/env python3
"""Exercise canonical and hostile M1 Unsupported-rationale artifacts."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


PROTOCOL = "ferric.m1-validator.unsupported-rationale.v1"
ARTIFACT_FORMAT = "FERRIC-M1-UNSUPPORTED-RATIONALE-V1"
AUTHORITY = "nonclaim-only"
NONCLAIM = (
    "This artifact grants no theorem, validation, artifact, load, launch, "
    "hardware, performance, or qualification authority."
)
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = ("Compiler", "Hardware", "Runtime")
RATIONALES: dict[str, dict[str, Any]] = {
    "distribution_preserved": {
        "reason_code": "outside-m1-deterministic-greedy-scope",
        "rationale": (
            "Stochastic sampling and stochastic speculative distribution "
            "preservation are outside the deterministic greedy M1 envelope."
        ),
        "excluded_claims": [
            "stochastic-sampling",
            "stochastic-speculative-distribution-preservation",
        ],
        "path_id": "speculation-proof",
    },
    "machine_refined": {
        "reason_code": "unresolved-machine-correspondence",
        "rationale": (
            "The five independent translation validators required by the "
            "assurance policy do not exist; source proofs do not establish "
            "machine semantics."
        ),
        "excluded_claims": [
            "mir-to-structured-algorithm-correspondence",
            "algorithm-to-schedule-refinement",
            "gpu-subset-to-llvm-correspondence",
            "llvm-optimization-validation",
            "object-to-amdgpu-isa-correspondence",
        ],
        "path_id": "identity-closure",
    },
    "multi_device_refined": {
        "reason_code": "outside-m1-single-device-scope",
        "rationale": (
            "M1 admits exactly one physical gfx942 device and makes no "
            "collective or multi-device refinement claim."
        ),
        "excluded_claims": [
            "collective-execution",
            "multi-device-execution",
            "multi-device-refinement",
        ],
        "path_id": "m1-tcb",
    },
}
PATHS = {
    "identity-closure": "crates/ferric-build/src/identity_closure.rs",
    "m1-tcb": "docs/M1_TCB.md",
    "speculation-proof": "proofs/m1/speculative_graph.rs",
}
Mutation = Callable[[Path, dict[str, Any], dict[str, Any]], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_digest(value: Any) -> str:
    return digest_bytes(
        json.dumps(
            value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
    )


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def refresh_binding(context: dict[str, Any]) -> None:
    binding = context["binding"]
    binding["binding_sha256"] = canonical_digest(
        {key: value for key, value in binding.items() if key != "binding_sha256"}
    )


def refresh_artifact(
    path: Path, context: dict[str, Any], artifact: dict[str, Any]
) -> None:
    data = canonical_bytes(artifact)
    path.write_bytes(data)
    context["artifact"]["sha256"] = digest_bytes(data)
    context["artifact"]["size_bytes"] = len(data)


def make_fixture(
    root: Path, obligation_id: str = "distribution_preserved"
) -> tuple[Path, dict[str, Any], dict[str, Any]]:
    expected = RATIONALES[obligation_id]
    artifact_path = root / f"{obligation_id}.json"
    sources = [
        {
            "base_commit": "1" * 40,
            "commit": "2" * 40,
            "id": "source.fe2o3",
            "repository": "fe2o3",
            "source_closure_artifact_id": "artifact.source.fe2o3",
            "source_closure_sha256": "3" * 64,
            "tree": "4" * 40,
        },
        {
            "base_commit": "5" * 40,
            "commit": "6" * 40,
            "id": "source.ferric",
            "repository": "ferric",
            "source_closure_artifact_id": "artifact.source.ferric",
            "source_closure_sha256": "7" * 64,
            "tree": "8" * 40,
        },
    ]
    tcb = [
        {
            "artifact_id": f"artifact.{identifier}",
            "id": identifier,
            "identity_sha256": str(offset) * 64,
            "kind": kind,
        }
        for offset, (identifier, kind) in enumerate(
            zip(TCB_IDS, TCB_KINDS, strict=True), start=1
        )
    ]
    statement_sha256 = digest_bytes(expected["rationale"].encode("utf-8"))
    binding = {
        "artifact_id": f"artifact.rationale.{obligation_id}",
        "binding_sha256": "",
        "evidence_kind": "unsupported-rationale",
        "id": f"binding.rationale.{obligation_id}",
        "obligation_class": "Assurance",
        "obligation_id": obligation_id,
        "path_id": expected["path_id"],
        "profile_id": "nonclaim",
        "source_identity_id": "source.ferric",
        "statement_sha256": statement_sha256,
        "tcb_ids": list(TCB_IDS),
    }
    context = {
        "artifact": {
            "id": binding["artifact_id"],
            "kind": "UnsupportedRationale",
            "path": f"artifacts/{artifact_path.name}",
            "sha256": "",
            "size_bytes": 0,
        },
        "artifact_absolute_path": str(artifact_path),
        "binding": binding,
        "format": "ferric.m1-evidence-index.v1",
        "path_resolution": {
            "availability": "RequiredFuture",
            "id": expected["path_id"],
            "path": PATHS[expected["path_id"]],
            "repository": "ferric",
            "source_identity_id": "source.ferric",
        },
        "requirements_sha256": "9" * 64,
        "sources": sources,
        "subject": f"binding:{binding['id']}",
        "tcb": tcb,
    }
    refresh_binding(context)
    artifact = {
        "authority": AUTHORITY,
        "binding_sha256": binding["binding_sha256"],
        "excluded_claims": copy.deepcopy(expected["excluded_claims"]),
        "format": ARTIFACT_FORMAT,
        "nonclaim": NONCLAIM,
        "obligation_class": "Assurance",
        "obligation_id": obligation_id,
        "path_id": binding["path_id"],
        "rationale": expected["rationale"],
        "reason_code": expected["reason_code"],
        "required_closure_status": "Unsupported",
        "requirements_sha256": context["requirements_sha256"],
        "source_identity_id": "source.ferric",
        "source_roster_sha256": canonical_digest(sources),
        "statement_sha256": statement_sha256,
        "tcb_identity_sha256s": {
            record["id"]: record["identity_sha256"] for record in tcb
        },
        "tcb_roster_sha256": canonical_digest(tcb),
    }
    refresh_artifact(artifact_path, context, artifact)
    return artifact_path, context, artifact


def invoke(
    validator: Path,
    context: dict[str, Any],
    *,
    protocol: str = PROTOCOL,
    raw_context: bytes | None = None,
) -> subprocess.CompletedProcess[str]:
    if raw_context is None:
        raw_context = (
            json.dumps(
                context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
            )
            + "\n"
        ).encode("ascii")
    return subprocess.run(
        [sys.executable, "-I", str(validator), protocol],
        check=False,
        input=raw_context,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=False,
        timeout=10,
    )


def canonical_cases(validator: Path, root: Path) -> None:
    for obligation_id in RATIONALES:
        _, context, _ = make_fixture(root, obligation_id)
        result = invoke(validator, context)
        payload = json.dumps(
            context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
        expected = (
            f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
            f"context_sha256={digest_bytes(payload)}\n"
        ).encode("ascii")
        if result.returncode != 0 or result.stdout != expected:
            fail(
                f"canonical {obligation_id} rationale rejected: "
                f"exit={result.returncode}, output={result.stdout!r}"
            )


def hostile_cases(validator: Path, root: Path) -> None:
    def artifact_field(key: str, value: Any) -> Mutation:
        def mutate(
            path: Path, context: dict[str, Any], artifact: dict[str, Any]
        ) -> None:
            artifact[key] = value
            refresh_artifact(path, context, artifact)

        return mutate

    def binding_field(key: str, value: Any) -> Mutation:
        def mutate(
            path: Path, context: dict[str, Any], artifact: dict[str, Any]
        ) -> None:
            context["binding"][key] = value
            refresh_binding(context)
            artifact["binding_sha256"] = context["binding"]["binding_sha256"]
            refresh_artifact(path, context, artifact)

        return mutate

    mutations: list[tuple[str, Mutation]] = [
        ("promote-status", artifact_field("required_closure_status", "Proved")),
        ("grant-authority", artifact_field("authority", "proof-authority")),
        ("weaken-nonclaim", artifact_field("nonclaim", "No machine claim.")),
        ("reason-drift", artifact_field("reason_code", "temporary-gap")),
        ("rationale-drift", artifact_field("rationale", "Not implemented.")),
        ("excluded-claim-omission", artifact_field("excluded_claims", [])),
        ("artifact-format", artifact_field("format", "FERRIC-M1-UNSUPPORTED-V2")),
        ("artifact-requirements", artifact_field("requirements_sha256", "a" * 64)),
        ("artifact-statement", artifact_field("statement_sha256", "b" * 64)),
        ("artifact-source-roster", artifact_field("source_roster_sha256", "c" * 64)),
        ("artifact-tcb-roster", artifact_field("tcb_roster_sha256", "d" * 64)),
        (
            "artifact-tcb-identity",
            artifact_field(
                "tcb_identity_sha256s",
                {
                    "tcb.compiler": "1" * 64,
                    "tcb.hardware": "2" * 64,
                    "tcb.runtime": "f" * 64,
                },
            ),
        ),
        ("unknown-property", binding_field("obligation_id", "operator_refined")),
        ("binding-class", binding_field("obligation_class", "Roadmap")),
        ("binding-kind", binding_field("evidence_kind", "verus-theorem")),
        ("binding-profile", binding_field("profile_id", "proof")),
        ("binding-source", binding_field("source_identity_id", "source.fe2o3")),
        ("binding-path", binding_field("path_id", "kernel-proof")),
        ("binding-statement", binding_field("statement_sha256", "e" * 64)),
        ("binding-tcb-order", binding_field("tcb_ids", list(reversed(TCB_IDS)))),
    ]

    for name, mutation in mutations:
        case_root = root / name
        case_root.mkdir()
        path, context, artifact = make_fixture(case_root)
        mutation(path, context, artifact)
        result = invoke(validator, context)
        if result.returncode == 0:
            fail(f"hostile rationale was accepted: {name}")

    direct_cases: list[
        tuple[str, Callable[[Path, dict[str, Any], dict[str, Any]], None]]
    ] = [
        (
            "artifact-kind",
            lambda _path, context, _artifact: context["artifact"].__setitem__(
                "kind", "TheoremTranscript"
            ),
        ),
        (
            "artifact-digest",
            lambda _path, context, _artifact: context["artifact"].__setitem__(
                "sha256", "f" * 64
            ),
        ),
        (
            "subject-replay",
            lambda _path, context, _artifact: context.__setitem__(
                "subject", "binding:rationale.other"
            ),
        ),
        (
            "path-resolution",
            lambda _path, context, _artifact: context["path_resolution"].__setitem__(
                "id", "m1-tcb"
            ),
        ),
        (
            "path-substitution",
            lambda _path, context, _artifact: context["path_resolution"].__setitem__(
                "path", "docs/ASSURANCE.md"
            ),
        ),
        (
            "source-order",
            lambda _path, context, _artifact: context["sources"].reverse(),
        ),
        ("tcb-order", lambda _path, context, _artifact: context["tcb"].reverse()),
    ]
    for name, mutation in direct_cases:
        case_root = root / name
        case_root.mkdir()
        path, context, artifact = make_fixture(case_root)
        mutation(path, context, artifact)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile rationale context was accepted: {name}")

    case_root = root / "symlink-artifact"
    case_root.mkdir()
    path, context, _ = make_fixture(case_root)
    target = case_root / "target.json"
    path.rename(target)
    path.symlink_to(target.name)
    if invoke(validator, context).returncode == 0:
        fail("symlink rationale artifact was accepted")

    case_root = root / "noncanonical-artifact"
    case_root.mkdir()
    path, context, artifact = make_fixture(case_root)
    data = (json.dumps(artifact, sort_keys=True) + "\n").encode("ascii")
    path.write_bytes(data)
    context["artifact"]["sha256"] = digest_bytes(data)
    context["artifact"]["size_bytes"] = len(data)
    if invoke(validator, context).returncode == 0:
        fail("noncanonical rationale artifact was accepted")

    case_root = root / "extra-artifact-field"
    case_root.mkdir()
    path, context, artifact = make_fixture(case_root)
    artifact["proof"] = True
    refresh_artifact(path, context, artifact)
    if invoke(validator, context).returncode == 0:
        fail("rationale artifact with extra authority field was accepted")

    raw_root = root / "raw-context"
    raw_root.mkdir()
    _, context, _ = make_fixture(raw_root)
    raw = json.dumps(context, ensure_ascii=True, sort_keys=True).encode("ascii") + b"\n"
    if invoke(validator, context, raw_context=raw).returncode == 0:
        fail("noncanonical validator context was accepted")
    duplicate = raw.replace(b'{"artifact":', b'{"format":"drift","artifact":', 1)
    if invoke(validator, context, raw_context=duplicate).returncode == 0:
        fail("duplicate-key validator context was accepted")
    if invoke(validator, context, protocol=PROTOCOL + ".drift").returncode == 0:
        fail("wrong validator protocol was accepted")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: test-unsupported-rationale-policy.py FERRIC_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    validator = repo / "proofs/m1/evidence/validate-unsupported-rationale.py"
    with tempfile.TemporaryDirectory(prefix="ferric-m1-rationale-policy.") as raw:
        root = Path(raw)
        canonical_cases(validator, root)
        hostile_root = root / "hostile"
        hostile_root.mkdir()
        hostile_cases(validator, hostile_root)
    print(
        "PASS: M1 unsupported-rationale validator accepted 3 canonical "
        "nonclaims and rejected 33 hostile fixtures"
    )


if __name__ == "__main__":
    main()
