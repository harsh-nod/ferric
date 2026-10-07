#!/usr/bin/env python3
"""Accept unequal retained-record bytes in the exact comparator."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-build/src/auth.rs"
source = path.read_text(encoding="utf-8")
old = """    if !bytes_equal(left.as_bytes(), right.as_bytes()) {
        return false;
    }
"""
new = old.replace("        return false;", "        return true;")
if source.count(old) != 1:
    raise SystemExit("model-bundle record-binding mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-build/src/auth.rs")
print("MUTATION=model-bundle-record-binding")
print("CLAUSE=retained-record-equality")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
