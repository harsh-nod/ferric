#!/usr/bin/env python3
"""Admit an out-of-order R33 measurement window."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "proofs/m1/r33_daemon_lifecycle.rs"
source = path.read_text(encoding="utf-8")
old = """                } else if !exact_instance_action_exec(
                    command,
                    M1R33DaemonActionV1::Measure,
                    instance,
                    server_start,
                ) || command.window != next {
"""
new = old.replace("command.window != next", "command.window == next")
if source.count(old) != 1:
    raise SystemExit("R33 measure-order mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=proofs/m1/r33_daemon_lifecycle.rs")
print("MUTATION=r33-daemon-measure-order")
print("CLAUSE=exact-next-measure-window")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
