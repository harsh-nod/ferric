#!/usr/bin/env python3
"""Export an allowlisted, identifier-free summary of four bound GPU receipts."""

import argparse
import hashlib
import json
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def summarize(directory):
    cases = []
    common = None
    for name in ("zero", "basis", "mixed", "cancellation"):
        run = directory / "runs" / name
        path = run / "dispatch-numerical.json"
        receipt = json.loads(path.read_text())
        if (receipt["schema"] != "ferric-qwen3-kproj-dispatch-numerical-v1"
                or receipt["case"] != name or receipt["status"] != "pass"
                or receipt["authority"] != "none" or receipt["values_checked"] != 1024):
            raise ValueError("unexpected numerical receipt contract")
        for key, item in (("report_sha256", run / "report.json"),
                          ("comparison_sha256", run / "comparison.json"),
                          ("output_sha256", run / "output.f32le"),
                          ("artifact_record_sha256", directory / "artifact.json"),
                          ("preflight_sha256", directory / "preflight.json")):
            if receipt[key] != digest(item):
                raise ValueError("bound evidence file identity changed")
        identity = {key: receipt[key] for key in (
            "source_sha256", "object_sha256", "probe_sha256", "worker_sha256",
            "weights_sha256", "reference_sha256", "extractor_sha256", "binder_sha256",
            "preflight_sha256", "artifact_record_sha256")}
        if common is not None and identity != common:
            raise ValueError("cross-case executable, artifact or numerical source changed")
        common = identity
        cases.append({key: receipt[key] for key in (
            "case", "values_checked", "exact_rows_checked", "maximum_absolute_error",
            "maximum_error_to_bound_ratio", "input_sha256", "output_sha256", "report_sha256",
            "comparison_sha256")} | {"receipt_sha256": digest(path)})
    return {
        "schema": "ferric-qwen3-kproj-public-engineering-summary-v1", "authority": "none",
        "scope": "four actual-weight scalar GEMV correctness dispatches; no production, full-model or performance claim",
        "target": "gfx950", "workgroup": [128, 1, 1], "grid_work_items": [1024, 1, 1],
        "policy": "bf16-exact-fp32-fma-gamma1024-v1", "values_checked": 4096,
        "identity": common, "cases": cases,
        "redaction": "allowlisted receipt fields only; no hardware identifiers, device selector or raw worker transcript",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    result = summarize(args.evidence_dir)
    with args.output.open("x") as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
        stream.write("\n")
