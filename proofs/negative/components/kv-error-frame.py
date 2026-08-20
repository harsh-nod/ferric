#!/usr/bin/env python3
"""Mutate KV state before returning a context-bound error."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/cache.rs"
source = path.read_text(encoding="utf-8")
old = "        if new_resident > self.max_context_tokens { return Err(KvError::ContextExceeded); }"
new = """        if new_resident > self.max_context_tokens {
            self.requests[request_index].live = false;
            return Err(KvError::ContextExceeded);
        }"""
if source.count(old) != 1:
    raise SystemExit("KV error frame mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/cache.rs")
print("MUTATION=modify-live-bit-before-context-error")
