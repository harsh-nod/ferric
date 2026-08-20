#!/usr/bin/env python3
"""Misroute Engine constructor bounds while preserving Rust typing."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = "let kv = match KvPool::new(page_count, page_tokens, max_context_tokens) {"
new = "let kv = match KvPool::new(page_count, max_context_tokens, page_tokens) {"
if source.count(old) != 1:
    raise SystemExit("system constructor routing mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=swap-engine-kv-constructor-bounds")
