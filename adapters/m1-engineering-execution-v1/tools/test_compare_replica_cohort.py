"""Synthetic host evidence only; fixtures never launch a controller or GPU."""

import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location("replica_checker", Path(__file__).with_name("compare_replica_cohort.py"))
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)
BASE, TRACE = CHECK.BASE, CHECK.TRACE
PROFILE = {"runtime_cache_admission": False, "runtime_operational": True, "dispatch_sequences": False,
           "queue_rollover": False, "projection": "baseline", "attention": "baseline", "runtime_profiling": False}
DOMAIN = {"clock": "CLOCK_MONOTONIC_RAW", "hostname": "synthetic-host",
          "boot_id": "11111111-2222-3333-4444-555555555555", "time_namespace_dev": 4, "time_namespace_ino": 12}


def expect(layout="1xTP8", budget=16, chunk=16, wide=False, pruning=False):
    return {"schema": "FerricReplicaExpectationV1", "layout": layout, "device_unique_ids": list(range(100, 108)),
        "controller_sha256": BASE.sha256(b"synthetic-controller"), "worker_sha256": BASE.sha256(b"synthetic-worker"),
        "artifact_hsaco_id": "3" * 64, "artifact_manifest_id": "4" * 64, "artifact_handoff_id": "5" * 64,
        "reference_sha256": BASE.REFERENCE_SHA256, "cohort_path": "/original/cohort", "source_path": "/model",
        "artifact_path": "/artifact", "source_controller_path": "/source/controller", "source_worker_path": "/source/worker",
        "snapshot_command": ["/snapshot", "--json"], "clock_domain": copy.deepcopy(DOMAIN), "nonce": "a" * 64,
        "settings": {"ready_timeout_ns": 10_000_000_000, "run_timeout_ns": 100_000_000_000,
                     "start_lead_ns": 1_000_000_000, "max_lateness_ns": 10_000_000},
        "host_reserve_bytes": 32 * 1024**3,
        "controller_options": ["--runtime-operational", "--batch-tokens", str(budget), "--prefill-chunk", str(chunk)]
                              + (["--kernel-profile", "v5-wave32"] if wide else [])
                              + (["--prune-output-head"] if pruning else []),
        "kernel_profile": "v5-wave32" if wide else "v2", "performance_profile": copy.deepcopy(PROFILE),
        "output_head_pruning": pruning, "row_policy": {"scope": "per-instance", "row_budget": budget},
        "batch_tokens": budget, "prefill_chunk": chunk, "context_tokens": 128, "physical_pages": 64,
        "cache_ttl_ticks": 1024, "max_batches": 240}


def synthetic_batches(count, budget, chunk):
    # Separate fixture implementation: sort ready sets by cyclic distance, then
    # update entire-request state after collecting rows, as the coordinator does.
    state = {slot: {"position": 0, "tokens": []} for slot in range(count)}
    cursors, alternate = [0, 0], False
    batches = []
    while state:
        ready = [[slot for slot, value in state.items() if (value["position"] >= 5) == decoding]
                 for decoding in (True, False)]
        for mode in (0, 1):
            ready[mode].sort(key=lambda slot: (slot - cursors[mode]) % 32)
        both = all(ready)
        decode_limit = (int(not alternate) if budget == 1 else budget - 1) if both else budget
        rows = []
        for slot in ready[0][:decode_limit]:
            rows.append({"slot": slot, "generation": 1, "position": state[slot]["position"],
                         "kind": "Decode", "token": state[slot]["tokens"][-1]})
            cursors[0] = (slot + 1) % 32
        for slot in ready[1]:
            length = min(chunk, budget - len(rows), 5 - state[slot]["position"])
            if length == 0:
                break
            for position in range(state[slot]["position"], state[slot]["position"] + length):
                rows.append({"slot": slot, "generation": 1, "position": position,
                             "kind": "PrefillFinal" if position == 4 else "PrefillIntermediate",
                             "token": BASE.PROMPT_IDS[position]})
            cursors[1] = (slot + 1) % 32
        if both and budget == 1:
            alternate = not alternate
        outputs = []
        for row in rows:
            value = state[row["slot"]]
            value["position"] += 1
            if row["kind"] != "PrefillIntermediate":
                ordinal = len(value["tokens"])
                token = BASE.REFERENCE_IDS[ordinal]
                value["tokens"].append(token)
                outputs.append({"slot": row["slot"], "generation": 1, "index": ordinal,
                                "token": token, "finished": ordinal == 7})
        pages = sum(value["position"] != 0 for value in state.values())
        retired = sorted(slot for slot, value in state.items() if len(value["tokens"]) == 8)
        batches.append((rows, outputs, pages, retired))
        for slot in retired:
            del state[slot]
    return batches


def trace_fixture(expected, replica, started):
    world = len(replica["device_unique_ids"])
    index = int(replica["replica_id"][-2:])
    workers = [2000 + index * 10 + rank for rank in range(world)]
    record = lambda kind, **values: {"schema": BASE.SCHEMA_PREFIX + kind + "V2", "authority": "none", **values}
    setup = record("Setup", model="Qwen/Qwen3-8B", dtype="BF16", target="gfx950:xnack-", tensor_parallel=world,
        device_unique_ids=replica["device_unique_ids"], worker_pids=workers,
        running_worker_sha256=[expected["worker_sha256"]] * world, model_bundle_id=BASE.BUNDLE,
        session_id=f"{index + 1:064x}", page_tokens=16, prefix_cache=False, setup_seconds=1.0,
        collective=BASE.LEGACY_COLLECTIVE, prefill="true_multirow_chunked", attention="paged_causal_gqa",
        cache="complete_page_radix_after_retirement", arrival_policy="logical batch ticks; elapsed latency starts at admission",
        numerical_status="Contracted; independently compare emitted token IDs; not a serving qualification",
        output_head_pruning=expected["output_head_pruning"], performance_profile=copy.deepcopy(expected["performance_profile"]),
        replica_benchmark=started, weight_payload_bytes={"host_target": 16_381_470_720,
            "device_base": 13_891_534_848 + world * 608_256 + 2_489_327_616,
            "device_transposed": 15_136_194_560 if expected["performance_profile"]["projection"] in ("mfma", "auto") else 0})
    setup.update({key: expected[key] for key in BASE.IDENTITY_FIELDS | set(CHECK.POLICY)})
    if expected["kernel_profile"].startswith("v5-"):
        setup.update(kernel_profile=expected["kernel_profile"], kernel_row_capacity=32)
    records, arrivals, timestamps = [setup], [], [[] for _ in replica["request_names"]]
    at = started["lateness_ns"] + 100
    for slot, name in enumerate(replica["request_names"]):
        at += 10
        arrivals.append(at)
        records.append(record("Admission", name=name, slot=slot, generation=1, tick=0,
                              arrival_ns=at, prompt_tokens=BASE.PROMPT_IDS, cached_tokens=0, cached_pages=0))
    counts = [0] * world
    for tick, (rows, outputs, pages, retired) in enumerate(synthetic_batches(len(arrivals), expected["batch_tokens"], expected["prefill_chunk"])):
        begin = at + 100
        at = begin + 1_000_000 * (index + 1)
        for event in outputs:
            event["completed_ns"] = at
            timestamps[event["slot"]].append(at)
        head = len(outputs) if expected["output_head_pruning"] else len(rows)
        dispatch = [541 + (3 if head else 0)] + [540] * (world - 1)
        counts = [left + right for left, right in zip(counts, dispatch, strict=True)]
        records.append(record("Completed", tick=tick, batch_id=tick + 1, pool_batch_id=tick + 1, rows=rows,
            outputs=outputs, started_ns=begin, completed_ns=at, rank_dispatch_counts=dispatch,
            free_pages=64 - pages, retained_pages=pages, cached_pages=0, prefix_hits=0, hit_tokens=0,
            evicted_pages=0, output_head_rows=head))
        for slot in retired:
            times = timestamps[slot]
            gaps = [right - left for left, right in zip(times, times[1:])]
            text = " Paris. The capital of Italy is Rome"
            records.append(record("Request", name=replica["request_names"][slot], slot=slot, generation=1, state="Completed",
                prompt_tokens=BASE.PROMPT_IDS, generated_tokens=BASE.REFERENCE_IDS[:8], generated_text=text,
                generated_utf8_bytes=list(text.encode()), cached_prefix_tokens=0, arrival_tick=0, arrival_ns=arrivals[slot],
                output_timestamps_ns=times, cancelled_ns=None, ttft_ns=times[0] - arrivals[slot], decode_intervals_ns=gaps,
                tpot_ns=sum(gaps) // 7))
    closed = CHECK.frame("Closed", started["identity"], {"pid": started["pid"], "epoch_ns": started["epoch_ns"],
                                                        "closed_ns": started["epoch_ns"] + at + 1000})
    records.append(record("Closed", worker_pids=workers, all_workers_exited=True, rank_dispatch_counts=counts,
                          whole_seconds=20.0, replica_benchmark=closed))
    return records, closed


def cohort_fixture(root, expected):
    def write(name, value, raw=False):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(value if raw else CHECK.encoded(value))
    plan = CHECK.expected_plan(expected)
    write("plan.json", plan)
    write("launch-artifacts/controller", b"synthetic-controller", True)
    write("launch-artifacts/worker", b"synthetic-worker", True)
    cards = {f"card{i}": {"Unique ID": hex(100 + i), "GPU use (%)": "0", "GPU Memory Allocated (VRAM%)": "0",
                          "GPU Memory Read/Write Activity (%)": "0"} for i in range(8)}
    write("gpu-before.json", cards)
    write("gpu-after.json", cards)
    available = 2 * 1024**4
    write("meminfo-before.txt", f"MemAvailable: {available // 1024} kB\n".encode(), True)
    write("memory-headroom.json", {"available_bytes": available,
          "required_bytes": 3 * plan["retained_target_bytes"] + expected["host_reserve_bytes"],
          "retained_target_bytes": plan["retained_target_bytes"], "reserve_bytes": expected["host_reserve_bytes"],
          "transient_multiplier": 3})
    first, all_ready, epoch = 2_000_000_000, 3_000_000_000, 4_000_000_000
    states, events = [], []
    for index, replica in enumerate(plan["replicas"]):
        rid = replica["replica_id"]
        identity = {"nonce": expected["nonce"], "replica_id": rid, "workload_sha256": plan["workload_sha256"],
                    "requests_sha256": replica["requests_sha256"], "device_unique_ids": replica["device_unique_ids"],
                    "clock_domain": expected["clock_domain"]}
        pid, spawn = 1000 + index, first + index * 1_000_000
        process = {"pid": pid, "state": "S", "pgid": pid, "session": pid, "start_ticks": 100 + index}
        sent = all_ready + index * 100
        ready = CHECK.frame("Ready", identity, {"pid": pid, "ready_ns": spawn + 10_000_000})
        started = CHECK.frame("Started", identity, {"pid": pid, "ready_ns": ready["ready_ns"], "start_received_ns": sent + 50,
             "epoch_ns": epoch, "started_ns": epoch + 1000 + index * 100, "lateness_ns": 1000 + index * 100})
        records, closed = trace_fixture(expected, replica, started)
        for direction, value, observed in (("receive", ready, ready["ready_ns"] + 10),
            ("send", CHECK.frame("Start", identity, {"epoch_ns": epoch}), sent),
            ("receive", started, started["started_ns"] + 10), ("receive", closed, closed["closed_ns"] + 10),
            ("send", CHECK.frame("CloseAck", identity, {"epoch_ns": epoch}), closed["closed_ns"] + 20)):
            events.append({"observed_ns": observed, "replica_id": rid, "direction": direction, "frame": value})
        original = expected["cohort_path"] + "/" + rid
        argv = [expected["cohort_path"] + "/launch-artifacts/controller", "--source", expected["source_path"],
            "--artifact", expected["artifact_path"], "--worker", expected["cohort_path"] + "/launch-artifacts/worker",
            "--devices", ",".join(map(str, replica["device_unique_ids"])), "--requests", original + "/requests.json",
            "--benchmark-control", original + "/control.json", "--disable-prefix-cache", "--allow-unauthenticated-machine-code",
            "--collective", "host-staged-v1"] + expected["controller_options"]
        states.append({"replica": replica, "identity": identity, "pid": pid, "spawned_ns": spawn, "argv": argv,
                       "ready": ready, "started": started, "closed": closed, "eof_ns": closed["closed_ns"] + 30,
                       "process_identity": process, "start_sent_ns": sent})
        write(rid + "/requests.json", replica["requests"])
        write(rid + "/control.json", {"schema": "FerricReplicaControlConfigV1",
            "socket_path": expected["cohort_path"] + "/control.sock", "launcher_pid": 900, "identity": identity,
            "io_timeout_ms": 10000, "min_start_lead_ns": 1_000_000, "max_start_lead_ns": 10_000_000_000,
            "max_lateness_ns": 10_000_000})
        write(rid + "/launch.json", {"pid": pid, "spawned_ns": spawn, "argv": argv, "process_identity": process})
        write(rid + "/control-received.bin", b"".join(map(CHECK.encoded, (ready, started, closed))), True)
        write(rid + "/stdout.jsonl", b"".join(map(CHECK.encoded, records)), True)
    quiet = max(state["eof_ns"] for state in states) + 1000
    write("control-events.jsonl", b"".join(CHECK.encoded(value) for value in sorted(events, key=lambda value: value["observed_ns"])), True)
    write("cohort.json", {"schema": "FerricReplicaCohortV1", "authority": "none", "status": "unvalidated-complete",
        "model_parity_qualified": False, "controller_sha256": expected["controller_sha256"],
        "worker_sha256": expected["worker_sha256"], "snapshot_command": expected["snapshot_command"],
        "workload_sha256": plan["workload_sha256"], "error": None, "clock_domain": expected["clock_domain"],
        "executables": {"controller": expected["cohort_path"] + "/launch-artifacts/controller",
                        "worker": expected["cohort_path"] + "/launch-artifacts/worker",
                        "source_controller": expected["source_controller_path"], "source_worker": expected["source_worker_path"]},
        "control": {"clock_domain": expected["clock_domain"], "nonce": expected["nonce"], "settings": expected["settings"],
                    "first_spawn_ns": first, "all_ready_ns": all_ready, "epoch_ns": epoch, "replicas": states},
        "exit_codes": [0] * len(states), "final_reap_ns": quiet + 100,
        "group_termination": {"schema": "FerricReplicaGroupTerminationV1", "observed_ns": quiet,
            "controller_pids": [state["pid"] for state in states], "live_group_members": [],
            "scope": "owned process groups; descendants must not detach", "confirmed": True},
        "gpu_snapshot_intervals": [{"phase": "before", "start_ns": first - 1000, "end_ns": first - 100, "completed": True},
                                   {"phase": "after", "start_ns": quiet + 200, "end_ns": quiet + 1000, "completed": True}]})
    write("reference.json", b"synthetic-reference-not-a-model-qualification", True)


class CohortTests(unittest.TestCase):
    def compare(self, root, expected):
        with mock.patch.object(BASE, "load_reference", return_value={}):
            return CHECK.compare(root, root / "reference.json", expected)

    def test_all_layouts_exact_outputs_global_window_and_memory_accounting(self):
        for layout, host, gpu in (("1xTP8", 16_381_470_720, 16_385_728_512),
                                  ("4xTP2", 65_525_882_880, 65_528_315_904),
                                  ("8xTP1", 131_051_765_760, 131_051_765_760)):
            with self.subTest(layout=layout), tempfile.TemporaryDirectory() as directory:
                root, expected = Path(directory), expect(layout)
                cohort_fixture(root, expected)
                result = self.compare(root, expected)
                self.assertTrue(result["passed"])
                self.assertEqual(result["global_output_tokens"], 64)
                self.assertEqual(result["physical_token_rows"], 96)
                self.assertEqual(result["weight_payload_bytes"], {"host_target": host, "device_base": gpu, "device_transposed": 0})
                self.assertEqual(result["global_output_tokens_per_second"], 64e9 / result["release_to_last_output_ns"])
                self.assertEqual(len(result["requests"]), 8)
                for request in result["requests"].values():
                    self.assertEqual(request["decode_interval_count"], 7)
                    self.assertEqual(request["release_to_first_token_ns"] - request["admission_ttft_ns"], request["arrival_epoch_offset_ns"])

    def test_wide_pruning_and_fixed_total_policy(self):
        for layout, budget, chunk in (("1xTP8", 32, 17), ("4xTP2", 4, 2), ("8xTP1", 2, 1)):
            with self.subTest(layout=layout), tempfile.TemporaryDirectory() as directory:
                root, expected = Path(directory), expect(layout, budget, chunk, True, True)
                expected["row_policy"] = {"scope": "fixed-total", "row_budget": budget * (8 // CHECK.LAYOUTS[layout])}
                cohort_fixture(root, expected)
                result = self.compare(root, expected)
                self.assertEqual(result["total_row_budget"], expected["row_policy"]["row_budget"])

    def test_independent_schedule_all_budgets_and_chunks(self):
        for count in (1, 2, 8):
            for budget in range(1, 33):
                for chunk in {1, min(5, budget), budget}:
                    with self.subTest(count=count, budget=budget, chunk=chunk):
                        actual = TRACE.schedule(count, budget, chunk)
                        fixture = synthetic_batches(count, budget, chunk)
                        self.assertEqual([len(batch["rows"]) for batch in actual], [len(batch[0]) for batch in fixture])
                        for left, (rows, outputs, pages, retired) in zip(actual, fixture, strict=True):
                            self.assertEqual(left["rows"], [(row["slot"], row["position"], row["kind"], row["token"]) for row in rows])
                            self.assertEqual(left["outputs"], [(event["slot"], event["index"], event["token"], event["finished"]) for event in outputs])
                            self.assertEqual((left["retained_pages"], left["retired"]), (pages, retired))

    def test_real_coordinator_schedule_vectors_supplied_independently(self):
        # Observed by the integration lead's actual coordinator/FakeRunner test;
        # these vectors are not derived by this checker's schedule function.
        for budget, rows, outputs in (
                (16, [16, 16, 16, 8, 8, 8, 8, 8, 5, 3], [3, 5, 8, 8, 8, 8, 8, 8, 5, 3]),
                (32, [32, 14, 8, 8, 8, 8, 8, 8, 2], [6, 8, 8, 8, 8, 8, 8, 8, 2])):
            batches = TRACE.schedule(8, budget, budget)
            self.assertEqual([len(batch["rows"]) for batch in batches], rows)
            self.assertEqual([len(batch["outputs"]) for batch in batches], outputs)
        batches = TRACE.schedule(8, 16, 16)
        self.assertEqual([[slot for slot, ordinal, _, _ in batch["outputs"] if ordinal == 0]
                          for batch in batches[:3]], [[0, 1, 2], [4, 5], [7, 3, 6]])
        for count in (1, 2):
            for budget in (16, 32):
                batches = TRACE.schedule(count, budget, budget)
                self.assertEqual([len(batch["rows"]) for batch in batches], [count * 5] + [count] * 7)
                self.assertEqual([len(batch["outputs"]) for batch in batches], [count] * 8)

    def test_control_custody_negative_mutations(self):
        mutations = [
            lambda c: c.update(status="failed"), lambda c: c.update(error="failure"), lambda c: c.update(extra=True),
            lambda c: c.update(exit_codes=[True]), lambda c: c.update(controller_sha256="e" * 64),
            lambda c: c["clock_domain"].update(hostname="different"),
            lambda c: c["control"].update(epoch_ns=c["control"]["epoch_ns"] + 1),
            lambda c: c["control"]["replicas"][0].update(eof_ns=1),
            lambda c: c["control"]["replicas"][0]["started"].update(lateness_ns=True),
            lambda c: c["control"]["replicas"][0]["process_identity"].update(pgid=123),
            lambda c: c["control"]["replicas"][0]["argv"].append("--runtime-profiling"),
            lambda c: c["group_termination"].update(confirmed=False),
            lambda c: c["group_termination"].update(live_group_members=[{"pid": 9999}]),
            lambda c: c.update(final_reap_ns=1),
            lambda c: c["gpu_snapshot_intervals"][0].update(end_ns=c["control"]["first_spawn_ns"] + 1),
            lambda c: c["gpu_snapshot_intervals"][1].update(start_ns=1),
            lambda c: c["gpu_snapshot_intervals"][1].update(completed=False),
        ]
        for index, mutation in enumerate(mutations):
            with self.subTest(mutation=index), tempfile.TemporaryDirectory() as directory:
                root, expected = Path(directory), expect()
                cohort_fixture(root, expected)
                path = root / "cohort.json"
                value = json.loads(path.read_bytes())
                mutation(value)
                path.write_bytes(CHECK.encoded(value))
                with self.assertRaises(ValueError):
                    self.compare(root, expected)

    def test_trace_numerical_allocation_and_timestamp_mutations(self):
        mutations = [
            lambda rows: rows[0]["running_worker_sha256"].__setitem__(0, "e" * 64),
            lambda rows: rows[0]["weight_payload_bytes"].update(device_base=1),
            lambda rows: rows[0].update(peer_artifact={}),
            lambda rows: rows[0]["performance_profile"].update(runtime_operational=False),
            lambda rows: rows[0]["replica_benchmark"].update(epoch_ns=float(rows[0]["replica_benchmark"]["epoch_ns"])),
            lambda rows: rows[1].update(arrival_ns=0), lambda rows: rows[1].update(slot=True),
            lambda rows: next(row for row in rows if "outputs" in row)["outputs"][0].update(token=1),
            lambda rows: next(row for row in rows if "rows" in row)["rows"][0].update(position=1),
            lambda rows: next(row for row in rows if "retained_pages" in row).update(retained_pages=0),
            lambda rows: next(row for row in rows if "generated_text" in row).update(generated_text="wrong"),
            lambda rows: next(row for row in rows if "tpot_ns" in row).update(tpot_ns=1),
            lambda rows: rows[-1].update(all_workers_exited=False),
        ]
        for index, mutation in enumerate(mutations):
            with self.subTest(mutation=index), tempfile.TemporaryDirectory() as directory:
                root, expected = Path(directory), expect()
                cohort_fixture(root, expected)
                path = root / "replica-00/stdout.jsonl"
                records = [json.loads(line) for line in path.read_bytes().splitlines()]
                mutation(records)
                path.write_bytes(b"".join(map(CHECK.encoded, records)))
                with self.assertRaises(ValueError):
                    self.compare(root, expected)

    def test_raw_frames_hashes_and_global_idle_are_required(self):
        changes = [("launch-artifacts/worker", b"different"), ("replica-00/control-received.bin", b"{}\n"),
                   ("control-events.jsonl", b"{}\n"), ("plan.json", b"{\"schema\":1,\"schema\":2}\n"),
                   ("meminfo-before.txt", b"MemAvailable: 1 kB\n"), ("gpu-after.json", b"{}\n")]
        for name, raw in changes:
            with self.subTest(path=name), tempfile.TemporaryDirectory() as directory:
                root, expected = Path(directory), expect()
                cohort_fixture(root, expected)
                (root / name).write_bytes(raw)
                with self.assertRaises(ValueError):
                    self.compare(root, expected)

    def test_external_expectation_is_closed_and_not_derived_from_cohort(self):
        for key, value in (("layout", "2xTP4"), ("nonce", "0" * 64), ("batch_tokens", True),
                           ("cohort_path", "/tmp/../cohort"), ("kernel_profile", "unreviewed"),
                           ("controller_options", ["--source", "/other"]), ("extra", True)):
            with self.subTest(key=key), self.assertRaises(ValueError):
                CHECK.expectation({**expect(), key: value})

    def test_reference_hash_gate_is_not_mocked_in_real_entry_point(self):
        with tempfile.TemporaryDirectory() as directory:
            root, expected = Path(directory), expect()
            cohort_fixture(root, expected)
            with self.assertRaisesRegex(ValueError, "reference file identity"):
                CHECK.compare(root, root / "reference.json", expected)

    def test_cross_replica_worker_session_and_request_replays_reject(self):
        for field in ("worker_pids", "session_id", "name"):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                root, expected = Path(directory), expect("8xTP1")
                cohort_fixture(root, expected)
                first = [json.loads(line) for line in (root / "replica-00/stdout.jsonl").read_bytes().splitlines()]
                path = root / "replica-01/stdout.jsonl"
                rows = [json.loads(line) for line in path.read_bytes().splitlines()]
                if field == "worker_pids":
                    rows[0][field] = rows[-1][field] = first[0][field]
                elif field == "session_id":
                    rows[0][field] = first[0][field]
                else:
                    rows[1][field] = first[1][field]
                path.write_bytes(b"".join(map(CHECK.encoded, rows)))
                with self.assertRaises(ValueError):
                    self.compare(root, expected)

    def test_partial_extra_and_duplicate_frames_reject(self):
        for variant in ("partial", "extra", "duplicate_key", "oversized"):
            with self.subTest(variant=variant), tempfile.TemporaryDirectory() as directory:
                root, expected = Path(directory), expect()
                cohort_fixture(root, expected)
                path = root / "replica-00/control-received.bin"
                data = path.read_bytes()
                if variant == "partial":
                    data = data[:-1]
                elif variant == "extra":
                    data += b"{}\n"
                elif variant == "duplicate_key":
                    data = data.replace(b'{"authority":"none",', b'{"authority":"none","authority":"none",', 1)
                else:
                    data = b" " * 4096 + data
                path.write_bytes(data)
                with self.assertRaises(ValueError):
                    self.compare(root, expected)

    def test_symlinked_evidence_cannot_be_read(self):
        with tempfile.TemporaryDirectory() as directory:
            root, expected = Path(directory), expect()
            cohort_fixture(root, expected)
            real = root / "actual-worker"
            path = root / "launch-artifacts/worker"
            path.rename(real)
            path.symlink_to(real)
            with self.assertRaisesRegex(ValueError, "symlinked"):
                self.compare(root, expected)

    def test_mfma_and_auto_transposed_payload_is_not_hidden(self):
        for projection in ("mfma", "auto"):
            with self.subTest(projection=projection), tempfile.TemporaryDirectory() as directory:
                root, expected = Path(directory), expect("4xTP2")
                expected["kernel_profile"] = "v3-mfma"
                expected["performance_profile"]["projection"] = projection
                expected["controller_options"] += ["--kernel-profile", "v3-mfma", "--projection", projection]
                cohort_fixture(root, expected)
                result = self.compare(root, expected)
                self.assertEqual(result["weight_payload_bytes"]["device_transposed"], 60_544_778_240)

    @unittest.skipUnless(os.environ.get("FERRIC_FROZEN_REFERENCE"), "frozen reference not configured")
    def test_cli_positive_and_exclusive_output_with_real_reference_pin(self):
        with tempfile.TemporaryDirectory() as directory:
            root, expected = Path(directory), expect("4xTP2")
            cohort_fixture(root, expected)
            reference = Path(os.environ["FERRIC_FROZEN_REFERENCE"])
            expectation_path, output = root / "expect.json", root / "report.json"
            expectation_path.write_bytes(CHECK.encoded(expected))
            argv = [sys.executable, "-I", "-B", str(Path(CHECK.__file__)), "--cohort-dir", str(root),
                    "--reference", str(reference), "--expect", str(expectation_path), "--output", str(output)]
            result = subprocess.run(argv, capture_output=True, timeout=20, check=False)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            self.assertTrue(json.loads(output.read_bytes())["passed"])
            original = output.read_bytes()
            result = subprocess.run(argv, capture_output=True, timeout=20, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(output.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
