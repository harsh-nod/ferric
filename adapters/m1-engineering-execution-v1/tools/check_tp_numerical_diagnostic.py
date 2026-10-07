#!/usr/bin/env python3
"""Bound TP1 capture custody and fixed-token parity; never performance acceptance."""

import argparse
import copy
import heapq
import importlib.util
import json
import math
from pathlib import Path
import struct


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


HERE = Path(__file__).resolve().parent
CHECK = module("diagnostic_frozen_comparator", HERE / "compare_tp_batch.py")
CHECK_SHA = "1be4f2b3b0f8d1c96ef0b5bf21388da232d6b23e5700c7bef6295117b6093b6a"
REPLAY_SHA = "0008f8425f00c2f1668a584fad99a2234bfd27277b261fdaba0f0fcc1916e3e1"
NORMAL_STATUS = "Contracted; independently compare emitted token IDs; not a serving qualification"
DIAGNOSTIC_STATUS = "Diagnostic readback run; all timings unqualified; fixed-reference token checks remain unchanged"
SCHEMA = "FerricTpNumericalDiagnosticV1"
ROLES = {"Query", "Key", "Value", "AttentionOutput", "Gate", "Up", "Down"}
require = CHECK.require


def exact(left, right, label):
    require(json.dumps(left, sort_keys=True, allow_nan=False) ==
            json.dumps(right, sort_keys=True, allow_nan=False), label)


def descriptor(value, name, size, width=None):
    fields = {"file", "bytes", "sha256"}
    if width is not None:
        fields |= {"buffer_id", "buffer_offset", "element_bytes"}
    CHECK.fields(value, fields, "capture payload descriptor")
    exact([value["file"], value["bytes"]], [name, size], "payload name/extent")
    CHECK.hash_value(value["sha256"], "payload digest")
    if width is not None:
        CHECK.integer(value["buffer_id"], 1)
        offset = CHECK.integer(value["buffer_offset"])
        require(offset % width == 0 and offset + size <= CHECK.U64_MAX, "payload buffer extent/alignment")
        exact(value["element_bytes"], width, "payload element width")


def kernel_name(mode, partial=False):
    if mode == "mfma":
        return "ferric_qwen3_tp_mfma_gemm_partial_f32_v3" if partial else "ferric_qwen3_tp_mfma_gemm_bf16_v3"
    return "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2" if partial else "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2"


def expectation(value):
    CHECK.fields(value, CHECK.IDENTITY_FIELDS | {
        "schema", "workload_sha256", "reference_sha256", "prefix_cache", "output_head_pruning",
        "performance_profile", "collective", "physical_gpu_ids", "selection"}, "diagnostic expectation")
    require(value["schema"] == "FerricTpNumericalDiagnosticExpectationV1", "expectation schema")
    for key in CHECK.IDENTITY_FIELDS | {"workload_sha256", "reference_sha256"}:
        CHECK.hash_value(value[key], key)
    for key in ("prefix_cache", "output_head_pruning"):
        require(type(value[key]) is bool, "diagnostic boolean policy")
    profile = CHECK.performance_profile(value["performance_profile"])
    require(profile["projection"] in ("baseline", "mfma") and profile["attention"] == "baseline"
            and all(not profile[key] for key in CHECK.PROFILE_BOOLS - {"runtime_operational"}),
            "diagnostic supports only baseline/MFMA and optional operational checks")
    require(value["collective"] in (None, "host-staged-reuse-v3", "device-tp1-v3"), "TP1 collective")
    ids = CHECK.integer_list(value["physical_gpu_ids"], 1)
    require(len(ids) == 8 and len(set(ids)) == 8, "external eight-GPU roster")
    selection = value["selection"]
    CHECK.fields(selection, {"batch_ordinal", "layer", "role"}, "capture selection")
    CHECK.integer(selection["batch_ordinal"], 1, 64)
    CHECK.integer(selection["layer"], 0, 35)
    require(selection["role"] in ROLES, "capture role")
    return value


def observed_stream(records, expected, gpu_ids):
    """Check complete causal structure using actual tokens, not a parity oracle."""
    setup = records[0]
    CHECK.check_setup(setup, 1, gpu_ids, {key: expected[key] for key in CHECK.IDENTITY_FIELDS},
                      expected["prefix_cache"], expected["output_head_pruning"],
                      expected["collective"], expected["performance_profile"], None, None)
    require(setup["batch_tokens"] == setup["prefill_chunk"] == 16, "diagnostic geometry")
    cache = setup["prefix_cache"]
    batches, order = CHECK.timeline(cache)
    require(len(records) == len(order) + 2, "diagnostic stream order/count")
    prompts = [CHECK.PROMPT_IDS + CHECK.REFERENCE_IDS[:prefix] for prefix in CHECK.PREFIX_LENGTHS]
    progress = [0, 0, 0, 16 if cache else 0]
    generated, timestamps = [[] for _ in CHECK.NAMES], [[] for _ in CHECK.NAMES]
    admissions, completed = {}, []
    retained = [1, 3, 4, 3, 2] if cache else [1, 3, 4, 2, 2, 2]
    last_time, dispatches, row_count = 0, 0, 0
    for value, (action, index) in zip(records[1:-1], order, strict=True):
        if action == "admit":
            CHECK.record(value, "Admission", CHECK.ADMISSION_FIELDS)
            slot, generation = CHECK.REQUEST_IDS[index]
            exact([value["name"], value["slot"], value["generation"], value["tick"]],
                  [CHECK.NAMES[index], slot, generation, index], "admission identity")
            exact(value["prompt_tokens"], prompts[index], "admission prompt")
            hit = 16 if cache and index == 3 else 0
            exact([value["cached_tokens"], value["cached_pages"]], [hit, hit // 16], "cache admission")
            last_time = CHECK.integer(value["arrival_ns"], last_time)
            admissions[index] = value
        elif action == "batch":
            CHECK.record(value, "Completed", CHECK.BATCH_FIELDS | {"output_head_rows"})
            exact([value["tick"], value["batch_id"], value["pool_batch_id"]],
                  [index, index + 1, index + 1], "batch identity")
            start = CHECK.integer(value["started_ns"], last_time)
            end = CHECK.integer(value["completed_ns"], start + 1)
            require(type(value["outputs"]) is list, "output list")
            rows, events = [], []
            for request, begin, count in batches[index]:
                require(request in admissions and progress[request] == begin, "schedule progress")
                for position in range(begin, begin + count):
                    prompt = prompts[request]
                    kind = ("PrefillIntermediate" if position + 1 < len(prompt) else
                            "PrefillFinal" if position < len(prompt) else "Decode")
                    token = prompt[position] if position < len(prompt) else generated[request][-1]
                    slot, generation = CHECK.REQUEST_IDS[request]
                    rows.append(dict(slot=slot, generation=generation, token=token, position=position, kind=kind))
                    if kind != "PrefillIntermediate":
                        require(len(events) < len(value["outputs"]), "missing output event")
                        choice = CHECK.integer(value["outputs"][len(events)]["token"], 0, 151935)
                        ordinal = len(generated[request])
                        generated[request].append(choice)
                        timestamps[request].append(end)
                        events.append(dict(slot=slot, generation=generation, token=choice, index=ordinal,
                                           completed_ns=end, finished=ordinal + 1 == CHECK.REQUESTED_LENGTHS[request]))
                progress[request] += count
            exact(value["rows"], rows, "actual causal input rows/order/kinds")
            exact(value["outputs"], events, "actual output order/identity/flags")
            head_rows = len(events) if expected["output_head_pruning"] else len(rows)
            exact(value["output_head_rows"], head_rows, "head extent")
            count = 541 + (3 if head_rows else 0) + (72 if expected["collective"] == "device-tp1-v3" else 0)
            exact(value["rank_dispatch_counts"], [count], "batch dispatch count")
            dispatches += count
            hits = int(cache and index >= 3)
            for key, wanted in {"retained_pages": retained[index],
                                "free_pages": setup["physical_pages"] - retained[index],
                                "cached_pages": hits, "prefix_hits": hits,
                                "hit_tokens": hits * 16, "evicted_pages": 0}.items():
                exact(value[key], wanted, "page accounting " + key)
            row_count += len(rows)
            completed.append(value)
            last_time = end
        else:
            CHECK.record(value, "Request", CHECK.REQUEST_FIELDS)
            slot, generation = CHECK.REQUEST_IDS[index]
            exact([value["name"], value["slot"], value["generation"]],
                  [CHECK.NAMES[index], slot, generation], "request identity")
            exact(value["prompt_tokens"], prompts[index], "request prompt")
            exact(value["generated_tokens"], generated[index], "request/committed tokens")
            require(len(generated[index]) == CHECK.OUTPUT_LENGTHS[index], "request output count")
            require(type(value["generated_text"]) is str, "decoded text type")
            exact(value["generated_utf8_bytes"], list(value["generated_text"].encode("utf-8")), "decoded byte consistency")
            exact(value["output_timestamps_ns"], timestamps[index], "request/output clock consistency")
            admission = admissions[index]
            for key, original in (("arrival_ns", "arrival_ns"), ("arrival_tick", "tick"),
                                  ("cached_prefix_tokens", "cached_tokens")):
                exact(value[key], admission[original], "request admission " + key)
            if index == 2:
                require(value["state"] == "Cancelled", "cancel state")
                last_time = CHECK.integer(value["cancelled_ns"], last_time)
            else:
                require(value["state"] == "Completed" and value["cancelled_ns"] is None, "completed state")
            gaps = [right - left for left, right in zip(timestamps[index], timestamps[index][1:])]
            require(all(gap > 0 for gap in gaps), "nonpositive output gap")
            exact(value["ttft_ns"], timestamps[index][0] - admission["arrival_ns"], "reported TTFT arithmetic")
            exact(value["decode_intervals_ns"], gaps, "reported interval arithmetic")
            exact(value["tpot_ns"], sum(gaps) // len(gaps) if gaps else None, "reported TPOT arithmetic")
    require(row_count == (34 if cache else 50), "total physical rows")
    closed = records[-1]
    CHECK.record(closed, "Closed", CHECK.CLOSED_FIELDS)
    exact(closed["worker_pids"], setup["worker_pids"], "closed process roster")
    require(closed["all_workers_exited"] is True, "worker close incomplete")
    exact(closed["rank_dispatch_counts"], [dispatches], "closed dispatch total")
    whole = CHECK.positive_seconds(closed["whole_seconds"], "whole-run clock")
    require(whole + 0.000001 >= setup["setup_seconds"] + last_time / 1e9, "controller clock order")
    return completed, generated


def bind_capture(manifest, receipt, selection, records, completed, expected, raw_workload_hash, manifest_hash, total):
    CHECK.fields(manifest, {"schema", "authority", "complete", "model_parity_qualified", "performance_qualified",
                 "timing_scope", "identity", "batch_ordinal", "scheduler_batch_id", "pool_batch_id", "execution_rows",
                 "projection", "head", "payload_bytes_before_manifest", "maximum_total_bytes"}, "capture manifest")
    require(manifest["complete"] is True and manifest["model_parity_qualified"] is False
            and manifest["performance_qualified"] is False, "manifest qualification flags")
    exact(manifest["timing_scope"], "diagnostic readbacks invalidate all performance measurements for this run",
          "manifest diagnostic timing scope")
    exact(manifest["maximum_total_bytes"], 224 * 1024 * 1024, "manifest byte cap")
    CHECK.fields(receipt, {"schema", "authority", "performance_qualified", "manifest", "total_bytes"}, "capture receipt")
    require(receipt["schema"] == "FerricTpNumericalCaptureReceiptV1" and receipt["authority"] == "none"
            and receipt["performance_qualified"] is False, "capture receipt scope")
    CHECK.fields(receipt["manifest"], {"file", "bytes", "sha256"}, "manifest receipt")
    exact(receipt["manifest"], {"file": "manifest.json", "bytes": selection["manifest_bytes"],
                                "sha256": manifest_hash}, "manifest receipt identity")
    exact(receipt["total_bytes"], total, "capture total receipt")
    setup = records[0]
    profile = expected["performance_profile"]
    identity = {key: setup[key] for key in CHECK.IDENTITY_FIELDS | {"model_bundle_id", "session_id", "tensor_parallel",
               "running_worker_sha256", "batch_tokens", "prefill_chunk", "prefix_cache", "output_head_pruning"}}
    identity.update(requests_sha256=raw_workload_hash, device_unique_id=setup["device_unique_ids"][0],
                    projection=profile["projection"], runtime_operational=profile["runtime_operational"],
                    runtime_cache_admission=profile["runtime_cache_admission"], queue_rollover=profile["queue_rollover"],
                    collective=expected["collective"] or "host-staged-v1")
    exact(manifest["identity"], identity, "capture/trace identity")
    selected = expected["selection"]
    ordinal = selected["batch_ordinal"]
    require(ordinal <= len(completed), "selected batch absent")
    batch = completed[ordinal - 1]
    exact([manifest["batch_ordinal"], manifest["scheduler_batch_id"], manifest["pool_batch_id"]],
          [ordinal, batch["batch_id"], batch["pool_batch_id"]], "selected batch binding")
    exact([manifest["projection"]["layer"], manifest["projection"]["role"]],
          [selected["layer"], selected["role"]], "selected projection binding")
    order = list(range(len(batch["rows"])))
    if expected["output_head_pruning"]:
        order.sort(key=lambda source: batch["rows"][source]["kind"] == "PrefillIntermediate")
    row_map = [{**batch["rows"][source], "execution_row": physical, "source_row": source,
                "publishable": batch["rows"][source]["kind"] != "PrefillIntermediate"}
               for physical, source in enumerate(order)]
    exact(manifest["execution_rows"], row_map, "capture execution/source row map")
    projection = manifest["projection"]
    CHECK.fields(projection, {"kernel", "role", "layer", "rows", "n", "k", "world", "tag", "input",
                 "weights_nk", "actual_weights", "actual_weight_layout", "output"}, "captured projection")
    n, k, tag = {"Query": (4096, 4096, 1), "Key": (1024, 4096, 2), "Value": (1024, 4096, 3),
                 "AttentionOutput": (4096, 4096, 1), "Gate": (12288, 4096, 4),
                 "Up": (12288, 4096, 5), "Down": (4096, 12288, 2)}[selected["role"]]
    exact([projection["rows"], projection["n"], projection["k"], projection["world"], projection["tag"]],
          [len(row_map), n, k, 1, tag], "captured projection geometry")
    partial = selected["role"] in ("AttentionOutput", "Down")
    exact(projection["kernel"], kernel_name(profile["projection"], partial), "projection kernel/profile/role")
    descriptor(projection["input"], "projection-input.bf16", len(row_map) * k * 2, 2)
    descriptor(projection["weights_nk"], "projection-weights-nk.bf16", n * k * 2, 2)
    if profile["projection"] == "mfma":
        exact(projection["actual_weight_layout"], "kn", "MFMA weight layout")
        descriptor(projection["actual_weights"], "projection-weights-kn.bf16", n * k * 2, 2)
    else:
        exact(projection["actual_weight_layout"], "nk", "baseline weight layout")
        exact(projection["actual_weights"], projection["weights_nk"], "baseline weight descriptor")
    width = 4 if partial else 2
    descriptor(projection["output"], "projection-output.f32" if partial else "projection-output.bf16",
               len(row_map) * n * width, width)
    head = manifest["head"]
    exact(head["rows"], batch["output_head_rows"], "head captured row extent")
    exact(head["kernel"], kernel_name(profile["projection"]), "head kernel/profile")
    descriptor(head["input"], "head-input.bf16", head["rows"] * 4096 * 2, 2)
    descriptor(head["logits"], "head-logits.bf16", head["rows"] * 151936 * 2, 2)
    exact(head["watch_label_scope"], "token IDs only; no tokenizer decoding assumed", "head watch label scope")
    require(len(head["request_rows"]) == head["rows"], "head summary count")
    events = {(event["slot"], event["generation"]): event for event in batch["outputs"]}
    for summary, row in zip(head["request_rows"], row_map[:head["rows"]], strict=True):
        exact(summary["row_identity"], row, "head row identity")
        choice = CHECK.integer(summary["gpu_choice"], 0, 151935)
        if row["publishable"]:
            exact(choice, events[(row["slot"], row["generation"])]["token"], "head/committed token binding")


def validate_head(manifest, payloads):
    head = manifest["head"]
    CHECK.fields(head, {"kernel", "rows", "n", "k", "input", "logits", "request_rows", "selected_weight_tokens",
                 "selected_weights_nk", "original_weight_buffer_id", "original_weight_buffer_offset", "watch_tokens",
                 "watch_label_scope"}, "capture head")
    exact([head["n"], head["k"], head["watch_tokens"]], [151936, 4096, [9856, 17689]], "head geometry/watch IDs")
    data = payloads[head["logits"]["file"]]
    require(len(data) == head["rows"] * 151936 * 2, "head logits payload extent")
    require(len(payloads[head["input"]["file"]]) == head["rows"] * 4096 * 2, "head input extent")
    selected = {9856, 17689}
    for index, summary in enumerate(head["request_rows"]):
        CHECK.fields(summary, {"row_identity", "gpu_choice", "top16", "top1_minus_top2", "top_two_tied",
                     "tie_policy", "watch"}, "head summary")
        for label, count in (("top16", 16), ("watch", 2)):
            require(type(summary[label]) is list and len(summary[label]) == count, "head summary extent")
            for entry in summary[label]:
                CHECK.fields(entry, {"token", "bf16_bits", "value"}, "logit entry")
                CHECK.integer(entry["token"], 0, 151935)
                CHECK.integer(entry["bf16_bits"], 0, 65535)
                require(type(entry["value"]) in (int, float) and math.isfinite(entry["value"]), "logit value")
        values = []
        for (bits,) in struct.iter_unpack("<H", data[index * 151936 * 2:(index + 1) * 151936 * 2]):
            value = struct.unpack("<f", struct.pack("<I", bits << 16))[0]
            require(math.isfinite(value), "head nonfinite logits")
            values.append((value, bits))
        tokens = heapq.nlargest(16, range(151936), key=lambda token: (values[token][0], -token))
        top = [{"token": token, "bf16_bits": values[token][1], "value": values[token][0]} for token in tokens]
        # JSON may render an exact floating value as an integer; validate numbers separately.
        require(summary["top16"] == top, "head top16 differs from actual logits")
        exact(summary["gpu_choice"], tokens[0], "head CPU/GPU argmax")
        require(type(summary["top1_minus_top2"]) in (int, float)
                and summary["top1_minus_top2"] == top[0]["value"] - top[1]["value"], "head top margin")
        exact(summary["top_two_tied"], top[0]["value"] == top[1]["value"], "head tie flag")
        exact(summary["tie_policy"], "lowest token ID among equal finite BF16 logits", "head tie policy")
        require(summary["watch"] == [{"token": token, "bf16_bits": values[token][1], "value": values[token][0]}
                                     for token in (9856, 17689)], "head watched logits")
        selected.update(tokens)
    exact(head["selected_weight_tokens"], sorted(selected), "selected LM weight rows")
    descriptor(head["selected_weights_nk"], "head-selected-weights-nk.bf16", len(selected) * 4096 * 2)
    require(len(payloads[head["selected_weights_nk"]["file"]]) == len(selected) * 4096 * 2, "selected LM weights extent")


def diagnose(run_dir, capture, workload, reference, expected, manifest_hash, replay):
    expectation(expected)
    CHECK.hash_value(manifest_hash, "manifest hash")
    files = {"status": CHECK.read_bounded(run_dir / "status", 16),
             "gpu-before.json": CHECK.read_bounded(run_dir / "gpu-before.json", 65536),
             "gpu-after.json": CHECK.read_bounded(run_dir / "gpu-after.json", 65536),
             "results.jsonl": CHECK.read_bounded(run_dir / "results.jsonl", 8 * 1024 * 1024),
             "workload.json": CHECK.read_bounded(workload, 1024 * 1024),
             "reference.json": CHECK.read_bounded(reference, 128 * 1024)}
    require(files["status"] == b"0\n", "diagnostic process failed")
    CHECK.load_reference(files["reference.json"])
    CHECK.load_workload(files["workload.json"])
    for name in ("workload", "reference"):
        exact(CHECK.sha256(files[name + ".json"]), expected[name + "_sha256"], "external " + name + " identity")
    gpu_ids = CHECK.gpu_roster(files["gpu-before.json"])
    exact(gpu_ids, CHECK.gpu_roster(files["gpu-after.json"]), "post-run idle roster")
    exact(gpu_ids, expected["physical_gpu_ids"], "external physical roster")
    raw = files["results.jsonl"]
    require(raw.endswith(b"\n"), "truncated diagnostic stream")
    lines = raw.splitlines()
    require(1 < len(lines) <= 64 and all(lines), "diagnostic JSONL count")
    records = [CHECK.json_value(line) for line in lines]
    normalized = copy.deepcopy(records)
    selection = normalized[0].pop("numerical_capture")
    receipt = normalized[-1].pop("numerical_capture")
    CHECK.fields(selection, {"schema", "batch_ordinal", "layer", "role", "directory", "performance_qualified"}, "capture selection")
    require(selection["schema"] == "FerricTpNumericalSelectionV1" and selection["performance_qualified"] is False
            and type(selection["directory"]) is str and Path(selection["directory"]).is_absolute(), "selection scope")
    exact({key: selection[key] for key in expected["selection"]}, expected["selection"], "external capture selection")
    exact(normalized[0]["numerical_status"], DIAGNOSTIC_STATUS, "diagnostic status required")
    normalized[0]["numerical_status"] = NORMAL_STATUS
    completed, generated = observed_stream(normalized, expected, gpu_ids)
    manifest, payloads, actual_hash, total = replay.load_capture(capture, manifest_hash)
    # Parse again with the strict finite/duplicate JSON decoder before binding.
    raw_manifest = CHECK.read_bounded(capture / "manifest.json", 1024 * 1024)
    exact(CHECK.sha256(raw_manifest), actual_hash, "manifest retained identity")
    exact(CHECK.json_value(raw_manifest), manifest, "manifest parser agreement")
    require(manifest["authority"] == "none", "manifest authority")
    bind_capture(manifest, receipt, {"manifest_bytes": len(raw_manifest)}, normalized, completed, expected,
                 expected["workload_sha256"], actual_hash, total)
    validate_head(manifest, payloads)
    replay.analyze(manifest, payloads)
    error = None
    try:
        report = CHECK.validate_records(normalized, gpu_ids, 1,
                 {key: expected[key] for key in CHECK.IDENTITY_FIELDS}, expected["prefix_cache"],
                 expected["output_head_pruning"], expected["collective"], expected["performance_profile"])
        require(report["passed"] is True, "unbound strict reference result")
    except ValueError as failure:
        error = str(failure)
    outputs = []
    for index, name in enumerate(CHECK.NAMES):
        wanted = CHECK.REFERENCE_IDS[CHECK.PREFIX_LENGTHS[index]:CHECK.PREFIX_LENGTHS[index] + CHECK.OUTPUT_LENGTHS[index]]
        outputs.append({"name": name, "slot": CHECK.REQUEST_IDS[index][0], "generation": CHECK.REQUEST_IDS[index][1],
                        "observed_token_ids": generated[index], "expected_token_ids": wanted,
                        "exact_token_ids_match": generated[index] == wanted})
    return {"schema": SCHEMA, "authority": "none", "performance_qualified": False,
            "capture_custody_validated": True, "complete_observed_trace_validated": True,
            "fixed_reference_passed": error is None, "fixed_reference_failure": error,
            "requests": outputs, "manifest_sha256": actual_hash, "total_capture_bytes": total,
            "identity": manifest["identity"], "selection": expected["selection"],
            "input_sha256": {name: CHECK.sha256(data) for name, data in files.items()},
            "checker_sha256": CHECK_SHA, "replay_sha256": REPLAY_SHA,
            "normalization": ["remove exact validated Setup numerical_capture", "remove exact validated Closed numerical_capture",
                              "replace exact diagnostic numerical_status with frozen normal status in memory only"],
            "nonclaims": ["no performance or serving qualification", "observed-token causality is not reference parity",
                          "sampled numerical replay is not exhaustive projection correctness",
                          "idle/close records are observations, not authority attestations"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("run-dir", "capture", "workload", "reference", "expect", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--manifest-sha256", required=True)
    args = parser.parse_args()
    replay_path = HERE.parents[2] / "tools/replay_tp_numerical_capture.py"
    exact(CHECK.sha256(CHECK.read_bounded(HERE / "compare_tp_batch.py", 128 * 1024)), CHECK_SHA, "frozen checker source")
    exact(CHECK.sha256(CHECK.read_bounded(replay_path, 128 * 1024)), REPLAY_SHA, "frozen replay source")
    replay = module("diagnostic_frozen_replay", replay_path)
    expected_raw = CHECK.read_bounded(args.expect, 65536)
    result = diagnose(args.run_dir, args.capture, args.workload, args.reference,
                      CHECK.json_value(expected_raw), args.manifest_sha256, replay)
    result["expectation_sha256"] = CHECK.sha256(expected_raw)
    result["source_sha256"] = CHECK.sha256(CHECK.read_bounded(Path(__file__), 128 * 1024))
    with args.output.open("x", encoding="utf-8") as target:
        json.dump(result, target, indent=2, sort_keys=True, allow_nan=False)
        target.write("\n")
    return 0 if result["fixed_reference_passed"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
