#!/usr/bin/env python3
"""Accept retained manifest bytes unequal to the actual canonical encoding."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-build/src/weight_stream.rs"
source = path.read_text(encoding="utf-8")
function = "pub(crate) fn revalidate_weight_manifest_commitment("
if source.count(function) != 1:
    raise SystemExit("model-bundle manifest revalidator function anchor drifted")
start = source.index(function)
end = source.index("\n}\n", start) + 3
body = source[start:end]
old = """    if !manifest_bytes_equal(&canonical_bytes, &manifest.canonical_bytes) {
        return false;
    }
"""
new = old.replace("        return false;", "        return true;")
if body.count(old) != 1 or source.count(old) != 1:
    raise SystemExit("model-bundle manifest canonical-byte mutation anchor drifted")
path.write_text(source[:start] + body.replace(old, new) + source[end:], encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-build/src/weight_stream.rs")
print("MUTATION=model-bundle-manifest-canonical-bytes")
print("CLAUSE=retained-manifest-canonical-bytes")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
