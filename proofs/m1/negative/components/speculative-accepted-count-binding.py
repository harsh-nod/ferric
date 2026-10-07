#!/usr/bin/env python3
"""Settle zero accepted draft tokens instead of the publication-derived count."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/speculative_step_composition.rs"
source = path.read_text(encoding="utf-8")
old = """    let kv_permit = match preflight_isolated_speculative_kv(
        batch,
        selected,
        other,
        index,
        accepted_draft_tokens,
        expected,
    ) {"""
new = old.replace("        accepted_draft_tokens,", "        0,")
if source.count(old) != 1:
    raise SystemExit("speculative accepted-count binding mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/speculative_step_composition.rs")
print("MUTATION=speculative-accepted-count-binding")
print("CLAUSE=publication-kv-accepted-count")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
