#!/usr/bin/env python3
"""Discard the Engine read span while preserving Rust typing."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = "let read_result = self.kv.validate_read(request, logical_offset, span);"
new = "let read_result = self.kv.validate_read(request, logical_offset, 0);"
if source.count(old) != 1:
    raise SystemExit("system read routing mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=discard-engine-read-span")
