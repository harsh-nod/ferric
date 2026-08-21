#!/usr/bin/env python3
"""Substitute the draft step count for the target executable plan."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/graph.rs"
source = path.read_text(encoding="utf-8")
old = """pub const fn plan_step_count(role: Qwen3ModelRole) -> (count: u32)
    ensures count == plan_step_count_spec(role),
{
    match role {
        Qwen3ModelRole::Target8B => QWEN3_TARGET_PLAN_STEPS,
        Qwen3ModelRole::Draft06B => QWEN3_DRAFT_PLAN_STEPS,
    }
}"""
new = old.replace(
    "Qwen3ModelRole::Target8B => QWEN3_TARGET_PLAN_STEPS",
    "Qwen3ModelRole::Target8B => QWEN3_DRAFT_PLAN_STEPS",
)
if source.count(old) != 1:
    raise SystemExit("graph role-step-count mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/graph.rs")
print("MUTATION=graph-role-step-count")
print("CLAUSE=exact-role-step-count")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
