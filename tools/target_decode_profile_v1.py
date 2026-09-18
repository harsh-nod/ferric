#!/usr/bin/env python3
"""Validate a pinned-reference TP1 BF16 target capture; publish a whitelist only."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import sys

REFERENCE_SHA256 = "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094"
REVISION = "b968826d9c46dd6066d109eabc6255188de91218"
PROMPT = "The capital of France is"
PROMPT_TOKENS = [785, 6722, 315, 9625, 374]
PREFIX = [12095, 13]
PREFIX_BYTES = b" Paris."
MAX_BYTES = 16 * 1024 * 1024
MAX_LINE = 131072
PACKETS = 544
RING_LIMIT = 131072
HASH_FIELDS = ("controller_sha256", "worker_sha256", "artifact_hsaco_id",
               "artifact_manifest_id", "artifact_handoff_id", "model_bundle_id")
RUNTIME_FIELDS = ("runtime_cache_admission", "runtime_operational", "runtime_profile")
PLAN_KEYS = {"schema", "authority", *HASH_FIELDS, *RUNTIME_FIELDS, "device_unique_id",
             "reference_sha256", "new_tokens", "warmup_runs", "repetitions", "capacity"}
SETUP_KEYS = {
    "schema", "authority", "model", "dtype", "target", "tensor_parallel", "device_unique_ids",
    "worker_pids", "worker_sha256", "controller_sha256", "running_worker_sha256",
    "executable_identity", "model_bundle_id", "artifact_hsaco_id", "artifact_manifest_id",
    "artifact_handoff_id", "prompt", "prompt_tokens", "new_tokens", "capacity", "repetitions",
    "warmup_runs", "rank_zero_dispatch_budget", "conservative_ring_packet_limit",
    "model_intake_seconds", "setup_seconds", "collective", "prefill", "decoding",
    "numerical_status", "timing",
}
PROFILE_KEYS = {
    "schema", "authority", "performance_qualified", "reference_sha256",
    "reference_model_revision", "reference_device", "reference_passes",
    "reference_tokens_available", "reference_tokens_consumed", "reference_expected_tokens",
    "reference_expected_utf8_bytes", "precision", "target_only", "speculation",
    "concurrent_requests", "tensor_parallel", *RUNTIME_FIELDS, "runtime_sequences",
    "runtime_ordered_batches", "runtime_rollover", "kv_prefix_cache", "sequence_reuse",
    "timing", "nonclaim",
}
RUN_KEYS = {
    "schema", "authority", "run", "warmup", "world_size", "prompt_tokens", "generated_tokens",
    "generated_text", "generated_utf8_bytes", "ttft_seconds", "tpot_seconds",
    "decode_intervals_seconds", "generation_seconds", "rank_dispatch_counts", "kv_tokens_processed",
}
PASS_KEYS = {"schema", "authority", "run", "warmup", "reference_passed", "performance_qualified",
             "generated_tokens_checked", "generated_utf8_bytes_checked", "kv_tokens_processed"}
CLOSE_KEYS = {"schema", "authority", "worker_pids", "all_workers_exited", "whole_seconds"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def read_bounded(path, limit):
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit, "input file type/extent")
        chunks = []
        remaining = before.st_size + 1
        while remaining:
            chunk = os.read(descriptor, remaining)
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        data = b"".join(chunks)
        after = os.fstat(descriptor)
        identity = lambda value: (value.st_dev, value.st_ino, value.st_size,
                                  value.st_mtime_ns, value.st_ctime_ns)
        require(identity(before) == identity(after) and len(data) == before.st_size,
                "input changed during bounded read")
        return data
    finally:
        os.close(descriptor)


def decode_json(data):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result

    def constant(_value):
        raise ValueError("nonfinite JSON constant")

    return json.loads(data.decode("utf-8"), object_pairs_hook=pairs, parse_constant=constant)


def exact(record, keys, schema):
    require(type(record) is dict and record.keys() == keys, "record fields differ from closed schema")
    require(record["schema"] == schema and record["authority"] == "none", "record schema/authority")


def integer(value, low, high, name):
    require(type(value) is int and low <= value <= high, f"invalid {name}")
    return value


def boolean(value, expected, name):
    require(type(value) is bool and value is expected, f"invalid {name}")


def finite(value, name, positive=True):
    require(type(value) in (int, float) and math.isfinite(value)
            and (value > 0 if positive else value >= 0), f"invalid {name}")
    return value


def near(actual, expected, name):
    require(math.isclose(actual, expected, rel_tol=1e-9, abs_tol=1e-6), f"inconsistent {name}")


def digest(value):
    require(type(value) is str and re.fullmatch("[0-9a-f]{64}", value) is not None, "invalid digest")
    return value


def tokens(value, count):
    require(type(value) is list and len(value) == count, "token count")
    for token in value:
        integer(token, 0, 151935, "token ID")
    return value


def byte_list(value):
    require(type(value) is list and 0 < len(value) <= MAX_LINE, "decoded byte count")
    for byte in value:
        integer(byte, 0, 255, "decoded byte")
    return bytes(value)


def reference_values(reference, count):
    """Call only after the CLI has checked the complete immutable file digest."""
    require(reference["format"] == "FERRIC-QWEN3-8B-GREEDY-REFERENCE-V1"
            and reference["authority"] == "independent-offline-reference-only", "reference scope")
    model = reference["model"]
    require(model["repository"] == "Qwen/Qwen3-8B" and model["revision"] == REVISION
            and model["class"] == "Qwen3ForCausalLM", "reference target identity")
    digest(model["deployment_bundle_identity"])
    for key, expected in {"hidden_size": 4096, "num_attention_heads": 32, "num_hidden_layers": 36,
                          "num_key_value_heads": 8, "vocab_size": 151936}.items():
        require(type(model["config"][key]) is int and model["config"][key] == expected,
                "reference model configuration")
    require(model["config"]["model_type"] == "qwen3"
            and model["config"]["torch_dtype"] == "torch.bfloat16", "reference model precision")
    prompt = reference["prompt"]
    require(prompt["text"] == PROMPT and tokens(prompt["token_ids"], 5) == PROMPT_TOKENS,
            "reference prompt")
    boolean(prompt["add_special_tokens"], False, "reference special tokens")
    execution = reference["execution"]
    for key, expected in {"attention_implementation": "sdpa", "device_dtype": "bfloat16",
                          "model_forward": "transformers.generate", "network": "offline"}.items():
        require(execution[key] == expected, "reference execution")
    boolean(execution["do_sample"], False, "reference sampling")
    boolean(execution["use_cache"], True, "reference cache")
    integer(execution["num_beams"], 1, 1, "reference beams")
    integer(execution["max_new_tokens"], 32, 32, "reference output count")
    for key in ("error_msgs", "mismatched_keys", "missing_keys", "unexpected_keys"):
        require(execution["output_loading_info"][key] == [], "reference loading anomaly")
    passes = execution["passes"]
    require(type(passes) is list and len(passes) == 2, "reference pass count")
    expected_tokens = tokens(passes[0]["token_ids"], 32)
    require(type(passes[0]["decoded_new_tokens"]) is str, "reference decoded text")
    expected_bytes = passes[0]["decoded_new_tokens"].encode("utf-8")
    require(expected_tokens[:2] == PREFIX and expected_bytes.startswith(PREFIX_BYTES), "reference prefix")
    for entry in passes:
        require(tokens(entry["token_ids"], 32) == expected_tokens
                and tokens(entry["argmax_token_ids"], 32) == expected_tokens
                and entry["decoded_new_tokens"] == passes[0]["decoded_new_tokens"],
                "reference passes disagree")
    require(type(count) is int and count in (2, 32), "unsupported reference byte prefix")
    return (expected_tokens, expected_bytes) if count == 32 else (PREFIX, PREFIX_BYTES)


def validate_plan(plan):
    exact(plan, PLAN_KEYS, "FerricQwen3TargetDecodePlanV1")
    for key in HASH_FIELDS:
        digest(plan[key])
    integer(plan["device_unique_id"], 1, (1 << 64) - 1, "planned device")
    require(plan["reference_sha256"] == REFERENCE_SHA256, "planned reference identity")
    require(type(plan["new_tokens"]) is int and plan["new_tokens"] in (2, 32), "planned output count")
    integer(plan["warmup_runs"], 0, 10, "planned warmups")
    integer(plan["repetitions"], 1, 30, "planned repetitions")
    steps = plan["new_tokens"] + 4
    integer(plan["capacity"], steps, 8192, "planned capacity")
    for key in RUNTIME_FIELDS:
        require(type(plan[key]) is bool, "planned runtime flag")
    budget = steps * PACKETS * (plan["warmup_runs"] + plan["repetitions"])
    require(budget <= RING_LIMIT, "planned no-rollover packet budget")
    return steps, budget


def records_from_bytes(data):
    require(0 < len(data) <= MAX_BYTES and data.endswith(b"\n"), "capture extent/newline")
    lines = data.splitlines()
    require(5 <= len(lines) <= 83 and all(0 < len(line) <= MAX_LINE for line in lines),
            "capture record count/extent")
    return [decode_json(line) for line in lines]


def validate(records, reference, plan):
    steps, budget = validate_plan(plan)
    expected_tokens, expected_bytes = reference_values(reference, plan["new_tokens"])
    require(plan["model_bundle_id"] == reference["model"]["deployment_bundle_identity"],
            "planned canonical model bundle")
    runs = plan["warmup_runs"] + plan["repetitions"]
    require(type(records) is list and len(records) == 3 + 2 * runs, "exact ordered record count")
    setup, profile, closed = records[0], records[1], records[-1]
    exact(setup, SETUP_KEYS, "FerricQwen3TpEngineeringSetupV1")
    exact(profile, PROFILE_KEYS, "FerricQwen3TargetDecodeProfileSetupV1")
    exact(closed, CLOSE_KEYS, "FerricQwen3TpEngineeringClosedV1")
    for key in HASH_FIELDS:
        require(digest(setup[key]) == plan[key], "predeclared source/artifact identity")
    require(setup["model"] == "Qwen/Qwen3-8B" and setup["dtype"] == "BF16"
            and setup["target"] == "gfx950:xnack-", "target workload")
    integer(setup["tensor_parallel"], 1, 1, "target tensor parallelism")
    require(type(setup["device_unique_ids"]) is list and len(setup["device_unique_ids"]) == 1,
            "single private device")
    require(integer(setup["device_unique_ids"][0], 1, (1 << 64) - 1, "device")
            == plan["device_unique_id"], "planned device identity")
    pids = setup["worker_pids"]
    require(type(pids) is list and len(pids) == 1, "single worker")
    integer(pids[0], 1, (1 << 32) - 1, "worker PID")
    require(setup["running_worker_sha256"] == [plan["worker_sha256"]]
            and setup["executable_identity"] == "live_proc_exe_sha256", "live executable identity")
    require(setup["prompt"] == PROMPT and tokens(setup["prompt_tokens"], 5) == PROMPT_TOKENS,
            "canonical prompt")
    for key in ("new_tokens", "capacity", "repetitions", "warmup_runs"):
        require(type(setup[key]) is int and setup[key] == plan[key], "planned request dimensions")
    integer(setup["rank_zero_dispatch_budget"], budget, budget, "dispatch budget")
    integer(setup["conservative_ring_packet_limit"], RING_LIMIT, RING_LIMIT, "ring limit")
    for key, expected in {
        "collective": "host_staged_fp32_rank_order_reduce_bf16_residual",
        "prefill": "token_at_a_time_m1", "decoding": "greedy_lowest_id_fixed_length",
        "numerical_status": "Contracted; compare emitted token IDs independently",
        "timing": "monotonic controller clock; includes IPC, host collectives and per-token progress logging; excludes setup",
    }.items():
        require(setup[key] == expected, "target execution/timing contract")
    setup_seconds = finite(setup["setup_seconds"], "setup duration")
    require(finite(setup["model_intake_seconds"], "model intake") <= setup_seconds,
            "model intake/setup ordering")
    for key, expected in {
        "performance_qualified": False, "target_only": True, "speculation": False,
        "runtime_sequences": False, "runtime_ordered_batches": False,
        "runtime_rollover": False, "kv_prefix_cache": False,
        **{key: plan[key] for key in RUNTIME_FIELDS},
    }.items():
        boolean(profile[key], expected, "profile boolean contract")
    for key, expected in {
        "reference_sha256": REFERENCE_SHA256, "reference_model_revision": REVISION,
        "reference_device": "AMD Instinct MI300X", "precision": "BF16",
        "sequence_reuse": "reset logical cursor; each request overwrites every consumed KV position",
        "timing": "host Instant; includes IPC, host collectives, progress logging and reference checks; not GPU timestamps",
        "nonclaim": "Reference-checked bounded diagnostic; not model-wide numerical qualification, GPU overlap, megakernel execution, or a controlled benchmark",
    }.items():
        require(profile[key] == expected, "profile reference/timing contract")
    for key, expected in {"reference_passes": 2, "reference_tokens_available": 32,
                          "reference_tokens_consumed": plan["new_tokens"],
                          "concurrent_requests": 1, "tensor_parallel": 1}.items():
        integer(profile[key], expected, expected, "profile integer contract")
    require(tokens(profile["reference_expected_tokens"], plan["new_tokens"]) == expected_tokens
            and byte_list(profile["reference_expected_utf8_bytes"]) == expected_bytes,
            "profile reference outputs")
    public_runs = []
    pooled_intervals = []
    all_generations = []
    for ordinal in range(runs):
        run, passed = records[2 + 2 * ordinal:4 + 2 * ordinal]
        exact(run, RUN_KEYS, "FerricQwen3TpEngineeringMeasurementV1")
        exact(passed, PASS_KEYS, "FerricQwen3TargetDecodeProfileRunV1")
        warmup = ordinal < plan["warmup_runs"]
        index = ordinal if warmup else ordinal - plan["warmup_runs"]
        for record in (run, passed):
            integer(record["run"], index, index, "ordered run index")
            boolean(record["warmup"], warmup, "ordered warmup marker")
            integer(record["kv_tokens_processed"], steps, steps, "fresh-sequence KV cursor")
        integer(run["world_size"], 1, 1, "run world")
        require(tokens(run["prompt_tokens"], 5) == PROMPT_TOKENS, "run prompt")
        require(tokens(run["generated_tokens"], plan["new_tokens"]) == expected_tokens,
                "independent output tokens")
        require(byte_list(run["generated_utf8_bytes"]) == expected_bytes
                and run["generated_text"] == expected_bytes.decode("utf-8"), "independent decoded bytes")
        require(type(run["rank_dispatch_counts"]) is list and len(run["rank_dispatch_counts"]) == 1,
                "single-rank dispatch count")
        integer(run["rank_dispatch_counts"][0], steps * PACKETS, steps * PACKETS, "exact dispatches")
        boolean(passed["reference_passed"], True, "reference pass")
        boolean(passed["performance_qualified"], False, "unqualified capture")
        integer(passed["generated_tokens_checked"], plan["new_tokens"], plan["new_tokens"], "checked tokens")
        integer(passed["generated_utf8_bytes_checked"], len(expected_bytes), len(expected_bytes), "checked bytes")
        intervals = run["decode_intervals_seconds"]
        require(type(intervals) is list and len(intervals) == plan["new_tokens"] - 1,
                "post-first interval count")
        intervals = [finite(value, "decode interval") for value in intervals]
        ttft = finite(run["ttft_seconds"], "TTFT")
        generation = finite(run["generation_seconds"], "generation duration")
        interval_sum = math.fsum(intervals)
        near(finite(run["tpot_seconds"], "TPOT"), interval_sum / len(intervals), "mean TPOT")
        near(generation, ttft + interval_sum, "generation duration")
        all_generations.append(generation)
        if not warmup:
            pooled_intervals.extend(intervals)
        public_runs.append({
            "run": index, "warmup": warmup, "reference_passed": True,
            "ttft_seconds": ttft, "decode_intervals_seconds": intervals,
            "tpot_seconds": interval_sum / len(intervals), "generation_seconds": generation,
            "completed_dispatches": steps * PACKETS, "kv_tokens_processed": steps,
        })
    require(type(closed["worker_pids"]) is list and closed["worker_pids"] == pids,
            "closed worker roster")
    integer(closed["worker_pids"][0], 1, (1 << 32) - 1, "closed worker PID")
    boolean(closed["all_workers_exited"], True, "worker completion")
    whole = finite(closed["whole_seconds"], "whole duration")
    require(whole + 1e-6 >= setup_seconds + math.fsum(all_generations), "whole duration omits work")
    pooled_seconds = finite(math.fsum(pooled_intervals), "derived pooled decode duration")
    pooled_rate = finite(len(pooled_intervals) / pooled_seconds, "derived post-first token rate")
    pooled_tpot = finite(pooled_seconds / len(pooled_intervals), "derived pooled TPOT")
    ordered = sorted(pooled_intervals)
    # Construct every public field explicitly. Never embed raw setup, plan,
    # reference, close, process identities, or arbitrary input dictionaries.
    return {
        "schema": "FerricQwen3TargetDecodeComparisonV1", "authority": "none",
        "performance_qualified": False, "benchmark_comparable": False,
        "gpu_timestamps": False, "overlap_measured": False, "cpu_isolation_verified": False,
        "model": "Qwen/Qwen3-8B", "model_revision": REVISION, "precision": "BF16",
        "target": "gfx950:xnack-", "target_only": True, "speculation": False,
        "concurrent_requests": 1, "tensor_parallel": 1,
        "reference_sha256": REFERENCE_SHA256, "reference_passes": 2,
        "reference_device": "AMD Instinct MI300X", "reference_tokens_available": 32,
        "reference_tokens_consumed": plan["new_tokens"],
        "expected_tokens": list(expected_tokens), "expected_utf8_bytes": list(expected_bytes),
        "all_reference_checks_passed": True, "all_workers_exited": True,
        "controller_sha256": plan["controller_sha256"], "worker_sha256": plan["worker_sha256"],
        "artifact_hsaco_id": plan["artifact_hsaco_id"], "artifact_manifest_id": plan["artifact_manifest_id"],
        "artifact_handoff_id": plan["artifact_handoff_id"], "model_bundle_id": plan["model_bundle_id"],
        "runtime_cache_admission": plan["runtime_cache_admission"],
        "runtime_operational": plan["runtime_operational"], "runtime_profile": plan["runtime_profile"],
        "runtime_sequences": False, "runtime_ordered_batches": False, "runtime_rollover": False,
        "kv_prefix_cache": False, "capacity": plan["capacity"],
        "warmup_runs": plan["warmup_runs"], "measured_runs": plan["repetitions"],
        "output_tokens_per_request": plan["new_tokens"], "total_dispatches": budget,
        "setup_seconds": setup_seconds, "whole_seconds": whole, "runs": public_runs,
        "summary": {
            "measured_decode_intervals": len(pooled_intervals), "decode_seconds": pooled_seconds,
            "post_first_tokens_per_second": pooled_rate,
            "pooled_tpot_seconds": pooled_tpot,
            "p50_interval_seconds": ordered[math.ceil(0.50 * len(ordered)) - 1],
            "p95_interval_seconds": ordered[math.ceil(0.95 * len(ordered)) - 1],
        },
        "timing_scope": "Host Instant at completed token steps. Excludes setup, final reference/decode checks and report emission; preceding reference checks and per-token logging affect following intervals. Includes host IPC and host-staged collectives. No GPU clock calibration.",
        "nonclaim": "Bounded reference-checked diagnostic, not production or model-wide numerical qualification, a controlled benchmark, GPU overlap evidence, a megakernel, or a 700 tokens/s result. A matching token sequence does not establish logit-error bounds. Runtime records are consistency-checked, not independently authenticated hardware telemetry.",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("capture", "status", "reference", "plan", "output"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    status_bytes = read_bounded(args.status, 8)
    require(status_bytes in (b"0", b"0\n"), "capture process failed")
    reference_bytes = read_bounded(args.reference, MAX_LINE)
    require(sha256(reference_bytes) == REFERENCE_SHA256, "frozen reference file identity")
    plan_bytes = read_bounded(args.plan, MAX_LINE)
    capture_bytes = read_bounded(args.capture, MAX_BYTES)
    report = validate(records_from_bytes(capture_bytes), decode_json(reference_bytes), decode_json(plan_bytes))
    report["input_sha256"] = {
        "capture": sha256(capture_bytes), "status": sha256(status_bytes), "plan": sha256(plan_bytes),
        "reference": sha256(reference_bytes), "validator": sha256(read_bounded(Path(__file__), MAX_LINE)),
    }
    serialized = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
    with args.output.open("x", encoding="utf-8") as output:
        output.write(serialized)
    print(json.dumps({"all_reference_checks_passed": True, "performance_qualified": False,
                      "summary": report["summary"]}, sort_keys=True, allow_nan=False))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, TypeError, OverflowError, UnicodeError) as error:
        print(f"Target profile rejected: {error}", file=sys.stderr)
        sys.exit(2)
