#!/usr/bin/env python3
"""Break the submitted-epoch to pending-batch accounting relation."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/scheduler.rs"
source = path.read_text(encoding="utf-8")
old = """        self.member_len += selected;
        self.cursor = slot_index;
        self.submitted = next_epoch;"""
new = """        self.member_len += selected;
        self.cursor = slot_index;
        self.submitted = next_epoch - 1;"""
if source.count(old) != 1:
    raise SystemExit("scheduler epoch-accounting mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/scheduler.rs")
print("MUTATION=retain-completed-epoch-after-dispatch")
