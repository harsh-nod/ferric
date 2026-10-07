#!/usr/bin/env python3
"""Admit a retired-page release when the recorded epoch is stale and lower."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/request_isolation.rs"
source = path.read_text(encoding="utf-8")
old = "    if selected.quiescent_epoch.value != exact_epoch.value {"
new = "    if selected.quiescent_epoch.value > exact_epoch.value {"
if source.count(old) != 1:
    raise SystemExit("terminal release exact-epoch mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/request_isolation.rs")
print("MUTATION=kv-terminal-release-exact-epoch")
print("CLAUSE=exact-quiescent-epoch-match")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
