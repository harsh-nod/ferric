#!/usr/bin/env python3
"""Bounded, same-client streaming measurements; never a qualification by itself."""

import argparse
from collections import deque
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
import hashlib
from http.client import HTTPException
import json
import math
from pathlib import Path
import random
import statistics
import threading
import time
from urllib.parse import urlsplit
from urllib.request import ProxyHandler, Request, build_opener

SCHEMA = "FerricCompetitiveWorkloadV1"
LINE_LIMIT = 1024 * 1024
RESPONSE_LIMIT = 16 * 1024 * 1024
RUN_RESPONSE_LIMIT = 64 * 1024 * 1024
FAILURE_KINDS = ("client_overload", "queue_timeout", "deadline", "request_error", "client_budget")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def integer(value, low, high, label):
    require(type(value) is int and low <= value <= high, f"invalid {label}")
    return value


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def json_value(raw):
    def invalid(value):
        raise ValueError(f"nonfinite JSON value: {value}")
    def number(value):
        result = float(value)
        require(math.isfinite(result), "nonfinite JSON number")
        return result
    try:
        return json.loads(raw, object_pairs_hook=unique_object, parse_constant=invalid, parse_float=number)
    except RecursionError as error:
        raise ValueError("JSON nesting exceeds parser bound") from error


def workload(value):
    require(type(value) is dict and set(value) == {"schema", "model", "requests"},
            "workload fields drifted")
    require(value["schema"] == SCHEMA and value["model"] == "Qwen/Qwen3-8B",
            "unsupported workload identity")
    requests = value["requests"]
    require(type(requests) is list and 1 <= len(requests) <= 256,
            "workload requires 1..256 requests")
    seen = set()
    for request in requests:
        require(type(request) is dict and set(request) == {"id", "prompt", "max_tokens"},
                "request fields drifted")
        name = request["id"]
        require(type(name) is str and 0 < len(name) <= 128 and name not in seen,
                "invalid or duplicate request ID")
        seen.add(name)
        prompt = request["prompt"]
        require(type(prompt) is str and 0 < len(prompt.encode("utf-8")) <= 128 * 1024,
                "invalid prompt")
        integer(request["max_tokens"], 1, 2048, "output limit")
    return value


def endpoint(value):
    parsed = urlsplit(value)
    require(parsed.scheme == "http" and parsed.hostname in ("127.0.0.1", "localhost", "::1")
            and parsed.username is None and parsed.password is None
            and parsed.path == "/v1/completions" and not parsed.query and not parsed.fragment,
            "benchmark endpoint must be a loopback HTTP /v1/completions URL")
    require(parsed.port is not None and 1 <= parsed.port <= 65535, "explicit port required")
    return value


class ResponseBudget:
    def __init__(self, limit=RUN_RESPONSE_LIMIT):
        self.limit = limit
        self.used = 0
        self.exhausted = False
        self.lock = threading.Lock()

    def consume(self, count):
        with self.lock:
            if self.exhausted or self.used + count > self.limit:
                self.exhausted = True
                raise ValueError("run response-byte budget exhausted")
            self.used += count


def sse_events(response, clock=time.monotonic_ns, deadline_ns=None, budget=None):
    """Read complete SSE frames, bounding both individual lines and the response."""
    data, total = [], 0
    while True:
        if deadline_ns is not None:
            require(clock() <= deadline_ns, "stream deadline exceeded")
        line = response.readline(LINE_LIMIT + 1)
        if deadline_ns is not None:
            require(clock() <= deadline_ns, "stream deadline exceeded")
        total += len(line)
        require(len(line) <= LINE_LIMIT and total <= RESPONSE_LIMIT, "oversized SSE response")
        if budget is not None:
            budget.consume(len(line))
        if not line:
            require(not data, "truncated SSE frame")
            return
        line = line.rstrip(b"\r\n")
        if not line:
            if data:
                yield b"\n".join(data), clock()
                data = []
        elif line.startswith(b"data:"):
            data.append(line[5:].removeprefix(b" "))


def summarize_stream(events, started_ns, max_tokens, chunks=None):
    chunks = [] if chunks is None else chunks
    texts, content_times = [], []
    usage = None
    finished = None
    done_ns = None
    for data, timestamp in events:
        require(type(timestamp) is int and timestamp >= started_ns
                and (not chunks or timestamp >= chunks[-1]["received_ns"]),
                "nonmonotonic stream clock")
        if data == b"[DONE]":
            done_ns = timestamp
            break
        value = json_value(data)
        require(type(value) is dict and "error" not in value, "server stream error")
        require(len(chunks) < 16384, "SSE chunk count exceeded bound")
        chunks.append({"received_ns": timestamp, "event": value})
        if value.get("usage") is not None:
            require(usage is None or usage == value["usage"], "conflicting final usage")
            usage = value["usage"]
        choices = value.get("choices", [])
        require(type(choices) is list and len(choices) <= 1, "expected one completion")
        for choice in choices:
            require(type(choice) is dict and choice.get("index", 0) == 0,
                    "completion index drifted")
            text = choice.get("text", "")
            require(type(text) is str, "invalid completion text")
            if text:
                require(finished is None, "content after completion finished")
                texts.append(text)
                content_times.append(timestamp)
            reason = choice.get("finish_reason")
            if reason is not None:
                require(reason == "length" and finished is None, "unexpected finish reason")
                finished = reason
    require(done_ns is not None and finished == "length", "incomplete completion stream")
    require(type(usage) is dict, "missing server token usage")
    count = integer(usage.get("completion_tokens"), 1, max_tokens, "completion token count")
    require(count == max_tokens, "output limit was not honored with ignore_eos")
    prompt_count = integer(usage.get("prompt_tokens"), 1, 1000000, "prompt token count")
    if "total_tokens" in usage:
        require(integer(usage["total_tokens"], 1, 1002048, "total token count") == prompt_count + count,
                "total usage does not match prompt and completion tokens")
    require(content_times, "completion has no observable text")
    return {
        "started_ns": started_ns, "first_text_ns": content_times[0],
        "last_text_ns": content_times[-1], "completed_ns": done_ns,
        "ttft_ns": content_times[0] - started_ns,
        "tpot_ns": ((content_times[-1] - content_times[0]) / (count - 1)
                    if count > 1 else None),
        "e2e_ns": done_ns - started_ns, "usage": usage,
        "text": "".join(texts), "chunks": chunks,
        "chunk_intervals_ns": [b - a for a, b in zip(content_times, content_times[1:])],
        "token_itl_ns": None,
    }


def request_one(url, model, item, timeout, deadline_ns=None, budget=None):
    body = {"model": model, "prompt": item["prompt"], "max_tokens": item["max_tokens"],
            "temperature": 0, "seed": 0, "ignore_eos": True, "stream": True,
            "stream_options": {"include_usage": True}, "n": 1}
    request = Request(url, data=json.dumps(body).encode(),
                      headers={"Content-Type": "application/json", "Accept": "text/event-stream"})
    started = time.monotonic_ns()
    chunks = []
    try:
        deadline_ns = started + int(timeout * 1e9) if deadline_ns is None else deadline_ns
        remaining = (deadline_ns - started) / 1e9
        require(remaining > 0, "request deadline exceeded before send")
        # This timeout is per socket read, not a watchdog for an in-progress
        # readline. V2 reports explicitly retain soft-deadline semantics.
        with build_opener(ProxyHandler({})).open(request, timeout=min(timeout, remaining)) as response:
            require(response.status == 200, "non-success HTTP response")
            require(response.headers.get_content_type() == "text/event-stream",
                    "endpoint did not return an SSE stream")
            events = sse_events(response, deadline_ns=deadline_ns, budget=budget)
            record = summarize_stream(events, started, item["max_tokens"], chunks)
        return {"id": item["id"], "success": True, **record}
    except (OSError, ValueError, TypeError, KeyError, HTTPException) as error:
        return {"id": item["id"], "success": False, "started_ns": started,
                "completed_ns": time.monotonic_ns(), "error": str(error), "chunks": chunks}


def percentile(values, fraction):
    if not values:
        return None
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    low, high = math.floor(position), math.ceil(position)
    return ordered[low] + (ordered[high] - ordered[low]) * (position - low)


def aggregate(records, started_ns, completed_ns, ttft_slo_ms, tpot_slo_ms):
    require(completed_ns > started_ns, "nonpositive benchmark window")
    successes = [item for item in records if item["success"]]
    elapsed = (completed_ns - started_ns) / 1e9
    good = [item for item in successes if item["ttft_ns"] <= ttft_slo_ms * 1e6
            and (item["tpot_ns"] is None or item["tpot_ns"] <= tpot_slo_ms * 1e6)]
    metrics = {"requests": len(records), "successful_requests": len(successes),
               "failed_requests": len(records) - len(successes), "window_seconds": elapsed,
               "output_tokens_per_second": sum(item["usage"]["completion_tokens"]
                                               for item in successes) / elapsed,
               "request_goodput_per_second": len(good) / elapsed,
               "output_goodput_per_second": sum(item["usage"]["completion_tokens"]
                                                for item in good) / elapsed,
               "all_requests_succeeded": len(successes) == len(records)}
    for name in ("ttft_ns", "tpot_ns", "e2e_ns"):
        values = [item[name] / 1e6 for item in successes if item[name] is not None]
        metrics[name.removesuffix("_ns") + "_ms"] = {
            "p50": percentile(values, .5), "p90": percentile(values, .9),
            "p99": percentile(values, .99),
            "mean": statistics.mean(values) if values else None}
    return metrics


def run_window(url, value, concurrency, timeout):
    # Executor queuing is included in window throughput, not per-request TTFT.
    # This is a bounded closed-loop window, not an open-loop arrival/SLO test.
    with ThreadPoolExecutor(max_workers=concurrency) as executor:
        started = time.monotonic_ns()
        pending = iter(enumerate(value["requests"]))
        active, records = {}, [None] * len(value["requests"])
        while True:
            while len(active) < concurrency:
                entry = next(pending, None)
                if entry is None:
                    break
                index, item = entry
                active[executor.submit(request_one, url, value["model"], item, timeout)] = index
            if not active:
                break
            done, _ = wait(active, return_when=FIRST_COMPLETED)
            for future in done:
                records[active.pop(future)] = future.result()
        completed = time.monotonic_ns()
    return records, started, completed


def arrival_offsets(count, policy, rate, seed):
    integer(count, 1, 256, "arrival count")
    require(policy in ("constant", "poisson"), "unsupported open-loop arrival policy")
    require(type(rate) in (int, float) and math.isfinite(rate) and 0 < rate <= 1e6,
            "arrival rate must be finite and in (0, 1000000]")
    integer(seed, 0, 2**64 - 1, "arrival seed")
    rng = random.Random(seed)
    elapsed = 0.0
    offsets = []
    for index in range(count):
        elapsed = index / rate if policy == "constant" else elapsed + rng.expovariate(rate)
        require(elapsed <= 3600, "arrival schedule exceeds one hour")
        offsets.append(round(elapsed * 1e9))
    return offsets


def arrival_failure(item, intended, observed, reason):
    return {"id": item["id"], "success": False, "started_ns": None,
            "intended_arrival_ns": intended, "send_started_ns": None,
            "send_delay_ns": None, "completed_ns": observed,
            "failure_kind": reason, "error": reason, "chunks": []}


def arrival_request(url, model, item, timeout, intended, budget=None):
    deadline = intended + int(timeout * 1e9)
    now = time.monotonic_ns()
    if budget is not None and budget.exhausted:
        return arrival_failure(item, intended, now, "client_budget")
    if now >= deadline:
        return arrival_failure(item, intended, now, "queue_timeout")
    record = request_one(url, model, item, timeout, deadline_ns=deadline, budget=budget)
    record.update(intended_arrival_ns=intended, send_started_ns=record["started_ns"],
                  send_delay_ns=record["started_ns"] - intended)
    if record["success"]:
        record.update(wire_ttft_ns=record["ttft_ns"], wire_e2e_ns=record["e2e_ns"],
                      ttft_ns=record["first_text_ns"] - intended,
                      e2e_ns=record["completed_ns"] - intended)
    else:
        record["failure_kind"] = ("client_budget" if record.get("error") == "run response-byte budget exhausted"
                                  else "deadline" if record["completed_ns"] >= deadline else "request_error")
    return record


def run_open_loop(url, value, concurrency, pending_capacity, timeout, offsets, budget=None):
    integer(concurrency, 1, 32, "concurrency")
    integer(pending_capacity, 0, 256, "pending capacity")
    require(len(offsets) == len(value["requests"]) and offsets == sorted(offsets)
            and all(type(offset) is int and 0 <= offset <= 3600 * 10**9 for offset in offsets),
            "invalid intended arrival offsets")
    records = [None] * len(offsets)
    budget = ResponseBudget() if budget is None else budget
    pending, active = deque(), {}
    next_arrival = 0
    started = time.monotonic_ns()
    with ThreadPoolExecutor(max_workers=concurrency) as executor:
        while next_arrival < len(offsets) or pending or active:
            now = time.monotonic_ns()
            for future in [future for future in active if future.done()]:
                index = active.pop(future)
                records[index] = future.result()
            while pending and now >= started + offsets[pending[0]] + int(timeout * 1e9):
                index = pending.popleft()
                records[index] = arrival_failure(value["requests"][index], started + offsets[index],
                                                 now, "queue_timeout")

            def dispatch(index):
                active[executor.submit(arrival_request, url, value["model"],
                                       value["requests"][index], timeout,
                                       started + offsets[index], budget)] = index

            while pending and len(active) < concurrency:
                dispatch(pending.popleft())
            # Never move an intended arrival to a later free-capacity instant.
            while next_arrival < len(offsets) and started + offsets[next_arrival] <= now:
                index = next_arrival
                next_arrival += 1
                if now >= started + offsets[index] + int(timeout * 1e9):
                    records[index] = arrival_failure(value["requests"][index], started + offsets[index],
                                                     now, "queue_timeout")
                elif len(active) < concurrency:
                    dispatch(index)
                elif len(pending) < pending_capacity:
                    pending.append(index)
                else:
                    records[index] = arrival_failure(value["requests"][index], started + offsets[index],
                                                     now, "client_overload")
            deadlines = [now + 10**9]
            if next_arrival < len(offsets):
                deadlines.append(started + offsets[next_arrival])
            if pending:
                deadlines.append(started + offsets[pending[0]] + int(timeout * 1e9))
            delay = max(0, (min(deadlines) - time.monotonic_ns()) / 1e9)
            if active:
                wait(active, timeout=delay, return_when=FIRST_COMPLETED)
            elif next_arrival < len(offsets):
                time.sleep(delay)
    return records, started, time.monotonic_ns()


def read_input(path):
    with path.open("rb") as source:
        raw = source.read(1024 * 1024 + 1)
    require(0 < len(raw) <= 1024 * 1024, "empty or oversized input")
    return raw, json_value(raw)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--endpoint", required=True, type=endpoint)
    parser.add_argument("--workload", required=True, type=Path)
    parser.add_argument("--identity", required=True, type=Path,
                        help="externally collected engine/model/hardware/configuration identities")
    parser.add_argument("--engine", required=True, choices=("ferric", "vllm", "sglang"))
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--concurrency", type=int, default=1)
    parser.add_argument("--warmups", type=int, default=10)
    parser.add_argument("--samples", type=int, default=30)
    parser.add_argument("--timeout", type=float, default=600)
    parser.add_argument("--ttft-slo-ms", type=float, required=True)
    parser.add_argument("--tpot-slo-ms", type=float, required=True)
    parser.add_argument("--arrival", choices=("closed-loop", "constant", "poisson"), default="closed-loop")
    parser.add_argument("--arrival-rate", type=float)
    parser.add_argument("--arrival-seed", type=int, default=0)
    parser.add_argument("--pending-capacity", type=int, default=0)
    parser.add_argument("--start-evidence", type=Path)
    parser.add_argument("--tuning-policy", type=Path)
    args = parser.parse_args()
    integer(args.concurrency, 1, 32, "concurrency")
    integer(args.warmups, 0, 100, "warmups")
    integer(args.samples, 1, 100, "samples")
    for number in (args.timeout, args.ttft_slo_ms, args.tpot_slo_ms):
        require(math.isfinite(number) and number > 0, "timeout/SLO must be finite and positive")
    require(args.timeout <= 3600, "timeout exceeds one hour")
    integer(args.pending_capacity, 0, 256, "pending capacity")
    integer(args.arrival_seed, 0, 2**64 - 1, "arrival seed")
    open_loop = args.arrival != "closed-loop"
    require(open_loop or (args.arrival_rate is None and args.arrival_seed == 0
                         and args.pending_capacity == 0 and args.start_evidence is None
                         and args.tuning_policy is None), "open-loop options require open-loop arrival")
    with args.workload.open("rb") as source:
        raw = source.read(1024 * 1024 + 1)
    require(len(raw) <= 1024 * 1024, "oversized workload")
    value = workload(json_value(raw))
    with args.identity.open("rb") as source:
        identity_raw = source.read(1024 * 1024 + 1)
    require(len(identity_raw) <= 1024 * 1024, "oversized identity")
    identity = json_value(identity_raw)
    require(type(identity) is dict and identity.get("engine") == args.engine,
            "engine identity mismatch")
    report = {"schema": "FerricCompetitiveStreamingRunV1", "authority": "none",
              "qualification": False, "engine": args.engine, "identity": identity,
              "identity_sha256": hashlib.sha256(identity_raw).hexdigest(),
              "workload_sha256": hashlib.sha256(raw).hexdigest(), "workload": value,
              "client_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "endpoint": args.endpoint, "concurrency": args.concurrency,
              "arrival_policy": "bounded-closed-loop-windows",
              "ttft_semantics": "client-send-to-first-nonempty-text-chunk",
              "tpot_semantics": "first-to-last-text-chunk-divided-by-usage-tokens-minus-one",
              "token_itl_available": False,
              "ttft_slo_ms": args.ttft_slo_ms, "tpot_slo_ms": args.tpot_slo_ms,
              "warmups": [], "samples": []}
    offsets = None
    response_budget = ResponseBudget() if open_loop else None
    if open_loop:
        offsets = arrival_offsets(len(value["requests"]), args.arrival, args.arrival_rate,
                                  args.arrival_seed)
        report.update(schema="FerricCompetitiveStreamingRunV2", arrival_policy=args.arrival,
                      arrival_rate=args.arrival_rate, arrival_seed=args.arrival_seed,
                      arrival_offsets_ns=offsets, pending_capacity=args.pending_capacity,
                      timeout_seconds=args.timeout, completed=False,
                      ttft_semantics="intended-arrival-to-first-nonempty-text-chunk",
                      e2e_semantics="intended-arrival-to-DONE",
                      deadline_semantics="soft-absolute-checks-with-per-read-socket-timeout",
                      window_semantics="finite-arrival-cohort-including-drain-not-steady-state",
                      failure_policy="retain-all-planned-windows-including-overload",
                      start_evidence=None, start_evidence_sha256=None,
                      tuning_policy=None, tuning_policy_sha256=None)
        report["response_byte_budget"] = RUN_RESPONSE_LIMIT
        for field, path in (("start_evidence", args.start_evidence), ("tuning_policy", args.tuning_policy)):
            if path is not None:
                blob, data = read_input(path)
                require(type(data) is dict, f"invalid {field}")
                report[field] = data
                report[field + "_sha256"] = hashlib.sha256(blob).hexdigest()
    # Reserve the evidence path before network activity; preserve failed/partial runs.
    with args.output.open("x", encoding="utf-8") as output:
        if open_loop:
            report["started_unix_ns"] = time.time_ns()
        try:
            for kind, count in (("warmups", args.warmups), ("samples", args.samples)):
                for index in range(count):
                    if open_loop:
                        records, start, end = run_open_loop(args.endpoint, value, args.concurrency,
                            args.pending_capacity, args.timeout, offsets, response_budget)
                    else:
                        records, start, end = run_window(args.endpoint, value, args.concurrency, args.timeout)
                    metrics = aggregate(records, start, end, args.ttft_slo_ms, args.tpot_slo_ms)
                    if open_loop:
                        metrics["failure_counts"] = {kind: sum(record.get("failure_kind") == kind
                                                              for record in records)
                            for kind in FAILURE_KINDS}
                    report[kind].append({"index": index, "started_ns": start, "completed_ns": end,
                                         "metrics": metrics, "requests": records})
                    print(json.dumps({"phase": kind, "index": index, "metrics": metrics}), flush=True)
                    if not open_loop:
                        require(metrics["all_requests_succeeded"], "request failure; run retained, not accepted")
            report["completed"] = True
        except BaseException as error:
            if open_loop:
                report["run_error"] = f"{type(error).__name__}: {error}"
            raise
        finally:
            if open_loop:
                report["completed_unix_ns"] = time.time_ns()
                report["response_bytes_observed"] = response_budget.used
                report["response_budget_exhausted"] = response_budget.exhausted
            json.dump(report, output, indent=2, allow_nan=False)
            output.write("\n")


if __name__ == "__main__":
    main()
