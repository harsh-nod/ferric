#!/usr/bin/env python3
"""Incorrectly admit a new window without its current-plus-lineage predecessor set."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "proofs/m1/physical_new_window.rs"
source = path.read_text(encoding="utf-8")
header = "pub fn check_m1_physical_new_window_precommit_v1("
old = """        && check_m1_physical_new_window_condition_v1(
            checks.current_historical_predecessor_set_exact,
        )
"""
if source.count(header) != 1:
    raise SystemExit("physical new-window precommit function anchor drifted")
prefix, tail = source.split(header, 1)
if tail.count(old) != 1:
    raise SystemExit("physical new-window predecessor-set mutation anchor drifted")
new = old.replace(
    "checks.current_historical_predecessor_set_exact",
    "M1PhysicalNewWindowConditionV1::Satisfied",
)
path.write_text(prefix + header + tail.replace(old, new), encoding="utf-8")
anchor = (header + old).encode("utf-8")
print("MUTATED_SOURCE=proofs/m1/physical_new_window.rs")
print("MUTATION=physical-new-window-predecessor-set")
print("CLAUSE=current-historical-predecessor-set")
print(f"ANCHOR_SHA256={hashlib.sha256(anchor).hexdigest()}")
