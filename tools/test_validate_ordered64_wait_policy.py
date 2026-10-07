"""Synthetic policy consistency fixtures; success grants no native authority."""

import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock


spec = importlib.util.spec_from_file_location(
    "validate_ordered64_wait_policy",
    Path(__file__).with_name("validate_ordered64_wait_policy.py"),
)
policy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(policy)


def encoded(records):
    return b"".join((json.dumps(row, separators=(",", ":")) + "\n").encode()
                    for row in records)


def fixture(active=True):
    metadata = dict(policy.METADATA,
                    live_profile=policy.ACTIVE_PROFILE if active else policy.CONTROL_PROFILE)
    if active:
        metadata["ordered64_wait_policy"] = copy.deepcopy(policy.WAIT_POLICY)
    context = dict(schema="FerricOrdered64WaitPolicyContextV1", active_poll=active,
                   controller_pid=411, worker_pid=912, device_unique_id=1956390207832604050,
                   controller_sha256="a" * 64, worker_sha256="b" * 64,
                   expected_ordered_groups=4, expected_dispatches=12)
    lifecycle = dict(metadata, worker_pids=[912], authority="none",
                     performance_qualified=False, benchmark_admitted=False,
                     serving_admitted=False)
    context["setup"] = dict(copy.deepcopy(lifecycle), schema="FerricQwen3TpBatchSetupV2",
                            tensor_parallel=1, device_unique_ids=[context["device_unique_id"]],
                            controller_sha256="a" * 64, worker_sha256="b" * 64,
                            running_worker_sha256=["b" * 64], serving_qualified=False,
                            performance_profile=copy.deepcopy(metadata))
    context["closed"] = dict(copy.deepcopy(lifecycle), schema="FerricQwen3TpBatchClosedV2",
                             execution_completed=True, all_workers_exited=True,
                             rank_dispatch_counts=[12])
    context["cleanup"] = dict(owned_pgid=411, returncode=0, owned_group_absent=True,
                              child_reaped=True, cleanup_ok=True, term_sent=False,
                              kill_sent=False, errors=[])
    before = dict(commands=7, command_ns=100, full_currentness_checks=2,
                  full_currentness_ns=20, operational_currentness_checks=5,
                  operational_currentness_ns=40, kernel_admissions=2, kernel_admission_ns=10,
                  dispatches=0, dispatch_prepare_ns=0, dispatch_publish_ns=0,
                  dispatch_wait_ns=0, completion_polls=0, reads=2, read_bytes=8,
                  read_ns=10, writes=3, write_bytes=12, write_ns=15)
    after = dict(commands=19, command_ns=1100, full_currentness_checks=4,
                 full_currentness_ns=40, operational_currentness_checks=30,
                 operational_currentness_ns=300, kernel_admissions=2, kernel_admission_ns=10,
                 dispatches=12, dispatch_prepare_ns=100, dispatch_publish_ns=50,
                 dispatch_wait_ns=600, completion_polls=16, reads=3, read_bytes=12,
                 read_ns=20, writes=4, write_bytes=16, write_ns=30)
    records = []
    for ordinal, phase, counters in ((0, "before_workload", before), (1, "after_workload", after)):
        snapshot = dict(schema="FerricRuntimeDiagnosticSnapshotV1", authority="none",
                        performance_qualified=False,
                        scope="cumulative overlapping worker host-wall counters, not GPU timestamps",
                        process_id=912, device_unique_id=context["device_unique_id"],
                        rank=0, ordinal=ordinal, counters=counters)
        delta = {} if ordinal == 0 else {key: after[key] - before[key] for key in before}
        records.append(dict(copy.deepcopy(policy.RECORD_HEADER), **copy.deepcopy(metadata),
                            phase=phase, snapshots=[snapshot], counter_delta=delta,
                            placement={"controller_cpus": [0, 1]}))
    if active:
        records.append(dict(schema="Fe2o3Ordered64WaitPolicyDiagnosticV1",
                            policy="ActivePoll10msV1", worker_pid=912,
                            device_unique_id=context["device_unique_id"],
                            scope="successful ordered64 batches in this worker process",
                            active_window_ns=10000000, fallback_sleep_ns=50000,
                            closed_cleanly=True,
                            counters=dict(completed_batches=4, spin_pauses=10, fallback_sleeps=2,
                                          completed_without_fallback=3, completed_after_fallback=1)))
    return records, context


def set_after(records, key, value):
    records[1]["snapshots"][0]["counters"][key] = value
    records[1]["counter_delta"][key] = value - records[0]["snapshots"][0]["counters"][key]


class WaitPolicyTests(unittest.TestCase):
    def reject(self, records, context):
        with self.assertRaises(ValueError):
            policy.validate_capture(encoded(records), context)

    def test_control_and_active_are_consistent_without_native_authority(self):
        for active in (False, True):
            with self.subTest(active=active):
                records, context = fixture(active)
                raw = encoded(records)
                result = policy.validate_capture(raw, context)
                self.assertIs(result["policy_evidence_consistent"], True)
                self.assertEqual(result["policy"], "ActivePoll10msV1" if active else "Sleep50usV1")
                self.assertEqual(result["completed_ordered_groups"], 4)
                self.assertEqual(result["runtime_counter_delta"]["commands"], 12)
                self.assertEqual(result["runtime_counter_delta"]["command_ns"], 1000)
                self.assertEqual(result["runtime_counter_delta"]["completion_polls"], 16)
                self.assertEqual(result["stderr_sha256"], hashlib.sha256(raw).hexdigest())
                self.assertEqual(result["terminal_counters"], records[2]["counters"] if active else None)
                for key in ("native_execution_accepted", "performance_qualified",
                            "vendor_comparison", "gpu_time_measured"):
                    self.assertIs(result[key], False)

    def test_record_count_order_missing_and_duplicate_terminal_fail(self):
        records, context = fixture()
        for rows in (records[:2], records + [records[2]], records[1:] + records[:1],
                     [records[0], records[2], records[1]], [records[0], records[0], records[2]]):
            with self.subTest(rows=rows):
                self.reject(rows, context)
        control, context = fixture(False)
        self.reject(control + [records[2]], context)
        self.reject(control[:1], context)

    def test_context_closed_keys_and_stale_bindings_fail(self):
        for key, value in (("schema", "old"), ("active_poll", 1), ("worker_pid", 913),
                           ("controller_pid", 412), ("device_unique_id", 1),
                           ("expected_ordered_groups", 5), ("expected_dispatches", 13),
                           ("controller_sha256", "c" * 64), ("worker_sha256", "c" * 64)):
            records, context = fixture()
            context[key] = value
            with self.subTest(key=key):
                self.reject(records, context)
        for missing in (True, False):
            records, context = fixture()
            if missing:
                del context["worker_pid"]
            else:
                context["unchecked"] = True
            self.reject(records, context)

    def test_context_integer_and_digest_bounds_fail(self):
        for key in ("controller_pid", "worker_pid", "device_unique_id",
                    "expected_ordered_groups", "expected_dispatches"):
            for value in (True, -1, 1 << 64, 1.0):
                records, context = fixture()
                context[key] = value
                with self.subTest(key=key, value=value):
                    self.reject(records, context)
        for key, value in (("controller_pid", 0), ("worker_pid", 1 << 32),
                           ("worker_pid", 411), ("expected_ordered_groups", 0),
                           ("expected_ordered_groups", 13), ("worker_sha256", "B" * 64),
                           ("controller_sha256", "a" * 63), ("worker_sha256", 123)):
            records, context = fixture()
            context[key] = value
            self.reject(records, context)

    def test_setup_closed_profiles_hashes_and_lifecycle_must_match(self):
        mutations = (("setup", "live_profile", "old"), ("closed", "live_profile", "old"),
                     ("setup", "worker_pids", [913]), ("closed", "worker_pids", [913]),
                     ("setup", "device_unique_ids", [1]), ("setup", "controller_sha256", "c" * 64),
                     ("setup", "worker_sha256", "c" * 64),
                     ("setup", "running_worker_sha256", ["c" * 64]),
                     ("closed", "rank_dispatch_counts", [11]),
                     ("closed", "execution_completed", False), ("closed", "all_workers_exited", False))
        for section, key, value in mutations:
            records, context = fixture()
            context[section][key] = value
            with self.subTest(section=section, key=key):
                self.reject(records, context)
        records, context = fixture()
        context["setup"]["performance_profile"]["live_profile"] = policy.CONTROL_PROFILE
        self.reject(records, context)

    def test_admission_flags_and_deep_boolean_integer_aliases_fail(self):
        for section in ("setup", "closed"):
            for key in ("performance_qualified", "benchmark_admitted", "serving_admitted"):
                for value in (True, 0):
                    records, context = fixture()
                    context[section][key] = value
                    self.reject(records, context)
        mutations = (("setup", "serving_qualified", True), ("setup", "tensor_parallel", True),
                     ("setup", "worker_pids", [True]), ("closed", "rank_dispatch_counts", [True]),
                     ("cleanup", "returncode", False), ("cleanup", "child_reaped", 1))
        for section, key, value in mutations:
            records, context = fixture()
            context[section][key] = value
            self.reject(records, context)
        records, context = fixture()
        context["setup"]["performance_profile"]["ordered64_wait_policy"]["performance_qualified"] = 0
        self.reject(records, context)

    def test_closed_cleanly_terminal_cannot_override_failed_cleanup(self):
        for key, value in (("returncode", 1), ("owned_pgid", 999), ("owned_group_absent", False),
                           ("child_reaped", False), ("cleanup_ok", False), ("term_sent", True),
                           ("kill_sent", True), ("errors", ["lost process visibility"])):
            records, context = fixture()
            self.assertIs(records[2]["closed_cleanly"], True)
            context["cleanup"][key] = value
            with self.subTest(key=key):
                self.reject(records, context)

    def test_active_annotations_are_required_exact_and_forbidden_in_control(self):
        for where in (0, 1, "setup", "closed", "performance_profile"):
            records, context = fixture()
            target = (records[where] if type(where) is int else context["setup"][where]
                      if where == "performance_profile" else context[where])
            del target["ordered64_wait_policy"]
            self.reject(records, context)
        records, context = fixture(False)
        records[0]["ordered64_wait_policy"] = copy.deepcopy(policy.WAIT_POLICY)
        self.reject(records, context)
        records, context = fixture()
        records[0]["ordered64_wait_policy"]["active_window_ns"] = 9999999
        self.reject(records, context)

    def test_snapshot_and_terminal_identity_types_and_closed_shapes_fail(self):
        for section, key, value in ((0, "process_id", 913), (1, "device_unique_id", 1),
                                    (0, "rank", False), (1, "ordinal", True)):
            records, context = fixture()
            records[section]["snapshots"][0][key] = value
            self.reject(records, context)
        for key, value in (("worker_pid", 913), ("device_unique_id", 1), ("closed_cleanly", 1),
                           ("closed_cleanly", False), ("policy", "Sleep50usV1"),
                           ("active_window_ns", True), ("fallback_sleep_ns", 1)):
            records, context = fixture()
            records[2][key] = value
            self.reject(records, context)
        for index in (0, 1, 2):
            records, context = fixture()
            records[index]["unchecked"] = 0
            self.reject(records, context)

    def test_runtime_counter_keys_types_and_u64_bounds_fail(self):
        for ordinal in (0, 1):
            for key in policy.COUNTERS:
                for value in (True, -1, 1 << 64, 1.0):
                    records, context = fixture()
                    records[ordinal]["snapshots"][0]["counters"][key] = value
                    with self.subTest(ordinal=ordinal, key=key, value=value):
                        self.reject(records, context)
            for missing in (True, False):
                records, context = fixture()
                counters = records[ordinal]["snapshots"][0]["counters"]
                if missing:
                    del counters["reads"]
                else:
                    counters["unchecked"] = 0
                self.reject(records, context)

    def test_all_deltas_are_exact_and_nonnegative(self):
        for key in policy.COUNTERS:
            records, context = fixture()
            records[1]["counter_delta"][key] += 1
            self.reject(records, context)
        for value in (True, 1.0):
            records, context = fixture()
            records[1]["counter_delta"]["reads"] = value
            self.reject(records, context)
        records, context = fixture()
        set_after(records, "commands", 6)
        self.reject(records, context)
        records, context = fixture()
        records[0]["counter_delta"] = {"commands": 0}
        self.reject(records, context)

    def test_dispatch_enclosure_and_nonempty_interval_fail(self):
        for ordinal, key, value in ((0, "dispatches", 1), (1, "dispatches", 13),
                                    (1, "commands", 7), (1, "command_ns", 100)):
            records, context = fixture()
            if ordinal:
                set_after(records, key, value)
            else:
                records[0]["snapshots"][0]["counters"][key] = value
            self.reject(records, context)

    def test_policy_counter_types_bounds_and_closed_keys_fail(self):
        for key in policy.POLICY_COUNTERS:
            for value in (True, -1, 1 << 64, 1.0):
                records, context = fixture()
                records[2]["counters"][key] = value
                self.reject(records, context)
        for missing in (True, False):
            records, context = fixture()
            if missing:
                del records[2]["counters"]["spin_pauses"]
            else:
                records[2]["counters"]["unchecked"] = 0
            self.reject(records, context)

    def test_completed_groups_partition_fallback_and_poll_arithmetic_fail(self):
        for changes in ({"completed_batches": 3, "completed_without_fallback": 2},
                        {"completed_without_fallback": 4},
                        {"completed_without_fallback": (1 << 64) - 1},
                        {"fallback_sleeps": 0},
                        {"fallback_sleeps": 1, "completed_after_fallback": 2,
                         "completed_without_fallback": 2},
                        {"completed_after_fallback": 0, "completed_without_fallback": 4},
                        {"spin_pauses": (1 << 64) - 1}):
            records, context = fixture()
            records[2]["counters"].update(changes)
            self.reject(records, context)
        records, context = fixture()
        set_after(records, "completion_polls", 15)
        self.reject(records, context)

    def test_poll_bound_is_inclusive_and_allows_direct_dispatch_polls(self):
        for polls in (16, 17):
            records, context = fixture()
            set_after(records, "completion_polls", polls)
            self.assertTrue(policy.validate_capture(encoded(records), context)["policy_evidence_consistent"])
        for counts in (dict(spin_pauses=12, fallback_sleeps=0, completed_without_fallback=4,
                            completed_after_fallback=0),
                       dict(spin_pauses=8, fallback_sleeps=4, completed_without_fallback=0,
                            completed_after_fallback=4)):
            records, context = fixture()
            records[2]["counters"].update(counts)
            self.assertTrue(policy.validate_capture(encoded(records), context)["policy_evidence_consistent"])
        records, context = fixture()
        records[2]["counters"].update(spin_pauses=0, fallback_sleeps=0,
                                      completed_without_fallback=4, completed_after_fallback=0)
        set_after(records, "completion_polls", 4)
        self.assertTrue(policy.validate_capture(encoded(records), context)["policy_evidence_consistent"])

    def test_raw_input_must_be_complete_bounded_bytes_with_exact_lines(self):
        records, context = fixture()
        raw = encoded(records)
        for bad in (b"", raw[:-1], raw + b"\n", b"\n" + raw, raw.decode(), bytearray(raw),
                    b" " * policy.LIMIT + b"\n", raw.replace(b'"none"', b'"\xff"', 1),
                    b"{\n" + b"\n".join(raw.splitlines()[1:]) + b"\n"):
            with self.subTest(kind=type(bad)), self.assertRaises(ValueError):
                policy.validate_capture(bad, context)
        lines = raw.splitlines()
        lines[2] += b" " * 1024
        with self.assertRaises(ValueError):
            policy.validate_capture(b"\n".join(lines) + b"\n", context)

    def test_duplicate_keys_nonfinite_values_and_nonobject_records_fail(self):
        records, context = fixture()
        raw = encoded(records)
        for old, new in ((b'"authority":"none"', b'"authority":"none","authority":"none"'),
                         (b'"commands":7', b'"commands":7,"commands":7')):
            bad = raw.replace(old, new, 1)
            self.assertNotEqual(raw, bad)
            with self.assertRaises(ValueError):
                policy.validate_capture(bad, context)
        for token in (b"NaN", b"Infinity", b"-Infinity"):
            records, context = fixture()
            records[0]["placement"] = {"extra": "NONFINITE"}
            bad = encoded(records).replace(b'"NONFINITE"', token)
            with self.assertRaises(ValueError):
                policy.validate_capture(bad, context)
        for first in ([], None, True):
            records, context = fixture()
            records[0] = first
            self.reject(records, context)
        with self.assertRaises(ValueError):
            policy.decode(b'{"setup":{"flag":false,"flag":0}}')

    def test_read_bound_requires_exact_hash_regular_size_and_canonical_path(self):
        with tempfile.TemporaryDirectory(prefix="ordered64-policy-fixture-") as directory:
            path = Path(directory).resolve() / "capture.jsonl"
            raw = b'{"fixture":true}\n'
            path.write_bytes(raw)
            sha = hashlib.sha256(raw).hexdigest()
            self.assertEqual(policy.read_bound(path, sha, len(raw)), raw)
            for expected, maximum in (("0" * 64, len(raw)), (sha, len(raw) - 1), ("A" * 64, len(raw))):
                with self.assertRaises(ValueError):
                    policy.read_bound(path, expected, maximum)
            link = path.with_name("link")
            link.symlink_to(path)
            with self.assertRaises(ValueError):
                policy.read_bound(link, sha, len(raw))
            with self.assertRaises(ValueError):
                policy.read_bound(Path("capture.jsonl"), sha, len(raw))
            path.write_bytes(b"")
            with self.assertRaises(ValueError):
                policy.read_bound(path, hashlib.sha256(b"").hexdigest(), 1)

    @unittest.skipUnless(hasattr(os, "mkfifo"), "FIFO fixture requires POSIX")
    def test_read_bound_rejects_fifo_without_reading(self):
        with tempfile.TemporaryDirectory(prefix="ordered64-policy-fifo-") as directory:
            path = Path(directory).resolve() / "fifo"
            os.mkfifo(path, 0o600)
            real_open = os.open

            def nonblocking_open(filename, flags, *args, **kwargs):
                self.assertTrue(flags & os.O_NONBLOCK)
                return real_open(filename, flags, *args, **kwargs)

            with mock.patch.object(policy.os, "open", side_effect=nonblocking_open) as checked_open:
                with self.assertRaises(ValueError):
                    policy.read_bound(path, "0" * 64, 1024)
                checked_open.assert_called_once()


if __name__ == "__main__":
    unittest.main()
