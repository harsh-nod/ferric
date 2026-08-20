#!/usr/bin/env python3
"""Accept a stale KV page generation and reject the current one."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = "        if slot.generation != page.generation { return Err(KvError::StalePage(page)); }"
new = "        if slot.generation == page.generation { return Err(KvError::StalePage(page)); }"
if source.count(old) != 1:
    raise SystemExit("KV stale-page mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=reverse-page-generation-check")
