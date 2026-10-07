#!/usr/bin/env python3
"""Alias the executable draft weight-role byte to the target role."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-build/src/weight_stream.rs"
source = path.read_text(encoding="utf-8")
old = """const fn role_code(role: Qwen3ModelRole) -> (code: u8)
    ensures code == role_code_spec(role),
{
    match role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    }
}
"""
new = old.replace("Qwen3ModelRole::Draft06B => 2", "Qwen3ModelRole::Draft06B => 1")
if source.count(old) != 1:
    raise SystemExit("canonical weight role-byte mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-build/src/weight_stream.rs")
print("MUTATION=canonical-weight-role-byte")
print("CLAUSE=exact-draft-role-byte")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
