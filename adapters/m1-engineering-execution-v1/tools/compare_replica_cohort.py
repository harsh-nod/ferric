#!/usr/bin/env python3
"""Read-only, fail-closed comparison of one fixed eight-request replica cohort."""

import argparse
import importlib.util
import json
from pathlib import Path, PurePosixPath
import re


SPEC = importlib.util.spec_from_file_location("replica_trace", Path(__file__).with_name("replica_trace.py"))
TRACE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TRACE)
BASE = TRACE.BASE
require, integer, fields = BASE.require, BASE.integer, BASE.fields
LAYOUTS = {"1xTP8": 8, "4xTP2": 2, "8xTP1": 1}
POLICY = {"batch_tokens": (16, 1, 32), "prefill_chunk": (16, 1, 32),
          "context_tokens": (128, 12, 2048), "physical_pages": (64, 1, 512),
          "cache_ttl_ticks": (1024, 1, BASE.U64_MAX), "max_batches": (240, 1, 10000)}
OPTION_KEYS = {"--batch-tokens": "batch_tokens", "--prefill-chunk": "prefill_chunk",
               "--context": "context_tokens", "--pages": "physical_pages",
               "--cache-ttl": "cache_ttl_ticks", "--max-batches": "max_batches"}
FLAG_KEYS = {"--runtime-cache-admission": "runtime_cache_admission", "--runtime-operational": "runtime_operational",
             "--dispatch-sequences": "dispatch_sequences", "--queue-rollover": "queue_rollover"}
EXPECT_FIELDS = BASE.IDENTITY_FIELDS | set(POLICY) | {
    "schema", "layout", "device_unique_ids", "reference_sha256", "cohort_path", "source_path", "artifact_path",
    "source_controller_path", "source_worker_path", "snapshot_command", "clock_domain", "nonce", "settings",
    "host_reserve_bytes", "controller_options", "kernel_profile", "performance_profile", "output_head_pruning",
    "row_policy"}


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode() + b"\n"


def same(actual, expected, label):
    # Canonical JSON comparison also distinguishes booleans, floats and integers.
    require(encoded(actual) == encoded(expected), f"{label} differs")


def clock_domain(value):
    fields(value, {"clock", "hostname", "boot_id", "time_namespace_dev", "time_namespace_ino"}, "clock domain")
    require(value["clock"] == "CLOCK_MONOTONIC_RAW", "common RAW clock required")
    require(type(value["hostname"]) is str and 0 < len(value["hostname"]) <= 255
            and not any(char.isspace() for char in value["hostname"]), "invalid hostname")
    require(type(value["boot_id"]) is str
            and re.fullmatch(r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}", value["boot_id"]), "invalid boot identity")
    integer(value["time_namespace_dev"], 1)
    integer(value["time_namespace_ino"], 1)


def settings(value):
    limits = {"ready_timeout_ns": (1_000_000, 3_600_000_000_000),
              "run_timeout_ns": (1_000_000, 14_400_000_000_000),
              "start_lead_ns": (10_000_000, 10_000_000_000), "max_lateness_ns": (1, 1_000_000_000)}
    fields(value, set(limits), "control settings")
    for key, bounds in limits.items():
        integer(value[key], *bounds, key)


def absolute_path(value):
    require(type(value) is str and 0 < len(value) <= 4096 and "\x00" not in value
            and value.startswith("/") and ".." not in PurePosixPath(value).parts
            and str(PurePosixPath(value)) == value, "canonical absolute recorded path required")
    return value


def options_policy(options):
    require(type(options) is list and len(options) <= 32
            and all(type(item) is str for item in options), "bounded controller options required")
    result = {key: default for key, (default, _, _) in POLICY.items()}
    result.update(kernel_profile="v2", output_head_pruning=False,
                  performance_profile={**{key: False for key in BASE.PROFILE_BOOLS},
                                       "projection": "baseline", "attention": "baseline"})
    seen, cursor = set(), 0
    while cursor < len(options):
        flag = options[cursor]
        cursor += 1
        require(flag not in seen, "duplicate controller option")
        seen.add(flag)
        if flag == "--prune-output-head":
            result["output_head_pruning"] = True
        elif flag in FLAG_KEYS:
            result["performance_profile"][FLAG_KEYS[flag]] = True
        else:
            require(flag in set(OPTION_KEYS) | {"--kernel-profile", "--projection", "--attention"}
                    and cursor < len(options), "unsupported controller option")
            value = options[cursor]
            cursor += 1
            if flag in OPTION_KEYS:
                require(re.fullmatch(r"[1-9][0-9]{0,19}", value), "canonical positive numeric option required")
                result[OPTION_KEYS[flag]] = int(value)
            elif flag == "--kernel-profile":
                result["kernel_profile"] = value
            else:
                result["performance_profile"][flag[2:]] = value
    return result


def expectation(value):
    fields(value, EXPECT_FIELDS, "external expectation")
    require(value["schema"] == "FerricReplicaExpectationV1", "unknown expectation schema")
    require(type(value["layout"]) is str and value["layout"] in LAYOUTS, "unsupported replica layout")
    ids = BASE.integer_list(value["device_unique_ids"], 1)
    require(len(ids) == 8 and len(set(ids)) == 8, "exact eight unique physical IDs required")
    for key in BASE.IDENTITY_FIELDS:
        BASE.hash_value(value[key], key)
    require(value["reference_sha256"] == BASE.REFERENCE_SHA256, "frozen reference pin required")
    for key in ("cohort_path", "source_path", "artifact_path", "source_controller_path", "source_worker_path"):
        absolute_path(value[key])
    command = value["snapshot_command"]
    require(type(command) is list and 1 <= len(command) <= 32
            and all(type(arg) is str and 0 < len(arg) <= 4096 and "\x00" not in arg for arg in command),
            "expected snapshot argv required")
    clock_domain(value["clock_domain"])
    BASE.hash_value(value["nonce"], "external cohort nonce")
    settings(value["settings"])
    integer(value["host_reserve_bytes"], 32 * 1024**3)
    for key, (_, low, high) in POLICY.items():
        integer(value[key], low, high, key)
    capacity = 32 if value["kernel_profile"] in BASE.WIDE_KERNEL_PROFILES else 16
    require(1 <= value["prefill_chunk"] <= value["batch_tokens"] <= capacity, "row/chunk capacity differs")
    require(value["kernel_profile"] in ("v2", "v3-wave", "v3-mfma", *BASE.WIDE_KERNEL_PROFILES), "unknown kernel image profile")
    profile = BASE.performance_profile(value["performance_profile"])
    require(profile["runtime_profiling"] is False, "cohort launcher cannot enable profiling")
    require(type(value["output_head_pruning"]) is bool, "pruning must be boolean")
    if value["kernel_profile"] == "v2":
        require(profile["projection"] == profile["attention"] == "baseline", "v2 image arithmetic differs")
    if value["kernel_profile"] in ("v3-wave", "v5-wave32"):
        require(profile["projection"] in ("baseline", "wave"), "wave image lacks MFMA projection")
    resolved = options_policy(value["controller_options"])
    for key, actual in resolved.items():
        same(actual, value[key], f"controller option policy {key}")
    replicas = 8 // LAYOUTS[value["layout"]]
    fields(value["row_policy"], {"scope", "row_budget"}, "row policy")
    scope = value["row_policy"]["scope"]
    require(scope in ("per-instance", "fixed-total"), "explicit row policy scope required")
    budget = value["batch_tokens"] * (replicas if scope == "fixed-total" else 1)
    require(integer(value["row_policy"]["row_budget"], 1) == budget, "row policy accounting differs")
    return value


def expected_plan(expect):
    world = LAYOUTS[expect["layout"]]
    count = 8 // world
    workload = {"schema": "FerricReplicaFixedWorkloadV1", "model_bundle_id": BASE.BUNDLE,
        "reference_sha256": BASE.REFERENCE_SHA256, "dtype": "BF16", "greedy": True, "prefix_cache": False,
        "arrival_offset_ns": 0, "cancellation": False, "total_output_tokens": 64,
        "requests": [{"name": name, "prompt": BASE.PROMPT, "prompt_tokens": BASE.PROMPT_IDS,
                      "output_tokens": BASE.REFERENCE_IDS[:8],
                      "output_utf8_bytes": list("".join(BASE.REFERENCE_PIECES[:8]).encode())} for name in TRACE.NAMES]}
    replicas = []
    for index in range(count):
        names = TRACE.NAMES[index::count]
        requests = {"schema": "FerricQwen3TpWorkloadV2", "requests": [
            {"name": name, "prompt": BASE.PROMPT, "new_tokens": 8, "arrival_tick": 0} for name in names]}
        replicas.append({"replica_id": f"replica-{index:02}",
            "device_unique_ids": expect["device_unique_ids"][index * world:(index + 1) * world],
            "request_names": names, "requests": requests, "requests_sha256": BASE.sha256(encoded(requests))})
    gpu_bytes = TRACE.weight_payload(world, "baseline")["device_base"]
    return {"schema": "FerricReplicaPlanV1", "authority": "none", "layout": expect["layout"],
        "device_unique_ids": expect["device_unique_ids"], "tensor_parallel": world, "replicas": replicas,
        "workload": workload, "workload_sha256": BASE.sha256(encoded(workload)),
        "controller_options": expect["controller_options"], "collective": "host-staged-v1",
        "retained_target_bytes": count * TRACE.TARGET_BYTES, "per_instance_base_gpu_weight_bytes": gpu_bytes,
        "aggregate_base_gpu_weight_bytes": count * gpu_bytes,
        "host_headroom_rule": "three times retained target bytes plus explicit reserve",
        "per_instance_row_budget": expect["batch_tokens"], "total_row_budget": count * expect["batch_tokens"],
        "model_parity_qualified": False}


class Inputs:
    def __init__(self, root):
        self.root, self.hashes = Path(root), {}

    def raw(self, name, limit=1024**2):
        path = self.root / name
        require(not any(parent.is_symlink() for parent in (path, *path.parents)), "symlinked evidence path")
        data = BASE.read_bounded(path, limit)
        self.hashes[name] = BASE.sha256(data)
        return data

    def json(self, name):
        return BASE.json_value(self.raw(name))

    def lines(self, name, limit, count, frame_limit=None):
        data = self.raw(name, limit)
        require(data.endswith(b"\n"), "partial trailing JSON frame")
        lines = data.splitlines(keepends=True)
        require(1 <= len(lines) <= count and all(line.strip() for line in lines), "invalid JSON frame count")
        if frame_limit is not None:
            require(all(len(line) <= frame_limit for line in lines), "control frame exceeds byte bound")
        return [BASE.json_value(line) for line in lines]


def frame(kind, identity, extra):
    return {"schema": "FerricReplica" + kind + "V1", "authority": "none", "identity": identity, **extra}


def validate_control(inputs, cohort, plan, expect):
    control = cohort["control"]
    fields(control, {"clock_domain", "nonce", "settings", "first_spawn_ns", "all_ready_ns", "epoch_ns", "replicas"}, "control")
    for key in ("clock_domain", "nonce", "settings"):
        same(control[key], expect[key], f"control {key}")
    first = integer(control["first_spawn_ns"], 1)
    ready_at = integer(control["all_ready_ns"], first, first + expect["settings"]["ready_timeout_ns"] - 1)
    epoch = integer(control["epoch_ns"], ready_at + 1)
    require(epoch == ready_at + expect["settings"]["start_lead_ns"], "common epoch differs")
    states = control["replicas"]
    require(type(states) is list and len(states) == len(plan["replicas"]), "incomplete controller cohort")
    events = inputs.lines("control-events.jsonl", 256 * 1024, 40, 8192)
    require(len(events) == 5 * len(states), "missing or extra control events")
    grouped = {replica["replica_id"]: [] for replica in plan["replicas"]}
    observed = first
    for event in events:
        fields(event, {"observed_ns", "replica_id", "direction", "frame"}, "control event")
        observed = integer(event["observed_ns"], observed, epoch + expect["settings"]["run_timeout_ns"])
        require(type(event["replica_id"]) is str and event["replica_id"] in grouped, "unknown event replica")
        grouped[event["replica_id"]].append(event)
    results, controllers, launchers = [], [], []
    prior_spawn = first
    for state, replica in zip(states, plan["replicas"], strict=True):
        fields(state, {"replica", "identity", "pid", "spawned_ns", "argv", "ready", "started", "closed", "eof_ns",
                       "process_identity", "start_sent_ns"}, "replica process state")
        same(state["replica"], replica, "process replica assignment")
        identity = {"nonce": expect["nonce"], "replica_id": replica["replica_id"],
                    "workload_sha256": plan["workload_sha256"], "requests_sha256": replica["requests_sha256"],
                    "device_unique_ids": replica["device_unique_ids"], "clock_domain": expect["clock_domain"]}
        same(state["identity"], identity, "process identity")
        pid = integer(state["pid"], 1, (1 << 31) - 1)
        controllers.append(pid)
        spawned = integer(state["spawned_ns"], prior_spawn, ready_at)
        prior_spawn = spawned
        process = state["process_identity"]
        fields(process, {"pid", "state", "pgid", "session", "start_ticks"}, "Linux process identity")
        for key in ("pid", "pgid", "session"):
            require(integer(process[key], 1) == pid, "owned controller process group/session differs")
        integer(process["start_ticks"], 1)
        require(type(process["state"]) is str and process["state"] in ("R", "S", "D", "T", "t", "I"), "controller was not live at launch")
        directory = replica["replica_id"]
        original = expect["cohort_path"] + "/" + directory
        requests = inputs.raw(directory + "/requests.json")
        require(BASE.sha256(requests) == replica["requests_sha256"], "actual request file differs")
        same(BASE.json_value(requests), replica["requests"], "request file semantics")
        config = inputs.json(directory + "/control.json")
        launcher = integer(config.get("launcher_pid"), 1, (1 << 31) - 1)
        launchers.append(launcher)
        expected_config = {"schema": "FerricReplicaControlConfigV1", "socket_path": expect["cohort_path"] + "/control.sock",
            "launcher_pid": launcher, "identity": identity, "io_timeout_ms": expect["settings"]["ready_timeout_ns"] // 1_000_000,
            "min_start_lead_ns": 1_000_000, "max_start_lead_ns": 10_000_000_000,
            "max_lateness_ns": expect["settings"]["max_lateness_ns"]}
        same(config, expected_config, "controller control configuration")
        argv = [expect["cohort_path"] + "/launch-artifacts/controller", "--source", expect["source_path"],
            "--artifact", expect["artifact_path"], "--worker", expect["cohort_path"] + "/launch-artifacts/worker",
            "--devices", ",".join(map(str, replica["device_unique_ids"])), "--requests", original + "/requests.json",
            "--benchmark-control", original + "/control.json", "--disable-prefix-cache",
            "--allow-unauthenticated-machine-code", "--collective", "host-staged-v1"] + expect["controller_options"]
        same(state["argv"], argv, "launched controller argv")
        same(inputs.json(directory + "/launch.json"), {"pid": pid, "spawned_ns": spawned,
             "process_identity": process, "argv": argv}, "retained process launch receipt")
        stream = grouped[directory]
        same([item["direction"] for item in stream], ["receive", "send", "receive", "receive", "send"], "control event order")
        ready, started, closed = state["ready"], state["started"], state["closed"]
        ready_ns = integer(ready.get("ready_ns"), spawned, stream[0]["observed_ns"])
        same(ready, frame("Ready", identity, {"pid": pid, "ready_ns": ready_ns}), "Ready receipt")
        require(stream[0]["observed_ns"] <= ready_at, "Ready observed after global barrier")
        sent = integer(state["start_sent_ns"], ready_at, epoch - 1)
        require(stream[1]["observed_ns"] == sent, "Start send timestamp differs")
        received = integer(started.get("start_received_ns"), sent, epoch - 1)
        actual = integer(started.get("started_ns"), epoch, stream[2]["observed_ns"])
        late = integer(started.get("lateness_ns"), 0, expect["settings"]["max_lateness_ns"])
        require(actual - epoch == late, "release lateness arithmetic differs")
        same(started, frame("Started", identity, {"pid": pid, "ready_ns": ready_ns, "start_received_ns": received,
             "epoch_ns": epoch, "started_ns": actual, "lateness_ns": late}), "Started receipt")
        closed_at = integer(closed.get("closed_ns"), actual, stream[3]["observed_ns"])
        same(closed, frame("Closed", identity, {"pid": pid, "epoch_ns": epoch, "closed_ns": closed_at}), "Closed receipt")
        expected_frames = [ready, frame("Start", identity, {"epoch_ns": epoch}), started, closed,
                           frame("CloseAck", identity, {"epoch_ns": epoch})]
        same([item["frame"] for item in stream], expected_frames, "control frames versus state")
        wire = inputs.lines(directory + "/control-received.bin", 4 * 4096, 3, 4096)
        same(wire, [ready, started, closed], "raw received control frames")
        eof = integer(state["eof_ns"], stream[4]["observed_ns"], epoch + expect["settings"]["run_timeout_ns"])
        trace = inputs.lines(directory + "/stdout.jsonl", 64 * 1024**2, 256)
        result = TRACE.validate_trace(trace, expect, replica, started, closed)
        result.update(controller_pid=pid, spawned_ns=spawned, ready_ns=ready_ns,
                      started_ns=actual, closed_ns=closed_at, eof_ns=eof, lateness_ns=late)
        results.append(result)
    require(states[0]["spawned_ns"] == first, "first spawn identity differs")
    require(len(set(controllers)) == len(states) and len(set(launchers)) == 1
            and not set(launchers) & set(controllers), "duplicate controller or launcher PID")
    workers = [pid for result in results for pid in result["worker_pids"]]
    require(len(workers) == len(set(workers)) == 8 and not set(workers) & (set(controllers) | set(launchers)),
            "worker process reuse across replicas")
    require(len({result["session_id"] for result in results}) == len(results), "session replay across replicas")
    return results


def compare(cohort_dir, reference_path, expect):
    expect = expectation(expect)
    BASE.load_reference(BASE.read_bounded(reference_path, 1024**2))
    inputs = Inputs(cohort_dir)
    plan = expected_plan(expect)
    same(inputs.json("plan.json"), plan, "fixed partition/workload/resource plan")
    cohort = inputs.json("cohort.json")
    fields(cohort, {"schema", "authority", "status", "model_parity_qualified", "controller_sha256", "worker_sha256",
        "snapshot_command", "workload_sha256", "error", "clock_domain", "gpu_snapshot_intervals", "executables",
        "control", "group_termination", "exit_codes", "final_reap_ns"}, "complete cohort")
    require(cohort["schema"] == "FerricReplicaCohortV1" and cohort["authority"] == "none"
            and cohort["status"] == "unvalidated-complete" and cohort["model_parity_qualified"] is False
            and cohort["error"] is None, "launcher did not complete cleanly")
    for key in ("controller_sha256", "worker_sha256", "clock_domain", "snapshot_command"):
        same(cohort[key], expect[key], f"cohort {key}")
    require(cohort["workload_sha256"] == plan["workload_sha256"], "cohort workload identity differs")
    for binary in ("controller", "worker"):
        require(BASE.sha256(inputs.raw("launch-artifacts/" + binary, 256 * 1024**2)) == expect[binary + "_sha256"],
                "retained frozen executable differs")
    same(cohort["executables"], {"controller": expect["cohort_path"] + "/launch-artifacts/controller",
         "worker": expect["cohort_path"] + "/launch-artifacts/worker", "source_controller": expect["source_controller_path"],
         "source_worker": expect["source_worker_path"]}, "executable custody paths")
    require(BASE.gpu_roster(inputs.raw("gpu-before.json")) == BASE.gpu_roster(inputs.raw("gpu-after.json"))
            == expect["device_unique_ids"], "global idle physical roster differs")
    available = []
    for line in inputs.raw("meminfo-before.txt").decode("ascii").splitlines():
        if line.startswith("MemAvailable:"):
            match = re.fullmatch(r"MemAvailable:\s+([0-9]+)\s+kB", line)
            require(match is not None, "invalid available-memory receipt")
            available.append(integer(int(match[1])) * 1024)
    require(len(available) == 1, "unique MemAvailable receipt required")
    required = 3 * plan["retained_target_bytes"] + expect["host_reserve_bytes"]
    require(available[0] >= required, "concurrent host headroom not satisfied")
    same(inputs.json("memory-headroom.json"), {"available_bytes": available[0], "required_bytes": required,
         "retained_target_bytes": plan["retained_target_bytes"], "reserve_bytes": expect["host_reserve_bytes"],
         "transient_multiplier": 3}, "memory headroom accounting")
    replicas = validate_control(inputs, cohort, plan, expect)
    same(cohort["exit_codes"], [0] * len(replicas), "all controller exit statuses")
    termination = cohort["group_termination"]
    last_eof = max(replica["eof_ns"] for replica in replicas)
    quiet_at = integer(termination.get("observed_ns"), last_eof)
    same(termination, {"schema": "FerricReplicaGroupTerminationV1", "observed_ns": quiet_at,
         "controller_pids": [replica["controller_pid"] for replica in replicas], "live_group_members": [],
         "scope": "owned process groups; descendants must not detach", "confirmed": True}, "owned process-group termination")
    reap = integer(cohort["final_reap_ns"], quiet_at)
    intervals = cohort["gpu_snapshot_intervals"]
    require(type(intervals) is list and len(intervals) == 2, "exact global before/after snapshot intervals required")
    for phase, interval in zip(("before", "after"), intervals, strict=True):
        fields(interval, {"phase", "start_ns", "end_ns", "completed"}, "GPU snapshot interval")
        require(interval["phase"] == phase and interval["completed"] is True, "GPU snapshot not completed")
        integer(interval["end_ns"], integer(interval["start_ns"], 1))
    require(intervals[0]["end_ns"] <= cohort["control"]["first_spawn_ns"]
            and reap <= intervals[1]["start_ns"], "global idle snapshots do not bracket the whole cohort")
    requests = {name: request for replica in replicas for name, request in replica["requests"].items()}
    require(sorted(requests) == TRACE.NAMES and sum(request["output_tokens"] for request in requests.values()) == 64,
            "exact eight global requests / 64 output tokens required")
    first_admission = min(request["arrival_epoch_offset_ns"] for request in requests.values())
    last_output = max(request["output_epoch_offsets_ns"][-1] for request in requests.values())
    integer(last_output, first_admission + 1)
    payload = {key: sum(replica["weight_payload_bytes"][key] for replica in replicas)
               for key in ("host_target", "device_base", "device_transposed")}
    return {"schema": "FerricReplicaComparisonV1", "authority": "none", "passed": True,
        "scope": "one fixed short eight-request / 64-output-token engineering cohort; not steady state or a serving SLO",
        "qualification": "exact frozen reference tokens and bytes for this cohort only",
        "custody_scope": "retained receipts and externally pinned paths/hashes; not authenticated execution attestation",
        "process_scope": "owned process groups, assuming descendants do not detach; not a cgroup-wide proof",
        "expectation": expect, "expectation_sha256": BASE.sha256(encoded(expect)), "input_sha256": inputs.hashes,
        "checker_sha256": {name: BASE.sha256(BASE.read_bounded(Path(__file__).with_name(name), 1024**2))
                           for name in ("compare_replica_cohort.py", "replica_trace.py", "compare_tp_batch.py")},
        "workload_sha256": plan["workload_sha256"], "reference_sha256": BASE.REFERENCE_SHA256,
        "replica_count": len(replicas), "requests": requests, "replicas": replicas,
        "global_output_tokens": 64, "physical_token_rows": 96, "repetition_count": 1,
        "latency_scope": "seven decode intervals per request; never pool different request identities as repetitions",
        "row_policy": expect["row_policy"], "per_instance_row_budget": expect["batch_tokens"],
        "total_row_budget": plan["total_row_budget"], "weight_payload_bytes": payload,
        "weight_payload_scope": "loaded BF16 payload only; excludes KV, activations, scratch, allocator and page rounding",
        "global_release_epoch_ns": cohort["control"]["epoch_ns"],
        "release_to_last_output_ns": last_output, "global_output_tokens_per_second": 64 * 1e9 / last_output,
        "admission_window_ns": last_output - first_admission,
        "admission_window_output_tokens_per_second": 64 * 1e9 / (last_output - first_admission),
        "barrier_setup_ns": cohort["control"]["all_ready_ns"] - cohort["control"]["first_spawn_ns"],
        "whole_cohort_spawn_to_reap_ns": reap - cohort["control"]["first_spawn_ns"],
        "release_to_all_closed_ns": max(replica["closed_ns"] for replica in replicas) - cohort["control"]["epoch_ns"],
        "all_controllers_reaped": True, "global_before_after_idle": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cohort-dir", type=Path, required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--expect", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = compare(args.cohort_dir, args.reference, BASE.json_value(BASE.read_bounded(args.expect, 1024**2)))
    with args.output.open("x", encoding="ascii") as output:
        json.dump(report, output, indent=2, sort_keys=True, allow_nan=False)
        output.write("\n")
    print("PASS: eight fixed requests, 64 reference output tokens, common-clock receipts, global idle/reap")


if __name__ == "__main__":
    main()
