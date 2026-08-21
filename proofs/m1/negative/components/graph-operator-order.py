#!/usr/bin/env python3
"""Swap the first layer operator in the executable exact graph."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-spec/src/graph.rs"
source = path.read_text(encoding="utf-8")
old = "            0 => Qwen3Operator::InputRmsNorm,"
new = "            0 => Qwen3Operator::QueryProjection,"
if source.count(old) != 1:
    raise SystemExit("graph operator-order mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-spec/src/graph.rs")
print("MUTATION=graph-operator-order")
print("CLAUSE=exact-layer-operator-order")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
