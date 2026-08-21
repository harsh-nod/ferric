#!/usr/bin/env python3
"""Accept only a mismatching plan identity during step-plan validation."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/step_plan_publication.rs"
source = path.read_text(encoding="utf-8")
old = "    if !plan.plan_id.equals(expected_plan_id) {"
new = "    if plan.plan_id.equals(expected_plan_id) {"
if source.count(old) != 1:
    raise SystemExit("publication plan-identity mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/step_plan_publication.rs")
print("MUTATION=publication-plan-identity")
print("CLAUSE=exact-plan-identity")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
