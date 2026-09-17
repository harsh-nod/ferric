"""Synthetic CPU-only records. Never hardware or performance evidence."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

from common import canonical, digest
import draft
import report


def fixture(prefill="full", tokens=4, warmups=1, samples=2):
    hashes = {key: digest(("synthetic-CPU-only-" + key).encode()) for key in draft.FILES}
    identity = {key: digest(key.encode()) for key in draft.IDENTITY}
    forwards = tokens + (4 if prefill == "tokenwise" else 0)
    pages = (tokens + 19) // 16
    setup = {
        "schema": draft.PREFIX + "SetupV1", "authority": "none", "performance_qualified": False,
        "clock": "CLOCK_MONOTONIC_RAW", "clock_origin_ns": 100000, "setup_completed_ns": 100,
        "timing_boundary": "host-reserve-through-checked-completion-and-KV-commit",
        "timing_exclusions": "model/artifact/worker setup; tokenization; retirement; teardown",
        "timing_includes": "controller/IPC/queue/waits; per-step observation emission between token completions",
        "device_timing": False, "device_overlap_claim": False,
        "sampling_class": "benchmark-sized-unqualified" if (warmups, samples) == (10, 30) else "diagnostic-only",
        "warmups": warmups, "samples": samples, "new_tokens": tokens,
        "prompt": "The capital of France is", "prompt_tokens": [11, 12, 13, 14, 15],
        "initial_kv_tokens": 0, "prefill_tokens": 5, "context_tokens": pages * 16, "physical_pages": pages,
        "model": draft.MODEL, "model_revision": draft.REVISION, "model_role": "Draft06B", "identity": identity,
        "projection": "baseline", "prefill": prefill, "head_precision": "fp32-v10",
        "collective": "draft-device-tp1-v10", "attention": "baseline", "tensor_parallel": 1, "row_capacity": 32,
        "prefix_cache": False, "eos_stopping": False, "reference_sha256": hashes["reference"],
        "reference_producer": "synthetic CPU test, not a GPU observation",
        "controller_sha256": hashes["controller"], "worker_sha256": hashes["worker"],
        "running_worker_sha256": hashes["worker"], "worker_pid": 123, "device_unique_id": 987654321,
        "artifact_hsaco_id": hashes["hsaco"], "artifact_manifest_id": hashes["artifact_manifest"],
        "artifact_handoff_id": hashes["handoff"], "runtime_cache_admission": True, "runtime_operational": True,
        "runtime_sequences": False, "runtime_ordered_batches": False, "runtime_rollover": False,
        "expected_forwards_per_run": forwards, "expected_dispatches_per_forward": 480,
        "expected_total_dispatches": (warmups + samples) * forwards * 480, "no_rollover_packet_budget": 131072,
        "reused_worker_weights_and_pool": True, "retry_policy": "none-stop-on-first-error",
        "draft_payload_bytes": 100, "retained_target_payload_bytes": 200, "transposed_weight_bytes": 300,
    }
    reference = {key: copy.deepcopy(setup[key]) for key in ("model", "model_revision", "identity", "head_precision", "prefill", "prompt_tokens")}
    reference.update(schema="FerricDraftDecodeProfileReferenceV1", producer=setup["reference_producer"],
                     steps=[], generated_tokens=[], generated_utf8_bytes=list(b"synthetic CPU test"))
    previous = None
    for ordinal in range(forwards):
        inputs, positions, row = draft.schedule(prefill, setup["prompt_tokens"], ordinal, previous)
        choice = 100 + ordinal
        reference["steps"].append(dict(inputs=inputs, positions=positions, selected_row=row,
                                       choice=choice, cache_tokens=positions[-1] + 1))
        if positions[-1] + 1 >= 5:
            reference["generated_tokens"].append(choice)
        previous = choice
    setup["reference_sha256"] = hashes["reference"] = digest(canonical(reference))
    records = [setup]
    dispatches = 0
    for run in range(warmups + samples):
        start = 1000 + run * (forwards * 100 + 1000)
        emitted = []
        for ordinal, expected in enumerate(reference["steps"]):
            reserve = start + ordinal * 100
            commit = reserve + 80
            output = len(emitted) if expected["cache_tokens"] >= 5 else None
            if output is not None:
                emitted.append(commit)
            dispatches += 480
            records.append(dict(schema=draft.PREFIX + "StepV1", authority="none", run=run,
                warmup=run < warmups, ordinal=ordinal, output_index=output, **copy.deepcopy(expected),
                expected_choice=expected["choice"], completed_dispatches=dispatches, reserve_start_ns=reserve,
                forward_start_ns=reserve + 10, forward_complete_ns=reserve + 70, commit_complete_ns=commit,
                reference_passed=True))
        records.append(dict(schema=draft.PREFIX + "RunV1", authority="none", performance_qualified=False,
            run=run, warmup=run < warmups, run_start_ns=start, first_token_ns=emitted[0], terminal_ns=emitted[-1],
            retired_ns=emitted[-1] + 100, generated_tokens=reference["generated_tokens"].copy(),
            expected_tokens=reference["generated_tokens"].copy(), generated_utf8_bytes=reference["generated_utf8_bytes"].copy(),
            reference_passed=True, kv_tokens_processed=tokens + 4, steps=forwards, completed_dispatches=forwards * 480,
            free_pages=pages, retained_pages=0, cached_pages=0, quarantined_pages=0))
    end = records[-1]["retired_ns"] + 100
    records.append(dict(schema=draft.PREFIX + "ClosedV1", authority="none", performance_qualified=False,
        execution_completed=True, error=None, completed_runs=warmups + samples, all_workers_exited=True,
        close_error=None, teardown_start_ns=end, teardown_end_ns=end + 50, worker_pid=123,
        rank_dispatch_counts=[dispatches], completed_batches=(warmups + samples) * forwards))
    expect = {key: copy.deepcopy(setup[key]) for key in draft.EXPECT}
    return records, reference, expect, hashes


def materialize(base, values=None):
    records, reference, expect, _ = copy.deepcopy(values or fixture())
    payloads = {key: ("synthetic-CPU-only-" + key).encode() for key in draft.FILES}
    payloads["reference"] = canonical(reference)
    # Component identity fixture only, not an admitted native artifact.
    observation = dict(schema="EngineeringHsacoObservationV1", namespace="fe2o3-engineering-v1",
        authority="none", artifact="observation.hsaco", crate_name="ferric_qwen3_draft_batch32_kernels_device_v10",
        target="gfx950:xnack-", code_object_version=6,
        compiler_handoff=dict(sha256=digest(payloads["handoff"]), byte_len=len(payloads["handoff"])),
        tools={}, providers=[], options={}, execution={},
        hsaco=dict(identity=dict(sha256=digest(payloads["hsaco"]), byte_len=len(payloads["hsaco"])),
                   canonical_descriptor_sha256="a" * 64, kernel_names=[]),
        grants=dict(publication=False, load=False, launch=False))
    payloads["artifact_manifest"] = json.dumps(observation, separators=(",", ":")).encode() + b"\n"
    setup = records[0]
    for key, pin in (("reference_sha256", "reference"), ("controller_sha256", "controller"),
                     ("worker_sha256", "worker"), ("running_worker_sha256", "worker"),
                     ("artifact_hsaco_id", "hsaco"), ("artifact_manifest_id", "artifact_manifest"),
                     ("artifact_handoff_id", "handoff")):
        setup[key] = digest(payloads[pin])
    payloads["capture"] = b"\n".join(canonical(row) for row in records) + b"\n"
    pins = {}
    for key, raw in payloads.items():
        path = base / (key + ".fixture")
        path.write_bytes(raw)
        pins[key] = {"path": path.name, "sha256": digest(raw)}
    definition = dict(schema="FerricDraftDecodeReportManifestV1", authority="none", label="SYNTHETIC CPU TEST ONLY",
                      workload_kind="full-model-draft06b-greedy", files=pins, expect=expect, trace=None)
    path = base / "manifest.json"
    path.write_bytes(canonical(definition))
    return path, definition


class DraftCaptureTests(unittest.TestCase):
    def test_full_reference_and_r33_request_arithmetic(self):
        setup, requests, events, life = draft.validate_capture(*fixture())
        self.assertEqual([row["run"] for row in requests], [1, 2])
        self.assertEqual([row["ttft_ns"] for row in requests], [80, 80])
        self.assertEqual([row["tpot_floor_ns"] for row in requests], [100, 100])
        self.assertEqual(requests[0]["itl_ns"], [100, 100, 100])
        self.assertEqual(len(events), 12)
        self.assertEqual(life["teardown_ns"], 50)
        self.assertEqual(setup["initial_kv_tokens"], 0)

    def test_tokenwise_prefill_is_not_five_output_tokens(self):
        _, requests, events, _ = draft.validate_capture(*fixture(prefill="tokenwise"))
        self.assertEqual(requests[0]["output_tokens"], 4)
        self.assertEqual(requests[0]["ttft_ns"], 480)
        self.assertEqual(sum(row["token"] == -1 for row in events), 12)

    def test_setup_mutants_reject_even_with_matching_operator_expectations(self):
        mutants = {"clock": "GPU", "device_timing": True, "device_overlap_claim": True,
            "model": "Qwen/Qwen3-8B", "head_precision": "bf16", "new_tokens": True,
            "initial_kv_tokens": 5, "runtime_rollover": True, "expected_forwards_per_run": 3,
            "expected_total_dispatches": 1, "no_rollover_packet_budget": 999999,
            "sampling_class": "qualified", "projection": "unknown", "row_capacity": 16,
            "timing_includes": "GPU-only", "retry_policy": "retry", "performance_qualified": True,
            "reference_sha256": "f" * 64, "artifact_handoff_id": "e" * 64}
        for key, value in mutants.items():
            with self.subTest(key=key):
                rows, ref, expect, hashes = fixture()
                rows[0][key] = value
                if key in expect:
                    expect[key] = value
                with self.assertRaises(ValueError):
                    draft.validate_capture(rows, ref, expect, hashes)

    def test_dispatch_budget_and_benchmark_sized_label(self):
        setup, requests, _, _ = draft.validate_capture(*fixture(tokens=4, warmups=10, samples=30))
        self.assertEqual(setup["sampling_class"], "benchmark-sized-unqualified")
        self.assertEqual(len(requests), 30)
        with self.assertRaises(ValueError):
            draft.validate_capture(*fixture(tokens=8, warmups=10, samples=30))

    def test_all_reference_steps_are_required(self):
        for key, value in (("choice", 333), ("inputs", [22]), ("positions", [99]), ("selected_row", 2), ("cache_tokens", 44)):
            with self.subTest(key=key):
                rows, ref, expect, hashes = fixture()
                ref["steps"][1][key] = value
                with self.assertRaises(ValueError):
                    draft.validate_capture(rows, ref, expect, hashes)

    def test_old_reference_is_admitted_only_for_two_tokens(self):
        rows, ref, expect, hashes = fixture(tokens=2)
        ref["schema"] = "FerricDraftPagedCanaryReferenceV10"
        draft.validate_capture(rows, ref, expect, hashes)
        rows, ref, expect, hashes = fixture(tokens=4)
        ref["schema"] = "FerricDraftPagedCanaryReferenceV10"
        with self.assertRaises(ValueError):
            draft.validate_capture(rows, ref, expect, hashes)

    def test_step_mutants_fail_closed(self):
        mutations = {"choice": 444, "expected_choice": 444, "cache_tokens": 6, "selected_row": 0,
            "output_index": None, "completed_dispatches": 0, "warmup": False, "ordinal": 1,
            "forward_complete_ns": 1000, "reserve_start_ns": 0, "reference_passed": False,
            "schema": "fixture", "run": True, "commit_complete_ns": None}
        for key, value in mutations.items():
            with self.subTest(key=key):
                rows, ref, expect, hashes = fixture()
                rows[1][key] = value
                with self.assertRaises(ValueError):
                    draft.validate_capture(rows, ref, expect, hashes)

    def test_run_and_teardown_mutants_fail_closed(self):
        mutations = {"first_token_ns": 1, "terminal_ns": 2, "retired_ns": 0,
            "reference_passed": False, "generated_tokens": [1, 2, 3, 4], "generated_utf8_bytes": [65],
            "free_pages": 0, "retained_pages": 1, "cached_pages": 1, "quarantined_pages": 1,
            "completed_dispatches": 1, "steps": 3, "warmup": False}
        for key, value in mutations.items():
            with self.subTest(run_key=key):
                rows, ref, expect, hashes = fixture()
                rows[5][key] = value
                with self.assertRaises(ValueError):
                    draft.validate_capture(rows, ref, expect, hashes)
        for key, value in {"execution_completed": False, "all_workers_exited": False, "error": "bad",
            "close_error": "bad", "completed_runs": 2, "worker_pid": 222, "rank_dispatch_counts": [1],
            "completed_batches": 1, "teardown_start_ns": None, "teardown_end_ns": None}.items():
            with self.subTest(closed_key=key):
                rows, ref, expect, hashes = fixture()
                rows[-1][key] = value
                with self.assertRaises(ValueError):
                    draft.validate_capture(rows, ref, expect, hashes)

    def test_reordered_missing_and_extra_records_reject(self):
        for mutation in (lambda r: r.pop(1), lambda r: r.insert(1, r[1]),
                         lambda r: r[1].update(unrecognized=True), lambda r: r.reverse()):
            rows, ref, expect, hashes = fixture()
            mutation(rows)
            with self.assertRaises(ValueError):
                draft.validate_capture(rows, ref, expect, hashes)


class DraftReportTests(unittest.TestCase):
    def test_validated_report_and_svg_are_sanitized_and_do_not_claim_target(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            path, _ = materialize(base)
            result = report.draft_report(path, base / "report")
            self.assertEqual(result["metrics"]["measured_requests"], 2)
            self.assertEqual(result["metrics"]["excluded_warmups"], 1)
            self.assertEqual(result["metrics"]["output_tokens"], 8)
            self.assertEqual(result["metrics"]["aggregate_window_ns"], 1780)
            self.assertEqual(result["metrics"]["pooled_post_first_decode_tokens_per_second"], 10000000)
            self.assertFalse(result["goal"]["applicable_to_this_draft06b_capture"])
            self.assertFalse(result["performance_qualified"])
            text = json.dumps(result)
            for private in ("worker_pid", "device_unique_id", "987654321", str(base), "clock_origin_ns"):
                self.assertNotIn(private, text)
            self.assertTrue((base / "report" / "host-timeline.svg").is_file())
            with self.assertRaises(FileExistsError):
                report.draft_report(path, base / "report")

    def test_correctness_fixture_and_artifact_drift_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            _, definition = materialize(base)
            other = copy.deepcopy(definition)
            other["workload_kind"] = "kproj-correctness-fixture"
            with self.assertRaises(ValueError):
                draft.load_manifest(other, base)
            (base / "worker.fixture").write_bytes(b"changed")
            with self.assertRaises(ValueError):
                draft.load_manifest(definition, base)

    def test_exact_same_workload_ablation_and_drift_rejection(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            one, two = base / "one", base / "two"
            one.mkdir()
            two.mkdir()
            path1, _ = materialize(one)
            values = fixture()
            values[0][0]["runtime_operational"] = values[2]["runtime_operational"] = False
            path2, _ = materialize(two, values)
            definition = dict(schema="FerricDecodeAblationManifestV1", authority="none", baseline="baseline",
                variants=[dict(name=name, manifest=dict(path=str(path.relative_to(base)), sha256=digest(path.read_bytes())))
                          for name, path in (("baseline", path1), ("operational off", path2))])
            manifest = base / "compare.json"
            manifest.write_bytes(canonical(definition))
            result = report.compare(manifest, base / "comparison")
            self.assertEqual(result["variants"][1]["decode_rate_ratio"], 1)
            self.assertEqual(set(result["variants"][1]["runtime_option_changes"]), {"runtime_operational"})
            path2, _ = materialize(two, fixture(tokens=2))
            definition["variants"][1]["manifest"]["sha256"] = digest(path2.read_bytes())
            manifest.write_bytes(canonical(definition))
            with self.assertRaises(ValueError):
                report.compare(manifest, base / "rejected")
            self.assertFalse((base / "rejected").exists())

    def test_analytical_bounds_are_explicit_and_separate(self):
        result = draft.goal_metadata()
        values = result["analytical_only"]
        self.assertEqual(values["assumed_streamed_weight_bytes_per_token"], 15136819200)
        self.assertAlmostEqual(values["optimistic_product_weight_streaming_tokens_per_second"], 528.5126217270271)
        self.assertAlmostEqual(values["installed_host_advertised_weight_streaming_tokens_per_second"], 6.810e12 / 15136819200)
        self.assertEqual(result["target_decode_tokens_per_second"], 700)

    def test_imported_trace_is_bound_to_actual_step_not_only_capture_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            _, definition = materialize(base)
            clocks = [dict(id="host", kind="controller-monotonic-raw", ticks_per_second=10**9,
                           counter_bits=64, unwrapped=True, max_drift_ppm=0, anchors=[])]
            clock = dict(schema="FerricDecodeClockEvidenceV1", capture_sha256=definition["files"]["capture"]["sha256"],
                         artifact_sha256=definition["files"]["hsaco"]["sha256"], clocks=clocks)
            clock_raw, collector_raw = canonical(clock), b"SYNTHETIC CPU TEST COLLECTOR"
            trace = dict(schema="FerricDecodeTraceV1", authority="none", capture_sha256=clock["capture_sha256"],
                artifact_sha256=clock["artifact_sha256"], clocks=clocks, dropped_events=0,
                collector=dict(kind="host-raw-spans", source_sha256=digest(collector_raw),
                               clock_evidence_sha256=digest(clock_raw), overhead="unmeasured"),
                events=[dict(id="step", label="synthetic", lane="host", clock="host", run=0, token=0,
                             begin_tick=1010, end_tick=1070)])
            pins = {}
            for key, raw in (("record", canonical(trace)), ("collector", collector_raw), ("clock_evidence", clock_raw)):
                path = base / (key + ".json")
                path.write_bytes(raw)
                pins[key] = dict(path=path.name, sha256=digest(raw))
            definition["trace"] = pins
            result, _, checked = draft.load_manifest(definition, base)
            self.assertIsNotNone(checked)
            self.assertFalse(result["gpu_overlap_measured"])
            trace["events"][0]["begin_tick"] = 999
            raw = canonical(trace)
            (base / "record.json").write_bytes(raw)
            pins["record"]["sha256"] = digest(raw)
            with self.assertRaisesRegex(ValueError, "host envelope"):
                draft.load_manifest(definition, base)

    def test_each_projection_label_is_actual_cli_vocabulary(self):
        for label in ("baseline", "wave", "mfma", "auto"):
            rows, reference, expect, hashes = fixture()
            rows[0]["projection"] = expect["projection"] = label
            draft.validate_capture(rows, reference, expect, hashes)


if __name__ == "__main__":
    unittest.main()
