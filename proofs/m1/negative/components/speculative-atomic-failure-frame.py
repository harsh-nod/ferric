#!/usr/bin/env python3
"""Mutate publication after KV preflight failure instead of preserving all state."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/speculative_step_composition.rs"
source = path.read_text(encoding="utf-8")
old = "        Err(error) => return Err(AtomicSpeculativeStepError::Kv(error)),"
new = """        Err(error) => {
            let _ = crate::discard_reserved_delta(publication);
            return Err(AtomicSpeculativeStepError::Kv(error));
        },"""
if source.count(old) != 1:
    raise SystemExit("speculative atomic failure-frame mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/speculative_step_composition.rs")
print("MUTATION=speculative-atomic-failure-frame")
print("CLAUSE=atomic-preflight-failure-frame")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
