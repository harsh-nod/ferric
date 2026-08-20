#!/usr/bin/env python3
"""Break Engine accepted-token publication while preserving Rust typing."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = """self.kv.finalize_tentative(
                        request,
                        accepted_tokens[index],
                        permit,
                    )"""
new = """self.kv.finalize_tentative(
                        request,
                        0,
                        permit,
                    )"""
if source.count(old) != 1:
    raise SystemExit("system completion mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=discard-engine-accepted-token-count")
