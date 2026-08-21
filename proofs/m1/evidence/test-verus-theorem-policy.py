#!/usr/bin/env python3
"""Exercise canonical and hostile artifacts against the M1 theorem validator."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


PROTOCOL = "ferric.m1-validator.verus-theorem.v1"
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
VERUS_COMMIT = "b677dd5a766f25f56e9aa1e32621aa4e53304b47"
RUN_KEYS = (
    "FORMAT",
    "FERRIC_COMMIT",
    "FERRIC_TREE",
    "FERRIC_SOURCE_CLOSURE_SHA256",
    "VERUS_VERSION",
    "VERUS_SHA256",
    "VERUS_CLOSURE_MANIFEST_SHA256",
    "VERUS_CLOSURE_SHA256",
    "VERIFIED_MODULES_SHA256",
    "REGISTRY_SHA256",
    "RUNNER_SHA256",
    "AUTHORITY",
    "NONCLAIM",
)
RESULT_KEYS = (
    "FORMAT",
    "THEOREM",
    "RUN_IDENTITY_SHA256",
    "ACTIVE_FOUNDATIONS_SHA256",
    "SELECTED_FOUNDATIONS_SHA256",
    "VERUS_CLOSURE_TRANSCRIPT_SHA256",
    "THEOREM_RECORD",
    "THEOREM_RECORD_SHA256",
    "THEOREM_RECORD_SIZE",
    "COMPILE_TRANSCRIPT",
    "COMPILE_TRANSCRIPT_SHA256",
    "COMPILE_TRANSCRIPT_SIZE",
    "COMPILE_EXIT_STATUS",
    "VERUS_SUMMARY",
    "VERUS_SUMMARY_SHA256",
    "VERUS_SUMMARY_SIZE",
    "VERUS_TRANSCRIPT",
    "VERUS_TRANSCRIPT_SHA256",
    "VERUS_TRANSCRIPT_SIZE",
    "VERUS_EXIT_STATUS",
    "RESULT",
)
THEOREM_KEYS = (
    "FORMAT",
    "THEOREM",
    "FOUNDATION",
    "OPEN_ASSURANCE_PROPERTY",
    "OPEN_PATH_OBLIGATION",
    "VERUS_PACKAGE",
    "VERUS_SOURCE",
    "VERUS_MODULE",
    "VERUS_FUNCTION",
    "COMPILER_PATH",
    "VERIFIED_MODULES_SHA256",
    "SOURCE_SHA256",
    "FUNCTION_SOURCE_IDENTITY_SHA256",
    "CARGO_CHECK",
    "VERUS_RESULT",
)
SUMMARY_KEYS = (
    "FORMAT",
    "COMPILER_PATH",
    "TRANSCRIPT_SHA256",
    "VERIFIED_COUNT",
    "DETAILS_COUNT",
    "IS_VERIFYING_ENTIRE_CRATE",
    "ENCOUNTERED_ERROR",
    "ENCOUNTERED_VIR_ERROR",
    "ERRORS",
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
        (repo / "proofs/m1/theorem/REQUIRED_FOUNDATIONS")
        .read_text(encoding="ascii")
        .splitlines()
    )
    rows = [tuple(line.removeprefix("theorem=").split("|")) for line in lines[1:]]
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
        fail(f"cannot build theorem fixture source identity\n{result.stdout}")
    return digest_file(output)


def closure_identity(repo: Path) -> tuple[str, str, bytes]:
    manifest = (repo / "proofs/verus/VERUS_CLOSURE_MANIFEST").read_bytes()
    fields = dict(
        line.split("=", 1)
        for line in manifest.decode("ascii").splitlines()
        if "=" in line
    )
    transcript = (
        "PASS: pinned Verus release closure matched "
        f"({fields['file-count']} files, {fields['total-bytes']} bytes)\n"
    ).encode("ascii")
    return digest_bytes(manifest), fields["closure-sha256"], transcript


def rust_toolchain(repo: Path) -> str:
    for line in (repo / "rust-toolchain.toml").read_text(encoding="ascii").splitlines():
        if line.startswith('channel = "'):
            return line.removeprefix('channel = "').removesuffix('"') + (
                "-x86_64-unknown-linux-gnu"
            )
    fail("fixture Rust toolchain identity is unavailable")


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
    paths = {record["id"]: record for record in requirements["path_obligations"]}
    name, _, property_name, path_id, *_ = row
    artifact = {
        "id": f"artifact.theorem.{name}",
        "kind": "TheoremTranscript",
        "path": f"runs/{result.name}",
        "sha256": digest_file(result),
        "size_bytes": result.stat().st_size,
    }
    binding = {
        "artifact_id": artifact["id"],
        "evidence_kind": "verus-theorem",
        "id": f"binding.theorem.{name}",
        "obligation_class": "Assurance",
        "obligation_id": property_name,
        "path_id": path_id,
        "profile_id": "composition",
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
        ("verus-closure.transcript", "VERUS_CLOSURE_TRANSCRIPT_SHA256", None),
        (result["THEOREM_RECORD"], "THEOREM_RECORD_SHA256", "THEOREM_RECORD_SIZE"),
        (
            result["COMPILE_TRANSCRIPT"],
            "COMPILE_TRANSCRIPT_SHA256",
            "COMPILE_TRANSCRIPT_SIZE",
        ),
        (result["VERUS_SUMMARY"], "VERUS_SUMMARY_SHA256", "VERUS_SUMMARY_SIZE"),
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


def structured_root(repo: Path, compiler_path: str) -> dict[str, Any]:
    return {
        "func-details": {
            compiler_path: {
                "obligation_proof_notes": [],
                "failed_proof_notes": [],
            }
        },
        "verification-results": {
            "is-verifying-entire-crate": False,
            "encountered-error": False,
            "encountered-vir-error": False,
            "verified": 1,
            "errors": 0,
        },
        "verus": {
            "commit": VERUS_COMMIT,
            "version": (repo / "proofs/verus/VERUS_VERSION")
            .read_text(encoding="ascii")
            .strip(),
            "profile": "release",
            "toolchain": rust_toolchain(repo),
            "platform": {"arch": "x86_64", "os": "linux"},
        },
    }


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
        "FORMAT": "FERRIC-M1-POSITIVE-RUN-V1",
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
        "VERIFIED_MODULES_SHA256": digest_file(repo / "proofs/VERIFIED_MODULES"),
        "REGISTRY_SHA256": digest_file(repo / "proofs/m1/theorem/REQUIRED_FOUNDATIONS"),
        "RUNNER_SHA256": digest_file(repo / "proofs/m1/theorem/run-same-source.sh"),
        "AUTHORITY": "direct-verus-foundation-success-only",
        "NONCLAIM": "no-m1-property-path-or-roadmap-closure",
    }
    write_kv(run / "RUN_IDENTITY", RUN_KEYS, run_values)

    (
        name,
        foundation,
        property_name,
        path_id,
        package,
        source,
        module,
        function,
    ) = row
    compiler_path = f"{package.replace('-', '_')}::{module}::{function}"
    source_sha = digest_file(repo / source)
    function_source_identity = digest_bytes(
        f"FERRIC-M1-THEOREM-SOURCE-IDENTITY-V1|{source_sha}|{compiler_path}\n".encode(
            "ascii"
        )
    )
    theorem_values = {
        "FORMAT": "FERRIC-M1-POSITIVE-THEOREM-V1",
        "THEOREM": name,
        "FOUNDATION": foundation,
        "OPEN_ASSURANCE_PROPERTY": property_name,
        "OPEN_PATH_OBLIGATION": path_id,
        "VERUS_PACKAGE": package,
        "VERUS_SOURCE": source,
        "VERUS_MODULE": module,
        "VERUS_FUNCTION": function,
        "COMPILER_PATH": compiler_path,
        "VERIFIED_MODULES_SHA256": digest_file(repo / "proofs/VERIFIED_MODULES"),
        "SOURCE_SHA256": source_sha,
        "FUNCTION_SOURCE_IDENTITY_SHA256": function_source_identity,
        "CARGO_CHECK": "passed",
        "VERUS_RESULT": "proved",
    }
    write_kv(run / f"{name}.theorem", THEOREM_KEYS, theorem_values)
    compile_text = (
        "FORMAT=FERRIC-M1-POSITIVE-COMPILE-V1\n"
        f"CARGO_PACKAGE={package}\n"
        "COMMAND=cargo-check-locked-all-targets\n"
        "    Checking proc-macro2 v1.0.107\n"
        "    Checking quote v1.0.47\n"
        f"    Checking {package} v0.1.0 (/qualified/crates/{package})\n"
        "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.25s\n"
    )
    (run / f"{package}.compile.transcript").write_text(compile_text, encoding="utf-8")
    root_json = json.dumps(structured_root(repo, compiler_path), indent=2) + "\n"
    verus_text = (
        "FORMAT=FERRIC-M1-POSITIVE-VERUS-V1\n"
        f"THEOREM={name}\n"
        f"VERUS_PACKAGE={package}\n"
        f"VERUS_MODULE={module}\n"
        f"VERUS_FUNCTION={function}\n"
        "COMMAND=cargo-verus-build-locked-release-no-cheating-output-json-exact-function\n"
        f"   Compiling {package} v0.1.0 (/qualified/crates/{package})\n"
        f"note: verifying module {module} (selected functions)\n"
        f"{root_json}"
        "    Finished `release` profile [optimized] target(s) in 4.00s\n"
    )
    verus_path = run / f"{name}.verus.transcript"
    verus_path.write_text(verus_text, encoding="utf-8")
    summary = {
        "FORMAT": "FERRIC-M1-POSITIVE-OUTPUT-V1",
        "COMPILER_PATH": compiler_path,
        "TRANSCRIPT_SHA256": digest_file(verus_path),
        "VERIFIED_COUNT": "1",
        "DETAILS_COUNT": "1",
        "IS_VERIFYING_ENTIRE_CRATE": "false",
        "ENCOUNTERED_ERROR": "false",
        "ENCOUNTERED_VIR_ERROR": "false",
        "ERRORS": "0",
        "RESULT": "success",
    }
    write_kv(run / f"{name}.verus.summary", SUMMARY_KEYS, summary)
    result_values = {
        "FORMAT": "FERRIC-M1-POSITIVE-RESULT-V1",
        "THEOREM": name,
        "RUN_IDENTITY_SHA256": "0" * 64,
        "ACTIVE_FOUNDATIONS_SHA256": "0" * 64,
        "SELECTED_FOUNDATIONS_SHA256": "0" * 64,
        "VERUS_CLOSURE_TRANSCRIPT_SHA256": "0" * 64,
        "THEOREM_RECORD": f"{name}.theorem",
        "THEOREM_RECORD_SHA256": "0" * 64,
        "THEOREM_RECORD_SIZE": "1",
        "COMPILE_TRANSCRIPT": f"{package}.compile.transcript",
        "COMPILE_TRANSCRIPT_SHA256": "0" * 64,
        "COMPILE_TRANSCRIPT_SIZE": "1",
        "COMPILE_EXIT_STATUS": "0",
        "VERUS_SUMMARY": f"{name}.verus.summary",
        "VERUS_SUMMARY_SHA256": "0" * 64,
        "VERUS_SUMMARY_SIZE": "1",
        "VERUS_TRANSCRIPT": f"{name}.verus.transcript",
        "VERUS_TRANSCRIPT_SHA256": "0" * 64,
        "VERUS_TRANSCRIPT_SIZE": "1",
        "VERUS_EXIT_STATUS": "0",
        "RESULT": "proved",
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
            str(repo / "proofs/m1/evidence/validate-verus-theorem.py"),
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
    source_identity: str,
    row: tuple[str, ...],
    name: str,
    expected: str,
    mutation: FixtureMutation,
) -> None:
    fixture = root / name
    fixture.mkdir()
    _, context = build_run(repo, fixture, row, source_identity)
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
    path = run / (
        f"{row[0]}.{suffix}.transcript"
        if suffix == "verus"
        else f"{row[4]}.compile.transcript"
    )
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        fail(f"hostile theorem fixture anchor drifted: {suffix}/{old}")
    path.write_text(text.replace(old, new), encoding="utf-8")
    refresh_result(run, row[0])
    refresh_context(context)


def edit_run_value(run: Path, key: str, value: str) -> None:
    fields = parse_kv(run / "RUN_IDENTITY")
    fields[key] = value
    write_kv(run / "RUN_IDENTITY", RUN_KEYS, fields)


def edit_result_value(
    run: Path, context: dict[str, Any], row: tuple[str, ...], key: str, value: str
) -> None:
    path = run / f"{row[0]}.result"
    fields = parse_kv(path)
    fields[key] = value
    write_kv(path, RESULT_KEYS, fields)
    refresh_context(context)


def edit_theorem_value(
    run: Path, context: dict[str, Any], row: tuple[str, ...], key: str, value: str
) -> None:
    path = run / f"{row[0]}.theorem"
    fields = parse_kv(path)
    fields[key] = value
    write_kv(path, THEOREM_KEYS, fields)
    refresh_result(run, row[0])
    refresh_context(context)


def rebind_context(
    repo: Path, context: dict[str, Any], property_name: str, path_id: str
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
    if len(rows) != 10 or not active:
        fail("M1 positive-theorem registry baseline drifted")
    row = rows[0]
    with tempfile.TemporaryDirectory(prefix="ferric-m1-theorem-policy.") as scratch:
        root = Path(scratch)
        source_identity = exact_source_closure(repo, root)
        baseline_root = root / "baseline"
        baseline_root.mkdir()
        _, baseline_context = build_run(repo, baseline_root, row, source_identity)
        expect_pass(repo, baseline_context, "canonical Verus-theorem fixture")

        cases: list[tuple[str, str, FixtureMutation]] = [
            (
                "fabricated-success",
                "did not compile its selected package",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    f"   Compiling {selected[4]} v0.1.0",
                    "fabricated success",
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
                    f"VERUS_FUNCTION={selected[7]}",
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
                    f"VERUS_MODULE={selected[6]}",
                    "VERUS_MODULE=wrong_module",
                ),
            ),
            (
                "wrong-source",
                "source/function binding drifted",
                lambda run, context, selected: edit_theorem_value(
                    run,
                    context,
                    selected,
                    "VERUS_SOURCE",
                    "crates/ferric-spec/src/graph.rs",
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
                "wrong-context-commit",
                "not the qualified source identity",
                lambda _run, context, _selected: context["sources"][1].__setitem__(
                    "commit", "a" * 40
                ),
            ),
            (
                "wrong-context-tree",
                "not the qualified source identity",
                lambda _run, context, _selected: context["sources"][1].__setitem__(
                    "tree", "b" * 40
                ),
            ),
            (
                "wrong-context-closure",
                "not the qualified source identity",
                lambda _run, context, _selected: context["sources"][1].__setitem__(
                    "source_closure_sha256", "c" * 64
                ),
            ),
            (
                "wrong-verus-closure",
                "compiler closure identity drifted",
                lambda run, _context, _selected: edit_run_value(
                    run, "VERUS_CLOSURE_SHA256", "d" * 64
                ),
            ),
            (
                "admit-rejection",
                "proof or infrastructure error",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    f"note: verifying module {selected[6]} (selected functions)\n",
                    f"note: verifying module {selected[6]} (selected functions)\n"
                    "error: assume/admit not allowed with --no-cheating\n",
                ),
            ),
            (
                "nonzero-errors",
                "structured result is not an exact success",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '"errors": 0',
                    '"errors": 1',
                ),
            ),
            (
                "forged-success-field",
                "structured verification result fields drifted",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '"verification-results": {\n',
                    '"verification-results": {\n    "success": true,\n',
                ),
            ),
            (
                "missing-encountered-error",
                "structured verification result fields drifted",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '    "encountered-error": false,\n',
                    "",
                ),
            ),
            (
                "missing-encountered-vir-error",
                "structured verification result fields drifted",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '    "encountered-vir-error": false,\n',
                    "",
                ),
            ),
            (
                "zero-verified",
                "structured result is not an exact success",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '"verified": 1',
                    '"verified": 0',
                ),
            ),
            (
                "multiple-verified",
                "structured result is not an exact success",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '"verified": 1',
                    '"verified": 2',
                ),
            ),
            (
                "boolean-errors",
                "structured result is not an exact success",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '"errors": 0',
                    '"errors": false',
                ),
            ),
            (
                "boolean-verified",
                "structured result is not an exact success",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '"verified": 1',
                    '"verified": true',
                ),
            ),
            (
                "future-result-schema",
                "structured verification result fields drifted",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '"verification-results": {\n',
                    '"verification-results": {\n    "future-field": false,\n',
                ),
            ),
            (
                "duplicate-structured-key",
                "duplicate JSON key",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '    "errors": 0\n',
                    '    "errors": 0,\n    "errors": 0\n',
                ),
            ),
            (
                "status-mismatch",
                "VERUS_EXIT_STATUS",
                lambda run, context, selected: edit_result_value(
                    run, context, selected, "VERUS_EXIT_STATUS", "1"
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
                "timeout",
                "VERUS_EXIT_STATUS",
                lambda run, context, selected: edit_result_value(
                    run, context, selected, "VERUS_EXIT_STATUS", "124"
                ),
            ),
            (
                "selector-failure",
                "proof or infrastructure error",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    f"note: verifying module {selected[6]} (selected functions)\n",
                    f"note: verifying module {selected[6]} (selected functions)\n"
                    "error: could not find function selected_function\n",
                ),
            ),
            (
                "cargo-verus-compile-failure",
                "proof or infrastructure error",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    f"note: verifying module {selected[6]} (selected functions)\n",
                    f"note: verifying module {selected[6]} (selected functions)\n"
                    "could not compile selected package\n",
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
                "missing-companion",
                "run file roster is incomplete",
                lambda run, _context, selected: (
                    run / f"{selected[0]}.verus.summary"
                ).unlink(),
            ),
            (
                "extra-companion",
                "contains extras",
                lambda run, _context, _selected: (run / "extra").write_text(
                    "extra\n", encoding="ascii"
                ),
            ),
            (
                "path-escape",
                "path escaped or drifted",
                lambda run, context, selected: edit_result_value(
                    run, context, selected, "VERUS_TRANSCRIPT", "../replay"
                ),
            ),
            (
                "summary-substitution",
                "summary does not match structured output-json",
                lambda run, context, selected: (
                    lambda path, fields: (
                        fields.__setitem__("VERIFIED_COUNT", "2"),
                        write_kv(path, SUMMARY_KEYS, fields),
                        refresh_result(run, selected[0]),
                        refresh_context(context),
                    )
                )(
                    run / f"{selected[0]}.verus.summary",
                    parse_kv(run / f"{selected[0]}.verus.summary"),
                ),
            ),
            (
                "missing-function-details",
                "selected function is absent",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    f'"{selected[4].replace("-", "_")}::{selected[6]}::{selected[7]}"',
                    '"wrong::compiler::path"',
                ),
            ),
            (
                "extra-function-proof-note",
                "function detail has unresolved proof notes",
                lambda run, context, selected: edit_transcript(
                    run,
                    context,
                    selected,
                    "verus",
                    '"func-details": {\n',
                    '"func-details": {\n'
                    '    "forged::extra_function": {\n'
                    '      "obligation_proof_notes": ["forged extra claim"],\n'
                    '      "failed_proof_notes": []\n'
                    "    },\n",
                ),
            ),
            (
                "coverage-substitution",
                "source/function binding drifted",
                lambda run, context, selected: edit_theorem_value(
                    run,
                    context,
                    selected,
                    "VERIFIED_MODULES_SHA256",
                    "d" * 64,
                ),
            ),
            (
                "source-identity-substitution",
                "source/function binding drifted",
                lambda run, context, selected: edit_theorem_value(
                    run,
                    context,
                    selected,
                    "FUNCTION_SOURCE_IDENTITY_SHA256",
                    "e" * 64,
                ),
            ),
            (
                "source-sha-substitution",
                "source/function binding drifted",
                lambda run, context, selected: edit_theorem_value(
                    run,
                    context,
                    selected,
                    "SOURCE_SHA256",
                    "f" * 64,
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
                "artifact-substitution",
                "bytes do not match their context identity",
                lambda _run, context, _selected: context["artifact"].__setitem__(
                    "sha256", "f" * 64
                ),
            ),
        ]
        for name, expected, mutation in cases:
            expect_rejected(repo, root, source_identity, row, name, expected, mutation)

        incomplete_root = root / "incomplete-property-product"
        incomplete_root.mkdir()
        _, incomplete_context = build_run(
            repo, incomplete_root, rows[2], source_identity
        )
        incomplete = invoke(repo, incomplete_context)
        if (
            incomplete.returncode == 0
            or b"incomplete for its bound property and path" not in incomplete.stdout
        ):
            fail("incomplete multi-row theorem product was not rejected")

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
            fail("duplicate theorem context field was not rejected")
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
            fail("trailing theorem validator context was not rejected")

        if len(sys.argv) == 3:
            real_result = Path(sys.argv[2]).resolve(strict=True)
            real_name = real_result.name.removesuffix(".result")
            real_row = next((record for record in rows if record[0] == real_name), None)
            if real_row is None or real_result.name != f"{real_name}.result":
                fail("real theorem result does not name one registered row")
            real_context = make_context(repo, real_result, real_row, source_identity)
            expect_pass(repo, real_context, "real canonical Verus-theorem artifact")

    print(
        f"PASS: M1 theorem validator accepted its canonical fixture and rejected "
        f"{len(cases) + 3} hostile artifacts"
    )


if __name__ == "__main__":
    main()
