#!/usr/bin/env python3
"""Remove retained-record equality rejection from bundle revalidation."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-build/src/auth.rs"
source = path.read_text(encoding="utf-8")
old = """    if !admission_records_equal(authority.record(), &sealed.0) {
        return Err(BundleAdmissionError::AuthorityRecordMismatch);
    }
"""
if source.count(old) != 1:
    raise SystemExit("model-bundle record-binding mutation anchor drifted")
path.write_text(source.replace(old, ""), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-build/src/auth.rs")
print("MUTATION=model-bundle-record-binding")
print("CLAUSE=retained-record-equality")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
