#!/usr/bin/env python3
"""Replay a predeclared ABBA experiment and retain hash-linked change outcomes."""
import argparse
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import re
import statistics


GATES = {
    "blocks": 3, "order": ["A", "B", "B", "A"],
    "warmups_per_cell": 2, "samples_per_cell": 4,
    "minimum_median_tpot_gain_percent": 5.0, "minimum_faster_pairs": 5,
    "maximum_regression_percent": 5.0,
    "percentile_method": "linear-(n-1)*p", "positive_both_orders": True,
}
LIMIT = 32 * 1024 * 1024


def require(condition, message):
    if not condition:
        raise ValueError(message)


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def digest(value):
    return sha(canonical(value))


def no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key: " + key)
        result[key] = value
    return result


def parse(raw):
    def invalid(value):
        raise ValueError("nonfinite JSON value: " + value)
    return json.loads(raw, object_pairs_hook=no_duplicates, parse_constant=invalid)


def load(path):
    with Path(path).open("rb") as source:
        raw = source.read(LIMIT + 1)
    require(len(raw) <= LIMIT, "evidence exceeds byte bound")
    return raw, parse(raw)


def binding(base, value):
    require(type(value) is dict and set(value) == {"path", "sha256"}, "exact evidence binding required")
    require(type(value["path"]) is str and not Path(value["path"]).is_absolute()
            and ".." not in Path(value["path"]).parts, "relative nonescaping evidence path required")
    path = (base / value["path"]).resolve(strict=True)
    require(path.is_relative_to(base.resolve(strict=True)), "evidence symlink escapes attempt root")
    raw, result = load(path)
    require(value["sha256"] == sha(raw), "evidence hash mismatch: " + value["path"])
    return result


def integer(value, low=0):
    require(type(value) is int and value >= low, "bounded integer required")
    return value


def positive(value):
    require(type(value) in (int, float) and math.isfinite(value) and value > 0,
            "positive finite number required")
    return value


def hash_string(value):
    require(type(value) is str and re.fullmatch(r"[0-9a-f]{64}", value), "SHA-256 required")
    return value


def differences(left, right, prefix=""):
    if type(left) is dict and type(right) is dict:
        result = []
        for key in sorted(set(left) | set(right)):
            require(type(key) is str and "/" not in key and "~" not in key, "simple composition key required")
            path = prefix + "/" + key
            result.extend([path] if key not in left or key not in right
                          else differences(left[key], right[key], path))
        return result
    return [] if type(left) is type(right) and left == right else [prefix]


def validate_plan(plan):
    require(plan.get("schema") == "FerricCachedAbiChangePlanV1", "wrong plan schema")
    require(type(plan.get("change_id")) is str and re.fullmatch(r"[a-z0-9][a-z0-9._-]{0,95}", plan["change_id"]),
            "bounded change ID required")
    require(canonical(plan.get("gates")) == canonical(GATES), "predeclared promotion gates differ")
    require(canonical(plan.get("workload")) == canonical({"input_tokens": 128, "output_tokens": 128,
            "context_tokens": 8192, "concurrency": 1, "tensor_parallel": 1,
            "greedy": True, "prefix_caching": False, "speculation": False}), "workload drift")
    for key in ("reference_sha256", "workload_sha256", "client_sha256"):
        hash_string(plan.get(key))
    profiles = plan.get("profiles")
    require(type(profiles) is dict and set(profiles) == {"A", "B"}, "exact A/B profiles required")
    for profile in profiles.values():
        require(type(profile) is dict and set(profile) == {"artifacts", "configuration"}, "exact profile shape required")
        require(type(profile["artifacts"]) is dict and profile["artifacts"], "artifact hashes required")
        for value in profile["artifacts"].values():
            hash_string(value)
        require(type(profile["configuration"]) is dict and profile["configuration"], "explicit composition required")
    delta = differences(profiles["A"], profiles["B"])
    require(delta and plan.get("allowed_profile_differences") == delta,
            "exact, nonempty predeclared profile difference set required")
    mechanism = plan.get("mechanism")
    require(type(mechanism) is dict and set(mechanism) == {"A", "B"}, "per-arm mechanism declaration required")
    for arm, expected in mechanism.items():
        wanted = {"prefill_chunks": 8, "prefill_commands_per_chunk": 613,
                  "prefill_program_dynamic_slots": 0,
                  "decode_program_executions": 127, "decode_commands_per_program": 652,
                  "decode_dynamic_slots": 180, "program_executions": 127,
                  "program_dispatches": 82804,
                  "program_publications": 127,
                  "program_final_waits": 127,
                  "model_dispatches": 87711, "model_batches": 135}
        require(canonical(expected) == canonical(wanted), "exact whole-request prefill/decode mechanism required")
    require(plan.get("timing_semantics") in ("http-text-chunk-v1", "native-ingress-v1"), "unsupported raw timing format")
    return plan


def percentile(values, fraction):
    ordered = sorted(values)
    index = (len(ordered) - 1) * fraction
    low, high = math.floor(index), math.ceil(index)
    return ordered[low] + (ordered[high] - ordered[low]) * (index - low)


def request_metrics(record, reference, proof):
    require(record.get("success") is True, "request failed")
    count = record.get("usage", {}).get("completion_tokens")
    require(type(count) is int and count == 128 and record["usage"].get("prompt_tokens") == 128,
            "128/128 request required")
    started, first, last, ended = [integer(record.get(key), 1) for key in
                                  ("started_ns", "first_text_ns", "last_text_ns", "completed_ns")]
    require(started < first < last <= ended, "request clock order invalid")
    texts, times = [], []
    previous = started
    finished = False
    usage_seen = False
    chunks = record.get("chunks")
    require(type(chunks) is list and 1 <= len(chunks) <= 16384, "bounded raw chunks required")
    for chunk in chunks:
        when = integer(chunk.get("received_ns"), 1)
        require(previous <= when <= ended, "raw chunk clock order invalid")
        previous = when
        event = chunk.get("event")
        require(type(event) is dict and "error" not in event, "stream error")
        if event.get("usage") is not None:
            require(event["usage"] == record["usage"], "raw usage disagrees")
            usage_seen = True
        choices = event.get("choices", [])
        require(type(choices) is list and len(choices) <= 1, "single completion required")
        for choice in choices:
            require(choice.get("index", 0) == 0, "completion index changed")
            text = choice.get("text", "")
            require(type(text) is str, "invalid text chunk")
            if text:
                require(not finished, "content after finish")
                texts.append(text)
                times.append(when)
            if choice.get("finish_reason") is not None:
                require(not finished and choice["finish_reason"] == "length", "unexpected completion reason")
                finished = True
    expected_text = bytes.fromhex(reference["generated_utf8_hex"]).decode("utf-8")
    require(finished and usage_seen and times and first == times[0] and last == times[-1],
            "raw stream boundaries missing or changed")
    require("".join(texts) == record.get("text") == expected_text, "exact streamed output mismatch")
    require(canonical(proof.get("generated_token_ids")) == canonical(reference["generated_token_ids"])
            and proof.get("generated_utf8_hex") == reference["generated_utf8_hex"], "exact token/byte output mismatch")
    for key in ("timeout", "poison", "fallback", "leak"):
        require(proof.get(key) is False, "missing or failed correctness check: " + key)
    require(type(proof.get("expected_frontier")) is int
            and proof["expected_frontier"] == proof.get("completed_frontier"), "completion frontier mismatch")
    require(type(proof.get("expected_dispatches")) is int and proof["expected_dispatches"] > 0
            and proof["expected_dispatches"] == proof.get("completed_dispatches"), "dispatch count mismatch")
    ttft, tpot = first - started, (last - first) / 127
    require(record.get("ttft_ns") == ttft and record.get("tpot_ns") == tpot
            and record.get("e2e_ns") == ended - started, "derived request metrics disagree")
    return {"ttft_ms": ttft / 1e6, "tpot_ms": tpot / 1e6,
            "started_ns": started, "completed_ns": ended, "output_tokens": count}


def validate_lifecycle(value):
    require(value.get("schema") == "FerricMeasuredCellLifecycleV1", "lifecycle successor receipt required")
    require(value.get("exit_status") == 0 and value.get("clean_teardown") is True
            and value.get("forced_signal") is False and value.get("errors") == [], "unclean cell lifecycle")
    samples = value.get("device_samples")
    require(type(samples) is list and len(samples) >= 3, "endpoint and active device evidence required")
    require(samples[0].get("phase") == "preflight" and samples[-1].get("phase") == "postflight",
            "device endpoint sequence required")
    active = 0
    for sample in samples:
        require(sample.get("schema") == "FerricDeviceDescriptorSampleV1"
                and sample.get("accepted") is True and sample.get("complete") is True
                and sample.get("errors") == [] and sample.get("foreign_users") == [], "device sample not admitted")
        require(sample.get("method") == "proc-fd-rdev-all-pids", "path-based fuser is not sufficient")
        if sample.get("phase") == "active":
            require(sample.get("owned_users"), "active sample needs positive device attribution")
            active += 1
        else:
            require(sample.get("phase") in ("preflight", "postflight") and not sample.get("device_users"),
                    "nonempty device endpoint")
    require(active > 0, "no positively attributed active sample")


def cell_metrics(plan, cell, base, reference):
    arm = cell.get("arm")
    require(arm in ("A", "B") and cell.get("profile_sha256") == digest(plan["profiles"][arm]),
            "cell profile binding changed")
    require(cell.get("instrumented") is False, "instrumented latency is not promotable")
    run = binding(base, cell["run"])
    proofs = binding(base, cell["correctness"])
    validate_lifecycle(binding(base, cell["lifecycle"]))
    require(run.get("schema") == "FerricCompetitiveStreamingRunV1" and run.get("completed") is True,
            "completed V1 streaming run required")
    require(run.get("concurrency") == 1 and run.get("arrival_policy") == "bounded-closed-loop-windows",
            "matched C1 closed-loop timing required")
    require(run.get("workload_sha256") == plan["workload_sha256"]
            and run.get("client_sha256") == plan["client_sha256"], "workload or client binding drift")
    require(run.get("ttft_semantics") == "client-send-to-first-nonempty-text-chunk"
            and run.get("tpot_semantics") == "first-to-last-text-chunk-divided-by-usage-tokens-minus-one",
            "timing semantics drift")
    require(run.get("identity", {}).get("profile_sha256") == cell["profile_sha256"],
            "running identity not bound to profile")
    require(proofs.get("schema") == "FerricCellCorrectnessV1" and proofs.get("run_sha256") == cell["run"]["sha256"],
            "correctness receipt not bound to raw run")
    proof_rows = proofs.get("requests")
    require(type(proof_rows) is list and len(proof_rows) == 6, "all six correctness rows required")
    measured, window_ns, proof_index = [], 0, 0
    previous_window_end = 0
    for phase, count in (("warmups", 2), ("samples", 4)):
        windows = run.get(phase)
        require(type(windows) is list and len(windows) == count, "exact predeclared request counts required")
        for index, window in enumerate(windows):
            require(window.get("index") == index and type(window.get("requests")) is list
                    and len(window["requests"]) == 1, "one request per ordered window required")
            started, completed = integer(window.get("started_ns"), 1), integer(window.get("completed_ns"), 1)
            require(previous_window_end <= started < completed, "window clocks overlap or regress")
            previous_window_end = completed
            record, proof = window["requests"][0], proof_rows[proof_index]
            proof_index += 1
            require(proof.get("phase") == phase and proof.get("index") == index
                    and proof.get("request_id") == record.get("id"), "correctness row order or ID mismatch")
            row = request_metrics(record, reference, proof)
            require(started <= row["started_ns"] < row["completed_ns"] <= completed, "request outside window")
            if phase == "samples":
                measured.append(row)
                window_ns += completed - started
    return {"arm": arm, "cell_id": cell["cell_id"], "requests": measured,
            "started_ns": run["warmups"][0]["started_ns"], "completed_ns": previous_window_end,
            "window_ns": window_ns, "median_tpot_ms": statistics.median(r["tpot_ms"] for r in measured)}


def summary(rows):
    requests = [request for row in rows for request in row["requests"]]
    value = {"requests": len(requests), "finite_output_tokens_per_second":
             sum(r["output_tokens"] for r in requests) / (sum(r["window_ns"] for r in rows) / 1e9)}
    for key in ("ttft_ms", "tpot_ms"):
        values = [r[key] for r in requests]
        value[key] = {"median": statistics.median(values), "p95": percentile(values, .95),
                      "mean": statistics.mean(values)}
    return value


def evaluate(plan, attempt, base):
    validate_plan(plan)
    require(plan["timing_semantics"] == "http-text-chunk-v1", "HTTP evaluator cannot relabel native timings")
    require(attempt.get("schema") == "FerricChangeAttemptV1" and attempt.get("plan_sha256") == digest(plan),
            "attempt not bound to canonical plan")
    reference = binding(base, attempt["reference"])
    require(attempt["reference"]["sha256"] == plan["reference_sha256"], "reference binding changed")
    tokens = reference.get("generated_token_ids")
    require(type(tokens) is list and len(tokens) == 128 and all(type(v) is int and v >= 0 for v in tokens),
            "128 exact reference token IDs required")
    require(type(reference.get("generated_utf8_hex")) is str, "reference output bytes required")
    counters = binding(base, attempt["mechanism"])
    require(counters.get("schema") == "FerricCachedAbiRequestMechanismV1"
            and counters.get("plan_sha256") == digest(plan), "mechanism not bound to plan")
    for arm in ("A", "B"):
        actual = counters.get("arms", {}).get(arm, {})
        require(actual.get("profile_sha256") == digest(plan["profiles"][arm])
                and actual.get("decode_tokens") == 127 and actual.get("correctness_passed") is True,
                "counter arm not qualified")
        expected = plan["mechanism"][arm]
        require(canonical(actual.get("request_mechanism")) == canonical(expected), "whole-request mechanism invariant failed")
    cells = attempt.get("cells")
    require(type(cells) is list and len(cells) == 12, "all twelve ABBA cells required; never drop failed cells")
    require(len({c.get("cell_id") for c in cells}) == 12, "duplicate cell ID")
    seen_paths = set()
    seen_runs = set()
    results = []
    for index, cell in enumerate(cells):
        require(cell.get("block") == index // 4 and cell.get("position") == index % 4
                and cell.get("arm") == GATES["order"][index % 4], "predeclared ABBA order changed")
        path = cell.get("run", {}).get("path")
        require(path not in seen_paths, "a raw run cannot be reused as multiple cells")
        seen_paths.add(path)
        raw_hash = cell["run"]["sha256"]
        require(raw_hash not in seen_runs, "raw run bytes cannot be reused as multiple cells")
        seen_runs.add(raw_hash)
        result = cell_metrics(plan, cell, base, reference)
        require(not results or result["started_ns"] >= results[-1]["completed_ns"], "actual cell times overlap or reorder")
        results.append(result)
    return compare_results(plan, digest(attempt), results)


def compare_results(plan, attempt_sha256, results):
    """Shared arithmetic after the format-specific correctness/raw replay checks."""
    validate_plan(plan)
    hash_string(attempt_sha256)
    require(len(results) == 12 and [r["arm"] for r in results] == GATES["order"] * 3,
            "twelve ordered ABBA result cells required")
    require(len({r["cell_id"] for r in results}) == 12, "distinct ABBA result cells required")
    for row in results:
        require(len(row["requests"]) == 4 and row["window_ns"] > 0, "four measured requests per result cell required")
        for request in row["requests"]:
            require(request["output_tokens"] == 128, "fixed-length output required")
            positive(request["ttft_ms"])
            positive(request["tpot_ms"])
        require(row["median_tpot_ms"] == statistics.median(r["tpot_ms"] for r in row["requests"]),
                "derived cell median changed")
    arms = {arm: summary([r for r in results if r["arm"] == arm]) for arm in ("A", "B")}
    pairs = []
    for index in range(0, 12, 2):
        first, second = results[index:index + 2]
        a, b = (first, second) if first["arm"] == "A" else (second, first)
        gain = (1 - b["median_tpot_ms"] / a["median_tpot_ms"]) * 100
        pairs.append({"order": first["arm"] + second["arm"], "cells": [first["cell_id"], second["cell_id"]],
                      "tpot_gain_percent": gain})
    a, b = arms["A"], arms["B"]
    gain = (1 - b["tpot_ms"]["median"] / a["tpot_ms"]["median"]) * 100
    regressions = {key + "_" + p: (b[key][p] / a[key][p] - 1) * 100
                   for key, p in (("ttft_ms", "median"), ("ttft_ms", "p95"), ("tpot_ms", "p95"))}
    regressions["finite_output_rate"] = (1 - b["finite_output_tokens_per_second"] / a["finite_output_tokens_per_second"]) * 100
    checks = {"median_tpot_gain": gain >= 5 - 1e-10,
              "faster_adjacent_pairs": sum(p["tpot_gain_percent"] > 0 for p in pairs) >= 5,
              "positive_both_orders": all(statistics.median(p["tpot_gain_percent"] for p in pairs if p["order"] == order) > 0
                                           for order in ("AB", "BA")),
              "regression_limits": all(v <= 5 + 1e-10 for v in regressions.values())}
    return {"schema": "FerricCachedAbiChangeEvaluationV1", "change_id": plan["change_id"],
            "plan_sha256": digest(plan), "attempt_sha256": attempt_sha256,
            "status": "promotable" if all(checks.values()) else "inconclusive",
            "qualification": "engineering-gates-only-not-statistical-proof",
            "gates": GATES, "checks": checks, "arms": arms, "pairs": pairs,
            "median_tpot_gain_percent": gain, "regressions_percent": regressions,
            "median_ttft_gain_percent": -regressions["ttft_ms_median"],
            "p95_ttft_gain_percent": -regressions["ttft_ms_p95"],
            "finite_output_rate_gain_percent": -regressions["finite_output_rate"],
            "latency_semantics": ("HTTP chunk observations, not calibrated token ITL" if plan["timing_semantics"] == "http-text-chunk-v1"
                                  else "native ingress event observations, not HTTP or calibrated GPU time"),
            "rate_semantics": "sum of measured finite windows; not steady loaded throughput"}


def append_ledger(path, entry):
    require(entry.get("schema") == "FerricCachedAbiChangeEvaluationV1"
            and entry.get("status") in ("pending", "failed", "inconclusive", "promotable"), "ledger entry schema/status invalid")
    require(type(entry.get("change_id")) is str and entry["change_id"], "ledger change ID required")
    hash_string(entry.get("plan_sha256"))
    payload = canonical(entry)
    require(len(payload) <= 1024 * 1024, "bounded ledger entry required")
    flags = os.O_RDWR | os.O_CREAT | os.O_CLOEXEC | os.O_NOFOLLOW
    with os.fdopen(os.open(path, flags, 0o600), "r+b") as target:
        fcntl.flock(target, fcntl.LOCK_EX)
        raw = target.read(LIMIT + 1)
        require(len(raw) <= LIMIT and (not raw or raw.endswith(b"\n")), "oversized or interrupted ledger")
        previous = "0" * 64
        count = 0
        for line in raw.splitlines():
            row = parse(line)
            require(set(row) == {"sequence", "previous_sha256", "entry", "sha256"}
                    and row["sequence"] == count and row["previous_sha256"] == previous,
                    "ledger chain order invalid")
            claimed = row.pop("sha256")
            require(claimed == digest(row), "ledger hash mismatch")
            previous, count = claimed, count + 1
        row = {"sequence": count, "previous_sha256": previous, "entry": entry}
        row["sha256"] = digest(row)
        target.write(canonical(row) + b"\n")
        target.flush()
        os.fsync(target.fileno())
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("evaluate")
    check.add_argument("--plan", type=Path, required=True)
    check.add_argument("--attempt", type=Path, required=True)
    check.add_argument("--output", type=Path, required=True)
    append = commands.add_parser("append")
    append.add_argument("--ledger", type=Path, required=True)
    append.add_argument("--entry", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "append":
        _, entry = load(args.entry)
        print(json.dumps(append_ledger(args.ledger, entry), sort_keys=True, allow_nan=False))
        return 0
    _, plan = load(args.plan)
    _, attempt = load(args.attempt)
    try:
        result = evaluate(plan, attempt, args.attempt.parent)
    except (ValueError, KeyError, TypeError, OSError) as error:
        result = {"schema": "FerricCachedAbiChangeEvaluationV1", "change_id": plan.get("change_id"),
                  "plan_sha256": digest(plan), "attempt_sha256": digest(attempt),
                  "status": "failed", "error": type(error).__name__ + ": " + str(error),
                  "gates": GATES, "latency_admitted": False}
    with args.output.open("x", encoding="utf-8") as target:
        json.dump(result, target, indent=2, sort_keys=True, allow_nan=False)
        target.write("\n")
    print(json.dumps(result, sort_keys=True, allow_nan=False))
    return 0 if result["status"] in ("promotable", "inconclusive") else 1


if __name__ == "__main__":
    raise SystemExit(main())
