#!/usr/bin/env python3
"""Advance residency without initializing the physical token prefix."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/paged_kv_refinement.rs"
source = path.read_text(encoding="utf-8")
old = "    state.page_slots[page.index as usize].initialized_prefix += 1;"
new = "    state.page_slots[page.index as usize].initialized_prefix += 0;"
if source.count(old) != 1:
    raise SystemExit("KV write-prefix mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/paged_kv_refinement.rs")
print("MUTATION=kv-write-prefix")
print("CLAUSE=initialized-prefix-advance")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
