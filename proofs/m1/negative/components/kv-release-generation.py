#!/usr/bin/env python3
"""Release a retired page without advancing its physical generation."""

from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/paged_kv_refinement.rs"
source = path.read_text(encoding="utf-8")
old = "        generation: page.generation + 1,"
new = "        generation: page.generation,"
if source.count(old) != 1:
    raise SystemExit("KV release-generation mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/paged_kv_refinement.rs")
print("MUTATION=kv-release-generation")
print("CLAUSE=released-generation-advance")
