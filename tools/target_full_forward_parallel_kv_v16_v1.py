#!/usr/bin/env python3
"""Strict parallel KV-copy comparison on a matched full-forward stack.

Both paths execute the same 616 packets per forward and pinned 32-token
reference. One bounded forward command is not a persistent GPU megakernel.
"""

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import stat
import types


CORE_SHA256 = "90cd589fe9b98660f6efb3400775cd269af236688706f37eaedee2d12e92b975"
WORKER_SHA256 = "05503764710e8add6ee15fbb1f8526ed9bdddfce7dd670acd7c604f9491490a7"
SOURCE_PINS = {
    "controller_source_manifest_sha256": "6bc0ed1f8bf732cb48772f8e04a2f801960c222247366732b7f1f60eadcde1e2",
    "controller_parent_source_manifest_sha256": "2489bee6d4237eeb5873487c96c67c37d0c44d91051b707b07dbfed5a87c3d6c",
    "controller_base_source_manifest_sha256": "07a696d0dfd8ad14eb808ce5d58ad817a487b2f1352cb8401fabbc09fa818c70",
    "worker_source_manifest_sha256": "b4de3efc58249ab345c89c2d5d423c5c78c32d348b4d5c5c8f67e49213136fb3",
    "paired_source_manifest_sha256": "b27b9cffc159ee82667d039056aa6a414569cc1309e25a5dca442279aba59fb0",
    "codegen_catalog_sha256": "d4a1e53ed7d8ca8bd0d29dded44192e6cbeac2b825174d9faa391fd414af99e6",
    "argmax_source_manifest_sha256": "1556bf8bdba72646a62da57c80b2676495b0e6b5655896bd22a43c4c878c3384",
    "argmax_original_source_manifest_sha256": "4ff99114c52e0a9313a3ae92422566636818bf41be57fd379c007ad0e220fe87",
    "argmax_shared_source_manifest_sha256": "165e1f3fa2e87a1f99e34afa5a1b86ab0b8e9463d049adbc38718aada8d19d9c",
    "rmsnorm_source_manifest_sha256": "1a8fbbcb2dbb05d74b9108634bd235d537cdba817b9126484ae15bbb1fccd537",
    "parallel_kv_source_manifest_sha256": "d24c70959d194d973d001e341acb78bf905857ad914a780278bf27a7a1911b66",
}
# Native-v4 release, with separate ordinary CPU admission for both modes.
ARGMAX_ARTIFACT = {
    "artifact_hsaco_id": "86c3ee4cead26f6432ef590434b3335e1ee2a95c9c900cf6315dca4ef0a542b0",
    "artifact_manifest_id": "8ab01a5a2913a76b09912cb39c831d714fb1c03cb1d0ad197e0b8d63e6ab8118",
    "artifact_handoff_id": "d7aa3543e0b58bcfcbd181b6702934f417b8b8e55847745f980a09dffce32e98",
}
ARGMAX_CANONICAL_DESCRIPTOR_SHA256 = "2e83fee4aeb6c5c014d78dc1536bf6aa2d96606e958cdfeb6e3fa7f1e7bc4629"
ARGMAX_ADMISSION_SHA256 = "8d574bd5d838c4f83e7f57b8855e9bee2ded5745db0aea8c7624f4ba1ac9af14"
ARGMAX_ADMISSION_PROVENANCE_SHA256 = "25c151cb56dbe3412345787d2bae89172141558b04663f3e4008d46a68f92dbc"
ARGMAX_SETUP = {"fp32_argmax", "fp32_argmax_artifact"}
ARGMAX_ROOTS = {
    "serial-v7": "ferric_qwen3_tp_argmax_f32_v7",
    "wave-v11": "ferric_qwen3_tp_batch32_wave_argmax_f32_v11",
}
# Independently released controller loading the same sidecar for both modes.
CONTROLLER_SHA256 = "7ef9f9e87a9fe1a5722dd3d1ab32c633de5ae82abfafd9d97443eb1e5880fa70"
NORM_ADMISSION_CONTROLLER_SHA256 = "dbb09fb78689667c9c6bf80659770e9a340f1fbcbca4ff4520c9b94da3368545"
KV_ARTIFACT = {
    "artifact_hsaco_id": "55f0953fc5e60279201d7523b5a11823170e1afcb48c90a985236ecb8518cb66",
    "artifact_manifest_id": "4951443039b903cb4f69e7d38763643ae75de6057aab044139f7e26451d2f0bf",
    "artifact_handoff_id": "828f650ab7e0c644cfe25d84ac202f3640e36f39c539612ad216cddb06cfd2f3",
}
KV_CANONICAL_DESCRIPTOR_SHA256 = "134aa309ba2a9eb7abf76762b3c588e965c2dc2ed7175f4b37fda6512b58c367"
KV_ADMISSION_SHA256 = "c6b96a4b92d82244dd3d179dcfbae2269cb4eba27959ce37985208307c05aa94"
KV_ADMISSION_PROVENANCE_SHA256 = "81f8e3d43af3f36927dda12d295ceeb2cec9989b8656fbe0abda282c4bde1450"
KV_CLI_ADMISSION_SHA256 = "767113d41542b0644e91d558f25d494d6b5baa43209de130a6985a0d158c0a12"
KV_CLI_ADMISSION_PROVENANCE_SHA256 = "b4fdd9d5db848b1895b2cfb1e30999a7138474c3085cd1d17161e98ad3a0f765"
KV_FIXTURES_SHA256 = "e8665f799a5488142f9639fde182c835bc7703143d60adbcd2812916616bd89b"
KV_SETUP = {"kv_append", "kv_append_artifact"}
KV_ROOTS = {"baseline": "ferric_qwen3_tp_batch_paged_kv_append_v2",
            "parallel-v16": "ferric_qwen3_tp_batch_parallel_kv_append_v16"}
RMSNORM_ARTIFACT = {
    "artifact_hsaco_id": "c33882db1afcd8eb26ec02bc43ea2144af322d29bf4e0e921e03ea74adc55af6",
    "artifact_manifest_id": "7bc199a5be639d4b7cfcb5a3abefa85312ea31077c081849b86dd2136f9dca7a",
    "artifact_handoff_id": "348c74397e520f5dca74d655ec50d9207a57ef2b36b378266b2318a0fdacebb5",
}
RMSNORM_CANONICAL_DESCRIPTOR_SHA256 = "67ed3415901de9936ff4dd01bfdf06847b7cb23067268af6921456faf4cb47f2"
RMSNORM_ADMISSION_SHA256 = "38318484211888d2d20da75985c7b8f8789dbf61907425031567ebea173928f9"
RMSNORM_SETUP = {"rmsnorm", "rmsnorm_artifact"}
ARTIFACT = {
    "artifact_hsaco_id": "2e677384a5e86c6e1ae95f10333f2d0d330922ed4d1348acdb1b09f5655281e2",
    "artifact_manifest_id": "e295584f1c5eeed36bdc653e83259e1dac3515b54d504b0704f54cd884531c67",
    "artifact_handoff_id": "dc7f7981a477b8fae0c3449930dcc890028fe26fff4c454a09ad6208be3d0a16",
}
HEAD_ARTIFACT = {
    "artifact_hsaco_id": "b21caccae5ec030640242034940822cb1a3f1e8531b7b6ded7371492da3747ae",
    "artifact_manifest_id": "6815ba33eb0c96a88a0722b1cdc73473d09cce3b4594d98370e6e1282437a008",
    "artifact_handoff_id": "d6ca1b9b6c5170eec8a904e2117e64c4988bd5a7334a6d3569c855e7400219c6",
}
HEAD_SETUP = {"head_precision", "fp32_head_artifact", "fp32_head_workspace_bytes"}
PROFILE = "mfma-v3-fp32-v7-wave-attention-v11-v15-v16-sidecars-tp1-616"
EXPECTATION_SCHEMA = "FerricTargetFullForwardParallelKvV16ExpectationV1"
STDERR = b"additional resident transposed weight bytes: 15136194560\n"
EXTRA_SETUP = {"runtime_full_forward", "full_forward_profile"}
VARIANTS = {"baseline": False, "parallel-v16": True}


def load_pinned():
    path = Path(__file__).with_name("target_batch_decode_v1.py")
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 131072:
            raise ValueError("shared source extent")
        raw = os.read(descriptor, 131073)
        after = os.fstat(descriptor)
        attrs = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        if len(raw) != before.st_size or any(getattr(before, key) != getattr(after, key) for key in attrs):
            raise ValueError("shared source changed")
        if hashlib.sha256(raw).hexdigest() != CORE_SHA256:
            raise ValueError("frozen shared source digest")
    finally:
        os.close(descriptor)
    module = types.ModuleType("_frozen_full_forward_parallel_kv_v16_batch_checker")
    module.__file__ = str(path)
    # Only verified bytes execute; no import cache or shared-module mutation.
    exec(compile(raw, str(path), "exec"), module.__dict__)
    return module


core = load_pinned()
PLAN = core.PLAN | EXTRA_SETUP | HEAD_SETUP | ARGMAX_SETUP | RMSNORM_SETUP | KV_SETUP | set(SOURCE_PINS) | {
    "variant", "common_comparator_sha256", "argmax_canonical_descriptor_sha256", "argmax_admission_sha256",
    "argmax_admission_provenance_sha256",
    "rmsnorm_canonical_descriptor_sha256", "rmsnorm_admission_sha256",
    "kv_canonical_descriptor_sha256", "kv_admission_sha256", "kv_cli_admission_sha256", "kv_fixtures_sha256",
}


def common_plan(plan):
    common = {key: copy.deepcopy(plan[key]) for key in core.PLAN}
    common["schema"] = "FerricTargetBatchDecodeExpectationV1"
    # Validate MFMA/head metadata first; only the old metadata allowlist is
    # adapted, never observed packets, scheduling, arithmetic receipts or times.
    common["kernel_profile"] = "v3-wave"
    common["performance_profile"]["projection"] = "baseline"
    common["performance_profile"]["attention"] = "baseline"
    return common


def checked_plan(plan):
    for value in KV_ARTIFACT.values():
        core.digest(value, "released KV artifact")
    for value in RMSNORM_ARTIFACT.values():
        core.digest(value, "released norm artifact")
    core.digest(RMSNORM_CANONICAL_DESCRIPTOR_SHA256, "released norm descriptor")
    core.digest(RMSNORM_ADMISSION_SHA256, "ordinary norm admission")
    for value in ARGMAX_ARTIFACT.values():
        core.digest(value, "released and admitted v11 sidecar")
    core.digest(ARGMAX_CANONICAL_DESCRIPTOR_SHA256, "released v11 descriptor")
    core.digest(ARGMAX_ADMISSION_SHA256, "ordinary v11 admission evidence")
    core.digest(ARGMAX_ADMISSION_PROVENANCE_SHA256, "ordinary v11 admission provenance")
    core.fields(plan, PLAN, "full-forward expectation")
    core.same(plan["schema"], EXPECTATION_SCHEMA, "expectation schema")
    core.require(type(plan["variant"]) is str and plan["variant"] in VARIANTS, "explicit variant")
    core.same(plan["runtime_full_forward"], True, "full-forward mode for both KV roots")
    core.same(plan["full_forward_profile"], PROFILE, "full-forward wave profile")
    for key, value in SOURCE_PINS.items():
        core.digest(value, "released source digest")
        core.same(plan[key], value, "frozen source " + key)
    core.same(plan["fp32_argmax"], "wave-v11", "unchanged wave argmax")
    core.same(plan["rmsnorm"], "wave-v15", "unchanged wave norm")
    core.same(plan["kv_append"], plan["variant"], "explicit KV mode")
    core.same(plan["kv_append_artifact"], KV_ARTIFACT, "same admitted KV image for both variants")
    for key, value in [("kv_canonical_descriptor_sha256", KV_CANONICAL_DESCRIPTOR_SHA256),
                       ("kv_admission_sha256", KV_ADMISSION_SHA256),
                       ("kv_cli_admission_sha256", KV_CLI_ADMISSION_SHA256),
                       ("kv_fixtures_sha256", KV_FIXTURES_SHA256)]:
        core.digest(value, "released KV prerequisite")
        core.same(plan[key], value, "frozen KV prerequisite " + key)
    core.same(plan["rmsnorm_artifact"], RMSNORM_ARTIFACT, "same admitted norm for both variants")
    core.same(plan["rmsnorm_canonical_descriptor_sha256"], RMSNORM_CANONICAL_DESCRIPTOR_SHA256, "frozen norm descriptor")
    core.same(plan["rmsnorm_admission_sha256"], RMSNORM_ADMISSION_SHA256, "frozen norm admission")
    core.same(plan["fp32_argmax_artifact"], ARGMAX_ARTIFACT, "same admitted sidecar for both variants")
    core.same(plan["argmax_canonical_descriptor_sha256"], ARGMAX_CANONICAL_DESCRIPTOR_SHA256, "frozen sidecar descriptor")
    core.same(plan["argmax_admission_sha256"], ARGMAX_ADMISSION_SHA256, "frozen sidecar admission")
    core.same(plan["argmax_admission_provenance_sha256"], ARGMAX_ADMISSION_PROVENANCE_SHA256, "frozen sidecar admission provenance")
    core.same(plan["common_comparator_sha256"], CORE_SHA256, "shared numerical checker")
    core.digest(CONTROLLER_SHA256, "independently frozen controller")
    core.same(plan["controller_sha256"], CONTROLLER_SHA256, "frozen integrated controller")
    core.same(plan["worker_sha256"], WORKER_SHA256, "same frozen preparation-fence worker")
    for key, value in ARTIFACT.items():
        core.same(plan[key], value, "frozen MFMA " + key)
    core.same(plan["new_tokens"], 32, "full frozen token window")
    core.same(plan["kernel_profile"], "v3-mfma", "closed MFMA v3 image")
    core.same(plan["head_precision"], "fp32-v7", "explicit FP32-v7 output head")
    core.same(plan["fp32_head_workspace_bytes"], 9_723_904, "exact capacity16 FP32 workspace")
    core.same(plan["fp32_head_artifact"], HEAD_ARTIFACT, "frozen v7 head image")
    core.same(plan["collective"], "device-tp1-v3", "device TP1 residual")
    core.same(plan["host_timing_enabled"], False, "no host timing instrumentation")
    core.same(plan["performance_profile"], {
        "runtime_cache_admission": True, "runtime_operational": True,
        "runtime_profiling": False, "dispatch_sequences": False, "queue_rollover": False,
        "projection": "mfma", "attention": "wave",
    }, "exact MFMA-v7-wave execution profile")
    core.checked_plan(common_plan(plan))
    return plan


def expected_workload(plan):
    return core.expected_workload(checked_plan(plan))


def execution_counts(plan):
    return {"forwards": 36, "packets_per_forward": 616, "packets": 22_176,
            "completion_frontiers": 36,
            "frontier_count_scope": "source-derived schedule; not measured polls or GPU event counts"}


def validate_records(records, plan, reference):
    load_pinned()
    checked_plan(plan)
    core.require(type(records) is list and len(records) == 40, "complete 32-token capture")
    extra = EXTRA_SETUP | ARGMAX_SETUP | RMSNORM_SETUP | KV_SETUP
    core.fields(records[0], core.SETUP | {"schema", "authority"} | extra | HEAD_SETUP, "full-forward setup")
    for key in extra | HEAD_SETUP | {"performance_profile"}:
        core.same(records[0][key], plan[key], "explicit setup " + key)
    normalized = copy.deepcopy(records)
    for key in extra | HEAD_SETUP:
        del normalized[0][key]
    normalized[0]["performance_profile"] = copy.deepcopy(common_plan(plan)["performance_profile"])
    # Dispatches, scheduling, bytes, identities and timestamps remain untouched.
    result = core.validate_records(normalized, common_plan(plan), reference)
    result.update(controller_cohort="full-forward-parallel-kv-v16-controller-v2", schema="FerricTargetFullForwardParallelKvV16ObservationV1", variant=plan["variant"],
                  performance_qualified=False, runtime_profiling=False, gpu_timestamps=False,
                  overlap_measured=False, completion_polls_measured=False, persistent_kernel=False,
                  common_comparator_sha256=CORE_SHA256, execution_counts=execution_counts(plan),
                  weight_precision="BF16", activation_precision="BF16", logits_precision="FP32",
                  head_precision="fp32-v7", head_artifact=copy.deepcopy(HEAD_ARTIFACT),
                  fp32_head_workspace_bytes=plan["fp32_head_workspace_bytes"],
                  fp32_argmax=plan["fp32_argmax"], fp32_argmax_artifact=copy.deepcopy(ARGMAX_ARTIFACT),
                  argmax_canonical_descriptor_sha256=ARGMAX_CANONICAL_DESCRIPTOR_SHA256,
                  argmax_admission_sha256=ARGMAX_ADMISSION_SHA256,
                  argmax_admission_provenance_sha256=ARGMAX_ADMISSION_PROVENANCE_SHA256,
                  argmax_root=ARGMAX_ROOTS["wave-v11"], sidecar_loaded_both_variants=True,
                  rmsnorm=plan["rmsnorm"], rmsnorm_artifact=copy.deepcopy(RMSNORM_ARTIFACT),
                  rmsnorm_canonical_descriptor_sha256=RMSNORM_CANONICAL_DESCRIPTOR_SHA256,
                  rmsnorm_admission_sha256=RMSNORM_ADMISSION_SHA256,
                  kv_append=plan["kv_append"], kv_append_artifact=copy.deepcopy(KV_ARTIFACT),
                  kv_canonical_descriptor_sha256=KV_CANONICAL_DESCRIPTOR_SHA256,
                  kv_admission_sha256=KV_ADMISSION_SHA256, kv_cli_admission_sha256=KV_CLI_ADMISSION_SHA256,
                  kv_fixtures_sha256=KV_FIXTURES_SHA256, kv_root=KV_ROOTS[plan["variant"]],
                  changed_kv_packets_per_forward=36, kv_workgroups_per_packet=64 if VARIANTS[plan["variant"]] else 1,
                  unchanged_wave_norm_packets_per_forward=73, rmsnorm_source_row_capacity=32,
                  argmax_source_row_capacity=32, controller_row_capacity=16, observed_rows=1,
                  transaction_preparation_fence=True, currentness_check_count_measured=False,
                  **SOURCE_PINS)
    result["configuration"].update(runtime_full_forward=plan["runtime_full_forward"],
                                   full_forward_profile=plan["full_forward_profile"],
                                   target_only=True, speculation=False, head_precision="fp32-v7",
                                   kernel_profile="v3-mfma",
                                   performance_profile=copy.deepcopy(plan["performance_profile"]),
                                   fp32_argmax=plan["fp32_argmax"], rmsnorm=plan["rmsnorm"], kv_append=plan["kv_append"])
    result["nonclaims"].append(
        "Both variants load identical v11/v15/v16 sidecars before allocation and retain the same controller, "
        "preparation-fence worker, paired-prefetch main image, FP32-v7 head and wave-attention/argmax. "
        "Only 36 KV append symbols and their grids change from one to 64 Wave64 workgroups. "
        "All 616 ordered packets, argument bytes, buffer extents, wave norms and FP32 association remain unchanged.")
    result["nonclaims"].append(
        "This observation validates one active TP1 row/context64/pages4 only, not other KV geometries "
        "or the controller capacity16 domain. No quantization, pruning, speculation, persistent kernel, "
        "GPU overlap, pure-prefetch causal gain, measured currentness counts or qualified speedup is claimed.")
    return result


def compare(capture, status, workload, reference, expectation, stderr):
    core.require(status == b"0\n", "successful controller status")
    core.require(stderr == STDERR, "exact unprofiled stderr preamble")
    plan = checked_plan(core.json_value(expectation))
    core.same(core.json_value(workload), expected_workload(plan), "frozen single-request workload")
    oracle = core.load_reference(reference)
    core.require(capture.endswith(b"\n"), "capture terminal newline")
    lines = capture.splitlines()
    core.require(0 < len(capture) <= 1024 * 1024 and len(lines) == 40
                 and all(lines) and all(len(line) <= 65536 for line in lines), "capture extent")
    result = validate_records([core.json_value(line) for line in lines], plan, oracle)
    result["input_sha256"] = {name: core.sha256(raw) for name, raw in (
        ("capture", capture), ("status", status), ("workload", workload), ("reference", reference),
        ("expectation", expectation), ("stderr", stderr))}
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("capture", "status", "workload", "reference", "expect", "stderr", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    os.umask(0o077)
    try:
        report = compare(core.read_bounded(args.capture, 1024 * 1024), core.read_bounded(args.status, 16),
                         core.read_bounded(args.workload, 65536), core.read_bounded(args.reference, 131072),
                         core.read_bounded(args.expect, 65536), core.read_bounded(args.stderr, 65536))
        report["comparator_sha256"] = core.sha256(core.read_bounded(Path(__file__), 131072))
        encoded = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        with args.output.open("x", encoding="utf-8") as stream:
            stream.write(encoded)
        print(json.dumps({"passed": True, "variant": report["variant"], "tokens": 32,
                          "packets": report["dispatches"],
                          "source_frontiers": report["execution_counts"]["completion_frontiers"]}))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as error:
        parser.exit(1, f"Target full-forward KV-v16 observation rejected ({type(error).__name__}); no successful observation.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
