#!/usr/bin/env python3
"""Join pinned C1 live events and host diagnostics into a bounded host timeline.

The --manifest JSON has schema FerricLiveRuntimeProfileInputsV1, inputs containing
events/host_timing/diagnostics {path, sha256} bindings, and expected_identity with
controller_sha256, worker_sha256, model_bundle_id, target_model_id, session_id,
layer_projection, worker_pids and device_unique_ids. Paths must be absolute.
This is structural diagnostic validation, not token-reference validation,
runtime admission, GPU profiling, or a Ferric/vLLM performance comparison.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re


PROFILE = "layer-c1-wave-runtime-diagnostic-v1"
EVENT = "FerricQwen3TpLiveEventV1"
ENVELOPE = "FerricLayerC1WaveRuntimeDiagnosticV1"
SCOPE = "cumulative overlapping worker host-wall counters; not GPU timestamps"
ACCOUNTING = ("workload command delta equals one earlier snapshot command plus successful workload IPC "
              "round trips; an ordered command carries 1..16 dispatches")
FALSE_FLAGS = dict(performance_qualified=False, benchmark_qualified=False, serving_qualified=False)
IDENTITY = {"controller_sha256", "worker_sha256", "model_bundle_id", "target_model_id", "session_id",
            "layer_projection", "worker_pids", "device_unique_ids"}
COUNTERS = set(("commands command_ns full_currentness_checks full_currentness_ns "
                "operational_currentness_checks operational_currentness_ns kernel_admissions kernel_admission_ns "
                "dispatches dispatch_prepare_ns dispatch_publish_ns dispatch_wait_ns completion_polls "
                "reads read_bytes read_ns writes write_bytes write_ns").split())
KEYS = ("batch", "phase", "category", "label", "rank")
TOTALS = ("count", "failed", "elapsed_ns", "max_ns", "request_payload_bytes", "response_payload_bytes", "dispatches")
MAX_RECORDS = 65536
MAX_BATCHES = 4096
MAX_REQUESTS = 4096
MAX_BYTES = 64 * 1024 * 1024
NON_WORKLOAD = {"setup", "runtime_snapshot_before", "runtime_snapshot_after", "runtime_diagnostic_close"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def uint(value, minimum=0, maximum=(1 << 64) - 1):
    require(type(value) is int and minimum <= value <= maximum, "bounded exact integer required")
    return value


def exact(actual, expected, label):
    require(type(actual) is type(expected) and canonical(actual) == canonical(expected), label + " mismatch")


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def fields(value, expected):
    require(type(value) is dict, "object required")
    for key, wanted in expected.items():
        require(key in value, "missing " + key)
        exact(value[key], wanted, key)


def strict_json(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result

    def nonfinite(_):
        raise ValueError("nonfinite JSON")

    value = json.loads(raw, object_pairs_hook=pairs, parse_constant=nonfinite)
    canonical(value)  # Reject finite-looking JSON numbers that overflow to inf.
    return value


def json_lines(raw, diagnostics=False):
    require(type(raw) is bytes and 0 < len(raw) <= MAX_BYTES and raw.endswith(b"\n"), "bounded complete JSONL required")
    lines = raw.decode("utf-8", errors="strict").splitlines()
    require(0 < len(lines) <= MAX_RECORDS, "JSONL record bound exceeded")
    result = []
    for line in lines:
        require(0 < len(line.encode("utf-8")) <= 1024 * 1024, "JSONL line bound exceeded")
        if not diagnostics or line.lstrip().startswith("{") or ENVELOPE in line:
            result.append(strict_json(line))
    return result


def validate_identity(setup, closed, expected):
    require(type(expected) is dict and set(expected) == IDENTITY, "exact expected identity required")
    for key in IDENTITY - {"layer_projection", "worker_pids", "device_unique_ids"}:
        require(type(expected[key]) is str and re.fullmatch("[0-9a-f]{64}", expected[key]), "identity SHA256 required")
    require(expected["layer_projection"] in ("mfma", "c1-wave"), "unsupported layer projection")
    for key in ("worker_pids", "device_unique_ids"):
        require(type(expected[key]) is list and len(expected[key]) == 1, "one pinned worker/device required")
        uint(expected[key][0], 1)
    common = dict(FALSE_FLAGS, authority="none", live_profile=PROFILE, submission="ordered", runtime_profiling=True,
                  layer_projection=expected["layer_projection"])
    fields(setup, dict(common, **expected, schema="FerricQwen3TpBatchSetupV2", tensor_parallel=1,
                       running_worker_sha256=[expected["worker_sha256"]], runtime_diagnostic_scope=SCOPE,
                       command_accounting=ACCOUNTING, prefix_cache=False, context_tokens=8192, physical_pages=512))
    profile_expected = dict(common, runtime_ordered_batches=True,
           dispatch_sequences=False, runtime_cache_admission=True, runtime_operational=True,
           queue_rollover=True, projection="mfma", attention="wave", argmax_mode="wave-v11")
    del profile_expected["authority"]
    fields(setup.get("performance_profile"), profile_expected)
    require(set(setup["performance_profile"]) == set(profile_expected), "closed diagnostic performance profile required")
    fields(closed, dict(common, schema="FerricQwen3TpBatchClosedV2", runtime_diagnostics_completed=True,
                       all_workers_exited=True, execution_completed=True, worker_pids=expected["worker_pids"]))
    return common


def event_timeline(events, setup, closed):
    require(type(events) is list and 4 <= len(events) <= MAX_RECORDS, "bounded complete live trace required")
    requests, active, batches, waiting_outputs = {}, {}, {}, []
    emitted, previous_end, previous_batch_emitted, last_non_token_emitted = 0, None, 0, 0
    ready, draining, stopped = False, False, False
    for event in events[1:-1]:
        fields(event, dict(schema=EVENT, authority="none"))
        timestamp = uint(event.get("emission_started_ns"))
        require(timestamp >= emitted and not stopped, "live event clock/order regressed")
        emitted = timestamp
        kind = event.get("event")
        require(not waiting_outputs or kind in ("token", "batch"), "token group missing its batch")
        if kind not in ("token", "batch"):
            last_non_token_emitted = timestamp
        if kind == "ready":
            require(not ready and not requests and not batches, "duplicate/out-of-order ready")
            fields(event, dict(clock="monotonic_ns_since_live_start", context_tokens=8192, physical_pages=512,
                               max_batches=setup["max_batches"], max_active_requests=32, max_pending_requests=32,
                               eos_policy="fixed output count"))
            ready = True
            continue
        require(ready, "missing ready event")
        if kind == "queued":
            request_id = uint(event.get("request_id"), 1)
            require((not requests or request_id > next(reversed(requests))) and len(requests) < MAX_REQUESTS
                    and not draining, "request roster overflow/reuse/regression")
            arrival, queued = uint(event.get("arrival_ns")), uint(event.get("queued_ns"))
            require(arrival <= queued <= timestamp, "queue clock order")
            requests[request_id] = dict(request_id=request_id, arrival_ns=arrival, queued_ns=queued,
                                        admitted_ns=None, outputs=[], completed_ns=None)
        elif kind == "admission":
            request = requests[uint(event.get("request_id"), 1)]
            require(request["admitted_ns"] is None and len(active) < 32, "duplicate/overflow admission")
            arrival, admitted = request["arrival_ns"], uint(event.get("admitted_ns"))
            fields(event, dict(arrival_ns=arrival, queue_wait_ns=admitted - arrival, cached_tokens=0, cached_pages=0))
            require(request["queued_ns"] <= admitted <= timestamp, "admission clock order")
            owner = (uint(event.get("slot"), 0, 31), uint(event.get("generation"), 1))
            require(all(existing[0] != owner[0] for existing in active), "duplicate active slot")
            prompt = event.get("prompt_tokens")
            require(type(prompt) is list and 0 < len(prompt) <= 8192, "prompt token bound")
            for token in prompt:
                uint(token, 0, 151935)
            fields(event, dict(prompt_token_count=len(prompt)))
            active[owner] = request["request_id"]
            request.update(admitted_ns=admitted, queue_wait_ns=admitted - arrival, owner=owner,
                           prompt_tokens=prompt)
        elif kind == "token":
            request_id = uint(event.get("request_id"), 1)
            request = requests[request_id]
            owner = (uint(event.get("slot"), 0, 31), uint(event.get("generation"), 1))
            require(active.get(owner) == request_id and len(request["outputs"]) < 8192, "token ownership/bound")
            fields(event, dict(index=len(request["outputs"])))
            completed = uint(event.get("completed_ns"))
            require(request["admitted_ns"] <= completed <= timestamp, "token clock order")
            require(not request["outputs"] or not request["outputs"][-1]["finished"], "token after finished")
            require(type(event.get("finished")) is bool, "token completion flag")
            token = dict(index=event["index"], token=uint(event.get("token"), 0, 151935), completed_ns=completed,
                         finished=event["finished"], decoded_bytes=event.get("decoded_bytes"), batch_id=None)
            require(type(token["decoded_bytes"]) is list and len(token["decoded_bytes"]) <= 32768, "token byte bound")
            for byte in token["decoded_bytes"]:
                uint(byte, 0, 255)
            request["outputs"].append(token)
            waiting_outputs.append((request_id, token))
        elif kind == "batch":
            batch_id = uint(event.get("batch_id"), 1)
            require(batch_id not in batches and len(batches) < MAX_BATCHES, "batch roster overflow/reuse")
            require(not batches or batch_id > next(reversed(batches)), "batch ID regressed")
            fields(event, dict(tick=len(batches), outputs=len(waiting_outputs)))
            start, end = uint(event.get("started_ns")), uint(event.get("completed_ns"))
            require(max(previous_batch_emitted, last_non_token_emitted) <= start <= end <= timestamp and active,
                    "batch clock/active roster")
            rows = uint(event.get("rows"), 1, 32)
            heads = uint(event.get("output_head_rows"), 0, rows)
            require(heads == len(waiting_outputs), "batch output-head count mismatch")
            require(len({request for request, _ in waiting_outputs}) == len(waiting_outputs), "duplicate request output in batch")
            counts = event.get("rank_dispatch_counts")
            require(type(counts) is list and len(counts) == 1, "batch rank count")
            uint(counts[0], 1)
            for _, output in waiting_outputs:
                require(output["completed_ns"] == end, "token/batch completion clock mismatch")
                output["batch_id"] = batch_id
            batches[batch_id] = dict(batch_id=batch_id, tick=event["tick"], started_ns=start, completed_ns=end,
                host_interval_ns=end - start, between_batch_host_gap_ns=None if previous_end is None else start - previous_end,
                rows=rows, output_head_rows=heads, output_request_ids=[r for r, _ in waiting_outputs],
                active_request_ids=sorted(active.values()), dispatches=counts[0], gpu_duration_ns=None,
                host_observations=[])
            waiting_outputs.clear()
            previous_end, previous_batch_emitted = end, timestamp
        elif kind == "request":
            request = requests[uint(event.get("request_id"), 1)]
            fields(event, dict(state="Completed", admitted=True, cancelled_ns=None, cached_prefix_tokens=0,
                               arrival_ns=request["arrival_ns"], prompt_tokens=request["prompt_tokens"]))
            owner = (uint(event.get("slot"), 0, 31), uint(event.get("generation"), 1))
            require(active.get(owner) == request["request_id"] and request["outputs"] and request["outputs"][-1]["finished"],
                    "incomplete or unowned request")
            outputs = request["outputs"]
            times = [output["completed_ns"] for output in outputs]
            require(times == sorted(times), "request output clock regressed")
            require(sum(len(output["decoded_bytes"]) for output in outputs) <= 32768, "request output byte bound")
            fields(event, dict(generated_tokens=[output["token"] for output in outputs], output_timestamps_ns=times,
                               generated_utf8_bytes=[byte for output in outputs for byte in output["decoded_bytes"]],
                               ttft_ns=times[0] - request["arrival_ns"]))
            request.update(completed_ns=times[-1], terminal_event_ns=timestamp)
            del active[owner]
        elif kind == "draining":
            require(not draining, "duplicate draining event")
            fields(event, dict(reason="command"))
            draining = True
        elif kind == "stopped":
            fields(event, dict(reason="drained", batches=len(batches)))
            require(draining and not active and all(r["completed_ns"] is not None for r in requests.values()),
                    "incomplete request lifecycle")
            stopped = True
        else:
            raise ValueError("unsupported event: completed uncancelled diagnostic runs only")
    require(stopped and batches and not waiting_outputs, "incomplete live trace")
    fields(closed, dict(rank_dispatch_counts=[sum(b["dispatches"] for b in batches.values())]))
    return requests, batches


def host_observations(timing, events, batches):
    require(type(timing) is dict and set(timing) == {
        "schema", "clock", "measurement", "aggregation", "payload_accounting", "record_limit", "incomplete",
        "active_records", "run_status", "failure", "setup", "closed", "workload_sha256", "controller_pid", "records"},
        "host timing field set")
    fields(timing, dict(schema="FerricHostTimingV1", clock="controller-std-instant",
        measurement="host-wall-latency-not-gpu-duration", aggregation="overlapping-not-additive",
        payload_accounting="payload-only-excludes-wire-headers", record_limit=65536, incomplete=False,
        active_records=0, run_status="completed", failure=None, setup=events[0], closed=events[-1], workload_sha256=None))
    uint(timing.get("controller_pid"), 1)
    records = timing.get("records")
    require(type(records) is list and 0 < len(records) <= MAX_RECORDS, "host record bound")
    seen, sends, receipts, physical, global_spans, workload = set(), {}, {}, set(), {}, []
    controller_batches = 0
    for row in records:
        require(type(row) is dict and set(row) == set(KEYS + TOTALS), "host aggregate field set")
        for key in TOTALS:
            uint(row[key], 1 if key == "count" else 0)
        require(row["failed"] == 0 and row["max_ns"] <= row["elapsed_ns"] <= row["max_ns"] * row["count"],
                "failed/inconsistent host aggregate")
        require(row["rank"] is None or type(row["rank"]) is int and row["rank"] == 0, "host worker rank")
        require(row["batch"] is None or type(row["batch"]) is int and row["batch"] in batches, "unknown host batch")
        require(all(type(row[k]) is str and re.fullmatch("[a-z][a-z0-9_]{0,95}", row[k]) for k in ("phase", "label")),
                "host label/phase")
        require(row["category"] in ("span", "ipc_send", "ipc_roundtrip"), "host category")
        key = tuple(row[k] for k in KEYS)
        require(key not in seen, "duplicate host aggregate")
        seen.add(key)
        if row["batch"] is not None:
            require(row["phase"] not in NON_WORKLOAD, "batch escaped workload")
            batches[row["batch"]]["host_observations"].append(dict(row))
        if row["category"] == "span":
            require(row["dispatches"] == row["request_payload_bytes"] == row["response_payload_bytes"] == 0, "span counts bytes/dispatches")
            if row["label"] == "batch":
                require(row["batch"] in batches and row["batch"] not in physical and row["count"] == 1
                        and row["rank"] is None and row["phase"] == "batch", "physical batch span")
                require(row["elapsed_ns"] <= batches[row["batch"]]["host_interval_ns"],
                        "physical batch span exceeds live interval")
                physical.add(row["batch"])
            if row["label"] == "controller_batch":
                require(row["batch"] is None and row["rank"] is None, "controller batch scope")
                controller_batches += row["count"]
            if row["label"] in NON_WORKLOAD | {"workload"}:
                require(row["batch"] is None and row["rank"] is None and row["count"] == 1
                        and row["phase"] == row["label"] and row["label"] not in global_spans, "global phase span")
                global_spans[row["label"]] = row
            continue
        require(type(row["rank"]) is int and row["rank"] == 0, "missing IPC worker rank")
        ipc_key = (row["batch"], row["phase"], row["label"], row["rank"])
        if row["category"] == "ipc_send":
            require(row["response_payload_bytes"] == row["dispatches"] == 0, "send response/dispatch counts")
            sends[ipc_key] = row
            continue
        receipts[ipc_key] = row
        if row["batch"] is not None or row["phase"] in ("workload", "controller_batch"):
            require(row["label"] in ("write", "read", "rollover_queue", "dispatch", "dispatch_ordered_batch"), "unknown workload IPC")
            workload.append(row)
        else:
            require(row["phase"] in NON_WORKLOAD, "unattributed IPC")
        if row["label"] == "dispatch_ordered_batch":
            require(row["count"] <= row["dispatches"] <= 16 * row["count"], "ordered dispatch bound")
        elif row["label"] == "dispatch":
            require(row["dispatches"] == row["count"], "dispatch count")
        else:
            require(row["dispatches"] == 0, "unexpected dispatch route")
    require(physical == set(batches) and controller_batches == len(batches), "missing/excess batch host scopes")
    require(set(global_spans) == NON_WORKLOAD | {"workload"}, "missing whole-path host scopes")
    require(set(sends) == set(receipts), "send/receipt roster mismatch")
    for key, sent in sends.items():
        received = receipts[key]
        require(all(sent[k] == received[k] for k in ("count", "request_payload_bytes"))
                and sent["elapsed_ns"] <= received["elapsed_ns"], "send/receipt accounting mismatch")
    for phase in ("runtime_snapshot_before", "runtime_snapshot_after"):
        boundary = [r for r in receipts.values() if r["phase"] == phase]
        require(len(boundary) == 1, "snapshot IPC boundary count")
        fields(boundary[0], dict(batch=None, label="other", count=1, request_payload_bytes=0, response_payload_bytes=0))
    for batch in batches.values():
        require(sum(r["dispatches"] for r in workload if r["batch"] == batch["batch_id"]) == batch["dispatches"],
                "per-batch dispatch accounting mismatch")
    return workload, [dict(r) for r in records if r["batch"] is None]


def counter_delta(envelopes, setup, common, workload, batches):
    require(type(envelopes) is list and len(envelopes) == 2, "exactly two snapshots required")
    values = []
    for ordinal, envelope in enumerate(envelopes):
        expected = dict(common, schema=ENVELOPE, phase=("before_workload", "after_workload")[ordinal],
                        measurement=SCOPE, command_accounting=ACCOUNTING)
        del expected["submission"]
        fields(envelope, expected)
        require(set(envelope) == set(expected) | {"snapshots"}, "snapshot envelope field set")
        snapshots = envelope["snapshots"]
        require(type(snapshots) is list and len(snapshots) == 1, "one worker snapshot required")
        snapshot = snapshots[0]
        expected = dict(schema="FerricRuntimeDiagnosticSnapshotV1", authority="none", performance_qualified=False,
            scope="cumulative overlapping worker host-wall counters, not GPU timestamps", process_id=setup["worker_pids"][0],
            device_unique_id=setup["device_unique_ids"][0], rank=0, ordinal=ordinal)
        fields(snapshot, expected)
        require(set(snapshot) == set(expected) | {"counters"}, "worker snapshot field set")
        counters = snapshot["counters"]
        require(type(counters) is dict and set(counters) == COUNTERS, "counter field set")
        values.append({key: uint(counters[key]) for key in sorted(COUNTERS)})
    before, after = values
    require(all(after[key] >= before[key] for key in COUNTERS), "worker counters regressed")
    delta = {key: after[key] - before[key] for key in sorted(COUNTERS)}
    require(before["dispatches"] == 0 and delta["dispatches"] == sum(b["dispatches"] for b in batches.values())
            == sum(r["dispatches"] for r in workload), "worker dispatch accounting mismatch")
    require(delta["commands"] == 1 + sum(r["count"] for r in workload), "snapshot command accounting mismatch")
    for label, count, byte_count, payload in (("read", "reads", "read_bytes", "response_payload_bytes"),
                                            ("write", "writes", "write_bytes", "request_payload_bytes")):
        rows = [row for row in workload if row["label"] == label]
        require(delta[count] == sum(r["count"] for r in rows) and delta[byte_count] == sum(r[payload] for r in rows),
                "worker I/O accounting mismatch")
        require(delta[count] != 0 or delta[label + "_ns"] == 0,
                "worker I/O duration without operation")
    return delta


def profile(events, timing, envelopes, expected):
    require(type(events) is list and len(events) >= 4, "complete live events required")
    setup, closed = events[0], events[-1]
    common = validate_identity(setup, closed, expected)
    requests, batches = event_timeline(events, setup, closed)
    workload, globals_ = host_observations(timing, events, batches)
    delta = counter_delta(envelopes, setup, common, workload, batches)
    timeline = []
    for request in requests.values():
        timeline.append({key: request[key] for key in ("request_id", "arrival_ns", "queued_ns", "admitted_ns",
                                                       "queue_wait_ns", "completed_ns", "terminal_event_ns")})
        timeline[-1]["outputs"] = [{key: output[key] for key in ("index", "completed_ns", "batch_id")}
                                   for output in request["outputs"]]
    return dict(schema="FerricLiveRuntimeProfileV1", authority="none", **FALSE_FLAGS, numerical_qualified=False,
        identity=expected, gpu_duration_ns=None, clock="monotonic_ns_since_live_start",
        measurement="host-wall-latency-not-gpu-duration", aggregation="overlapping-not-additive",
        requests=timeline, batches=list(batches.values()), global_host_observations=globals_, worker_counter_delta=delta,
        nonclaims=["No GPU timestamps were collected; GPU duration is unavailable, not zero.",
            "Batch endpoints and request events share the live clock; host phase aggregates have durations but no start timestamps.",
            "Nested host spans, IPC sends/round trips and worker counter durations overlap and are never added into exclusive time.",
            "Active requests are scheduling eligibility, not proof of row ownership; shared batch time is not assigned to requests.",
            "Only emitted token groups establish request-to-batch links; tokenless batches are not classified as prefill or decode.",
            "Between-batch gaps include host work, output backpressure and ingress waits; they are not pure IPC or idle-GPU time.",
            "Ordered phase attribution follows flush/wait location, not the deferred kernels' logical operation.",
            "Worker delta includes the earlier snapshot command; it is not a per-batch counter timeline.",
            "Input hashes and internal consistency do not establish token correctness, current runtime admission or performance equivalence."])


def read_bounded(path, limit=MAX_BYTES):
    require(path.is_file(), "regular input file required")
    with path.open("rb") as source:
        raw = source.read(limit + 1)
    require(0 < len(raw) <= limit, "input file bound exceeded")
    return raw


def load_profile(manifest):
    require(type(manifest) is dict and set(manifest) == {"schema", "inputs", "expected_identity"}, "manifest field set")
    fields(manifest, dict(schema="FerricLiveRuntimeProfileInputsV1"))
    inputs = manifest["inputs"]
    require(type(inputs) is dict and set(inputs) == {"events", "host_timing", "diagnostics"}, "input roster")
    raw = {}
    for key, binding in inputs.items():
        require(type(binding) is dict and set(binding) == {"path", "sha256"}, "input binding field set")
        require(type(binding["path"]) is str and Path(binding["path"]).is_absolute(), "absolute input path required")
        require(type(binding["sha256"]) is str and re.fullmatch("[0-9a-f]{64}", binding["sha256"]), "input hash required")
        path = Path(binding["path"])
        require(path.resolve(strict=True) == path, "canonical nonsymlink input path required")
        raw[key] = read_bounded(path)
        exact(hashlib.sha256(raw[key]).hexdigest(), binding["sha256"], "input SHA256")
    report = profile(json_lines(raw["events"]), strict_json(raw["host_timing"]),
                     json_lines(raw["diagnostics"], diagnostics=True), manifest["expected_identity"])
    report["input_sha256"] = {key: value["sha256"] for key, value in inputs.items()}
    report["tool_sha256"] = hashlib.sha256(read_bounded(Path(__file__), 128 * 1024)).hexdigest()
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        raw = read_bounded(args.manifest, 65536)
        report = load_profile(strict_json(raw))
        report["manifest_sha256"] = hashlib.sha256(raw).hexdigest()
        with args.output.open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2, allow_nan=False)
            output.write("\n")
    except (ValueError, TypeError, KeyError, IndexError, OSError) as error:
        parser.exit(1, f"Live runtime profile rejected: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
