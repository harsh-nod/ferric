"""Summarize a stock baseline without granting reference or Ferric equivalence."""

import hashlib
import math
import statistics
import struct

import competitive_benchmark as client


DISCLOSURE = ("Stock-vLLM baseline only; independent-reference output mismatch is "
              "preserved. Not Ferric correctness, quality equivalence or speedup evidence.")


def token_hash(tokens):
    if (type(tokens) is not list or len(tokens) != 256 or
            any(type(t) is not int or not 0 <= t < 151936 for t in tokens)):
        raise ValueError("expected exactly 256 vocabulary token IDs")
    return hashlib.sha256(struct.pack("<256I", *tokens)).hexdigest()


def distribution(values):
    if not values:
        return None
    return {"mean": statistics.mean(values),
            **{name: client.percentile(values, fraction)
               for name, fraction in (("p50", .5), ("p90", .9), ("p99", .99))}}


def window_interval(window, previous_end):
    start = client.integer(window.get("started_ns"), 0, 2**63 - 1, "window start")
    end = client.integer(window.get("completed_ns"), 0, 2**63 - 1, "window end")
    if end <= start or (previous_end is not None and start < previous_end):
        raise ValueError("invalid or overlapping closed-loop window")
    for record in window["requests"]:
        request_start = client.integer(record.get("started_ns"), 0, 2**63 - 1, "request start")
        request_end = client.integer(record.get("completed_ns"), 0, 2**63 - 1, "request end")
        if not start <= request_start < request_end <= end:
            raise ValueError("request interval outside enclosing window")
    return start, end


def summarize(run, reference_tokens, stock_tokens):
    """Keep failed/missing samples explicit; successful timing is not quality admission."""
    reference_hash, stock_hash = token_hash(reference_tokens), token_hash(stock_tokens)
    if (run.get("schema") != "FerricCompetitiveTokenStreamingRunV1" or
            run.get("engine") != "vllm" or run.get("concurrency") != 1 or
            run.get("arrival_policy") != "bounded-closed-loop-windows"):
        raise ValueError("expected stock token-streaming C1 closed-loop report")
    requests = run["workload"]["requests"]
    if run["workload"].get("schema") != "FerricCompetitiveTokenWorkloadV1":
        raise ValueError("expected physical-token workload")
    if len(requests) != 1 or len(requests[0]["prompt"]) != 128 or requests[0]["max_tokens"] != 256:
        raise ValueError("expected raw128/256 workload")
    parity, measured, elapsed = [], [], 0
    counts = {}
    previous_end = None
    for phase, planned in (("warmups", 10), ("samples", 30)):
        windows = run[phase]
        if len(windows) > planned:
            raise ValueError("excess samples")
        successes = 0
        for index, window in enumerate(windows):
            if window["index"] != index or len(window["requests"]) != 1:
                raise ValueError("invalid C1 window")
            record = window["requests"][0]
            item = {"phase": phase, "index": index, "success": record["success"],
                    "error": record.get("error"), "reference_equal": False,
                    "retained_stock_equal": False, "token_u32le_sha256": None}
            start, end = window_interval(window, previous_end)
            previous_end = end
            duration = end - start
            if phase == "samples":
                elapsed += duration
            if record["success"]:
                if any(type(record["usage"].get(k)) is not int or record["usage"].get(k) != v
                       for k, v in {"prompt_tokens": 128, "completion_tokens": 256,
                                    "total_tokens": 384}.items()):
                    raise ValueError("invalid successful token accounting")
                digest = token_hash(record["token_ids"])
                item.update(reference_equal=digest == reference_hash,
                            retained_stock_equal=digest == stock_hash,
                            token_u32le_sha256=digest)
                for key in ("ttft_ns", "tpot_ns", "e2e_ns"):
                    value = record[key]
                    if (type(value) not in (int, float) or not math.isfinite(value) or value < 0):
                        raise ValueError("invalid successful timing")
                successes += 1
                if phase == "samples":
                    measured.append(record)
            parity.append(item)
        counts[phase] = {"planned": planned, "recorded": len(windows),
                         "successful": successes, "failed": len(windows) - successes,
                         "missing": planned - len(windows)}
    complete = (run.get("completed") is True and run.get("run_error") is None and
                all(c["successful"] == c["planned"] for c in counts.values()))
    return {"schema": "FerricDisclosedStockC1SummaryV1", "authority": "none",
            "disclosure": DISCLOSURE, "ferric_comparison": False,
            "performance_qualified": False, "tuned": False,
            "independent_reference_equivalence": False,
            "reference_token_sha256": reference_hash, "retained_stock_token_sha256": stock_hash,
            "complete_declared_baseline": complete, "counts": counts,
            "run_error": run.get("run_error"), "parity": parity,
            "ttft_ms": distribution([r["ttft_ns"] / 1e6 for r in measured]),
            "tpot_ms": distribution([r["tpot_ns"] / 1e6 for r in measured]),
            "e2e_ms": distribution([r["e2e_ns"] / 1e6 for r in measured]),
            "successful_output_tokens": len(measured) * 256,
            "recorded_measurement_window_seconds": elapsed / 1e9,
            "closed_loop_output_tokens_per_second": len(measured) * 256 / (elapsed / 1e9)
                if elapsed else None,
            "rate_semantics": "successful tokens / recorded measured windows including failed windows; not steady-state throughput",
            "warmups_excluded": True, "token_itl_available": False,
            "ttft_semantics": run.get("ttft_semantics"),
            "tpot_semantics": run.get("tpot_semantics")}
