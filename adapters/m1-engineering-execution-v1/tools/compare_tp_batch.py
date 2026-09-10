#!/usr/bin/env python3
"""Check the fixed four-request engineering workload, not a serving benchmark.

The frozen reference's exact 32-token text is asserted against explicit token
pieces before using its subsequences. This is deliberately not a general
tokenizer or a comparator for arbitrary workloads. No subprocess is launched.
"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat


REFERENCE_SHA256 = "1ed868663df52a146dd7921f9fbb2cf1e1d0c0ceed8a7bfda030d65f8ec5b094"
BUNDLE = "6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b"
PROMPT = "The capital of France is"
PROMPT_IDS = [785, 6722, 315, 9625, 374]
REFERENCE_IDS = [12095, 13, 576, 6722, 315, 15344, 374, 21718, 13, 576, 6722,
                 315, 17689, 374, 24081, 13, 576, 6722, 315, 9856, 374, 19846,
                 13, 576, 6722, 315, 279, 25662, 374, 37741, 13, 576]
REFERENCE_PIECES = [" Paris", ".", " The", " capital", " of", " Italy", " is", " Rome",
                    ".", " The", " capital", " of", " Spain", " is", " Madrid", ".",
                    " The", " capital", " of", " Germany", " is", " Berlin", ".",
                    " The", " capital", " of", " the", " Netherlands", " is",
                    " Amsterdam", ".", " The"]
NAMES = ["seed-prefix", "arriving-short", "cancel-between-batches", "reuse-prefix"]
PREFIX_LENGTHS = [12, 0, 0, 14]
OUTPUT_LENGTHS = [2, 3, 1, 2]
REQUESTED_LENGTHS = [2, 3, 4, 2]
REQUEST_IDS = [(0, 1), (1, 1), (2, 1), (0, 2)]
U64_MAX = (1 << 64) - 1
SCHEMA_PREFIX = "FerricQwen3TpBatch"
SETUP_FIELDS = set("model dtype target tensor_parallel device_unique_ids worker_pids worker_sha256 running_worker_sha256 controller_sha256 model_bundle_id artifact_hsaco_id artifact_manifest_id artifact_handoff_id session_id batch_tokens prefill_chunk page_tokens physical_pages context_tokens cache_ttl_ticks prefix_cache max_batches setup_seconds collective prefill attention cache arrival_policy numerical_status".split())
ADMISSION_FIELDS = set("name slot generation tick arrival_ns prompt_tokens cached_tokens cached_pages".split())
BATCH_FIELDS = set("tick batch_id pool_batch_id rows outputs started_ns completed_ns rank_dispatch_counts free_pages retained_pages cached_pages prefix_hits hit_tokens evicted_pages".split())
REQUEST_FIELDS = set("name slot generation state prompt_tokens generated_tokens generated_utf8_bytes generated_text cached_prefix_tokens arrival_tick arrival_ns output_timestamps_ns cancelled_ns ttft_ns decode_intervals_ns tpot_ns".split())
CLOSED_FIELDS = set("worker_pids all_workers_exited rank_dispatch_counts whole_seconds".split())
ROW_FIELDS = set("slot generation token position kind".split())
OUTPUT_FIELDS = set("slot generation token index completed_ns finished".split())


def require(condition, message):
    if not condition:
        raise ValueError(message)


def integer(value, low=0, high=U64_MAX, label="integer"):
    require(type(value) is int and low <= value <= high, f"invalid {label}")
    return value


def integer_list(value, low=0, high=U64_MAX, label="integer list"):
    require(type(value) is list, f"invalid {label}")
    return [integer(item, low, high, label) for item in value]


def positive_seconds(value, label):
    require(type(value) in (int, float) and math.isfinite(value) and value > 0,
            f"invalid {label}")
    return value


def sha256(value):
    return hashlib.sha256(value).hexdigest()


def hash_value(value, label):
    require(type(value) is str and re.fullmatch(r"[0-9a-f]{64}", value)
            and value != "0" * 64, f"invalid {label}")
    return value


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def json_value(data):
    def invalid_constant(value):
        raise ValueError(f"nonfinite JSON constant: {value}")
    return json.loads(data, object_pairs_hook=unique_object, parse_constant=invalid_constant)


def read_bounded(path, limit):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit,
                f"invalid bounded input: {Path(path).name}")
        parts, count = [], 0
        while block := os.read(fd, min(65536, limit - count + 1)):
            count += len(block)
            require(count <= limit, "input grew beyond byte bound")
            parts.append(block)
        after = os.fstat(fd)
        fields = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        require(count == before.st_size and all(getattr(before, key) == getattr(after, key)
                for key in fields), "input identity changed while reading")
        return b"".join(parts)
    finally:
        os.close(fd)


def fields(value, expected, label):
    require(type(value) is dict and set(value) == expected, f"{label} fields drifted")


def record(value, kind, expected):
    fields(value, expected | {"schema", "authority"}, kind)
    require(value["schema"] == SCHEMA_PREFIX + kind + "V2" and value["authority"] == "none",
            f"{kind} schema or authority drifted")


def load_reference(data):
    require(sha256(data) == REFERENCE_SHA256, "frozen reference file identity drifted")
    reference = json_value(data)
    require(reference["format"] == "FERRIC-QWEN3-8B-GREEDY-REFERENCE-V1"
            and reference["authority"] == "independent-offline-reference-only",
            "reference schema drifted")
    require(reference["model"]["repository"] == "Qwen/Qwen3-8B"
            and reference["model"]["deployment_bundle_identity"] == BUNDLE,
            "reference model drifted")
    require(reference["prompt"]["text"] == PROMPT
            and integer_list(reference["prompt"]["token_ids"], high=151935) == PROMPT_IDS
            and reference["prompt"]["add_special_tokens"] is False, "reference prompt drifted")
    passes = reference["execution"]["passes"]
    require(type(passes) is list and len(passes) == 2, "reference pass count drifted")
    for run in passes:
        require(integer_list(run["token_ids"], high=151935) == REFERENCE_IDS
                and integer_list(run["argmax_token_ids"], high=151935) == REFERENCE_IDS
                and run["decoded_new_tokens"] == "".join(REFERENCE_PIECES),
                "reference tokens or exact token-piece text drifted")
    return reference


def expected_workload():
    requests = []
    for index, name in enumerate(NAMES):
        value = {"name": name, "prompt": PROMPT + "".join(REFERENCE_PIECES[:PREFIX_LENGTHS[index]]),
                 "new_tokens": REQUESTED_LENGTHS[index], "arrival_tick": index}
        if index == 2:
            value["cancel_tick"] = 3
        requests.append(value)
    return {"schema": "FerricQwen3TpWorkloadV2", "requests": requests}


def load_workload(data):
    workload = json_value(data)
    expected = expected_workload()
    require(workload == expected, "fixed workload profile drifted")
    for request in workload["requests"]:
        integer(request["new_tokens"], 1, 256)
        integer(request["arrival_tick"], 0, 10000)
        if "cancel_tick" in request:
            integer(request["cancel_tick"], 0, 10000)
    return workload


def gpu_roster(data):
    value = json_value(data)
    require(type(value) is dict and set(value) == {f"card{i}" for i in range(8)},
            "GPU snapshot must contain the eight physical cards")
    identifiers = []
    for index in range(8):
        card = value[f"card{index}"]
        identity = card["Unique ID"]
        require(type(identity) is str and re.fullmatch(r"0x[0-9a-f]{1,16}", identity),
                "invalid physical GPU identity")
        identifiers.append(integer(int(identity, 16), 1))
        for field in ("GPU use (%)", "GPU Memory Allocated (VRAM%)",
                      "GPU Memory Read/Write Activity (%)"):
            require(card[field] == "0", f"GPU snapshot is not idle: {field}")
    require(len(set(identifiers)) == 8, "duplicate physical GPU identity")
    return identifiers


def timeline(cache):
    # Explicit independent schedule for this bounded workload and 16-row policy.
    batches = [
        [(0, 0, 16)],
        [(1, 0, 5), (0, 16, 1)],
        [(0, 17, 1), (1, 5, 1), (2, 0, 5)],
        [(1, 6, 1), (3, 16, 3)] if cache else [(1, 6, 1), (3, 0, 15)],
        [(3, 19, 1)] if cache else [(3, 15, 4)],
    ]
    if not cache:
        batches.append([(3, 19, 1)])
    order = [("admit", 0), ("batch", 0), ("admit", 1), ("batch", 1),
             ("admit", 2), ("batch", 2), ("retire", 0), ("admit", 3),
             ("retire", 2), ("batch", 3), ("retire", 1), ("batch", 4)]
    if not cache:
        order.append(("batch", 5))
    order.append(("retire", 3))
    return batches, order


def check_setup(setup, world, gpu_ids, expected_hashes, expected_cache):
    record(setup, "Setup", SETUP_FIELDS)
    constants = {"model": "Qwen/Qwen3-8B", "dtype": "BF16", "target": "gfx950:xnack-",
                 "model_bundle_id": BUNDLE,
                 "collective": "host_staged_fp32_rank_order_reduce_bf16_residual",
                 "prefill": "true_multirow_chunked", "attention": "paged_causal_gqa",
                 "cache": "complete_page_radix_after_retirement",
                 "arrival_policy": "logical batch ticks; elapsed latency starts at admission",
                 "numerical_status": "Contracted; independently compare emitted token IDs; not a serving qualification"}
    for key, expected in constants.items():
        require(setup[key] == expected, f"setup {key} drifted")
    require(integer(setup["tensor_parallel"], 1, 8) == world, "tensor-parallel world drifted")
    require(integer_list(setup["device_unique_ids"], 1) == gpu_ids[:world], "rank device roster drifted")
    pids = integer_list(setup["worker_pids"], 1, (1 << 31) - 1, "worker PID")
    require(len(pids) == world and len(set(pids)) == world, "worker PID roster drifted")
    for key in ("controller_sha256", "worker_sha256", "artifact_hsaco_id",
                "artifact_manifest_id", "artifact_handoff_id", "session_id"):
        hash_value(setup[key], key)
    running = setup["running_worker_sha256"]
    require(type(running) is list and running == [setup["worker_sha256"]] * world,
            "live worker executable hashes drifted")
    for key, expected in expected_hashes.items():
        require(key in ("controller_sha256", "worker_sha256", "artifact_hsaco_id"), "unknown expected identity")
        require(setup[key] == hash_value(expected, key), f"expected {key} drifted")
    for key in ("batch_tokens", "prefill_chunk", "page_tokens"):
        require(integer(setup[key], 1, 16) == 16, f"fixed profile {key} drifted")
    integer(setup["physical_pages"], 4, 512, "physical page capacity")
    integer(setup["context_tokens"], 20, 8192, "context capacity")
    integer(setup["cache_ttl_ticks"], 6, U64_MAX, "cache TTL")
    require(type(setup["prefix_cache"]) is bool, "cache switch must be boolean")
    integer(setup["max_batches"], 5 if setup["prefix_cache"] else 6, 240, "batch budget")
    if expected_cache is not None:
        require(setup["prefix_cache"] is expected_cache, "expected cache profile drifted")
    positive_seconds(setup["setup_seconds"], "setup time")


def validate_records(records, gpu_ids, world=8, expected_hashes=None, expected_cache=None):
    require(world in (1, 2, 8) and type(world) is int, "unsupported expected world")
    require(type(records) is list and 1 < len(records) <= 64, "invalid JSONL record count")
    expected_hashes = {} if expected_hashes is None else expected_hashes
    setup = records[0]
    check_setup(setup, world, gpu_ids, expected_hashes, expected_cache)
    cache = setup["prefix_cache"]
    batches, order = timeline(cache)
    require(len(records) == len(order) + 2, "missing, duplicated or extra workload records")
    prompts = [PROMPT_IDS + REFERENCE_IDS[:prefix] for prefix in PREFIX_LENGTHS]
    expected_outputs = [REFERENCE_IDS[prefix:prefix + length]
                        for prefix, length in zip(PREFIX_LENGTHS, OUTPUT_LENGTHS, strict=True)]
    admissions, generated, timestamps, summaries = {}, [[] for _ in NAMES], [[] for _ in NAMES], {}
    progress = [0, 0, 0, 16 if cache else 0]
    last_time, row_count, mixed, full_batch, boundary = 0, 0, False, False, False
    dispatch = [544] + [540] * (world - 1)
    batch_durations = []
    retained = [1, 3, 4, 3, 2] if cache else [1, 3, 4, 2, 2, 2]
    for value, (action, index) in zip(records[1:-1], order, strict=True):
        slot, generation = REQUEST_IDS[index] if action != "batch" else (None, None)
        if action == "admit":
            record(value, "Admission", ADMISSION_FIELDS)
            require(value["name"] == NAMES[index] and integer(value["slot"], 0, 31) == slot
                    and integer(value["generation"], 1) == generation, "admitted generational identity drifted")
            require(integer(value["tick"], 0, 10000) == index, "arrival tick drifted")
            now = integer(value["arrival_ns"], last_time, U64_MAX, "admission clock")
            require(integer_list(value["prompt_tokens"], high=151935) == prompts[index], "admission prompt tokens drifted")
            hit = 16 if index == 3 and cache else 0
            require(integer(value["cached_tokens"], 0, 8191) == hit
                    and integer(value["cached_pages"], 0, 512) == hit // 16,
                    "initialized cached prefix drifted")
            admissions[index] = value
            last_time = now
        elif action == "batch":
            record(value, "Completed", BATCH_FIELDS)
            require(integer(value["tick"], 0, 10000) == index
                    and integer(value["batch_id"], 1) == index + 1
                    and integer(value["pool_batch_id"], 1) == index + 1,
                    "committed batch tick/generation drifted")
            start = integer(value["started_ns"], last_time, U64_MAX, "batch start")
            end = integer(value["completed_ns"], start + 1, U64_MAX, "batch completion")
            require(integer_list(value["rank_dispatch_counts"], 1) == dispatch, "rank dispatch schedule drifted")
            expected_rows, expected_events = [], []
            for request, begin, count in batches[index]:
                require(request in admissions and progress[request] == begin, "internal expected schedule drifted")
                for position in range(begin, begin + count):
                    prompt = prompts[request]
                    kind = ("PrefillIntermediate" if position + 1 < len(prompt) else
                            "PrefillFinal" if position < len(prompt) else "Decode")
                    token = prompt[position] if position < len(prompt) else generated[request][-1]
                    expected_rows.append({"slot": REQUEST_IDS[request][0], "generation": REQUEST_IDS[request][1],
                                          "token": token, "position": position, "kind": kind})
                    if kind != "PrefillIntermediate":
                        ordinal = len(generated[request])
                        choice = expected_outputs[request][ordinal]
                        generated[request].append(choice)
                        timestamps[request].append(end)
                        expected_events.append({"slot": REQUEST_IDS[request][0], "generation": REQUEST_IDS[request][1],
                                                "token": choice, "index": ordinal, "completed_ns": end,
                                                "finished": ordinal + 1 == REQUESTED_LENGTHS[request]})
                    boundary |= position == 16 and request in (0, 3)
                progress[request] += count
            require(type(value["rows"]) is list and 1 <= len(value["rows"]) <= 16, "invalid row count")
            for row in value["rows"]:
                fields(row, ROW_FIELDS, "row")
                for key, high in (("slot", 31), ("generation", U64_MAX), ("token", 151935), ("position", 8191)):
                    integer(row[key], 1 if key == "generation" else 0, high, key)
            require(value["rows"] == expected_rows, "causal token rows/order/kinds drifted")
            require(type(value["outputs"]) is list, "invalid output events")
            for event in value["outputs"]:
                fields(event, OUTPUT_FIELDS, "output event")
                for key, high in (("slot", 31), ("generation", U64_MAX), ("token", 151935), ("index", 255), ("completed_ns", U64_MAX)):
                    integer(event[key], 1 if key == "generation" else 0, high, key)
                require(type(event["finished"]) is bool, "output completion flag must be boolean")
            require(value["outputs"] == expected_events, "published output tokens/order/completion drifted")
            hits = 1 if cache and index >= 3 else 0
            expected_stats = {"retained_pages": retained[index], "free_pages": setup["physical_pages"] - retained[index],
                              "cached_pages": hits, "prefix_hits": hits, "hit_tokens": hits * 16, "evicted_pages": 0}
            for key, expected in expected_stats.items():
                require(integer(value[key]) == expected, f"committed {key} accounting drifted")
            kinds = {row["kind"] for row in expected_rows}
            mixed |= "Decode" in kinds and bool(kinds & {"PrefillIntermediate", "PrefillFinal"})
            full_batch |= len(expected_rows) == 16
            row_count += len(expected_rows)
            batch_durations.append(end - start)
            last_time = end
        else:
            record(value, "Request", REQUEST_FIELDS)
            admission = admissions[index]
            require(value["name"] == NAMES[index] and integer(value["slot"], 0, 31) == slot
                    and integer(value["generation"], 1) == generation, "retired request identity drifted")
            require(integer_list(value["prompt_tokens"], high=151935) == prompts[index]
                    and integer_list(value["generated_tokens"], high=151935) == expected_outputs[index]
                    and generated[index] == expected_outputs[index], "retired request tokens drifted")
            require(integer_list(value["output_timestamps_ns"]) == timestamps[index], "request/output timestamps disagree")
            for key in ("arrival_ns", "cached_prefix_tokens", "arrival_tick"):
                original = {"arrival_ns": "arrival_ns", "cached_prefix_tokens": "cached_tokens", "arrival_tick": "tick"}[key]
                require(integer(value[key]) == admission[original], f"request {key} drifted")
            text = "".join(REFERENCE_PIECES[PREFIX_LENGTHS[index]:PREFIX_LENGTHS[index] + OUTPUT_LENGTHS[index]])
            require(integer_list(value["generated_utf8_bytes"], high=255) == list(text.encode("utf-8"))
                    and value["generated_text"] == text, "decoded reference bytes drifted")
            if index == 2:
                require(value["state"] == "Cancelled", "cancelled request became complete")
                last_time = integer(value["cancelled_ns"], last_time, U64_MAX, "cancellation clock")
            else:
                require(value["state"] == "Completed" and value["cancelled_ns"] is None, "completion state drifted")
            intervals = [second - first for first, second in zip(timestamps[index], timestamps[index][1:])]
            require(all(interval > 0 for interval in intervals), "nonpositive decode interval")
            ttft = timestamps[index][0] - admission["arrival_ns"]
            require(integer(value["ttft_ns"], 1) == ttft
                    and integer_list(value["decode_intervals_ns"], 1) == intervals, "TTFT/interval arithmetic drifted")
            if intervals:
                require(integer(value["tpot_ns"], 1) == sum(intervals) // len(intervals), "TPOT arithmetic drifted")
            else:
                require(value["tpot_ns"] is None, "one-output cancellation cannot have TPOT")
            summaries[NAMES[index]] = {"state": value["state"], "generated_tokens": generated[index],
                                      "generated_text": text, "cached_prefix_tokens": admission["cached_tokens"],
                                      "ttft_ns": ttft, "decode_intervals_ns": intervals,
                                      "tpot_ns": value["tpot_ns"]}
    require(mixed and full_batch and boundary, "required mixed/full/page-crossing execution not observed")
    require(row_count == (34 if cache else 50), "total physical token rows drifted")
    closed = records[-1]
    record(closed, "Closed", CLOSED_FIELDS)
    require(integer_list(closed["worker_pids"], 1, (1 << 31) - 1) == setup["worker_pids"]
            and closed["all_workers_exited"] is True, "worker close/reap record drifted")
    counts = [count * len(batches) for count in dispatch]
    require(integer_list(closed["rank_dispatch_counts"], 1) == counts, "final dispatch totals drifted")
    whole = positive_seconds(closed["whole_seconds"], "whole-run time")
    require(whole + 0.000001 >= setup["setup_seconds"] + last_time / 1_000_000_000,
            "whole/setup/workload timing inconsistent")
    pinned = set(expected_hashes) == {"controller_sha256", "worker_sha256", "artifact_hsaco_id"}
    return {"schema": "FerricQwen3TpBatchComparisonV2", "authority": "none",
            "validation": "passed_bound_evidence" if pinned else "unpinned_not_pass",
            "passed": pinned, "tensor_parallel": world, "prefix_cache": cache,
            "model_bundle_id": BUNDLE, "batch_count": len(batches), "physical_token_rows": row_count,
            "generated_tokens": sum(OUTPUT_LENGTHS), "rank_dispatch_counts": counts,
            "batch_durations_ns": batch_durations, "setup_seconds": setup["setup_seconds"],
            "whole_seconds": whole, "requests": summaries,
            "identities": {key: setup[key] for key in ("controller_sha256", "worker_sha256", "artifact_hsaco_id",
                                                       "artifact_manifest_id", "artifact_handoff_id", "session_id")},
            "externally_pinned_identities": sorted(expected_hashes),
            "manifest_handoff_identity_scope": "self-reported; externally pin the admitted artifact receipt separately",
            "all_reference_tokens_and_bytes_match": True, "mixed_decode_prefill_observed": mixed,
            "sixteen_row_batch_observed": full_batch, "page_boundary_observed": boundary,
            "clean_teardown_recorded": True,
            "timing_scope": "one logical-tick workload; TTFT starts at actual admission; TPOT is integer mean of committed output intervals",
            "nonclaims": ["not a repeated or controlled benchmark", "no cache speedup claim",
                          "not HTTP serving or protected qualification", "not a numerical error bound",
                          "GPU idle snapshots and CLI close records are observations, not authenticated attestations"]}


def compare(run_dir, workload_path, reference_path, world=8, expected_hashes=None, expected_cache=None):
    run_dir = Path(run_dir)
    files = {"status": read_bounded(run_dir / "status", 16),
             "gpu-before.json": read_bounded(run_dir / "gpu-before.json", 65536),
             "gpu-after.json": read_bounded(run_dir / "gpu-after.json", 65536),
             "results.jsonl": read_bounded(run_dir / "results.jsonl", 8 * 1024 * 1024),
             "workload.json": read_bounded(workload_path, 1024 * 1024),
             "reference.json": read_bounded(reference_path, 128 * 1024)}
    require(files["status"] == b"0\n", "run did not exit successfully")
    load_reference(files["reference.json"])
    load_workload(files["workload.json"])
    before = gpu_roster(files["gpu-before.json"])
    require(before == gpu_roster(files["gpu-after.json"]), "physical GPU roster changed after run")
    raw = files["results.jsonl"]
    require(raw.endswith(b"\n"), "truncated JSONL final record")
    lines = raw.splitlines()
    require(len(lines) <= 64 and all(lines), "empty or excessive JSONL records")
    records = [json_value(line) for line in lines]
    report = validate_records(records, before, world, expected_hashes, expected_cache)
    report["input_sha256"] = {name: sha256(data) for name, data in files.items()}
    report["gpu_idle_before_and_after"] = True
    report["comparator_sha256"] = sha256(read_bounded(Path(__file__), 128 * 1024))
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", required=True, type=Path)
    parser.add_argument("--workload", required=True, type=Path)
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--expect-world", type=int, choices=(1, 2, 8), default=8)
    parser.add_argument("--expect-cache", choices=("on", "off"))
    parser.add_argument("--expect-controller-sha256")
    parser.add_argument("--expect-worker-sha256")
    parser.add_argument("--expect-artifact-sha256")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    expected = {key: value for key, value in (("controller_sha256", args.expect_controller_sha256),
                ("worker_sha256", args.expect_worker_sha256), ("artifact_hsaco_id", args.expect_artifact_sha256)) if value is not None}
    try:
        report = compare(args.run_dir, args.workload, args.reference, args.expect_world, expected,
                         None if args.expect_cache is None else args.expect_cache == "on")
        encoded = json.dumps(report, sort_keys=True, indent=2, allow_nan=False) + "\n"
        if args.output is not None:
            with args.output.open("x", encoding="utf-8") as output:
                output.write(encoded)
        else:
            print(encoded, end="")
        return 0 if report["passed"] else 2
    except (ValueError, KeyError, TypeError, IndexError, OSError) as error:
        parser.exit(1, f"TP workload comparison rejected: {error}\n")


if __name__ == "__main__":
    raise SystemExit(main())
