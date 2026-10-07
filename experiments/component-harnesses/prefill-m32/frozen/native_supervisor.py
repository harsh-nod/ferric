#!/usr/bin/env python3
"""Private-stage native probe supervision; never signals discovered KFD processes."""

import argparse
import datetime
import json
import os
from pathlib import Path
import signal
import socket
import stat
import subprocess
import sys
import time

STAGE = Path("/tmp/ferric-perf-v4-r2.UUuJAiA5")
HOST = "smci350-rck-g03-b19-03"
UID = 9661
DEVICE_UNIQUE_ID = "16366993098680759275"
ROOT_FREE_BYTES = 67108864 * 1024
MEMORY_AVAILABLE_BYTES = 134217728 * 1024
STAGE_BYTES = 2097152 * 1024
KFD_PROCESSES = Path("/sys/class/kfd/kfd/proc")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n")


def timestamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def require_facilities():
    for name in ("pidfd_open", "waitid", "P_PID", "WEXITED", "WNOHANG", "WNOWAIT"):
        require(hasattr(os, name), "required process facility unavailable: " + name)
    require(hasattr(signal, "pidfd_send_signal"), "pidfd signaling unavailable")
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL,
            "supervision requires default SIGCHLD disposition")
    descriptor = os.pidfd_open(os.getpid(), 0)
    try:
        signal.pidfd_send_signal(descriptor, 0)
    finally:
        os.close(descriptor)


def stage_usage(stage):
    total = inspected = 0
    seen = set()
    pending = [Path(stage)]
    deadline = time.monotonic() + 3
    while pending:
        path = pending.pop()
        inspected += 1
        require(inspected <= 65536 and time.monotonic() < deadline,
                "bounded stage census exceeded")
        observed = path.lstat()
        key = (observed.st_dev, observed.st_ino)
        if key in seen:
            continue
        seen.add(key)
        total += observed.st_blocks * 512
        if stat.S_ISDIR(observed.st_mode):
            with os.scandir(path) as entries:
                for entry in entries:
                    require(inspected + len(pending) < 65536 and time.monotonic() < deadline,
                            "bounded stage census exceeded")
                    pending.append(Path(entry.path))
    return total


def resource_snapshot(stage):
    filesystem = os.statvfs("/")
    available = None
    with open("/proc/meminfo", encoding="ascii") as handle:
        for line in handle:
            fields = line.split()
            if fields and fields[0] == "MemAvailable:":
                require(len(fields) == 3 and fields[2] == "kB", "invalid available-memory record")
                available = int(fields[1]) * 1024
                break
    require(available is not None, "available-memory record missing")
    result = {"root_free_bytes": filesystem.f_bavail * filesystem.f_frsize,
              "memory_available_bytes": available, "stage_bytes": stage_usage(stage)}
    require(result["root_free_bytes"] >= ROOT_FREE_BYTES, "root-disk floor reached")
    require(available >= MEMORY_AVAILABLE_BYTES, "available-memory floor reached")
    require(result["stage_bytes"] < STAGE_BYTES, "private-stage limit reached")
    return result


def kfd_snapshot(stage, directory=KFD_PROCESSES):
    require(directory.is_dir(), "KFD process directory missing")
    result = {"stage_worker_pids": [], "foreign_pids": []}
    deadline = time.monotonic() + 3
    with os.scandir(directory) as entries:
        for count, entry in enumerate(entries, 1):
            require(count <= 32768 and time.monotonic() < deadline
                    and entry.name.isdecimal() and entry.is_dir(),
                    "unrecognized or excessive KFD process entries")
            try:
                executable = os.readlink("/proc/" + entry.name + "/exe")
            except FileNotFoundError:
                # A process that retired during observation is no longer an admission conflict.
                if not os.path.lexists(entry.path):
                    continue
                raise
            field = "stage_worker_pids" if executable == str(stage / "worker-candidate") else "foreign_pids"
            result[field].append(int(entry.name))
    for values in result.values():
        values.sort()
    return result


def admission(stage, initial=False):
    resources = resource_snapshot(stage)
    processes = kfd_snapshot(stage)
    require(not processes["foreign_pids"], "foreign KFD work appeared")
    require(not initial or not processes["stage_worker_pids"], "KFD processes exist before launch")
    return {"resources": resources, "kfd": processes}


def supervise(argv, stage, check, stop_signal=lambda: None, duration=2400, grace=20,
              kill_wait=5, interval=3):
    """Own one unreaped child and pidfd; process discovery never grants signal authority."""
    receipt = {"started": timestamp(), "argv": argv, "term_sent": False,
               "kill_sent": False, "child_reaped": False, "errors": [],
               "termination_reason": "not_started", "duration_seconds": duration,
               "term_grace_seconds": grace, "cleanup_ok": False}
    child = None
    descriptor = None
    child_owned = False
    status = 125

    def observe():
        nonlocal child_owned
        try:
            return os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        except ChildProcessError:
            child_owned = False
            raise

    def send_owned(number, label):
        require(not receipt["child_reaped"], "cannot signal a reaped child")
        try:
            if descriptor is not None:
                signal.pidfd_send_signal(descriptor, number)
            else:
                # Only used if pidfd_open itself failed after Popen. No wait/poll
                # has reaped this direct child; default SIGCHLD reserves its PID.
                require(child_owned, "unreaped child identity unavailable")
                os.kill(child.pid, number)
            receipt[label] = True
        except ProcessLookupError:
            pass

    def await_exit(seconds):
        until = time.monotonic() + seconds
        while True:
            if observe() is not None:
                return True
            if time.monotonic() >= until:
                return False
            time.sleep(min(0.05, max(0, until - time.monotonic())))

    try:
        require_facilities()
        receipt["admission"] = check(True)
        require(stop_signal() is None, "stop requested before child launch")
        with (stage / "launch.stdout").open("xb") as stdout, (stage / "launch.stderr").open("xb") as stderr:
            child = subprocess.Popen(argv, cwd=stage, stdout=stdout, stderr=stderr,
                                     stdin=subprocess.DEVNULL, start_new_session=True)
        child_owned = True
        receipt["child_pid"] = child.pid
        descriptor = os.pidfd_open(child.pid, 0)
        receipt["pidfd_opened"] = True
        save(stage / "launch-supervisor.json", receipt)
        deadline = time.monotonic() + duration
        next_check = time.monotonic()
        while observe() is None:
            number = stop_signal()
            if number is not None:
                receipt["termination_reason"] = "signal"
                receipt["stop_signal"] = number
                break
            now = time.monotonic()
            if now >= deadline:
                receipt["termination_reason"] = "timeout"
                break
            if now >= next_check:
                try:
                    receipt["last_admission"] = check(False)
                except Exception as error:
                    receipt["termination_reason"] = "resource_or_foreign_kfd"
                    receipt["errors"].append(type(error).__name__ + ": " + str(error))
                    (stage / "launch-resource-stop.txt").write_text(timestamp() + "\n")
                    break
                next_check = time.monotonic() + interval
            time.sleep(min(0.1, max(0, deadline - time.monotonic())))
        else:
            receipt["termination_reason"] = "completed"
    except BaseException as error:
        receipt["errors"].append(type(error).__name__ + ": " + str(error))
        receipt["termination_reason"] = "supervisor_error"
    finally:
        if child is not None:
            try:
                if observe() is None:
                    send_owned(signal.SIGTERM, "term_sent")
                    if not await_exit(grace):
                        send_owned(signal.SIGKILL, "kill_sent")
                        require(await_exit(kill_wait), "child did not exit after KILL")
                receipt["returncode"] = child.wait(timeout=1)
                receipt["child_reaped"] = True
                child_owned = False
            except BaseException as error:
                receipt["errors"].append("cleanup: " + type(error).__name__ + ": " + str(error))
            finally:
                if descriptor is not None:
                    os.close(descriptor)
        returncode = receipt.get("returncode")
        if returncode is not None and returncode < 0:
            receipt["errors"].append("signal-exited runner: separate controller cleanup is unverified")
        receipt["cleanup_ok"] = (child is None or (receipt["child_reaped"]
                                 and returncode is not None and returncode >= 0)) and not receipt["kill_sent"]
        if receipt["termination_reason"] == "completed":
            returncode = receipt.get("returncode", 125)
            status = returncode if returncode >= 0 else 128 - returncode
            if not receipt["cleanup_ok"] and status == 0:
                status = 125
        elif receipt["termination_reason"] == "timeout":
            status = 124
        elif receipt["termination_reason"] == "signal":
            status = 128 + receipt["stop_signal"]
        receipt["status"] = status
        receipt["finished"] = timestamp()
        save(stage / "launch-supervisor.json", receipt)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan-sha256", required=True)
    args = parser.parse_args()
    require(socket.gethostname() == HOST and os.getuid() == UID, "wrong native host or UID")
    require(STAGE.resolve(strict=True) == STAGE, "native stage was redirected")
    owner = STAGE.stat()
    require(owner.st_uid == UID and stat.S_IMODE(owner.st_mode) == 0o700, "native stage ownership drift")
    require(len(args.plan_sha256) == 64 and all(c in "0123456789abcdef" for c in args.plan_sha256),
            "invalid plan SHA256")
    plan = json.loads((STAGE / "plan.json").read_text())
    values = plan["common_args"]
    for option, expected in (("--device-unique-id", DEVICE_UNIQUE_ID),
                             ("--worker", str(STAGE / "worker-candidate"))):
        require(values.count(option) == 1 and values[values.index(option) + 1] == expected,
                "native plan identity mismatch: " + option)
    requested = []
    def request_stop(number, _frame):
        if not requested:
            requested.append(number)
    for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(number, request_stop)
    argv = [sys.executable, "-B", "run_v17_native.py", "--plan", str(STAGE / "plan.json"),
            "--plan-sha256", args.plan_sha256, "--output-dir", str(STAGE / "native-results"), "--execute"]
    result = supervise(argv, STAGE, lambda initial: admission(STAGE, initial),
                       lambda: requested[0] if requested else None)
    try:
        result["kfd_after"] = kfd_snapshot(STAGE)
        pids = result["kfd_after"]["stage_worker_pids"] + result["kfd_after"]["foreign_pids"]
        (STAGE / "kfd-processes-after.txt").write_text(
            "".join(str(KFD_PROCESSES / str(pid)) + "\n" for pid in sorted(pids)))
        require(not result["kfd_after"]["stage_worker_pids"], "stage workers survived runner exit")
    except Exception as error:
        result["errors"].append("post-run KFD: " + type(error).__name__ + ": " + str(error))
        result["cleanup_ok"] = False
        result["status"] = 125
    try:
        with (STAGE / "launch-inputs-after.log").open("x") as output:
            subprocess.run(["sha256sum", "--check", "--strict", "launch-inputs.sha256"],
                           cwd=STAGE, stdout=output, stderr=subprocess.STDOUT, check=True, timeout=30)
    except Exception as error:
        result["errors"].append("input recheck: " + type(error).__name__ + ": " + str(error))
        result["status"] = 125
    save(STAGE / "launch-supervisor.json", result)
    (STAGE / "launch.status").write_text(str(result["status"]) + "\n")
    (STAGE / "launch-finished.txt").write_text(timestamp() + "\n")
    print(json.dumps(result, sort_keys=True), flush=True)
    return result["status"]


if __name__ == "__main__":
    sys.exit(main())
