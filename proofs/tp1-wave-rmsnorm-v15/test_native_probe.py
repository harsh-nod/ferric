"""Synthetic V15 report/closure tests; these never construct a device worker."""
import copy
import hashlib
import os
from pathlib import Path
import unittest
from unittest.mock import Mock, patch

import native_probe as native


class NativeProbeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixtures = native.load_fixture(Path(__file__).with_name("probe.py"))
        cls.core = cls.fixtures.load_helper(Path(os.environ["FERRIC_RMSNORM_HELPER"]))
        cls.artifact_sha, cls.worker_sha, cls.source_sha = "12" * 32, "34" * 32, "56" * 32
        cls.cases = [cls.fixtures.make_case(cls.core, spec) for spec in cls.fixtures.specifications()]
        cls.results = []
        for case in cls.cases:
            arguments = [dict(offset=offset, bytes=8, global_buffer=offset % 16 == 0,
                              access=None, pointee_alignment=None) for offset in range(0, 80, 8)]
            arguments.extend(dict(offset=offset, bytes=4, global_buffer=False,
                                  access=None, pointee_alignment=None) for offset in range(80, 96, 4))
            metadata = dict(explicit_arguments=arguments, group_segment_bytes=0, implicit_argument_bytes=256,
                implicit_argument_offset=96, kernarg_alignment=8, kernarg_bytes=352,
                object_sha256=list(bytes.fromhex(cls.artifact_sha)), private_segment_bytes=0,
                symbol=case["symbol"], wavefront_size=64)
            checks = [dict(name=buf["name"], access=buf["access"], bytes=len(buf["expected"]),
                           sha256=hashlib.sha256(buf["expected"]).hexdigest(),
                           guard_bytes_each_side=64, guards_unchanged=True) for buf in case["buffers"]]
            cls.results.append(dict(name=case["name"], symbol=case["symbol"], elapsed_ns=10,
                                    metadata=metadata, checks=checks))
        cls.report = dict(schema="FerricTp1WaveRmsnormProbeV15", authority="none", benchmark=False,
            performance_qualified=False, model_inference=False, model_parity_qualified=False,
            checks_pass=True, source_sha256=cls.source_sha, fixture_sha256=native.FIXTURE_SHA,
            helper_sha256=native.HELPER_SHA, worker_sha256=cls.worker_sha, artifact_sha256=cls.artifact_sha,
            runtime_operational=True, device_unique_id=101, worker_pid=123, worker_start_ticks=1,
            clean_teardown=True, timing_scope=native.TIMING_SCOPE, closed_roots=[cls.fixtures.ROOT],
            active_inputs_finite=True, results=cls.results)

    def validate(self, report):
        return native.validate_report(report, self.fixtures, self.core, self.artifact_sha,
                                      self.worker_sha, 101, self.source_sha)

    def test_exact_full_report(self):
        self.assertEqual(self.validate(self.report), 123)
        self.assertEqual(sum(len(row["checks"]) for row in self.results), 40)
        self.assertEqual(sum(check["bytes"] == 0 for row in self.results for check in row["checks"]), 16)

    def test_closed_report_identity_and_nonclaims(self):
        for key in self.report:
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.validate(dict(self.report, **{key: None}))
        for key, value in (("extra", True), ("device_unique_id", True), ("worker_pid", True),
                           ("worker_start_ticks", 0), ("clean_teardown", False),
                           ("performance_qualified", True), ("model_parity_qualified", True)):
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.validate(dict(self.report, **{key: value}))

    def test_roster_order_and_duplicates(self):
        for results in (self.results[:-1], list(reversed(self.results)), [self.results[0]] * 8):
            with self.subTest(roster=[row["name"] for row in results]), self.assertRaises(ValueError):
                self.validate(dict(self.report, results=results))

    def test_every_buffer_extent_hash_access_and_guard(self):
        for index in range(5):
            for key, value in (("sha256", "0" * 64), ("bytes", True), ("guards_unchanged", 1),
                               ("guard_bytes_each_side", 0), ("access", "invalid"), ("name", "wrong")):
                result = copy.deepcopy(self.results[0])
                result["checks"][index][key] = value
                with self.subTest(buffer=index, field=key), self.assertRaises(ValueError):
                    native.validate_result(result, self.cases[0], self.artifact_sha, self.core)

    def test_exact_abi_resources_and_argument_offsets(self):
        for key in self.results[0]["metadata"]:
            result = copy.deepcopy(self.results[0])
            result["metadata"][key] = None
            with self.subTest(field=key), self.assertRaises(ValueError):
                native.validate_result(result, self.cases[0], self.artifact_sha, self.core)
        for index in range(14):
            result = copy.deepcopy(self.results[0])
            result["metadata"]["explicit_arguments"][index]["offset"] += 4
            with self.subTest(argument=index), self.assertRaises(ValueError):
                native.validate_result(result, self.cases[0], self.artifact_sha, self.core)
        for key, value in (("global_buffer", 1), ("bytes", 16), ("access", "write"),
                           ("pointee_alignment", True), ("extra", 1)):
            result = copy.deepcopy(self.results[0])
            result["metadata"]["explicit_arguments"][0][key] = value
            with self.subTest(pointer=key), self.assertRaises(ValueError):
                native.validate_result(result, self.cases[0], self.artifact_sha, self.core)

    def test_argument_hints_match_read_and_write_source(self):
        result = copy.deepcopy(self.results[0])
        for index, buffer in enumerate(self.cases[0]["buffers"]):
            result["metadata"]["explicit_arguments"][index * 2].update(
                access=buffer["access"], pointee_alignment=2)
        native.validate_result(result, self.cases[0], self.artifact_sha, self.core)

    def test_case_result_has_no_extra_or_missing_fields(self):
        for key, value in (("elapsed_ns", True), ("checks", []), ("symbol", "other"),
                           ("name", "other"), ("extra", True)):
            with self.subTest(field=key), self.assertRaises(ValueError):
                native.validate_result(dict(self.results[0], **{key: value}), self.cases[0],
                                       self.artifact_sha, self.core)

    def test_run_closes_once_after_all_eight_checks(self):
        worker = Mock()
        worker.command.return_value = ({"op": "performance_configured"}, b"")
        with patch.object(self.core, "probe", side_effect=self.results) as probe:
            self.assertEqual(native.run_cases(worker, self.core, self.fixtures, b"image", self.artifact_sha),
                             self.results)
        self.assertEqual(probe.call_count, 8)
        worker.finish.assert_called_once_with()
        worker.abort.assert_not_called()
        worker.command.assert_called_once_with(dict(op="configure_performance", cache_kernel_admission=False,
            operational_currentness=True, profile=False), expected="performance_configured")

    def test_probe_or_close_failure_aborts_without_continuation(self):
        for failure in ("configuration", "dispatch", "mismatch", "close"):
            worker = Mock()
            worker.command.return_value = ({"op": "performance_configured"}, b"bad" if failure == "configuration" else b"")
            worker.finish.side_effect = ValueError("close") if failure == "close" else None
            rows = copy.deepcopy(self.results)
            if failure == "mismatch":
                rows[0]["checks"][4]["sha256"] = "0" * 64
            with patch.object(self.core, "probe", side_effect=ValueError("dispatch") if failure == "dispatch" else rows):
                with self.subTest(failure=failure), self.assertRaises(ValueError):
                    native.run_cases(worker, self.core, self.fixtures, b"image", self.artifact_sha)
            worker.abort.assert_called_once_with()
            self.assertEqual(worker.finish.call_count, int(failure == "close"))

    def test_cli_requires_explicit_run_before_loading(self):
        with patch.object(native, "load_fixture") as loader, self.assertRaises(SystemExit):
            native.main([])
        loader.assert_not_called()


if __name__ == "__main__":
    unittest.main()
