#!/usr/bin/env python3
"""Discard a successful Engine admission result while preserving Rust typing."""

from pathlib import Path
import sys

repo = Path(sys.argv[1])
path = repo / "crates/ferric-engine/src/system.rs"
source = path.read_text(encoding="utf-8")
old = """                Ok(request)
            }
            Err(error) => {"""
new = """                Err(EngineError::RequestNotReady)
            }
            Err(error) => {"""
if source.count(old) != 1:
    raise SystemExit("system admit return mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-engine/src/system.rs")
print("MUTATION=discard-successful-engine-admission-result")
