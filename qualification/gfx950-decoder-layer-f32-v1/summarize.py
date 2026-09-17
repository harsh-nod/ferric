#!/usr/bin/env python3
"""Bind numerical comparisons to completed engineering GPU observations."""

import argparse
import hashlib
import json
from pathlib import Path

CASES = ("mixed", "zero", "saturation", "skewed-attention")
STAGE_WIDTHS = (
    ("input_norm", 4), ("rotated_q", 4), ("rotated_k", 2), ("current_v", 2),
    ("attention", 4), ("attention_residual", 4), ("post_norm", 4),
    ("gate", 4), ("up", 4), ("swiglu", 4), ("final_residual", 4),
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_json(path):
    return json.loads(path.read_text(encoding="ascii"))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def summarize(root):
    observations = []
    identity = None
    for case in CASES:
        case_path = root / "cases" / case
        run_path = root / "runs" / case
        reference = read_json(case_path / "case.json")
        comparison = read_json(run_path / "comparison.json")
        dispatch = read_json(run_path / "report.json")
        require(reference["case"] == comparison["case"] == case, "case identity")
        require(comparison["format"] == "ferric.gfx950-decoder-layer-comparison.v1",
                "comparison format")
        require(dispatch["schema"] == "ferric-finite-decoder-probe-v1", "dispatch format")
        require(dispatch["authority"] == "none" and comparison["production_qualified"] is False
                and comparison["full_model"] is False, "unqualified engineering scope")
        require(comparison["passed"] is True and comparison["compared_values"] == 10240,
                "numerical comparison did not pass")
        require(comparison["atol"] == reference["atol"] == 2e-5
                and comparison["rtol"] == reference["rtol"] == 2e-4, "tolerance mismatch")
        require([(stage["stage"], stage["values"]) for stage in comparison["stages"]]
                == [(name, width * 256) for name, width in STAGE_WIDTHS]
                and all(stage["failed"] == 0 for stage in comparison["stages"]),
                "checkpoint coverage")
        require(dispatch["completed_dispatches"] == 1
                and dispatch["input_immutability_and_output_guards_passed"] is True
                and dispatch["free_close_and_worker_exit_passed"] is True, "runtime checks")
        require(dispatch["grid_work_items"] == [256, 1, 1]
                and dispatch["workgroup"] == [128, 1, 1], "launch geometry")
        require(digest(case_path / "case.json") == comparison["case_sha256"], "case digest")
        require(digest(case_path / "inputs.f32le") == reference["inputs_sha256"]
                == comparison["inputs_sha256"] == dispatch["input_sha256"], "input digest")
        require(digest(case_path / "weights.f32le") == reference["weights_sha256"]
                == comparison["weights_sha256"] == dispatch["weights_sha256"], "weights digest")
        require(digest(case_path / "expected.f64le") == reference["expected_sha256"],
                "expected digest")
        require(digest(run_path / "output.f32le") == comparison["actual_sha256"]
                == dispatch["output_sha256"], "output digest")
        require(reference["reference_sha256"] == comparison["reference_sha256"]
                == digest(Path(__file__).with_name("reference.py")), "reference digest")
        artifact = dispatch["artifact"]
        metadata = artifact["metadata"]
        require(bytes(metadata["object_sha256"]).hex() == artifact["object_sha256"],
                "metadata object digest")
        require(metadata["wavefront_size"] == 64 and metadata["private_segment_bytes"] == 0
                and metadata["group_segment_bytes"] == 0
                and metadata["symbol"] == "ferric_gfx950_decoder_layer_f32_v1",
                "metadata resources")
        current = (artifact["source_sha256"], artifact["object_sha256"],
                   dispatch["worker_sha256"], reference["reference_sha256"], artifact)
        require(identity is None or current == identity, "artifact changed between cases")
        identity = current
        observations.append({
            "case": case,
            "passed": True,
            "compared_values": comparison["compared_values"],
            "stages": comparison["stages"],
            "case_sha256": digest(case_path / "case.json"),
            "dispatch_report_sha256": digest(run_path / "report.json"),
            "comparison_report_sha256": digest(run_path / "comparison.json"),
            "output_sha256": dispatch["output_sha256"],
        })
    return {
        "format": "ferric.gfx950-decoder-layer-gpu-evidence.v1",
        "authority": "engineering-observation-only",
        "production_qualified": False,
        "full_model": False,
        "persistent_scheduler": False,
        "performance_claim": None,
        "target": "gfx950:xnack-",
        "source_sha256": identity[0],
        "object_sha256": identity[1],
        "worker_sha256": identity[2],
        "reference_sha256": identity[3],
        "grid_work_items": [256, 1, 1],
        "workgroup": [128, 1, 1],
        "wavefront_size": 64,
        "atol": 2e-5,
        "rtol": 2e-4,
        "completed_dispatches": 4,
        "compared_values": 40960,
        "passed": True,
        "observations": observations,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence_root", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = summarize(args.evidence_root)
    with args.output.open("x", encoding="ascii") as stream:
        json.dump(report, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write("\n")
    print("GPU numerical evidence: 4 completed dispatches, 40960 values passed")


if __name__ == "__main__":
    main()
