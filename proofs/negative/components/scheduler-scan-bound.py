#!/usr/bin/env python3
"""Allow the deterministic dispatch scan to inspect more than C slots."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/scheduler.rs"
source = path.read_text(encoding="utf-8")
old = "        while scanned < C && selected < limit"
new = "        while scanned <= C && selected < limit"
if source.count(old) != 1:
    raise SystemExit("scheduler scan-bound mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/scheduler.rs")
print("MUTATION=scan-more-than-capacity")
