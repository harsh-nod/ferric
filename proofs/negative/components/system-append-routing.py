#!/usr/bin/env python3
"""Discard the Engine tentative-append token count while preserving Rust typing."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = "let append_result = self.kv.append_tentative(request, token_count);"
new = "let append_result = self.kv.append_tentative(request, 0);"
if source.count(old) != 1:
    raise SystemExit("system append routing mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=discard-engine-append-token-count")
