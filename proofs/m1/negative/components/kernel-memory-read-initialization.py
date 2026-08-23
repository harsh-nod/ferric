#!/usr/bin/env python3
"""Skip the executable initialized-read rejection branch."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-kernels/src/m1_kernel_safety.rs"
source = path.read_text(encoding="utf-8")
old = """    if !reads_initialized {
        return Err(M1KernelSafetyCertificateErrorV1::MemoryUninitializedRead);
    }
"""
new = old.replace("if !reads_initialized", "if false && !reads_initialized")
if source.count(old) != 1:
    raise SystemExit("kernel memory initialized-read mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-kernels/src/m1_kernel_safety.rs")
print("MUTATION=kernel-memory-read-initialization")
print("CLAUSE=initialized-read-range-coverage")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
