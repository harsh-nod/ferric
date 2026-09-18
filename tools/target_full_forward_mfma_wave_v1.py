#!/usr/bin/env python3
"""Strict MFMA/BF16, wave attention and FP32-v7 full-forward observations.

Both paths execute the same 616 packets per forward and pinned 32-token
reference. One bounded forward command is not a persistent GPU megakernel.
"""

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import stat
import types


CORE_SHA256 = "90cd589fe9b98660f6efb3400775cd269af236688706f37eaedee2d12e92b975"
WORKER_SHA256 = "8284a44f702b8e42a35fe9e04cf051dd709ce9995329adaa2ea5a57ab80fb06a"
# Independently released controller for the frozen wave-attention cohort.
CONTROLLER_SHA256 = "5b218caa6aa51c56749f64329054d29faf0cc766e401db2c41f58bc8d1324f48"
ARTIFACT = {
    "artifact_hsaco_id": "6b0889e2834bda81313b3cbda5a60207dd9fa41c40ea80251bbec89d7befef2c",
    "artifact_manifest_id": "ba4e0a6fe1a965a8a87566d2995ec26c40062f6fad0ff5b565aa7a064418dc50",
    "artifact_handoff_id": "73562e1b89472c1bbc2e008a9696466ce216f419b9ab9e0eb785fa7dc9e0e429",
}
HEAD_ARTIFACT = {
    "artifact_hsaco_id": "b21caccae5ec030640242034940822cb1a3f1e8531b7b6ded7371492da3747ae",
    "artifact_manifest_id": "6815ba33eb0c96a88a0722b1cdc73473d09cce3b4594d98370e6e1282437a008",
    "artifact_handoff_id": "d6ca1b9b6c5170eec8a904e2117e64c4988bd5a7334a6d3569c855e7400219c6",
}
HEAD_SETUP = {"head_precision", "fp32_head_artifact", "fp32_head_workspace_bytes"}
PROFILE = "mfma-v3-fp32-v7-wave-attention-tp1-616"
EXPECTATION_SCHEMA = "FerricTargetFullForwardMfmaWaveExpectationV1"
STDERR = b"additional resident transposed weight bytes: 15136194560\n"
EXTRA_SETUP = {"runtime_full_forward", "full_forward_profile"}
VARIANTS = {"serial-mfma-v7-wave-control": False, "full-forward-mfma-v7-wave": True}


def load_pinned():
    path = Path(__file__).with_name("target_batch_decode_v1.py")
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 131072:
            raise ValueError("shared source extent")
        raw = os.read(descriptor, 131073)
        after = os.fstat(descriptor)
        attrs = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        if len(raw) != before.st_size or any(getattr(before, key) != getattr(after, key) for key in attrs):
            raise ValueError("shared source changed")
        if hashlib.sha256(raw).hexdigest() != CORE_SHA256:
            raise ValueError("frozen shared source digest")
    finally:
        os.close(descriptor)
    module = types.ModuleType("_frozen_full_forward_mfma_wave_batch_checker")
    module.__file__ = str(path)
    # Only verified bytes execute; no import cache or shared-module mutation.
    exec(compile(raw, str(path), "exec"), module.__dict__)
    return module


core = load_pinned()
PLAN = core.PLAN | EXTRA_SETUP | HEAD_SETUP | {"variant", "common_comparator_sha256"}


def common_plan(plan):
    common = {key: copy.deepcopy(plan[key]) for key in core.PLAN}
    common["schema"] = "FerricTargetBatchDecodeExpectationV1"
    # Validate MFMA/head metadata first; only the old metadata allowlist is
    # adapted, never observed packets, scheduling, arithmetic receipts or times.
    common["kernel_profile"] = "v3-wave"
    common["performance_profile"]["projection"] = "baseline"
    common["performance_profile"]["attention"] = "baseline"
    return common


def checked_plan(plan):
    core.fields(plan, PLAN, "full-forward expectation")
    core.same(plan["schema"], EXPECTATION_SCHEMA, "expectation schema")
    core.require(type(plan["variant"]) is str and plan["variant"] in VARIANTS, "explicit variant")
    full_forward = VARIANTS[plan["variant"]]
    core.same(plan["runtime_full_forward"], full_forward, "full-forward mode")
    core.same(plan["full_forward_profile"], PROFILE if full_forward else None, "full-forward profile")
    core.same(plan["common_comparator_sha256"], CORE_SHA256, "shared numerical checker")
    core.digest(CONTROLLER_SHA256, "independently frozen controller")
    core.same(plan["controller_sha256"], CONTROLLER_SHA256, "frozen integrated controller")
    core.same(plan["worker_sha256"], WORKER_SHA256, "frozen full-forward worker for both variants")
    for key, value in ARTIFACT.items():
        core.same(plan[key], value, "frozen MFMA " + key)
    core.same(plan["new_tokens"], 32, "full frozen token window")
    core.same(plan["kernel_profile"], "v3-mfma", "closed MFMA v3 image")
    core.same(plan["head_precision"], "fp32-v7", "explicit FP32-v7 output head")
    core.same(plan["fp32_head_workspace_bytes"], 9_723_904, "exact capacity16 FP32 workspace")
    core.same(plan["fp32_head_artifact"], HEAD_ARTIFACT, "frozen v7 head image")
    core.same(plan["collective"], "device-tp1-v3", "device TP1 residual")
    core.same(plan["host_timing_enabled"], False, "no host timing instrumentation")
    core.same(plan["performance_profile"], {
        "runtime_cache_admission": True, "runtime_operational": True,
        "runtime_profiling": False, "dispatch_sequences": False, "queue_rollover": False,
        "projection": "mfma", "attention": "wave",
    }, "exact MFMA-v7-wave execution profile")
    core.checked_plan(common_plan(plan))
    return plan


def expected_workload(plan):
    return core.expected_workload(checked_plan(plan))


def execution_counts(plan):
    return {"forwards": 36, "packets_per_forward": 616, "packets": 22_176,
            "completion_frontiers": 36 if plan["runtime_full_forward"] else 22_176,
            "frontier_count_scope": "source-derived schedule; not measured polls or GPU event counts"}


def validate_records(records, plan, reference):
    load_pinned()
    checked_plan(plan)
    core.require(type(records) is list and len(records) == 40, "complete 32-token capture")
    extra = EXTRA_SETUP if plan["runtime_full_forward"] else set()
    core.fields(records[0], core.SETUP | {"schema", "authority"} | extra | HEAD_SETUP, "full-forward setup")
    for key in extra | HEAD_SETUP | {"performance_profile"}:
        core.same(records[0][key], plan[key], "explicit setup " + key)
    normalized = copy.deepcopy(records)
    for key in extra | HEAD_SETUP:
        del normalized[0][key]
    normalized[0]["performance_profile"] = copy.deepcopy(common_plan(plan)["performance_profile"])
    # Dispatches, scheduling, bytes, identities and timestamps remain untouched.
    result = core.validate_records(normalized, common_plan(plan), reference)
    result.update(controller_cohort="mfma-v7-wave-attention-controller-v7", schema="FerricTargetFullForwardMfmaWaveObservationV1", variant=plan["variant"],
                  performance_qualified=False, runtime_profiling=False, gpu_timestamps=False,
                  overlap_measured=False, completion_polls_measured=False, persistent_kernel=False,
                  common_comparator_sha256=CORE_SHA256, execution_counts=execution_counts(plan),
                  weight_precision="BF16", activation_precision="BF16", logits_precision="FP32",
                  head_precision="fp32-v7", head_artifact=copy.deepcopy(HEAD_ARTIFACT),
                  fp32_head_workspace_bytes=plan["fp32_head_workspace_bytes"])
    result["configuration"].update(runtime_full_forward=plan["runtime_full_forward"],
                                   full_forward_profile=plan["full_forward_profile"],
                                   target_only=True, speculation=False, head_precision="fp32-v7",
                                   kernel_profile="v3-mfma",
                                   performance_profile=copy.deepcopy(plan["performance_profile"]))
    result["nonclaims"].append(
        "Full-forward submission retains all 616 sequential packets per forward and unchanged MFMA/v7/wave-attention arithmetic. "
        "It changes host submission/completion frontiers, not a persistent GPU kernel, GPU overlap or measured poll count.")
    return result


def compare(capture, status, workload, reference, expectation, stderr):
    core.require(status == b"0\n", "successful controller status")
    core.require(stderr == STDERR, "exact unprofiled stderr preamble")
    plan = checked_plan(core.json_value(expectation))
    core.same(core.json_value(workload), expected_workload(plan), "frozen single-request workload")
    oracle = core.load_reference(reference)
    core.require(capture.endswith(b"\n"), "capture terminal newline")
    lines = capture.splitlines()
    core.require(0 < len(capture) <= 1024 * 1024 and len(lines) == 40
                 and all(lines) and all(len(line) <= 65536 for line in lines), "capture extent")
    result = validate_records([core.json_value(line) for line in lines], plan, oracle)
    result["input_sha256"] = {name: core.sha256(raw) for name, raw in (
        ("capture", capture), ("status", status), ("workload", workload), ("reference", reference),
        ("expectation", expectation), ("stderr", stderr))}
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("capture", "status", "workload", "reference", "expect", "stderr", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    os.umask(0o077)
    try:
        report = compare(core.read_bounded(args.capture, 1024 * 1024), core.read_bounded(args.status, 16),
                         core.read_bounded(args.workload, 65536), core.read_bounded(args.reference, 131072),
                         core.read_bounded(args.expect, 65536), core.read_bounded(args.stderr, 65536))
        report["comparator_sha256"] = core.sha256(core.read_bounded(Path(__file__), 131072))
        encoded = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        with args.output.open("x", encoding="utf-8") as stream:
            stream.write(encoded)
        print(json.dumps({"passed": True, "variant": report["variant"], "tokens": 32,
                          "packets": report["dispatches"],
                          "source_frontiers": report["execution_counts"]["completion_frontiers"]}))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as error:
        parser.exit(1, f"Target MFMA-v7-wave full-forward observation rejected ({type(error).__name__}); no successful observation.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
