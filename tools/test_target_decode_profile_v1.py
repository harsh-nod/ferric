"""CPU-only adversarial fixtures for the independent target-profile validator."""

import copy
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import target_decode_profile_v1 as gate


def fixture(count=32, warmups=0, repetitions=1):
    ids = [12095, 13] + list(range(30))
    text = " Paris. independently fixed fixture text"
    source_pass = {"token_ids": ids, "argmax_token_ids": ids, "decoded_new_tokens": text}
    bundle = "6" * 64
    reference = {
        "format": "FERRIC-QWEN3-8B-GREEDY-REFERENCE-V1",
        "authority": "independent-offline-reference-only",
        "model": {
            "repository": "Qwen/Qwen3-8B", "revision": gate.REVISION,
            "class": "Qwen3ForCausalLM", "deployment_bundle_identity": bundle,
            "source_root": "/private/oracle/source",
            "config": {"hidden_size": 4096, "num_attention_heads": 32, "num_hidden_layers": 36,
                       "num_key_value_heads": 8, "vocab_size": 151936, "model_type": "qwen3",
                       "torch_dtype": "torch.bfloat16"},
        },
        "prompt": {"text": gate.PROMPT, "token_ids": gate.PROMPT_TOKENS, "add_special_tokens": False},
        "execution": {
            "attention_implementation": "sdpa", "device_dtype": "bfloat16", "do_sample": False,
            "max_new_tokens": 32, "model_forward": "transformers.generate", "network": "offline",
            "num_beams": 1, "use_cache": True, "passes": [source_pass, copy.deepcopy(source_pass)],
            "output_loading_info": {"error_msgs": [], "mismatched_keys": [], "missing_keys": [], "unexpected_keys": []},
        },
        "runtime": {"private_path": "/private/oracle/runtime"},
    }
    plan = {
        "schema": "FerricQwen3TargetDecodePlanV1", "authority": "none",
        **{key: str(index + 1) * 64 for index, key in enumerate(gate.HASH_FIELDS)},
        "model_bundle_id": bundle, "device_unique_id": 123456789012345,
        "reference_sha256": gate.REFERENCE_SHA256, "new_tokens": count,
        "warmup_runs": warmups, "repetitions": repetitions, "capacity": 64,
        "runtime_cache_admission": False, "runtime_operational": False, "runtime_profile": False,
    }
    expected_ids = ids if count == 32 else gate.PREFIX
    expected_bytes = text.encode() if count == 32 else gate.PREFIX_BYTES
    steps = count + 4
    setup = {
        "schema": "FerricQwen3TpEngineeringSetupV1", "authority": "none",
        **{key: plan[key] for key in gate.HASH_FIELDS},
        "model": "Qwen/Qwen3-8B", "dtype": "BF16", "target": "gfx950:xnack-",
        "tensor_parallel": 1, "device_unique_ids": [plan["device_unique_id"]], "worker_pids": [8888888],
        "running_worker_sha256": [plan["worker_sha256"]], "executable_identity": "live_proc_exe_sha256",
        "prompt": gate.PROMPT, "prompt_tokens": gate.PROMPT_TOKENS,
        **{key: plan[key] for key in ("new_tokens", "capacity", "repetitions", "warmup_runs")},
        "rank_zero_dispatch_budget": steps * gate.PACKETS * (warmups + repetitions),
        "conservative_ring_packet_limit": gate.RING_LIMIT, "model_intake_seconds": 2.0, "setup_seconds": 3.0,
        "collective": "host_staged_fp32_rank_order_reduce_bf16_residual",
        "prefill": "token_at_a_time_m1", "decoding": "greedy_lowest_id_fixed_length",
        "numerical_status": "Contracted; compare emitted token IDs independently",
        "timing": "monotonic controller clock; includes IPC, host collectives and per-token progress logging; excludes setup",
    }
    profile = {
        "schema": "FerricQwen3TargetDecodeProfileSetupV1", "authority": "none",
        "performance_qualified": False, "reference_sha256": gate.REFERENCE_SHA256,
        "reference_model_revision": gate.REVISION, "reference_device": "AMD Instinct MI300X",
        "reference_passes": 2, "reference_tokens_available": 32, "reference_tokens_consumed": count,
        "reference_expected_tokens": expected_ids, "reference_expected_utf8_bytes": list(expected_bytes),
        "precision": "BF16", "target_only": True, "speculation": False,
        "concurrent_requests": 1, "tensor_parallel": 1,
        **{key: plan[key] for key in gate.RUNTIME_FIELDS},
        "runtime_sequences": False, "runtime_ordered_batches": False,
        "runtime_rollover": False, "kv_prefix_cache": False,
        "sequence_reuse": "reset logical cursor; each request overwrites every consumed KV position",
        "timing": "host Instant; includes IPC, host collectives, progress logging and reference checks; not GPU timestamps",
        "nonclaim": "Reference-checked bounded diagnostic; not model-wide numerical qualification, GPU overlap, megakernel execution, or a controlled benchmark",
    }
    records = [setup, profile]
    whole = 3.0
    for ordinal in range(warmups + repetitions):
        warmup = ordinal < warmups
        index = ordinal if warmup else ordinal - warmups
        interval = 50.0 if warmup else 0.1 * (index + 1)
        intervals = [interval] * (count - 1)
        duration = 1.0 + math.fsum(intervals)
        whole += duration
        records.extend([
            {"schema": "FerricQwen3TpEngineeringMeasurementV1", "authority": "none",
             "run": index, "warmup": warmup, "world_size": 1, "prompt_tokens": gate.PROMPT_TOKENS,
             "generated_tokens": expected_ids, "generated_text": expected_bytes.decode(),
             "generated_utf8_bytes": list(expected_bytes), "ttft_seconds": 1.0,
             "tpot_seconds": interval, "decode_intervals_seconds": intervals,
             "generation_seconds": duration, "rank_dispatch_counts": [steps * gate.PACKETS],
             "kv_tokens_processed": steps},
            {"schema": "FerricQwen3TargetDecodeProfileRunV1", "authority": "none",
             "run": index, "warmup": warmup, "reference_passed": True, "performance_qualified": False,
             "generated_tokens_checked": count, "generated_utf8_bytes_checked": len(expected_bytes),
             "kv_tokens_processed": steps},
        ])
    records.append({"schema": "FerricQwen3TpEngineeringClosedV1", "authority": "none",
                    "worker_pids": [8888888], "all_workers_exited": True, "whole_seconds": whole + 1.0})
    return copy.deepcopy((records, reference, plan))


class TargetProfileTests(unittest.TestCase):
    def test_full32_checks_31_decode_intervals_and_reference_bytes(self):
        records, reference, plan = fixture()
        report = gate.validate(records, reference, plan)
        self.assertEqual(report["summary"]["measured_decode_intervals"], 31)
        self.assertAlmostEqual(report["summary"]["post_first_tokens_per_second"], 10.0)
        self.assertEqual(report["total_dispatches"], 19584)
        self.assertFalse(report["performance_qualified"])
        self.assertFalse(report["gpu_timestamps"])

    def test_warmups_are_validated_but_excluded_from_pooled_intervals(self):
        records, reference, plan = fixture(warmups=2, repetitions=3)
        report = gate.validate(records, reference, plan)
        self.assertEqual(report["summary"]["measured_decode_intervals"], 93)
        self.assertAlmostEqual(report["summary"]["post_first_tokens_per_second"], 5.0)
        self.assertAlmostEqual(report["summary"]["p95_interval_seconds"], 0.3)
        records[2]["generated_tokens"][0] = 0
        with self.assertRaises(ValueError):
            gate.validate(records, reference, plan)

    def test_two_token_40_request_budget_is_exact(self):
        report = gate.validate(*fixture(count=2, warmups=10, repetitions=30))
        self.assertEqual(report["total_dispatches"], 130560)
        self.assertEqual(report["summary"]["measured_decode_intervals"], 30)
        self.assertEqual(report["expected_utf8_bytes"], list(b" Paris."))

    def test_plan_identity_runtime_and_workload_drift_rejected(self):
        changes = {**{key: "f" * 64 for key in gate.HASH_FIELDS},
                   "device_unique_id": 9, "reference_sha256": "f" * 64,
                   "runtime_cache_admission": True, "runtime_operational": True, "runtime_profile": True,
                   "new_tokens": 2, "warmup_runs": 1, "repetitions": 2, "capacity": 128}
        for key, value in changes.items():
            with self.subTest(key=key):
                records, reference, plan = fixture()
                plan[key] = value
                with self.assertRaises(ValueError):
                    gate.validate(records, reference, plan)

    def test_plan_bounds_and_bool_as_integer_are_rejected(self):
        for key, value in [("new_tokens", 4), ("repetitions", 0), ("repetitions", 31),
                           ("warmup_runs", 11), ("capacity", 35), ("capacity", 8193),
                           ("device_unique_id", True), ("runtime_profile", 1), ("repetitions", 7)]:
            records, reference, plan = fixture()
            plan[key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                gate.validate(records, reference, plan)

    def test_private_u64_survives_json_above_double_precision(self):
        records, reference, plan = fixture()
        exact_u64 = (1 << 64) - 123
        plan["device_unique_id"] = exact_u64
        records[0]["device_unique_ids"] = [exact_u64]
        plan = gate.decode_json(json.dumps(plan).encode())
        records = gate.decode_json(json.dumps(records).encode())
        self.assertEqual(plan["device_unique_id"], exact_u64)
        self.assertTrue(gate.validate(records, reference, plan)["all_reference_checks_passed"])
        for value in [float(exact_u64), int(float(exact_u64))]:
            mutated = copy.deepcopy(plan)
            mutated["device_unique_id"] = value
            with self.assertRaises(ValueError):
                gate.validate(records, reference, mutated)
        records[0]["device_unique_ids"] = [float(exact_u64)]
        with self.assertRaises(ValueError):
            gate.validate(records, reference, plan)

    def test_each_record_has_closed_keys_schema_and_authority(self):
        for index in range(5):
            for change in ("extra", "missing", "schema", "authority"):
                records, reference, plan = fixture()
                if change == "extra":
                    records[index]["private_path"] = "/do/not/publish"
                elif change == "missing":
                    del records[index]["authority"]
                else:
                    records[index][change] = "other"
                with self.subTest(index=index, change=change), self.assertRaises(ValueError):
                    gate.validate(records, reference, plan)

    def test_record_count_order_run_indices_and_warmups_are_exact(self):
        records, reference, plan = fixture(warmups=1, repetitions=1)
        for mutated in [records[:-1], records + [records[-1]], records[1:] + records[:1]]:
            with self.assertRaises(ValueError):
                gate.validate(mutated, reference, plan)
        for index, key, value in [(2, "run", 1), (3, "run", 1), (4, "warmup", True),
                                  (5, "warmup", True), (2, "run", False)]:
            mutated = copy.deepcopy(records)
            mutated[index][key] = value
            with self.assertRaises(ValueError):
                gate.validate(mutated, reference, plan)

    def test_output_dispatch_cursor_and_pass_markers_cannot_be_forged(self):
        for index, key, value in [
            (2, "generated_tokens", [0] * 32), (2, "generated_utf8_bytes", list(b"wrong")),
            (2, "generated_text", "wrong"), (2, "rank_dispatch_counts", [19585]),
            (2, "kv_tokens_processed", 37), (3, "kv_tokens_processed", 37),
            (3, "generated_tokens_checked", 31), (3, "generated_utf8_bytes_checked", 0),
            (3, "reference_passed", False), (3, "reference_passed", 1),
            (3, "performance_qualified", True), (1, "reference_expected_tokens", [0] * 32),
            (1, "reference_expected_utf8_bytes", list(b"wrong")),
        ]:
            records, reference, plan = fixture()
            records[index][key] = value
            with self.subTest(index=index, key=key), self.assertRaises(ValueError):
                gate.validate(records, reference, plan)

    def test_time_values_are_finite_positive_and_algebraically_coherent(self):
        for value in [0, -1, float("nan"), float("inf"), True, "1"]:
            for index, field in [(0, "setup_seconds"), (0, "model_intake_seconds"),
                                 (2, "ttft_seconds"), (2, "tpot_seconds"),
                                 (2, "generation_seconds"), (4, "whole_seconds")]:
                records, reference, plan = fixture()
                records[index][field] = value
                with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                    gate.validate(records, reference, plan)
        for index, field, value in [(0, "model_intake_seconds", 4.0), (2, "tpot_seconds", 2.0),
                                     (2, "generation_seconds", 2.0), (4, "whole_seconds", 1.0),
                                     (2, "decode_intervals_seconds", [0.1] * 32),
                                     (2, "decode_intervals_seconds", [float("nan")] * 31)]:
            records, reference, plan = fixture()
            records[index][field] = value
            with self.assertRaises(ValueError):
                gate.validate(records, reference, plan)

    def test_subnormal_intervals_cannot_create_nonfinite_derived_rate(self):
        records, reference, plan = fixture()
        records[2]["decode_intervals_seconds"] = [5e-324] * 31
        records[2]["tpot_seconds"] = 5e-324
        records[2]["generation_seconds"] = records[2]["ttft_seconds"]
        with self.assertRaisesRegex(ValueError, "derived post-first token rate"):
            gate.validate(records, reference, plan)

    def test_live_worker_and_close_rosters_are_exact(self):
        for index, key, value in [(0, "running_worker_sha256", ["f" * 64]),
                                  (0, "worker_pids", [True]), (0, "device_unique_ids", [True]),
                                  (4, "worker_pids", [9]), (4, "all_workers_exited", False),
                                  (4, "all_workers_exited", 1)]:
            records, reference, plan = fixture()
            records[index][key] = value
            with self.assertRaises(ValueError):
                gate.validate(records, reference, plan)

    def test_reference_role_dtype_prompt_and_two_pass_agreement(self):
        mutations = [
            ("model", "repository", "other"), ("model", "revision", "f" * 40),
            ("prompt", "text", "other"), ("prompt", "add_special_tokens", True),
            ("execution", "device_dtype", "float8"), ("execution", "num_beams", True),
            ("execution", "do_sample", True), ("execution", "use_cache", False),
        ]
        for section, key, value in mutations:
            records, reference, plan = fixture()
            reference[section][key] = value
            with self.assertRaises(ValueError):
                gate.validate(records, reference, plan)
        for key in ("token_ids", "argmax_token_ids", "decoded_new_tokens"):
            records, reference, plan = fixture()
            reference["execution"]["passes"][1][key] = "wrong" if key == "decoded_new_tokens" else [0] * 32
            with self.assertRaises(ValueError):
                gate.validate(records, reference, plan)

    def test_public_report_whitelist_never_embeds_private_input_records(self):
        records, reference, plan = fixture()
        encoded = json.dumps(gate.validate(records, reference, plan), sort_keys=True)
        for private in ["123456789012345", "8888888", "/private/", "device_unique_id", "worker_pids",
                        "source_root", "private_path", '"setup":', '"plan":', '"closed":']:
            self.assertNotIn(private, encoded)

    def test_json_decoder_rejects_duplicates_nonfinite_values_and_invalid_utf8(self):
        for data in [b'{"a":1,"a":2}', b'{"x":NaN}', b'{"x":Infinity}', b'"\xff"']:
            with self.assertRaises((ValueError, UnicodeError)):
                gate.decode_json(data)
        records, _, _ = fixture()
        encoded = b"\n".join(json.dumps(record).encode() for record in records) + b"\n"
        self.assertEqual(len(gate.records_from_bytes(encoded)), 5)
        for data in [encoded[:-1], b"\n" + encoded, encoded + b"\n", b"{}\n", b"x" * (gate.MAX_BYTES + 1)]:
            with self.assertRaises(ValueError):
                gate.records_from_bytes(data)

    def test_reader_rejects_symlinks_nonfiles_empty_and_oversized_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "input"
            source.write_bytes(b"abc")
            self.assertEqual(gate.read_bounded(source, 3), b"abc")
            link = root / "link"
            link.symlink_to(source)
            for path, limit in [(link, 3), (source, 2), (root, 100)]:
                with self.assertRaises((ValueError, OSError)):
                    gate.read_bounded(path, limit)
            source.write_bytes(b"")
            with self.assertRaises(ValueError):
                gate.read_bounded(source, 3)

    def test_cli_rejects_nonzero_status_or_unpinned_reference_without_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            status = root / "status"
            reference = root / "reference"
            reference.write_bytes(b"{}\n")
            output = root / "report.json"
            command = [sys.executable, "-B", gate.__file__, "--capture", str(root / "missing-capture"),
                       "--status", str(status), "--reference", str(reference),
                       "--plan", str(root / "missing-plan"), "--output", str(output)]
            for status_bytes, expected_error in [(b"1\n", "capture process failed"),
                                                  (b"0\n", "frozen reference file identity")]:
                status.write_bytes(status_bytes)
                result = subprocess.run(command, capture_output=True, text=True, timeout=10, check=False)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertIn(expected_error, result.stderr)
                self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
