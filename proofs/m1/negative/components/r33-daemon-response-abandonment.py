#!/usr/bin/env python3
"""Incorrectly advance an R33 lifecycle after response abandonment."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "proofs/m1/r33_daemon_lifecycle.rs"
source = path.read_text(encoding="utf-8")
header = "pub fn resolve_m1_r33_daemon_response_v1("
old = """            M1R33DaemonResponseV1::Abandoned => {
                Some(M1R33DaemonLifecycleV1::Stable(pending.abandoned))
            },
"""
if source.count(header) != 1:
    raise SystemExit("R33 response-abandonment function anchor drifted")
prefix, tail = source.split(header, 1)
if tail.count(old) != 1:
    raise SystemExit("R33 response-abandonment mutation anchor drifted")
new = old.replace("pending.abandoned", "pending.delivered")
path.write_text(prefix + header + tail.replace(old, new), encoding="utf-8")
anchor = (header + old).encode("utf-8")
print("MUTATED_SOURCE=proofs/m1/r33_daemon_lifecycle.rs")
print("MUTATION=r33-daemon-response-abandonment")
print("CLAUSE=abandoned-response-does-not-advance")
print(f"ANCHOR_SHA256={hashlib.sha256(anchor).hexdigest()}")
