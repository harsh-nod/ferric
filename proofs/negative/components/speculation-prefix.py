#!/usr/bin/env python3
"""Break maximal greedy-prefix acceptance in the executable loop."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/speculation.rs"
source = path.read_text(encoding="utf-8")
old = "        && draft_tokens[accepted] == target_choices[accepted]"
new = "        && draft_tokens[accepted] != target_choices[accepted]"
if source.count(old) != 1:
    raise SystemExit("speculation prefix mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/speculation.rs")
print("MUTATION=accept-mismatching-draft-prefix")
