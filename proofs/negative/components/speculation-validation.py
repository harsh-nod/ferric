#!/usr/bin/env python3
"""Reverse the executable greedy target-length validation."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/speculation.rs"
source = path.read_text(encoding="utf-8")
old = "    if target_choices.len() != expected {"
new = "    if target_choices.len() == expected {"
if source.count(old) != 1:
    raise SystemExit("speculation validation mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/speculation.rs")
print("MUTATION=reject-exact-target-choice-count")
