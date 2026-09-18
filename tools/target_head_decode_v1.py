#!/usr/bin/env python3
"""Strict 32-token v7-head diagnostic; never broadens the frozen batch policy.

All weights and intermediate activations remain BF16. The explicitly selected
output head stores either BF16 control logits or FP32 logits. Both must reproduce
the same previously pinned independent 32-token target reference.
"""

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import stat
import types


BASE_CHECKER_SHA256 = "90cd589fe9b98660f6efb3400775cd269af236688706f37eaedee2d12e92b975"


def shared_source():
    path = Path(__file__).with_name("target_batch_decode_v1.py")
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 131072:
            raise ValueError("shared checker source extent")
        raw = os.read(descriptor, 131073)
        after = os.fstat(descriptor)
        attrs = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        if len(raw) != before.st_size or any(getattr(before, key) != getattr(after, key) for key in attrs):
            raise ValueError("shared checker source changed")
    finally:
        os.close(descriptor)
    if hashlib.sha256(raw).hexdigest() != BASE_CHECKER_SHA256:
        raise ValueError("frozen shared checker digest")
    return path, raw


def load_shared_checker():
    path, raw = shared_source()
    module = types.ModuleType("_target_head_frozen_batch_checker")
    module.__file__ = str(path)
    # Execute exactly the bytes whose digest passed, without importing .pyc or
    # changing the separately used batch-checker module/global allowlists.
    exec(compile(raw, str(path), "exec"), module.__dict__)
    return module


batch = load_shared_checker()
HEAD_IDENTITIES = {"artifact_hsaco_id", "artifact_manifest_id", "artifact_handoff_id"}
HEAD_SETUP = {"head_precision", "fp32_head_artifact", "fp32_head_workspace_bytes"}
PLAN = batch.PLAN | HEAD_SETUP | {"variant"}
VARIANTS = {
    "baseline-bf16-v7-control": ("baseline", "bf16-v7-control"),
    "baseline-fp32-v7": ("baseline", "fp32-v7"),
    "mfma-fp32-v7": ("mfma", "fp32-v7"),
}


def pinned_base():
    shared_source()


def checked_plan(plan):
    batch.fields(plan, PLAN, "head expectation")
    batch.same(plan["schema"], "FerricTargetHeadDecodeExpectationV1", "head expectation schema")
    batch.require(type(plan["variant"]) is str and plan["variant"] in VARIANTS, "head variant")
    projection, precision = VARIANTS[plan["variant"]]
    batch.same(plan["kernel_profile"], "v3-mfma", "head main-artifact profile")
    batch.same(plan["new_tokens"], 32, "head diagnostic requires full frozen token window")
    batch.same(plan["collective"], "host-staged-reuse-v3", "head collective")
    batch.same(plan["host_timing_enabled"], False, "no instrumented head timing")
    batch.same(plan["head_precision"], precision, "head precision variant")
    batch.same(plan["fp32_head_workspace_bytes"], 9_723_904 if precision == "fp32-v7" else 0,
               "exact v7 head workspace")
    batch.fields(plan["fp32_head_artifact"], HEAD_IDENTITIES, "head artifact")
    for key in HEAD_IDENTITIES:
        batch.digest(plan["fp32_head_artifact"][key], "expected head " + key)
    profile = plan["performance_profile"]
    batch.fields(profile, batch.PROFILE, "head performance profile")
    for key in ("runtime_cache_admission", "runtime_operational"):
        batch.same(profile[key], True, "head runtime policy")
    for key in ("dispatch_sequences", "queue_rollover", "runtime_profiling"):
        batch.same(profile[key], False, "head unsupported execution mode")
    batch.same(profile["projection"], projection, "head projection variant")
    batch.same(profile["attention"], "baseline", "head attention")

    # The common checker owns schedule, bytes, pages, time and closure. Its
    # original projection/profile allowlist is immutable; adapt only metadata
    # already checked above, never numerical records or observed dispatches.
    common = {key: copy.deepcopy(plan[key]) for key in batch.PLAN}
    common["schema"] = "FerricTargetBatchDecodeExpectationV1"
    common["kernel_profile"] = "v3-wave"
    common["performance_profile"]["projection"] = "baseline"
    batch.checked_plan(common)
    return common


def validate_records(records, plan, reference):
    pinned_base()
    common = checked_plan(plan)
    batch.require(type(records) is list and len(records) == 40, "complete 32-token head capture")
    setup = records[0]
    batch.fields(setup, batch.SETUP | HEAD_SETUP | {"schema", "authority"}, "head setup")
    for key in HEAD_SETUP | {"performance_profile"}:
        batch.same(setup[key], plan[key], "pinned head setup " + key)
    normalized = copy.deepcopy(records)
    for key in HEAD_SETUP:
        del normalized[0][key]
    normalized[0]["performance_profile"] = copy.deepcopy(common["performance_profile"])
    result = batch.validate_records(normalized, common, reference)
    result["schema"] = "FerricTargetHeadDecodeObservationV1"
    result["variant"] = plan["variant"]
    result["weight_precision"] = "BF16"
    result["activation_precision"] = "BF16"
    result["logits_precision"] = "FP32" if plan["head_precision"] == "fp32-v7" else "BF16"
    result["head_precision"] = plan["head_precision"]
    result["head_artifact"] = copy.deepcopy(plan["fp32_head_artifact"])
    result["fp32_head_workspace_bytes"] = plan["fp32_head_workspace_bytes"]
    result["configuration"]["kernel_profile"] = plan["kernel_profile"]
    result["configuration"]["performance_profile"] = copy.deepcopy(plan["performance_profile"])
    result["configuration"]["target_only"] = True
    result["configuration"]["speculation"] = False
    result["shared_checker_sha256"] = BASE_CHECKER_SHA256
    result["nonclaims"].append(
        "This distinct v7-head diagnostic retains BF16 weights/activations; FP32 logits, when selected, are explicit. "
        "No result changes the frozen wave/BF16-head rejection or establishes its cause.")
    return result


def compare(capture, status, workload, reference, expectation):
    batch.require(status == b"0\n", "successful controller status")
    plan = batch.json_value(expectation)
    checked_plan(plan)
    batch.same(batch.json_value(workload), batch.expected_workload(plan), "frozen head workload")
    oracle = batch.load_reference(reference)
    batch.require(capture.endswith(b"\n"), "capture terminal newline")
    lines = capture.splitlines()
    batch.require(0 < len(capture) <= 1024 * 1024 and len(lines) == 40
                  and all(lines) and all(len(line) <= 65536 for line in lines), "head capture extent")
    result = validate_records([batch.json_value(line) for line in lines], plan, oracle)
    result["input_sha256"] = {"capture": batch.sha256(capture), "status": batch.sha256(status),
                             "workload": batch.sha256(workload), "reference": batch.sha256(reference),
                             "expectation": batch.sha256(expectation)}
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("capture", "status", "workload", "reference", "expect", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        report = compare(batch.read_bounded(args.capture, 1024 * 1024), batch.read_bounded(args.status, 16),
                         batch.read_bounded(args.workload, 65536), batch.read_bounded(args.reference, 131072),
                         batch.read_bounded(args.expect, 65536))
        report["comparator_sha256"] = batch.sha256(batch.read_bounded(Path(__file__), 131072))
        encoded = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        with args.output.open("x", encoding="utf-8") as stream:
            stream.write(encoded)
        print(json.dumps({"passed": True, "variant": report["variant"], "tokens": 32,
                          "logits_precision": report["logits_precision"],
                          "post_first_tokens_per_second": report["timing"]["post_first_tokens_per_second"]}))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as error:
        parser.exit(1, f"Target head diagnostic rejected ({type(error).__name__}); no successful observation.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
