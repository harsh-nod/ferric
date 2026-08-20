#!/usr/bin/env python3
"""Accept a stale request generation in the actual retirement preflight."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/scheduler.rs"
source = path.read_text(encoding="utf-8")
old = """        let slot = self.slots[slot_index];
        if slot.generation != request.generation() {
            proof {
                reveal(retire_expected_error);
"""
new = """        let slot = self.slots[slot_index];
        if slot.generation == request.generation() {
            proof {
                reveal(retire_expected_error);
"""
if source.count(old) != 1:
    raise SystemExit("scheduler exact-rejection mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/scheduler.rs")
print("MUTATION=accept-stale-retirement-generation")
