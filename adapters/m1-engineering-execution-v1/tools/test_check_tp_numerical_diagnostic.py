"""Host-synthetic trace/custody tests, not GPU or numerical qualification."""

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_tp_numerical_diagnostic as D
import test_compare_tp_batch as F


class ToyReplay:
    """Isolate trace binding from the separately tested real payload replayer."""

    @staticmethod
    def load_capture(directory, digest):
        raw = D.CHECK.read_bounded(directory / "manifest.json", 1024 * 1024)
        D.exact(D.CHECK.sha256(raw), digest, "toy manifest hash")
        return D.CHECK.json_value(raw), {}, digest, len(raw)

    @staticmethod
    def analyze(_manifest, _payloads):
        return {}


def fixture(root, wrong_token=False):
    profile = dict(F.PROFILE, runtime_operational=True)
    rows = F.extended_fixture(1, True, False, D.CHECK.LEGACY_COLLECTIVE, profile)
    if wrong_token:
        for record in rows:
            if record["schema"] == "FerricQwen3TpBatchCompletedV2":
                for output in record["outputs"]:
                    if output["slot"] == 0 and output["generation"] == 1 and output["index"] == 0:
                        output["token"] = 9856
                for row in record["rows"]:
                    if row["slot"] == 0 and row["generation"] == 1 and row["kind"] == "Decode":
                        row["token"] = 9856
            elif record.get("name") == "seed-prefix" and record["schema"] == "FerricQwen3TpBatchRequestV2":
                record["generated_tokens"][0] = 9856
                record["generated_text"] = " Germany is"
                record["generated_utf8_bytes"] = list(b" Germany is")
    setup = rows[0]
    reference, workload = F.encoded(F.synthetic_reference()), F.encoded(D.CHECK.expected_workload())
    expected = {key: setup[key] for key in D.CHECK.IDENTITY_FIELDS}
    expected.update(schema="FerricTpNumericalDiagnosticExpectationV1", workload_sha256=D.CHECK.sha256(workload),
                    reference_sha256=D.CHECK.sha256(reference), prefix_cache=True, output_head_pruning=False,
                    performance_profile=profile, collective=None, physical_gpu_ids=F.GPU_IDS,
                    selection={"batch_ordinal": 2, "layer": 0, "role": "Query"})
    completed = [value for value in rows if value["schema"] == "FerricQwen3TpBatchCompletedV2"]
    selected = completed[1]
    row_map = [{**row, "execution_row": index, "source_row": index,
                "publishable": row["kind"] != "PrefillIntermediate"} for index, row in enumerate(selected["rows"])]
    events = {(event["slot"], event["generation"]): event for event in selected["outputs"]}
    identity = {key: setup[key] for key in D.CHECK.IDENTITY_FIELDS | {"model_bundle_id", "session_id", "tensor_parallel",
               "running_worker_sha256", "batch_tokens", "prefill_chunk", "prefix_cache", "output_head_pruning"}}
    identity.update(requests_sha256=expected["workload_sha256"], device_unique_id=setup["device_unique_ids"][0],
                    projection="baseline", runtime_operational=True, runtime_cache_admission=False,
                    queue_rollover=False, collective="host-staged-v1")
    def descriptor(name, size):
        return {"file": name, "bytes": size, "sha256": "a" * 64,
                "buffer_id": 1, "buffer_offset": 0, "element_bytes": 2}
    manifest = {"schema": "FerricTpNumericalCaptureV1", "authority": "none", "complete": True,
                "model_parity_qualified": False, "performance_qualified": False,
                "timing_scope": "diagnostic readbacks invalidate all performance measurements for this run",
                "identity": identity, "batch_ordinal": 2, "scheduler_batch_id": 2, "pool_batch_id": 2,
                "execution_rows": row_map, "projection": {"layer": 0, "role": "Query", "kernel": D.kernel_name("baseline"),
                    "rows": len(row_map), "n": 4096, "k": 4096, "world": 1, "tag": 1,
                    "input": descriptor("projection-input.bf16", len(row_map) * 4096 * 2),
                    "weights_nk": descriptor("projection-weights-nk.bf16", 4096 * 4096 * 2),
                    "actual_weights": descriptor("projection-weights-nk.bf16", 4096 * 4096 * 2),
                    "actual_weight_layout": "nk", "output": descriptor("projection-output.bf16", len(row_map) * 4096 * 2)},
                "head": {"rows": len(row_map), "kernel": D.kernel_name("baseline"),
                    "input": descriptor("head-input.bf16", len(row_map) * 4096 * 2),
                    "logits": descriptor("head-logits.bf16", len(row_map) * 151936 * 2),
                    "watch_label_scope": "token IDs only; no tokenizer decoding assumed", "request_rows": [
                    {"row_identity": row, "gpu_choice": events[(row["slot"], row["generation"])]["token"]
                     if row["publishable"] else 0} for row in row_map]},
                "payload_bytes_before_manifest": 0, "maximum_total_bytes": 224 * 1024 * 1024}
    capture = root / "capture"
    capture.mkdir()
    rows[0]["numerical_capture"] = {"schema": "FerricTpNumericalSelectionV1", **expected["selection"],
                                    "directory": "/original/private/capture", "performance_qualified": False}
    rows[0]["numerical_status"] = D.DIAGNOSTIC_STATUS
    for name, data in {"status": b"0\n", "gpu-before.json": F.encoded(F.snapshots()),
                       "gpu-after.json": F.encoded(F.snapshots()), "workload.json": workload,
                       "reference.json": reference}.items():
        (root / name).write_bytes(data)
    return rows, manifest, expected


def write_case(root, rows, manifest):
    raw = F.encoded(manifest)
    digest = D.CHECK.sha256(raw)
    (root / "capture/manifest.json").write_bytes(raw)
    rows[-1]["numerical_capture"] = {"schema": "FerricTpNumericalCaptureReceiptV1", "authority": "none",
        "performance_qualified": False, "manifest": {"file": "manifest.json", "bytes": len(raw), "sha256": digest},
        "total_bytes": len(raw)}
    (root / "results.jsonl").write_bytes(b"".join(F.encoded(row) for row in rows))
    return digest


class DiagnosticTests(unittest.TestCase):
    def diagnose(self, root, expected, digest):
        with mock.patch.object(D.CHECK, "REFERENCE_SHA256", expected["reference_sha256"]), \
                mock.patch.object(D, "validate_head"):
            return D.diagnose(root, root / "capture", root / "workload.json", root / "reference.json",
                              expected, digest, ToyReplay)

    def test_exact_tokens_and_wrong_tokens_have_separate_custody_and_parity(self):
        for wrong in (False, True):
            with self.subTest(wrong=wrong), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                rows, manifest, expected = fixture(root, wrong)
                digest = write_case(root, rows, manifest)
                raw_before = (root / "results.jsonl").read_bytes()
                result = self.diagnose(root, expected, digest)
                self.assertTrue(result["capture_custody_validated"])
                self.assertTrue(result["complete_observed_trace_validated"])
                self.assertEqual(result["fixed_reference_passed"], not wrong)
                self.assertFalse(result["performance_qualified"])
                self.assertEqual(result["requests"][0]["exact_token_ids_match"], not wrong)
                self.assertEqual(raw_before, (root / "results.jsonl").read_bytes())
                self.assertFalse(set(result) & {"metrics", "setup_seconds", "whole_seconds", "ttft_ns", "tpot_ns"})
                self.assertFalse(any(path.name.endswith("comparison.json") for path in root.iterdir()))

    def test_closed_receipt_hash_selection_row_and_extra_field_faults_reject(self):
        mutations = (
            lambda rows, manifest: rows[-1].update(all_workers_exited=False),
            lambda rows, manifest: rows[-1].update(rank_dispatch_counts=[1]),
            lambda rows, manifest: rows[-1]["numerical_capture"]["manifest"].update(sha256="a" * 64),
            lambda rows, manifest: rows[-1]["numerical_capture"].update(total_bytes=1),
            lambda rows, manifest: rows[-1].pop("numerical_capture"),
            lambda rows, manifest: rows[0]["numerical_capture"].update(batch_ordinal=3),
            lambda rows, manifest: rows[0].update(extra=True),
            lambda rows, manifest: rows[-1].update(extra=True),
            lambda rows, manifest: rows[2].update(extra=True),
        )
        for index, mutate in enumerate(mutations):
            with self.subTest(mutation=index), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                rows, manifest, expected = fixture(root)
                digest = write_case(root, rows, manifest)
                mutate(rows, manifest)
                (root / "results.jsonl").write_bytes(b"".join(F.encoded(row) for row in rows))
                with self.assertRaises((ValueError, KeyError)):
                    self.diagnose(root, expected, digest)
        for mutate in (
                lambda value: value.update(scheduler_batch_id=3),
                lambda value: value["execution_rows"][0].update(source_row=1),
                lambda value: value["execution_rows"][0].update(token=1),
                lambda value: value["identity"].update(controller_sha256="f" * 64),
                lambda value: value["head"]["request_rows"][-1].update(gpu_choice=1),
                lambda value: value["projection"].update(kernel=D.kernel_name("mfma")),
                lambda value: value["head"].update(kernel=D.kernel_name("baseline", True)),
                lambda value: value["projection"]["output"].update(element_bytes=4),
                lambda value: value["head"]["logits"].update(bytes=2),
                lambda value: value.update(timing_scope="performance"),
                lambda value: value["head"].update(watch_label_scope="invented decoded labels"),
                lambda value: value.update(extra=True)):
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                rows, manifest, expected = fixture(root)
                mutate(manifest)
                digest = write_case(root, rows, manifest)
                with self.assertRaises(ValueError):
                    self.diagnose(root, expected, digest)

    def test_truncation_nonzero_status_busy_gpu_and_bad_external_pins_reject(self):
        for name, data in (("status", b"1\n"), ("results.jsonl", b"{}"), ("gpu-after.json", b"{}\n")):
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                rows, manifest, expected = fixture(root)
                digest = write_case(root, rows, manifest)
                (root / name).write_bytes(data)
                with self.assertRaises(ValueError):
                    self.diagnose(root, expected, digest)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            rows, manifest, expected = fixture(root)
            digest = write_case(root, rows, manifest)
            for key in D.CHECK.IDENTITY_FIELDS | {"workload_sha256", "reference_sha256"}:
                changed = copy.deepcopy(expected)
                changed[key] = "f" * 64
                with self.subTest(key=key), self.assertRaises(ValueError):
                    self.diagnose(root, changed, digest)

    def test_full_head_payload_recheck_enforces_ties_finite_logits_and_summary_types(self):
        selected = list(range(16)) + [9856, 17689]
        entry = lambda token: {"token": token, "bf16_bits": 0, "value": 0.0}
        summary = {"row_identity": {}, "gpu_choice": 0, "top16": [entry(token) for token in range(16)],
                   "top1_minus_top2": 0.0, "top_two_tied": True,
                   "tie_policy": "lowest token ID among equal finite BF16 logits",
                   "watch": [entry(9856), entry(17689)]}
        weights = {"file": "head-selected-weights-nk.bf16", "bytes": len(selected) * 8192, "sha256": "a" * 64}
        head = {"kernel": D.kernel_name("baseline"), "rows": 1, "n": 151936, "k": 4096,
                "input": {"file": "input"}, "logits": {"file": "logits"}, "request_rows": [summary],
                "selected_weight_tokens": selected, "selected_weights_nk": weights,
                "original_weight_buffer_id": 1, "original_weight_buffer_offset": 0,
                "watch_tokens": [9856, 17689], "watch_label_scope": "token IDs only; no tokenizer decoding assumed"}
        payloads = {"input": bytes(8192), "logits": bytes(151936 * 2), weights["file"]: bytes(weights["bytes"])}
        D.validate_head({"head": head}, payloads)
        for mutate in (lambda value: value["request_rows"][0].update(gpu_choice=1),
                       lambda value: value["request_rows"][0]["top16"][0].update(bf16_bits=False),
                       lambda value: value["request_rows"][0]["watch"][0].update(value=False),
                       lambda value: value.update(selected_weight_tokens=selected[:-1])):
            changed = copy.deepcopy(head)
            mutate(changed)
            with self.assertRaises(ValueError):
                D.validate_head({"head": changed}, payloads)
        changed_payloads = dict(payloads, logits=b"\x80\x7f" + payloads["logits"][2:])
        with self.assertRaises(ValueError):
            D.validate_head({"head": head}, changed_payloads)


if __name__ == "__main__":
    unittest.main()
