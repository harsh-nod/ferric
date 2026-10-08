"""Export original CPU and failed compile/capture evidence, without mutating it."""
import difflib
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import tarfile

E = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220")
ROOT = E / "guarded-mlp-semantic-capture-publication-v228-v1"
PAIR = E / "guarded-mlp-early-stop-compile-v228-v8"
CPUS = {
    "driver": ("guarded-mlp-semantic-capture-driver-cpu-v228-v1",
               "guarded-mlp-inline-hint-driver-cpu-v228-v1",
               "d8d5be4c85fbcac8b963657d1b5054c8fc8e1fdd987b4a9ba1954bc1cff1895f",
               7, {"driver": 25}),
    "backend": ("guarded-mlp-semantic-capture-backend-cpu-v228-v1",
                "guarded-mlp-memory-bounds-dag-cpu-v228-v1",
                "699c8e48b8ce20f1a1f4f52a5f2f9e3e7329bc4ae23857a63398f0bb81c25337",
                8, {"capture": 11, "helper-controls": 14}),
}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def read(path, maximum=16 << 20):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= maximum, "bounded ordinary original")
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(path.lstat()) == stamp(before) and len(raw) == before.st_size, "original changed")
    return raw


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def compact(row):
    return {key: row[key] for key in ("bytes", "sha256")}


def put(path, raw):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as stream:
        stream.write(raw)
    require(read(path) == raw, "export differs")


def clean_phase(phase, code):
    require(phase["exit_code"] == code and phase["natural_exit"] is True
            and phase["reaped"] is True and phase["process_group_absent"] is True
            and phase["timed_out"] is False and phase["forced_cleanup"] is False
            and phase["exception"] is None and phase["storage_failure"] is None,
            "actual clean process retirement")


def main():
    require(os.uname().nodename == "smci350-rck-g03-b19-03"
            and os.getuid() == os.geteuid() == 9661, "MI350 owner")
    os.sched_setaffinity(0, {8, 9})
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    members, originals, cpu_summary = {}, {}, {}

    def add(name, path):
        require(name not in members and len(members) < 256, "unique bounded original roster")
        raw = read(path)
        require(sum(map(len, members.values())) + len(raw) <= 256 << 20, "archive expanded byte cap")
        members[name] = raw
        originals[name] = dict(path=str(path), **pin(raw))

    for role, (directory, parent, digest, phase_count, expected_tests) in CPUS.items():
        cpu_root, base = E / directory, E / parent
        raw = read(cpu_root / "evidence/complete.json")
        require(pin(raw)["sha256"] == digest, "qualified CPU receipt")
        cpu = json.loads(raw)
        require(cpu["passed"] is True and cpu["failure"] is None
                and cpu["postcheck_errors"] == [] and cpu["source_unchanged"] is True
                and len(cpu["phases"]) == phase_count
                and set(cpu["tests"]) == set(expected_tests), "actual focused CPU acceptance")
        for key, count in expected_tests.items():
            require(cpu["tests"][key]["passed"] == count
                    and cpu["tests"][key]["failed"] == cpu["tests"][key]["ignored"] == 0,
                    "exact focused test counts")
        for phase in cpu["phases"]:
            clean_phase(phase, 0)
        require(cpu["input_sources"] == cpu["final_sources"], "unchanged CPU inputs")
        for name, row in cpu["raw"].items():
            require(pin(read(cpu_root / "evidence" / name)) == compact(row), "CPU original identity")
        for name in ("run_cpu.py", "prepare_inputs.py", "supervisor.py", "input-manifest.json"):
            add("cpu/" + role + "/" + name, cpu_root / name)
        for path in sorted((cpu_root / "evidence").iterdir()):
            if path.is_file():
                add("cpu/" + role + "/evidence/" + path.name, path)
        diff = []
        for name, delta in sorted(cpu["source_delta"].items()):
            old, new = read(base / name), read(cpu_root / name)
            require(pin(old) == delta["before"] and pin(new) == delta["after"], "qualified source delta")
            add(role + "/source-before/" + name, base / name)
            add(role + "/source-after/" + name, cpu_root / name)
            put(ROOT / "source" / role / name.removeprefix("fe2o3/"), new)
            diff.extend(difflib.unified_diff(old.decode().splitlines(keepends=True),
                                           new.decode().splitlines(keepends=True),
                                           fromfile="a/" + name.removeprefix("fe2o3/"),
                                           tofile="b/" + name.removeprefix("fe2o3/")))
        put(ROOT / (role + ".patch"), "".join(diff).encode())
        add(role + "/source-base-complete.json", base / "evidence/complete.json")
        for name in ("Cargo.toml", "Cargo.lock"):
            add(role + "/source-after/fe2o3/" + name, cpu_root / "fe2o3" / name)
        for name in ["complete.json"] + [key + "-tests.stdout" if key == "driver" else
                     "capture-tests.stdout" if key == "capture" else "helper-controls.stdout"
                     for key in expected_tests]:
            put(ROOT / "cpu" / role / name, read(cpu_root / "evidence" / name))
        cpu_summary[role] = dict(receipt=pin(raw), selected_tests=sum(expected_tests.values()),
                                phases=phase_count, elapsed_seconds=cpu["elapsed_seconds"])

    add("pair/probe.py", PAIR / "probe.py")
    attempts, scratch = {}, {}
    for arm in ("fixed", "early"):
        for path in sorted((PAIR / arm).rglob("*")):
            if path.is_file():
                add("pair/" + arm + "/" + str(path.relative_to(PAIR / arm)), path)
        run = PAIR / (arm + "-run")
        evidence = run / "evidence"
        raw = read(evidence / "result.json")
        result = json.loads(raw)
        require(result["arm"] == arm and result["compile_accepted"] is False
                and result["postcheck_errors"] == [] and len(result["phases"]) == 1,
                "actual failed terminal compiler result")
        phase = result["phases"][0]
        clean_phase(phase, 1)
        for name in ("command", "stdout", "stderr"):
            require(pin(read(Path(phase[name]["path"]))) == compact(phase[name]), "compiler original identity")
        require(read(evidence / "sources-before.json") == read(evidence / "sources-after.json"),
                "unchanged compile inputs")
        inputs = json.loads(read(evidence / "inputs.json"))
        require(inputs["frontend_cpu_receipt"]["sha256"] == CPUS["driver"][2]
                and inputs["backend_cpu_receipt"]["sha256"] == CPUS["backend"][2]
                and inputs["mir_normalization"] == "optimized-inline-hint-16384-v1"
                and inputs["explicit_inner_cargo_job_limit"] is False, "qualified compiler substitutions")
        require(all(result[k] is False for k in ("gpu_execution", "numerical_acceptance",
                    "performance_claim", "production_authority", "full_model_acceptance",
                    "semantic_identity_join_verified")), "no downstream authority")
        for path in sorted(evidence.iterdir()):
            if path.is_file():
                add("pair/" + arm + "-run/evidence/" + path.name, path)
        names = {"semantic-mir-v1.bin", "semantic-source-map-v1.json"}
        require(set(result["diagnostic_files"]) == names, "both actual diagnostic captures")
        diagnostic = run / "fe2o3-engineering-diagnostics-v1"
        captures = {}
        for name in sorted(names):
            data = read(diagnostic / name)
            require(pin(data) == compact(result["diagnostic_files"][name]), "captured original identity")
            add("pair/" + arm + "-run/diagnostic/" + name, diagnostic / name)
            captures[name] = pin(data)
        mapping = json.loads(read(diagnostic / "semantic-source-map-v1.json"))
        semantic = captures["semantic-mir-v1.bin"]
        require(mapping["schema"] == "fe2o3-diagnostic-semantic-source-map-v1"
                and mapping["stage"] == "pre-ranked" and mapping["execution_authority"] is False
                and mapping["semantic_bytes"] == semantic["bytes"]
                and mapping["semantic_file_sha256"] == semantic["sha256"], "sidecar/raw-byte binding")
        stderr = read(evidence / "compile.stderr").decode()
        for kind, size in (("MIR", semantic["bytes"]),
                           ("source map", captures["semantic-source-map-v1.json"]["bytes"])):
            expected = ("fe2o3 diagnostic semantic " + kind + ": stage=pre-ranked status=complete bytes="
                        + str(size) + " semantic_sha256=" + mapping["semantic_sha256"] + " error_kind=None")
            require(stderr.splitlines().count(expected) == 1, "actual complete capture receipt")
        helper = re.findall(r"helper_identity=([0-9a-f]{64}); adjusted_argument=0;", stderr)
        require(helper == ["00b15eaa5a80fc7f25e19c1c333fcde91f768f51097ae1c2c4f27ced27c66af0"],
                "unchanged actual helper refusal")
        for name in ("result.json", "compile.stderr"):
            put(ROOT / "pair" / (arm + "-" + name), read(evidence / name))
        put(ROOT / "pair" / (arm + "-source-map.json"), read(diagnostic / "semantic-source-map-v1.json"))
        attempts[arm] = dict(compile_accepted=False, exit_code=1, result=pin(raw),
                             elapsed_ns=phase["elapsed_ns"], captures=captures,
                             semantic_sha256=mapping["semantic_sha256"],
                             capture_receipts_complete=True, semantic_model_decoded=False,
                             helper_identity=helper[0], stderr=pin(read(evidence / "compile.stderr")),
                             natural_exit=True, reaped=True, process_group_absent=True)
        scratch[arm] = {name: sum(p.lstat().st_size for p in (run / name).rglob("*") if p.is_file())
                        for name in ("tmp", "fe2o3-engineering-v1")}
        require(all(size == 0 for size in scratch[arm].values()), "no retained output or temporary body")

    manifest = dict(schema="ferric-mlp-semantic-capture-originals-v1",
                    originals=originals, cpus=cpu_summary, attempts=attempts)
    members["archive-manifest.json"] = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()
    tar_path = ROOT / "qualification.tar.gz"
    with tar_path.open("xb") as stream:
        with gzip.GzipFile(filename="", mode="wb", fileobj=stream, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w") as archive:
                for name, raw in sorted(members.items()):
                    info = tarfile.TarInfo(name)
                    info.size, info.mode, info.mtime = len(raw), 0o600, 0
                    archive.addfile(info, io.BytesIO(raw))
    with tarfile.open(tar_path, "r:gz") as archive:
        require(len(archive.getmembers()) == len(members)
                and {m.name for m in archive.getmembers()} == set(members), "archive roster")
        for name, raw in members.items():
            require(archive.extractfile(name).read() == raw, "archive body")
    files = {str(p.relative_to(ROOT)): pin(read(p, 64 << 20 if p == tar_path else 16 << 20))
             for p in sorted(ROOT.rglob("*")) if p.is_file()}
    summary = dict(schema="ferric-mlp-semantic-capture-qualification-v1", files=files,
                   archive_members=len(members), cpus=cpu_summary, attempts=attempts, scratch_bytes=scratch,
                   full_suites=False, semantic_model_decoded=False, gpu_execution=False,
                   numerical_acceptance=False, performance_claim=False, production_authority=False,
                   full_model_acceptance=False)
    put(ROOT / "manifest.json", (json.dumps(summary, indent=2, sort_keys=True) + "\n").encode())
    print(json.dumps(dict(files=len(files) + 1, archive=pin(read(tar_path, 64 << 20)),
                         members=len(members), attempts=attempts, scratch=scratch)), flush=True)


if __name__ == "__main__":
    main()

