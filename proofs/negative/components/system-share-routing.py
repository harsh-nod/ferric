#!/usr/bin/env python3
"""Reverse Engine committed-prefix source and target while preserving Rust typing."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = ".share_committed_prefix(source, target, token_count)"
new = ".share_committed_prefix(target, source, token_count)"
if source.count(old) != 1:
    raise SystemExit("system share routing mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=reverse-engine-prefix-share-routing")
