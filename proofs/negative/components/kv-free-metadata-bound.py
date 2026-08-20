#!/usr/bin/env python3
"""Fail to consume one bounded KV free-list entry."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = """        self.free_len -= 1;
        self.free_bitmap[page_index] = false;"""
new = """        self.free_len -= 0;
        self.free_bitmap[page_index] = false;"""
if source.count(old) != 1:
    raise SystemExit("KV free-metadata mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=retain-consumed-free-list-entry")
