#!/usr/bin/env python3
"""Replace strict argmax improvement with last-token tie selection."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/m1_completion.rs"
source = path.read_text(encoding="utf-8")
old = "        if scores[index as usize] > scores[best as usize] {"
new = "        if scores[index as usize] >= scores[best as usize] {"
if source.count(old) != 1:
    raise SystemExit("sampler lowest-ID tie mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/m1_completion.rs")
print("MUTATION=sampler-lowest-id-publication")
print("CLAUSE=lowest-token-id-tie-breaking")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
