"""Synthetic checker-policy tests, not GPU or numerical evidence."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import target_batch_decode_v1 as check


def fixture(count=32, device_residual=True):
    plan = {key: str(index + 1) * 64 for index, key in enumerate(sorted(check.IDENTITIES))}
    plan.update(schema="FerricTargetBatchDecodeExpectationV1", device_unique_id=987654321,
                request_name="target-single-request", new_tokens=count,
                collective="device-tp1-v3" if device_residual else "host-staged-reuse-v3",
                max_batches=count + 4, cache_ttl_ticks=1024, kernel_profile="v3-wave",
                host_timing_enabled=False,
                performance_profile={"runtime_cache_admission": True, "runtime_operational": True,
                    "dispatch_sequences": False, "queue_rollover": False, "runtime_profiling": False,
                    "projection": "wave", "attention": "baseline"})
    reference = {"format": "FERRIC-QWEN3-8B-GREEDY-REFERENCE-V1",
                 "authority": "independent-offline-reference-only",
                 "model": {"repository": "Qwen/Qwen3-8B", "revision": check.REVISION,
                           "deployment_bundle_identity": check.BUNDLE},
                 "prompt": {"text": check.PROMPT, "token_ids": check.PROMPT_IDS, "add_special_tokens": False},
                 "execution": {"passes": [{"token_ids": check.TOKENS, "argmax_token_ids": check.TOKENS,
                     "decoded_new_tokens": "".join(check.PIECES)} for _ in range(2)]}}
    setup = {key: plan[key] for key in check.IDENTITIES | {"collective", "max_batches", "cache_ttl_ticks", "performance_profile"}}
    setup.update(model="Qwen/Qwen3-8B", dtype="BF16", target="gfx950:xnack-", tensor_parallel=1,
                 device_unique_ids=[plan["device_unique_id"]], worker_pids=[87654321],
                 running_worker_sha256=[plan["worker_sha256"]], model_bundle_id=check.BUNDLE,
                 session_id="a" * 64, batch_tokens=1, prefill_chunk=1, page_tokens=16,
                 physical_pages=4, context_tokens=64, prefix_cache=False, output_head_pruning=False,
                 setup_seconds=1.0, prefill="true_multirow_chunked", attention="paged_causal_gqa",
                 cache="complete_page_radix_after_retirement",
                 arrival_policy="logical batch ticks; elapsed latency starts at admission",
                 numerical_status="Contracted; independently compare emitted token IDs; not a serving qualification")
    def record(kind, value):
        return {"schema": "FerricQwen3TpBatch" + kind + "V2", "authority": "none", **value}
    records = [record("Setup", setup), record("Admission", {
        "name": plan["request_name"], "slot": 0, "generation": 1, "tick": 0,
        "arrival_ns": 1000, "prompt_tokens": check.PROMPT_IDS, "cached_tokens": 0, "cached_pages": 0})]
    times = []
    dispatch = 616 if device_residual else 544
    for at in range(count + 4):
        end = 2000 + (at + 1) * 1000
        row = {"slot": 0, "generation": 1, "token": check.PROMPT_IDS[at] if at < 5 else check.TOKENS[at - 5],
               "position": at, "kind": "PrefillIntermediate" if at < 4 else "PrefillFinal" if at == 4 else "Decode"}
        outputs = []
        if at >= 4:
            times.append(end)
            outputs = [{"slot": 0, "generation": 1, "token": check.TOKENS[at - 4], "index": at - 4,
                        "completed_ns": end, "finished": at == count + 3}]
        retained = (at + 16) // 16
        records.append(record("Completed", {"tick": at, "batch_id": at + 1, "pool_batch_id": at + 1,
            "rows": [row], "outputs": outputs, "started_ns": end - 900, "completed_ns": end,
            "rank_dispatch_counts": [dispatch], "output_head_rows": 1, "retained_pages": retained,
            "free_pages": 4 - retained, "cached_pages": 0, "prefix_hits": 0, "hit_tokens": 0, "evicted_pages": 0}))
    text = "".join(check.PIECES[:count])
    records.append(record("Request", {"name": plan["request_name"], "slot": 0, "generation": 1,
        "state": "Completed", "prompt_tokens": check.PROMPT_IDS, "generated_tokens": check.TOKENS[:count],
        "generated_utf8_bytes": list(text.encode()), "generated_text": text,
        "cached_prefix_tokens": 0, "arrival_tick": 0, "arrival_ns": 1000,
        "output_timestamps_ns": times, "cancelled_ns": None, "ttft_ns": times[0] - 1000,
        "decode_intervals_ns": [1000] * (count - 1), "tpot_ns": 1000}))
    records.append(record("Closed", {"worker_pids": [87654321], "all_workers_exited": True,
        "rank_dispatch_counts": [(count + 4) * dispatch], "whole_seconds": 2.0}))
    return records, plan, reference


class TargetBatchPolicyTests(unittest.TestCase):
    def test_accepts_bounded_profiles_and_collectives(self):
        for count in (2, 32):
            for device in (False, True):
                with self.subTest(count=count, device=device):
                    records, plan, reference = fixture(count, device)
                    result = check.validate_records(records, plan, reference)
                    self.assertTrue(result["passed"])
                    self.assertEqual(result["dispatches"], (count + 4) * (616 if device else 544))
                    self.assertEqual(result["timing"]["decode_interval_count"], count - 1)
                    self.assertEqual(result["timing"]["post_first_tokens_per_second"], 1_000_000)
                    self.assertFalse(result["benchmark_qualified"])

    def test_public_report_redacts_private_identity(self):
        records, plan, reference = fixture()
        result = check.validate_records(records, plan, reference)
        raw = json.dumps(result)
        for forbidden in ("987654321", "87654321", "worker_pids", "device_unique_id", "session_id", "/home/"):
            self.assertNotIn(forbidden, raw)
        self.assertFalse(result["cleanup"]["final_free_page_count_observed"])

    def test_rejects_reference_oracle_mutations(self):
        for key in ("token_ids", "argmax_token_ids", "decoded_new_tokens"):
            records, plan, reference = copy.deepcopy(fixture())
            reference["execution"]["passes"][1][key] = []
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_rejects_unpinned_reference_file(self):
        with self.assertRaisesRegex(ValueError, "digest"):
            check.load_reference(json.dumps(fixture()[2]).encode())

    def test_rejects_unrelated_setup_extensions(self):
        for field in ("runtime_ordered_batches", "head_precision", "runtime_diagnostic_status", "peer_artifact"):
            records, plan, reference = fixture()
            records[0][field] = True
            with self.subTest(field=field), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_rejects_wrong_precision_or_work(self):
        for key, value in (("dtype", "FP8"), ("tensor_parallel", 2), ("batch_tokens", 2),
                           ("prefix_cache", True), ("output_head_pruning", True), ("physical_pages", 2),
                           ("model_bundle_id", "a" * 64), ("controller_sha256", "a" * 64),
                           ("device_unique_ids", [1]), ("running_worker_sha256", ["a" * 64])):
            records, plan, reference = fixture()
            records[0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_rejects_wrong_plan_or_runtime_options(self):
        for key, value in (("new_tokens", 16), ("max_batches", 100), ("kernel_profile", "v5-mfma32"),
                           ("collective", "device-peer-serial-v4"), ("host_timing_enabled", 1)):
            records, plan, reference = fixture()
            plan[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)
        for key in ("runtime_profiling", "queue_rollover"):
            records, plan, reference = fixture()
            plan["performance_profile"][key] = True
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_dispatch_sequences_are_explicit_and_limited_to_wave_device(self):
        records, plan, reference = fixture()
        plan["performance_profile"]["dispatch_sequences"] = True
        result = check.validate_records(records, plan, reference)
        self.assertTrue(result["configuration"]["performance_profile"]["dispatch_sequences"])
        self.assertFalse(result["configuration"]["runtime_ordered_batches"])
        self.assertEqual(result["dispatches"], 22176)
        for field, value in (("projection", "baseline"), ("dispatch_sequences", 1)):
            bad_records, bad_plan, bad_reference = copy.deepcopy((records, plan, reference))
            bad_plan["performance_profile"][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                check.validate_records(bad_records, bad_plan, bad_reference)
        records, plan, reference = fixture(device_residual=False)
        plan["performance_profile"]["dispatch_sequences"] = True
        with self.assertRaises(ValueError):
            check.validate_records(records, plan, reference)
        records, plan, reference = fixture()
        plan["performance_profile"] = {**plan["performance_profile"], "dispatch_sequences": True}
        with self.assertRaisesRegex(ValueError, "performance_profile"):
            check.validate_records(records, plan, reference)

    def test_rejects_missing_duplicate_or_reordered_batches(self):
        for mutation in (lambda r: r.pop(3), lambda r: r.insert(3, r[3]),
                         lambda r: r.__setitem__(slice(2, 4), r[2:4][::-1])):
            records, plan, reference = fixture()
            mutation(records)
            with self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_rejects_row_and_output_changes(self):
        for key, value in (("token", 123), ("position", 2), ("generation", 2), ("slot", 1), ("kind", "Decode")):
            records, plan, reference = fixture()
            records[2]["rows"][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)
        for key, value in (("token", 123), ("index", 2), ("finished", True), ("completed_ns", 999)):
            records, plan, reference = fixture()
            records[6]["outputs"][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_rejects_dispatch_and_page_count_changes(self):
        for key, value in (("rank_dispatch_counts", [544]), ("retained_pages", 0),
                           ("free_pages", 4), ("cached_pages", 1), ("output_head_rows", 0)):
            records, plan, reference = fixture()
            records[2][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_rejects_timing_changes(self):
        for index, key, value in ((2, "started_ns", 0), (2, "completed_ns", 0),
                                  (-2, "tpot_ns", 0), (-2, "ttft_ns", 0),
                                  (-1, "whole_seconds", 1.0), (0, "setup_seconds", float("nan"))):
            records, plan, reference = fixture()
            records[index][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_rejects_wrong_output_bytes_or_incomplete_cleanup(self):
        for index, key, value in ((-2, "generated_utf8_bytes", [0]), (-2, "generated_tokens", []),
                                  (-2, "state", "Cancelled"), (-2, "cancelled_ns", 10),
                                  (-1, "all_workers_exited", False), (-1, "worker_pids", [1]),
                                  (-1, "rank_dispatch_counts", [1])):
            records, plan, reference = fixture()
            records[index][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_bool_is_not_an_integer_receipt(self):
        for index, key in ((0, "tensor_parallel"), (1, "generation"), (2, "output_head_rows")):
            records, plan, reference = fixture()
            records[index][key] = True
            with self.subTest(key=key), self.assertRaises(ValueError):
                check.validate_records(records, plan, reference)

    def test_duplicate_keys_and_nonfinite_json_rejected(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}'):
            with self.assertRaises(ValueError):
                check.json_value(raw)

    def test_file_reader_rejects_symlink_empty_and_oversize(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input"
            path.write_bytes(b"123")
            self.assertEqual(check.read_bounded(path, 3), b"123")
            with self.assertRaises(ValueError):
                check.read_bounded(path, 2)
            link = Path(directory) / "link"
            link.symlink_to(path)
            with self.assertRaises(OSError):
                check.read_bounded(link, 3)
            path.write_bytes(b"")
            with self.assertRaises(ValueError):
                check.read_bounded(path, 3)

    def test_compare_checks_status_workload_and_terminal_newline(self):
        records, plan, reference = fixture()
        capture = b"".join(json.dumps(record).encode() + b"\n" for record in records)
        workload = json.dumps(check.expected_workload(plan)).encode()
        expected = json.dumps(plan).encode()
        # Only the file-digest join is mocked here; structural reference checks
        # still run. The actual pinned oracle is exercised separately on host.
        with mock.patch.object(check, "load_reference", return_value=reference):
            result = check.compare(capture, b"0\n", workload, b"synthetic", expected)
            self.assertTrue(result["passed"])
            for raw, status, work in ((capture[:-1], b"0\n", workload),
                                      (capture, b"1\n", workload),
                                      (capture, b"0\n", b'{"requests":[]}')):
                with self.assertRaises(ValueError):
                    check.compare(raw, status, work, b"synthetic", expected)


if __name__ == "__main__":
    unittest.main()
