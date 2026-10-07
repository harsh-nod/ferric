"""Synthetic host timeline tests; no GPU launch or numerical qualification."""

import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import live_runtime_profile as P


def encode(value):
    return (json.dumps(value, sort_keys=True, allow_nan=False) + "\n").encode()


def fixture():
    identity = dict(controller_sha256="a" * 64, worker_sha256="b" * 64, model_bundle_id="c" * 64,
                    target_model_id="d" * 64, session_id="e" * 64, layer_projection="c1-wave",
                    worker_pids=[123], device_unique_ids=[456])
    common = dict(P.FALSE_FLAGS, authority="none", live_profile=P.PROFILE, submission="ordered",
                  runtime_profiling=True, layer_projection="c1-wave")
    performance = dict(common, runtime_ordered_batches=True, dispatch_sequences=False,
                       runtime_cache_admission=True, runtime_operational=True, queue_rollover=True,
                       projection="mfma", attention="wave", argmax_mode="wave-v11")
    del performance["authority"]
    setup = dict(common, **identity, schema="FerricQwen3TpBatchSetupV2", tensor_parallel=1,
                 running_worker_sha256=[identity["worker_sha256"]], runtime_diagnostic_scope=P.SCOPE,
                 command_accounting=P.ACCOUNTING, prefix_cache=False, context_tokens=8192,
                 physical_pages=512, max_batches=10000, performance_profile=performance)
    closed = dict(common, schema="FerricQwen3TpBatchClosedV2", runtime_diagnostics_completed=True,
                  all_workers_exited=True, execution_completed=True, worker_pids=[123], rank_dispatch_counts=[1229])

    def event(kind, emitted, **data):
        return dict(schema=P.EVENT, authority="none", event=kind, emission_started_ns=emitted, **data)

    events = [setup, event("ready", 1, clock="monotonic_ns_since_live_start", context_tokens=8192,
                physical_pages=512, max_batches=10000, max_active_requests=32, max_pending_requests=32,
                eos_policy="fixed output count")]
    for request in (1, 2):
        events.append(event("queued", request * 10, request_id=request, arrival_ns=request * 5,
                            queued_ns=request * 10))
        events.append(event("admission", request * 10 + 1, request_id=request, arrival_ns=request * 5,
                            admitted_ns=request * 10 + 1, queue_wait_ns=request * 5 + 1, slot=request - 1,
                            generation=1, cached_tokens=0, cached_pages=0, prompt_tokens=[1, 2], prompt_token_count=2))
    events.append(event("batch", 201, batch_id=1, tick=0, started_ns=100, completed_ns=200,
                        rows=4, outputs=0, output_head_rows=0, rank_dispatch_counts=[613]))
    for request in (1, 2):
        events.append(event("token", 451 + request, request_id=request, slot=request - 1, generation=1,
                            index=0, token=42 + request, completed_ns=450, finished=True, decoded_bytes=[64 + request]))
    events.append(event("batch", 454, batch_id=2, tick=1, started_ns=300, completed_ns=450,
                        rows=2, outputs=2, output_head_rows=2, rank_dispatch_counts=[616]))
    for request in (1, 2):
        events.append(event("request", 455 + request, request_id=request, slot=request - 1, generation=1,
                            state="Completed", admitted=True, cancelled_ns=None, cached_prefix_tokens=0,
                            arrival_ns=request * 5, prompt_tokens=[1, 2], generated_tokens=[42 + request],
                            generated_utf8_bytes=[64 + request], output_timestamps_ns=[450], ttft_ns=450 - request * 5))
    events += [event("draining", 460, reason="command"), event("stopped", 461, reason="drained", batches=2), closed]
    records = []

    def row(batch, phase, category, label, rank=None, count=1, elapsed=1, **totals):
        record = dict.fromkeys(P.TOTALS, 0)
        record.update(batch=batch, phase=phase, category=category, label=label, rank=rank, count=count,
                      elapsed_ns=elapsed, max_ns=elapsed, **totals)
        records.append(record)
        return record

    for phase in sorted(P.NON_WORKLOAD | {"workload"}):
        row(None, phase, "span", phase, elapsed=500)
    row(None, "controller_batch", "span", "controller_batch", count=2, elapsed=250)["max_ns"] = 150
    for phase in ("runtime_snapshot_before", "runtime_snapshot_after"):
        row(None, phase, "ipc_send", "other", rank=0)
        row(None, phase, "ipc_roundtrip", "other", rank=0, elapsed=2)
    for batch, dispatches in ((1, 613), (2, 616)):
        row(batch, "batch", "span", "batch", elapsed=90)
        row(batch, "attention", "span", "attention", elapsed=70)
        row(batch, "attention", "span", "attention_gqa_math", elapsed=60)
        row(batch, "attention", "ipc_send", "dispatch_ordered_batch", rank=0, count=39, elapsed=10)
        row(batch, "attention", "ipc_roundtrip", "dispatch_ordered_batch", rank=0,
            count=39, elapsed=50, dispatches=dispatches)
    timing = dict(schema="FerricHostTimingV1", clock="controller-std-instant",
        measurement="host-wall-latency-not-gpu-duration", aggregation="overlapping-not-additive",
        payload_accounting="payload-only-excludes-wire-headers", record_limit=65536, incomplete=False,
        active_records=0, run_status="completed", failure=None, setup=copy.deepcopy(setup), closed=copy.deepcopy(closed),
        workload_sha256=None, controller_pid=100, records=records)
    before = dict.fromkeys(P.COUNTERS, 100)
    before["dispatches"] = 0
    after = dict(before, commands=179, dispatches=1229, command_ns=1100, dispatch_wait_ns=900)
    envelopes = []
    for ordinal, counters in enumerate((before, after)):
        envelope = dict(common, schema=P.ENVELOPE, phase=("before_workload", "after_workload")[ordinal],
                        measurement=P.SCOPE, command_accounting=P.ACCOUNTING)
        del envelope["submission"]
        envelope["snapshots"] = [dict(schema="FerricRuntimeDiagnosticSnapshotV1", authority="none",
            performance_qualified=False, scope="cumulative overlapping worker host-wall counters, not GPU timestamps",
            process_id=123, device_unique_id=456, rank=0, ordinal=ordinal, counters=counters)]
        envelopes.append(envelope)
    return events, timing, envelopes, identity


class LiveRuntimeProfileTests(unittest.TestCase):
    def test_request_batch_links_preserve_shared_batches_without_invented_gpu_or_exclusive_time(self):
        data = fixture()
        before = copy.deepcopy(data)
        report = P.profile(*data)
        self.assertEqual(data, before)
        self.assertIsNone(report["gpu_duration_ns"])
        self.assertFalse(report["performance_qualified"])
        self.assertFalse(report["numerical_qualified"])
        first, second = report["batches"]
        self.assertEqual(first["active_request_ids"], [1, 2])
        self.assertEqual(first["output_request_ids"], [])
        self.assertEqual(second["output_request_ids"], [1, 2])
        self.assertEqual(second["between_batch_host_gap_ns"], 100)
        self.assertEqual(second["host_interval_ns"], 150)
        self.assertIsNone(first["between_batch_host_gap_ns"])
        self.assertIsNone(second["gpu_duration_ns"])
        self.assertGreater(sum(r["elapsed_ns"] for r in first["host_observations"]), first["host_interval_ns"])
        self.assertNotIn("exclusive_host_ns", first)
        self.assertNotIn("phase", first)
        for request in report["requests"]:
            self.assertEqual(request["outputs"], [dict(index=0, completed_ns=450, batch_id=2)])
            self.assertNotIn("host_observations", request)
        self.assertEqual(report["worker_counter_delta"]["commands"], 79)
        self.assertEqual(report["worker_counter_delta"]["dispatch_wait_ns"], 800)

    def test_identity_scope_completion_and_worker_mutations_reject(self):
        mutations = [
            lambda e, t, s, i: e[0].update(controller_sha256="f" * 64),
            lambda e, t, s, i: e[0].update(running_worker_sha256=["f" * 64]),
            lambda e, t, s, i: e[0].update(runtime_diagnostic_scope="GPU timestamps"),
            lambda e, t, s, i: e[0]["performance_profile"].update(runtime_operational=False),
            lambda e, t, s, i: e[-1].update(all_workers_exited=False),
            lambda e, t, s, i: e[-1].update(execution_completed=False),
            lambda e, t, s, i: t.update(setup={}),
            lambda e, t, s, i: t.update(incomplete=True),
            lambda e, t, s, i: t.update(active_records=1),
            lambda e, t, s, i: t.update(aggregation="additive"),
            lambda e, t, s, i: s[1]["snapshots"][0].update(process_id=124),
            lambda e, t, s, i: s[1]["snapshots"][0].update(device_unique_id=457),
            lambda e, t, s, i: s[1]["snapshots"][0].update(rank=False),
            lambda e, t, s, i: s[1]["snapshots"][0].update(scope="GPU duration"),
            lambda e, t, s, i: s[1].update(snapshots=[]),
            lambda e, t, s, i: s[1].update(phase="before_workload"),
            lambda e, t, s, i: s.reverse(),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index):
                data = fixture()
                mutate(*data)
                with self.assertRaises(ValueError):
                    P.profile(*data)

    def test_counter_regressions_and_accounting_mismatches_reject(self):
        for key, value in (("read_ns", 99), ("commands", 178), ("commands", 180), ("dispatches", 1228),
                           ("reads", 101), ("read_bytes", 101), ("writes", 101), ("write_bytes", 101),
                           ("dispatch_wait_ns", True), ("command_ns", 1 << 64)):
            with self.subTest(key=key, value=value):
                data = fixture()
                data[2][1]["snapshots"][0]["counters"][key] = value
                with self.assertRaises(ValueError):
                    P.profile(*data)

    def test_per_batch_attribution_requires_complete_matching_timing_records(self):
        mutations = [
            lambda r: r.append(copy.deepcopy(r[-1])),
            lambda r: r.pop(),
            lambda r: r[-1].update(batch=3),
            lambda r: r[-1].update(failed=1),
            lambda r: r[-1].update(dispatches=615),
            lambda r: r[-1].update(phase="setup"),
            lambda r: r[-1].update(rank=None),
            lambda r: r[-1].update(count=1),
            lambda r: r[-1].update(elapsed_ns=1),
            lambda r: r[-2].update(elapsed_ns=51, max_ns=51),
            lambda r: r.__setitem__(slice(None), [x for x in r if x["label"] != "batch"]),
            lambda r: r.__setitem__(slice(None), [x for x in r if x["phase"] != "runtime_snapshot_before"]),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index):
                data = fixture()
                mutate(data[1]["records"])
                with self.assertRaises(ValueError):
                    P.profile(*data)

    def test_io_duration_requires_a_matching_operation(self):
        for key in ("read_ns", "write_ns"):
            with self.subTest(counter=key):
                data = fixture()
                data[2][1]["snapshots"][0]["counters"][key] += 1
                with self.assertRaisesRegex(ValueError, "I/O duration without operation"):
                    P.profile(*data)

    def test_physical_batch_span_is_bounded_by_its_live_interval(self):
        for batch_id, duration in ((1, 101), (2, 151)):
            with self.subTest(batch=batch_id):
                data = fixture()
                row = next(r for r in data[1]["records"] if r["label"] == "batch" and r["batch"] == batch_id)
                row.update(elapsed_ns=duration, max_ns=duration)
                with self.assertRaisesRegex(ValueError, "batch span exceeds live interval"):
                    P.profile(*data)
                row.update(elapsed_ns=duration - 1, max_ns=duration - 1)
                P.profile(*data)

    def test_lifecycle_clock_and_token_batch_mutations_reject(self):
        for kind, changes in (("admission", dict(queue_wait_ns=0)), ("token", dict(completed_ns=449)),
                              ("token", dict(index=1)), ("token", dict(slot=31)),
                              ("batch", dict(started_ns=202)), ("batch", dict(tick=1)),
                              ("batch", dict(outputs=1)), ("request", dict(output_timestamps_ns=[449])),
                              ("request", dict(generated_tokens=[99])), ("request", dict(state="Cancelled")),
                              ("stopped", dict(batches=1)), ("stopped", dict(reason="batch_budget"))):
            with self.subTest(kind=kind, changes=changes):
                data = fixture()
                next(e for e in data[0] if e.get("event") == kind).update(changes)
                with self.assertRaises(ValueError):
                    P.profile(*data)
        data = fixture()
        data[0].pop(-2)
        with self.assertRaises(ValueError):
            P.profile(*data)

    def test_bounds_reject_without_truncation(self):
        for name, limit in (("MAX_BATCHES", 1), ("MAX_REQUESTS", 1), ("MAX_RECORDS", 3)):
            with self.subTest(name=name), mock.patch.object(P, name, limit), self.assertRaises(ValueError):
                P.profile(*fixture())

    def test_json_parser_rejects_duplicate_keys_nonfinite_and_truncation(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}', b'{"x":1e999}'):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                P.strict_json(raw)
        for raw in (b'{}', b'{}\n\n', b'\xff\n'):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                P.json_lines(raw)

    def test_hash_bound_file_loading_and_exclusive_output(self):
        events, timing, envelopes, identity = fixture()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            inputs = {}
            for name, raw in (("events", b"".join(map(encode, events))), ("host_timing", encode(timing)),
                              ("diagnostics", b"startup\n" + b"".join(map(encode, envelopes)))):
                path = root / (name + ".json")
                path.write_bytes(raw)
                inputs[name] = dict(path=str(path), sha256=hashlib.sha256(raw).hexdigest())
            manifest = dict(schema="FerricLiveRuntimeProfileInputsV1", inputs=inputs, expected_identity=identity)
            report = P.load_profile(manifest)
            self.assertEqual(report["input_sha256"], {key: value["sha256"] for key, value in inputs.items()})
            manifest_path, output = root / "manifest.json", root / "report.json"
            manifest_path.write_bytes(encode(manifest))
            self.assertEqual(P.main(["--manifest", str(manifest_path), "--output", str(output)]), 0)
            original = output.read_bytes()
            with self.assertRaises(SystemExit):
                P.main(["--manifest", str(manifest_path), "--output", str(output)])
            self.assertEqual(output.read_bytes(), original)
            inputs["events"]["sha256"] = "0" * 64
            with self.assertRaises(ValueError):
                P.load_profile(manifest)


if __name__ == "__main__":
    unittest.main()
