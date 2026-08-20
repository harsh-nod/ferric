#!/usr/bin/env python3
"""Accept a stale KV request generation and reject the current one."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = """        if slot.generation != request.generation {
            return Err(KvError::StaleRequestGeneration {"""
new = """        if slot.generation == request.generation {
            return Err(KvError::StaleRequestGeneration {"""
if source.count(old) != 1:
    raise SystemExit("KV stale-request mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=reverse-request-generation-check")
