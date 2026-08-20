#!/usr/bin/env python3
"""Reuse a reclaimed KV page without advancing its generation."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = "        let generation = self.pages[page_index].generation + 1;"
new = "        let generation = self.pages[page_index].generation;"
if source.count(old) != 1:
    raise SystemExit("KV page-generation mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=reuse-page-generation")
