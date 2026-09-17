"""Strict adapter for the existing Draft06B v10 repeated-decode capture."""
from common import CHECK, canonical, digest, fields, hash_value, integer, percentiles, pinned, require, statistics
from trace import validate_trace
from artifact import handoff_binding

PREFIX = "FerricDraftDecodeProfile"
IDENTITY = set("model_bundle_id draft_model_id draft_config_id draft_weights_sha256".split())
SETUP = set("""schema authority performance_qualified clock clock_origin_ns setup_completed_ns timing_boundary timing_exclusions timing_includes
device_timing device_overlap_claim sampling_class warmups samples new_tokens prompt prompt_tokens initial_kv_tokens prefill_tokens context_tokens physical_pages
model model_revision model_role identity projection prefill head_precision collective attention tensor_parallel row_capacity prefix_cache eos_stopping
reference_sha256 reference_producer controller_sha256 worker_sha256 running_worker_sha256 worker_pid device_unique_id artifact_hsaco_id artifact_manifest_id artifact_handoff_id
runtime_cache_admission runtime_operational runtime_sequences runtime_ordered_batches runtime_rollover expected_forwards_per_run expected_dispatches_per_forward
expected_total_dispatches no_rollover_packet_budget reused_worker_weights_and_pool retry_policy draft_payload_bytes retained_target_payload_bytes transposed_weight_bytes""".split())
STEP = set("schema authority run warmup ordinal output_index inputs positions selected_row choice expected_choice cache_tokens completed_dispatches reserve_start_ns forward_start_ns forward_complete_ns commit_complete_ns reference_passed".split())
RUN = set("schema authority performance_qualified run warmup run_start_ns first_token_ns terminal_ns retired_ns generated_tokens expected_tokens generated_utf8_bytes reference_passed kv_tokens_processed steps completed_dispatches free_pages retained_pages cached_pages quarantined_pages".split())
CLOSED = set("schema authority performance_qualified execution_completed error completed_runs all_workers_exited close_error teardown_start_ns teardown_end_ns worker_pid rank_dispatch_counts completed_batches".split())
REFERENCE = set("schema model model_revision identity head_precision prefill prompt_tokens steps generated_tokens generated_utf8_bytes producer".split())
EXPECT = set("model model_revision identity projection prefill head_precision warmups samples new_tokens runtime_cache_admission runtime_operational device_unique_id".split())
FILES = set("capture reference controller worker hsaco artifact_manifest handoff model_manifest environment plan compiler".split())
MODEL = "Qwen/Qwen3-0.6B"
REVISION = "c1899de289a04d12100db370d81485cdf75e47ca"
PACKETS = 480
LIMIT_NS = 24 * 60 * 60 * 10**9


def goal_metadata():
    weight_bytes = 16381470720 - 151936 * 4096 * 2 + 4096 * 2
    return {"model": "Qwen/Qwen3-8B", "revision": "b968826d9c46dd6066d109eabc6255188de91218",
            "single_request": True, "batch_size": 1, "target_decode_tokens_per_second": 700,
            "applicable_to_this_draft06b_capture": False, "target_reached_claim": False,
            "analytical_only": {"assumed_streamed_weight_bytes_per_token": weight_bytes,
                "optimistic_product_peak_bytes_per_second": 8_000_000_000_000,
                "optimistic_product_weight_streaming_tokens_per_second": 8_000_000_000_000 / weight_bytes,
                "installed_host_advertised_bytes_per_second": 6_810_000_000_000,
                "installed_host_advertised_weight_streaming_tokens_per_second": 6_810_000_000_000 / weight_bytes,
                "product_source": "https://rocm.docs.amd.com/en/latest/reference/gpu-arch/mi350.html",
                "host_source": "operator-reported sanitized AMD-SMI static max_bandwidth, not measured bandwidth",
                "caveat": "BF16 weight-streaming model, no cache credit; excludes KV, activations and compute. Not a measured or universal upper bound."}}


def token_list(value, label, maximum=151935):
    require(type(value) is list and len(value) <= 132, label)
    return [integer(item, 0, maximum, label) for item in value]


def schedule(prefill, prompt, ordinal, previous):
    if prefill == "full" and ordinal == 0:
        return prompt, list(range(5)), 4
    position = ordinal + 4 if prefill == "full" else ordinal
    token = prompt[position] if position < 5 else previous
    require(token is not None, "missing previous completed choice")
    return [token], [position], 0


def validate_reference(ref, setup):
    fields(ref, REFERENCE, "reference")
    require(ref["schema"] == "FerricDraftDecodeProfileReferenceV1" or
            (setup["new_tokens"] == 2 and ref["schema"] == "FerricDraftPagedCanaryReferenceV10"), "not full-model draft reference")
    for key in ("model", "model_revision", "identity", "head_precision", "prefill", "prompt_tokens"):
        require(ref[key] == setup[key], "reference/setup identity drift")
    require(type(ref["producer"]) is str and 0 < len(ref["producer"]) <= 1024
            and ref["producer"] == setup["reference_producer"], "reference producer")
    require(type(ref["generated_utf8_bytes"]) is list and len(ref["generated_utf8_bytes"]) <= 16384, "decoded byte bound")
    for byte in ref["generated_utf8_bytes"]:
        integer(byte, 0, 255, "decoded byte")
    bytes(ref["generated_utf8_bytes"]).decode("utf-8")
    outputs = token_list(ref["generated_tokens"], "reference output tokens")
    require(len(outputs) == setup["new_tokens"] and type(ref["steps"]) is list
            and len(ref["steps"]) == setup["expected_forwards_per_run"], "reference length")
    previous, selected = None, []
    for ordinal, step in enumerate(ref["steps"]):
        fields(step, {"inputs", "positions", "selected_row", "choice", "cache_tokens"}, "reference step")
        token_list(step["inputs"], "reference inputs")
        token_list(step["positions"], "reference positions")
        for key in ("selected_row", "choice", "cache_tokens"):
            integer(step[key], 0, 151935, "reference step scalar")
        inputs, positions, row = schedule(setup["prefill"], setup["prompt_tokens"], ordinal, previous)
        require(step["inputs"] == inputs and step["positions"] == positions and step["selected_row"] == row
                and step["cache_tokens"] == positions[-1] + 1, "independent reference schedule")
        integer(step["choice"], 0, 151935, "reference choice")
        if step["cache_tokens"] >= 5:
            selected.append(step["choice"])
        previous = step["choice"]
    require(selected == outputs, "reference generated choices")


def validate_setup(setup, expect, hashes):
    fields(setup, SETUP, "setup")
    fields(expect, EXPECT, "predeclared expectation")
    require(all(setup[key] == value for key, value in expect.items()), "predeclared setup drift")
    fields(setup["identity"], IDENTITY, "model identity")
    for value in setup["identity"].values():
        hash_value(value, "model identity")
    constants = {"schema": PREFIX + "SetupV1", "authority": "none", "performance_qualified": False,
        "clock": "CLOCK_MONOTONIC_RAW", "device_timing": False, "device_overlap_claim": False,
        "timing_boundary": "host-reserve-through-checked-completion-and-KV-commit",
        "timing_exclusions": "model/artifact/worker setup; tokenization; retirement; teardown",
        "timing_includes": "controller/IPC/queue/waits; per-step observation emission between token completions",
        "model": MODEL, "model_revision": REVISION, "model_role": "Draft06B", "head_precision": "fp32-v10",
        "collective": "draft-device-tp1-v10", "attention": "baseline", "tensor_parallel": 1, "row_capacity": 32,
        "prefix_cache": False, "eos_stopping": False, "initial_kv_tokens": 0, "prefill_tokens": 5,
        "runtime_sequences": False, "runtime_ordered_batches": False, "runtime_rollover": False,
        "reused_worker_weights_and_pool": True, "retry_policy": "none-stop-on-first-error",
        "expected_dispatches_per_forward": PACKETS, "no_rollover_packet_budget": 131072}
    for key, value in constants.items():
        require(type(setup[key]) is type(value) and setup[key] == value, "fixed profile contract: " + key)
    require(setup["prefill"] in ("full", "tokenwise") and setup["projection"] in ("baseline", "wave", "mfma", "auto"), "profile policy")
    for key in ("runtime_cache_admission", "runtime_operational"):
        require(type(setup[key]) is bool, "runtime flag")
    n = integer(setup["new_tokens"], 2, 128, "output limit")
    integer(setup["warmups"], 0, 10, "warmups")
    integer(setup["samples"], 1, 30, "samples")
    prompt = token_list(setup["prompt_tokens"], "prompt")
    require(len(prompt) == 5 and setup["prompt"] == "The capital of France is", "fixed genuine prompt")
    forwards = n + (4 if setup["prefill"] == "tokenwise" else 0)
    pages = (n + 4 + 15) // 16
    total = (setup["warmups"] + setup["samples"]) * forwards * PACKETS
    for key in ("expected_forwards_per_run", "physical_pages", "context_tokens", "expected_total_dispatches"):
        integer(setup[key], 1, 131072, "profile extent/count")
    require(setup["expected_forwards_per_run"] == forwards and setup["physical_pages"] == pages
            and setup["context_tokens"] == pages * 16 and setup["expected_total_dispatches"] == total
            and total <= 131072, "exact context/dispatch budget")
    wanted_class = "benchmark-sized-unqualified" if setup["warmups"] == 10 and setup["samples"] == 30 else "diagnostic-only"
    require(setup["sampling_class"] == wanted_class, "sampling class")
    for field, pin in (("reference_sha256", "reference"), ("controller_sha256", "controller"),
                       ("worker_sha256", "worker"), ("running_worker_sha256", "worker"),
                       ("artifact_hsaco_id", "hsaco"), ("artifact_manifest_id", "artifact_manifest"),
                       ("artifact_handoff_id", "handoff")):
        require(setup[field] == hashes[pin], "actual pinned binary/reference/artifact mismatch")
    integer(setup["worker_pid"], 1, (1 << 31) - 1, "retained private worker identity")
    integer(setup["device_unique_id"], 1, (1 << 64) - 1, "retained private device identity")
    integer(setup["clock_origin_ns"], 1, (1 << 64) - 1, "clock origin")
    integer(setup["setup_completed_ns"], 0, LIMIT_NS, "setup boundary")
    for key in ("draft_payload_bytes", "retained_target_payload_bytes", "transposed_weight_bytes"):
        integer(setup[key], 0, 128 * 1024**3, "payload bytes")


def validate_capture(records, reference, expect, hashes):
    require(4 <= len(records) <= 5500, "capture record bound")
    setup, closed = records[0], records[-1]
    validate_setup(setup, expect, hashes)
    validate_reference(reference, setup)
    run_count = setup["warmups"] + setup["samples"]
    forwards = setup["expected_forwards_per_run"]
    require(len(records) == 2 + run_count * (forwards + 1), "missing/extra raw records")
    index, dispatches, prior_end, request_metrics, events = 1, 0, setup["setup_completed_ns"], [], []
    for run in range(run_count):
        warmup = run < setup["warmups"]
        rows = records[index:index + forwards]
        item = records[index + forwards]
        fields(item, RUN, "run record")
        for key in ("run", "first_token_ns", "terminal_ns", "kv_tokens_processed", "steps", "completed_dispatches", "free_pages", "retained_pages", "cached_pages", "quarantined_pages"):
            integer(item[key], 0, LIMIT_NS, "run scalar")
        token_list(item["generated_tokens"], "generated tokens")
        token_list(item["expected_tokens"], "expected tokens")
        require(type(item["generated_utf8_bytes"]) is list and len(item["generated_utf8_bytes"]) <= 16384, "generated byte extent")
        for byte in item["generated_utf8_bytes"]:
            integer(byte, 0, 255, "generated byte")
        require(item["schema"] == PREFIX + "RunV1" and item["authority"] == "none"
                and item["performance_qualified"] is False and item["run"] == run
                and item["warmup"] is warmup and item["reference_passed"] is True, "run result")
        arrival = integer(item["run_start_ns"], prior_end, LIMIT_NS, "run start")
        previous = arrival
        emitted = []
        for ordinal, (row, wanted) in enumerate(zip(rows, reference["steps"])):
            fields(row, STEP, "step record")
            for key in ("run", "ordinal", "selected_row", "choice", "expected_choice", "cache_tokens", "completed_dispatches"):
                integer(row[key], 0, 1310720, "step scalar")
            token_list(row["inputs"], "actual inputs")
            token_list(row["positions"], "actual positions")
            require(row["schema"] == PREFIX + "StepV1" and row["authority"] == "none"
                    and row["run"] == run and row["warmup"] is warmup and row["ordinal"] == ordinal
                    and row["reference_passed"] is True, "step sequence/failure")
            for key in ("inputs", "positions", "selected_row", "choice", "cache_tokens"):
                require(row[key] == wanted[key], "raw step differs from frozen reference")
            require(row["expected_choice"] == wanted["choice"], "expected choice substitution")
            reserve = integer(row["reserve_start_ns"], previous, LIMIT_NS, "reserve start")
            start = integer(row["forward_start_ns"], reserve, LIMIT_NS, "forward start")
            end = integer(row["forward_complete_ns"], start + 1, LIMIT_NS, "forward completion")
            commit = integer(row["commit_complete_ns"], end, LIMIT_NS, "checked commit")
            dispatches += PACKETS
            require(row["completed_dispatches"] == dispatches, "cumulative dispatch count")
            if row["cache_tokens"] >= 5:
                require(type(row["output_index"]) is int and row["output_index"] == len(emitted), "emitted output order")
                emitted.append(commit)
            else:
                require(row["output_index"] is None, "prefill is not an emitted output")
            phase = "prefill" if row["output_index"] is None else "forward"
            events.append({"id": f"run{run}.step{ordinal}", "label": ("warmup_" if warmup else "measured_") + phase,
                           "lane": "controller_forward", "clock": "controller-raw", "run": run,
                           "token": row["output_index"] if row["output_index"] is not None else -1, "begin_tick": start, "end_tick": end,
                           "begin_ns": start, "end_ns": end, "max_error_ns": 0})
            previous = commit
        require(len(emitted) == setup["new_tokens"] and arrival < emitted[0] < emitted[-1], "output timing order")
        require(item["first_token_ns"] == emitted[0] and item["terminal_ns"] == emitted[-1], "run/step timing disagreement")
        require(item["generated_tokens"] == item["expected_tokens"] == reference["generated_tokens"]
                and item["generated_utf8_bytes"] == reference["generated_utf8_bytes"], "token or decoded-byte parity")
        require(item["kv_tokens_processed"] == setup["new_tokens"] + 4 and item["steps"] == forwards
                and item["completed_dispatches"] == forwards * PACKETS, "run work count")
        require(item["free_pages"] == setup["physical_pages"]
                and all(item[key] == 0 for key in ("retained_pages", "cached_pages", "quarantined_pages")), "pool not returned")
        retired = integer(item["retired_ns"], emitted[-1], LIMIT_NS, "retirement")
        gaps = [b - a for a, b in zip(emitted, emitted[1:])]
        require(all(gap > 0 for gap in gaps), "nonpositive committed-output gap")
        if not warmup:
            duration = emitted[-1] - arrival
            decode = emitted[-1] - emitted[0]
            request_metrics.append({"run": run, "output_tokens": len(emitted), "post_first_tokens": len(emitted)-1,
                "arrival_ns": arrival, "first_token_ns": emitted[0], "terminal_ns": emitted[-1],
                "workload_ns": duration, "decode_ns": decode, "ttft_ns": emitted[0]-arrival,
                "tpot_floor_ns": decode // (len(emitted)-1), "tpot_mean_ns": decode / (len(emitted)-1),
                "itl_ns": gaps, "output_tokens_per_second": len(emitted)*1e9/duration,
                "post_first_decode_tokens_per_second": (len(emitted)-1)*1e9/decode})
        prior_end = retired
        index += forwards + 1
    fields(closed, CLOSED, "closed record")
    for key in ("completed_runs", "worker_pid", "completed_batches"):
        integer(closed[key], 0, (1 << 31) - 1, "closed scalar")
    require(type(closed["rank_dispatch_counts"]) is list, "closed count roster")
    for count in closed["rank_dispatch_counts"]:
        integer(count, 0, 131072, "closed dispatch count")
    require(closed["schema"] == PREFIX + "ClosedV1" and closed["authority"] == "none"
            and closed["performance_qualified"] is False and closed["execution_completed"] is True
            and closed["all_workers_exited"] is True and closed["error"] is None and closed["close_error"] is None,
            "failed execution/teardown")
    require(closed["completed_runs"] == run_count and closed["worker_pid"] == setup["worker_pid"]
            and closed["rank_dispatch_counts"] == [dispatches]
            and closed["completed_batches"] == run_count * forwards, "closed identity/count")
    start = integer(closed["teardown_start_ns"], prior_end, LIMIT_NS, "teardown start")
    end = integer(closed["teardown_end_ns"], start, LIMIT_NS, "teardown end")
    return setup, request_metrics, events, {"setup_ns": setup["setup_completed_ns"], "teardown_ns": end-start, "whole_controller_ns": end}


def load_manifest(manifest, base):
    fields(manifest, {"schema", "authority", "label", "workload_kind", "files", "expect", "trace"}, "report manifest")
    require(manifest["schema"] == "FerricDraftDecodeReportManifestV1" and manifest["authority"] == "none"
            and manifest["workload_kind"] == "full-model-draft06b-greedy", "fixture is not a model decode capture")
    CHECK.require(type(manifest["label"]) is str and 0 < len(manifest["label"]) <= 64, "variant label")
    fields(manifest["files"], FILES, "provenance files")
    identity_only = manifest["files"]["handoff"] == {"mode": "manifest-identity"}
    loaded = {key: pinned(pin, base, 256 * 1024 * 1024) for key, pin in manifest["files"].items()
              if not (key == "handoff" and identity_only)}
    hashes = {key: digest(raw) for key, (_, raw) in loaded.items()}
    binding = handoff_binding(loaded["artifact_manifest"][1], loaded["hsaco"][1],
                              None if identity_only else loaded["handoff"][1])
    hashes["handoff"] = binding["sha256"]
    raw = loaded["capture"][1]
    require(len(raw) <= 16 * 1024 * 1024, "capture bound")
    records = [CHECK.json_value(line) for line in raw.splitlines()]
    reference = CHECK.json_value(loaded["reference"][1])
    setup, requests, events, lifecycle = validate_capture(records, reference, manifest["expect"], hashes)
    window = requests[-1]["terminal_ns"] - requests[0]["arrival_ns"]
    outputs = sum(row["output_tokens"] for row in requests)
    post_first = sum(row["post_first_tokens"] for row in requests)
    decode_ns = sum(row["decode_ns"] for row in requests)
    metrics = {"measured_requests": len(requests), "excluded_warmups": setup["warmups"],
        "output_tokens": outputs, "aggregate_window_ns": window, "aggregate_output_tokens_per_second": outputs*1e9/window,
        "post_first_tokens": post_first, "summed_post_first_decode_ns": decode_ns,
        "pooled_post_first_decode_tokens_per_second": post_first*1e9/decode_ns,
        "per_request_output_rate": statistics([row["output_tokens_per_second"] for row in requests]),
        "per_request_decode_rate": statistics([row["post_first_decode_tokens_per_second"] for row in requests]),
        "ttft_ns": percentiles([row["ttft_ns"] for row in requests]),
        "r33_tpot_floor_ns": percentiles([row["tpot_floor_ns"] for row in requests])}
    equivalence = {key: setup[key] for key in ("model", "model_revision", "identity", "prompt_tokens", "new_tokens", "context_tokens", "physical_pages", "head_precision", "tensor_parallel", "prefix_cache", "eos_stopping", "warmups", "samples", "clock")}
    equivalence.update(expected_output_sha256=digest(canonical({key: reference[key] for key in ("generated_tokens", "generated_utf8_bytes")})),
                       environment_sha256=hashes["environment"])
    trace = None
    if manifest["trace"] is not None:
        fields(manifest["trace"], {"record", "collector", "clock_evidence"}, "trace evidence pins")
        _, trace_raw = pinned(manifest["trace"]["record"], base, 4 * 1024 * 1024)
        _, collector_raw = pinned(manifest["trace"]["collector"], base)
        _, clock_raw = pinned(manifest["trace"]["clock_evidence"], base)
        trace_value, clock_value = CHECK.json_value(trace_raw), CHECK.json_value(clock_raw)
        fields(clock_value, {"schema", "capture_sha256", "artifact_sha256", "clocks"}, "clock evidence")
        require(clock_value == {"schema": "FerricDecodeClockEvidenceV1", "capture_sha256": hashes["capture"],
                "artifact_sha256": hashes["hsaco"], "clocks": trace_value["clocks"]}, "clock evidence join")
        require(trace_value["collector"]["source_sha256"] == digest(collector_raw)
                and trace_value["collector"]["clock_evidence_sha256"] == digest(clock_raw), "trace producer/evidence identity")
        trace = validate_trace(trace_value, hashes["capture"], hashes["hsaco"],
                               setup["warmups"] + setup["samples"], setup["new_tokens"])
        emitted_steps = {(row["run"], row["output_index"]): row for row in records
                         if row["schema"] == PREFIX + "StepV1" and row["output_index"] is not None}
        for event in trace["events"]:
            step = emitted_steps[(event["run"], event["token"])]
            require(step["reserve_start_ns"] <= event["begin_ns"] <= event["end_ns"] <= step["commit_complete_ns"],
                    "trace event outside its actual token host envelope")
        hashes["trace"] = digest(trace_raw)
    report = {"schema": "FerricDraftDecodeReportV1", "authority": "none", "status": "observations-validated",
        "label": manifest["label"], "workload_kind": manifest["workload_kind"], "equivalence": equivalence,
        "runtime_options": {key: setup[key] for key in ("projection", "prefill", "runtime_cache_admission", "runtime_operational")},
        "sampling_class": setup["sampling_class"], "provenance_sha256": hashes, "metrics": metrics,
        "handoff_provenance": binding,
        "requests": requests, "lifecycle": lifecycle, "goal": goal_metadata(), "trace_overhead": "unmeasured",
        "gpu_overlap_measured": False, "performance_qualified": False,
        "nonclaims": ["Hash/record checks do not authenticate collector truth, hardware, model correctness or M1 qualification",
                      "Draft06B is not the Qwen3-8B target; 700 tok/s target comparison is not applicable",
                      "RAW host intervals include IPC, queue waits, checked KV commit and inter-step observation emission",
                      "Fresh uncached sequence per run; warm weights/worker do not imply steady-state serving",
                      "Small decode lengths/repetitions do not establish stable tails, sustained throughput or GPU overlap"]}
    host_trace = {"events": events, "domains": [{"clock": "controller-raw", "kind": "controller-monotonic-raw"}],
                  "cross_domain_overlap_computed": False}
    return report, host_trace, trace
