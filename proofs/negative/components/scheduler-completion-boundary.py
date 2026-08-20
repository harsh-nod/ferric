#!/usr/bin/env python3
"""Reverse the exact-next completion epoch boundary."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/scheduler.rs"
source = path.read_text(encoding="utf-8")
old = """        let observed = completion.epoch().value;
        if observed != expected {
            return Err(CompletionFailure {
"""
new = """        let observed = completion.epoch().value;
        if observed == expected {
            return Err(CompletionFailure {
"""
if source.count(old) != 1:
    raise SystemExit("scheduler completion-boundary mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/scheduler.rs")
print("MUTATION=reverse-exact-next-completion-boundary")
