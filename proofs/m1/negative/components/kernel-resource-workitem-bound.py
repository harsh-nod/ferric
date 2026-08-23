#!/usr/bin/env python3
"""Permit an out-of-family workitem count in the executable validator."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-kernels/src/m1_kernel_safety.rs"
source = path.read_text(encoding="utf-8")
old = """    if input.workitems == 0 || input.workitems > workitem_bound {
        return Err(M1KernelSafetyCertificateErrorV1::ResourceWorkitems);
    }
"""
new = old.replace(" == 0 || ", " == 0 && ")
if source.count(old) != 1:
    raise SystemExit("kernel resource workitem mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-kernels/src/m1_kernel_safety.rs")
print("MUTATION=kernel-resource-workitem-bound")
print("CLAUSE=finite-workitem-schedule-bound")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
