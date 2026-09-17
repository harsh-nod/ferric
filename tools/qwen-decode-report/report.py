#!/usr/bin/env python3
"""Validate retained decode observations and render non-authoritative reports."""
import argparse
import json
from pathlib import Path
import sys

from common import CHECK, M1, canonical, digest, fields, load_module, pinned, read_json, require, tool_identities, write_new
import draft
import plots


def load_draft(path):
    manifest, raw = read_json(path, 1024 * 1024)
    result, host, device = draft.load_manifest(manifest, path.parent)
    result["manifest_sha256"] = digest(raw)
    result["report_tool_sha256"] = tool_identities()
    return result, host, device, manifest


def draft_report(path, destination):
    result, host, device, _ = load_draft(path)
    destination.mkdir(mode=0o700)
    write_new(destination / "report.json", result)
    plots.rates(destination / "decode.svg", [(f"request {r['run']}", r["post_first_decode_tokens_per_second"])
                for r in result["requests"]], "Qwen3-0.6B post-first decode observations (not the 8B target)")
    plots.timeline(destination / "host-timeline.svg", host)
    if device is not None:
        write_new(destination / "imported-trace.json", device)
        plots.timeline(destination / "imported-timeline.svg", device)
    notes = ["# Draft Decode Observation Report", "", "Authority: none. Not M1 qualification or authenticated hardware evidence.", "",
        f"Model: {result['equivalence']['model']}; {result['sampling_class']}.",
        f"Measured requests: {result['metrics']['measured_requests']}; excluded warmups: {result['metrics']['excluded_warmups']}.",
        f"Pooled post-first decode rate: {result['metrics']['pooled_post_first_decode_tokens_per_second']:.6f} tokens/s.",
        f"Aggregate output/window rate: {result['metrics']['aggregate_output_tokens_per_second']:.6f} tokens/s.",
        "These are different denominators. Neither is GPU execution time.", "",
        "The 700 tokens/s goal applies to single-request Qwen3-8B only, not this 0.6B bring-up.",
        "The separate analytical bandwidth ceilings are assumptions, not measurements or universal limits.", "",
        *["- " + text for text in result["nonclaims"]]]
    with (destination / "README.md").open("x") as stream:
        stream.write("\n".join(notes) + "\n")
    return result


def m1_report(path, destination):
    manifest, raw = read_json(path, 1024 * 1024)
    host = load_module("decode_report_existing_m1_host", M1 / "host_timing_summary.py")
    original = host.aggregate(manifest)
    # Existing M1 validators retain private device identities; do not export them.
    result = {"schema": "FerricM1HostObservationPlotV1", "authority": "none",
        "measurement": original["measurement"], "baseline": original["baseline"],
        "manifest_sha256": digest(raw), "validated_existing_m1_report_sha256": digest(canonical(original)),
        "report_tool_sha256": tool_identities(), "gpu_overlap_measured": False,
        "equivalence": {key: original["equivalence"][key] for key in host.LEDGER.EQUIVALENCE_FIELDS if key != "device_unique_ids"},
        "workload_sha256": original["equivalence"]["workload_sha256"],
        "reference_sha256": original["equivalence"]["reference_sha256"],
        "goal_comparison": "Not applicable: fixed multi-request M1 workload is not the single-request Qwen3-8B 700 tok/s goal",
        "performance_qualified": False,
        "nonclaims": original["nonclaims"], "variants": []}
    for variant in original["variants"]:
        metrics = [run["metrics"] for run in variant["runs"]]
        result["variants"].append({"name": variant["name"], "kind": variant["kind"],
            "repetitions": variant["repetitions"], "host_observations": variant["observations"],
            "binary_and_artifact_identities": {key: variant["expected"][key] for key in host.CHECK.IDENTITY_FIELDS},
            "runtime_profile": variant["expected"]["performance_profile"],
            "retained_capture_sha256": [{"timing": run["timing_sha256"], "comparison": run["metrics"]["comparison_sha256"],
                                          "raw": run["metrics"]["input_sha256"]} for run in variant["runs"]],
            "request_metrics": [value["requests"] for value in metrics],
            "output_tokens_per_second": host.LEDGER.statistics([value["output_tokens_per_second"] for value in metrics])})
    destination.mkdir(mode=0o700)
    write_new(destination / "report.json", result)
    plots.rates(destination / "host-workload.svg", [(v["name"], v["output_tokens_per_second"]["mean"])
                for v in result["variants"]], "Existing M1 fixed-workload host output rate (not sustained serving)")
    return result


def compare(path, destination):
    manifest, raw = read_json(path, 1024 * 1024)
    fields(manifest, {"schema", "authority", "baseline", "variants"}, "ablation manifest")
    require(manifest["schema"] == "FerricDecodeAblationManifestV1" and manifest["authority"] == "none", "ablation schema")
    rows = manifest["variants"]
    require(type(rows) is list and 2 <= len(rows) <= 16, "ablation count")
    reports, names, device = [], set(), None
    for row in rows:
        fields(row, {"name", "manifest"}, "ablation row")
        CHECK.require(type(row["name"]) is str and 0 < len(row["name"]) <= 64, "ablation name")
        name = row["name"]
        require(name not in names, "duplicate ablation")
        names.add(name)
        report_path, report_raw = pinned(row["manifest"], path.parent, 1024 * 1024)
        definition = CHECK.json_value(report_raw)
        result, _, _ = draft.load_manifest(definition, report_path.parent)
        actual_device = definition["expect"]["device_unique_id"]
        if reports:
            require(result["equivalence"] == reports[0]["report"]["equivalence"], "ablation model/workload/precision/context/environment drift")
            require(actual_device == device, "ablation device drift")
        device = actual_device
        reports.append({"name": name, "report": result, "manifest_sha256": digest(report_raw)})
    require(manifest["baseline"] in names, "missing baseline")
    baseline = next(row["report"] for row in reports if row["name"] == manifest["baseline"])
    base_rate = baseline["metrics"]["pooled_post_first_decode_tokens_per_second"]
    variants = []
    for row in reports:
        result = row["report"]
        rate = result["metrics"]["pooled_post_first_decode_tokens_per_second"]
        options = result["runtime_options"]
        changed_options = {key: {"baseline": baseline["runtime_options"][key], "variant": value}
                           for key, value in options.items() if value != baseline["runtime_options"][key]}
        changed_bindings = [key for key, value in result["provenance_sha256"].items()
                            if value != baseline["provenance_sha256"].get(key)]
        variants.append({"name": row["name"], "manifest_sha256": row["manifest_sha256"], "metrics": result["metrics"],
            "runtime_option_changes": changed_options, "changed_provenance_fields": changed_bindings,
            "decode_rate_ratio": rate / base_rate, "decode_rate_change_percent": (rate / base_rate - 1) * 100})
    result = {"schema": "FerricDecodeAblationReportV1", "authority": "none", "baseline": manifest["baseline"],
        "manifest_sha256": digest(raw), "report_tool_sha256": tool_identities(), "equivalence": baseline["equivalence"],
        "variants": variants, "goal": draft.goal_metadata(), "performance_qualified": False,
        "trace_overhead": "unmeasured; this capture always emits per-step records and has no matched trace-off switch",
        "nonclaim": "Descriptive same-workload observations, not causal attribution, statistical significance, M1 qualification or the Qwen3-8B target."}
    destination.mkdir(mode=0o700)
    write_new(destination / "report.json", result)
    plots.rates(destination / "ablations.svg", [(v["name"], v["metrics"]["pooled_post_first_decode_tokens_per_second"])
                for v in variants], "Same-workload Draft06B decode ablations (host observations)")
    lines = ["# Decode Ablations", "", "| Variant | Measured Requests | Decode Tokens/s | Ratio |", "|---|---:|---:|---:|"]
    for row in variants:
        clean_name = row["name"].replace("|", "\\|").replace("\n", " ")
        lines.append(f"| {clean_name} | {row['metrics']['measured_requests']} | {row['metrics']['pooled_post_first_decode_tokens_per_second']:.6f} | {row['decode_rate_ratio']:.6f} |")
    lines += ["", result["nonclaim"], "", "Trace overhead is unmeasured. Multiple changed bindings/options preclude single-change causal attribution."]
    with (destination / "README.md").open("x") as stream:
        stream.write("\n".join(lines) + "\n")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("validate", "draft", "m1-host", "compare"):
        command = sub.add_parser(name)
        command.add_argument("--manifest", type=Path, required=True)
        if name != "validate":
            command.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "validate":
        result = load_draft(args.manifest)[0]
    else:
        result = {"draft": draft_report, "m1-host": m1_report, "compare": compare}[args.command](args.manifest, args.output)
    print(json.dumps({"schema": result["schema"], "authority": "none", "records_validated": True,
                      "gpu_overlap_claim": False, "performance_qualified": False}, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, UnicodeError) as error:
        print("decode report rejected: " + str(error), file=sys.stderr)
        sys.exit(1)
