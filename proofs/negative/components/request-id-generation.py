#!/usr/bin/env python3
"""Break the executable RequestId generation constructor field."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/identity.rs"
source = path.read_text(encoding="utf-8")
old = "        Self { slot, generation }"
new = "        Self { slot, generation: 0 }"
if source.count(old) != 1:
    raise SystemExit("request identity generation mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/identity.rs")
print("MUTATION=request-generation-is-zero")
