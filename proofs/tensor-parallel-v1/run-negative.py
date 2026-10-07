#!/usr/bin/env python3
"""Run actual-body TP mutations in an exclusively owned remote source copy.

This developer check emits no qualification or source-admission authority.
The caller must first run the positive exact-source module proof in the same
strict dedicated target, and must not run other jobs against this source copy.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess


MUTATIONS = {
    "wrong_projection_axis": (
        "full_columns, TensorParallelMatrixModeV1::ColumnParallel)",
        "full_columns, TensorParallelMatrixModeV1::RowParallelSum)",
    ),
    "wrong_source_byte_offset": (
        "let offset = (row * u64::from(self.source_columns) + u64::from(self.columns.start)) * 2;",
        "let offset = (row * u64::from(self.source_columns)) * 2;",
    ),
    "duplicate_rank": ("if self.arrived & bit != 0 {", "if false {"),
    "missing_rank": ("if self.arrived != all {", "if false {"),
    "invalid_model": ("if model.validate().is_err() {", "if false {"),
    "wrong_kv_head_mapping": ("start: kv_heads.start * 128,", "start: 0,"),
    "wrong_group": (
        "if key.group_id != self.group_id || !same_model_role(key.model_role, self.model_role) {",
        "if false {",
    ),
    "epoch_overflow": ("if self.epoch == u64::MAX {", "if false {"),
}


def run_bounded(command, *, cwd, env, timeout):
    process = subprocess.Popen(command, cwd=cwd, env=env, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, text=True, start_new_session=True)
    try:
        output, _ = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.communicate()
        raise
    return subprocess.CompletedProcess(command, process.returncode, output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repo", type=Path)
    parser.add_argument("verus", type=Path)
    parser.add_argument("target", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve(strict=True)
    verus = args.verus.resolve(strict=True)
    target = args.target.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    source = repo / "crates/ferric-engine/src/tensor_parallel.rs"
    original = source.read_text()
    digest = hashlib.sha256(original.encode()).hexdigest()
    (output / "original.rs").write_text(original)
    env = dict(os.environ, VERUS_Z3_PATH=str(verus / "z3"),
               RUSTC_BOOTSTRAP="fe2o3_device,fe2o3_macros", CARGO_TERM_COLOR="never")
    command = [str(verus / "cargo-verus"), "build", "-p", "ferric-engine",
               "--locked", "--release", "--target-dir", str(target),
               "--fwd-verus-args-to", "roots", "-j", "1", "--lib", "--",
               "--no-cheating", "--output-json", "--verify-only-module", "tensor_parallel"]
    results = []
    try:
        for name, (before, after) in MUTATIONS.items():
            if original.count(before) != 1:
                raise RuntimeError(f"{name}: actual-body selector is not unique")
            mutant = original.replace(before, after, 1)
            source.write_text(mutant)
            (output / f"{name}.rs").write_text(mutant)
            clean = run_bounded(["cargo", "clean", "-p", "ferric-engine", "--release",
                                 "--target-dir", str(target)], cwd=repo, env=env, timeout=120)
            (output / f"{name}.clean.log").write_text(clean.stdout)
            clean.check_returncode()
            run = run_bounded(command, cwd=repo, env=env, timeout=600)
            (output / f"{name}.log").write_text(run.stdout)
            intended = any(marker in run.stdout for marker in [
                "postcondition not satisfied", "assertion failed",
                "possible arithmetic underflow/overflow",
                "loop invariant not satisfied",
            ])
            if run.returncode == 0 or not intended:
                raise RuntimeError(f"{name}: failed to reject at a Verus property obligation")
            results.append({"name": name, "status": "rejected", "exit_status": run.returncode,
                            "source_sha256": hashlib.sha256(mutant.encode()).hexdigest()})
            print(f"PASS {name}: actual body rejected", flush=True)
    finally:
        source.write_text(original)
        if hashlib.sha256(source.read_bytes()).hexdigest() != digest:
            raise RuntimeError("original source restoration failed")
        (output / "results.json").write_text(json.dumps({
            "original_source_sha256": digest, "command": command,
            "authority": "none", "results": results,
        }, indent=2) + "\n")


if __name__ == "__main__":
    main()
