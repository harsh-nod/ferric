#!/usr/bin/env python3
"""Break the actual greedy correction body while preserving its contract."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/speculation.rs"
source = path.read_text(encoding="utf-8")
old = "let correction_or_bonus = target_choices[accepted];"
new = "let correction_or_bonus = target_choices[0];"
if source.count(old) != 1:
    raise SystemExit("speculation mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/speculation.rs")
print("MUTATION=target_choices[accepted]->target_choices[0]")
