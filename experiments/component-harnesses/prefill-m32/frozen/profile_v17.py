#!/usr/bin/env python3
"""One combined V17 host-wall diagnostic using the frozen V4 process owners."""

import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import resource
import signal
import socket
import stat
import sys

STAGE = Path("/tmp/ferric-opt-v5.Q8EAzf7k")
HOST = "smci350-rck-g03-b19-03"
UID = 9661
DEVICE = "16366993098680759275"
PINS = {
    "run_v17_native.py": "810a22cb4e805662a7575e2e4ec261fe161a43405f675a7b639f45a62664301f",
    "native_supervisor.py": "0c2a7cacbffbe74827ade03fd09f05a927cb9ba051ff475744be8d79f3a96436",
}
SIDECAR_LIMIT = 32 * 1024 * 1024
RECORD_LIMIT = 65536
PLAN_KEYS = {"schema", "common_args", "controller", "workload", "reference", "target_manifest",
             "warmup_requests", "measured_requests", "modes", "timeouts"}
CAVEAT = ("Overlapping controller host-wall scopes are not additive or GPU durations; "
          "ordered operation spans can measure queued command creation, with execution in later flushes. "
          "One instrumented diagnostic is not HTTP, sustained throughput, or a competitive comparison.")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load_dependencies(directory):
    modules = []
    identities = {}
    for name, expected in PINS.items():
        path = directory / name
        require(path.resolve(strict=True) == path and path.is_file() and not path.is_symlink(),
                "regular canonical pinned helper required")
        with path.open("rb") as stream:
            before = os.fstat(stream.fileno())
            require(0 < before.st_size <= 1024 * 1024, "helper extent")
            source = stream.read()
            after = os.fstat(stream.fileno())
        current = path.stat()
        identity_fields = ("st_dev", "st_ino", "st_size", "st_mode", "st_mtime_ns", "st_ctime_ns")
        require(all(getattr(before, field) == getattr(after, field) == getattr(current, field)
                    for field in identity_fields) and hashlib.sha256(source).hexdigest() == expected,
                "pinned helper bytes differ: " + name)
        module_name = "ferric_v5_frozen_" + path.stem
        spec = importlib.util.spec_from_loader(module_name, loader=None, origin=str(path))
        module = importlib.util.module_from_spec(spec)
        module.__file__ = str(path)
        # Execute exactly the verified bytes, not a second path lookup or a .pyc cache.
        exec(compile(source, str(path), "exec"), module.__dict__)
        modules.append(module)
        identities[name] = {"path": str(path), "bytes": before.st_size, "sha256": expected}
    return modules[0], modules[1], identities


def validate_plan(plan, runner, stage):
    require(type(plan) is dict and set(plan) == PLAN_KEYS, "closed native plan fields required")
    require(plan["schema"] == "FerricWaveTargetV17NativePlanV1", "native plan schema")
    require(plan["modes"] == list(runner.MODES), "preserve the source plan's closed four-mode roster")
    for key in ("warmup_requests", "measured_requests"):
        require(type(plan[key]) is int and plan[key] == 1, "exactly one warmup and one diagnostic required")
    require(plan["timeouts"] == {"setup_seconds": 600, "request_seconds": 180, "arm_seconds": 1200}
            and all(type(value) is int for value in plan["timeouts"].values()), "fixed diagnostic timeouts")
    for key in ("controller", "workload", "reference", "target_manifest"):
        require(type(plan[key]) is dict and set(plan[key]) == {"path", "sha256"}, "closed input binding")
        runner.sha(plan[key]["sha256"])
    options = runner.arguments(plan["common_args"])
    require(options["--device-unique-id"] == DEVICE, "wrong GPU identity")
    require(options["--worker"] == str(stage / "worker-candidate"), "wrong owned worker path")
    require(plan["controller"]["path"] == str(stage / "controller-v17"), "wrong owned controller path")
    return options


def nonnegative(value):
    return type(value) is int and 0 <= value <= (1 << 64) - 1


def validate_sidecar(sidecar, setup, closed, process, arm, argv):
    require(type(sidecar) is dict, "sidecar object required")
    expected = {"schema": "FerricHostTimingV1", "clock": "controller-std-instant",
                "measurement": "host-wall-latency-not-gpu-duration", "aggregation": "overlapping-not-additive",
                "payload_accounting": "payload-only-excludes-wire-headers", "record_limit": RECORD_LIMIT,
                "incomplete": False, "active_records": 0, "run_status": "completed", "failure": None,
                "workload_sha256": None}
    require(set(sidecar) == set(expected) | {"controller_pid", "setup", "closed", "records"},
            "closed timing sidecar fields required")
    for key, value in expected.items():
        require(type(sidecar.get(key)) is type(value) and sidecar[key] == value, "sidecar mismatch: " + key)
    require(sidecar["setup"] == setup and sidecar["closed"] == closed, "sidecar setup/close differs from stdout")
    require(type(sidecar["controller_pid"]) is int and sidecar["controller_pid"] > 0
            and sidecar["controller_pid"] == process.get("pid") == process.get("owned_pgid")
            and process.get("start_new_session") is True and process.get("argv") == argv,
            "sidecar process binding mismatch")
    require(arm.get("accepted") is True and arm.get("batches") == 270
            and arm.get("dispatches") == 166278 and len(arm.get("requests", [])) == 2,
            "exact two-request completed diagnostic required")
    require(arm["requests"][0].get("warmup") is True and arm["requests"][1].get("warmup") is False,
            "warmup boundary differs")
    records = sidecar["records"]
    require(type(records) is list and 0 < len(records) <= RECORD_LIMIT, "bounded nonempty records required")
    keys = set()
    batches = set()
    packets = 0
    numeric = {"count", "failed", "elapsed_ns", "max_ns", "request_payload_bytes",
               "response_payload_bytes", "dispatches"}
    for row in records:
        require(type(row) is dict and set(row) == numeric | {"batch", "phase", "category", "label", "rank"},
                "closed timing record required")
        require(all(nonnegative(row[field]) for field in numeric), "nonnegative u64 timing counters required")
        require(row["count"] > 0 and row["failed"] == 0 and row["max_ns"] <= row["elapsed_ns"]
                <= row["count"] * row["max_ns"], "complete coherent timing totals required")
        require(row["category"] in ("span", "ipc_send", "ipc_roundtrip"), "unknown timing category")
        require(all(type(row[field]) is str and 0 < len(row[field]) <= 96 for field in ("phase", "label")),
                "bounded timing labels required")
        require(row["rank"] is None or (type(row["rank"]) is int and row["rank"] == 0), "TP1 timing rank")
        require(row["batch"] is None or (type(row["batch"]) is int and 1 <= row["batch"] <= 270),
                "diagnostic batch identity")
        key = tuple(row[field] for field in ("batch", "phase", "category", "label", "rank"))
        require(key not in keys, "duplicate aggregate timing key")
        keys.add(key)
        if row["category"] == "span" and row["phase"] == row["label"] == "batch":
            require(row["count"] == 1 and row["batch"] is not None, "one span per physical batch")
            batches.add(row["batch"])
        if row["category"] == "ipc_roundtrip":
            packets += row["dispatches"]
    require(batches == set(range(1, 271)), "timing sidecar omits a physical batch")
    require(packets == arm["dispatches"], "timing packet coverage differs from completed stream")
    return {"records": len(records), "physical_batches": len(batches), "completed_dispatches": packets,
            "clock": sidecar["clock"], "caveat": CAVEAT}


def read_retained(runner, path, maximum):
    receipt = runner.identity(path, maximum=maximum)
    value = runner.decode(path.read_bytes())
    runner.identity(path, receipt["sha256"], maximum)
    return value, receipt


def run_profile(plan, output, stage, runner, helper_identities):
    output.mkdir(mode=0o700)
    report = {"schema": "FerricWaveTargetV17HostProfileV1", "authority": "none", "accepted": False,
              "selected_mode": "combined", "source_plan_modes_executed": False,
              "http": False, "sustained": False, "vendor_comparison": False,
              "performance_qualified": False, "formal_qualification": False,
              "warmup_requests": 1, "diagnostic_requests": 1, "instrumented": True,
              "caveat": CAVEAT, "helpers": helper_identities}
    runner.save(output / "plan.json", plan)
    try:
        options = validate_plan(plan, runner, stage)
        request, reference = runner.workload_reference(plan)
        inputs = runner.input_identities(plan, options)
        runner.save(output / "inputs.json", inputs)
        model = runner.model_identities(options)
        runner.save(output / "model-inputs.json", model)
        require(runner.input_identities(plan, options) == inputs, "inputs changed before diagnostic")
        sidecar_path = output / "host-timing.json"
        require(not os.path.lexists(sidecar_path), "host timing output must be fresh")
        argv = [inputs["controller"]["path"], *plan["common_args"], "--wave-target-mode", "combined",
                "--host-timing", str(sidecar_path)]
        setups = []
        def checked_setup(value):
            identity = runner.check_setup(value, "combined", options, inputs)
            setups.append(value)
            return identity
        arm_output = output / "combined"
        arm = runner.run_arm(argv, arm_output, "combined", request, reference, 1,
                             plan["timeouts"], checked_setup)
        report["arm"] = arm
        require(len(setups) == 1, "one setup required")
        metadata = sidecar_path.lstat()
        require(stat.S_ISREG(metadata.st_mode) and metadata.st_uid == os.getuid()
                and stat.S_IMODE(metadata.st_mode) == 0o600 and metadata.st_nlink == 1,
                "exclusive private timing sidecar required")
        sidecar, receipt = read_retained(runner, sidecar_path, SIDECAR_LIMIT)
        process, process_receipt = read_retained(runner, arm_output / "process.json", runner.MAX_LINE)
        stdout_receipt = runner.identity(arm_output / "stdout.raw", maximum=runner.MAX_STREAM)
        records = [runner.decode(line) for line in (arm_output / "stdout.raw").read_bytes().splitlines()]
        runner.identity(arm_output / "stdout.raw", stdout_receipt["sha256"], runner.MAX_STREAM)
        require(records and records[0] == setups[0], "retained setup mismatch")
        report["timing"] = validate_sidecar(sidecar, setups[0], records[-1], process, arm, argv)
        report["sidecar"] = receipt
        report["process_receipt"] = process_receipt
        report["stdout_receipt"] = stdout_receipt
        require(runner.input_identities(plan, options) == inputs, "inputs changed after diagnostic")
        runner.workload_reference(plan)
        require(runner.model_identities(options) == model, "model bytes changed during diagnostic")
        runner.save(output / "inputs-after.json", inputs)
        runner.save(output / "model-inputs-after.json", model)
        report["model_identity"] = arm["model_identity"]
        report["accepted"] = True
        return report
    except BaseException as error:
        report["error"] = type(error).__name__ + ": " + str(error)
        raise
    finally:
        runner.save(output / "report.json", report)


def gpu_idle():
    properties = {}
    for line in Path("/sys/class/kfd/kfd/topology/nodes/2/properties").read_text().splitlines():
        fields = line.split()
        require(len(fields) == 2 and fields[0] not in properties, "GPU topology field drift")
        properties[fields[0]] = fields[1]
    require(properties.get("unique_id") == DEVICE, "GPU0 identity drift")
    cards = list(Path("/sys/class/drm").glob("card*/device/gpu_busy_percent"))
    require(len(cards) == 8, "exactly eight inspectable GPUs required")
    require(len({path.parent.resolve(strict=True) for path in cards}) == 8, "aliased GPU census")
    for path in cards:
        require(path.read_text().strip() == "0", "GPU is busy")
        memory = (path.parent / "mem_info_vram_used").read_text().strip()
        require(memory.isdecimal() and int(memory) < 524288000, "GPU memory admission floor")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan-sha256", required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--supervise", action="store_true")
    mode.add_argument("--execute", action="store_true", help="internal supervised child only")
    args = parser.parse_args()
    require(socket.gethostname() == HOST and os.getuid() == UID, "wrong native host or UID")
    require(STAGE.resolve(strict=True) == STAGE and STAGE.is_dir(), "native stage redirected")
    owner = STAGE.stat()
    require(owner.st_uid == UID and stat.S_IMODE(owner.st_mode) == 0o700, "native stage ownership drift")
    os.umask(0o077)
    directory = Path(__file__).resolve().parent
    require(directory == STAGE / "profile", "profile helper outside owned stage")
    runner, supervisor, helpers = load_dependencies(directory)
    require(supervisor.HOST == HOST and supervisor.UID == UID and supervisor.DEVICE_UNIQUE_ID == DEVICE
            and supervisor.ROOT_FREE_BYTES == 64 * 1024 ** 3
            and supervisor.MEMORY_AVAILABLE_BYTES == 128 * 1024 ** 3
            and supervisor.STAGE_BYTES == 2 * 1024 ** 3, "frozen supervisor bounds drift")
    plan_path = STAGE / "profile-plan.json"
    plan = runner.bound_json({"path": str(plan_path), "sha256": args.plan_sha256})
    validate_plan(plan, runner, STAGE)
    output = STAGE / "profile-results"
    if args.execute:
        require(os.environ.get("FERRIC_PROFILE_SUPERVISOR_PID") == str(os.getppid()),
                "profile child requires its live supervisor")
        for number in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            signal.signal(number, runner.interrupted)
        run_profile(plan, output, STAGE, runner, helpers)
        return 0
    lock = os.open(STAGE / "native.lock", os.O_CREAT | os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
    try:
        item = os.fstat(lock)
        require(stat.S_ISREG(item.st_mode) and item.st_uid == UID and item.st_nlink == 1, "invalid native lock")
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        for name in ("profile-results", "launch.stdout", "launch.stderr", "launch-supervisor.json", "launch.status"):
            require(not os.path.lexists(STAGE / name), "fresh profile launch outputs required")
        script_identity = runner.identity(Path(__file__).resolve())
        os.environ.update({"PATH": "/usr/bin:/bin", "LC_ALL": "C", "PYTHONDONTWRITEBYTECODE": "1",
                           "PYTHONNOUSERSITE": "1", "FERRIC_PROFILE_SUPERVISOR_PID": str(os.getpid())})
        for key in ("LD_PRELOAD", "LD_LIBRARY_PATH", "PYTHONPATH"):
            os.environ.pop(key, None)
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        resource.setrlimit(resource.RLIMIT_AS, (128 * 1024 ** 3, 128 * 1024 ** 3))
        os.sched_setaffinity(0, {min(os.sched_getaffinity(0))})
        os.nice(19)
        stopped = []
        for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
            signal.signal(number, lambda value, frame: stopped.append(value) if not stopped else None)
        def admitted(initial):
            if initial:
                gpu_idle()
            return supervisor.admission(STAGE, initial)
        argv = [sys.executable, "-B", str(Path(__file__).resolve()), "--execute", "--plan-sha256", args.plan_sha256]
        result = supervisor.supervise(argv, STAGE, admitted, lambda: stopped[0] if stopped else None, duration=1200)
        try:
            result["kfd_after"] = supervisor.kfd_snapshot(STAGE)
            require(not result["kfd_after"]["stage_worker_pids"], "owned stage workers survived; retain stage")
            require(not result["kfd_after"]["foreign_pids"], "foreign KFD work appeared; leave untouched")
            runner.bound_json({"path": str(plan_path), "sha256": args.plan_sha256})
            require(load_dependencies(directory)[2] == helpers, "pinned helpers changed")
            runner.identity(script_identity["path"], script_identity["sha256"])
            if result["status"] == 0:
                retained, receipt = read_retained(runner, output / "report.json", SIDECAR_LIMIT)
                require(retained.get("accepted") is True, "profile result was not accepted")
                result["profile_report"] = receipt
        except BaseException as error:
            result["errors"].append("post-run: " + type(error).__name__ + ": " + str(error))
            result["cleanup_ok"] = False
            result["status"] = 125
        runner.save(STAGE / "launch-supervisor.json", result)
        (STAGE / "launch.status").write_text(str(result["status"]) + "\n")
        print(json.dumps(result, sort_keys=True), flush=True)
        return result["status"]
    finally:
        os.close(lock)


if __name__ == "__main__":
    raise SystemExit(main())
