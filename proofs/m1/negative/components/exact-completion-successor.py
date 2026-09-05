#!/usr/bin/env python3
"""Accept a non-successor executable completion epoch."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/completion.rs"
source = path.read_text(encoding="utf-8")
old = "        Some(_) => Err(CompletionOrderError::NotExactNext),\n"
new = "        Some(_) => Ok(observed),\n"
if source.count(old) != 1:
    raise SystemExit("exact completion-successor mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/completion.rs")
print("MUTATION=exact-completion-successor")
print("CLAUSE=exact-successor-replay-skip-rejection")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
