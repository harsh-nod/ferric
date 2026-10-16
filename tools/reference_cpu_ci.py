"""Ordinary CPU CI only: frozen wheels are not native/proof qualification.

Upstream triton-rocm RECORD defects are disclosed, never repaired. Integrity is
whole locked archive SHA256 plus exact payload and installer-file accounting.
"""
import argparse
import configparser
import csv
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import sys
import tomllib
from urllib.parse import unquote, urlsplit
from urllib.request import urlopen
import venv
import zipfile


LOCK_SHA = "1307400cd3725b0d3936ce99e722b885fd05b07d0951f08d049c922520f7811c"
MIN_FREE = 40 * 1024**3
WHEEL_BYTES = 6539185035
DISCOVERY = ["python3", "-I", "-B", "-m", "unittest", "discover", "-s",
             "benches/m1/reference", "-p", "test*.py"]
HOSTS = {"files.pythonhosted.org", "download.pytorch.org", "download-r2.pytorch.org"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def safe_member(name):
    path = PurePosixPath(name)
    require(name and not path.is_absolute() and ".." not in path.parts
            and "\\" not in name and "\x00" not in name, "unsafe wheel path")
    return path


def wheel_dist_info(names):
    roots = {p.parts[0] for name in names for p in (safe_member(name),)
             if len(p.parts) == 2 and p.parts[0].endswith(".dist-info")
             and p.parts[1] == "METADATA"}
    require(len(roots) == 1, "wheel metadata roster")
    return roots.pop()


def installed_member(name, prefix):
    member = safe_member(name)
    if member.parts[0].endswith(".data"):
        # The sole non-site payload in the frozen 28-wheel Linux closure,
        # independently byte-audited in the ordinary CPU payload inventory.
        require(name == "sympy-1.14.0.data/data/share/man/man1/isympy.1",
                "unreviewed wheel data scheme/member")
        return prefix / "share/man/man1/isympy.1"
    return prefix / "lib/python3.12/site-packages" / member


def vendor_record_rows(rows, members, prefix):
    site = prefix / "lib/python3.12/site-packages"
    original = {r[0] for r in rows}
    allowed_missing = {"triton/backends/amd/lib/" + n for n in (
        "libelf.so", "libnuma.so", "libtinfo.so")}
    result = {os.path.relpath(installed_member(r[0], prefix), site): r[1:] for r in rows}
    for member in set(members) - original:
        # pip records these three authentic but upstream-unlisted payloads with
        # blank hashes/sizes. Their bytes were checked against the locked wheel;
        # these additions do not fix any of the ten defective upstream rows.
        require(member in allowed_missing, "unreviewed upstream-unlisted payload")
        result[os.path.relpath(installed_member(member, prefix), site)] = ["", ""]
    return result


def wheel_url(url):
    parsed = urlsplit(url)
    require(parsed.scheme == "https" and parsed.hostname in HOSTS
            and not parsed.username and not parsed.password
            and parsed.port in (None, 443) and not parsed.query
            and not parsed.fragment, "untrusted wheel URL")
    name = unquote(parsed.path.rsplit("/", 1)[-1])
    require(safe_member(name).name == name and name.endswith(".whl"), "unsafe wheel URL filename")
    return name


def capacity(paths):
    result = {}
    for path in paths:
        path = Path(path).resolve(strict=True)
        volume = os.statvfs(path)
        free = volume.f_bavail * volume.f_frsize
        result[str(path)] = {"available_bytes": free, "device": path.stat().st_dev}
        require(free >= MIN_FREE, "fresh runner requires at least 40 GiB free before downloads")
    return result


def selected_wheels(source):
    # Use the installer's real target tag and marker implementation, not filename scraping.
    import pip
    require(pip.__version__ == "24.0", "reviewed bootstrap pip24.0 required")
    from pip._vendor.packaging.markers import Marker
    from pip._vendor.packaging.requirements import Requirement
    from pip._vendor.packaging.tags import sys_tags
    from pip._vendor.packaging.utils import canonicalize_name, parse_wheel_filename
    require(sys.version_info[:2] == (3, 12) and sys.platform == "linux"
            and os.uname().machine == "x86_64", "requires Linux x86_64 Python3.12")
    ref = Path(source) / "benches/m1/reference"
    require(digest(ref / "uv.lock") == LOCK_SHA, "frozen lock drift")
    lock = tomllib.loads((ref / "uv.lock").read_text())
    project = tomllib.loads((ref / "pyproject.toml").read_text())["project"]
    require(project["requires-python"] == lock["requires-python"] == "==3.12.*", "Python pin drift")
    packages = {p["name"]: p for p in lock["package"]}
    require(len(packages) == len(lock["package"]) == 30, "lock package roster drift")
    root = packages[project["name"]]
    direct = {canonicalize_name(Requirement(r).name): Requirement(r) for r in project["dependencies"]}
    require(len(direct) == 8 and set(direct) == {d["name"] for d in root["dependencies"]}, "direct pin roster drift")
    for name, req in direct.items():
        require(str(req.specifier) == "==" + packages[name]["version"], "direct version pin drift")
    ranks = {tag: i for i, tag in enumerate(sys_tags())}
    pending = list(root["dependencies"])
    visited = set()
    result = []
    while pending:
        dep = pending.pop()
        if "marker" in dep and not Marker(dep["marker"]).evaluate():
            continue
        name = dep["name"]
        if name in visited:
            continue
        visited.add(name)
        package = packages[name]
        candidates = []
        for wheel in package["wheels"]:
            filename = wheel_url(wheel["url"])
            wheel_name, version, _, tags = parse_wheel_filename(filename)
            require(canonicalize_name(wheel_name) == name and str(version) == package["version"], "wheel identity drift")
            require(re.fullmatch(r"sha256:[0-9a-f]{64}", wheel["hash"]) is not None, "invalid locked SHA256")
            compatible = tags & ranks.keys()
            if compatible:
                candidates.append((min(ranks[t] for t in compatible), filename, wheel))
        require(candidates, "missing compatible locked wheel: " + name)
        _, filename, wheel = min(candidates, key=lambda row: (row[0], row[1]))
        result.append(dict(wheel, filename=filename, name=name, version=package["version"]))
        pending.extend(package.get("dependencies", []))
    require(len(result) == 28, "Linux wheel closure must contain 28 wheels")
    return sorted(result, key=lambda row: row["name"])


def checked_wheel(path, wheel):
    require(Path(path).is_file() and not Path(path).is_symlink(), "missing/linked archive")
    require(digest(path) == wheel["hash"].split(":")[1], "wheel SHA256 mismatch")
    size = Path(path).stat().st_size
    if "size" in wheel:
        require(size == wheel["size"], "wheel byte count mismatch")
    with zipfile.ZipFile(path) as archive:
        names = set()
        for info in archive.infolist():
            safe_member(info.filename)
            require(info.filename not in names, "duplicate wheel member")
            names.add(info.filename)
            require(not stat.S_ISLNK(info.external_attr >> 16), "symlink wheel member")
    return size


def offline_environment(root):
    root = Path(root)
    return dict(PATH="/usr/bin:/bin", LANG="C.UTF-8", HOME=str(root / "home"),
                TMPDIR=str(root / "tmp"), XDG_CACHE_HOME=str(root / "cache"),
                HF_HOME=str(root / "cache/hf"), HF_HUB_OFFLINE="1",
                TRANSFORMERS_OFFLINE="1", HF_DATASETS_OFFLINE="1",
                ROCR_VISIBLE_DEVICES="-1", HIP_VISIBLE_DEVICES="-1",
                CUDA_VISIBLE_DEVICES="-1", HSA_VISIBLE_DEVICES="-1",
                OMP_NUM_THREADS="1", MKL_NUM_THREADS="1")


def run_checked(argv, **kwargs):
    return subprocess.run(argv, check=True, timeout=570, **kwargs)


def prepare(source, root, cache=None):
    source, root = Path(source).resolve(), Path(root).resolve()
    require(not root.exists(), "installation stage must be fresh")
    preflight = capacity([source, root.parent])
    wheels = selected_wheels(source)
    root.mkdir(mode=0o700)
    for name in ("wheels", "home", "tmp", "cache"):
        (root / name).mkdir(mode=0o700)
    save(root / "capacity.json", preflight)
    total = 0
    for wheel in wheels:
        path = root / "wheels" / wheel["filename"]
        if cache is not None:
            cached = Path(cache) / wheel["filename"]
            checked_wheel(cached, wheel)
            os.link(cached, path)  # Reuse bytes, never a previous installation.
        else:
            with urlopen(wheel["url"], timeout=60) as response:
                wheel_url(response.url)
                with path.open("xb") as output:
                    shutil.copyfileobj(response, output, 1024 * 1024)
        total += checked_wheel(path, wheel)
    require(total == WHEEL_BYTES, "actual Linux wheel bytes drift")
    save(root / "wheels.json", {"wheels": wheels, "actual_archive_bytes": total,
                               "lock_sha256": LOCK_SHA, "cache_reused": cache is not None})
    # No pip/setuptools bootstrap packages inside the reference distribution closure.
    prefix = root / "venv"
    venv.EnvBuilder(with_pip=False, system_site_packages=False).create(prefix)
    req = root / "requirements.txt"
    req.write_text("".join(f"{(root / 'wheels' / w['filename']).as_uri()} --hash={w['hash']}\n" for w in wheels))
    run_checked([sys.executable, "-I", "-B", "-m", "pip", "--isolated", "--python",
                 str(prefix / "bin/python3"), "install", "--no-index", "--no-deps",
                 "--require-hashes", "--only-binary=:all:", "--no-compile",
                 "--no-cache-dir", "--disable-pip-version-check", "-r", str(req)],
                env=offline_environment(root))
    audit(source, root)


def audit(source, root):
    source, root = Path(source), Path(root)
    wheels = selected_wheels(source)
    prefix = root / "venv"
    site = prefix / "lib/python3.12/site-packages"
    # Exact distlib-generated entry point bytes, from the actual installer source.
    import pip._vendor.distlib.scripts as scripts
    template = scripts.SCRIPT_TEMPLATE
    expected, generated, records, upstream_rows, generated_owner = {}, {}, [], {}, {}
    payload_members = {}
    for wheel in wheels:
        path = root / "wheels" / wheel["filename"]
        checked_wheel(path, wheel)
        with zipfile.ZipFile(path) as archive:
            names = archive.namelist()
            dist = wheel_dist_info(names)
            upstream_rows[site / dist / "RECORD"] = list(csv.reader(
                archive.read(dist + "/RECORD").decode().splitlines()))
            payload_members[site / dist / "RECORD"] = [i.filename for i in archive.infolist() if not i.is_dir()]
            for info in archive.infolist():
                if info.is_dir():
                    continue
                member = safe_member(info.filename)
                dest = installed_member(info.filename, prefix)
                require(dest not in expected, "wheel payload collision")
                if member.as_posix() == dist + "/RECORD":
                    records.append(dest)
                    expected[dest] = None
                else:
                    with archive.open(info) as stream:
                        expected[dest] = hashlib.file_digest(stream, "sha256").hexdigest()
            for name, content in (("INSTALLER", b"pip\n"), ("REQUESTED", b"")):
                generated[site / dist / name] = content
                generated_owner[site / dist / name] = site / dist / "RECORD"
            direct = site / dist / "direct_url.json"
            obj = json.loads(direct.read_text())
            require(obj == {"url": path.as_uri(), "archive_info": {
                "hash": wheel["hash"].replace(":", "=", 1),
                "hashes": {"sha256": wheel["hash"].split(":")[1]}}}, "installer direct URL drift")
            generated[direct] = direct.read_bytes()
            generated_owner[direct] = site / dist / "RECORD"
            if dist + "/entry_points.txt" in names:
                cfg = configparser.ConfigParser(interpolation=None)
                cfg.optionxform = str
                cfg.read_string(archive.read(dist + "/entry_points.txt").decode())
                for section in ("console_scripts", "gui_scripts"):
                    if not cfg.has_section(section):
                        continue
                    for name, entry in cfg.items(section):
                        require(safe_member(name).name == name, "unsafe script name")
                        match = re.fullmatch(r"([\w.]+):([\w.]+)(?:\s*\[.*\])?", entry.strip())
                        require(match is not None, "unreviewed entry point")
                        module, func = match.groups()
                        body = template % dict(module=module, import_name=func.split(".")[0], func=func)
                        generated[prefix / "bin" / name] = ("#!" + str(prefix / "bin/python3") + "\n" + body).encode()
                        generated_owner[prefix / "bin" / name] = site / dist / "RECORD"
    for path, sha in expected.items():
        require(path.is_file() and not path.is_symlink(), "missing/linked installed payload")
        if sha is not None:
            require(digest(path) == sha, "installed wheel payload drift: " + str(path))
    for path, content in generated.items():
        require(path.is_file() and not path.is_symlink() and path.read_bytes() == content,
                "installer generated file drift: " + str(path))
    # Preserve the original (including defective) vendor rows. Only the real
    # installer's added files may extend them; this does not certify RECORD.
    import base64
    for record in records:
        rows = list(csv.reader(record.read_text().splitlines()))
        require(len(rows) == len({r[0] for r in rows}), "duplicate installed RECORD row")
        actual_rows = {r[0]: r[1:] for r in rows}
        required_rows = vendor_record_rows(upstream_rows[record], payload_members[record], prefix)
        for path, content in generated.items():
            if generated_owner[path] == record:
                encoded = base64.urlsafe_b64encode(hashlib.sha256(content).digest()).decode().rstrip("=")
                required_rows[os.path.relpath(path, site)] = ["sha256=" + encoded, str(len(content))]
        required_rows[os.path.relpath(record, site)] = ["", ""]
        require(actual_rows == required_rows, "unexplained installed RECORD rewrite")
    actual = {p for p in site.rglob("*") if not p.is_dir()}
    require(actual == {p for p in expected if p.is_relative_to(site)}
            | {p for p in generated if p.is_relative_to(site)}, "unexplained/missing site file")
    require({p.name for p in (prefix / "bin").iterdir()} == {
        "python", "python3", "python3.12", "Activate.ps1", "activate", "activate.csh", "activate.fish"
    } | {p.name for p in generated if p.parent == prefix / "bin"}, "unexpected venv executable")
    inventory = {str(p.relative_to(prefix)): digest(p) for p in set(expected) | set(generated)}
    record = dict(payload_files=len(expected) - len(records), rewritten_root_records=len(records),
                  generated_files=len(generated), inventory=inventory,
                  upstream_triton_record_valid=False, known_upstream_record_defects=10,
                  integrity_basis="locked whole archive and exact installed payload/generated bytes",
                  authority="ordinary CPU only; no native/proof qualification")
    old = root / "integrity.json"
    if old.exists():
        require(json.loads(old.read_text()) == record, "installed inventory changed")
    else:
        save(old, record)
    return record


def assert_cpu_isolation():
    require(sys.flags.isolated and sys.flags.no_user_site and sys.flags.ignore_environment,
            "isolated interpreter required")
    devices = [Path("/dev/kfd")] + [Path(f"/dev/dri/renderD{i}") for i in range(128, 192)]
    require(all(not p.exists() for p in devices), "GPU nodes must be inaccessible BEFORE imports")
    for fd in Path("/proc/self/fd").iterdir():
        try:
            mode = fd.stat()
        except FileNotFoundError:
            continue
        require(not (stat.S_ISCHR(mode.st_mode) and os.major(mode.st_rdev) in (226, 510)), "inherited GPU descriptor")
    status = dict(line.split(":", 1) for line in Path("/proc/self/status").read_text().splitlines())
    require(os.geteuid() != 0 and status["NoNewPrivs"].strip() == "1"
            and all(int(status[k], 16) == 0 for k in ("CapInh", "CapPrm", "CapEff", "CapBnd", "CapAmb")), "nonroot NNP/capability isolation required")
    mounts = [line.split() for line in Path("/proc/self/mountinfo").read_text().splitlines()]
    require(any(row[4] == "/dev" and "tmpfs" in row for row in mounts), "private tmpfs devices required")


def readonly(path):
    matches = []
    for line in Path("/proc/self/mountinfo").read_text().splitlines():
        fields = line.split()
        mount = Path(fields[4].replace("\\040", " "))
        if path.is_relative_to(mount):
            matches.append((len(mount.parts), fields[5].split(",")))
    require(matches and "ro" in max(matches)[1], "source/environment must actually be readonly")


def execute(source, root, command):
    require(command == DISCOVERY, "only original full unfiltered reference discovery admitted")
    assert_cpu_isolation()
    admission = json.loads((root / "launch.json").read_text())
    require(os.readlink("/proc/self/ns/net") != admission["host_net_namespace"], "private network required")
    require(sorted(os.sched_getaffinity(0)) == admission["cpu_affinity"]
            and os.getpriority(os.PRIO_PROCESS, 0) == 19, "CPU affinity/nice enforcement failed")
    cgroup = Path("/sys/fs/cgroup") / Path("/proc/self/cgroup").read_text().strip().split("::", 1)[1].lstrip("/")
    limits = {n: (cgroup / n).read_text().strip() for n in ("memory.max", "memory.swap.max", "cpu.max", "pids.max")}
    save(root / "run/admission.json", dict(pid=os.getpid(), birth=Path("/proc/self/stat").read_text(),
         cgroup=str(cgroup), limits=limits, affinity=sorted(os.sched_getaffinity(0)),
         nice=os.getpriority(os.PRIO_PROCESS, 0), private_devices_checked_before_import=True,
         private_network=True, capabilities_zero=True, no_new_privileges=True))
    require(limits == {"memory.max": "8589934592", "memory.swap.max": "0",
                       "cpu.max": "200000 100000", "pids.max": "64"}, "finite CPU resource profile required")
    prefix = root / "venv"
    readonly(source)
    readonly(prefix)
    identity = run_checked([str(prefix / "bin/python3"), "-I", "-B", "-c",
        "import sys,json;print(json.dumps([sys.prefix,sys.base_prefix,sys.flags.isolated,sys.version_info[:2]]))"],
        capture_output=True, text=True, env=offline_environment(root))
    p, base, isolated, version = json.loads(identity.stdout)
    require(p == str(prefix) and p != base and isolated and version == [3, 12], "real pinned isolated venv required")
    result = subprocess.run([str(prefix / "bin/python3"), *command[1:]], cwd=source,
                            env=offline_environment(root), capture_output=True, text=True, timeout=570)
    print(result.stdout, end="")
    print(result.stderr, end="", file=sys.stderr)
    save(root / "run/result.json", dict(returncode=result.returncode, stdout=result.stdout,
         stderr=result.stderr, command=command, qualification="ordinary CPU only"))
    require(result.returncode == 0 and re.search(r"Ran 169 tests in ", result.stderr)
            and re.search(r"\nOK\s*$", result.stderr), "full 169 unfiltered reference tests must pass without skips")


def launch(source, root, command):
    require(command == DISCOVERY, "only original full discovery admitted")
    import pwd
    require(os.geteuid() != 0, "launch as ordinary runner user")
    # A finite unit, not a persistent service; the same isolation is used remotely.
    unit = "ferric-reference-cpu-" + str(os.getpid())
    for name in ("run",):
        (root / name).mkdir(mode=0o700)
    cpus = sorted(os.sched_getaffinity(0))[:2]
    require(len(cpus) == 2, "two CPU lanes required")
    properties = ["User=" + pwd.getpwuid(os.getuid()).pw_name, "MemoryMax=8G", "MemorySwapMax=0",
        "CPUQuota=200%", "TasksMax=64", "RuntimeMaxSec=540", "Nice=19",
        "PrivateDevices=yes", "DevicePolicy=closed", "NoNewPrivileges=yes",
        "CapabilityBoundingSet=", "AmbientCapabilities=", "ProtectSystem=strict",
        "ProtectHome=read-only", "RestrictAddressFamilies=AF_UNIX", "PrivateNetwork=yes",
        "ReadOnlyPaths=" + str(source) + " " + str(root / "venv"),
        "ReadWritePaths=" + " ".join(str(root / n) for n in ("run", "tmp", "cache", "home"))]
    argv = ["sudo", "-n", "systemd-run", "--unit=" + unit, "--wait", "--pipe", "--collect"]
    argv += ["--property=" + p for p in properties]
    argv += ["taskset", "-c", ",".join(map(str, cpus)), "/usr/bin/python3.12", "-I", "-B",
             str(Path(__file__).resolve()), "execute", "--source", str(source), "--root", str(root), "--", *command]
    save(root / "launch.json", {"argv": argv, "unit": unit, "cpu_affinity": cpus,
         "host_net_namespace": os.readlink("/proc/self/ns/net"),
         "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip()})
    run_checked(argv)
    audit(source, root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("preflight", "prepare", "audit", "execute", "run"))
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--wheel-cache", type=Path)
    args, command = parser.parse_known_args()
    source, root = args.source.resolve(), args.root.resolve()
    if args.phase == "preflight":
        require(not command, "unexpected preflight arguments")
        print(json.dumps(capacity([source, root.parent]), sort_keys=True), flush=True)
    elif args.phase == "prepare":
        require(not command, "unexpected prepare arguments")
        prepare(source, root, args.wheel_cache)
    elif args.phase == "audit":
        audit(source, root)
    elif args.phase == "run":
        launch(source, root, command[1:] if command[:1] == ["--"] else command)
    else:
        execute(source, root, command[1:] if command[:1] == ["--"] else command)


if __name__ == "__main__":
    main()
