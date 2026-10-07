"""Copy exactly two reviewed inputs; no extraction, build, test, or import."""
import fcntl
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tarfile

D = Path("/tmp/ferric-v16-emitter-b95a642-r1")
DEST = D / "inputs/component-ordered-delta-a001"
PINS = {
    "delta-roster.json.gz": (35456, "7c1fe31e327ecc7439580a350ce215b94b05f4ba21454c132d2c2578968b4f82"),
    "export_delta.py": (5684, "5e41520526083b411f9bcd18fa586c96e1e2bf3dcc229d87bfba93768df16c49"),
}


def require(value, message):
    if not value:
        raise ValueError(message)


def allocation():
    result = subprocess.run(["/usr/bin/du", "-sx", "-B1", str(D)],
                            check=True, capture_output=True, timeout=60)
    fields = result.stdout.decode("ascii").split()
    require(not result.stderr and len(fields) == 2 and fields[1] == str(D), "exact allocation")
    return int(fields[0])


def main():
    os.umask(0o077)
    require(os.uname().nodename == "sharkmi300x-3" and os.getuid() == 1046, "fixed host/UID")
    for directory in (D, DEST.parent):
        info = directory.lstat()
        require(directory.resolve(strict=True) == directory and stat.S_ISDIR(info.st_mode)
                and info.st_uid == 1046 and stat.S_IMODE(info.st_mode) == 0o700, "private parent")
    require(not os.path.lexists(DEST), "create-only input namespace")
    lock = os.open(D / "build.lock", os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        info = os.fstat(lock)
        require(stat.S_ISREG(info.st_mode) and info.st_uid == 1046 and info.st_nlink == 1,
                "private existing build lock")
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        before = allocation()
        require(before <= 38117834752 - 32 * 1024**2, "fresh G36 reserve plus 32 MiB headroom")
        for path, minimum in (("/", 23622320128), ("/dev/shm", 34359738368)):
            vfs = os.statvfs(path)
            require(vfs.f_bavail * vfs.f_frsize >= minimum + 32 * 1024**2, "host space floor")
        mem = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
        require(int(mem["MemAvailable"].split()[0]) * 1024 >= 137438953472, "host RAM floor")
        raw = sys.stdin.buffer.read(256 * 1024 + 1)
        require(len(raw) <= 256 * 1024, "bounded transport")
        files = {}
        with tarfile.open(fileobj=io.BytesIO(raw), mode="r:") as archive:
            for member in archive:
                require(member.name in PINS and member.name not in files and member.isfile(),
                        "exact two regular members")
                size, digest = PINS[member.name]
                require(member.size == size, "exact member size")
                payload = archive.extractfile(member).read(size + 1)
                require(len(payload) == size and hashlib.sha256(payload).hexdigest() == digest,
                        "exact member content")
                files[member.name] = payload
        require(set(files) == set(PINS), "complete transport before any write")
        DEST.mkdir(mode=0o700)
        for name, payload in files.items():
            output = name
            path = DEST / output
            with path.open("xb") as stream:
                stream.write(payload)
            info = path.lstat()
            require(path.resolve(strict=True) == path and stat.S_ISREG(info.st_mode)
                    and info.st_uid == 1046 and info.st_nlink == 1
                    and stat.S_IMODE(info.st_mode) == 0o600
                    and hashlib.sha256(path.read_bytes()).hexdigest() == PINS[name][1], "written input")
        after = allocation()
        require(after <= 38117834752 and after - before <= 32 * 1024**2, "transport growth")
        receipt = {"schema": "FerricOrderedComponentDeltaInputTransportV1", "destination": str(DEST),
                   "inputs": PINS, "stage_before_bytes": before, "stage_after_bytes": after,
                   "build_executed": False, "tests_executed": False, "native_executed": False}
        with (DEST / "transport.json").open("x") as stream:
            json.dump(receipt, stream, indent=2, sort_keys=True)
            stream.write("\n")
        print(json.dumps(receipt, sort_keys=True))
    finally:
        os.close(lock)


if __name__ == "__main__":
    main()
