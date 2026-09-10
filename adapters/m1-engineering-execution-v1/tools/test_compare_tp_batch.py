"""Host-synthetic comparator fixtures; no GPU execution or timing evidence."""

import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location("compare_tp_batch", Path(__file__).with_name("compare_tp_batch.py"))
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)
PINS = {"controller_sha256": "1" * 64, "worker_sha256": "2" * 64, "artifact_hsaco_id": "3" * 64}
GPU_IDS = list(range(100, 108))


def encoded(value):
    return (json.dumps(value, sort_keys=True) + "\n").encode()


def fixture(world=8, cache=True):
    """Independent fixed trace in the CLI's schema, with invented host times."""
    pids = list(range(1000, 1000 + world))
    setup = {"schema": "FerricQwen3TpBatchSetupV2", "authority": "none",
             "model": "Qwen/Qwen3-8B", "dtype": "BF16", "target": "gfx950:xnack-",
             "tensor_parallel": world, "device_unique_ids": GPU_IDS[:world], "worker_pids": pids,
             **PINS, "running_worker_sha256": [PINS["worker_sha256"]] * world,
             "model_bundle_id": CHECK.BUNDLE, "artifact_manifest_id": "4" * 64,
             "artifact_handoff_id": "5" * 64, "session_id": "6" * 64,
             "batch_tokens": 16, "prefill_chunk": 16, "page_tokens": 16,
             "physical_pages": 8, "context_tokens": 128, "cache_ttl_ticks": 1024,
             "prefix_cache": cache, "max_batches": 20, "setup_seconds": 2.0,
             "collective": "host_staged_fp32_rank_order_reduce_bf16_residual",
             "prefill": "true_multirow_chunked", "attention": "paged_causal_gqa",
             "cache": "complete_page_radix_after_retirement",
             "arrival_policy": "logical batch ticks; elapsed latency starts at admission",
             "numerical_status": "Contracted; independently compare emitted token IDs; not a serving qualification"}
    prompts = [CHECK.PROMPT_IDS + CHECK.REFERENCE_IDS[:12], CHECK.PROMPT_IDS[:],
               CHECK.PROMPT_IDS[:], CHECK.PROMPT_IDS + CHECK.REFERENCE_IDS[:14]]
    choices = [[17689, 374], [12095, 13, 576], [12095], [24081, 13]]
    texts = [" Spain is", " Paris. The", " Paris", " Madrid."]
    ids = [(0, 1), (1, 1), (2, 1), (0, 2)]
    arrival = [index * 1_000_000_000 + 1_000_000 for index in range(4)]
    times = [[] for _ in range(4)]
    outputs = [[] for _ in range(4)]
    result = [setup]

    def admit(index):
        result.append({"schema": "FerricQwen3TpBatchAdmissionV2", "authority": "none",
                       "name": CHECK.NAMES[index], "slot": ids[index][0], "generation": ids[index][1],
                       "tick": index, "arrival_ns": arrival[index], "prompt_tokens": prompts[index],
                       "cached_tokens": 16 if cache and index == 3 else 0,
                       "cached_pages": 1 if cache and index == 3 else 0})

    def retire(index):
        intervals = [b - a for a, b in zip(times[index], times[index][1:])]
        result.append({"schema": "FerricQwen3TpBatchRequestV2", "authority": "none",
                       "name": CHECK.NAMES[index], "slot": ids[index][0], "generation": ids[index][1],
                       "state": "Cancelled" if index == 2 else "Completed", "prompt_tokens": prompts[index],
                       "generated_tokens": outputs[index][:], "generated_utf8_bytes": list(texts[index].encode()),
                       "generated_text": texts[index], "cached_prefix_tokens": 16 if index == 3 and cache else 0,
                       "arrival_tick": index, "arrival_ns": arrival[index], "output_timestamps_ns": times[index][:],
                       "cancelled_ns": 3_002_000_000 if index == 2 else None,
                       "ttft_ns": times[index][0] - arrival[index], "decode_intervals_ns": intervals,
                       "tpot_ns": sum(intervals) // len(intervals) if intervals else None})

    def batch(tick, chunks):
        start, end = tick * 1_000_000_000 + 5_000_000, tick * 1_000_000_000 + 105_000_000
        rows, events = [], []
        for index, begin, count in chunks:
            for position in range(begin, begin + count):
                kind = ("PrefillIntermediate" if position < len(prompts[index]) - 1 else
                        "PrefillFinal" if position == len(prompts[index]) - 1 else "Decode")
                token = prompts[index][position] if position < len(prompts[index]) else outputs[index][-1]
                rows.append({"slot": ids[index][0], "generation": ids[index][1], "token": token,
                             "position": position, "kind": kind})
                if kind != "PrefillIntermediate":
                    ordinal = len(outputs[index])
                    token = choices[index][ordinal]
                    outputs[index].append(token)
                    times[index].append(end)
                    events.append({"slot": ids[index][0], "generation": ids[index][1], "token": token,
                                   "index": ordinal, "completed_ns": end,
                                   "finished": index != 2 and ordinal + 1 == len(choices[index])})
        retained = ([1, 3, 4, 3, 2] if cache else [1, 3, 4, 2, 2, 2])[tick]
        hits = 1 if cache and tick >= 3 else 0
        result.append({"schema": "FerricQwen3TpBatchCompletedV2", "authority": "none",
                       "tick": tick, "batch_id": tick + 1, "pool_batch_id": tick + 1,
                       "rows": rows, "outputs": events, "started_ns": start, "completed_ns": end,
                       "rank_dispatch_counts": [544] + [540] * (world - 1), "free_pages": 8 - retained,
                       "retained_pages": retained, "cached_pages": hits, "prefix_hits": hits,
                       "hit_tokens": hits * 16, "evicted_pages": 0})

    admit(0)
    batch(0, [(0, 0, 16)])
    admit(1)
    batch(1, [(1, 0, 5), (0, 16, 1)])
    admit(2)
    batch(2, [(0, 17, 1), (1, 5, 1), (2, 0, 5)])
    retire(0)
    admit(3)
    retire(2)
    batch(3, [(1, 6, 1), (3, 16, 3)] if cache else [(1, 6, 1), (3, 0, 15)])
    retire(1)
    batch(4, [(3, 19, 1)] if cache else [(3, 15, 4)])
    if not cache:
        batch(5, [(3, 19, 1)])
    retire(3)
    count = 5 if cache else 6
    result.append({"schema": "FerricQwen3TpBatchClosedV2", "authority": "none",
                   "worker_pids": pids, "all_workers_exited": True,
                   "rank_dispatch_counts": [544 * count] + [540 * count] * (world - 1),
                   "whole_seconds": 10.0})
    # JSONL has independent values, not shared Python list aliases.
    return json.loads(json.dumps(result))


def synthetic_reference():
    """A small schema fixture with a test-only patched digest, never live evidence."""
    one = {"token_ids": CHECK.REFERENCE_IDS, "argmax_token_ids": CHECK.REFERENCE_IDS,
           "decoded_new_tokens": "".join(CHECK.REFERENCE_PIECES)}
    return {"format": "FERRIC-QWEN3-8B-GREEDY-REFERENCE-V1",
            "authority": "independent-offline-reference-only",
            "model": {"repository": "Qwen/Qwen3-8B", "deployment_bundle_identity": CHECK.BUNDLE},
            "prompt": {"text": CHECK.PROMPT, "token_ids": CHECK.PROMPT_IDS, "add_special_tokens": False},
            "execution": {"passes": [copy.deepcopy(one), copy.deepcopy(one)]}}


def snapshots():
    return {f"card{i}": {"Unique ID": hex(identity), "GPU use (%)": "0",
                        "GPU Memory Allocated (VRAM%)": "0", "GPU Memory Read/Write Activity (%)": "0",
                        "Memory Activity": "N/A"} for i, identity in enumerate(GPU_IDS)}


class ComparatorTests(unittest.TestCase):
    def check(self, records, world=8, cache=None, pins=None):
        return CHECK.validate_records(records, GPU_IDS, world, PINS if pins is None else pins, cache)

    def reject(self, change, world=8, cache=True):
        records = fixture(world, cache)
        change(records)
        with self.assertRaises((ValueError, KeyError, TypeError, IndexError)):
            self.check(records, world)

    def test_positive_worlds_cache_profiles_have_exact_counts_and_reference_bytes(self):
        for world in (1, 2, 8):
            for cache in (False, True):
                report = self.check(fixture(world, cache), world, cache)
                self.assertTrue(report["passed"])
                self.assertEqual(report["physical_token_rows"], 34 if cache else 50)
                self.assertEqual(report["batch_count"], 5 if cache else 6)
                self.assertEqual(report["generated_tokens"], 8)
                self.assertEqual(report["requests"]["reuse-prefix"]["generated_text"], " Madrid.")
                self.assertIsNone(report["requests"]["cancel-between-batches"]["tpot_ns"])
                self.assertIn("no cache speedup claim", report["nonclaims"])

    def test_unpinned_checks_cannot_report_pass_and_wrong_pins_reject(self):
        for pins in ({}, {"controller_sha256": PINS["controller_sha256"]}):
            report = self.check(fixture(), pins=pins)
            self.assertFalse(report["passed"])
            self.assertEqual(report["validation"], "unpinned_not_pass")
        with self.assertRaises(ValueError):
            self.check(fixture(), pins={**PINS, "worker_sha256": "a" * 64})
        with self.assertRaises(ValueError):
            self.check(fixture(), world=2)
        with self.assertRaises(ValueError):
            self.check(fixture(), cache=False)

    def test_setup_identity_and_authority_mutations_reject(self):
        for key, bad in (("tensor_parallel", 1), ("tensor_parallel", 8.0), ("prefix_cache", 1),
                         ("authority", "protected"), ("model_bundle_id", "a" * 64),
                         ("worker_pids", [1000] * 8), ("device_unique_ids", GPU_IDS[::-1]),
                         ("running_worker_sha256", ["a" * 64] * 8),
                         ("batch_tokens", 15), ("prefill_chunk", True), ("setup_seconds", float("nan"))):
            self.reject(lambda rows, key=key, bad=bad: rows[0].__setitem__(key, bad))

    def test_missing_extra_reordered_or_unknown_fields_reject(self):
        self.reject(lambda rows: rows.pop(3))
        self.reject(lambda rows: rows.insert(3, copy.deepcopy(rows[3])))
        self.reject(lambda rows: rows.__setitem__(slice(3, 5), rows[3:5][::-1]))
        for index in range(len(fixture())):
            self.reject(lambda rows, index=index: rows[index].__setitem__("extra", True))

    def test_causal_rows_input_position_intermediate_kind_and_generation_reject(self):
        for key, bad in (("token", 1), ("position", 1), ("kind", "Decode"),
                         ("generation", 2), ("token", float(CHECK.PROMPT_IDS[0])), ("position", False)):
            self.reject(lambda rows, key=key, bad=bad: rows[2]["rows"][0].__setitem__(key, bad))
        self.reject(lambda rows: rows[2]["rows"].pop())
        self.reject(lambda rows: rows[6]["rows"][0].__setitem__("token", 374))
        self.reject(lambda rows: rows[10]["rows"][1].__setitem__("position", 15))

    def test_output_tokens_count_order_finished_flags_and_float_values_reject(self):
        for key, bad in (("token", 99), ("index", 1), ("finished", True), ("finished", 0),
                         ("token", 12095.0), ("completed_ns", 0)):
            self.reject(lambda rows, key=key, bad=bad: rows[4]["outputs"][0].__setitem__(key, bad))
        self.reject(lambda rows: rows[4]["outputs"].reverse())
        self.reject(lambda rows: rows[2]["outputs"].append(copy.deepcopy(rows[4]["outputs"][0])))
        self.reject(lambda rows: rows[4]["outputs"].pop())

    def test_cancel_and_slot_reuse_are_strict(self):
        self.reject(lambda rows: rows[9].__setitem__("state", "Completed"))
        self.reject(lambda rows: rows[9].__setitem__("cancelled_ns", 1))
        self.reject(lambda rows: rows[9]["generated_tokens"].append(13))
        self.reject(lambda rows: rows[8].__setitem__("generation", 1))
        self.reject(lambda rows: rows[8].__setitem__("tick", 2))
        self.reject(lambda rows: rows[9].__setitem__("tpot_ns", 1))

    def test_cache_admission_stats_and_page_boundary_require_exact_evidence(self):
        self.reject(lambda rows: rows[8].__setitem__("cached_tokens", 0))
        self.reject(lambda rows: rows[8].__setitem__("cached_pages", 0))
        for key in ("retained_pages", "free_pages", "cached_pages", "prefix_hits", "hit_tokens", "evicted_pages"):
            self.reject(lambda rows, key=key: rows[10].__setitem__(key, 999))
        self.reject(lambda rows: rows[8].__setitem__("cached_tokens", 16), cache=False)

    def test_decode_timing_ttft_tpot_and_retired_bytes_reject_drift(self):
        self.reject(lambda rows: rows[4].__setitem__("started_ns", 0))
        self.reject(lambda rows: rows[4].__setitem__("completed_ns", rows[4]["started_ns"]))
        self.reject(lambda rows: rows[7].__setitem__("ttft_ns", 1))
        self.reject(lambda rows: rows[7].__setitem__("tpot_ns", 1))
        self.reject(lambda rows: rows[7].__setitem__("decode_intervals_ns", [1]))
        self.reject(lambda rows: rows[7]["output_timestamps_ns"].reverse())
        self.reject(lambda rows: rows[7].__setitem__("generated_text", " Spain"))
        self.reject(lambda rows: rows[7]["generated_utf8_bytes"].__setitem__(0, 33))
        self.reject(lambda rows: rows[-1].__setitem__("whole_seconds", 2.0))

    def test_each_rank_counts_and_close_identity_are_required(self):
        self.reject(lambda rows: rows[2]["rank_dispatch_counts"].__setitem__(7, 539))
        self.reject(lambda rows: rows[2]["rank_dispatch_counts"].pop())
        self.reject(lambda rows: rows[-1]["rank_dispatch_counts"].__setitem__(0, 544))
        self.reject(lambda rows: rows[-1].__setitem__("all_workers_exited", 1))
        self.reject(lambda rows: rows[-1]["worker_pids"].reverse())

    def test_json_duplicate_nonfinite_and_equal_float_workload_reject(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}'):
            with self.assertRaises(ValueError):
                CHECK.json_value(raw)
        for key, bad in (("new_tokens", 2.0), ("arrival_tick", False), ("prompt", "other")):
            workload = CHECK.expected_workload()
            workload["requests"][0][key] = bad
            with self.assertRaises(ValueError):
                CHECK.load_workload(encoded(workload))

    def test_reference_hash_both_passes_and_literal_token_pieces_are_bound(self):
        raw = encoded(synthetic_reference())
        with self.assertRaises(ValueError):
            CHECK.load_reference(raw)
        with mock.patch.object(CHECK, "REFERENCE_SHA256", CHECK.sha256(raw)):
            CHECK.load_reference(raw)
        for key in ("token_ids", "argmax_token_ids", "decoded_new_tokens"):
            reference = synthetic_reference()
            if key == "decoded_new_tokens":
                reference["execution"]["passes"][1][key] += " drift"
            else:
                reference["execution"]["passes"][1][key] = CHECK.REFERENCE_IDS[:-1] + [1]
            bad = encoded(reference)
            with mock.patch.object(CHECK, "REFERENCE_SHA256", CHECK.sha256(bad)):
                with self.assertRaises(ValueError):
                    CHECK.load_reference(bad)

    def test_gpu_snapshots_require_all_physical_ids_idle_and_unique(self):
        self.assertEqual(CHECK.gpu_roster(encoded(snapshots())), GPU_IDS)
        for field, bad in (("GPU use (%)", "1"), ("GPU Memory Allocated (VRAM%)", "1"),
                           ("GPU Memory Read/Write Activity (%)", "1"), ("Unique ID", hex(GPU_IDS[0]))):
            gpu = snapshots()
            gpu["card7"][field] = bad
            with self.assertRaises(ValueError):
                CHECK.gpu_roster(encoded(gpu))

    def test_file_entry_requires_success_complete_jsonl_and_all_bound_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            reference = encoded(synthetic_reference())
            files = {"status": b"0\n", "gpu-before.json": encoded(snapshots()), "gpu-after.json": encoded(snapshots()),
                     "results.jsonl": b"".join(encoded(row) for row in fixture()),
                     "workload.json": encoded(CHECK.expected_workload()), "reference.json": reference}
            for name, data in files.items():
                (root / name).write_bytes(data)
            def compare():
                return CHECK.compare(root, root / "workload.json", root / "reference.json", expected_hashes=PINS)
            with mock.patch.object(CHECK, "REFERENCE_SHA256", CHECK.sha256(reference)):
                report = compare()
                self.assertTrue(report["passed"])
                self.assertEqual(set(report["input_sha256"]), set(files))
                for name, bad in (("status", b"1\n"), ("status", b"0"),
                                  ("results.jsonl", files["results.jsonl"][:-1]),
                                  ("results.jsonl", files["results.jsonl"] + b"\n"),
                                  ("reference.json", reference + b" ")):
                    (root / name).write_bytes(bad)
                    with self.assertRaises(ValueError):
                        compare()
                    (root / name).write_bytes(files[name])

    def test_bounded_inputs_reject_symlink_growth_and_nonregular_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "input"
            source.write_bytes(b"abc")
            self.assertEqual(CHECK.read_bounded(source, 3), b"abc")
            with self.assertRaises(ValueError):
                CHECK.read_bounded(source, 2)
            link = root / "link"
            link.symlink_to(source)
            with self.assertRaises(OSError):
                CHECK.read_bounded(link, 10)
            fifo = root / "fifo"
            os.mkfifo(fifo)
            with self.assertRaises(ValueError):
                CHECK.read_bounded(fifo, 10)
            with mock.patch.object(CHECK.os, "read", side_effect=[b"abc", b"d"]) as reader:
                with self.assertRaisesRegex(ValueError, "grew beyond byte bound"):
                    CHECK.read_bounded(source, 3)
                self.assertEqual([call.args[1] for call in reader.call_args_list], [4, 1])


if __name__ == "__main__":
    unittest.main()
