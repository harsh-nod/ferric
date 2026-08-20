#!/usr/bin/env python3
"""Dispatch when either fixed-capacity submission ring is already full."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/scheduler.rs"
source = path.read_text(encoding="utf-8")
old = "        if self.batch_len == C || self.member_len == C {"
new = "        if self.batch_len == C && self.member_len == C {"
if source.count(old) != 1:
    raise SystemExit("scheduler ring-bound mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/scheduler.rs")
print("MUTATION=dispatch-with-one-full-submission-ring")
