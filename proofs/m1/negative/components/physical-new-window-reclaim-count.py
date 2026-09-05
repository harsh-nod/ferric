#!/usr/bin/env python3
"""Compute reclaimed predecessors from the successor cardinality."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/m1_new_window_cardinality.rs"
source = path.read_text(encoding="utf-8")
header = "pub fn plan_m1_new_window_cardinality_v1("
old = "    let reclaim_count = match old_count.checked_sub(reuse_count) {\n"
new = "    let reclaim_count = match new_count.checked_sub(reuse_count) {\n"
if source.count(header) != 1:
    raise SystemExit("physical new-window cardinality planner anchor drifted")
prefix, tail = source.split(header, 1)
if tail.count(old) != 1:
    raise SystemExit("physical new-window reclaim-count mutation anchor drifted")
path.write_text(prefix + header + tail.replace(old, new), encoding="utf-8")
anchor = (header + old).encode("utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/m1_new_window_cardinality.rs")
print("MUTATION=physical-new-window-reclaim-count")
print("CLAUSE=reclaim-predecessor-difference")
print(f"ANCHOR_SHA256={hashlib.sha256(anchor).hexdigest()}")
