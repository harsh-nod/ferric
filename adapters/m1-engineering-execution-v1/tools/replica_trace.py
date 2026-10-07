"""Strict trace checks for the fixed eight-request replica experiment."""

import importlib.util
from pathlib import Path


SPEC = importlib.util.spec_from_file_location("replica_reference", Path(__file__).with_name("compare_tp_batch.py"))
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
require, integer, fields = BASE.require, BASE.integer, BASE.fields
NAMES = [f"replica-request-{index:02}" for index in range(8)]
TARGET_BYTES = 16_381_470_720
PROJECTION_BYTES = 13_891_534_848
RANK_NORM_BYTES = 608_256
GLOBAL_BYTES = 2_489_327_616
TRANSPOSE_BYTES = 15_136_194_560


def same_json(left, right):
    return BASE.json.dumps(left, sort_keys=True, allow_nan=False) == BASE.json.dumps(right, sort_keys=True, allow_nan=False)


def weight_payload(world, projection):
    return {"host_target": TARGET_BYTES,
            "device_base": PROJECTION_BYTES + world * RANK_NORM_BYTES + GLOBAL_BYTES,
            "device_transposed": TRANSPOSE_BYTES if projection in ("mfma", "auto") else 0}


def schedule(request_count, budget, chunk):
    """Independent bounded model of decode-first rotating fixed-workload rows."""
    integer(request_count, 1, 8, "request count")
    integer(budget, 1, 32, "row budget")
    integer(chunk, 1, budget, "prefill chunk")
    position, generated = [0] * request_count, [0] * request_count
    active = set(range(request_count))
    decode_cursor = prefill_cursor = 0
    prefer_prefill = False
    result = []
    while active:
        require(len(result) < 96, "fixed schedule exceeded physical row bound")
        decode = [index for offset in range(32)
                  if (index := (decode_cursor + offset) % 32) in active and position[index] >= 5]
        prefill = [index for offset in range(32)
                   if (index := (prefill_cursor + offset) % 32) in active and position[index] < 5]
        both = bool(decode and prefill)
        limit = (int(not prefer_prefill) if budget == 1 else budget - 1) if both else budget
        rows = []
        for index in decode[:limit]:
            rows.append((index, position[index], "Decode", BASE.REFERENCE_IDS[generated[index] - 1]))
            decode_cursor = (index + 1) % 32
        for index in prefill:
            count = min(budget - len(rows), chunk, 5 - position[index])
            if count == 0:
                break
            for offset in range(count):
                at = position[index] + offset
                rows.append((index, at, "PrefillFinal" if at == 4 else "PrefillIntermediate", BASE.PROMPT_IDS[at]))
            prefill_cursor = (index + 1) % 32
        if both and budget == 1:
            prefer_prefill = not prefer_prefill
        outputs = []
        for index, at, kind, _ in rows:
            require(at == position[index], "internal fixed schedule position mismatch")
            position[index] += 1
            if kind != "PrefillIntermediate":
                ordinal = generated[index]
                outputs.append((index, ordinal, BASE.REFERENCE_IDS[ordinal], ordinal == 7))
                generated[index] += 1
        retained = sum(position[index] > 0 for index in active)
        retired = sorted(index for index in active if generated[index] == 8)
        result.append({"rows": rows, "outputs": outputs, "retained_pages": retained, "retired": retired})
        active.difference_update(retired)
    require(sum(len(batch["rows"]) for batch in result) == request_count * 12, "fixed physical rows differ")
    return result


def setup_record(setup, expectation, replica, started):
    wide = expectation["kernel_profile"] in BASE.WIDE_KERNEL_PROFILES
    extras = {"output_head_pruning", "performance_profile", "replica_benchmark", "weight_payload_bytes"}
    if wide:
        extras |= {"kernel_profile", "kernel_row_capacity"}
    BASE.record(setup, "Setup", BASE.SETUP_FIELDS | extras)
    constants = {"model": "Qwen/Qwen3-8B", "dtype": "BF16", "target": "gfx950:xnack-",
                 "model_bundle_id": BASE.BUNDLE, "collective": BASE.LEGACY_COLLECTIVE,
                 "prefill": "true_multirow_chunked", "attention": "paged_causal_gqa",
                 "cache": "complete_page_radix_after_retirement",
                 "arrival_policy": "logical batch ticks; elapsed latency starts at admission",
                 "numerical_status": "Contracted; independently compare emitted token IDs; not a serving qualification"}
    for key, value in constants.items():
        require(setup[key] == value, f"replica setup {key} differs")
    require(setup["prefix_cache"] is False, "replica prefix cache must be disabled")
    require(setup["output_head_pruning"] is expectation["output_head_pruning"], "pruning differs")
    require(BASE.performance_profile(setup["performance_profile"]) == expectation["performance_profile"],
            "replica runtime/arithmetic profile differs")
    world = len(replica["device_unique_ids"])
    require(integer(setup["tensor_parallel"]) == world, "replica world differs")
    require(BASE.integer_list(setup["device_unique_ids"], 1) == replica["device_unique_ids"], "replica rank allocation differs")
    for key in BASE.IDENTITY_FIELDS:
        require(setup[key] == BASE.hash_value(expectation[key], key), f"replica {key} differs")
    BASE.hash_value(setup["session_id"], "session identity")
    require(type(setup["running_worker_sha256"]) is list
            and setup["running_worker_sha256"] == [expectation["worker_sha256"]] * world,
            "live worker executable identities differ")
    pids = BASE.integer_list(setup["worker_pids"], 1, (1 << 31) - 1, "worker PID")
    require(len(pids) == world and len(set(pids)) == world, "distinct per-rank workers required")
    require(started["pid"] not in pids, "controller cannot masquerade as rank worker")
    for key in ("batch_tokens", "prefill_chunk", "physical_pages", "context_tokens", "cache_ttl_ticks", "max_batches"):
        require(integer(setup[key]) == expectation[key], f"replica {key} policy differs")
    require(integer(setup["page_tokens"]) == 16, "replica page geometry differs")
    if wide:
        BASE.wide_kernel_profile(expectation["kernel_profile"], expectation["performance_profile"])
        require(setup["kernel_profile"] == expectation["kernel_profile"]
                and integer(setup["kernel_row_capacity"]) == 32, "wide image identity/capacity differs")
    require(same_json(setup["replica_benchmark"], started), "Setup is not bound to the control Started receipt")
    counters = setup["weight_payload_bytes"]
    fields(counters, {"host_target", "device_base", "device_transposed"}, "actual weight payload")
    require(all(type(value) is int for value in counters.values())
            and counters == weight_payload(world, expectation["performance_profile"]["projection"]),
            "actual loaded weight payload differs")
    BASE.positive_seconds(setup["setup_seconds"], "replica setup time")
    return pids


def validate_trace(records, expectation, replica, started, closed):
    require(type(records) is list and 4 <= len(records) <= 256, "bounded replica trace required")
    setup = records[0]
    pids = setup_record(setup, expectation, replica, started)
    names = replica["request_names"]
    batches = schedule(len(names), expectation["batch_tokens"], expectation["prefill_chunk"])
    require(len(batches) <= expectation["max_batches"], "fixed cohort exceeds configured batch limit")
    require(len(records) == 2 + 2 * len(names) + len(batches), "missing or extra replica trace records")
    require(expectation["physical_pages"] >= len(names), "insufficient simultaneous request page capacity")
    require(expectation["context_tokens"] >= 12, "insufficient fixed-request context capacity")
    arrival, timestamps = [], [[] for _ in names]
    cursor = 1
    last = started["lateness_ns"]
    final_offset = integer(closed["closed_ns"] - started["epoch_ns"], last, label="close epoch offset")
    for index, name in enumerate(names):
        value = records[cursor]
        cursor += 1
        BASE.record(value, "Admission", BASE.ADMISSION_FIELDS)
        require(value["name"] == name and integer(value["slot"]) == index
                and integer(value["generation"]) == 1 and integer(value["tick"]) == 0,
                "fixed replica admission order/identity differs")
        require(BASE.integer_list(value["prompt_tokens"]) == BASE.PROMPT_IDS
                and integer(value["cached_tokens"]) == 0 and integer(value["cached_pages"]) == 0,
                "replica prompt or uncached admission differs")
        last = integer(value["arrival_ns"], last, final_offset, "admission epoch offset")
        arrival.append(last)
    counts = [0] * len(pids)
    requests, durations = {}, []
    for tick, expected in enumerate(batches):
        value = records[cursor]
        cursor += 1
        BASE.record(value, "Completed", BASE.BATCH_FIELDS | {"output_head_rows"})
        require(integer(value["tick"]) == tick and integer(value["batch_id"]) == tick + 1
                and integer(value["pool_batch_id"]) == tick + 1, "batch clock/identity differs")
        begin = integer(value["started_ns"], last, final_offset, "batch start offset")
        end = integer(value["completed_ns"], begin + 1, final_offset, "batch completion offset")
        rows = [{"slot": index, "generation": 1, "token": token, "position": position, "kind": kind}
                for index, position, kind, token in expected["rows"]]
        require(type(value["rows"]) is list, "batch rows must be an array")
        for row in value["rows"]:
            fields(row, BASE.ROW_FIELDS, "replica row")
            for key in ("slot", "generation", "token", "position"):
                integer(row[key])
        require(value["rows"] == rows, "fixed replica row tokens/positions/order differ")
        events = [{"slot": index, "generation": 1, "token": token, "index": ordinal,
                   "completed_ns": end, "finished": finished}
                  for index, ordinal, token, finished in expected["outputs"]]
        require(type(value["outputs"]) is list, "batch outputs must be an array")
        for event in value["outputs"]:
            fields(event, BASE.OUTPUT_FIELDS, "replica output")
            for key in ("slot", "generation", "token", "index", "completed_ns"):
                integer(event[key])
            require(type(event["finished"]) is bool, "output terminal flag must be boolean")
        require(value["outputs"] == events, "fixed reference outputs/order differ")
        for index, _, _, _ in expected["outputs"]:
            timestamps[index].append(end)
        head_rows = len(events) if expectation["output_head_pruning"] else len(rows)
        require(integer(value["output_head_rows"]) == head_rows, "output-head extent differs")
        dispatch = [541 + (3 if head_rows else 0)] + [540] * (len(pids) - 1)
        require(BASE.integer_list(value["rank_dispatch_counts"]) == dispatch, "replica dispatch schedule differs")
        counts = [total + increment for total, increment in zip(counts, dispatch, strict=True)]
        stats = {"retained_pages": expected["retained_pages"],
                 "free_pages": expectation["physical_pages"] - expected["retained_pages"],
                 "cached_pages": 0, "prefix_hits": 0, "hit_tokens": 0, "evicted_pages": 0}
        for key, wanted in stats.items():
            require(integer(value[key]) == wanted, f"replica page accounting {key} differs")
        durations.append(end - begin)
        last = end
        for index in expected["retired"]:
            record = records[cursor]
            cursor += 1
            BASE.record(record, "Request", BASE.REQUEST_FIELDS)
            intervals = [right - left for left, right in zip(timestamps[index], timestamps[index][1:])]
            text = "".join(BASE.REFERENCE_PIECES[:8])
            require(record["name"] == names[index] and integer(record["slot"]) == index
                    and integer(record["generation"]) == 1 and record["state"] == "Completed",
                    "request retirement identity/order/state differs")
            for key, wanted in (("prompt_tokens", BASE.PROMPT_IDS), ("generated_tokens", BASE.REFERENCE_IDS[:8]),
                                ("generated_utf8_bytes", list(text.encode())),
                                ("output_timestamps_ns", timestamps[index]), ("decode_intervals_ns", intervals)):
                require(BASE.integer_list(record[key]) == wanted, f"request {key} differs")
            require(record["generated_text"] == text and record["cancelled_ns"] is None
                    and integer(record["cached_prefix_tokens"]) == 0 and integer(record["arrival_tick"]) == 0
                    and integer(record["arrival_ns"]) == arrival[index], "request semantics differ")
            ttft = timestamps[index][0] - arrival[index]
            require(integer(record["ttft_ns"]) == ttft and integer(record["tpot_ns"]) == sum(intervals) // 7,
                    "request latency arithmetic differs")
            requests[names[index]] = {"arrival_epoch_offset_ns": arrival[index],
                "output_epoch_offsets_ns": timestamps[index], "decode_interval_count": 7,
                "admission_ttft_ns": ttft, "release_to_first_token_ns": timestamps[index][0],
                "actual_start_to_first_token_ns": timestamps[index][0] - started["lateness_ns"],
                "tpot_seconds": sum(intervals) / 7 / 1e9, "output_tokens": 8}
    require(cursor == len(records) - 1, "unexpected terminal trace records")
    terminal = records[-1]
    BASE.record(terminal, "Closed", BASE.CLOSED_FIELDS | {"replica_benchmark"})
    require(BASE.integer_list(terminal["worker_pids"], 1) == pids and terminal["all_workers_exited"] is True
            and BASE.integer_list(terminal["rank_dispatch_counts"]) == counts
            and same_json(terminal["replica_benchmark"], closed), "worker close/control receipt differs")
    whole = BASE.positive_seconds(terminal["whole_seconds"], "replica whole time")
    require(whole > setup["setup_seconds"], "whole process time precedes setup")
    return {"replica_id": replica["replica_id"], "requests": requests, "worker_pids": pids,
            "session_id": setup["session_id"], "batch_count": len(batches), "physical_token_rows": len(names) * 12,
            "rank_dispatch_counts": counts, "batch_durations_ns": durations,
            "weight_payload_bytes": setup["weight_payload_bytes"], "setup_seconds": setup["setup_seconds"],
            "whole_seconds": whole, "last_output_epoch_offset_ns": last}
