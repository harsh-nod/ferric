#!/usr/bin/env python3
"""Bounded, same-client streaming measurements; never a qualification by itself."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import math
from pathlib import Path
import statistics
import time
from urllib.parse import urlsplit
from urllib.request import ProxyHandler, Request, build_opener

SCHEMA = "FerricCompetitiveWorkloadV1"
LINE_LIMIT = 1024 * 1024
RESPONSE_LIMIT = 16 * 1024 * 1024


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
    return json.loads(raw, object_pairs_hook=unique_object, parse_constant=invalid)


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


def sse_events(response, clock=time.monotonic_ns):
    """Read complete SSE frames, bounding both individual lines and the response."""
    data, total = [], 0
    while True:
        line = response.readline(LINE_LIMIT + 1)
        total += len(line)
        require(len(line) <= LINE_LIMIT and total <= RESPONSE_LIMIT, "oversized SSE response")
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


def summarize_stream(events, started_ns, max_tokens):
    chunks, texts, content_times = [], [], []
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
    integer(usage.get("prompt_tokens"), 1, 1000000, "prompt token count")
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


def request_one(url, model, item, timeout):
    body = {"model": model, "prompt": item["prompt"], "max_tokens": item["max_tokens"],
            "temperature": 0, "seed": 0, "ignore_eos": True, "stream": True,
            "stream_options": {"include_usage": True}, "n": 1}
    request = Request(url, data=json.dumps(body).encode(),
                      headers={"Content-Type": "application/json", "Accept": "text/event-stream"})
    started = time.monotonic_ns()
    try:
        with build_opener(ProxyHandler({})).open(request, timeout=timeout) as response:
            require(response.status == 200, "non-success HTTP response")
            require(response.headers.get_content_type() == "text/event-stream",
                    "endpoint did not return an SSE stream")
            record = summarize_stream(sse_events(response), started, item["max_tokens"])
        return {"id": item["id"], "success": True, **record}
    except (OSError, ValueError, TypeError, KeyError) as error:
        return {"id": item["id"], "success": False, "started_ns": started,
                "completed_ns": time.monotonic_ns(), "error": str(error)}


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
        futures = [executor.submit(request_one, url, value["model"], item, timeout)
                   for item in value["requests"]]
        records = [future.result() for future in futures]
        completed = time.monotonic_ns()
    return records, started, completed


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
    args = parser.parse_args()
    integer(args.concurrency, 1, 32, "concurrency")
    integer(args.warmups, 0, 100, "warmups")
    integer(args.samples, 1, 100, "samples")
    for number in (args.timeout, args.ttft_slo_ms, args.tpot_slo_ms):
        require(math.isfinite(number) and number > 0, "timeout/SLO must be finite and positive")
    raw = args.workload.read_bytes()
    require(len(raw) <= 1024 * 1024, "oversized workload")
    value = workload(json_value(raw))
    identity_raw = args.identity.read_bytes()
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
    # Reserve the evidence path before network activity; preserve failed/partial runs.
    with args.output.open("x", encoding="utf-8") as output:
        try:
            for kind, count in (("warmups", args.warmups), ("samples", args.samples)):
                for index in range(count):
                    records, start, end = run_window(args.endpoint, value, args.concurrency, args.timeout)
                    metrics = aggregate(records, start, end, args.ttft_slo_ms, args.tpot_slo_ms)
                    report[kind].append({"index": index, "started_ns": start, "completed_ns": end,
                                         "metrics": metrics, "requests": records})
                    print(json.dumps({"phase": kind, "index": index, "metrics": metrics}), flush=True)
                    require(metrics["all_requests_succeeded"], "request failure; run retained, not accepted")
            report["completed"] = True
        finally:
            json.dump(report, output, indent=2, allow_nan=False)
            output.write("\n")


if __name__ == "__main__":
    main()
