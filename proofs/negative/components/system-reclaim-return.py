#!/usr/bin/env python3
"""Hide a successful Engine reclaim result while preserving Rust typing."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = "                Ok(Some(request))"
new = "                Ok(None)"
if source.count(old) != 1:
    raise SystemExit("system reclaim return mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=hide-successful-engine-reclaim-result")
