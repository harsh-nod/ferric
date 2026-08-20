#!/usr/bin/env python3
"""Make an executing cancellation dispatchable before exact completion."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/scheduler.rs"
source = path.read_text(encoding="utf-8")
old = """    {
        self.slots[slot_index].state = RequestState::Retiring;
        assert(self.retired_slot_refines(old(self), _request)) by {
"""
new = """    {
        self.slots[slot_index].state = RequestState::Ready;
        assert(self.retired_slot_refines(old(self), _request)) by {
"""
if source.count(old) != 1:
    raise SystemExit("scheduler cancellation early-ready mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/scheduler.rs")
print("MUTATION=cancel-executing-request-into-ready")
