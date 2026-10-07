#!/usr/bin/env python3
"""Substitute one speculative draft token into direct publication validation."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/step_plan_publication.rs"
source = path.read_text(encoding="utf-8")
old = """    let completion_result = validate_compact_completion(
        &record,
        publication.plan.request,
        publication.plan.completion_epoch,
        &publication.plan.plan_id,
        0,
    );"""
new = old.replace("        0,", "        1,")
if source.count(old) != 1:
    raise SystemExit("target direct publication mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/step_plan_publication.rs")
print("MUTATION=target-direct-publication")
print("CLAUSE=zero-draft-single-token-publication")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
