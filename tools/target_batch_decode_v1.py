#!/usr/bin/env python3
"""Check one BF16 Qwen3-8B request without changing older qualification policies.

This profile is intentionally narrow: TP1, one scheduled row, no prefix cache,
no head pruning, no speculation, and either two or 32 generated tokens. Each
invocation is an unwarmed observation, never repeated-run qualification.
"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import statistics


REFERENCE_SHA256 = "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094"
BUNDLE = "6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b"
REVISION = "b968826d9c46dd6066d109eabc6255188de91218"
PROMPT = "The capital of France is"
PROMPT_IDS = [785, 6722, 315, 9625, 374]
TOKENS = [12095, 13, 576, 6722, 315, 15344, 374, 21718, 13, 576, 6722,
          315, 17689, 374, 24081, 13, 576, 6722, 315, 9856, 374, 19846,
          13, 576, 6722, 315, 279, 25662, 374, 37741, 13, 576]
PIECES = [" Paris", ".", " The", " capital", " of", " Italy", " is", " Rome",
          ".", " The", " capital", " of", " Spain", " is", " Madrid", ".",
          " The", " capital", " of", " Germany", " is", " Berlin", ".",
          " The", " capital", " of", " the", " Netherlands", " is",
          " Amsterdam", ".", " The"]
IDENTITIES = {"controller_sha256", "worker_sha256", "artifact_hsaco_id",
              "artifact_manifest_id", "artifact_handoff_id"}
SETUP = set("model dtype target tensor_parallel device_unique_ids worker_pids worker_sha256 running_worker_sha256 controller_sha256 model_bundle_id artifact_hsaco_id artifact_manifest_id artifact_handoff_id session_id batch_tokens prefill_chunk page_tokens physical_pages context_tokens cache_ttl_ticks prefix_cache max_batches setup_seconds collective prefill attention cache arrival_policy numerical_status output_head_pruning performance_profile".split())
ADMISSION = set("name slot generation tick arrival_ns prompt_tokens cached_tokens cached_pages".split())
BATCH = set("tick batch_id pool_batch_id rows outputs started_ns completed_ns rank_dispatch_counts output_head_rows free_pages retained_pages cached_pages prefix_hits hit_tokens evicted_pages".split())
REQUEST = set("name slot generation state prompt_tokens generated_tokens generated_utf8_bytes generated_text cached_prefix_tokens arrival_tick arrival_ns output_timestamps_ns cancelled_ns ttft_ns decode_intervals_ns tpot_ns".split())
CLOSED = set("worker_pids all_workers_exited rank_dispatch_counts whole_seconds".split())
PROFILE = set("runtime_cache_admission runtime_operational dispatch_sequences queue_rollover projection attention runtime_profiling".split())
PLAN = IDENTITIES | {"schema", "device_unique_id", "request_name", "new_tokens",
                     "collective", "performance_profile", "max_batches",
                     "cache_ttl_ticks", "kernel_profile", "host_timing_enabled"}


def require(ok, label):
    if not ok:
        raise ValueError(label)


def same(actual, expected, label):
    # JSON comparison distinguishes true from 1 and rejects nonfinite values.
    require(json.dumps(actual, sort_keys=True, allow_nan=False) ==
            json.dumps(expected, sort_keys=True, allow_nan=False), label)


def fields(value, keys, label):
    require(type(value) is dict and set(value) == keys, label + " fields")


def integer(value, low, high, label):
    require(type(value) is int and low <= value <= high, label)
    return value


def digest(value, label):
    require(type(value) is str and re.fullmatch(r"[0-9a-f]{64}", value)
            and value != "0" * 64, label)
    return value


def seconds(value, label):
    require(type(value) in (int, float) and math.isfinite(value) and value > 0, label)
    return float(value)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def json_value(data):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result

    def nonfinite(_):
        raise ValueError("nonfinite JSON constant")

    return json.loads(data, object_pairs_hook=unique, parse_constant=nonfinite)


def read_bounded(path, limit):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit, "input type/extent")
        parts, count = [], 0
        while chunk := os.read(fd, min(65536, limit - count + 1)):
            count += len(chunk)
            require(count <= limit, "input grew beyond bound")
            parts.append(chunk)
        after = os.fstat(fd)
        attrs = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        require(count == before.st_size and all(getattr(before, key) == getattr(after, key)
                for key in attrs), "input changed during read")
        return b"".join(parts)
    finally:
        os.close(fd)


def record(value, kind, keys):
    fields(value, keys | {"schema", "authority"}, kind)
    same(value["schema"], "FerricQwen3TpBatch" + kind + "V2", kind + " schema")
    same(value["authority"], "none", kind + " authority")


def reference_contract(value):
    require(type(value) is dict, "reference object")
    same(value["format"], "FERRIC-QWEN3-8B-GREEDY-REFERENCE-V1", "reference schema")
    same(value["authority"], "independent-offline-reference-only", "reference authority")
    same(value["model"]["repository"], "Qwen/Qwen3-8B", "reference model")
    same(value["model"]["revision"], REVISION, "reference revision")
    same(value["model"]["deployment_bundle_identity"], BUNDLE, "reference bundle")
    same(value["prompt"]["text"], PROMPT, "reference prompt")
    same(value["prompt"]["token_ids"], PROMPT_IDS, "reference prompt tokens")
    same(value["prompt"]["add_special_tokens"], False, "reference special tokens")
    passes = value["execution"]["passes"]
    require(type(passes) is list and len(passes) == 2, "reference pass count")
    for run in passes:
        same(run["token_ids"], TOKENS, "reference tokens")
        same(run["argmax_token_ids"], TOKENS, "reference argmax")
        same(run["decoded_new_tokens"], "".join(PIECES), "reference decoded bytes")


def load_reference(data):
    same(sha256(data), REFERENCE_SHA256, "frozen reference digest")
    value = json_value(data)
    reference_contract(value)
    return value


def checked_plan(plan):
    fields(plan, PLAN, "independent expectation")
    same(plan["schema"], "FerricTargetBatchDecodeExpectationV1", "expectation schema")
    for key in IDENTITIES:
        digest(plan[key], "expected " + key)
    integer(plan["device_unique_id"], 1, (1 << 64) - 1, "expected device")
    require(type(plan["request_name"]) is str and
            re.fullmatch(r"[A-Za-z0-9_-]{1,64}", plan["request_name"]), "request name")
    integer(plan["new_tokens"], 2, 32, "output count")
    require(plan["new_tokens"] in (2, 32), "only two/32 token profiles")
    require(plan["collective"] in ("host_staged_fp32_rank_order_reduce_bf16_residual",
                                   "host-staged-reuse-v3", "device-tp1-v3"), "collective profile")
    same(plan["kernel_profile"], "v3-wave", "source/admission profile")
    require(type(plan["host_timing_enabled"]) is bool, "host timing flag")
    same(plan["max_batches"], len(PROMPT_IDS) + plan["new_tokens"] - 1, "exact batch budget")
    same(plan["cache_ttl_ticks"], 1024, "cache TTL")
    profile = plan["performance_profile"]
    fields(profile, PROFILE, "performance profile")
    for key in ("runtime_cache_admission", "runtime_operational"):
        require(type(profile[key]) is bool, "runtime option type")
    require(type(profile["dispatch_sequences"]) is bool, "dispatch sequence flag type")
    for key in ("queue_rollover", "runtime_profiling"):
        same(profile[key], False, "unsupported runtime profile")
    require(profile["projection"] in ("baseline", "wave"), "projection profile")
    same(profile["attention"], "baseline", "attention profile")
    if profile["dispatch_sequences"]:
        require(profile["projection"] == "wave" and plan["collective"] == "device-tp1-v3",
                "dispatch sequences require the separate wave/device TP1 candidate")
    return plan


def expected_workload(plan):
    return {"schema": "FerricQwen3TpWorkloadV2", "requests": [{
        "name": plan["request_name"], "prompt": PROMPT,
        "new_tokens": plan["new_tokens"], "arrival_tick": 0}]}


def validate_records(records, plan, reference):
    checked_plan(plan)
    reference_contract(reference)
    count = plan["new_tokens"]
    steps = 5 + count - 1
    require(type(records) is list and len(records) == steps + 4, "complete capture roster")
    setup, admission, request, closed = records[0], records[1], records[-2], records[-1]
    record(setup, "Setup", SETUP)
    constants = {"model": "Qwen/Qwen3-8B", "dtype": "BF16", "target": "gfx950:xnack-",
                 "tensor_parallel": 1, "model_bundle_id": BUNDLE, "batch_tokens": 1,
                 "prefill_chunk": 1, "page_tokens": 16, "physical_pages": 4,
                 "context_tokens": 64, "prefix_cache": False, "output_head_pruning": False,
                 "prefill": "true_multirow_chunked", "attention": "paged_causal_gqa",
                 "cache": "complete_page_radix_after_retirement",
                 "arrival_policy": "logical batch ticks; elapsed latency starts at admission",
                 "numerical_status": "Contracted; independently compare emitted token IDs; not a serving qualification"}
    for key, value in constants.items():
        same(setup[key], value, "setup " + key)
    for key in IDENTITIES | {"collective", "max_batches", "cache_ttl_ticks", "performance_profile"}:
        same(setup[key], plan[key], "pinned setup " + key)
    same(setup["device_unique_ids"], [plan["device_unique_id"]], "selected device")
    same(setup["running_worker_sha256"], [plan["worker_sha256"]], "live worker digest")
    digest(setup["session_id"], "session identity")
    pids = setup["worker_pids"]
    require(type(pids) is list and len(pids) == 1, "worker count")
    integer(pids[0], 1, (1 << 32) - 1, "worker identity")
    setup_seconds = seconds(setup["setup_seconds"], "setup time")
    record(admission, "Admission", ADMISSION)
    for key, value in {"name": plan["request_name"], "slot": 0, "generation": 1, "tick": 0,
                       "prompt_tokens": PROMPT_IDS, "cached_tokens": 0, "cached_pages": 0}.items():
        same(admission[key], value, "admission " + key)
    arrival = integer(admission["arrival_ns"], 0, (1 << 64) - 1, "arrival time")
    last = arrival
    outputs, durations = [], []
    per_forward = 616 if plan["collective"] == "device-tp1-v3" else 544
    require(steps * per_forward <= 131072, "no-rollover packet budget")
    for position, batch in enumerate(records[2:-2]):
        record(batch, "Completed", BATCH)
        for key, value in {"tick": position, "batch_id": position + 1,
                           "pool_batch_id": position + 1, "rank_dispatch_counts": [per_forward],
                           "output_head_rows": 1}.items():
            same(batch[key], value, "batch " + key)
        started = integer(batch["started_ns"], last, (1 << 64) - 1, "batch start ordering")
        completed = integer(batch["completed_ns"], started + 1, (1 << 64) - 1, "batch completion")
        row = {"slot": 0, "generation": 1, "token": PROMPT_IDS[position] if position < 5 else TOKENS[position - 5],
               "position": position, "kind": "PrefillIntermediate" if position < 4 else "PrefillFinal" if position == 4 else "Decode"}
        same(batch["rows"], [row], "exact scheduled row/token")
        if position >= 4:
            index = position - 4
            expected = [{"slot": 0, "generation": 1, "token": TOKENS[index], "index": index,
                         "completed_ns": completed, "finished": index == count - 1}]
            outputs.append(completed)
        else:
            expected = []
        same(batch["outputs"], expected, "exact reference output")
        retained = (position + 16) // 16
        for key, value in {"retained_pages": retained, "free_pages": 4 - retained,
                           "cached_pages": 0, "prefix_hits": 0, "hit_tokens": 0, "evicted_pages": 0}.items():
            same(batch[key], value, "page accounting " + key)
        durations.append(completed - started)
        last = completed
    intervals = [right - left for left, right in zip(outputs, outputs[1:])]
    text = "".join(PIECES[:count])
    record(request, "Request", REQUEST)
    for key, value in {"name": plan["request_name"], "slot": 0, "generation": 1,
                       "state": "Completed", "prompt_tokens": PROMPT_IDS,
                       "generated_tokens": TOKENS[:count], "generated_utf8_bytes": list(text.encode()),
                       "generated_text": text, "cached_prefix_tokens": 0, "arrival_tick": 0,
                       "arrival_ns": arrival, "output_timestamps_ns": outputs, "cancelled_ns": None,
                       "ttft_ns": outputs[0] - arrival, "decode_intervals_ns": intervals,
                       "tpot_ns": sum(intervals) // len(intervals)}.items():
        same(request[key], value, "request " + key)
    record(closed, "Closed", CLOSED)
    same(closed["worker_pids"], pids, "closed worker roster")
    same(closed["all_workers_exited"], True, "worker closure")
    same(closed["rank_dispatch_counts"], [steps * per_forward], "aggregate dispatches")
    whole = seconds(closed["whole_seconds"], "whole controller time")
    require(whole + 1e-6 >= setup_seconds + last / 1e9, "whole time omits workload")
    total = sum(intervals)
    # Only whitelisted public fields leave this checker. Raw receipts contain
    # physical selectors and process identifiers and must remain private.
    return {"schema": "FerricTargetBatchDecodeObservationV1", "authority": "none",
            "passed": True, "model": "Qwen/Qwen3-8B", "revision": REVISION,
            "precision": "BF16", "tensor_parallel": 1, "concurrent_requests": 1,
            "profile": "single-unwarmed-request", "warmup_requests": 0, "measured_requests": 1,
            "benchmark_qualified": False, "reference_tokens_and_bytes_match": True,
            "reference_sha256": REFERENCE_SHA256, "identities": {key: plan[key] for key in sorted(IDENTITIES)},
            "configuration": {"batch_tokens": 1, "prefill_chunk": 1, "context_tokens": 64,
                              "physical_pages": 4, "prefix_cache": False, "output_head_pruning": False,
                              "runtime_ordered_batches": False, "kernel_profile": plan["kernel_profile"],
                              "collective": plan["collective"], "performance_profile": plan["performance_profile"],
                              "host_timing_enabled": plan["host_timing_enabled"]},
            "generated_tokens": TOKENS[:count], "generated_utf8_bytes": list(text.encode()),
            "processed_kv_tokens": steps, "dispatches": steps * per_forward,
            "timing": {"clock": "host std::time::Instant; output completion receipts",
                       "setup_seconds": setup_seconds, "whole_controller_seconds": whole,
                       "admission_ttft_ns": outputs[0] - arrival, "generation_ns": outputs[-1] - arrival,
                       "decode_intervals_ns": intervals, "decode_interval_count": len(intervals),
                       "mean_tpot_ns": total / len(intervals), "median_tpot_ns": statistics.median(intervals),
                       "post_first_tokens_per_second": len(intervals) * 1e9 / total,
                       "batch_host_durations_ns": durations},
            "cleanup": {"completed_request_retirement_record": True, "worker_close_record": True,
                        "all_emitted_page_counts_checked": True, "final_free_page_count_observed": False},
            "nonclaims": ["No repeated-run statistics, benchmark qualification, speedup, or 700 tokens/s achievement.",
                          "No GPU timing, overlap, full-model megakernel, protected admission, or hardware attestation.",
                          "Host intervals include runtime, IPC and between-batch JSON logging; trace overhead is unmeasured.",
                          "Reference was generated earlier on MI300X; only this two/32-token prompt is checked.",
                          "Worker close receipts do not independently establish system-wide GPU idleness or final pool reclamation."]}


def compare(capture, status, workload, reference, expectation):
    require(status == b"0\n", "successful controller status")
    plan = checked_plan(json_value(expectation))
    same(json_value(workload), expected_workload(plan), "frozen single-request workload")
    oracle = load_reference(reference)
    require(capture.endswith(b"\n"), "capture terminal newline")
    lines = capture.splitlines()
    require(0 < len(capture) <= 1024 * 1024 and all(lines)
            and len(lines) <= 64 and all(len(line) <= 65536 for line in lines), "capture extent")
    result = validate_records([json_value(line) for line in lines], plan, oracle)
    result["input_sha256"] = {"capture": sha256(capture), "status": sha256(status),
                             "workload": sha256(workload), "reference": sha256(reference),
                             "expectation": sha256(expectation)}
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("capture", "status", "workload", "reference", "expect", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        report = compare(read_bounded(args.capture, 1024 * 1024), read_bounded(args.status, 16),
                         read_bounded(args.workload, 65536), read_bounded(args.reference, 131072),
                         read_bounded(args.expect, 65536))
        report["comparator_sha256"] = sha256(read_bounded(Path(__file__), 131072))
        with args.output.open("x", encoding="utf-8") as output:
            json.dump(report, output, indent=2, sort_keys=True, allow_nan=False)
            output.write("\n")
        print(json.dumps({"passed": True, "tokens": len(report["generated_tokens"]),
                          "post_first_tokens_per_second": report["timing"]["post_first_tokens_per_second"]}))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as error:
        parser.exit(1, f"Target batch observation rejected ({type(error).__name__}); no successful qualification.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
