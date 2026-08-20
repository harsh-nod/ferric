#!/usr/bin/env python3
"""Admit reads beyond the executable initialized-token boundary."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = "        if end > self.requests[request_index].resident_tokens { return Err(KvError::ReadOutOfBounds); }"
new = "        if end < self.requests[request_index].resident_tokens { return Err(KvError::ReadOutOfBounds); }"
if source.count(old) != 1:
    raise SystemExit("KV initialized-read mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=reverse-initialized-read-bound")
