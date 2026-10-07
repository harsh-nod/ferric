#!/usr/bin/env python3
"""Same-source V25 split-attention native A/B; not an HTTP or vendor benchmark."""

import argparse
import fcntl
import hashlib
import importlib.util
import os
from pathlib import Path
import resource
import signal
import socket
import stat
import sys
import time

PROFILE = "c1-split-attention-v25-live-v1"
MODES = ("baseline", "split8-v21")
HOST = "smci350-rck-g03-b19-03"
UID = 9661
DEVICE = "16366993098680759275"
SUPPORT_SHA = "0447f2629d94967baa819fae3c490deb89b23b69f12c19ec30290fd9dd19bc4c"
WORKER_SHA = "f68e42197f5f91854f7ec15c92b2fe1eee29a5620a6d313751e64f0af9598ebc"
SPLIT_OPTION = "--split-attention-artifact"
VARIED_REPORT_SHA = "ff4329f040dbfc4e62599f20ef6cfe3fe2e211633df383c4e553807e4692d469"
PROPOSAL_SHA = "ef709121b529a58ea37d53e1f2929761c308676340aa1f59fe16465b74294929"
WORKSPACE_BYTES = 133120
DISPATCHES_PER_REQUEST = {"baseline": 83139, "split8-v21": 87711}
SPLIT_POLICY = {"physical_rows": 1, "actual_context_min": 128, "actual_context_max": 256,
                "partitions": 8, "fallback": "query-hoist-v14", "fallback_packets_no_head": 613,
                "fallback_packets_with_head": 616, "split_packets_no_head": 649, "split_packets_with_head": 652}
CPU_PHASES = ("parser-tests", "prepare", "format", "device", "focused", "abi", "shapes", "bridge",
              "actual-abi", "cli", "legacy-v17", "legacy-v22", "legacy-diagnostic", "source-policy",
              "full-lib-minus-heavy", "clippy", "clippy-tests", "build", "harness")
HARNESS_TESTS = 18
SDK_NAME_PAIR_BRIDGE = {
    "adapter_revision": "5a503c04f5ae107a3b3e951ec970b36c5d6a9a79",
    "v21_device_revision": "c4c5cdd0f69f3844386440a5addb4d4c3dce0e4b",
    "exchange": "logical-and-export-name-strings-only",
    "cross_sdk_object_reinterpretation": False,
}
FORBIDDEN_METADATA = ("kv_append_mode", "kv_copy_artifact", "kv_copy_artifact_path",
                      "requested_c1_packet_mode", "c1_packet_mode", "requested_gemv_mode",
                      "gemv_mode", "gemv_artifact", "gemv_artifact_path")
# Five unchanged V17 images plus the separately emitted, admitted V21 image.
IMAGE_PINS = {
    "--target-artifact": ("c559d0533907323aff7dda03215adab6326423c3f151b296e22394c9baf37d00", "98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502"),
    "--target-head-artifact": ("19f4d8ec936d119214748445563b2b1f5955e04f4424503a0361c4bb6e2495d3", "5f19b3ba59035a5f0ebc90cdf3a40466f9910908a45e082d146cb674028da6cb"),
    "--argmax-artifact": ("7f9c0ab622d19721c71ff69197121b6af677fd85d07939bada4c148955cca0e8", "de9db78c0ef7ad5d84fc903d41ee9db59026113d79e23f12d3909761363b9390"),
    "--attention-artifact": ("eb058fceb9519c4e9f9fb9d347263dcb80e957c1accd87122c4e139a9e571752", "8f21681fe9103b670ee5666f429a45682e77fc90eb16802a02c4b6fb93a192c8"),
    "--rmsnorm-artifact": ("6698058977ce1634dcf1408861779c09f8538edf95d0fcd4dc74753d71a19c8c", "68607fc1d12eb6151dd55e7b53dcb88b00d484cfa0c087c002666b224093b45c"),
    SPLIT_OPTION: ("514a7d8a7db5227a4ed437b70fb549bd325a3c55fe907f48cb705737ae4f7d17", "45c91170c4e83178f3f3f849daffb0b0e1e302ec2c3d4f1178df1a48a9ccc069"),
}
PLAN_KEYS = {"schema", "stage", "common_args", "controller", "build_provenance", "images", "workload",
             "reference", "target_manifest", "modes", "warmup_requests", "measured_requests", "timeouts",
             "numerical_prerequisite"}
CAVEAT = ("One warmup and three measured native requests per arm in fixed baseline/candidate order. "
          "Controller-ingress TTFT/TPOT and finite-window output rate are not HTTP latency, true token ITL, "
          "GPU durations, sustained throughput, stable gains or a vendor comparison. Both arms load the "
          "same five unchanged V17 images plus V21 and use the same binary, f68 runtime, baseline packet policy and sequential arrivals; only the split-attention selector differs. "
          "The varied-score prerequisite is conditional on an unverified OCML exp error bound, not bitwise kernel parity or general numerical qualification. "
          "All eight full-model requests must independently match every reference ID and decoded byte exactly. "
          "The dual 5a/c4c SDK name-pair bridge is engineering custody, not formal or independent source-to-binary authentication. "
          "No serving, proof, TP8 or general performance qualification follows.")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load_support(directory):
    path = directory / "runtime_profile_v17.py"
    require(path.resolve(strict=True) == path and not path.is_symlink(), "canonical support file required")
    with path.open("rb") as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size < 1024 * 1024, "support extent")
        source = stream.read()
        after = os.fstat(stream.fileno())
    current = path.stat()
    fields = ("st_dev", "st_ino", "st_size", "st_mode", "st_mtime_ns", "st_ctime_ns")
    require(all(getattr(before, key) == getattr(after, key) == getattr(current, key) for key in fields)
            and hashlib.sha256(source).hexdigest() == SUPPORT_SHA, "support hash drift")
    spec = importlib.util.spec_from_loader("ferric_v25_frozen_runtime_support", loader=None, origin=str(path))
    support = importlib.util.module_from_spec(spec)
    support.__file__ = str(path)
    exec(compile(source, str(path), "exec"), support.__dict__)
    profile, runner, supervisor, identities = support.load_helpers(directory)
    identities["runtime_profile_v17.py"] = {"path": str(path), "bytes": len(source), "sha256": SUPPORT_SHA}
    return support, profile, runner, supervisor, identities


def validate_plan(plan, runner, support):
    require(type(plan) is dict and set(plan) == PLAN_KEYS, "closed V25 native plan required")
    require(plan["schema"] == "FerricC1SplitAttentionV25NativePlanV1" and plan["modes"] == list(MODES), "distinct V25 two-arm plan")
    require(type(plan["warmup_requests"]) is int and plan["warmup_requests"] == 1
            and type(plan["measured_requests"]) is int and plan["measured_requests"] == 3, "one warmup and three measured requests")
    require(plan["timeouts"] == {"setup_seconds": 600, "request_seconds": 180, "arm_seconds": 1200}
            and all(type(value) is int for value in plan["timeouts"].values()), "fixed bounded V25 timeouts")
    require(type(plan["stage"]) is str, "stage path string")
    stage = Path(plan["stage"])
    require(stage.is_absolute() and stage.parent == Path("/tmp") and stage.name.startswith("ferric-opt-v5-v25."), "private V25 stage")
    for key in ("controller", "build_provenance", "workload", "reference", "target_manifest", "numerical_prerequisite"):
        value = plan[key]
        require(type(value) is dict and set(value) == {"path", "sha256"}
                and type(value["path"]) is str and Path(value["path"]).is_absolute(), "closed input binding")
        runner.sha(value["sha256"])
    require(plan["build_provenance"]["path"] == str(stage / "build.json")
            and plan["numerical_prerequisite"] == {"path": str(stage / "attention-varied-report-r2.json"),
                                                   "sha256": VARIED_REPORT_SHA}, "owned actual build/numerical prerequisite required")
    for key, digest in support.INPUT_PINS.items():
        require(plan[key]["sha256"] == digest, "unchanged matched reference required: " + key)
    options = runner.arguments(plan["common_args"])
    require(options["--device-unique-id"] == DEVICE and options["--worker"] == str(stage / "worker-candidate"),
            "GPU or owned worker path mismatch")
    require(options["--worker-sha256"] == WORKER_SHA, "frozen f68 worker required")
    require(plan["controller"]["path"] == str(stage / "controller-c1-split-attention-v25"), "distinct V25 controller required")
    images = plan["images"]
    require(type(images) is dict and set(images) == set(runner.ARTIFACTS) | {SPLIT_OPTION}
            and set(images) == set(IMAGE_PINS), "exact five legacy plus one V21 image roster")
    paths = set()
    for option, image in images.items():
        require(type(image) is dict and set(image) == {"path", "manifest_sha256", "hsaco_sha256"}
                and type(image["path"]) is str and Path(image["path"]).is_absolute(), "closed image binding")
        require(image["path"] not in paths, "distinct image directories required")
        require(option != SPLIT_OPTION or Path(image["path"]).is_relative_to(stage), "owned V21 image path required")
        paths.add(image["path"])
        runner.sha(image["manifest_sha256"])
        runner.sha(image["hsaco_sha256"])
        require((image["manifest_sha256"], image["hsaco_sha256"]) == IMAGE_PINS[option], "frozen image identity required: " + option)
        require(option == SPLIT_OPTION or options[option] == image["path"], "image path differs from argv")
    return stage, options


def inputs(plan, options, runner):
    identities = runner.input_identities(plan, options)
    for option, image in plan["images"].items():
        path = Path(image["path"])
        require(path.resolve(strict=True) == path and path.is_dir() and not path.is_symlink()
                and {entry.name for entry in path.iterdir()} == {"observation.json", "observation.hsaco"}, "canonical two-file image")
        observed = {name: runner.identity(path / name, image[key], 64 * 1024 * 1024)
                    for name, key in (("observation.json", "manifest_sha256"), ("observation.hsaco", "hsaco_sha256"))}
        require(option not in identities or identities[option] == observed, "base image changed during capture")
        identities[option] = observed
    return identities


def validate_build(value, identities):
    require(type(value) is dict and value.get("schema") == "FerricV5V25NativeBuildV1", "distinct V25 engineering build receipt")
    for name in ("controller", "worker"):
        entry = value.get(name)
        require(type(entry) is dict and entry.get("sha256") == identities[name]["sha256"], "build executable hash mismatch")
    bridge = value.get("sdk_name_pair_bridge")
    require(type(bridge) is dict and set(bridge) == set(SDK_NAME_PAIR_BRIDGE)
            and all(type(bridge[key]) is type(wanted) and bridge[key] == wanted
                    for key, wanted in SDK_NAME_PAIR_BRIDGE.items()), "explicit dual-SDK name-pair bridge required")
    for key, wanted in {"authority": "none", "cpu_required_phases_passed": True, "native_parity_passed": False,
                        "performance_qualified": False, "formal_qualification": False,
                        "independent_source_to_binary_authentication": False}.items():
        require(type(value.get(key)) is type(wanted) and value[key] == wanted, "V25 build qualification mismatch: " + key)
    def receipt(entry):
        require(type(entry) is dict and type(entry.get("path")) is str and Path(entry["path"]).is_absolute()
                and type(entry.get("sha256")) is str and len(entry["sha256"]) == 64
                and all(c in "0123456789abcdef" for c in entry["sha256"]), "actual build custody binding required")
    receipt(value.get("proposal_manifest"))
    require(value["proposal_manifest"]["sha256"] == PROPOSAL_SHA, "reviewed V25 source proposal required")
    for field in ("before", "overlay", "locked", "formatted"):
        receipt(value.get("source_inventory", {}).get(field))
    for field in ("lock-delta", "feature-delta", "metadata-locked"):
        receipt(value.get("dependency_review", {}).get(field))
    phases = value.get("phase_receipts")
    require(type(phases) is dict and set(CPU_PHASES) <= set(phases), "all real V25 CPU phases required")
    for name in CPU_PHASES:
        phase = phases[name]
        require(type(phase) is dict and type(phase.get("status")) is int and phase["status"] == 0
                and phase.get("cleanup_ok") is True and phase.get("child_reaped") is True,
                "unclean or failed V25 CPU phase: " + name)
        for field in ("result", "stdout", "stderr", "exit.status"):
            receipt(phase.get(field))
    exact = {"device": [(5, 0), (8, 0)], "focused": [(11, 0)], "abi": [(2, 1)],
             "shapes": [(2, 0)], "bridge": [(1, 0)], "actual-abi": [(1, 0)] * 3}
    for name, counts in exact.items():
        wanted = [{"passed": passed, "failed": 0, "ignored": ignored} for passed, ignored in counts]
        observed = phases[name].get("reported_rust_test_summaries")
        require(observed == wanted and all(type(row.get(key)) is int for row in observed for key in wanted[0]),
                "exact V25 CPU count required: " + name)
    for name in ("cli", "legacy-v17", "legacy-v22", "legacy-diagnostic", "source-policy", "full-lib-minus-heavy"):
        summaries = phases[name].get("reported_rust_test_summaries")
        require(type(summaries) is list and len(summaries) == 1 and type(summaries[0].get("passed")) is int
                and summaries[0]["passed"] > 0 and type(summaries[0].get("failed")) is int
                and summaries[0]["failed"] == 0, "complete nonzero V25 CPU suite required: " + name)
    for name, count in (("parser-tests", 12), ("harness", HARNESS_TESTS)):
        require(type(phases[name].get("python_tests")) is int and phases[name]["python_tests"] == count,
                "complete V25 Python tests required: " + name)
    admission = value.get("actual_image_admission", {})
    receipt(admission.get("before"))
    receipt(admission.get("after"))
    for key, option in (("FERRIC_TEST_BATCH32_ARTIFACT", "--target-artifact"),
                        ("FERRIC_TEST_SPLIT_ATTENTION_V21_ARTIFACT", SPLIT_OPTION)):
        require(admission.get("images", {}).get(key, {}).get("files") ==
                dict(zip(("observation.json", "observation.hsaco"), IMAGE_PINS[option])), "actual V25 CPU image admission required")
    require(type(value.get("coverage_gaps")) is list and value["coverage_gaps"], "explicit library/formal coverage gaps required")


def validate_numerical_prerequisite(value):
    expected = {"schema": "FerricVariedAttentionCampaignV2", "authority": "conditional-engineering-check-not-proof",
                "accepted": True, "ordered_packets": 63, "model_inference": False, "measurement_admitted": False,
                "performance_qualified": False, "full_model_output_parity_established": False,
                "timings_excluded_from_comparison": True}
    for key, wanted in expected.items():
        require(type(value.get(key)) is type(wanted) and value[key] == wanted, "varied prerequisite mismatch: " + key)
    assumptions = value.get("assumptions", {})
    require(assumptions.get("ocml_accuracy_bound_verified") is False
            and assumptions.get("ocml_exp_relative_error_assumed_at_most") == "2^-20"
            and assumptions.get("automatic_tolerance_widening") is False
            and assumptions.get("decimal_precision") == 80, "conditional varied numerical assumptions required")
    require(value.get("inputs_before") == value.get("inputs_after") and type(value.get("inputs_before")) is dict,
            "varied prerequisite input custody changed")
    for name, digest in zip(("observation.json", "observation.hsaco"), IMAGE_PINS[SPLIT_OPTION]):
        require(value["inputs_before"].get("v21/" + name, {}).get("sha256") == digest,
                "varied prerequisite V21 image mismatch")
    cells = value.get("cells")
    expected_names = [kind + "-c" + str(context) for context in (128, 129, 191, 192, 193, 255, 256)
                      for kind in ("mixed", "range", "boundary")]
    require(type(cells) is list and [cell.get("name") for cell in cells] == expected_names,
            "exact twenty-one varied cells required")
    for cell in cells:
        cleanup, correctness, protocol = cell.get("cleanup", {}), cell.get("correctness", {}), cell.get("protocol", {})
        require(cell.get("accepted") is True and cell.get("completed") is True
                and correctness.get("accepted_under_stated_assumptions") is True
                and correctness.get("assumptions") == assumptions, "unaccepted varied correctness cell")
        require(cleanup.get("clean_exit") is True and cleanup.get("cleanup_ok") is True
                and type(cleanup.get("returncode")) is int and cleanup["returncode"] == 0
                and cleanup.get("signals") == [] and cleanup.get("errors") == []
                and cleanup.get("ownership_lost") is False, "unclean varied prerequisite cell")
        require(protocol.get("ordered_packets") == 3 and protocol.get("performance_snapshots") == 0
                and protocol.get("timings_excluded_from_comparison") is True, "varied prerequisite packet/timing mismatch")
    return {"sha256": VARIED_REPORT_SHA, "cells": 21, "ordered_packets": 63, "assumptions": assumptions,
            "conditional_numerics_only": True, "kernel_bitwise_parity_required": False,
            "full_model_exact_reference_required": True}


def retained_json(path, runner):
    receipt = runner.identity(path, maximum=65536)
    value = runner.decode(path.read_bytes())
    runner.identity(path, receipt["sha256"], 65536)
    return value, receipt


def check_image(value, option, identities, runner):
    require(type(value) is dict and value.get("artifact_hsaco_id") == identities[option]["observation.hsaco"]["sha256"]
            and value.get("artifact_manifest_id") == identities[option]["observation.json"]["sha256"], "loaded image identity mismatch: " + option)
    runner.sha(value.get("artifact_handoff_id"))


def check_split_policy(value):
    require(type(value.get("split_attention_workspace_bytes")) is int
            and value["split_attention_workspace_bytes"] == WORKSPACE_BYTES, "exact V25 scratch allocation required")
    policy = value.get("split_attention_policy")
    require(type(policy) is dict and set(policy) == set(SPLIT_POLICY)
            and all(type(policy[key]) is type(wanted) and policy[key] == wanted
                    for key, wanted in SPLIT_POLICY.items()), "exact V25 split/fallback policy required")


def check_setup(value, mode, options, identities, plan, runner):
    require(mode in MODES and type(value) is dict, "V25 setup object and mode required")
    expected = {"schema": "FerricQwen3TpBatchSetupV2", "authority": "none", "live_profile": PROFILE,
                "wave_target_mode": "combined", "requested_split_attention_mode": mode, "split_attention_mode": mode, "layer_projection": "c1-wave",
                "attention_mode": "query-hoist-v14", "rmsnorm_mode": "wave-v15", "tensor_parallel": 1,
                "model": "Qwen/Qwen3-8B", "dtype": "BF16", "target": "gfx950:xnack-", "head_precision": "fp32-v8",
                "argmax_mode": "wave-v11", "submission": "ordered", "runtime_ordered_batches": True,
                "prefix_cache": False, "context_tokens": 8192, "physical_pages": 512, "prefill_chunk": 16,
                "batch_tokens": 32, "performance_qualified": False, "serving_qualified": False,
                "benchmark_admitted": False, "serving_admitted": False, "output_head_pruning": True,
                "max_batches": int(options["--max-batches"]), "controller_sha256": identities["controller"]["sha256"],
                "worker_sha256": identities["worker"]["sha256"], "running_worker_sha256": [identities["worker"]["sha256"]],
                "device_unique_ids": [int(DEVICE)], "target_model_id": runner.TARGET_MODEL_ID,
                "split_attention_artifact_path": plan["images"][SPLIT_OPTION]["path"],
                "attention_artifact_path": options["--attention-artifact"], "rmsnorm_artifact_path": options["--rmsnorm-artifact"]}
    for key, wanted in expected.items():
        require(type(value.get(key)) is type(wanted) and value[key] == wanted, "V25 setup mismatch: " + key)
    require(not any(key in value for key in FORBIDDEN_METADATA), "V19/V20/V22 metadata is not V25")
    check_split_policy(value)
    for option, field in {**runner.ARTIFACTS, SPLIT_OPTION: "split_attention_artifact"}.items():
        check_image(value if field is None else value.get(field), option, identities, runner)
    profile = value.get("performance_profile", {})
    for key, wanted in {"live_profile": PROFILE, "wave_target_mode": "combined", "requested_split_attention_mode": mode, "split_attention_mode": mode,
                        "attention": "query-hoist-v14", "rmsnorm_mode": "wave-v15", "layer_projection": "c1-wave",
                        "runtime_profiling": False, "dispatch_sequences": False, "runtime_cache_admission": True,
                        "runtime_operational": True, "queue_rollover": True, "runtime_ordered_batches": True,
                        "submission": "ordered", "projection": "mfma", "argmax_mode": "wave-v11",
                        "benchmark_admitted": False, "serving_admitted": False,
                        "split_attention_artifact_path": plan["images"][SPLIT_OPTION]["path"]}.items():
        require(type(profile.get(key)) is type(wanted) and profile[key] == wanted, "V25 policy mismatch: " + key)
    require(not any(key in profile for key in FORBIDDEN_METADATA), "V19/V20/V22 policy is not V25")
    check_split_policy(profile)
    check_image(profile.get("split_attention_artifact"), SPLIT_OPTION, identities, runner)
    require(profile["split_attention_artifact"] == value["split_attention_artifact"], "split-attention image metadata differs")
    pids = value.get("worker_pids")
    require(type(pids) is list and len(pids) == 1 and type(pids[0]) is int and 0 < pids[0] <= 0xffffffff, "one worker PID")
    return {key: runner.sha(value.get(key)) for key in ("model_bundle_id", "target_model_id")}


def check_closed(value, mode, setup, dispatches):
    require(type(value) is dict, "V25 close object")
    expected = {"schema": "FerricQwen3TpBatchClosedV2", "authority": "none", "live_profile": PROFILE,
                "wave_target_mode": "combined", "requested_split_attention_mode": mode, "split_attention_mode": mode, "layer_projection": "c1-wave",
                "submission": "ordered", "rmsnorm_mode": "wave-v15", "attention_mode": "query-hoist-v14",
                "performance_qualified": False, "benchmark_admitted": False, "serving_admitted": False,
                "worker_pids": setup["worker_pids"], "execution_completed": True, "all_workers_exited": True,
                "rank_dispatch_counts": [dispatches], "split_attention_artifact": setup["split_attention_artifact"],
                "split_attention_artifact_path": setup["split_attention_artifact_path"]}
    for key, wanted in expected.items():
        require(type(value.get(key)) is type(wanted) and value[key] == wanted, "V25 close mismatch: " + key)
    require(not any(key in value for key in FORBIDDEN_METADATA), "V19/V20/V22 close is not V25")
    check_split_policy(value)


class Events:
    """Keep the frozen validator; translate only independently checked packet counts."""

    def __init__(self, runner, mode):
        require(mode in MODES, "closed V25 event mode")
        self.base = runner.Events()
        self.mode = mode
        self.extra_dispatches = 0

    @property
    def emission(self):
        return self.base.emission

    @property
    def batches(self):
        return self.base.batches

    @property
    def dispatches(self):
        return self.base.dispatches + self.extra_dispatches

    def validate(self, event):
        if event.get("event") != "batch":
            self.base.validate(event)
            return
        ordinal = self.batches % 135
        rows, outputs = (16 if ordinal < 8 else 1), (0 if ordinal < 7 else 1)
        require(type(event.get("rows")) is int and event["rows"] == rows
                and type(event.get("outputs")) is int and event["outputs"] == outputs
                and type(event.get("output_head_rows")) is int and event["output_head_rows"] == outputs,
                "128/128 physical batch geometry mismatch")
        legacy = 616 if outputs else 613
        extra = 36 if self.mode == "split8-v21" and ordinal >= 8 else 0
        counts = event.get("rank_dispatch_counts")
        require(type(counts) is list and len(counts) == 1 and type(counts[0]) is int
                and counts[0] == legacy + extra, "batch dispatch count drift")
        # The raw event and retained stream stay unchanged. All frozen event,
        # ordinal, timing and geometry checks still run on a shallow copy.
        self.base.validate(dict(event, rank_dispatch_counts=[legacy]))
        self.extra_dispatches += extra


def parse_placement(raw_stat, raw_status, pid):
    head, separator, rest = raw_stat.rpartition(")")
    require(separator and head.split("(", 1)[0].strip() == str(pid), "placement PID mismatch")
    fields = rest.split()
    require(len(fields) >= 37, "incomplete process stat")
    allowed = [line.split(":", 1)[1].strip() for line in raw_status.splitlines() if line.startswith("Cpus_allowed_list:")]
    require(len(allowed) == 1 and allowed[0], "one affinity observation required")
    return {"process_id": pid, "parent_pid": int(fields[1]), "process_group": int(fields[2]), "session": int(fields[3]),
            "priority": int(fields[15]), "nice": int(fields[16]), "start_time_ticks": int(fields[19]),
            "last_processor": int(fields[36]), "cpus_allowed_list": allowed[0]}


def read_placement(controller, setup, identities, placement, support, proc_root=Path("/proc")):
    result = {}
    controller_pid = controller.proc.pid
    worker_pid = setup["worker_pids"][0]
    require(worker_pid != controller_pid, "worker cannot be controller")
    for role, pid in (("controller", controller_pid), ("worker", worker_pid)):
        directory = proc_root / str(pid)
        def read(name):
            with (directory / name).open("rb") as stream:
                raw = stream.read(16385)
            require(0 < len(raw) <= 16384, "bounded process placement required")
            return raw.decode("utf-8")
        first_stat = read("stat")
        status = read("status")
        before = parse_placement(first_stat, status, pid)
        executable = os.readlink(directory / "exe")
        after = parse_placement(read("stat"), status, pid)
        require(before["start_time_ticks"] == after["start_time_ticks"] > 0, "process lifetime changed")
        require(all(before[key] == after[key] for key in before if key != "last_processor"), "process placement changed while reading")
        require(executable == identities[role]["path"], "running executable path differs")
        require(before["process_group"] == before["session"] == controller_pid
                and (role != "worker" or before["parent_pid"] == controller_pid), "owned process topology mismatch")
        require(before["nice"] == placement["nice"] == 0 and support.affinity_list(before["cpus_allowed_list"]) == placement["affinity"],
                "process placement differs from fresh default launch")
        result[role] = dict(before, executable=executable)
    return result


def stable_placement(before, after):
    for role in ("controller", "worker"):
        require(set(before[role]) == set(after[role]), "placement fields changed")
        for key in before[role]:
            if key != "last_processor":
                require(before[role][key] == after[role][key], "owned process identity/placement changed: " + key)


def run_arm(argv, output, mode, request, reference, timeouts, setup_check, identities, placement, runner, support):
    output.mkdir(mode=0o700)
    started = time.monotonic_ns()
    deadline = time.monotonic() + timeouts["arm_seconds"]
    result = {"mode": mode, "live_profile": PROFILE, "accepted": False, "requests": [], "started_ns": started,
              "performance_qualified": False, "http": False, "vendor_comparison": False, "caveat": CAVEAT}
    controller = None
    try:
        require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "default SIGCHLD required")
        controller = runner.Controller(argv, output, deadline)
        phase = min(deadline, time.monotonic() + timeouts["setup_seconds"])
        setup = controller.next(phase)
        result["model_identity"] = setup_check(setup)
        result["setup"] = setup
        result["requested_split_attention_mode"] = mode
        result["split_attention_mode"] = setup["split_attention_mode"]
        result["placement_before"] = read_placement(controller, setup, identities, placement, support)
        events = Events(runner, mode)
        ready = controller.next(phase)
        events.validate(ready)
        require(ready.get("event") == "ready" and ready.get("clock") == "monotonic_ns_since_live_start"
                and ready.get("eos_policy") == "fixed output count" and ready.get("context_tokens") == 8192
                and ready.get("physical_pages") == 512, "exact ready envelope required")
        for index in range(4):
            request_id = index + 1
            name = "warmup-0" if index == 0 else "measured-" + str(index)
            phase = min(deadline, time.monotonic() + timeouts["request_seconds"])
            controller.send({"schema": runner.COMMAND, "op": "submit", "request_id": request_id,
                             "name": name, "prompt": request["prompt"], "new_tokens": 128}, phase)
            before = (events.batches, events.dispatches)
            record = runner.collect_request(controller, events, request_id, name, reference, phase)
            require((events.batches - before[0], events.dispatches - before[1]) == (135, DISPATCHES_PER_REQUEST[mode]),
                    "exact per-request V25 packet geometry")
            record["batches"], record["dispatches"] = 135, DISPATCHES_PER_REQUEST[mode]
            record["warmup"] = index == 0
            result["requests"].append(record)
            runner.save(output / "partial-result.json", result)
        result["placement_after"] = read_placement(controller, setup, identities, placement, support)
        stable_placement(result["placement_before"], result["placement_after"])
        phase = min(deadline, time.monotonic() + 15)
        controller.send({"schema": runner.COMMAND, "op": "drain"}, phase)
        draining = controller.next(phase)
        events.validate(draining)
        require(draining.get("event") == "draining" and draining.get("reason") == "command", "drain missing")
        stopped = controller.next(phase)
        require(stopped.get("schema") == runner.EVENT and stopped.get("authority") == "none"
                and stopped.get("event") == "stopped" and stopped.get("reason") == "drained"
                and stopped.get("batches") == events.batches
                and runner.integer(stopped.get("emission_started_ns"), "stopped emission") >= events.emission, "clean stopped record required")
        closed = controller.next(phase)
        check_closed(closed, mode, setup, events.dispatches)
        require(events.batches == 540 and events.dispatches == 4 * DISPATCHES_PER_REQUEST[mode],
                "exact four-request batch/dispatch geometry")
        controller.finish(phase)
        samples = result["requests"][1:]
        span = samples[-1]["completed_ns"] - samples[0]["arrival_ns"]
        require(span > 0, "positive measured ingress span")
        require(all(row["ttft_ns"] > 0 and row["tpot_ns"] > 0 for row in samples), "positive native timing samples required")
        result.update(accepted=True, closed=closed, batches=events.batches, dispatches=events.dispatches,
                      exact_output_tokens_checked=sum(row["output_tokens"] for row in result["requests"]),
                      mean_ttft_ms=sum(row["ttft_ns"] for row in samples) / 3e6,
                      mean_tpot_ms=sum(row["tpot_ns"] for row in samples) / 3e6,
                      measured_span_ns=span, measured_ingress_output_tokens_per_second=384e9 / span)
        return result
    except BaseException as error:
        result["error"] = type(error).__name__ + ": " + str(error)
        raise
    finally:
        handlers = {number: signal.signal(number, signal.SIG_IGN) for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)}
        try:
            try:
                cleanup = controller.close() if controller is not None else {"cleanup_ok": True}
            except BaseException as error:
                cleanup = {"cleanup_ok": False, "error": type(error).__name__ + ": " + str(error)}
            result["cleanup"] = cleanup
            result["finished_ns"] = time.monotonic_ns()
            if not cleanup.get("cleanup_ok", False):
                result["accepted"] = False
            runner.save(output / "result.json", result)
            require(cleanup.get("cleanup_ok", False), "owned V25 process cleanup failed")
        finally:
            for number, handler in handlers.items():
                signal.signal(number, handler)


def run_pair(plan, output, runner, support, helpers, placement):
    output.mkdir(mode=0o700)
    report = {"schema": "FerricC1SplitAttentionV25NativePairV1", "authority": "none", "accepted": False, "arms": [],
              "http": False, "vendor_comparison": False, "sustained": False, "performance_qualified": False,
              "formal_qualification": False, "instrumented": False, "caveat": CAVEAT, "helpers": helpers,
              "launch_placement": placement, "cohort_order": list(MODES), "warmup_per_arm": 1, "measured_per_arm": 3}
    runner.save(output / "plan.json", plan)
    try:
        _, options = validate_plan(plan, runner, support)
        request, reference = runner.workload_reference(plan)
        common = inputs(plan, options, runner)
        build = runner.bound_json(plan["build_provenance"])
        validate_build(build, common)
        numerical = runner.bound_json(plan["numerical_prerequisite"])
        report["numerical_prerequisite"] = validate_numerical_prerequisite(numerical)
        runner.save(output / "numerical-prerequisite.json", numerical)
        runner.save(output / "inputs.json", common)
        runner.save(output / "build-provenance.json", build)
        report["build_provenance"] = {"binding": plan["build_provenance"], "independent_source_to_binary_authentication": False,
                                      "sdk_name_pair_bridge": build["sdk_name_pair_bridge"]}
        model = runner.model_identities(options)
        runner.save(output / "model-inputs.json", model)
        baseline_argv = None
        baseline_stdin = None
        for mode in MODES:
            require(inputs(plan, options, runner) == common, "inputs changed before arm")
            argv = [common["controller"]["path"], *plan["common_args"], "--wave-target-mode", "combined",
                    SPLIT_OPTION, plan["images"][SPLIT_OPTION]["path"], "--split-attention-mode", mode]
            if baseline_argv is None:
                baseline_argv = argv
            else:
                require(argv[:-1] == baseline_argv[:-1] and argv[-1] != baseline_argv[-1], "only split-attention mode may differ")
            arm_output = output / mode
            report["active_arm"] = mode
            runner.save(output / "report.json", report)
            arm = run_arm(argv, arm_output, mode, request, reference, plan["timeouts"],
                          lambda value: check_setup(value, mode, options, common, plan, runner), common, placement, runner, support)
            require(arm["accepted"] and arm["cleanup"]["cleanup_ok"], "arm or cleanup incomplete")
            if report["arms"]:
                require(arm["started_ns"] >= report["arms"][0]["finished_ns"], "arms overlapped")
                require(arm["model_identity"] == report["arms"][0]["model_identity"], "model identity changed between arms")
                for field in (None, "fp32_head_artifact", "argmax_artifact", "attention_artifact", "rmsnorm_artifact", "split_attention_artifact"):
                    first = report["arms"][0]["setup"]
                    second = arm["setup"]
                    first, second = (first, second) if field is None else (first[field], second[field])
                    require(all(first[key] == second[key] for key in ("artifact_hsaco_id", "artifact_manifest_id", "artifact_handoff_id")),
                            "loaded image identity changed between arms")
            arm["streams"] = {}
            for name in ("stdin", "stdout"):
                arm["streams"][name] = runner.identity(arm_output / (name + ".raw"), maximum=runner.MAX_STREAM)
            process, process_receipt = retained_json(arm_output / "process.json", runner)
            cleanup, cleanup_receipt = retained_json(arm_output / "cleanup.json", runner)
            require(process.get("argv") == argv and process.get("start_new_session") is True
                    and process.get("pid") == arm["placement_before"]["controller"]["process_id"]
                    and process.get("owned_pgid") == process.get("pid"), "retained process identity mismatch")
            require(cleanup == arm["cleanup"] and cleanup.get("cleanup_ok") is True, "retained cleanup differs")
            arm["process_receipt"], arm["cleanup_receipt"] = process_receipt, cleanup_receipt
            stderr = arm_output / "stderr.raw"
            require(stderr.is_file() and not stderr.is_symlink() and stderr.stat().st_size == 0, "unexpected V25 controller/worker stderr")
            arm["streams"]["stderr"] = {"path": str(stderr), "bytes": 0, "sha256": hashlib.sha256(b"").hexdigest()}
            commands = (arm_output / "stdin.raw").read_bytes()
            runner.identity(arm_output / "stdin.raw", arm["streams"]["stdin"]["sha256"], runner.MAX_STREAM)
            raw_lines = (arm_output / "stdout.raw").read_bytes().splitlines()
            require(raw_lines and runner.decode(raw_lines[0]) == arm["setup"] and runner.decode(raw_lines[-1]) == arm["closed"],
                    "raw setup/close differ from accepted arm")
            runner.identity(arm_output / "stdout.raw", arm["streams"]["stdout"]["sha256"], runner.MAX_STREAM)
            if baseline_stdin is None:
                baseline_stdin = commands
            else:
                require(commands == baseline_stdin, "request command sequence differs between arms")
            report["arms"].append(arm)
            require(inputs(plan, options, runner) == common, "inputs changed after arm")
            require(runner.model_identities(options) == model, "model bytes changed after arm")
            runner.workload_reference(plan)
            require(runner.bound_json(plan["build_provenance"]) == build, "build receipt changed")
            require(runner.bound_json(plan["numerical_prerequisite"]) == numerical, "numerical prerequisite changed")
            runner.save(arm_output / "inputs-after.json", common)
            runner.save(arm_output / "model-inputs-after.json", model)
        baseline, candidate = report["arms"]
        require(baseline["exact_output_tokens_checked"] == candidate["exact_output_tokens_checked"] == 512,
                "exactly 1024 full-model output tokens must be checked")
        report["exact_output_tokens_checked"] = 1024
        report["candidate_over_baseline"] = {key: candidate[key] / baseline[key] for key in
                                             ("mean_ttft_ms", "mean_tpot_ms", "measured_ingress_output_tokens_per_second")}
        report["active_arm"] = None
        report["accepted"] = True
        return report
    except BaseException as error:
        report["error"] = type(error).__name__ + ": " + str(error)
        raise
    finally:
        runner.save(output / "report.json", report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--plan-sha256", required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--supervise", action="store_true")
    mode.add_argument("--execute", action="store_true", help="internal supervised child only")
    args = parser.parse_args()
    require(socket.gethostname() == HOST and os.getuid() == UID, "wrong GPU host or UID")
    directory = Path(__file__).resolve().parent
    support, profile, runner, supervisor, helpers = load_support(directory)
    require(args.plan.resolve(strict=True) == args.plan, "canonical plan required")
    binding = {"path": str(args.plan), "sha256": args.plan_sha256}
    plan = runner.bound_json(binding)
    stage, _ = validate_plan(plan, runner, support)
    require(stage.resolve(strict=True) == stage and directory == stage / "v25-native"
            and args.plan == stage / "v25-native-plan.json", "owned stage mismatch")
    owner = stage.stat()
    require(owner.st_uid == UID and stat.S_IMODE(owner.st_mode) == 0o700, "private owned stage required")
    require(supervisor.HOST == HOST and supervisor.UID == UID and supervisor.DEVICE_UNIQUE_ID == DEVICE
            and supervisor.ROOT_FREE_BYTES == 64 * 1024 ** 3 and supervisor.MEMORY_AVAILABLE_BYTES == 128 * 1024 ** 3
            and supervisor.STAGE_BYTES == 2 * 1024 ** 3, "frozen resource limits drift")
    os.umask(0o077)
    placement = support.inherited_placement()
    output = stage / "v25-native-results"
    if args.execute:
        require(os.environ.get("FERRIC_V25_NATIVE_SUPERVISOR_PID") == str(os.getppid()), "live supervisor required")
        for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
            signal.signal(number, runner.interrupted)
        run_pair(plan, output, runner, support, helpers, placement)
        return 0
    lock = os.open(stage / "native.lock", os.O_CREAT | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
    try:
        metadata = os.fstat(lock)
        require(stat.S_ISREG(metadata.st_mode) and metadata.st_uid == UID and metadata.st_nlink == 1, "private native lock")
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        for name in ("v25-native-results", "launch.stdout", "launch.stderr", "launch-supervisor.json", "launch.status"):
            require(not os.path.lexists(stage / name), "fresh launch outputs required")
        script = runner.identity(Path(__file__).resolve())
        runner.save(stage / "launch-placement.json", placement)
        os.environ.update({"PATH": "/usr/bin:/bin", "LC_ALL": "C", "PYTHONDONTWRITEBYTECODE": "1",
                           "PYTHONNOUSERSITE": "1", "FERRIC_V25_NATIVE_SUPERVISOR_PID": str(os.getpid())})
        for key in ("LD_PRELOAD", "LD_LIBRARY_PATH", "PYTHONPATH"):
            os.environ.pop(key, None)
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        resource.setrlimit(resource.RLIMIT_AS, (128 * 1024 ** 3, 128 * 1024 ** 3))
        stopped = []
        for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
            signal.signal(number, lambda value, frame: stopped.append(value) if not stopped else None)
        def admission(initial):
            if initial:
                profile.gpu_idle()
            return supervisor.admission(stage, initial)
        argv = [sys.executable, "-B", str(Path(__file__).resolve()), "--execute", "--plan", str(args.plan), "--plan-sha256", args.plan_sha256]
        result = supervisor.supervise(argv, stage, admission, lambda: stopped[0] if stopped else None, duration=2400)
        try:
            result["kfd_after"] = supervisor.kfd_snapshot(stage)
            require(not result["kfd_after"]["stage_worker_pids"], "owned workers survived; retain stage")
            require(not result["kfd_after"]["foreign_pids"], "foreign KFD appeared; leave untouched")
            require(runner.bound_json(binding) == plan and load_support(directory)[4] == helpers, "launch inputs changed")
            runner.identity(script["path"], script["sha256"])
            if result["status"] == 0:
                retained, receipt = profile.read_retained(runner, output / "report.json", profile.SIDECAR_LIMIT)
                require(retained.get("accepted") is True and retained.get("launch_placement") == placement, "unaccepted pair/placement")
                result["v25_native_report"] = receipt
        except BaseException as error:
            result["errors"].append("post-run: " + type(error).__name__ + ": " + str(error))
            result["cleanup_ok"] = False
            result["status"] = 125
        runner.save(stage / "launch-supervisor.json", result)
        (stage / "launch.status").write_text(str(result["status"]) + "\n")
        return result["status"]
    finally:
        os.close(lock)


if __name__ == "__main__":
    raise SystemExit(main())
