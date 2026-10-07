#!/usr/bin/env python3
"""Reject target drift only when both processor and feature tuples drift."""

import hashlib
from pathlib import Path
import sys


repo = Path(sys.argv[1])
path = repo / "crates/ferric-kernels/src/validation.rs"
source = path.read_text(encoding="utf-8")
old = """    if !processor_bytes_match(candidate.processor, expected.processor)
        || !target_feature_bytes_match(&candidate.target_features, &expected.target_features)
    {
        return Err(KernelCatalogValidationError::CandidateTargetDrift);
    }
"""
new = old.replace("        || !target_feature_bytes_match", "        && !target_feature_bytes_match")
if source.count(old) != 1:
    raise SystemExit("target catalog processor/features mutation anchor drifted")
path.write_text(source.replace(old, new), encoding="utf-8")
print("MUTATED_SOURCE=crates/ferric-kernels/src/validation.rs")
print("MUTATION=target-catalog-processor-features")
print("CLAUSE=exact-processor-target-rejection")
print(f"ANCHOR_SHA256={hashlib.sha256(old.encode('utf-8')).hexdigest()}")
