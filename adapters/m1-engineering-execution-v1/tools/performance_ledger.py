#!/usr/bin/env python3
"""Revalidate pinned fixed-workload receipts and report per-variant observations."""

import argparse
import importlib.util
import json
import math
from pathlib import Path
import re


SPEC = importlib.util.spec_from_file_location("compare_tp_batch", Path(__file__).with_name("compare_tp_batch.py"))
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)
SCHEMA = "FerricTpPerformanceLedgerManifestV1"
EXPECT_FIELDS = CHECK.IDENTITY_FIELDS | {"world", "prefix_cache", "output_head_pruning", "collective",
                                         "performance_profile", "workload_sha256", "reference_sha256"}
RUN_FIELDS = {"id", "run_dir", "workload", "reference", "comparison", "comparison_sha256"}
EQUIVALENCE_FIELDS = ("model", "dtype", "target", "tensor_parallel", "device_unique_ids", "model_bundle_id",
                      "batch_tokens", "prefill_chunk", "page_tokens", "physical_pages", "context_tokens",
                      "cache_ttl_ticks", "prefix_cache", "max_batches", "arrival_policy")
REQUEST_METRICS = ("ttft_seconds", "tpot_seconds", "output_tokens_per_second")
GLOBAL_METRICS = ("output_tokens_per_second", "workload_window_seconds", "setup_seconds", "whole_seconds")


def name(value, label):
    CHECK.require(type(value) is str and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,79}", value),
                  f"invalid {label}")
    return value


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def validate_manifest(value):
    CHECK.fields(value, {"schema", "baseline", "warmup_policy", "variants"}, "ledger manifest")
    CHECK.require(value["schema"] == SCHEMA, "unknown ledger manifest schema")
    CHECK.require(value["warmup_policy"] == "fresh-worker-no-warmup",
                  "only the fixed fresh-worker/no-warmup policy is supported")
    baseline = name(value["baseline"], "baseline")
    variants = value["variants"]
    CHECK.require(type(variants) is list and 1 <= len(variants) <= 32, "variant count")
    variant_names, run_names, directories = set(), set(), set()
    for variant in variants:
        CHECK.fields(variant, {"name", "kind", "expect", "runs"}, "variant")
        identifier = name(variant["name"], "variant")
        CHECK.require(identifier not in variant_names, "duplicate variant")
        variant_names.add(identifier)
        CHECK.require(variant["kind"] in ("baseline", "standalone", "cumulative"), "variant kind")
        CHECK.require((variant["kind"] == "baseline") == (identifier == baseline), "baseline kind mismatch")
        expected = variant["expect"]
        peer = type(expected) is dict and expected.get("collective") == CHECK.PEER_COLLECTIVE
        wide = type(expected) is dict and "wide_kernel_profile" in expected
        extra = ({"peer_artifact"} if peer else set()) | ({"wide_kernel_profile"} if wide else set())
        CHECK.fields(expected, EXPECT_FIELDS | extra, "variant expectation")
        CHECK.require(type(expected["world"]) is int and expected["world"] in (1, 2, 8), "expected world")
        CHECK.require(type(expected["prefix_cache"]) is bool, "expected cache flag")
        CHECK.require(expected["output_head_pruning"] is None or type(expected["output_head_pruning"]) is bool,
                      "expected pruning flag")
        CHECK.require(expected["collective"] is None or expected["collective"] in CHECK.COLLECTIVES,
                      "expected collective")
        if peer:
            CHECK.require(expected["world"] in (2, 8), "peer collective world")
            CHECK.peer_artifact(expected["peer_artifact"])
        if expected["performance_profile"] is not None:
            CHECK.performance_profile(expected["performance_profile"])
        if wide:
            CHECK.wide_kernel_profile(expected["wide_kernel_profile"], expected["performance_profile"])
        for field in CHECK.IDENTITY_FIELDS | {"workload_sha256", "reference_sha256"}:
            CHECK.hash_value(expected[field], field)
        CHECK.require(expected["reference_sha256"] == CHECK.REFERENCE_SHA256, "frozen reference pin")
        runs = variant["runs"]
        CHECK.require(type(runs) is list and 1 <= len(runs) <= 20, "repetition count")
        for run in runs:
            CHECK.fields(run, RUN_FIELDS, "run")
            run_id = name(run["id"], "run ID")
            CHECK.require(run_id not in run_names, "duplicate run ID")
            run_names.add(run_id)
            CHECK.hash_value(run["comparison_sha256"], "comparison hash")
            for field in ("run_dir", "workload", "reference", "comparison"):
                CHECK.require(type(run[field]) is str and Path(run[field]).is_absolute(),
                              f"absolute {field} required")
            CHECK.require(run["run_dir"] not in directories, "run directory reused as a repetition")
            directories.add(run["run_dir"])
    CHECK.require(baseline in variant_names and len(run_names) <= 128, "baseline or total repetition count")
    return value


def statistics(values):
    CHECK.require(type(values) is list and values, "empty metric observations")
    if all(value is None for value in values):
        return None
    CHECK.require(values and all(type(value) in (int, float) and math.isfinite(value) and value > 0
                                for value in values), "inconsistent or nonpositive metric")
    ordered = sorted(values)
    count = len(ordered)
    return {"n": count, "mean": sum(ordered) / count, "min": ordered[0], "max": ordered[-1],
            "p50": ordered[math.ceil(count * 0.50) - 1],
            "p95": ordered[math.ceil(count * 0.95) - 1]}


def relative(current, baseline, higher_is_better):
    if current is None or baseline is None:
        CHECK.require(current is None and baseline is None, "missing baseline metric")
        return None
    result = {}
    for statistic in ("mean", "p50", "p95"):
        ratio = current[statistic] / baseline[statistic]
        result[statistic] = {"variant_over_baseline": ratio,
                             "improvement_percent": (ratio - 1 if higher_is_better else 1 - ratio) * 100}
    return result


def extract_metrics(records):
    setup, closed = records[0], records[-1]
    requests = {}
    terminal_times, admission_times = [], []
    for record in records:
        if record["schema"] != CHECK.SCHEMA_PREFIX + "RequestV2":
            continue
        timestamps = record["output_timestamps_ns"]
        arrival = record["arrival_ns"]
        terminal = record["cancelled_ns"] if record["state"] == "Cancelled" else timestamps[-1]
        CHECK.require(terminal > arrival, "nonpositive request window")
        intervals = [right - left for left, right in zip(timestamps, timestamps[1:])]
        count = len(record["generated_tokens"])
        requests[record["name"]] = {
            "slot": record["slot"], "generation": record["generation"], "state": record["state"],
            "generated_tokens": count, "decode_interval_count": len(intervals),
            "ttft_seconds": (timestamps[0] - arrival) / 1e9,
            "tpot_seconds": sum(intervals) / len(intervals) / 1e9 if intervals else None,
            "output_tokens_per_second": count * 1e9 / (terminal - arrival),
            "terminal_window_ns": terminal - arrival,
        }
        admission_times.append(arrival)
        terminal_times.append(terminal)
    CHECK.require(set(requests) == set(CHECK.NAMES), "request metric identities")
    window = max(terminal_times) - min(admission_times)
    CHECK.require(window > 0, "nonpositive workload window")
    output_count = sum(request["generated_tokens"] for request in requests.values())
    return {"requests": requests, "output_tokens": output_count,
            "output_tokens_per_second": output_count * 1e9 / window,
            "workload_window_seconds": window / 1e9, "setup_seconds": setup["setup_seconds"],
            "whole_seconds": closed["whole_seconds"]}


def load_run(run, expected):
    for field in ("run_dir", "workload", "reference", "comparison"):
        path = Path(run[field])
        CHECK.require(path.resolve(strict=True) == path, f"noncanonical {field}")
    comparison_bytes = CHECK.read_bounded(run["comparison"], 1024 * 1024)
    CHECK.require(CHECK.sha256(comparison_bytes) == run["comparison_sha256"], "pinned correctness report drifted")
    prior = CHECK.json_value(comparison_bytes)
    report = CHECK.compare(run["run_dir"], run["workload"], run["reference"], expected["world"],
                           {key: expected[key] for key in CHECK.IDENTITY_FIELDS}, expected["prefix_cache"],
                           expected["output_head_pruning"], expected["collective"], expected["performance_profile"],
                           expected["workload_sha256"], expected["reference_sha256"], expected.get("peer_artifact"),
                           expected.get("wide_kernel_profile"))
    CHECK.require(report["passed"] is True and report["gpu_idle_before_and_after"] is True,
                  "current comparison did not pass pinned idle evidence")
    CHECK.require(canonical(prior) == canonical(report), "correctness report is stale or differs from current raw validation")
    raw = CHECK.read_bounded(Path(run["run_dir"]) / "results.jsonl", 8 * 1024 * 1024)
    CHECK.require(CHECK.sha256(raw) == report["input_sha256"]["results.jsonl"], "raw trace changed after validation")
    records = [CHECK.json_value(line) for line in raw.splitlines()]
    equivalence = {key: records[0][key] for key in EQUIVALENCE_FIELDS}
    equivalence.update(workload_sha256=expected["workload_sha256"], reference_sha256=expected["reference_sha256"])
    metrics = extract_metrics(records)
    metrics.update(id=run["id"], run_dir=run["run_dir"], comparison_sha256=run["comparison_sha256"],
                   input_sha256=report["input_sha256"], identities=report["identities"],
                   batch_count=report["batch_count"], physical_token_rows=report["physical_token_rows"],
                   rank_dispatch_counts=report["rank_dispatch_counts"])
    return metrics, equivalence


def aggregate(manifest, loader=load_run):
    validate_manifest(manifest)
    variants, shared_equivalence = [], None
    for variant in manifest["variants"]:
        runs = []
        for run in variant["runs"]:
            metrics, equivalence = loader(run, variant["expect"])
            if shared_equivalence is None:
                shared_equivalence = equivalence
            CHECK.require(canonical(equivalence) == canonical(shared_equivalence),
                          "baseline equivalence drift: world/roster/context/chunk/cache/budget/workload/reference")
            runs.append(metrics)
        requests = {}
        for request_name in CHECK.NAMES:
            observations = [run["requests"][request_name] for run in runs]
            identity_fields = ("slot", "generation", "state", "generated_tokens", "decode_interval_count")
            identity = {key: observations[0][key] for key in identity_fields}
            CHECK.require(all(all(item[key] == identity[key] for key in identity_fields) for item in observations),
                          "request identity or decode interval count changed across repetitions")
            requests[request_name] = {**identity,
                "metrics": {key: statistics([item[key] for item in observations]) for key in REQUEST_METRICS}}
        variants.append({"name": variant["name"], "kind": variant["kind"], "expected": variant["expect"],
                         "repetitions": len(runs), "requests": requests, "runs": runs,
                         "metrics": {key: statistics([run[key] for run in runs]) for key in GLOBAL_METRICS},
                         "tail_warning": "nearest-rank p95 is a small-sample order statistic, not a stable serving tail"
                         if len(runs) < 20 else "p95 describes only this short logical-tick workload"})
    baseline = next(variant for variant in variants if variant["name"] == manifest["baseline"])
    for variant in variants:
        variant["baseline_relative"] = {key: relative(variant["metrics"][key], baseline["metrics"][key],
                                                       key == "output_tokens_per_second") for key in GLOBAL_METRICS}
        for request_name in CHECK.NAMES:
            request = variant["requests"][request_name]
            request["baseline_relative"] = {key: relative(request["metrics"][key],
                baseline["requests"][request_name]["metrics"][key], key == "output_tokens_per_second")
                for key in REQUEST_METRICS}
    return {"schema": "FerricTpPerformanceLedgerV1", "authority": "none", "baseline": manifest["baseline"],
            "warmup_policy": manifest["warmup_policy"], "equivalence": shared_equivalence, "variants": variants,
            "percentile_method": "nearest rank: sorted[ceil(p*n)-1]; never pool different request identities",
            "metric_definitions": {
                "ttft": "first committed output minus actual request admission",
                "tpot": "per-request arithmetic mean of adjacent committed output gaps; absent with fewer than two outputs",
                "request_output_rate": "request output count / (request terminal event minus its actual admission)",
                "workload_output_rate": "all eight outputs, including cancellation output, / (last terminal event minus first actual admission)",
                "terminal_event": "last output for a completed request; cancelled_ns for a cancelled request",
                "relative": "variant/baseline for each statistic; positive improvement means lower latency or higher output rate",
            },
            "nonclaims": ["short fixed logical-tick workload, not steady-state serving throughput or an SLO",
                          "no pooling of TTFT or TPOT across requests; one-output cancelled request has no TPOT",
                          "small repetition counts do not establish a stable p95 tail or statistical significance",
                          "fresh workers do not guarantee cold GPU caches or exclusive host resources",
                          "artifact/executable hashes and idle snapshots are observations, not authenticated attestations"]}


def markdown(report):
    lines = ["# Ferric Performance Ledger", "",
             "Fixed four-request logical-tick workload; eight output tokens including one from the cancelled request.",
             "Not steady-state serving throughput. Percentiles use nearest rank; small-sample p95 is not a stable tail.", "",
             "| Variant | Kind | Reps | Output tok/s p50 / p95 | vs Baseline p50 | Window s p50 | Setup s p50 | Whole s p50 |",
             "|---|---|---:|---:|---:|---:|---:|---:|"]
    for variant in report["variants"]:
        metrics = variant["metrics"]
        improvement = variant["baseline_relative"]["output_tokens_per_second"]["p50"]["improvement_percent"]
        lines.append(f"| {variant['name']} | {variant['kind']} | {variant['repetitions']} | "
                     f"{metrics['output_tokens_per_second']['p50']:.6f} / {metrics['output_tokens_per_second']['p95']:.6f} | {improvement:+.2f}% | "
                     f"{metrics['workload_window_seconds']['p50']:.6f} | {metrics['setup_seconds']['p50']:.6f} | "
                     f"{metrics['whole_seconds']['p50']:.6f} |")
    for request_name in CHECK.NAMES:
        lines += ["", f"## {request_name}", "",
                  "| Variant | Reps | Decode Gaps/Rep | TTFT s p50 / p95 | TTFT Improvement p50 | TPOT s p50 / p95 | TPOT Improvement p50 | Output tok/s p50 / p95 |",
                  "|---|---:|---:|---:|---:|---:|---:|---:|"]
        for variant in report["variants"]:
            request = variant["requests"][request_name]
            ttft, tpot = request["metrics"]["ttft_seconds"], request["metrics"]["tpot_seconds"]
            ttft_relative = request["baseline_relative"]["ttft_seconds"]["p50"]["improvement_percent"]
            tpot_text = "n/a" if tpot is None else f"{tpot['p50']:.6f} / {tpot['p95']:.6f}"
            tpot_relative = "n/a" if tpot is None else f"{request['baseline_relative']['tpot_seconds']['p50']['improvement_percent']:+.2f}%"
            rate = request["metrics"]["output_tokens_per_second"]
            lines.append(f"| {variant['name']} | {variant['repetitions']} | {request['decode_interval_count']} | "
                         f"{ttft['p50']:.6f} / {ttft['p95']:.6f} | {ttft_relative:+.2f}% | {tpot_text} | {tpot_relative} | "
                         f"{rate['p50']:.6f} / {rate['p95']:.6f} |")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--json-output", type=Path, required=True)
    parser.add_argument("--markdown-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        manifest_bytes = CHECK.read_bounded(args.manifest, 4 * 1024 * 1024)
        report = aggregate(CHECK.json_value(manifest_bytes))
        report["manifest_sha256"] = CHECK.sha256(manifest_bytes)
        report["ledger_sha256"] = CHECK.sha256(CHECK.read_bounded(Path(__file__), 128 * 1024))
        report["comparator_sha256"] = CHECK.sha256(CHECK.read_bounded(Path(CHECK.__file__), 128 * 1024))
        with args.json_output.open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2, allow_nan=False)
            output.write("\n")
        with args.markdown_output.open("x", encoding="utf-8") as output:
            output.write(markdown(report))
    except (ValueError, KeyError, TypeError, IndexError, OSError) as error:
        parser.exit(1, f"performance ledger rejected: {error}\n")


if __name__ == "__main__":
    main()
