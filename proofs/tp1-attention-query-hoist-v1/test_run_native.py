"""Real-shaped synthetic reports from frozen fixture bytes, never native evidence."""
import copy
import hashlib
import os
from pathlib import Path
import unittest
from unittest.mock import patch

import probe
import run_native as wrapper


class ReportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        proofs = Path(__file__).resolve().parents[1]
        fixture_path = Path(os.environ.get("FERRIC_ATTENTION_FIXTURES",
            proofs / "tp1-attention-kernels-v1/probe.py"))
        helper_path = Path(os.environ.get("FERRIC_ATTENTION_HELPER",
            proofs / "tensor-parallel-kernels-v1/probe.py"))
        cls.fixtures = probe.load_fixture(fixture_path)
        cls.core = cls.fixtures.load_helper(helper_path)
        cls.checker = wrapper.load_pinned(Path(os.environ["FERRIC_PAIRED_CHECKER"]),
                                          wrapper.CHECKER_SHA, "query_hoist_frozen_checker")
        cls.specs = tuple(probe.specifications())
        cls.cases = {spec: probe.make_case(cls.core, cls.fixtures, spec) for spec in cls.specs}
        results = []
        for spec in cls.specs:
            case = cls.cases[spec]
            image_sha = probe.IMAGES[spec[2]]["sha256"]
            arguments = [dict(offset=offset, bytes=8, global_buffer=offset % 16 == 0,
                              access=None, pointee_alignment=None) for offset in range(0, 96, 8)]
            arguments.extend(dict(offset=offset, bytes=4, global_buffer=False,
                                  access=None, pointee_alignment=None) for offset in range(96, 116, 4))
            metadata = dict(explicit_arguments=arguments, group_segment_bytes=0, implicit_argument_bytes=256,
                implicit_argument_offset=120, kernarg_alignment=8, kernarg_bytes=376,
                object_sha256=list(bytes.fromhex(image_sha)), private_segment_bytes=0,
                symbol=case["symbol"], wavefront_size=64)
            checks = [dict(name=buf["name"], access=buf["access"], bytes=len(buf["expected"]),
                           sha256=hashlib.sha256(buf["expected"]).hexdigest(),
                           guard_bytes_each_side=64, guards_unchanged=True) for buf in case["buffers"]]
            results.append(dict(name=case["name"], symbol=case["symbol"], role=spec[2], artifact_sha256=image_sha,
                                elapsed_ns=10, metadata=metadata, checks=checks))
        cls.report = dict(schema="FerricTp1AttentionQueryHoistProbeV1", authority="none", benchmark=False,
            performance_qualified=False, model_inference=False, model_parity_qualified=False,
            same_compiler_ablation=False, checks_pass=True, source_sha256=wrapper.PROBE_SHA,
            fixture_sha256=probe.FIXTURE_SHA, helper_sha256=probe.HELPER_SHA,
            worker_sha256=cls.checker.WORKER_SHA, images=probe.image_provenance(), runtime_operational=True,
            device_unique_id=101, worker_pid=123, worker_start_ticks=1, clean_teardown=True,
            timing_scope=probe.TIMING_SCOPE, closed_roots=list(probe.ROOTS), active_inputs_finite=True,
            results=results)

    def validate(self, report):
        # Cache actual frozen-generator buffers; retain the real case validator.
        with patch.object(probe, "make_case", side_effect=lambda _core, _fixtures, spec: self.cases[spec]):
            return wrapper.validate(report, probe, self.fixtures, self.core, self.checker, 101)

    def test_complete_real_shaped_synthetic_report(self):
        self.assertEqual(hashlib.sha256(Path(probe.__file__).read_bytes()).hexdigest(), wrapper.PROBE_SHA)
        self.assertEqual(self.validate(self.report), 123)
        self.assertEqual(sum(len(case["checks"]) for case in self.report["results"]), 48)
        self.assertEqual([case["checks"][-1]["bytes"] for case in self.report["results"]], [262144] * 8)

    def test_closed_top_identity_and_nonclaim_fields(self):
        for key in self.report:
            changed = copy.deepcopy(self.report)
            changed[key] = None
            with self.subTest(key=key), self.assertRaises((ValueError, TypeError)):
                self.validate(changed)
        for key, value in (("extra", True), ("worker_pid", True), ("worker_start_ticks", 0),
                           ("clean_teardown", False), ("performance_qualified", True),
                           ("same_compiler_ablation", True), ("device_unique_id", True)):
            changed = dict(self.report, **{key: value})
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                self.validate(changed)

    def test_case_order_role_and_image_are_independently_bound(self):
        for mutation in ("missing", "duplicate", "order", "role", "image", "symbol", "metadata_image", "latency_bool"):
            changed = copy.deepcopy(self.report)
            rows = changed["results"]
            if mutation == "missing":
                rows.pop()
            elif mutation == "duplicate":
                rows[0] = rows[1]
            elif mutation == "order":
                rows.reverse()
            elif mutation == "metadata_image":
                rows[0]["metadata"]["object_sha256"] = rows[1]["metadata"]["object_sha256"]
            else:
                key = {"role": "role", "image": "artifact_sha256", "symbol": "symbol", "latency_bool": "elapsed_ns"}[mutation]
                rows[0][key] = True if mutation == "latency_bool" else rows[1][key]
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.validate(changed)

    def test_all_inputs_output_tail_and_guards_are_bound(self):
        for index in range(6):
            for key, value in (("sha256", "0" * 64), ("bytes", 1), ("guards_unchanged", False),
                               ("guard_bytes_each_side", 0), ("access", "invalid")):
                changed = copy.deepcopy(self.report)
                changed["results"][0]["checks"][index][key] = value
                with self.subTest(buffer=index, key=key), self.assertRaises(ValueError):
                    self.validate(changed)

    def test_complete_abi_resource_and_slice_argument_contract(self):
        for key in self.report["results"][0]["metadata"]:
            changed = copy.deepcopy(self.report)
            changed["results"][0]["metadata"][key] = None
            with self.subTest(key=key), self.assertRaises((ValueError, TypeError)):
                self.validate(changed)
        for index in range(17):
            changed = copy.deepcopy(self.report)
            changed["results"][0]["metadata"]["explicit_arguments"][index]["offset"] += 4
            with self.subTest(argument=index), self.assertRaises(ValueError):
                self.validate(changed)
        for key, value in (("global_buffer", 1), ("bytes", 16), ("access", "write"), ("pointee_alignment", 8)):
            changed = copy.deepcopy(self.report)
            changed["results"][0]["metadata"]["explicit_arguments"][0][key] = value
            with self.subTest(pointer=key), self.assertRaises(ValueError):
                self.validate(changed)

    def test_observation_roots_images_handoffs_and_grants(self):
        for image in probe.IMAGES.values():
            manifest = dict(schema="EngineeringHsacoObservationV1", authority="none",
                hsaco=dict(identity=dict(sha256=image["sha256"]), kernel_names=[image["root"]]),
                compiler_handoff=dict(sha256=image["handoff_sha256"]),
                execution=dict(exact_output_replay=True), grants=dict(publication=False, load=False, launch=False))
            wrapper.validate_observation(manifest, image, self.checker)
            for mutation in ("image", "root", "handoff", "replay", "grant"):
                changed = copy.deepcopy(manifest)
                if mutation == "image":
                    changed["hsaco"]["identity"]["sha256"] = "0" * 64
                elif mutation == "root":
                    changed["hsaco"]["kernel_names"] = []
                elif mutation == "handoff":
                    changed["compiler_handoff"]["sha256"] = "0" * 64
                elif mutation == "replay":
                    changed["execution"]["exact_output_replay"] = 1
                else:
                    changed["grants"]["launch"] = True
                with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                    wrapper.validate_observation(changed, image, self.checker)

    def test_unbound_launch_and_forced_or_failed_exits_rejected(self):
        with patch.object(wrapper, "load_pinned") as loader:
            with self.assertRaisesRegex(ValueError, "binding pending"):
                wrapper.main(["--tag", "synthetic", "--runner-sha256", "0" * 64])
            loader.assert_not_called()
        wrapper.require_normal_exit(0, False)
        for status, forced in ((True, False), (1, False), (-9, False), (0, True), (0, 0), (0, None)):
            with self.subTest(status=status, forced=forced), self.assertRaises(ValueError):
                wrapper.require_normal_exit(status, forced)


if __name__ == "__main__":
    unittest.main()
