#!/usr/bin/env python3
"""Revalidate fixed-workload correctness before comparing opt-in host diagnostics."""

import argparse
import copy
import importlib.util
import json
from pathlib import Path
import re

SPEC = importlib.util.spec_from_file_location("performance_ledger", Path(__file__).with_name("performance_ledger.py"))
LEDGER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(LEDGER)
CHECK = LEDGER.CHECK
SCHEMA = "FerricHostTimingManifestV1"
KEYS = ("batch", "phase", "category", "label", "rank")
TOTALS = ("count", "failed", "elapsed_ns", "max_ns", "request_payload_bytes", "response_payload_bytes", "dispatches")
SIDE_FIELDS = {"schema", "clock", "measurement", "aggregation", "payload_accounting", "record_limit",
               "incomplete", "active_records", "records", "run_status", "failure", "controller_pid",
               "setup", "closed", "workload_sha256"}
CONSTANTS = {"schema": "FerricHostTimingV1", "clock": "controller-std-instant",
             "measurement": "host-wall-latency-not-gpu-duration", "aggregation": "overlapping-not-additive",
             "payload_accounting": "payload-only-excludes-wire-headers", "record_limit": 65536,
             "incomplete": False, "active_records": 0, "run_status": "completed", "failure": None}


def validate_manifest(manifest):
    CHECK.require(type(manifest) is dict and manifest.get("schema") == SCHEMA, "host timing manifest schema")
    ordinary = copy.deepcopy(manifest)
    ordinary["schema"] = LEDGER.SCHEMA
    for variant in ordinary.get("variants", []):
        for run in variant.get("runs", []):
            CHECK.fields(run, LEDGER.RUN_FIELDS | {"timing", "timing_sha256"}, "timed run")
            CHECK.require(type(run["timing"]) is str and Path(run["timing"]).is_absolute(), "absolute timing path")
            CHECK.hash_value(run["timing_sha256"], "timing hash")
            del run["timing"], run["timing_sha256"]
    LEDGER.validate_manifest(ordinary)
    return ordinary


def validate_sidecar(sidecar, records, expected):
    CHECK.fields(sidecar, SIDE_FIELDS, "host timing sidecar")
    for key, value in CONSTANTS.items():
        CHECK.require(type(sidecar[key]) is type(value) and sidecar[key] == value, f"host timing {key} drift/failure")
    CHECK.integer(sidecar["controller_pid"], 1, (1 << 31) - 1, "controller PID")
    CHECK.require(LEDGER.canonical(sidecar["setup"]) == LEDGER.canonical(records[0])
                  and LEDGER.canonical(sidecar["closed"]) == LEDGER.canonical(records[-1]),
                  "sidecar setup/close identity differs from exact raw trace")
    CHECK.require(sidecar["workload_sha256"] == expected["workload_sha256"], "sidecar workload identity")
    batches = {row["pool_batch_id"] for row in records if row["schema"] == CHECK.SCHEMA_PREFIX + "CompletedV2"}
    rows = sidecar["records"]
    CHECK.require(type(rows) is list and 1 <= len(rows) <= 65536, "timing record bound")
    seen, physical, controller_count, dispatches = set(), set(), 0, 0
    global_spans, sends, receipts = {}, {}, {}
    for row in rows:
        CHECK.fields(row, set(KEYS) | set(TOTALS), "timing aggregate")
        CHECK.require(row["batch"] is None or CHECK.integer(row["batch"], 1) in batches,
                      "timing batch not in completed trace")
        CHECK.require(row["rank"] is None or CHECK.integer(row["rank"], 0, expected["world"] - 1) < expected["world"],
                      "timing rank outside exact world")
        for key in ("label", "phase"):
            CHECK.require(type(row[key]) is str and re.fullmatch(r"[a-z][a-z0-9_]{0,95}", row[key]), "timing label")
        CHECK.require(row["category"] in ("span", "ipc_send", "ipc_roundtrip"), "timing category")
        identity = tuple(row[key] for key in KEYS)
        CHECK.require(identity not in seen, "duplicate timing aggregate")
        seen.add(identity)
        for key in TOTALS:
            CHECK.integer(row[key], 1 if key == "count" else 0, CHECK.U64_MAX, key)
        CHECK.require(row["failed"] == 0, "failed or abandoned transport observation")
        CHECK.require(row["max_ns"] <= row["elapsed_ns"] <= row["max_ns"] * row["count"], "inconsistent duration aggregate")
        if row["category"] == "span":
            CHECK.require(all(row[key] == 0 for key in ("request_payload_bytes", "response_payload_bytes", "dispatches")),
                          "span cannot claim bytes or dispatches")
            if row["label"] == "batch":
                CHECK.require(row["batch"] in batches and row["rank"] is None and row["count"] == 1,
                              "physical batch timing multiplicity")
                CHECK.require(row["batch"] not in physical, "duplicate physical batch timing")
                physical.add(row["batch"])
            if row["label"] == "controller_batch":
                CHECK.require(row["batch"] is None and row["rank"] is None, "controller batch timing scope")
                controller_count += row["count"]
            if row["label"] in ("controller", "setup", "workload", "close"):
                CHECK.require(row["batch"] is None and row["rank"] is None and row["count"] == 1,
                              "top-level timing scope")
                CHECK.require(row["label"] not in global_spans, "duplicate top-level span")
                global_spans[row["label"]] = row["elapsed_ns"]
        elif row["category"] == "ipc_roundtrip":
            dispatches += row["dispatches"]
            receipts[(row["batch"], row["phase"], row["label"], row["rank"])] = row
            if row["label"] == "dispatch_round":
                CHECK.require(expected["collective"] == CHECK.CONCURRENT_COLLECTIVE and row["rank"] is None
                              and row["count"] <= row["dispatches"] <= 8 * row["count"], "round timing profile/count")
            else:
                CHECK.require(row["rank"] is not None, "rank-local request missing rank")
                if row["label"] == "dispatch":
                    CHECK.require(row["dispatches"] == row["count"], "dispatch timing count")
                elif row["label"] == "dispatch_sequence":
                    CHECK.require(expected["performance_profile"] is not None
                                  and expected["performance_profile"]["dispatch_sequences"] is True
                                  and row["count"] <= row["dispatches"] <= 16 * row["count"], "sequence timing profile/count")
                else:
                    CHECK.require(row["dispatches"] == 0, "non-dispatch ticket claims GPU dispatches")
        else:
            CHECK.require(row["response_payload_bytes"] == 0 and row["dispatches"] == 0, "send cannot claim responses/dispatches")
            sends[(row["batch"], row["phase"], row["label"], row["rank"])] = row
    CHECK.require(physical == batches and controller_count == len(batches), "missing/excess controller or physical batch timings")
    CHECK.require(set(global_spans) == {"controller", "setup", "workload", "close"}, "missing whole-path spans")
    CHECK.require(sum(global_spans[key] for key in ("setup", "workload", "close")) <= global_spans["controller"],
                  "disjoint outer phase durations exceed controller lifetime")
    CHECK.require(all(row["max_ns"] <= global_spans["controller"] for row in rows), "record exceeds controller lifetime")
    CHECK.require(dispatches == sum(records[-1]["rank_dispatch_counts"]), "transport dispatch count differs from completed trace")
    CHECK.require(set(sends) == set(receipts), "missing send/receipt timing pair")
    for key, sent in sends.items():
        received = receipts[key]
        CHECK.require(sent["count"] == received["count"] and sent["request_payload_bytes"] == received["request_payload_bytes"]
                      and sent["elapsed_ns"] <= received["elapsed_ns"], "send/receipt aggregate mismatch")
    return rows


def load_timed_run(run, expected):
    ordinary = {key: run[key] for key in LEDGER.RUN_FIELDS}
    metrics, equivalence = LEDGER.load_run(ordinary, expected)
    timing_path = Path(run["timing"])
    CHECK.require(timing_path.resolve(strict=True) == timing_path, "noncanonical timing path")
    raw = CHECK.read_bounded(timing_path, 64 * 1024 * 1024)
    CHECK.require(CHECK.sha256(raw) == run["timing_sha256"], "externally pinned timing sidecar drifted")
    trace = CHECK.read_bounded(Path(run["run_dir"]) / "results.jsonl", 8 * 1024 * 1024)
    CHECK.require(CHECK.sha256(trace) == metrics["input_sha256"]["results.jsonl"], "trace changed after exact validation")
    records = [CHECK.json_value(line) for line in trace.splitlines()]
    rows = validate_sidecar(CHECK.json_value(raw), records, expected)
    # Keep per-batch observations in the raw sidecar. This comparison aggregates
    # only the same named host operation, never different request latencies.
    aggregates = {}
    for row in rows:
        key = tuple(row[field] for field in KEYS if field != "batch")
        total = aggregates.setdefault(key, {field: 0 for field in TOTALS})
        for field in TOTALS:
            total[field] = max(total[field], row[field]) if field == "max_ns" else total[field] + row[field]
    return {"metrics": metrics, "timing_sha256": run["timing_sha256"], "aggregates": aggregates}, equivalence


def aggregate(manifest, loader=load_timed_run):
    validate_manifest(manifest)
    shared, variants = None, []
    for variant in manifest["variants"]:
        runs, keys = [], set()
        for run in variant["runs"]:
            observation, equivalence = loader(run, variant["expect"])
            if shared is None:
                shared = equivalence
            CHECK.require(LEDGER.canonical(shared) == LEDGER.canonical(equivalence), "incompatible workload/world/cache/roster/budget")
            runs.append(observation)
            keys.update(observation["aggregates"])
        observations = []
        for key in sorted(keys, key=lambda value: tuple("" if item is None else str(item) for item in value)):
            values = [run["aggregates"].get(key) for run in runs]
            CHECK.require(all(value is not None for value in values), "timing operation set changed between repetitions")
            observations.append({**dict(zip(("phase", "category", "label", "rank"), key, strict=True)),
                "samples": values, "n": len(values), "mean_elapsed_ns": sum(v["elapsed_ns"] for v in values) / len(values),
                "mean_count": sum(v["count"] for v in values) / len(values)})
        variants.append({"name": variant["name"], "kind": variant["kind"], "expected": variant["expect"],
                         "repetitions": len(runs), "observations": observations,
                         "runs": [{k: v for k, v in run.items() if k != "aggregates"} for run in runs]})
    baseline = next(variant for variant in variants if variant["name"] == manifest["baseline"])
    baseline_rows = {tuple(row[key] for key in ("phase", "category", "label", "rank")): row for row in baseline["observations"]}
    for variant in variants:
        for row in variant["observations"]:
            reference = baseline_rows.get(tuple(row[key] for key in ("phase", "category", "label", "rank")))
            row["baseline_elapsed_ratio"] = (row["mean_elapsed_ns"] / reference["mean_elapsed_ns"]
                if reference and reference["mean_elapsed_ns"] > 0 else None)
    return {"schema": "FerricHostTimingSummaryV1", "baseline": manifest["baseline"], "equivalence": shared,
            "variants": variants, "measurement": CONSTANTS["measurement"],
            "nonclaims": ["host scopes and concurrent rank waits overlap; do not sum them as GPU duration",
                "parent response wait includes worker execution, checks, scheduling, pipe I/O and decoding",
                "send includes channel enqueue and pipe write acknowledgement, not pure IPC cost",
                "sequence/round phase attribution follows actual flush/wait location, not deferred logical kernel ownership",
                "all variants are explicitly profiled; instrumentation overhead is not removed",
                "fixed short logical-tick workload, not steady-state serving or a stable tail",
                "timing observations confer no new runtime authority or numerical qualification"]}


def markdown(report):
    lines = ["# Ferric Host Timing", "", "Host wall latency only. Nested/cross-rank timings overlap; none are GPU durations.", ""]
    for variant in report["variants"]:
        lines += [f"## {variant['name']} ({variant['kind']}, n={variant['repetitions']})", "",
                  "| Phase | Operation | Rank | Mean calls | Mean host seconds | vs baseline |",
                  "|---|---|---:|---:|---:|---:|"]
        for row in variant["observations"]:
            ratio = row["baseline_elapsed_ratio"]
            relative = "n/a" if ratio is None else f"{ratio:.4f}x"
            lines.append(f"| {row['phase']} | {row['category']}:{row['label']} | {row['rank']} | "
                         f"{row['mean_count']:.1f} | {row['mean_elapsed_ns'] / 1e9:.6f} | {relative} |")
        lines.append("")
    lines += ["## Scope", "", *[f"- {text}" for text in report["nonclaims"]]]
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    args = parser.parse_args()
    raw = CHECK.read_bounded(args.manifest, 1024 * 1024)
    report = aggregate(CHECK.json_value(raw))
    report["manifest_sha256"] = CHECK.sha256(raw)
    report["tool_sha256"] = CHECK.sha256(CHECK.read_bounded(Path(__file__), 128 * 1024))
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n")
    if args.markdown:
        args.markdown.write_text(markdown(report))


if __name__ == "__main__":
    main()
