import copy
import unittest

import bind_evidence as binding


class BindingTests(unittest.TestCase):
    def test_fixed_wave_dispatch_binding_and_hostile_mutations(self):
        hashes = {key + "_sha256": str(number) * 64 for number, key in enumerate(
            ("probe", "worker", "output", "source", "object"))}
        record = {"input_sha256": "a" * 64, "checkpoint": {"tensor_sha256": "b" * 64}}
        artifact = {
            "schema": "ferric-qwen3-kproj-artifact-v1",
            "metadata": {"symbol": binding.SYMBOL, "wavefront_size": 64},
            "source_declared_abi": {"grid_work_items": [65536, 1, 1], "max_workgroups": [512, 1, 1]},
            "source_sha256": hashes["source_sha256"], "object_sha256": hashes["object_sha256"],
        }
        report = {
            "schema": "ferric-qwen3-kproj-probe-v1", "authority": "none", "artifact": artifact,
            "workgroup": [128, 1, 1], "grid_work_items": [65536, 1, 1], "completed_dispatches": 1,
            "input_immutability_and_all_allocation_guards_passed": True,
            "free_close_and_worker_exit_passed": True, "input_sha256": record["input_sha256"],
            "weights_sha256": record["checkpoint"]["tensor_sha256"], **hashes,
        }
        binding.validate_dispatch(report, artifact, record, hashes)
        for key, bad in [
            ("schema", "wrong"), ("authority", "production"), ("artifact", {}),
            ("workgroup", [64, 1, 1]), ("grid_work_items", [1024, 1, 1]),
            ("completed_dispatches", 2), ("input_immutability_and_all_allocation_guards_passed", False),
            ("free_close_and_worker_exit_passed", False), ("input_sha256", "x"),
            ("weights_sha256", "x"), ("probe_sha256", "x"), ("worker_sha256", "x"),
            ("output_sha256", "x"),
        ]:
            changed = copy.deepcopy(report)
            changed[key] = bad
            with self.subTest(field=key), self.assertRaises(ValueError):
                binding.validate_dispatch(changed, artifact, record, hashes)
        for mutation in range(6):
            changed = copy.deepcopy(artifact)
            if mutation == 0:
                changed["metadata"]["symbol"] = "ferric_gfx950_qwen3_kproj_v1"
            elif mutation == 1:
                changed["metadata"]["wavefront_size"] = 32
            elif mutation == 2:
                changed["source_declared_abi"]["grid_work_items"] = [1024, 1, 1]
            elif mutation == 3:
                changed["source_declared_abi"]["max_workgroups"] = [8, 1, 1]
            else:
                changed["source_sha256" if mutation == 4 else "object_sha256"] = "x"
            altered_report = dict(report, artifact=changed)
            with self.subTest(artifact_mutation=mutation), self.assertRaises(ValueError):
                binding.validate_dispatch(altered_report, changed, record, hashes)
