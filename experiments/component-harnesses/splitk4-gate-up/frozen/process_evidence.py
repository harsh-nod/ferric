"""Bounded read-only process identity and request-interval CPU observations."""
import hashlib
import os
from pathlib import Path
import stat
import time

MAX_ELF = 512 * 1024 * 1024
FILE_FIELDS = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
STABLE_PROCESS = ("process_id", "parent_pid", "process_group", "session", "start_time_ticks",
                  "priority", "nice", "cpus_allowed_list")


def require(value, message):
    if not value:
        raise ValueError(message)


def bounded_text(path):
    with path.open("rb") as stream:
        raw = stream.read(16385)
    require(0 < len(raw) <= 16384, "bounded proc observation")
    return raw.decode("ascii")


def parse_stat(raw, status, pid):
    head, separator, tail = raw.rpartition(")")
    fields = tail.split()
    require(separator and head.split(" ", 1)[0] == str(pid) and len(fields) >= 37,
            "same complete proc stat PID")
    affinity = [line.split(":", 1)[1].strip() for line in status.splitlines()
                if line.startswith("Cpus_allowed_list:")]
    require(len(affinity) == 1 and affinity[0] and all(c in "0123456789,-" for c in affinity[0]),
            "one proc affinity observation")
    result = {"process_id": pid, "parent_pid": int(fields[1]), "process_group": int(fields[2]),
              "session": int(fields[3]), "utime_ticks": int(fields[11]), "stime_ticks": int(fields[12]),
              "priority": int(fields[15]), "nice": int(fields[16]), "start_time_ticks": int(fields[19]),
              "last_processor": int(fields[36]), "cpus_allowed_list": affinity[0]}
    require(all(0 <= result[key] < 2**64 for key in ("utime_ticks", "stime_ticks"))
            and result["start_time_ticks"] > 0 and fields[0] not in ("Z", "X", "x"),
            "live process and bounded CPU ticks")
    return result


def process(pid):
    start = time.monotonic_ns()
    directory = Path("/proc") / str(pid)
    raw = bounded_text(directory / "stat")
    status = bounded_text(directory / "status")
    observed = parse_stat(raw, status, pid)
    end_raw = bounded_text(directory / "stat")
    end = parse_stat(end_raw, status, pid)
    require(all(observed[key] == end[key] for key in STABLE_PROCESS), "PID lifetime changed during read")
    require(all(end[key] >= observed[key] for key in ("utime_ticks", "stime_ticks")), "CPU ticks decreased during read")
    return {"process": end, "raw_stat_before": raw, "raw_stat_after": end_raw, "raw_status": status,
            "monotonic_before_ns": start, "monotonic_after_ns": time.monotonic_ns()}


def file_fields(value):
    return {key: getattr(value, key) for key in FILE_FIELDS}


def digest_fd(fd):
    before = os.fstat(fd)
    require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= MAX_ELF, "bounded regular executable")
    digest = hashlib.sha256()
    count = 0
    while True:
        block = os.read(fd, 1024 * 1024)
        if not block:
            break
        count += len(block)
        require(count <= MAX_ELF, "executable read bound")
        digest.update(block)
    require(count == before.st_size and file_fields(before) == file_fields(os.fstat(fd)),
            "executable changed while hashing")
    return {"sha256": digest.hexdigest(), "bytes": count, **file_fields(before)}


def admitted_executables(identities):
    result = {}
    for role in ("controller", "worker"):
        item = identities[role]
        path = Path(item["path"])
        require(path.resolve(strict=True) == path, "canonical admitted executable")
        fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
        try:
            observed = digest_fd(fd)
            require(all(observed[key] == getattr(path.stat(), key) for key in FILE_FIELDS), "staged executable replaced")
        finally:
            os.close(fd)
        require(observed["sha256"] == item["sha256"] and observed["bytes"] == item["bytes"], "admitted ELF hash")
        result[role] = {"path": str(path), **observed}
    return result


def live_executable(pid, expected, placement):
    before = process(pid)
    path = Path("/proc") / str(pid) / "exe"
    require(os.readlink(path) == expected["path"], "live executable pathname")
    # Follow the kernel's exe link, not an arbitrary staged symlink, and retain its open inode.
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC)
    try:
        observed = digest_fd(fd)
        after = process(pid)
        require(all(before["process"][key] == after["process"][key] == placement[key]
                    for key in STABLE_PROCESS), "live PID/ownership/placement changed")
        require(os.readlink(path) == expected["path"], "process exec changed pathname")
        require(all(observed[key] == expected[key] for key in (*FILE_FIELDS, "sha256", "bytes")),
                "live opened executable does not match admitted inode and hash")
    finally:
        os.close(fd)
    return {"pid": pid, "before": before, "after": after, "opened_executable": observed}


def endpoint(placements, admitted, after_request=False):
    result = {"cpu": {}, "executables": {}}
    def cpu():
        for role in ("controller", "worker"):
            value = process(placements[role]["process_id"])
            require(all(value["process"][key] == placements[role][key] for key in STABLE_PROCESS),
                    "CPU endpoint ownership/placement mismatch")
            result["cpu"][role] = value
    def executables():
        for role in ("controller", "worker"):
            result["executables"][role] = live_executable(placements[role]["process_id"], admitted[role], placements[role])
    if after_request:
        cpu()
        executables()
    else:
        executables()
        cpu()
    return result


def cpu_cost(before, after, clock_ticks):
    require(type(clock_ticks) is int and 0 < clock_ticks <= 1000000, "positive SC_CLK_TCK")
    result = {}
    for role in ("controller", "worker"):
        a, b = before["cpu"][role], after["cpu"][role]
        require(all(a["process"][key] == b["process"][key] for key in STABLE_PROCESS), "same CPU process lifetime")
        require(0 < a["monotonic_before_ns"] <= a["monotonic_after_ns"]
                < b["monotonic_before_ns"] <= b["monotonic_after_ns"], "ordered endpoint timestamps")
        user = b["process"]["utime_ticks"] - a["process"]["utime_ticks"]
        system = b["process"]["stime_ticks"] - a["process"]["stime_ticks"]
        require(user >= 0 and system >= 0, "nondecreasing process CPU counters")
        result[role] = {"user_ticks": user, "system_ticks": system, "user_seconds": user / clock_ticks,
                        "system_seconds": system / clock_ticks, "cpu_seconds": (user + system) / clock_ticks,
                        "cpu_seconds_per_token": (user + system) / clock_ticks / 128,
                        "observed_interval_seconds": (b["monotonic_after_ns"] - a["monotonic_after_ns"]) / 1e9}
    return {"clock_ticks_per_second": clock_ticks, "roles": result,
            "scope": "Process CPU over request endpoints; excludes setup/teardown; not GPU or exclusive worker-wall time"}


class StreamParity:
    def __init__(self, controller, reference):
        self.controller = controller
        self.expected = bytes.fromhex(reference["generated_utf8_hex"])
        self.decoded = bytearray()
        self.tokens = 0
        self.complete = False

    def next(self, deadline):
        value = self.controller.next(deadline)
        if value.get("event") == "token":
            raw = value.get("decoded_bytes")
            require(not self.complete and self.tokens < 128 and type(raw) is list
                    and all(type(byte) is int and 0 <= byte <= 255 for byte in raw), "typed streamed bytes")
            self.decoded.extend(raw)
            self.tokens += 1
            require(len(self.decoded) <= 32768, "bounded streamed bytes")
        elif value.get("event") == "request":
            raw = value.get("generated_utf8_bytes")
            require(type(raw) is list and all(type(byte) is int and 0 <= byte <= 255 for byte in raw), "typed final bytes")
            require(not self.complete and self.tokens == 128 and bytes(self.decoded) == bytes(raw) == self.expected,
                    "stream/final/reference byte parity")
            self.complete = True
        return value

    def send(self, value, deadline):
        return self.controller.send(value, deadline)

    def finish(self, deadline):
        require(self.complete, "complete streamed-byte parity")
        return self.controller.finish(deadline)
