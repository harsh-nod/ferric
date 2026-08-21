#!/usr/bin/env python3
"""Write a successful request-local transition into a different batch slot."""

from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/continuous_batching.rs"
source = path.read_text(encoding="utf-8")
old = "    batch.replace(slot, updated);"
new = "    batch.replace(if slot == 0 { 1 } else { 0 }, updated);"
if source.count(old) != 1:
    raise SystemExit("isolation other-request-frame mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/continuous_batching.rs")
print("MUTATION=isolation-other-request-frame")
print("CLAUSE=other-request-frame")
