#!/usr/bin/env python3
"""Export allowlisted wave64 engineering observations, never private GPU selectors."""

import argparse
import json
from pathlib import Path

import bind_evidence as binding


def summarize(directory):
    artifact_path = directory / "artifact.json"
    preflight_path = directory / "preflight.json"
    artifact = binding.load(artifact_path)
    preflight = binding.load(preflight_path)
    cases = []
    executable_identity = None
    for index, name in enumerate(binding.CASES):
        run = directory / "runs" / name
        receipt_path = run / "dispatch-numerical.json"
        receipt = binding.load(receipt_path)
        report = binding.load(run / "report.json")
        record = preflight["cases"][index]
        binding.require(receipt["schema"] == "ferric-qwen3-kproj-wave64-dispatch-numerical-v1"
                        and receipt["status"] == "pass" and receipt["authority"] == "none"
                        and receipt["case"] == record["case"] == name
                        and receipt["policy"] == record["policy"] == binding.wave.POLICY
                        and receipt["values_checked"] == 1024,
                        "invalid wave numerical receipt")
        hashes = {key: receipt[key] for key in (
            "probe_sha256", "worker_sha256", "source_sha256", "object_sha256", "output_sha256")}
        binding.require(hashes["output_sha256"] == binding.digest(run / "output.f32le"),
                        "raw output changed")
        binding.validate_dispatch(report, artifact, record, hashes)
        for key, path in [("report_sha256", run / "report.json"),
                          ("artifact_record_sha256", artifact_path), ("preflight_sha256", preflight_path)]:
            binding.require(receipt[key] == binding.digest(path), "bound evidence changed")
        identity = {key: receipt[key] for key in (
            "probe_sha256", "worker_sha256", "source_sha256", "object_sha256",
            "weights_sha256", "reference_sha256", "serial_reference_sha256", "extractor_sha256")}
        binding.require(executable_identity is None or identity == executable_identity,
                        "cases use different executable/reference identities")
        executable_identity = identity
        cases.append({"case": name, "status": "pass", "receipt_sha256": binding.digest(receipt_path),
                      **{key: receipt[key] for key in (
                          "values_checked", "exact_rows_checked", "maximum_absolute_error",
                          "maximum_error_to_bound_ratio", "input_sha256", "output_sha256")}})
    return {
        "schema": "ferric-qwen3-kproj-wave64-public-evidence-v1", "authority": "none",
        "status": "pass", "target": "gfx950:xnack-", "workgroup": [128, 1, 1],
        "grid_work_items": [65536, 1, 1], "wavefront_size": 64, "compared_values": 4096,
        "completed_dispatches": 4, "policy": binding.wave.POLICY, **executable_identity,
        "observations": cases, "full_model": False, "production_qualified": False,
        "performance_claim": None,
        "scope": "file-bound engineering observations of one real-weight wave64 GEMV, not model inference",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binding.write_new(args.output, summarize(args.directory))
