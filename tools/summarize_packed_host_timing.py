#!/usr/bin/env python3
"""Validate one packed gate/up 128/128 host diagnostic, never GPU timing."""

import argparse
import hashlib
import json
from pathlib import Path


PROFILE = "prefill16-decode-ordered64-kv-packed-gate-up-r2-host-diagnostic-v1"
ORDERED = "dispatch_ordered_batch64"
LIMIT = 64 * 1024 * 1024
U64 = (1 << 64) - 1
HEADER = {
    "schema": "FerricOrdered64HostTimingV1",
    "clock": "controller-and-worker-std-instant-durations",
    "measurement": "host-wall-latency-not-gpu-duration",
    "aggregation": "overlapping-not-additive",
    "payload_accounting": "payload-only-excludes-wire-headers",
    "record_limit": 65536, "incomplete": False, "active_records": 0,
    "worker_reported_scope": "publication through completion checks; excludes preparation/staging; includes GPU work, polling and fences; not GPU duration",
    "scope_relationship": "packing is inside flush; send and worker elapsed are inside roundtrip; response wait overlaps worker elapsed; never sum nested scopes",
    "run_status": "completed", "failure": None, "workload_sha256": None,
}
KEYS = ("batch", "phase", "category", "label", "rank")
TOTALS = ("count", "failed", "elapsed_ns", "max_ns", "request_payload_bytes",
          "response_payload_bytes", "dispatches")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def exact(actual, expected, message):
    require(type(actual) is type(expected) and actual == expected, message)


def integer(value):
    require(type(value) is int and 0 <= value <= U64, "expected bounded unsigned integer")
    return value


def decode(raw):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=unique,
                      parse_constant=lambda _: require(False, "nonfinite JSON"))


def validate_workload(transcript, setup, closed, decode_packets):
    require(type(transcript) is list and len(transcript) >= 2, "native transcript")
    exact(transcript[0], setup, "transcript setup binding")
    exact(transcript[-1], closed, "transcript close binding")
    queued = admitted = completed = None
    tokens, timestamps = [], []
    batches = ready = drains = stops = 0
    pending_token = None
    for position, event in enumerate(transcript[1:-1]):
        require(type(event) is dict and event.get("schema") == "FerricQwen3TpLiveEventV1"
                and event.get("authority") == "none" and stops == 0, "live event identity/order")
        kind = event.get("event")
        if kind == "ready":
            require(position == 0 and ready == 0, "one initial ready event")
            ready += 1
        elif kind == "draining":
            require(ready == 1 and drains == 0 and event.get("reason") in ("command", "eof"), "normal drain")
            drains += 1
        elif kind == "stopped":
            require(completed is not None and drains == 1 and event.get("reason") == "drained",
                    "completed drained stop")
            exact(event.get("batches"), 135, "stopped batch count")
            stops += 1
        elif kind == "batch":
            require(admitted is not None and completed is None and batches < 135, "batch outside request")
            batches += 1
            expected_outputs = 0 if batches < 8 else 1
            for key, expected in {"batch_id": batches, "tick": batches - 1,
                                  "rows": 16 if batches <= 8 else 1,
                                  "outputs": expected_outputs, "output_head_rows": expected_outputs,
                                  "rank_dispatch_counts": [613 if batches < 8 else
                                                           616 if batches == 8 else decode_packets]}.items():
                exact(event.get(key), expected, "batch sequence/geometry: " + key)
            require((pending_token == integer(event.get("completed_ns"))) if expected_outputs
                    else pending_token is None, "token does not match producing batch")
            pending_token = None
        else:
            require(kind in ("queued", "admission", "token", "request") and ready == 1,
                    "unexpected/rejected request event")
            if kind == "queued":
                require(queued is None and admitted is None and batches == 0, "single queued request")
                require(integer(event.get("request_id")) > 0 and type(event.get("name")) is str,
                        "request identity")
                exact(event.get("prompt_tokens"), 128, "queued prompt length")
                queued = event
                continue
            require(queued is not None and completed is None, "request event outside lifetime")
            for key in ("request_id", "name"):
                exact(event.get(key), queued[key], "single request identity")
            if kind == "admission":
                require(admitted is None and batches == 0, "single admission")
                prompt = event.get("prompt_tokens")
                require(type(prompt) is list and len(prompt) == 128
                        and all(integer(token) < 151936 for token in prompt), "128 prompt token IDs")
                for key, expected in {"prompt_token_count": 128, "cached_tokens": 0, "cached_pages": 0}.items():
                    exact(event.get(key), expected, "cold admission: " + key)
                admitted = event
            elif kind == "token":
                require(admitted is not None and len(tokens) < 128 and pending_token is None
                        and batches == len(tokens) + 7, "token/batch sequence")
                exact(event.get("index"), len(tokens), "token index")
                exact(event.get("finished"), len(tokens) == 127, "final token marker")
                tokens.append(integer(event.get("token")))
                require(tokens[-1] < 151936, "vocabulary token ID")
                pending_token = integer(event.get("completed_ns"))
                require(not timestamps or timestamps[-1] <= pending_token, "monotonic output timestamps")
                timestamps.append(pending_token)
            else:
                require(admitted is not None and batches == 135 and len(tokens) == 128
                        and pending_token is None, "one complete 128/128 request")
                for key, expected in {"admitted": True, "state": "Completed", "cancelled_ns": None,
                                      "prompt_token_count": 128, "prompt_tokens": admitted["prompt_tokens"],
                                      "generated_tokens": tokens, "output_timestamps_ns": timestamps,
                                      "cached_prefix_tokens": 0}.items():
                    exact(event.get(key), expected, "completed request: " + key)
                completed = event
    require(ready == drains == stops == 1 and completed is not None, "complete single-request transcript")


def analyze(value, transcript=None):
    require(type(value) is dict and set(value) == set(HEADER) | {
        "controller_pid", "setup", "closed", "records"}, "closed sidecar schema")
    for key, expected in HEADER.items():
        exact(value[key], expected, "sidecar field: " + key)
    require(0 < integer(value["controller_pid"]) <= 0xffffffff, "controller PID")
    setup, closed = value["setup"], value["closed"]
    for record, schema in ((setup, "FerricQwen3TpBatchSetupV2"),
                           (closed, "FerricQwen3TpBatchClosedV2")):
        require(type(record) is dict, "setup/close object")
        for key, expected in {"schema": schema, "live_profile": PROFILE,
                              "host_timing_schema": HEADER["schema"],
                              "diagnostic_max_model_batches": 256}.items():
            exact(record.get(key), expected, "diagnostic identity: " + key)
    for key, expected in {"tensor_parallel": 1, "context_tokens": 8192,
                          "prefill_chunk": 16, "prefix_cache": False,
                          "head_precision": "fp32-v8"}.items():
        exact(setup.get(key), expected, "workload identity: " + key)
    require(135 <= integer(setup.get("max_batches")) <= 256, "bounded batch budget")
    for key in ("execution_completed", "all_workers_exited"):
        exact(closed.get(key), True, "incomplete diagnostic")
    packed = setup.get("packed_gate_up")
    require(type(packed) is dict and packed == closed.get("packed_gate_up"), "packed setup/close identity")
    mode = packed.get("actual_mode")
    require(mode in ("baseline", "packed-gate-up-u32-r2"), "explicit packed selection")
    selected = mode != "baseline"
    for key, expected in {
        "schema": "FerricPackedGateUpKvLiveSelectionR2", "requested_mode": mode,
        "additional_weight_bytes": 7247757312, "activation_scratch_bytes": 8192,
        "loaded_image_count": 10, "selected_rows": 1, "selected_published_rows": 1,
        "roles": [4, 5], "extra_packets_per_selected_forward": 36 if selected else 0,
        "ffn_producer_packets_per_selected_layer": 6 if selected else 5,
    }.items():
        exact(packed.get(key), expected, "packed identity: " + key)
    decode_packets = 688 if selected else 652
    total_packets = 4907 + 127 * decode_packets
    exact(closed.get("rank_dispatch_counts"), [total_packets], "closed dispatch count")
    if transcript is not None:
        validate_workload(transcript, setup, closed, decode_packets)
    rows = value["records"]
    require(type(rows) is list and 0 < len(rows) <= 65536, "bounded aggregate records")
    seen = set()
    for row in rows:
        require(type(row) is dict and set(row) == set(KEYS) | set(TOTALS), "closed aggregate schema")
        require(row["batch"] is None or 1 <= integer(row["batch"]) <= 135, "one-request batch domain")
        require(row["rank"] is None or type(row["rank"]) is int and row["rank"] == 0, "TP1 rank")
        require(all(type(row[key]) is str and 0 < len(row[key]) <= 96
                    for key in ("phase", "category", "label")), "bounded labels")
        require(row["category"] in ("span", "ipc_send", "ipc_roundtrip", "worker_reported_elapsed"), "category")
        signature = tuple(row[key] for key in KEYS)
        require(signature not in seen, "duplicate aggregate")
        seen.add(signature)
        for key in TOTALS:
            integer(row[key])
        require(row["count"] > 0 and row["failed"] == 0 and row["max_ns"] <= row["elapsed_ns"]
                <= row["count"] * row["max_ns"], "failed/inconsistent aggregate")
        if row["category"] == "span":
            require(all(row[key] == 0 for key in TOTALS[-3:]), "span transport counters")
        if row["category"] == "worker_reported_elapsed":
            require(row["label"] == ORDERED and row["rank"] == 0 and row["batch"] is not None
                    and row["request_payload_bytes"] == row["response_payload_bytes"] == 0
                    and row["count"] <= row["dispatches"] <= 64 * row["count"], "worker interval")
    total = lambda records, key: integer(sum(row[key] for row in records))
    batches = []
    for batch in range(1, 136):
        selected_rows = [row for row in rows if row["batch"] == batch]
        subset = lambda category, label: [row for row in selected_rows
                                          if row["category"] == category and row["label"] == label]
        outer = subset("span", "batch")
        require(len(outer) == 1 and outer[0]["count"] == 1 and outer[0]["rank"] is None
                and outer[0]["phase"] == "batch", "one enclosing batch")
        worker = subset("worker_reported_elapsed", ORDERED)
        ordered = subset("ipc_roundtrip", ORDERED)
        packing = subset("span", "ordered_dispatch_pack")
        transport = [row for row in selected_rows if row["category"] == "ipc_roundtrip"]
        groups, packets = (72, 613 if batch < 8 else 616) if batch <= 8 else (11, decode_packets)
        ordered_packets = 612 if batch <= 8 else decode_packets
        require(total(worker, "count") == total(ordered, "count") == total(packing, "count") == groups,
                "ordered group counts")
        require(total(worker, "dispatches") == total(ordered, "dispatches") == ordered_packets
                and total(transport, "dispatches") == packets, "packet schedule")
        require(all(row["label"] == ORDERED or row["label"] == "dispatch" and batch <= 8
                    for row in transport if row["dispatches"]), "unexpected dispatch mode")
        for item in worker:
            covering = [row for row in ordered if (row["phase"], row["rank"]) == (item["phase"], item["rank"])]
            require(len(covering) == 1 and covering[0]["count"] == item["count"]
                    and covering[0]["dispatches"] == item["dispatches"]
                    and item["elapsed_ns"] <= covering[0]["elapsed_ns"], "worker covering transport")
        for item in packing:
            covering = [row for row in ordered if (row["phase"], row["rank"]) == (item["phase"], item["rank"])]
            require(item["rank"] == 0 and len(covering) == 1
                    and item["count"] == covering[0]["count"], "packing covering transport")
        wall = outer[0]["elapsed_ns"]
        pack_ns, worker_ns = total(packing, "elapsed_ns"), total(worker, "elapsed_ns")
        ordered_ns, transport_ns = total(ordered, "elapsed_ns"), total(transport, "elapsed_ns")
        require(pack_ns + transport_ns <= wall, "packing/transport exceed enclosing batch")
        batches.append({"batch": batch, "batch_wall_ns": wall,
                        "controller_ordered_packing_ns": pack_ns,
                        "worker_publication_completion_wall_ns": worker_ns,
                        "ordered_roundtrip_minus_worker_ns": ordered_ns - worker_ns,
                        "other_transport_roundtrip_ns": transport_ns - ordered_ns,
                        "remaining_batch_wall_ns": wall - pack_ns - transport_ns})
    summaries = {}
    for name, cohort in (("prefill", batches[:8]), ("decode", batches[8:])):
        summaries[name] = {"batches": len(cohort), **{
            key: total(cohort, key) for key in batches[0] if key != "batch"}}
    return {"schema": "FerricPackedHostTimingAnalysisV1", "mode": mode,
            "performance_qualified": False, "gpu_time_measured": False, "numerical_correctness_verified": False,
            "transcript_bound": transcript is not None, "workload_verified": transcript is not None,
            "workload": {"prompt_tokens": 128, "output_tokens": 128, "requests": 1}
            if transcript is not None else None,
            "dispatches": total_packets, "ordered_groups": 1973,
            "summaries": summaries, "batches": batches,
            "scope": "Instrumented single request; batch sums exclude setup, between-batch gaps and teardown.",
            "partition_semantics": {
                "worker_publication_completion_wall_ns": "GPU work, publication, polling and fences; not GPU time",
                "ordered_roundtrip_minus_worker_ns": "Worker preparation/staging and transport/controller overhead; not pure IPC",
                "remaining_batch_wall_ns": "Remaining batch wall time including instrumentation; not exclusively CPU compute",
                "overlap": "Response waits overlap worker durations; nested timers are not added to this partition"}}


def read_bound(path, digest):
    require(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= LIMIT, "bounded regular input")
    raw = path.read_bytes()
    require(len(raw) <= LIMIT and hashlib.sha256(raw).hexdigest() == digest, "input hash mismatch")
    return raw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sidecar", type=Path, required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--transcript", type=Path, required=True)
    parser.add_argument("--transcript-sha256", required=True)
    args = parser.parse_args()
    sidecar = decode(read_bound(args.sidecar, args.sha256))
    transcript = [decode(line) for line in read_bound(args.transcript, args.transcript_sha256).splitlines()]
    result = analyze(sidecar, transcript)
    result.update(sidecar_sha256=args.sha256, transcript_sha256=args.transcript_sha256)
    print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))


if __name__ == "__main__":
    main()
