#!/usr/bin/env python3
"""Detach a KV request without advancing its generation."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = "        self.requests[request_index].generation += 1;"
new = "        self.requests[request_index].generation += 0;"
if source.count(old) != 1:
    raise SystemExit("KV request-generation mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=reuse-request-generation")
