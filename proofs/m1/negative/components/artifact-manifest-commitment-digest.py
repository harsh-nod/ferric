#!/usr/bin/env python3
"""Reject the matching manifest digest instead of a mismatching digest."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-build/src/auth.rs"
source = path.read_text(encoding="utf-8")
old = """    if !bytes_equal(&digest, &manifest.aggregate_id()) {
        return Err(invalid("canonical manifest digest"));
    }
"""
new = old.replace("if !bytes_equal", "if bytes_equal")
if source.count(old) != 1:
    raise SystemExit("artifact manifest-commitment digest mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-build/src/auth.rs")
print("MUTATION=artifact-manifest-commitment-digest")
print("CLAUSE=canonical-manifest-digest-binding")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
