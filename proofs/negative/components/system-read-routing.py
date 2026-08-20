#!/usr/bin/env python3
"""Reverse Engine read offset and span while preserving Rust typing."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = "match self.kv.validate_read(request, logical_offset, span) {"
new = "match self.kv.validate_read(request, span, logical_offset) {"
if source.count(old) != 1:
    raise SystemExit("system read routing mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=reverse-engine-read-range-routing")
