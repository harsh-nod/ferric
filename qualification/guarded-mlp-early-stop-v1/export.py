"""Retain original failed compilation evidence; no build or GPU execution."""
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile

E = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220")
ROOT = E / "guarded-mlp-early-stop-publication-v228-v1"
P = E / "guarded-mlp-segment-cpu-v228-v5/fe2o3/crates/fe2o3-device/src/finite_join"


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


def main():
    require(os.uname().nodename == "smci350-rck-g03-b19-03" and os.getuid() == 9661, "MI350 owner")
    os.sched_setaffinity(0, {8, 9})
    if os.getpriority(os.PRIO_PROCESS, 0) == 0:
        os.nice(10)
    members, originals, results, scratch = {}, {}, {}, {}
    for version in range(1, 6):
        root = E / ("guarded-mlp-early-stop-compile-v228-v" + str(version))
        selected = [root / "probe.py"]
        if version == 3:
            selected.append(root / "probe-before-lock-quote.py")
        for arm in ("fixed", "early"):
            selected += sorted(p for p in (root / arm).rglob("*") if p.is_file())
            run = root / (arm + "-run")
            if version <= 2 and arm == "early":
                require(not run.exists(), "original early arm was not run")
                results["v" + str(version) + "/" + arm] = dict(status="not_run")
                continue
            evidence = run / "evidence"
            result = json.loads(read(evidence / "result.json"))
            require(result["arm"] == arm and result["compile_accepted"] is False
                    and result["postcheck_errors"] == [] and len(result["phases"]) == 1, "actual unsuccessful terminal")
            phase = result["phases"][0]
            require(phase["exit_code"] == 1 and phase["natural_exit"] is True
                    and phase["reaped"] is True and phase["process_group_absent"] is True
                    and phase["timed_out"] is False and phase["forced_cleanup"] is False
                    and phase["exception"] is None and phase["storage_failure"] is None, "clean owned retirement")
            for stream in ("stdout", "stderr"):
                require(pin(read(evidence / ("compile." + stream))) == {
                    k: phase[stream][k] for k in ("bytes", "sha256")}, "original stream pin")
            before = json.loads(read(evidence / "sources-before.json"))
            require(before == json.loads(read(evidence / "sources-after.json")), "source equality")
            require(all(result[k] is False for k in ("gpu_execution", "numerical_acceptance",
                        "performance_claim", "production_authority", "full_model_acceptance")), "no downstream authority")
            selected += sorted(p for p in evidence.iterdir() if p.is_file())
            results["v" + str(version) + "/" + arm] = dict(
                status="compile_failed", result=pin(read(evidence / "result.json")),
                elapsed_ns=phase["elapsed_ns"], exit_code=1, natural_exit=True,
                reaped=True, process_group_absent=True, source_unchanged=True,
                stderr=pin(read(evidence / "compile.stderr")))
            scratch[str(run)] = {
                name: sum(p.lstat().st_size for p in (run / name).rglob("*") if p.is_file())
                for name in ("tmp", "fe2o3-engineering-v1")
            }
            require(not list(run.rglob("*.hsaco")), "no unreported HSACO")
        for path in selected:
            name = "v" + str(version) + "/" + str(path.relative_to(root))
            raw = read(path)
            require(name not in members, "unique archive member")
            members[name] = raw
            originals[name] = dict(path=str(path), **pin(raw))
    require(sum(r["status"] == "compile_failed" for r in results.values()) == 8, "eight actual invocations")
    manifest_body = dict(schema="ferric-mlp-early-stop-attempts-v1", originals=originals, attempts=results)
    members["archive-manifest.json"] = (json.dumps(manifest_body, sort_keys=True, indent=2) + "\n").encode()
    tar_path = ROOT / "attempts.tar.gz"
    with tar_path.open("xb") as raw_stream:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw_stream, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w") as archive:
                for name, raw in sorted(members.items()):
                    info = tarfile.TarInfo(name)
                    info.size, info.mode, info.mtime = len(raw), 0o600, 0
                    archive.addfile(info, io.BytesIO(raw))
    with tarfile.open(tar_path, "r:gz") as archive:
        require({m.name for m in archive.getmembers()} == set(members), "archive roster")
        for name, raw in members.items():
            require(archive.extractfile(name).read() == raw, "archive readback")
    source = E / "guarded-mlp-early-stop-compile-v228-v5"
    for rel in ("probe.py",):
        put(ROOT / "v5" / rel, read(source / rel))
    for arm in ("fixed", "early"):
        for path in sorted((source / arm).rglob("*")):
            if path.is_file():
                put(ROOT / "v5" / arm / path.relative_to(source / arm), read(path))
        for name in ("result.json", "compile.stderr"):
            put(ROOT / "v5" / (arm + "-" + name), read(source / (arm + "-run") / "evidence" / name))
    for name, digest in (
            ("wave_mlp_tiles_v2.rs", "f292dd381b827946b037a39b05548ed2a8819420686f2069b5432845530992b6"),
            ("wave_mlp_tiles_v2_tests.rs", "d5c527ff2134aaf32dcf2832842e99c615e7e04d64779061825246db948fe466")):
        raw = read(P / name)
        require(pin(raw)["sha256"] == digest, "original provider identity")
        put(ROOT / "provider" / name, raw)
    files = {str(p.relative_to(ROOT)): pin(read(p)) for p in sorted(ROOT.rglob("*")) if p.is_file()}
    manifest = dict(schema="ferric-mlp-early-stop-investigation-v1",
                    files=files, archive_members=len(members), attempts=results,
                    scratch_bytes=scratch, gpu_execution=False, numerical_acceptance=False,
                    performance_claim=False, production_authority=False, full_model_acceptance=False)
    put(ROOT / "manifest.json", (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode())
    print(json.dumps(dict(files=len(files) + 1, archive=pin(read(tar_path)),
                         members=len(members), compile_invocations=8, scratch=scratch)), flush=True)


if __name__ == "__main__":
    main()
