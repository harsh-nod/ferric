"""Retain frontend qualification and paired compiler attempts without changing originals."""
import difflib
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile

E = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220")
ROOT = E / "guarded-mlp-inline-hint-publication-v228-v1"
CPU = E / "guarded-mlp-inline-hint-driver-cpu-v228-v1"
BASE = E / "guarded-mlp-driver-cpu-v228-v1"
CPU_SHA = "237f3742f47464641584c65bf63526adf18ab0983d90ea2ee8526e8479c4c212"


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def read(path):
    before = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISREG(before.st_mode)
            and before.st_size <= 16 << 20, "bounded ordinary original")
    raw = path.read_bytes()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    require(stamp(path.lstat()) == stamp(before) and len(raw) == before.st_size, "original changed")
    return raw


def pin(raw):
    return dict(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())


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
    members, originals = {}, {}

    def add(name, path):
        require(name not in members, "unique original name")
        raw = read(path)
        members[name] = raw
        originals[name] = dict(path=str(path), **pin(raw))

    raw = read(CPU / "evidence/complete.json")
    require(pin(raw)["sha256"] == CPU_SHA, "qualified CPU receipt")
    cpu = json.loads(raw)
    require(cpu["passed"] is True and cpu["failure"] is None
            and cpu["postcheck_errors"] == [] and cpu["source_unchanged"] is True
            and cpu["tests"]["driver"]["passed"] == 21
            and cpu["tests"]["driver"]["failed"] == 0
            and cpu["tests"]["driver"]["ignored"] == 0
            and len(cpu["phases"]) == 7, "actual focused CPU acceptance")
    for phase in cpu["phases"]:
        clean_phase(phase, 0)
    require(cpu["input_sources"] == cpu["final_sources"], "unchanged CPU inputs")
    for name, row in cpu["raw"].items():
        require(pin(read(CPU / "evidence" / name)) == {k: row[k] for k in ("bytes", "sha256")},
                "CPU raw evidence identity")
    for name in ("run_cpu.py", "prepare_inputs.py", "supervisor.py", "input-manifest.json"):
        add("cpu/" + name, CPU / name)
    for path in sorted((CPU / "evidence").iterdir()):
        if path.is_file():
            add("cpu/evidence/" + path.name, path)

    diff = []
    for name, delta in sorted(cpu["source_delta"].items()):
        old, new = read(BASE / name), read(CPU / name)
        require(pin(old) == delta["before"] and pin(new) == delta["after"], "qualified source delta")
        add("source-before/" + name, BASE / name)
        add("source-after/" + name, CPU / name)
        put(ROOT / "source" / name.removeprefix("fe2o3/"), new)
        diff.extend(difflib.unified_diff(old.decode().splitlines(keepends=True),
                                       new.decode().splitlines(keepends=True),
                                       fromfile="a/" + name.removeprefix("fe2o3/"),
                                       tofile="b/" + name.removeprefix("fe2o3/")))
    put(ROOT / "frontend.patch", "".join(diff).encode())
    add("source-base-complete.json", BASE / "evidence/complete.json")
    for name in ("Cargo.toml", "Cargo.lock"):
        add("source-after/fe2o3/" + name, CPU / "fe2o3" / name)
    for version in (6, 7):
        pair = E / ("guarded-mlp-early-stop-compile-v228-v" + str(version))
        add("pair/v" + str(version) + "/probe.py", pair / "probe.py")

    attempts, scratch = {}, {}
    for version, arm in ((6, "fixed"), (6, "early"), (7, "fixed"), (7, "early")):
        pair = E / ("guarded-mlp-early-stop-compile-v228-v" + str(version))
        key = "v" + str(version) + "/" + arm
        prefix = "pair/v" + str(version) + "/"
        for path in sorted((pair / arm).rglob("*")):
            if path.is_file():
                add(prefix + arm + "/" + str(path.relative_to(pair / arm)), path)
        run = pair / (arm + "-run")
        evidence = run / "evidence"
        raw = read(evidence / "result.json")
        result = json.loads(raw)
        require(result["arm"] == arm and result["postcheck_errors"] == []
                and len(result["phases"]) == 1, "actual terminal compiler result")
        phase = result["phases"][0]
        code = 0 if result["compile_accepted"] else 1
        clean_phase(phase, code)
        for name in ("stdout", "stderr"):
            require(pin(read(evidence / ("compile." + name)))
                    == {k: phase[name][k] for k in ("bytes", "sha256")}, "compiler stream identity")
        require(read(evidence / "sources-before.json") == read(evidence / "sources-after.json"),
                "unchanged compile inputs")
        inputs = json.loads(read(evidence / "inputs.json"))
        require(inputs["frontend_cpu_receipt"]["sha256"] == CPU_SHA
                and inputs["mir_normalization"] == "optimized-inline-hint-16384-v1"
                and inputs["explicit_inner_cargo_job_limit"] is False, "frontend substitution join")
        require(all(result[k] is False for k in ("gpu_execution", "numerical_acceptance",
                    "performance_claim", "production_authority", "full_model_acceptance")),
                "no downstream authority")
        for path in sorted(evidence.iterdir()):
            if path.is_file():
                add(prefix + arm + "-run/evidence/" + path.name, path)
        for path in sorted((run / "fe2o3-engineering-v1").rglob("*")):
            if path.is_file():
                add(prefix + arm + "-run/output/" + str(path.relative_to(run / "fe2o3-engineering-v1")), path)
        for name in ("result.json", "compile.stderr"):
            put(ROOT / "pair" / ("v" + str(version)) / (arm + "-" + name), read(evidence / name))
        attempts[key] = dict(compile_accepted=result["compile_accepted"], exit_code=code,
                             result=pin(raw), elapsed_ns=phase["elapsed_ns"],
                             stderr=pin(read(evidence / "compile.stderr")),
                             natural_exit=True, reaped=True, process_group_absent=True)
        scratch[key] = {name: sum(p.lstat().st_size for p in (run / name).rglob("*") if p.is_file())
                        for name in ("tmp", "fe2o3-engineering-v1")}
    manifest = dict(schema="ferric-mlp-inline-hint-originals-v1", originals=originals,
                    cpu_receipt=pin(read(CPU / "evidence/complete.json")), attempts=attempts)
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
        require({m.name for m in archive.getmembers()} == set(members), "archive roster")
        for name, raw in members.items():
            require(archive.extractfile(name).read() == raw, "archive body")
    for name in ("complete.json", "driver-tests.stdout"):
        put(ROOT / "cpu" / name, read(CPU / "evidence" / name))
    files = {str(p.relative_to(ROOT)): pin(read(p)) for p in sorted(ROOT.rglob("*")) if p.is_file()}
    summary = dict(schema="ferric-mlp-inline-hint-qualification-v1", files=files,
                   archive_members=len(members), attempts=attempts, scratch_bytes=scratch,
                   focused_cpu_tests=21, full_bin_suite=False,
                   gpu_execution=False, numerical_acceptance=False, performance_claim=False,
                   production_authority=False, full_model_acceptance=False)
    put(ROOT / "manifest.json", (json.dumps(summary, indent=2, sort_keys=True) + "\n").encode())
    print(json.dumps(dict(files=len(files) + 1, archive=pin(read(tar_path)),
                         members=len(members), attempts=attempts, scratch=scratch)), flush=True)


if __name__ == "__main__":
    main()
