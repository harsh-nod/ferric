#!/usr/bin/env python3
"""Erase the retired tail token's initialized-prefix state."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/paged_kv_refinement.rs"
source = path.read_text(encoding="utf-8")
old = """            ownership: PhysicalPageOwnership::Retired {
                request: state.request,
                role: state.selection.role,
                after_epoch,
            },
            initialized_prefix: 1,
        };"""
new = old.replace("            initialized_prefix: 1,", "            initialized_prefix: 0,")
if source.count(old) != 1:
    raise SystemExit("KV rollback-retirement mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/paged_kv_refinement.rs")
print("MUTATION=kv-rollback-retirement")
print("CLAUSE=retired-tail-prefix")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
