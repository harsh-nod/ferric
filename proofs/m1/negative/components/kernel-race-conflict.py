#!/usr/bin/env python3
"""Skip the executable same-phase conflicting-access rejection branch."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-kernels/src/m1_kernel_safety.rs"
source = path.read_text(encoding="utf-8")
old = """    if !race_free {
        return Err(M1KernelSafetyCertificateErrorV1::RaceConflictingAccess);
    }
"""
new = old.replace("if !race_free", "if false && !race_free")
if source.count(old) != 1:
    raise SystemExit("kernel race conflict mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-kernels/src/m1_kernel_safety.rs")
print("MUTATION=kernel-race-conflict")
print("CLAUSE=same-phase-conflict-rejection")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
