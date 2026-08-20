#!/usr/bin/env python3
"""Lose the exact retained token count during KV rollback."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = "        self.pages[page_index].initialized_tokens = tail_tokens;"
new = "        self.pages[page_index].initialized_tokens = 0;"
if source.count(old) != 1:
    raise SystemExit("KV rollback-tail mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=discard-retained-rollback-tail")
