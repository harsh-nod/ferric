#!/usr/bin/env python3
"""Treat a sealed KV tail as writable capacity."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = "                    (0, slot.initialized_tokens)"
new = "                    (self.page_tokens, slot.initialized_tokens)"
if source.count(old) != 1:
    raise SystemExit("KV sealed copy-on-write mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=reuse-sealed-tail-as-writable-capacity")
