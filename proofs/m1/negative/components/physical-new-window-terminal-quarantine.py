#!/usr/bin/env python3
"""Incorrectly leave the engine healthy after committed physical failure."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "proofs/m1/physical_new_window.rs"
source = path.read_text(encoding="utf-8")
header = "pub fn execute_m1_physical_new_window_transaction_v1("
old = """        M1PhysicalNewWindowOutcomeV1::Terminal {
            phase: M1PhysicalNewWindowTerminalPhaseV1::PhysicalSubmission,
            custody,
            engine_quarantined: true,
            provider_input_dequeued: true,
        }
"""
if source.count(header) != 1:
    raise SystemExit("physical new-window executable function anchor drifted")
prefix, tail = source.split(header, 1)
if tail.count(old) != 1:
    raise SystemExit("physical new-window terminal-quarantine mutation anchor drifted")
new = old.replace("engine_quarantined: true", "engine_quarantined: false")
path.write_text(prefix + header + tail.replace(old, new), encoding="utf-8")
anchor = (header + old).encode("utf-8")
print("MUTATED_SOURCE=proofs/m1/physical_new_window.rs")
print("MUTATION=physical-new-window-terminal-quarantine")
print("CLAUSE=postcommit-failure-quarantined")
print(f"ANCHOR_SHA256={hashlib.sha256(anchor).hexdigest()}")
