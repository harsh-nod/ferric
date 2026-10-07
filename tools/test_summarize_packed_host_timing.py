import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "summarize_packed_host_timing", Path(__file__).with_name("summarize_packed_host_timing.py"))
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)


def row(batch, category, label, count, elapsed, maximum, packets=0, rank=0):
    return dict(batch=batch, phase="batch", category=category, label=label, rank=rank,
                count=count, failed=0, elapsed_ns=elapsed, max_ns=maximum,
                request_payload_bytes=0, response_payload_bytes=0, dispatches=packets)


def fixture(packed=True):
    mode = "packed-gate-up-u32-r2" if packed else "baseline"
    selection = dict(schema="FerricPackedGateUpKvLiveSelectionR2", actual_mode=mode, requested_mode=mode,
                     additional_weight_bytes=7247757312, activation_scratch_bytes=8192,
                     loaded_image_count=10, selected_rows=1, selected_published_rows=1, roles=[4, 5],
                     extra_packets_per_selected_forward=36 if packed else 0,
                     ffn_producer_packets_per_selected_layer=6 if packed else 5)
    identity = dict(live_profile=report.PROFILE, host_timing_schema=report.HEADER["schema"],
                    diagnostic_max_model_batches=256, packed_gate_up=selection)
    setup = dict(identity, schema="FerricQwen3TpBatchSetupV2", tensor_parallel=1, context_tokens=8192,
                 prefill_chunk=16, prefix_cache=False, head_precision="fp32-v8", max_batches=135)
    closed = dict(identity, schema="FerricQwen3TpBatchClosedV2", execution_completed=True,
                  all_workers_exited=True, rank_dispatch_counts=[92283 if packed else 87711])
    rows = []
    for batch in range(1, 136):
        groups, packets = (72, 612) if batch <= 8 else (11, 688 if packed else 652)
        rows.extend([
            row(batch, "span", "batch", 1, 10000, 10000, rank=None),
            row(batch, "span", "ordered_dispatch_pack", groups, 1000, 100),
            row(batch, "worker_reported_elapsed", report.ORDERED, groups, 2000, 200, packets),
            row(batch, "ipc_roundtrip", report.ORDERED, groups, 3000, 300, packets),
            row(batch, "span", "ipc_response_wait", groups, 2500, 250),
        ])
        if batch <= 8:
            count = 1 if batch < 8 else 4
            rows.append(row(batch, "ipc_roundtrip", "dispatch", count, 500, 500, count))
    return dict(report.HEADER, controller_pid=123, setup=setup, closed=closed, records=rows)


def transcript(value):
    def event(kind, **fields):
        return dict(schema="FerricQwen3TpLiveEventV1", authority="none", event=kind, **fields)
    request = dict(request_id=1, name="diagnostic")
    prompt = list(range(128))
    events = [value["setup"], event("ready"), event("queued", **request, prompt_tokens=128),
              event("admission", **request, prompt_tokens=prompt, prompt_token_count=128,
                    cached_tokens=0, cached_pages=0)]
    decode_packets = 688 if value["setup"]["packed_gate_up"]["actual_mode"] != "baseline" else 652
    for batch in range(1, 136):
        output = int(batch >= 8)
        if output:
            events.append(event("token", **request, index=batch - 8, token=batch - 8,
                                completed_ns=batch, finished=batch == 135))
        events.append(event("batch", batch_id=batch, tick=batch - 1, rows=16 if batch <= 8 else 1,
                            outputs=output, output_head_rows=output, completed_ns=batch,
                            rank_dispatch_counts=[613 if batch < 8 else 616 if batch == 8 else decode_packets]))
    events.extend([event("request", **request, admitted=True, state="Completed", cancelled_ns=None,
                         prompt_tokens=prompt, prompt_token_count=128, generated_tokens=list(range(128)),
                         output_timestamps_ns=list(range(8, 136)), cached_prefix_tokens=0),
                   event("draining", reason="command"), event("stopped", reason="drained", batches=135),
                   value["closed"]])
    return events


class PackedHostTimingTests(unittest.TestCase):
    def test_both_modes_have_exact_schedule_and_nonadditive_partition(self):
        for selected, dispatches in ((False, 87711), (True, 92283)):
            value = fixture(selected)
            result = report.analyze(value, transcript(value))
            self.assertEqual(result["dispatches"], dispatches)
            self.assertEqual(result["ordered_groups"], 1973)
            self.assertTrue(result["transcript_bound"])
            self.assertTrue(result["workload_verified"])
            self.assertFalse(result["gpu_time_measured"])
            self.assertFalse(result["performance_qualified"])
            self.assertFalse(result["numerical_correctness_verified"])
            for part in result["summaries"].values():
                self.assertEqual(sum(value for key, value in part.items()
                                     if key not in ("batches", "batch_wall_ns")), part["batch_wall_ns"])
            self.assertEqual(result["summaries"]["decode"]["batch_wall_ns"], 1270000)

    def test_outside_batch_time_does_not_enter_partition(self):
        value = fixture()
        value["records"].append(row(None, "span", "setup", 1, 99999999, 99999999, rank=None))
        self.assertEqual(report.analyze(value)["summaries"]["prefill"]["batch_wall_ns"], 80000)

    def test_header_schema_and_numeric_types_fail_closed(self):
        for key, replacement in (("incomplete", True), ("incomplete", 0), ("active_records", 1),
                                  ("controller_pid", True), ("failure", "failed"),
                                  ("schema", "FerricHostTimingV1")):
            value = fixture()
            value[key] = replacement
            with self.subTest(key=key), self.assertRaises(ValueError):
                report.analyze(value)

    def test_old_profile_mode_drift_and_false_completion_fail(self):
        for section, key, replacement in (("setup", "live_profile", "old"),
                                          ("closed", "execution_completed", False),
                                          ("closed", "rank_dispatch_counts", [87711]),
                                          ("setup", "max_batches", 257),
                                          ("setup", "tensor_parallel", True)):
            value = copy.deepcopy(fixture())
            value[section][key] = replacement
            with self.subTest(key=key), self.assertRaises(ValueError):
                report.analyze(value)
        value = copy.deepcopy(fixture())
        value["closed"]["packed_gate_up"] = dict(value["closed"]["packed_gate_up"], actual_mode="baseline")
        with self.assertRaises(ValueError):
            report.analyze(value)

    def test_missing_duplicate_and_failed_rows_fail(self):
        for kind in range(4):
            value = fixture()
            if kind == 0:
                value["records"].pop(2)
            elif kind == 1:
                value["records"].append(copy.deepcopy(value["records"][0]))
            elif kind == 2:
                value["records"][0]["failed"] = 1
            else:
                value["records"][0]["unexpected"] = 1
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                report.analyze(value)

    def test_counter_packet_rank_and_phase_corruption_fail(self):
        for key, replacement in (("dispatches", 613), ("count", 71), ("batch", 136),
                                  ("rank", 1), ("count", True), ("phase", "wrong"),
                                  ("elapsed_ns", 1 << 64), ("elapsed_ns", 1.0)):
            value = fixture()
            value["records"][2][key] = replacement
            with self.subTest(key=key), self.assertRaises(ValueError):
                report.analyze(value)

    def test_nested_worker_duration_must_fit_covering_roundtrip(self):
        value = fixture()
        value["records"][2].update(elapsed_ns=4000, max_ns=400)
        with self.assertRaises(ValueError):
            report.analyze(value)

    def test_packing_and_transport_must_fit_batch(self):
        value = fixture()
        value["records"][0].update(elapsed_ns=3500, max_ns=3500)
        with self.assertRaises(ValueError):
            report.analyze(value)

    def test_transcript_requires_exact_single_setup_and_close(self):
        value = fixture()
        for transcript in ([value["closed"], value["setup"]],
                           [value["setup"], value["setup"], value["closed"]],
                           [value["setup"], dict(value["closed"], execution_completed=False)]):
            with self.assertRaises(ValueError):
                report.analyze(value, transcript)

    def test_113_token_prompt_with_same_eight_batches_is_rejected(self):
        value = fixture()
        events = transcript(value)
        for event in events:
            if event.get("event") == "queued":
                event["prompt_tokens"] = 113
            elif event.get("event") in ("admission", "request"):
                event["prompt_tokens"] = list(range(113))
                event["prompt_token_count"] = 113
        with self.assertRaises(ValueError):
            report.analyze(value, events)

    def test_multiple_request_finals_are_rejected(self):
        value = fixture()
        events = transcript(value)
        final = next(event for event in events if event.get("event") == "request")
        events.insert(-3, copy.deepcopy(final))
        with self.assertRaises(ValueError):
            report.analyze(value, events)

    def test_changed_batch_and_token_sequence_are_rejected(self):
        value = fixture()
        for event_kind, key, changed in (("batch", "batch_id", 2), ("batch", "rows", 1),
                                          ("batch", "rank_dispatch_counts", [612]),
                                          ("token", "index", 1), ("token", "finished", True),
                                          ("token", "request_id", 2)):
            events = transcript(value)
            event = next(event for event in events if event.get("event") == event_kind)
            event[key] = changed
            with self.subTest(key=key), self.assertRaises(ValueError):
                report.analyze(value, events)

    def test_sidecar_only_does_not_claim_workload_or_correctness_verification(self):
        result = report.analyze(fixture())
        self.assertFalse(result["workload_verified"])
        self.assertFalse(result["numerical_correctness_verified"])
        self.assertIsNone(result["workload"])

    def test_invalid_json_and_duplicate_keys_fail(self):
        for raw in ('{"x":1,"x":2}', '{"x":NaN}', '{"x":Infinity}', '{'):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                report.decode(raw)


if __name__ == "__main__":
    unittest.main()
