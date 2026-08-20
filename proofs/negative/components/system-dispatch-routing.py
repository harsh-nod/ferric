#!/usr/bin/env python3
"""Disconnect Engine dispatch output from its successful result."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = "            Ok(batch) => Ok(batch),"
new = "            Ok(_batch) => Ok(None),"
if source.count(old) != 1:
    raise SystemExit("system dispatch routing mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=disconnect-engine-dispatch-result-from-output")
