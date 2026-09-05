#!/usr/bin/env python3
"""Relabel the executable submit-entry custody transformer as prepared."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/authenticated_target_rollover_phase_custody.rs"
source = path.read_text(encoding="utf-8")
old = """    let result = M1AuthenticatedTargetRolloverSubmitEntryCustodyV1 {
        phase: M1AuthenticatedTargetRolloverPhaseV1::SubmitEntry,
    };"""
new = old.replace(
    "M1AuthenticatedTargetRolloverPhaseV1::SubmitEntry",
    "M1AuthenticatedTargetRolloverPhaseV1::Prepared",
)
if source.count(old) != 1:
    raise SystemExit("authenticated target phase-custody mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/authenticated_target_rollover_phase_custody.rs")
print("MUTATION=authenticated-target-rollover-phase-custody")
print("CLAUSE=exact-submit-entry-phase-custody")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
