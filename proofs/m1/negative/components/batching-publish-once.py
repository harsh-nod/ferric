#!/usr/bin/env python3
"""Reject the first completion publication and admit an already-published one."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/continuous_batching.rs"
source = path.read_text(encoding="utf-8")
old = "    if current.published_for_active_epoch {"
new = "    if !current.published_for_active_epoch {"
if source.count(old) != 1:
    raise SystemExit("batching publish-once mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/continuous_batching.rs")
print("MUTATION=batching-publish-once")
print("CLAUSE=exact-once-completion-publication")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
