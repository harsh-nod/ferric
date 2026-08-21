#!/usr/bin/env python3
"""Invert the executable stale-generation rejection."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/continuous_batching.rs"
source = path.read_text(encoding="utf-8")
old = "    if current.generation != request.generation() {"
new = "    if current.generation == request.generation() {"
if source.count(old) != 1:
    raise SystemExit("batching request-routing mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/continuous_batching.rs")
print("MUTATION=batching-request-routing")
print("CLAUSE=stale-generation-rejection")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
