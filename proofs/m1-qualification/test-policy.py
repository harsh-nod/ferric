#!/usr/bin/env python3
"""Exercise the deterministic M1 evidence planner and its refusal boundary."""

from __future__ import annotations

import copy
import contextlib
import importlib.util
import io
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path
from typing import Any, NoReturn

sys.dont_write_bytecode = True


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_planner(path: Path) -> Any:
    spec = importlib.util.spec_from_file_location("ferric_m1_evidence_planner", path)
    if spec is None or spec.loader is None:
        fail("cannot load M1 evidence planner")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def command(arguments: list[str], description: str, *, cwd: Path | None = None) -> str:
    result = subprocess.run(
        arguments,
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"{description} failed (status {result.returncode}):\n{result.stdout}")
    return result.stdout.strip()


def git(repo: Path, arguments: list[str], description: str) -> str:
    return command(["git", "-C", str(repo), *arguments], description)


def clone_at(source: Path, destination: Path, commit: str | None = None) -> None:
    command(
        ["git", "clone", "--quiet", "--shared", str(source), str(destination)],
        f"clone integration fixture {source}",
    )
    if commit is not None:
        git(
            destination,
            ["checkout", "--quiet", "--detach", commit],
            f"checkout integration fixture {commit}",
        )


def commit_fixture(repo: Path, message: str) -> None:
    git(repo, ["add", "--all"], f"stage integration fixture {message}")
    git(
        repo,
        [
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "user.name=Ferric M1 Policy",
            "-c",
            "user.email=ferric-m1-policy.invalid",
            "commit",
            "--quiet",
            "--no-gpg-sign",
            "-m",
            message,
        ],
        f"commit integration fixture {message}",
    )


def run_planner(planner_path: Path, ferric: Path, fe2o3: Path, output: Path) -> Any:
    return subprocess.run(
        [sys.executable, "-I", str(planner_path), str(ferric), str(fe2o3), str(output)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={"PATH": os.environ.get("PATH", "")},
    )


def expect_prepare_failure(
    planner_path: Path,
    ferric: Path,
    fe2o3: Path,
    output: Path,
    expected: str,
) -> None:
    result = run_planner(planner_path, ferric, fe2o3, output)
    if result.returncode == 0 or expected not in result.stdout:
        fail(
            f"M1 planner accepted hostile integration fixture; expected {expected!r}:\n"
            f"{result.stdout}"
        )
    if output.is_dir() and (output / "plan.json").exists():
        fail("M1 planner published a plan after a hostile integration failure")


def replace_once(path: Path, before: str, after: str) -> None:
    source = path.read_text(encoding="utf-8")
    if source.count(before) != 1:
        fail(f"integration mutation has no unique anchor: {path}: {before!r}")
    path.write_text(source.replace(before, after, 1), encoding="utf-8")


def exercise_prepare_boundaries(repo: Path, fe2o3_source: Path, planner: Any) -> None:
    with tempfile.TemporaryDirectory(prefix="ferric-m1-planner-policy-") as raw:
        temporary = Path(raw)
        ferric_fixture = temporary / "ferric"
        clone_at(repo, ferric_fixture)
        shutil.copytree(
            repo / "proofs/m1-qualification",
            ferric_fixture / "proofs/m1-qualification",
            dirs_exist_ok=True,
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )
        if git(
            ferric_fixture,
            ["status", "--porcelain=v1", "--untracked-files=all"],
            "inspect M1 planner fixture",
        ):
            commit_fixture(ferric_fixture, "add M1 external evidence planner")

        workspace = tomllib.loads(
            (ferric_fixture / "Cargo.toml").read_text(encoding="utf-8")
        )
        fe2o3_commit = workspace["workspace"]["dependencies"]["fe2o3-amdhsa-loader"][
            "rev"
        ]
        if not isinstance(fe2o3_commit, str):
            fail("integration fixture has no exact fe2o3 revision")
        git(
            fe2o3_source,
            ["cat-file", "-e", f"{fe2o3_commit}^{{commit}}"],
            "locate pinned fe2o3 integration commit",
        )
        fe2o3_fixture = temporary / "fe2o3"
        clone_at(fe2o3_source, fe2o3_fixture, fe2o3_commit)
        fixture_planner = ferric_fixture / "proofs/m1-qualification/planner.py"

        output = temporary / "positive-output"
        positive = run_planner(fixture_planner, ferric_fixture, fe2o3_fixture, output)
        if (
            positive.returncode != 0
            or "PASS: prepared external M1 evidence plan" not in (positive.stdout)
        ):
            fail(f"M1 planner rejected exact integration fixture:\n{positive.stdout}")
        plan = planner.read_canonical_json(output / "plan.json", "integration plan")
        queue = planner.read_canonical_json(
            output / "missing-work.json", "integration work queue"
        )
        if (
            plan.get("format") != planner.PLAN_FORMAT
            or plan.get("authority") != planner.AUTHORITY
            or queue.get("format") != planner.WORK_FORMAT
            or queue.get("status") != "INCOMPLETE"
            or queue.get("counts", {}).get("missing_items") != 358
            or queue.get("counts", {}).get("available_producer_items") != 277
            or queue.get("counts", {}).get("missing_producer_items") != 81
        ):
            fail("M1 planner positive integration output weakened its nonclaim")
        if any(
            (output / name).exists() for name in ("evidence-index.json", "receipt.json")
        ):
            fail(
                "M1 planner positive integration output contains forbidden closure output"
            )
        closures = {
            record["artifact"]["id"]: record["artifact"]
            for record in plan["source_closures"]
        }
        for identifier in ("artifact.source.fe2o3", "artifact.source.ferric"):
            artifact = closures[identifier]
            path = output / artifact["path"]
            if (
                planner.digest_file(path) != artifact["sha256"]
                or path.stat().st_size != artifact["size_bytes"]
            ):
                fail(f"M1 planner source closure identity drifted: {identifier}")
        plan_time = (output / "plan.json").stat().st_mtime_ns
        for path in output.rglob("*"):
            mode = stat.S_IMODE(path.stat().st_mode)
            if path.is_dir() and mode != 0o700:
                fail(f"M1 planner published a nonprivate directory: {path}")
            if path.is_file() and mode != 0o600:
                fail(f"M1 planner published a nonprivate file: {path}")
            if (
                path.is_file()
                and path.name != "plan.json"
                and path.stat().st_mtime_ns > plan_time
            ):
                fail("M1 planner did not publish plan.json last")
        if git(ferric_fixture, ["status", "--porcelain=v1"], "recheck Ferric fixture"):
            fail("M1 planner dirtied its exact Ferric integration source")
        if git(fe2o3_fixture, ["status", "--porcelain=v1"], "recheck fe2o3 fixture"):
            fail("M1 planner dirtied its exact fe2o3 integration source")

        direct_case = temporary / "direct-pin"
        clone_at(ferric_fixture, direct_case)
        direct_line = (
            'fe2o3-aql = { git = "https://github.com/harsh-nod/fe2o3.git", '
            f'rev = "{fe2o3_commit}" }}'
        )
        replace_once(
            direct_case / "Cargo.toml",
            direct_line,
            direct_line.replace("github.com/harsh-nod", "evil.invalid"),
        )
        commit_fixture(direct_case, "mutate direct fe2o3 repository")
        expect_prepare_failure(
            direct_case / "proofs/m1-qualification/planner.py",
            direct_case,
            fe2o3_fixture,
            temporary / "direct-pin-output",
            "direct fe2o3 dependency declaration drifted",
        )

        resolved_case = temporary / "resolved-pin"
        clone_at(ferric_fixture, resolved_case)
        lock_path = resolved_case / "Cargo.lock"
        lock_source = lock_path.read_text(encoding="utf-8")
        expected_url = "git+https://github.com/harsh-nod/fe2o3.git"
        if lock_source.count(expected_url) != 27:
            fail("integration fixture resolved fe2o3 roster drifted before mutation")
        lock_path.write_text(
            lock_source.replace(expected_url, "git+https://evil.invalid/fe2o3.git"),
            encoding="utf-8",
        )
        commit_fixture(resolved_case, "mutate resolved fe2o3 repository")
        expect_prepare_failure(
            resolved_case / "proofs/m1-qualification/planner.py",
            resolved_case,
            fe2o3_fixture,
            temporary / "resolved-pin-output",
            "resolved fe2o3 package declaration drifted",
        )

        topology_case = temporary / "topology"
        clone_at(ferric_fixture, topology_case)
        replace_once(
            topology_case / "crates/ferric-engine/Cargo.toml",
            "fe2o3-service-host.workspace = true\n",
            "",
        )
        commit_fixture(topology_case, "mutate fe2o3 root topology")
        expect_prepare_failure(
            topology_case / "proofs/m1-qualification/planner.py",
            topology_case,
            fe2o3_fixture,
            temporary / "topology-output",
            "dependency topology does not equal the admitted root graph",
        )

        dirty_case = temporary / "dirty"
        clone_at(ferric_fixture, dirty_case)
        (dirty_case / "untracked-source-probe").write_text(
            "hostile\n", encoding="ascii"
        )
        expect_prepare_failure(
            dirty_case / "proofs/m1-qualification/planner.py",
            dirty_case,
            fe2o3_fixture,
            temporary / "dirty-output",
            "repository must be an exact clean worktree",
        )

        requirements = planner.read_canonical_json(
            ferric_fixture / "proofs/M1_REQUIREMENTS.json",
            "integration M1 requirements",
        )
        upstream_base = requirements["m1_upstream_base_commit"]
        upstream_parent = git(
            fe2o3_source,
            ["rev-parse", f"{upstream_base}^"],
            "resolve hostile fe2o3 base parent",
        )
        base_case = temporary / "base"
        clone_at(fe2o3_source, base_case, upstream_parent)
        expect_prepare_failure(
            fixture_planner,
            ferric_fixture,
            base_case,
            temporary / "base-output",
            "HEAD does not descend from its reviewed M1 base",
        )

        path_case = temporary / "path"
        clone_at(ferric_fixture, path_case)
        (path_case / "crates/ferric-qwen-kernels/src/gemm.rs").unlink()
        commit_fixture(path_case, "remove required M1 source path")
        expect_prepare_failure(
            path_case / "proofs/m1-qualification/planner.py",
            path_case,
            fe2o3_fixture,
            temporary / "path-output",
            "M1 path is absent from exact ferric tree",
        )

        closure_case = temporary / "closure"
        clone_at(ferric_fixture, closure_case)
        with (closure_case / ".git/info/exclude").open(
            "a", encoding="utf-8"
        ) as exclude:
            exclude.write("\n/ignored-source-probe\n")
        (closure_case / "ignored-source-probe").write_text(
            "hostile\n", encoding="ascii"
        )
        if git(closure_case, ["status", "--porcelain=v1"], "inspect closure fixture"):
            fail("ignored source-closure probe unexpectedly dirtied Git status")
        expect_prepare_failure(
            closure_case / "proofs/m1-qualification/planner.py",
            closure_case,
            fe2o3_fixture,
            temporary / "closure-output",
            "M1 source closure is not the exact committed tree",
        )

        preexisting = temporary / "preexisting-output"
        preexisting.mkdir()
        expect_prepare_failure(
            fixture_planner,
            ferric_fixture,
            fe2o3_fixture,
            preexisting,
            "planning output already exists",
        )
        symlink_target = temporary / "symlink-target"
        symlink_target.mkdir()
        symlink_output = temporary / "symlink-output"
        symlink_output.symlink_to(symlink_target, target_is_directory=True)
        expect_prepare_failure(
            fixture_planner,
            ferric_fixture,
            fe2o3_fixture,
            symlink_output,
            "planning output already exists",
        )
        inside = ferric_fixture / "forbidden-output"
        expect_prepare_failure(
            fixture_planner,
            ferric_fixture,
            fe2o3_fixture,
            inside,
            "planning output must be external to both source repositories",
        )


def main() -> None:
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_OBJECT_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    fe2o3_source = Path(sys.argv[2]).resolve(strict=True)
    planner_path = repo / "proofs/m1-qualification/planner.py"
    bytecode_before = set(planner_path.parent.rglob("*.pyc"))
    planner = load_planner(planner_path)
    requirements = planner.read_canonical_json(
        repo / "proofs/M1_REQUIREMENTS.json", "M1 requirements"
    )

    first = planner.allocate_bindings(repo, requirements)
    planner.validate_allocation(requirements, first)
    second = planner.allocate_bindings(repo, copy.deepcopy(requirements))
    if planner.canonical_bytes({"slots": first}) != planner.canonical_bytes(
        {"slots": second}
    ):
        fail("M1 binding allocation is nondeterministic")

    roadmap = [
        slot for slot in first if slot["binding"]["obligation_class"] == "Roadmap"
    ]
    assurance = [
        slot for slot in first if slot["binding"]["obligation_class"] == "Assurance"
    ]
    if (len(roadmap), len(assurance), len(first)) != (168, 186, 354):
        fail("M1 binding allocation has the wrong exact counts")
    if [slot["binding"]["id"] for slot in first] != [
        f"binding.{ordinal:05d}" for ordinal in range(354)
    ]:
        fail("M1 binding slot IDs are not stable and sequential")
    expected_allocation = (
        "948ad3023df7ad4b1313ed865b54464f63b6bad9406f1510c85e60f9db055bd6"
    )
    actual_allocation = planner.digest_bytes(planner.allocation_tsv(first))
    if actual_allocation != expected_allocation:
        fail(
            "M1 binding allocation golden identity drifted "
            f"(expected={expected_allocation}, actual={actual_allocation})"
        )
    graph = [
        slot
        for slot in assurance
        if slot["binding"]["obligation_id"] == "graph_refined"
    ]
    if len(graph) != 7:
        fail("graph_refined does not carry its exact constrained seven-slot allocation")
    foundations = [
        slot
        for slot in graph
        if slot["binding"]["evidence_kind"] in {"negative-mutation", "verus-theorem"}
    ]
    if {slot["binding"]["path_id"] for slot in foundations} != {"graph-proof"}:
        fail("graph_refined foundation reachability was weakened")
    if {slot["binding"]["path_id"] for slot in graph} != {
        "generated-runner",
        "graph-proof",
        "model-admission",
        "physical-runner",
        "runner-generator",
        "speculative-graph",
    }:
        fail("graph_refined path coverage is incomplete")

    available = [
        slot for slot in first if slot["producer"]["availability"] == "available"
    ]
    available_kinds: dict[str, int] = {}
    for slot in available:
        kind = slot["binding"]["evidence_kind"]
        available_kinds[kind] = available_kinds.get(kind, 0) + 1
    if available_kinds != {
        "artifact-identity": 74,
        "canonical-structure-check": 14,
        "external-contract": 15,
        "fe2o3-contract": 52,
        "hardware-test": 58,
        "negative-mutation": 30,
        "unsupported-rationale": 5,
        "verus-theorem": 26,
    }:
        fail(f"existing M1 producer coverage drifted: {available_kinds}")
    if len(available) != 274 or len(first) - len(available) != 80:
        fail("missing binding-producer count drifted")

    identity_slots = [
        slot
        for slot in first
        if slot["binding"]["evidence_kind"] == "artifact-identity"
    ]
    identity_ids = [slot["binding"]["id"] for slot in identity_slots]
    if planner.digest_bytes(("\n".join(identity_ids) + "\n").encode("ascii")) != (
        "036a350d44c964bd96c44328087d541db7116452093ed9067987fa8497e57258"
    ):
        fail("M1 artifact-identity binding ID roster drifted")
    for slot in identity_slots:
        binding = slot["binding"]
        artifact_id = binding["artifact_id"]
        if slot["producer"] != {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-artifact-identity.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                binding["id"],
            ],
            "role": "ferric-artifact-identity-reporter",
        } or slot["expected_artifact"] != {
            "id": artifact_id,
            "kind": "ArtifactIdentityReport",
            "path": f"artifacts/{artifact_id}.artifact-identity.json",
        }:
            fail(f"M1 artifact-identity producer command drifted: {binding['id']}")

    canonical_slots = [
        slot
        for slot in first
        if slot["binding"]["evidence_kind"] == "canonical-structure-check"
    ]
    canonical_ids = [slot["binding"]["id"] for slot in canonical_slots]
    if planner.digest_bytes(("\n".join(canonical_ids) + "\n").encode("ascii")) != (
        "9bcebd22a0ae9eaa63322c075ea6f8b69af1599a0ef521948d922dc6e8343b9d"
    ):
        fail("M1 canonical-structure binding ID roster drifted")
    canonical_rows = []
    for slot in canonical_slots:
        binding = slot["binding"]
        artifact_id = binding["artifact_id"]
        expected_artifact = {
            "id": artifact_id,
            "kind": "CheckerTranscript",
            "path": f"artifacts/{artifact_id}.canonical-structure.json",
        }
        expected_producer = {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-canonical-structure.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                binding["id"],
            ],
            "role": "ferric-canonical-structure-reporter",
        }
        if (
            slot["producer"] != expected_producer
            or slot["expected_artifact"] != expected_artifact
        ):
            fail(f"M1 canonical-structure producer command drifted: {binding['id']}")
        canonical_rows.append(
            "|".join(
                [
                    binding["id"],
                    binding["obligation_class"],
                    binding["obligation_id"],
                    binding["profile_id"],
                    binding["path_id"],
                    binding["source_identity_id"],
                    artifact_id,
                    expected_artifact["path"],
                ]
            )
            + "\n"
        )
    if planner.digest_bytes("".join(canonical_rows).encode("ascii")) != (
        "204b1a90357249a1b3e9ac8094e40a5f424b3ba1a7aac2fccd0661773054814d"
    ):
        fail("M1 canonical-structure allocation topology drifted")

    external_slots = [
        slot
        for slot in first
        if slot["binding"]["evidence_kind"] == "external-contract"
    ]
    external_ids = [slot["binding"]["id"] for slot in external_slots]
    if planner.digest_bytes(("\n".join(external_ids) + "\n").encode("ascii")) != (
        "1f8baa6f1e37438e0f2643425a38f1747900ebd41e74eed4c8d851cdb05ae20e"
    ):
        fail("M1 external-contract binding ID roster drifted")
    external_rows = []
    for slot in external_slots:
        binding = slot["binding"]
        artifact_id = binding["artifact_id"]
        expected_artifact = {
            "id": artifact_id,
            "kind": "ContractDocument",
            "path": f"artifacts/{artifact_id}.external-contract.json",
        }
        expected_producer = {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-external-contract.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                binding["id"],
            ],
            "role": "ferric-m1-external-assumption-reporter",
        }
        if (
            slot["producer"] != expected_producer
            or slot["expected_artifact"] != expected_artifact
        ):
            fail(f"M1 external-contract producer command drifted: {binding['id']}")
        external_rows.append(
            "|".join(
                [
                    binding["id"],
                    binding["obligation_class"],
                    binding["obligation_id"],
                    binding["profile_id"],
                    binding["path_id"],
                    binding["source_identity_id"],
                    artifact_id,
                    expected_artifact["path"],
                ]
            )
            + "\n"
        )
    if planner.digest_bytes("".join(external_rows).encode("ascii")) != (
        "2b88b7e5fdac2bfaecff2f2eef8345b35b101d8185c24fa9fbb43ce1304caf99"
    ):
        fail("M1 external-contract allocation topology drifted")

    fe2o3_contract_slots = [
        slot for slot in first if slot["binding"]["evidence_kind"] == "fe2o3-contract"
    ]
    fe2o3_contract_ids = [slot["binding"]["id"] for slot in fe2o3_contract_slots]
    if planner.digest_bytes(("\n".join(fe2o3_contract_ids) + "\n").encode("ascii")) != (
        "3a9caeaddd98840035fb55233aa1b3ccf53993313a955f95248a03f831cd45a9"
    ):
        fail("M1 fe2o3-contract binding ID roster drifted")
    fe2o3_contract_rows = []
    for slot in fe2o3_contract_slots:
        binding = slot["binding"]
        artifact_id = binding["artifact_id"]
        expected_artifact = {
            "id": artifact_id,
            "kind": "ContractDocument",
            "path": f"artifacts/{artifact_id}.fe2o3-contract.json",
        }
        expected_producer = {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-fe2o3-contract.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                binding["id"],
            ],
            "role": "ferric-m1-fe2o3-contract-reporter",
        }
        if (
            slot["producer"] != expected_producer
            or slot["expected_artifact"] != expected_artifact
        ):
            fail(f"M1 fe2o3-contract producer command drifted: {binding['id']}")
        fe2o3_contract_rows.append(
            "|".join(
                [
                    binding["id"],
                    binding["obligation_class"],
                    binding["obligation_id"],
                    binding["profile_id"],
                    binding["path_id"],
                    binding["source_identity_id"],
                    artifact_id,
                    expected_artifact["path"],
                ]
            )
            + "\n"
        )
    if planner.digest_bytes("".join(fe2o3_contract_rows).encode("ascii")) != (
        "04dee49ed87d5e3659abdf5478617188d45af3a278b4db958048f4598bfcf841"
    ):
        fail("M1 fe2o3-contract allocation topology drifted")

    hardware_slots = [
        slot for slot in first if slot["binding"]["evidence_kind"] == "hardware-test"
    ]
    hardware_ids = [slot["binding"]["id"] for slot in hardware_slots]
    if planner.digest_bytes(("\n".join(hardware_ids) + "\n").encode("ascii")) != (
        "50ab14c739eb88d8ded5becc86ccf5420386e905ab2d583463da4dfbf82f17cb"
    ):
        fail("M1 hardware-test binding ID roster drifted")
    hardware_rows = []
    for slot in hardware_slots:
        binding = slot["binding"]
        artifact_id = binding["artifact_id"]
        expected_artifact = {
            "id": artifact_id,
            "kind": "HardwareTranscript",
            "path": f"artifacts/{artifact_id}.hardware-transcript.json",
        }
        expected_producer = {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-hardware-transcript.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                "HARDWARE_HARNESS",
                "KERNEL_ARTIFACTS",
                "HARDWARE_ENVIRONMENT",
                binding["id"],
            ],
            "role": "ferric-mi300x-hardware-harness",
        }
        if (
            slot["producer"] != expected_producer
            or slot["expected_artifact"] != expected_artifact
        ):
            fail(f"M1 hardware-test producer command drifted: {binding['id']}")
        hardware_rows.append(
            "|".join(
                [
                    binding["id"],
                    binding["obligation_class"],
                    binding["obligation_id"],
                    binding["profile_id"],
                    binding["path_id"],
                    binding["source_identity_id"],
                    artifact_id,
                    expected_artifact["path"],
                ]
            )
            + "\n"
        )
    if planner.digest_bytes("".join(hardware_rows).encode("ascii")) != (
        "b860743335a8be9deb576f82b17612c0a009b6caf7adad86b5f34d6500f1e480"
    ):
        fail("M1 hardware-test allocation topology drifted")

    rationale_slots = [
        slot
        for slot in first
        if slot["binding"]["evidence_kind"] == "unsupported-rationale"
    ]
    rationale_ids = [slot["binding"]["id"] for slot in rationale_slots]
    if planner.digest_bytes(("\n".join(rationale_ids) + "\n").encode("ascii")) != (
        "234623d24473bb78252a0541395d68f09b591d7e947c8e55e286a2e8b57a6b81"
    ):
        fail("M1 unsupported-rationale binding ID roster drifted")
    rationale_rows = []
    for slot in rationale_slots:
        binding = slot["binding"]
        artifact_id = binding["artifact_id"]
        expected_artifact = {
            "id": artifact_id,
            "kind": "UnsupportedRationale",
            "path": f"artifacts/{artifact_id}.unsupported-rationale.json",
        }
        expected_producer = {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-unsupported-rationale.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                binding["id"],
            ],
            "role": "ferric-m1-nonclaim-reporter",
        }
        if (
            slot["producer"] != expected_producer
            or slot["expected_artifact"] != expected_artifact
        ):
            fail(f"M1 unsupported-rationale producer command drifted: {binding['id']}")
        rationale_rows.append(
            "|".join(
                [
                    binding["id"],
                    binding["obligation_class"],
                    binding["obligation_id"],
                    binding["profile_id"],
                    binding["path_id"],
                    binding["source_identity_id"],
                    artifact_id,
                    expected_artifact["path"],
                ]
            )
            + "\n"
        )
    if planner.digest_bytes("".join(rationale_rows).encode("ascii")) != (
        "5c5bd4569ae975c44b8cd8292a0216f063fbe9a4461b3eb89225790f7ce5bd41"
    ):
        fail("M1 unsupported-rationale allocation topology drifted")

    tcb_work = planner.global_work_items()[:3]
    expected_tcb_work = []
    for identifier, kind in planner.TCB:
        artifact_id = f"artifact.{identifier}"
        expected_tcb_work.append(
            {
                "expected_artifact": {
                    "id": artifact_id,
                    "kind": "TcbReport",
                    "path": f"artifacts/{artifact_id}.tcb-report.json",
                },
                "id": f"work.{identifier}",
                "producer": {
                    "availability": "available",
                    "command": [
                        "python3",
                        "-I",
                        "proofs/m1-qualification/produce-tcb-report.py",
                        "FERRIC_REPO",
                        "FE2O3_REPO",
                        "PLAN_DIR",
                        identifier,
                    ],
                    "role": f"ferric-{kind.lower()}-tcb-reporter",
                },
                "state": "missing",
                "subject": f"tcb:{identifier}",
            }
        )
    if tcb_work != expected_tcb_work:
        fail("M1 planner TCB producer commands drifted")

    kind_counts: dict[str, int] = {}
    for slot in first:
        kind = slot["binding"]["evidence_kind"]
        kind_counts[kind] = kind_counts.get(kind, 0) + 1
    if kind_counts != {
        "artifact-identity": 74,
        "canonical-structure-check": 14,
        "external-contract": 15,
        "fe2o3-contract": 52,
        "hardware-test": 58,
        "independent-validator": 44,
        "negative-mutation": 30,
        "performance-gate": 36,
        "unsupported-rationale": 5,
        "verus-theorem": 26,
    }:
        fail(f"M1 exact evidence-kind counts drifted: {kind_counts}")

    extras: list[tuple[str, str, str, str, str]] = []
    seen_pairs: set[tuple[str, str, str, str]] = set()
    for slot in first:
        binding = slot["binding"]
        pair = (
            binding["obligation_class"],
            binding["obligation_id"],
            binding["profile_id"],
            binding["evidence_kind"],
        )
        if pair in seen_pairs:
            extras.append((*pair, binding["path_id"]))
        else:
            seen_pairs.add(pair)
        payload = {
            key: value for key, value in binding.items() if key != "binding_sha256"
        }
        if binding["binding_sha256"] != planner.canonical_digest(payload):
            fail(f"M1 binding digest drifted: {binding['id']}")
        if binding["evidence_kind"] == "tcb-report":
            fail("M1 planner emitted a forbidden obligation-bound TCB report")
    if extras != [
        (
            "Assurance",
            "graph_refined",
            "composition",
            "artifact-identity",
            "runner-generator",
        ),
        (
            "Assurance",
            "graph_refined",
            "composition",
            "artifact-identity",
            "speculative-graph",
        ),
        (
            "Assurance",
            "distribution_preserved",
            "nonclaim",
            "unsupported-rationale",
            "speculation-proof",
        ),
        (
            "Assurance",
            "machine_refined",
            "nonclaim",
            "unsupported-rationale",
            "m1-tcb",
        ),
    ]:
        fail(f"M1 duplicate-pair path slots drifted: {extras}")

    registries = planner.foundation_registries(repo)
    for slot in available:
        binding = slot["binding"]
        if binding["evidence_kind"] in {
            "artifact-identity",
            "canonical-structure-check",
            "external-contract",
            "fe2o3-contract",
            "hardware-test",
            "unsupported-rationale",
        }:
            continue
        expected = registries[binding["evidence_kind"]][binding["obligation_id"]][
            binding["path_id"]
        ]
        if (
            binding["obligation_class"] != "Assurance"
            or binding["source_identity_id"] != "source.ferric"
            or slot["foundation_selectors"] != expected
            or any(selector.startswith("missing-") for selector in expected)
        ):
            fail(f"M1 foundation selector roster drifted: {binding['id']}")

    obligations = planner.obligation_slots(requirements, first)
    if len(obligations) != 50:
        fail("M1 obligation assembly roster is incomplete")

    ids = [slot["binding"]["id"] for slot in first]
    artifact_ids = [slot["expected_artifact"]["id"] for slot in first]
    artifact_paths = [slot["expected_artifact"]["path"] for slot in first]
    if (
        len(ids) != len(set(ids))
        or len(artifact_ids) != len(set(artifact_ids))
        or len(artifact_paths) != len(set(artifact_paths))
    ):
        fail("M1 planner reused a binding artifact identity or path")

    hostile = copy.deepcopy(requirements)
    graph_record = next(
        record
        for record in hostile["assurance_properties"]
        if record["name"] == "graph_refined"
    )
    graph_record["path_obligations"].remove("graph-proof")
    rejected = io.StringIO()
    try:
        with contextlib.redirect_stderr(rejected):
            planner.allocate_bindings(repo, hostile)
    except SystemExit:
        if "has no foundation path" not in rejected.getvalue():
            fail("M1 planner rejected an unreachable foundation for the wrong reason")
    else:
        fail("M1 planner accepted a property with no reachable proof foundation")

    refusal = subprocess.run(
        [sys.executable, "-I", str(planner_path), "--emit-index"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    expected = "cannot emit an M1 evidence index or receipt"
    if refusal.returncode == 0 or expected not in refusal.stdout:
        fail("M1 planner did not categorically refuse closure output")

    exercise_prepare_boundaries(repo, fe2o3_source, planner)
    if set(planner_path.parent.rglob("*.pyc")) != bytecode_before:
        fail("M1 planner policy created Python bytecode in the Ferric worktree")

    print(
        "PASS: M1 evidence planner policy accepted 354 deterministic slots "
        "and rejected hostile pins, sources, paths, publication, and closure output"
    )


if __name__ == "__main__":
    main()
