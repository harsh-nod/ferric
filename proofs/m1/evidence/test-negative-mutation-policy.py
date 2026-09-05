#!/usr/bin/env python3
"""Exercise hostile artifacts against the trusted M1 mutation validator."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


PROTOCOL = "ferric.m1-validator.negative-mutation.v1"
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
RUN_KEYS = (
    "FORMAT",
    "FERRIC_COMMIT",
    "FERRIC_TREE",
    "FERRIC_SOURCE_CLOSURE_SHA256",
    "VERUS_VERSION",
    "VERUS_SHA256",
    "VERUS_CLOSURE_MANIFEST_SHA256",
    "VERUS_CLOSURE_SHA256",
    "REGISTRY_SHA256",
    "RUNNER_SHA256",
    "AUTHORITY",
    "NONCLAIM",
)
RESULT_KEYS = (
    "FORMAT",
    "MUTATION",
    "RUN_IDENTITY_SHA256",
    "ACTIVE_FOUNDATIONS_SHA256",
    "SELECTED_FOUNDATIONS_SHA256",
    "VERUS_CLOSURE_TRANSCRIPT_SHA256",
    "MUTATION_RECORD",
    "MUTATION_RECORD_SHA256",
    "MUTATION_RECORD_SIZE",
    "COMPILE_TRANSCRIPT",
    "COMPILE_TRANSCRIPT_SHA256",
    "COMPILE_TRANSCRIPT_SIZE",
    "COMPILE_EXIT_STATUS",
    "VERUS_TRANSCRIPT",
    "VERUS_TRANSCRIPT_SHA256",
    "VERUS_TRANSCRIPT_SIZE",
    "VERUS_EXIT_STATUS",
    "RESULT",
)
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = ("Compiler", "Hardware", "Runtime")
FixtureMutation = Callable[[Path, dict[str, Any], tuple[str, ...]], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def canonical_digest(value: dict[str, Any]) -> str:
    payload = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    return digest_bytes(payload.encode("ascii"))


def git(repo: Path, revision: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), "rev-parse", revision],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    ).stdout.strip()


def write_kv(path: Path, keys: tuple[str, ...], values: dict[str, str]) -> None:
    path.write_text("".join(f"{key}={values[key]}\n" for key in keys), encoding="ascii")


def parse_kv(path: Path) -> dict[str, str]:
    return dict(
        line.split("=", 1) for line in path.read_text(encoding="ascii").splitlines()
    )


def registry(repo: Path) -> tuple[bytes, list[tuple[str, ...]]]:
    lines = (
        (repo / "proofs/m1/negative/REQUIRED_FOUNDATIONS")
        .read_text(encoding="ascii")
        .splitlines()
    )
    rows = [tuple(line.removeprefix("mutation=").split("|")) for line in lines[1:]]
    active = "".join("|".join(row) + "\n" for row in rows).encode("ascii")
    return active, rows


def exact_source_closure(repo: Path, root: Path) -> str:
    output = root / "source-closure"
    result = subprocess.run(
        [
            sys.executable,
            "-I",
            str(repo / "proofs/m1/evidence/measure-source-closure.py"),
            str(repo),
            str(output),
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if result.returncode != 0:
        fail(f"cannot build baseline M1 source identity\n{result.stdout}")
    return digest_file(output)


def closure_identity(repo: Path) -> tuple[str, str, bytes]:
    manifest = (repo / "proofs/verus/VERUS_CLOSURE_MANIFEST").read_bytes()
    records = dict(
        line.split("=", 1)
        for line in manifest.decode("ascii").splitlines()
        if "=" in line
    )
    transcript = (
        "PASS: pinned Verus release closure matched "
        f"({records['file-count']} files, {records['total-bytes']} bytes)\n"
    ).encode("ascii")
    return digest_bytes(manifest), records["closure-sha256"], transcript


def make_context(
    repo: Path,
    result: Path,
    row: tuple[str, ...],
    source_closure_sha256: str,
) -> dict[str, Any]:
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = json.loads(requirements_path.read_text(encoding="utf-8"))
    properties = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    profiles = {
        record["id"]: set(record["kinds"])
        for record in requirements["evidence_profiles"]
    }
    paths = {record["id"]: record for record in requirements["path_obligations"]}
    name, _, property_name, path_id, *_ = row
    profile_id = next(
        (
            candidate
            for candidate in properties[property_name]["evidence_profiles"]
            if "negative-mutation" in profiles[candidate]
        ),
        None,
    )
    if profile_id is None:
        fail(f"fixture property does not admit negative-mutation evidence: {property_name}")
    artifact = {
        "id": f"artifact.mutation.{name}",
        "kind": "MutationTranscript",
        "path": f"runs/{result.name}",
        "sha256": digest_file(result),
        "size_bytes": result.stat().st_size,
    }
    binding = {
        "artifact_id": artifact["id"],
        "evidence_kind": "negative-mutation",
        "id": f"binding.mutation.{name}",
        "obligation_class": "Assurance",
        "obligation_id": property_name,
        "path_id": path_id,
        "profile_id": profile_id,
        "source_identity_id": "source.ferric",
        "statement_sha256": digest_bytes(
            properties[property_name]["boundary"].encode("utf-8")
        ),
        "tcb_ids": list(TCB_IDS),
    }
    binding["binding_sha256"] = canonical_digest(binding)
    return {
        "artifact": artifact,
        "artifact_absolute_path": str(result),
        "binding": binding,
        "format": INDEX_FORMAT,
        "path_resolution": {
            "availability": paths[path_id]["availability"],
            "id": path_id,
            "path": paths[path_id]["path"],
            "repository": "ferric",
            "source_identity_id": "source.ferric",
        },
        "requirements_sha256": digest_file(requirements_path),
        "sources": [
            {
                "base_commit": requirements["m1_upstream_base_commit"],
                "commit": "1" * 40,
                "id": "source.fe2o3",
                "repository": "fe2o3",
                "source_closure_artifact_id": "artifact.source.fe2o3",
                "source_closure_sha256": "2" * 64,
                "tree": "3" * 40,
            },
            {
                "base_commit": "c5a86fd56c1c817664593df25c04bbed30e84971",
                "commit": git(repo, "HEAD^{commit}"),
                "id": "source.ferric",
                "repository": "ferric",
                "source_closure_artifact_id": "artifact.source.ferric",
                "source_closure_sha256": source_closure_sha256,
                "tree": git(repo, "HEAD^{tree}"),
            },
        ],
        "subject": f"binding:{binding['id']}",
        "tcb": [
            {
                "artifact_id": f"artifact.{identifier}",
                "id": identifier,
                "identity_sha256": f"{offset}" * 64,
                "kind": kind,
            }
            for offset, (identifier, kind) in enumerate(
                zip(TCB_IDS, TCB_KINDS, strict=True), start=4
            )
        ],
    }


def refresh_result(run: Path, name: str) -> None:
    path = run / f"{name}.result"
    result = parse_kv(path)
    identities = (
        ("RUN_IDENTITY", "RUN_IDENTITY_SHA256", None),
        ("active-foundations", "ACTIVE_FOUNDATIONS_SHA256", None),
        ("selected-foundations", "SELECTED_FOUNDATIONS_SHA256", None),
        (
            "verus-closure.transcript",
            "VERUS_CLOSURE_TRANSCRIPT_SHA256",
            None,
        ),
        (
            result["MUTATION_RECORD"],
            "MUTATION_RECORD_SHA256",
            "MUTATION_RECORD_SIZE",
        ),
        (
            result["COMPILE_TRANSCRIPT"],
            "COMPILE_TRANSCRIPT_SHA256",
            "COMPILE_TRANSCRIPT_SIZE",
        ),
        (
            result["VERUS_TRANSCRIPT"],
            "VERUS_TRANSCRIPT_SHA256",
            "VERUS_TRANSCRIPT_SIZE",
        ),
    )
    for filename, digest_key, size_key in identities:
        companion = run / filename
        result[digest_key] = digest_file(companion)
        if size_key is not None:
            result[size_key] = str(companion.stat().st_size)
    write_kv(path, RESULT_KEYS, result)


def refresh_context(context: dict[str, Any]) -> None:
    path = Path(context["artifact_absolute_path"])
    context["artifact"]["sha256"] = digest_file(path)
    context["artifact"]["size_bytes"] = path.stat().st_size


def reconstruct_mutation(
    repo: Path, root: Path, row: tuple[str, ...]
) -> tuple[str, str, str]:
    name, _, _, _, _, source, mutator, _, _, _, clause = row
    source_copy = root / "mutation-copy" / source
    source_copy.parent.mkdir(parents=True)
    shutil.copy2(repo / source, source_copy)
    result = subprocess.run(
        [
            sys.executable,
            "-I",
            str(repo / "proofs/m1/negative/components" / mutator),
            str(root / "mutation-copy"),
        ],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    expected = f"MUTATED_SOURCE={source}\nMUTATION={name}\nCLAUSE={clause}\n"
    if not result.stdout.startswith(expected):
        fail(f"baseline mutator output drifted: {name}")
    anchor = result.stdout.splitlines()[3].removeprefix("ANCHOR_SHA256=")
    return anchor, digest_file(repo / source), digest_file(source_copy)


def build_run(
    repo: Path, root: Path, row: tuple[str, ...], source_closure_sha256: str
) -> tuple[Path, dict[str, Any]]:
    run = root / "run"
    run.mkdir()
    active, _ = registry(repo)
    selected = ("|".join(row) + "\n").encode("ascii")
    (run / "active-foundations").write_bytes(active)
    (run / "selected-foundations").write_bytes(selected)
    manifest_sha, closure_sha, closure_transcript = closure_identity(repo)
    (run / "verus-closure.transcript").write_bytes(closure_transcript)
    run_values = {
        "FORMAT": "FERRIC-M1-NEGATIVE-RUN-V1",
        "FERRIC_COMMIT": git(repo, "HEAD^{commit}"),
        "FERRIC_TREE": git(repo, "HEAD^{tree}"),
        "FERRIC_SOURCE_CLOSURE_SHA256": source_closure_sha256,
        "VERUS_VERSION": (repo / "proofs/verus/VERUS_VERSION")
        .read_text(encoding="ascii")
        .strip(),
        "VERUS_SHA256": (repo / "proofs/verus/VERUS_SHA256")
        .read_text(encoding="ascii")
        .strip(),
        "VERUS_CLOSURE_MANIFEST_SHA256": manifest_sha,
        "VERUS_CLOSURE_SHA256": closure_sha,
        "REGISTRY_SHA256": digest_file(
            repo / "proofs/m1/negative/REQUIRED_FOUNDATIONS"
        ),
        "RUNNER_SHA256": digest_file(repo / "proofs/m1/negative/run-same-source.sh"),
        "AUTHORITY": "hostile-foundation-proof-rejection-only",
        "NONCLAIM": "no-m1-property-or-roadmap-closure",
    }
    write_kv(run / "RUN_IDENTITY", RUN_KEYS, run_values)

    (
        name,
        foundation,
        property_name,
        path_id,
        package,
        source,
        mutator,
        marker,
        module,
        function,
        clause,
    ) = row
    anchor, original_sha, mutated_sha = reconstruct_mutation(repo, root, row)
    mutation_values = {
        "FORMAT": "FERRIC-M1-NEGATIVE-MUTATION-V1",
        "MUTATED_SOURCE": source,
        "MUTATION": name,
        "CLAUSE": clause,
        "ANCHOR_SHA256": anchor,
        "MUTATOR_SHA256": digest_file(repo / "proofs/m1/negative/components" / mutator),
        "ORIGINAL_SOURCE_SHA256": original_sha,
        "MUTATED_SOURCE_SHA256": mutated_sha,
        "FOUNDATION": foundation,
        "OPEN_ASSURANCE_PROPERTY": property_name,
        "OPEN_PATH_OBLIGATION": path_id,
        "VERUS_PACKAGE": package,
        "VERUS_MODULE": module,
        "VERUS_FUNCTION": function,
        "EXPECTED_FAILURE_MARKER": marker,
        "CARGO_CHECK": "passed",
    }
    mutation_keys = tuple(mutation_values)
    write_kv(run / f"{name}.mutation", mutation_keys, mutation_values)

    compile_text = (
        "FORMAT=FERRIC-M1-NEGATIVE-COMPILE-V1\n"
        f"MUTATION={name}\n"
        f"CARGO_PACKAGE={package}\n"
        "COMMAND=cargo-check-locked-all-targets\n"
        "    Checking proc-macro2 v1.0.107\n"
        "    Checking quote v1.0.47\n"
        f"    Checking {package} v0.1.0 "
        f"(/tmp/ferric-m1-negative.TEST/copy-{name}/crates/{package})\n"
        "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.25s\n"
    )
    (run / f"{name}.compile.transcript").write_text(compile_text, encoding="utf-8")
    error = (
        "error: assertion failed"
        if marker == "assertion"
        else "error: postcondition not satisfied"
    )
    verus_text = (
        "FORMAT=FERRIC-M1-NEGATIVE-VERUS-V1\n"
        f"MUTATION={name}\n"
        f"VERUS_PACKAGE={package}\n"
        f"VERUS_MODULE={module}\n"
        f"VERUS_FUNCTION={function}\n"
        "COMMAND=cargo-verus-build-lib-locked-release-no-cheating-exact-function\n"
        f"EXPECTED_FAILURE_MARKER={marker}\n"
        "   Compiling proc-macro2 v1.0.107\n"
        "verification results:: 2044 verified, 0 errors\n"
        f"   Compiling {package} v0.1.0 "
        f"(/tmp/ferric-m1-negative.TEST/copy-{name}/crates/{package})\n"
        f"note: verifying module {module} (selected functions)\n\n"
        f"{error}\n"
        f"   --> {source}:427:13\n"
        "    |\n"
        "427 |     ensures result == expected_result,\n"
        "    |             ^^^^^^^^^^^^^^^^^^^^^^^^^ failed this proof obligation\n"
        "    |\n"
        "    = note: exact selected executable body did not establish its contract\n"
        "    = note: the canonical same-source runner selected one registered function\n"
        "    = note: compiler diagnostics are retained as complete UTF-8 output\n\n"
        "verification results:: 0 verified, 1 errors (partial verification with `--verify-*`)\n"
        f"error: could not compile `{package}` (lib) due to 1 previous error\n"
    )
    (run / f"{name}.verus.transcript").write_text(verus_text, encoding="utf-8")

    result_values = {
        "FORMAT": "FERRIC-M1-NEGATIVE-RESULT-V1",
        "MUTATION": name,
        "RUN_IDENTITY_SHA256": "0" * 64,
        "ACTIVE_FOUNDATIONS_SHA256": "0" * 64,
        "SELECTED_FOUNDATIONS_SHA256": "0" * 64,
        "VERUS_CLOSURE_TRANSCRIPT_SHA256": "0" * 64,
        "MUTATION_RECORD": f"{name}.mutation",
        "MUTATION_RECORD_SHA256": "0" * 64,
        "MUTATION_RECORD_SIZE": "1",
        "COMPILE_TRANSCRIPT": f"{name}.compile.transcript",
        "COMPILE_TRANSCRIPT_SHA256": "0" * 64,
        "COMPILE_TRANSCRIPT_SIZE": "1",
        "COMPILE_EXIT_STATUS": "0",
        "VERUS_TRANSCRIPT": f"{name}.verus.transcript",
        "VERUS_TRANSCRIPT_SHA256": "0" * 64,
        "VERUS_TRANSCRIPT_SIZE": "1",
        "VERUS_EXIT_STATUS": "101",
        "RESULT": "proof-rejected",
    }
    result_path = run / f"{name}.result"
    write_kv(result_path, RESULT_KEYS, result_values)
    refresh_result(run, name)
    return result_path, make_context(repo, result_path, row, source_closure_sha256)


def invoke(
    repo: Path, context: dict[str, Any], raw: bytes | None = None
) -> subprocess.CompletedProcess[bytes]:
    if raw is None:
        raw = (
            json.dumps(
                context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
            )
            + "\n"
        ).encode("ascii")
    return subprocess.run(
        [
            sys.executable,
            "-I",
            str(repo / "proofs/m1/evidence/validate-negative-mutation.py"),
            PROTOCOL,
        ],
        check=False,
        input=raw,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        cwd=repo,
    )


def expect_pass(repo: Path, context: dict[str, Any], description: str) -> None:
    payload = json.dumps(
        context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")
    expected = (
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(payload)}\n"
    ).encode("ascii")
    result = invoke(repo, context, payload + b"\n")
    if result.returncode != 0 or result.stdout != expected:
        fail(
            f"{description} was not accepted\n{result.stdout.decode(errors='replace')}"
        )


def expect_rejected(
    repo: Path,
    root: Path,
    source_closure_sha256: str,
    row: tuple[str, ...],
    name: str,
    expected: str,
    mutation: FixtureMutation,
) -> None:
    fixture = root / name
    fixture.mkdir()
    _, context = build_run(repo, fixture, row, source_closure_sha256)
    mutation(fixture / "run", context, row)
    result = invoke(repo, context)
    output = result.stdout.decode(errors="replace")
    if result.returncode == 0 or expected not in output:
        fail(
            f"{name} was not rejected with {expected!r} "
            f"(status={result.returncode})\n{output}"
        )
    shutil.rmtree(fixture)


def edit_transcript(
    run: Path,
    context: dict[str, Any],
    row: tuple[str, ...],
    suffix: str,
    old: str,
    new: str,
) -> None:
    name = row[0]
    path = run / f"{name}.{suffix}.transcript"
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        fail(f"hostile fixture anchor drifted: {suffix}/{old}")
    path.write_text(text.replace(old, new), encoding="utf-8")
    refresh_result(run, name)
    refresh_context(context)


def edit_run_value(run: Path, key: str, value: str) -> None:
    values = parse_kv(run / "RUN_IDENTITY")
    values[key] = value
    write_kv(run / "RUN_IDENTITY", RUN_KEYS, values)


def edit_result_value(
    run: Path, context: dict[str, Any], row: tuple[str, ...], key: str, value: str
) -> None:
    path = run / f"{row[0]}.result"
    values = parse_kv(path)
    values[key] = value
    write_kv(path, RESULT_KEYS, values)
    refresh_context(context)


def rebind_context(
    repo: Path,
    context: dict[str, Any],
    property_name: str,
    path_id: str,
) -> None:
    requirements = json.loads(
        (repo / "proofs/M1_REQUIREMENTS.json").read_text(encoding="utf-8")
    )
    properties = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    paths = {record["id"]: record for record in requirements["path_obligations"]}
    binding = context["binding"]
    binding["obligation_id"] = property_name
    binding["path_id"] = path_id
    binding["statement_sha256"] = digest_bytes(
        properties[property_name]["boundary"].encode("utf-8")
    )
    payload = {key: value for key, value in binding.items() if key != "binding_sha256"}
    binding["binding_sha256"] = canonical_digest(payload)
    context["path_resolution"] = {
        "availability": paths[path_id]["availability"],
        "id": path_id,
        "path": paths[path_id]["path"],
        "repository": "ferric",
        "source_identity_id": "source.ferric",
    }


def main() -> None:
    if len(sys.argv) not in (2, 3):
        fail(f"usage: {sys.argv[0]} REPO [REAL_RESULT]")
    repo = Path(sys.argv[1]).resolve(strict=True)
    active, rows = registry(repo)
    if len(rows) != 23 or not active:
        fail("M1 negative registry baseline drifted")
    artifact_row = rows[0]
    if artifact_row[0] != "artifact-manifest-commitment-digest":
        fail("M1 negative registry first-row identity drifted")
    row = next(selected for selected in rows if selected[0] == "batching-publish-once")

    with tempfile.TemporaryDirectory(prefix="ferric-m1-validator-policy.") as scratch:
        root = Path(scratch)
        source_identity = exact_source_closure(repo, root)
        baseline_root = root / "baseline"
        baseline_root.mkdir()
        baseline_result, baseline_context = build_run(
            repo, baseline_root, row, source_identity
        )
        expect_pass(repo, baseline_context, "canonical negative-mutation fixture")
        if baseline_result.name != f"{row[0]}.result":
            fail("baseline result identity drifted")
        artifact_root = root / "artifact-manifest-baseline"
        artifact_root.mkdir()
        artifact_result, artifact_context = build_run(
            repo, artifact_root, artifact_row, source_identity
        )
        expect_pass(repo, artifact_context, "canonical artifact-manifest mutation fixture")
        if artifact_result.name != f"{artifact_row[0]}.result":
            fail("artifact-manifest result identity drifted")
        model_row = next(
            selected
            for selected in rows
            if selected[0] == "model-bundle-record-binding"
        )
        model_root = root / "model-bundle-baseline"
        model_root.mkdir()
        model_result, model_context = build_run(
            repo, model_root, model_row, source_identity
        )
        expect_pass(repo, model_context, "canonical model-bundle mutation fixture")
        if model_result.name != f"{model_row[0]}.result":
            fail("model-bundle result identity drifted")
        target_row = next(
            selected
            for selected in rows
            if selected[0] == "target-catalog-processor-features"
        )
        target_root = root / "target-catalog-baseline"
        target_root.mkdir()
        target_result, target_context = build_run(
            repo, target_root, target_row, source_identity
        )
        expect_pass(repo, target_context, "canonical target-catalog mutation fixture")
        if target_result.name != f"{target_row[0]}.result":
            fail("target-catalog result identity drifted")
        sampler_row = next(
            selected
            for selected in rows
            if selected[0] == "sampler-lowest-id-publication"
        )
        sampler_root = root / "sampler-baseline"
        sampler_root.mkdir()
        sampler_result, sampler_context = build_run(
            repo, sampler_root, sampler_row, source_identity
        )
        expect_pass(repo, sampler_context, "canonical sampler mutation fixture")
        if sampler_result.name != f"{sampler_row[0]}.result":
            fail("sampler result identity drifted")
        lifetime_row = next(
            selected
            for selected in rows
            if selected[0] == "kv-terminal-release-exact-epoch"
        )
        lifetime_root = root / "lifetime-baseline"
        lifetime_root.mkdir()
        lifetime_result, lifetime_context = build_run(
            repo, lifetime_root, lifetime_row, source_identity
        )
        expect_pass(repo, lifetime_context, "canonical lifetime mutation fixture")
        if lifetime_result.name != f"{lifetime_row[0]}.result":
            fail("lifetime result identity drifted")
        operator_row = next(
            selected
            for selected in rows
            if selected[0] == "operator-declared-profile-effect"
        )
        operator_root = root / "operator-baseline"
        operator_root.mkdir()
        operator_result, operator_context = build_run(
            repo, operator_root, operator_row, source_identity
        )
        edit_transcript(
            operator_root / "run",
            operator_context,
            operator_row,
            "verus",
            "due to 1 previous error\n",
            "due to 1 previous error; 1 warning emitted\n",
        )
        expect_pass(repo, operator_context, "canonical operator mutation fixture")
        if operator_result.name != f"{operator_row[0]}.result":
            fail("operator result identity drifted")
        operator_ten_root = root / "operator-ten-warnings-baseline"
        operator_ten_root.mkdir()
        _, operator_ten_context = build_run(
            repo, operator_ten_root, operator_row, source_identity
        )
        edit_transcript(
            operator_ten_root / "run",
            operator_ten_context,
            operator_row,
            "verus",
            "due to 1 previous error\n",
            "due to 1 previous error; 10 warnings emitted\n",
        )
        expect_pass(
            repo,
            operator_ten_context,
            "canonical operator ten-warning mutation fixture",
        )
        kernel_rows = [
            next(selected for selected in rows if selected[0] == name)
            for name in (
                "kernel-memory-read-initialization",
                "kernel-race-conflict",
                "kernel-resource-workitem-bound",
            )
        ]
        for selected in kernel_rows:
            kernel_root = root / f"{selected[0]}-baseline"
            kernel_root.mkdir()
            kernel_result, kernel_context = build_run(
                repo, kernel_root, selected, source_identity
            )
            expect_pass(
                repo,
                kernel_context,
                f"canonical {selected[0]} mutation fixture",
            )
            if kernel_result.name != f"{selected[0]}.result":
                fail(f"{selected[0]} result identity drifted")

        cases: list[tuple[str, str, FixtureMutation]] = [
            (
                "fabricated-transcript",
                "did not compile its selected package",
                lambda run, context, selected: (
                    edit_transcript(
                        run,
                        context,
                        selected,
                        "verus",
                        "   Compiling proc-macro2 v1.0.107\n",
                        "",
                    )
                    or edit_transcript(
                        run,
                        context,
                        selected,
                        "verus",
                        "verification results:: 2044 verified, 0 errors\n",
                        "",
                    )
                    or edit_transcript(
                        run,
                        context,
                        selected,
                        "verus",
                        f"   Compiling {selected[4]} v0.1.0 ",
                        "fabricated ",
                    )
                ),
            ),
            (
                "compile-only-error",
                "ordinary compilation was not clean",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "compile",
                    "    Checking quote v1.0.47\n",
                    "error: compile-only failure\n",
                ),
            ),
            (
                "wrong-function",
                "header, order, or trailing newline drifted",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    f"VERUS_FUNCTION={selected[9]}",
                    "VERUS_FUNCTION=wrong_function",
                ),
            ),
            (
                "wrong-module",
                "header, order, or trailing newline drifted",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    f"VERUS_MODULE={selected[8]}",
                    "VERUS_MODULE=wrong_module",
                ),
            ),
            (
                "wrong-marker",
                "lacks its exact proof diagnostic",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    "error: postcondition not satisfied",
                    "error: assertion failed",
                ),
            ),
            (
                "wrong-terminal-count",
                "no exact rejected terminal result",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    "due to 1 previous error",
                    "due to 2 previous errors",
                ),
            ),
            (
                "unbound-proof-diagnostic",
                "not bound to its selected source",
                lambda run, context, selected: (
                    edit_transcript(
                        run,
                        context,
                        selected,
                        "verus",
                        "verification results:: 0 verified, 1 errors",
                        "error: postcondition not satisfied\n"
                        "    = note: fabricated unbound diagnostic\n\n"
                        "verification results:: 0 verified, 1 errors",
                    )
                    or edit_transcript(
                        run,
                        context,
                        selected,
                        "verus",
                        "due to 1 previous error",
                        "due to 2 previous errors",
                    )
                ),
            ),
            (
                "wrong-run-commit",
                "source commit, tree, or closure drifted",
                lambda run, _context, _selected: edit_run_value(
                    run, "FERRIC_COMMIT", "a" * 40
                ),
            ),
            (
                "wrong-run-tree",
                "source commit, tree, or closure drifted",
                lambda run, _context, _selected: edit_run_value(
                    run, "FERRIC_TREE", "b" * 40
                ),
            ),
            (
                "wrong-run-closure",
                "source commit, tree, or closure drifted",
                lambda run, _context, _selected: edit_run_value(
                    run, "FERRIC_SOURCE_CLOSURE_SHA256", "c" * 64
                ),
            ),
            (
                "wrong-context-closure",
                "not the qualified source identity",
                lambda _run, context, _selected: context["sources"][1].__setitem__(
                    "source_closure_sha256", "d" * 64
                ),
            ),
            (
                "wrong-verus-closure",
                "Verus closure identity drifted",
                lambda run, _context, _selected: edit_run_value(
                    run, "VERUS_CLOSURE_SHA256", "e" * 64
                ),
            ),
            (
                "timeout",
                "VERUS_EXIT_STATUS",
                lambda run, context, selected: edit_result_value(
                    run, context, selected, "VERUS_EXIT_STATUS", "124"
                ),
            ),
            (
                "cross-row-replay",
                "incomplete for its bound property and path",
                lambda _run, context, _selected: rebind_context(
                    repo, context, "request_isolated", "isolation-proof"
                ),
            ),
            (
                "companion-path-escape",
                "path escaped or drifted",
                lambda run, context, selected: edit_result_value(
                    run, context, selected, "VERUS_TRANSCRIPT", "../replay"
                ),
            ),
            (
                "malformed-count",
                "malformed batching-publish-once Verus transcript size",
                lambda run, context, selected: edit_result_value(
                    run, context, selected, "VERUS_TRANSCRIPT_SIZE", "01"
                ),
            ),
            (
                "artifact-substitution",
                "bytes do not match their context identity",
                lambda _run, context, _selected: context["artifact"].__setitem__(
                    "sha256", "f" * 64
                ),
            ),
            (
                "unknown-result-field",
                "record count drifted",
                lambda run, context, selected: (
                    (run / f"{selected[0]}.result").write_text(
                        (run / f"{selected[0]}.result").read_text(encoding="ascii")
                        + "UNKNOWN=value\n",
                        encoding="ascii",
                    ),
                    refresh_context(context),
                ),
            ),
            (
                "reordered-result",
                "field order or identity drifted",
                lambda run, context, selected: (
                    (run / f"{selected[0]}.result").write_text(
                        "\n".join(
                            reversed(
                                (run / f"{selected[0]}.result")
                                .read_text(encoding="ascii")
                                .splitlines()
                            )
                        )
                        + "\n",
                        encoding="ascii",
                    ),
                    refresh_context(context),
                ),
            ),
            (
                "missing-companion",
                "run file roster is incomplete",
                lambda run, _context, selected: (
                    run / f"{selected[0]}.compile.transcript"
                ).unlink(),
            ),
            (
                "trailing-selected-record",
                "duplicate or unknown rows",
                lambda run, _context, _selected: (
                    run / "selected-foundations"
                ).write_text("unknown|row\n", encoding="ascii"),
            ),
            (
                "anchor-substitution",
                "current anchor does not reconstruct",
                lambda run, context, selected: (
                    lambda path, values: (
                        values.__setitem__("ANCHOR_SHA256", "a" * 64),
                        write_kv(path, tuple(values), values),
                        refresh_result(run, selected[0]),
                        refresh_context(context),
                    )
                )(
                    run / f"{selected[0]}.mutation",
                    parse_kv(run / f"{selected[0]}.mutation"),
                ),
            ),
        ]
        for name, expected, mutation in cases:
            expect_rejected(repo, root, source_identity, row, name, expected, mutation)
        expect_rejected(
            repo,
            root,
            source_identity,
            operator_row,
            "operator-selector-substitution",
            "marker binding drifted",
            lambda run, context, selected: (
                lambda path, values: (
                    values.__setitem__(
                        "VERUS_FUNCTION", "bind_declared_operation_kernel_plan"
                    ),
                    write_kv(path, tuple(values), values),
                    refresh_result(run, selected[0]),
                    refresh_context(context),
                )
            )(
                run / f"{selected[0]}.mutation",
                parse_kv(run / f"{selected[0]}.mutation"),
            ),
        )
        expect_rejected(
            repo,
            root,
            source_identity,
            operator_row,
            "operator-invalid-warning-count",
            "no exact rejected terminal result",
            lambda run, context, selected: edit_transcript(
                run,
                context,
                selected,
                "verus",
                "due to 1 previous error\n",
                "due to 1 previous error; 0 warnings emitted\n",
            ),
        )
        expect_rejected(
            repo,
            root,
            source_identity,
            operator_row,
            "operator-leading-zero-warning-count",
            "no exact rejected terminal result",
            lambda run, context, selected: edit_transcript(
                run,
                context,
                selected,
                "verus",
                "due to 1 previous error\n",
                "due to 1 previous error; 01 warnings emitted\n",
            ),
        )

        incomplete_root = root / "incomplete-property-product"
        incomplete_root.mkdir()
        incomplete_row = next(
            selected for selected in rows if selected[0] == "graph-operator-order"
        )
        _, incomplete_context = build_run(
            repo, incomplete_root, incomplete_row, source_identity
        )
        incomplete = invoke(repo, incomplete_context)
        if (
            incomplete.returncode == 0
            or b"incomplete for its bound property and path" not in incomplete.stdout
        ):
            fail("incomplete multi-row property mutation product was not rejected")

        duplicate_payload = json.dumps(
            baseline_context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        )
        duplicate_payload = duplicate_payload.replace(
            '"format":"ferric.m1-evidence-index.v1"',
            '"format":"ferric.m1-evidence-index.v1","format":"duplicate"',
            1,
        )
        duplicate = invoke(repo, baseline_context, (duplicate_payload + "\n").encode())
        if duplicate.returncode == 0 or b"duplicate JSON key" not in duplicate.stdout:
            fail("duplicate context field was not rejected")
        trailing = invoke(
            repo,
            baseline_context,
            (
                json.dumps(
                    baseline_context,
                    ensure_ascii=True,
                    separators=(",", ":"),
                    sort_keys=True,
                )
                + "\n\n"
            ).encode(),
        )
        if trailing.returncode == 0 or b"one trailing newline" not in trailing.stdout:
            fail("trailing validator context was not rejected")

        if len(sys.argv) == 3:
            real_result = Path(sys.argv[2]).resolve(strict=True)
            real_name = real_result.name.removesuffix(".result")
            real_row = next((record for record in rows if record[0] == real_name), None)
            if real_row is None or real_result.name != f"{real_name}.result":
                fail("real result does not name one registered mutation")
            real_context = make_context(repo, real_result, real_row, source_identity)
            expect_pass(repo, real_context, "real canonical runner artifact")

    print(
        f"PASS: M1 negative validator accepted its canonical fixtures and rejected "
        f"{len(cases) + 4} hostile artifacts"
    )


if __name__ == "__main__":
    main()
