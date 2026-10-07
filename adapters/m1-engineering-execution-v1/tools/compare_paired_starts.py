#!/usr/bin/env python3
"""Aggregate retained matched HTTP pairs; descriptive screening, not qualification.

The manifest has schema FerricPairedStartsManifestV1, a reference file path, and
an ordered pairs list. Each pair contains summary, ferric, and vllm paths; the
latter two locate relocated retained engine directories. Paths are relative to
the manifest unless absolute. List EVERY attempted pair, including failed or
missing outputs. There is deliberately no exclusion or best-run option.

Frozen per-pair replay remains responsible for full launch/custody validation.
This reader verifies its hash-bound receipts, output and metric consistency; it
does not independently authenticate binaries, hardware isolation or omissions
from a caller-supplied roster. No number of these finite cohorts qualifies a
held-out serving-suite win under docs/PERFORMANCE.md.
"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import statistics
import sys

import competitive_benchmark as bench


MANIFEST_SCHEMA = "FerricPairedStartsManifestV1"
SCHEMA = "FerricPairedStartsScreeningV1"
PAIR_SCHEMAS = {
    "FerricV17MatchedPairSummaryV1": "FerricV17Matched128ReceiptV1",
    "FerricCandidateMatchedPairSummaryV2": "FerricCandidateMatched128ReceiptV2",
    "FerricCandidateMatchedPairSummaryV3": "FerricCandidateMatched128ReceiptV3",
}
SETTINGS = {
    "arrival": "closed-loop", "completion_tokens": 128, "concurrency": 1,
    "context": 8192, "dtype": "bfloat16", "head_dtype": "float32",
    "ignore_eos": True, "prefix_cache": False, "prompt_tokens": 128,
    "request_timeout_seconds": 120, "samples": 30, "seed": 0,
    "speculation": False, "temperature": 0, "tpot_slo_ms": 1000,
    "ttft_slo_ms": 10000, "warmups": 10,
}
TTFT = "client-send-to-first-nonempty-text-chunk"
TPOT = "first-to-last-text-chunk-divided-by-usage-tokens-minus-one"
THROUGHPUT = "30*128 / ((last measured window completed_ns - first measured window started_ns)/1e9)"
PROFILE_FIELDS = ("schema", "plan_sha256", "settings", "ferric_build", "vllm_image",
                  "reference_sha256", "client_sha256")
METRICS = ("mean_ttft_ms", "mean_tpot_ms", "output_tokens_per_second")
require = bench.require


def equal(actual, expected, label):
    # JSON equality must not turn true into 1 or false into 0.
    require(json.dumps(actual, sort_keys=True, allow_nan=False) ==
            json.dumps(expected, sort_keys=True, allow_nan=False), label)


def sha256(value):
    require(type(value) is str and len(value) == 64 and
            all(char in "0123456789abcdef" for char in value), "invalid SHA256")
    return value


def read_json(path, maximum=4 * 1024**2):
    path = Path(path).resolve(strict=True)
    require(path.is_file(), "retained regular file required")
    with path.open("rb") as stream:
        before = os.fstat(stream.fileno())
        require(0 < before.st_size <= maximum, "retained file size bound exceeded")
        raw = stream.read(maximum + 1)
        after = os.fstat(stream.fileno())
    require(len(raw) == before.st_size and
            (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) ==
            (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns),
            "retained file changed during read")
    return bench.json_value(raw), hashlib.sha256(raw).hexdigest()


def path_at(root, value):
    require(type(value) is str and 0 < len(value) <= 4096, "invalid retained path")
    return (root / value).resolve(strict=True)


def positive(value, label):
    require(type(value) in (int, float) and math.isfinite(value) and value > 0,
            "invalid " + label)
    return value


def clock(value):
    require(type(value) is int and 0 <= value <= 2**63 - 1, "invalid clock")
    return value


def close(actual, expected, label):
    positive(actual, label)
    require(math.isclose(actual, expected, rel_tol=1e-12, abs_tol=1e-9), label)


def validate_record(record, reference, window):
    require(record.get("success") is True, "failed request in cohort")
    stamps = [clock(record[key]) for key in
              ("started_ns", "first_text_ns", "last_text_ns", "completed_ns")]
    require(stamps == sorted(stamps) and window[0] <= stamps[0] and
            stamps[-1] <= window[1], "request clock outside ordered window")
    close(record["ttft_ns"], stamps[1] - stamps[0], "TTFT definition differs")
    close(record["tpot_ns"], (stamps[2] - stamps[1]) / 127, "TPOT definition differs")
    close(record["e2e_ns"], stamps[3] - stamps[0], "E2E definition differs")
    equal(record["usage"], {"prompt_tokens": 128, "completion_tokens": 128,
                            "total_tokens": 256}, "fixed-length token usage differs")
    require(record["text"].encode("utf-8").hex() == reference["generated_utf8_hex"],
            "timed output correctness mismatch")
    require(record.get("token_itl_ns") is None, "SSE chunks are not token ITL")


def validate_final(final, reference):
    require(final.get("event") == "request" and final.get("state") == "Completed"
            and final.get("admitted") is True and final.get("cancelled_ns") is None
            and final.get("cached_prefix_tokens") == 0, "invalid Ferric final")
    equal(final["prompt_tokens"], reference["prompt_token_ids"], "prompt IDs differ")
    equal(final["generated_tokens"], reference["generated_token_ids"], "output IDs differ")
    equal(final["generated_utf8_bytes"], list(bytes.fromhex(reference["generated_utf8_hex"])),
          "final output bytes differ")


def analyze_engine(directory, engine, pair, reference):
    summary = pair["engines"][engine]
    require(summary.get("engine") == engine and summary.get("gpu_postflight_idle") is True,
            "engine or postflight identity differs")
    for key, expected in (("warmup_requests", 10), ("measured_requests", 30),
                          ("untimed_diagnostics", 2), ("measured_output_tokens", 3840)):
        equal(summary[key], expected, "incomplete cohort: " + key)
    hashes = summary["input_hashes"]

    def retained(name, maximum=4 * 1024**2):
        value, digest = read_json(directory / name, maximum)
        require(digest == sha256(hashes[name]), "retained hash differs: " + name)
        return value

    plan = retained("frozen-plan.json")
    require(hashes["frozen-plan.json"] == pair["plan_sha256"], "plan hash differs")
    equal(plan["settings"], SETTINGS, "plan settings differ")
    identity = retained("identity.json")
    require(identity["engine"] == engine and identity["plan_sha256"] == pair["plan_sha256"],
            "retained engine identity differs")
    equal(identity["settings"], SETTINGS, "identity settings differ")
    receipt = retained("receipt.json")
    require(receipt.get("schema") == PAIR_SCHEMAS[pair["schema"]] and
            receipt.get("engine") == engine and receipt.get("plan_sha256") == pair["plan_sha256"]
            and receipt.get("timing_admitted") is True and receipt.get("cleanup_completed") is True
            and receipt.get("errors") == [] and receipt.get("qualification") is False
            and receipt.get("framework_win_claim") is False, "failed or unadmitted receipt")
    equal(receipt["settings"], SETTINGS, "receipt settings differ")
    equal(receipt["numerical_diagnostics"], [
        {"admitted": True, "classification": "exact_match", "first_token_mismatch": None}
    ] * 2, "diagnostic correctness mismatch")
    replay = receipt["timing_replay"]
    require(replay.get("timing_admitted") is True and
            replay.get("classification") == "exact_utf8_and_usage" and
            replay.get("token_itl_available") is False and
            replay.get("qualification") is False and replay.get("framework_win_claim") is False,
            "timing/correctness replay not accepted")
    before = retained("input-identities-before.json")
    equal(before, retained("input-identities-after.json"), "model/image inputs changed")
    if engine == "ferric":
        equal(summary["cleanup"], {"controller_status": 0, "signals": [],
                                  "threads_joined": True, "worker_close_receipt": True},
              "Ferric shutdown was not clean")
        equal(receipt["timed_stream_checks"], {"timed_final_token_ids_checked": 40,
              "timed_http_request_identities_checked": 40}, "incomplete final token checks")
        events = retained("ferric-final-events.json", 64 * 1024**2)
        finals = [row for row in events if row.get("event") == "request"]
        require(len(finals) == 42, "complete Ferric final roster required")
        equal([row["request_id"] for row in finals], list(range(1, 43)), "final ID roster differs")
        for final in finals:
            validate_final(final, reference)
    else:
        cleanup = summary["cleanup"]
        require(cleanup.get("host_exit_clean") is True and
                cleanup.get("internal_force_kill_observed") is False and
                type(cleanup.get("owned_container_absence_check_status")) is int and
                cleanup["owned_container_absence_check_status"] != 0, "vLLM shutdown was not clean")
        for name in ("diagnostic-before.json", "diagnostic-after.json"):
            diagnostic = retained(name)
            require(len(diagnostic["choices"]) == 1, "one diagnostic choice required")
            choice = diagnostic["choices"][0]
            equal(choice["prompt_token_ids"], reference["prompt_token_ids"], "diagnostic prompt mismatch")
            equal(choice["token_ids"], reference["generated_token_ids"], "diagnostic output mismatch")
            require(choice["text"].encode("utf-8").hex() == reference["generated_utf8_hex"]
                    and choice["finish_reason"] == "length", "diagnostic output mismatch")
    report = retained("timed-raw.json", 64 * 1024**2)
    require(report.get("schema") == "FerricCompetitiveStreamingRunV1" and
            report.get("authority") == "none" and report.get("engine") == engine and
            report.get("completed") is True and report.get("qualification") is False and
            report.get("ttft_semantics") == TTFT and report.get("tpot_semantics") == TPOT and
            report.get("token_itl_available") is False and
            report.get("arrival_policy") == "bounded-closed-loop-windows", "timing boundary differs")
    equal(report["concurrency"], 1, "concurrency differs")
    equal(report["identity"], identity, "raw identity differs")
    require(report["identity_sha256"] == hashes["identity.json"] and
            report["client_sha256"] == pair["client_sha256"] and
            report["workload_sha256"] == reference["workload_sha256"], "client/workload identity differs")
    records, previous, first = [], None, None
    for phase, count in (("warmups", 10), ("samples", 30)):
        windows = report[phase]
        require(type(windows) is list and len(windows) == count, "incomplete " + phase)
        for index, window in enumerate(windows):
            equal(window["index"], index, "window order differs")
            stamps = [clock(window["started_ns"]), clock(window["completed_ns"])]
            require(stamps[0] < stamps[1] and (previous is None or previous <= stamps[0]),
                    "windows overlap or clock changed")
            previous = stamps[1]
            first = stamps[0] if first is None else first
            require(len(window["requests"]) == 1, "one request per window required")
            record = window["requests"][0]
            validate_record(record, reference, stamps)
            if phase == "samples":
                records.append(record)
    measured = [report["samples"][0]["started_ns"], previous]
    equal(summary["measured_window_ns"], measured, "measured span differs")
    metrics = bench.aggregate(records, *measured, SETTINGS["ttft_slo_ms"], SETTINGS["tpot_slo_ms"])
    equal(summary["metrics"], metrics, "summary metrics differ from complete cohort")
    return {"metrics": metrics, "cohort_window_ns": [first, previous],
            "raw_sha256": hashes["timed-raw.json"], "directory": str(directory)}, before


def analyze_pair(entry, root, reference, reference_sha):
    require(type(entry) is dict and set(entry) == {"summary", "ferric", "vllm"},
            "pair roster fields differ")
    pair, digest = read_json(path_at(root, entry["summary"]))
    require(pair.get("schema") in PAIR_SCHEMAS and pair.get("accepted") is True and
            pair.get("qualification") is False and pair.get("framework_win_claim") is False,
            "pair replay not accepted")
    equal(pair["settings"], SETTINGS, "unsupported/mismatched settings")
    require(pair["reference_sha256"] == reference_sha and pair["throughput_formula"] == THROUGHPUT,
            "reference or throughput definition differs")
    for key in ("plan_sha256", "reference_sha256", "client_sha256"):
        sha256(pair[key])
    require(set(pair["engines"]) == {"ferric", "vllm"}, "exact engine pair required")
    engines, inputs = {}, []
    for engine in ("ferric", "vllm"):
        engines[engine], common = analyze_engine(path_at(root, entry[engine]), engine, pair, reference)
        inputs.append(common)
    equal(inputs[0], inputs[1], "engines used different model/image inputs")
    order = sorted(engines, key=lambda engine: engines[engine]["cohort_window_ns"][0])
    require(engines[order[0]]["cohort_window_ns"][1] < engines[order[1]]["cohort_window_ns"][0],
            "engine cohorts overlap")
    equal(pair["cohort_order"], order, "reported engine order differs")
    values = {engine: {"mean_ttft_ms": value["metrics"]["ttft_ms"]["mean"],
                      "mean_tpot_ms": value["metrics"]["tpot_ms"]["mean"],
                      "output_tokens_per_second": value["metrics"]["output_tokens_per_second"]}
              for engine, value in engines.items()}
    ratios = {metric: positive(values["ferric"][metric], metric) /
              positive(values["vllm"][metric], metric) for metric in METRICS}
    return {"summary_sha256": digest, "cohort_order": order, "values": values,
            "ferric_over_vllm": ratios, "engines": engines}, {
                key: pair[key] for key in PROFILE_FIELDS}


def spread(values):
    return {"count": len(values), "min": min(values), "max": max(values),
            "mean": statistics.mean(values), "median": statistics.median(values),
            "sample_stdev": statistics.stdev(values) if len(values) > 1 else None}


def summarize(manifest_path):
    manifest_path = Path(manifest_path).resolve(strict=True)
    manifest, manifest_sha = read_json(manifest_path)
    require(type(manifest) is dict and set(manifest) == {"schema", "reference", "pairs"} and
            manifest["schema"] == MANIFEST_SCHEMA, "invalid series manifest")
    entries = manifest["pairs"]
    require(type(entries) is list and 1 <= len(entries) <= 32, "requires 1..32 declared pairs")
    reference, reference_sha = read_json(path_at(manifest_path.parent, manifest["reference"]))
    for key in ("prompt_token_ids", "generated_token_ids"):
        require(type(reference[key]) is list and len(reference[key]) == 128 and
                all(type(value) is int and 0 <= value < 2**32 for value in reference[key]),
                "reference token roster differs")
    bytes.fromhex(reference["generated_utf8_hex"]).decode("utf-8")
    attempts, errors, profile, seen = [], [], None, set()
    previous_order, previous_end = None, None
    for index, entry in enumerate(entries):
        attempt = {"index": index, "inputs": entry, "accepted": False}
        try:
            value, current = analyze_pair(entry, manifest_path.parent, reference, reference_sha)
            if profile is not None:
                equal(current, profile, "frozen profile/build differs across starts")
            profile = current
            order = value["cohort_order"]
            require(previous_order is None or order == list(reversed(previous_order)),
                    "paired starts must alternate AB/BA")
            begin = value["engines"][order[0]]["cohort_window_ns"][0]
            end = value["engines"][order[1]]["cohort_window_ns"][1]
            require(previous_end is None or previous_end < begin,
                    "pair roster is not chronological or clocks are incomparable")
            for engine in value["engines"].values():
                require(engine["raw_sha256"] not in seen and engine["directory"] not in seen,
                        "reused cohort is not a fresh start")
                seen.update((engine["raw_sha256"], engine["directory"]))
            previous_order, previous_end = order, end
            attempt.update(accepted=True, **value)
        except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as error:
            attempt["error"] = type(error).__name__ + ": " + str(error)
            errors.append({"index": index, "error": attempt["error"]})
        attempts.append(attempt)
    accepted = not errors
    return {"schema": SCHEMA, "manifest_sha256": manifest_sha, "accepted": accepted,
            "classification": ("rejected-series" if errors else
                               "repeated-pair-screening" if len(entries) >= 3 else "limited-pair-screening"),
            "competitiveness_accepted": False, "qualification": False, "framework_win_claim": False,
            "declared_pairs": len(entries), "attempts": attempts, "errors": errors,
            "profile": profile, "ttft_semantics": TTFT, "tpot_semantics": TPOT,
            "token_itl_available": False, "throughput_formula": THROUGHPUT,
            "ratio_directions": {"mean_ttft_ms": "lower", "mean_tpot_ms": "lower",
                                 "output_tokens_per_second": "higher"},
            "ferric_over_vllm_spread": {metric: spread([
                attempt["ferric_over_vllm"][metric] for attempt in attempts
            ]) for metric in METRICS} if accepted else None,
            "limitations": [
                "Descriptive ratios of paired-start means, not pooled request percentiles or confidence intervals.",
                "Measured-span throughput includes inter-window gaps and drain; not sustained goodput.",
                "FP32 head, TP1/C1, ISL128/OSL128, speculation and prefix caching off only.",
                "vLLM exact token IDs are checked only in two untimed diagnostics per start.",
                "Full launch/custody validation is delegated to the frozen per-pair replay.",
                "Roster completeness, common host boot, and independent starts require external launch records.",
                "No held-out-suite, equal-p99-SLO, environment-drift or release qualification is established.",
            ]}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        result = summarize(args.manifest)
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as error:
        print(json.dumps({"schema": SCHEMA, "accepted": False,
                          "error": type(error).__name__ + ": " + str(error)}), file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
