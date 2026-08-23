#!/usr/bin/env python3
"""Permit an absent declared profile identity after catalog/profile selection."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/operation_kernel_plan.rs"
source = path.read_text(encoding="utf-8")
old = """    if !declared.profile_id().is_present() {
        return Err(DeclaredOperatorCertificateError::MissingIdentity(
            DeclaredOperatorCertificateIdentityRole::Profile,
        ));
    }
"""
new = old.replace("declared.profile_id()", "declared.profile_catalog_id()")
if source.count(old) != 1:
    raise SystemExit("operator declared profile identity mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/operation_kernel_plan.rs")
print("MUTATION=operator-declared-profile-effect")
print("CLAUSE=profile-identity-presence")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
