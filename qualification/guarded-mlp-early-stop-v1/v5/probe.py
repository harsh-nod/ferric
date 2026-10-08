"""Paired compile-feasibility probe; grants no GPU or numerical authority."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import signal
import sys
import time
import tomllib
import types

E = Path("/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220")
ROOT = E / "guarded-mlp-early-stop-compile-v228-v5"
ORIGINAL = E / "row-silu-materialized-checked-probe-v228-v1"
PROVIDER = E / "guarded-mlp-segment-cpu-v228-v5/fe2o3"
LOCK = PROVIDER.parent / "candidate/Cargo.lock"
TOOLS = E / "guarded-mlp-memory-bounds-dag-compiler-tools-v228-v1"
COMMAND = E / "guarded-mlp-memory-bounds-dag-lowering-v228-v1/compile.command.json"
SUPERVISOR = E / "guarded-mlp-currentness-duration-diagnostic-cpu-v228-v2/supervisor.py"
PINNED = {
    SUPERVISOR: "8824478a6415dee39d28ccefb0881fc0687ac724f5a9c5a4540310fb5196b9bc",
    COMMAND: "9488ece7a30056534f29dfae7ef5f2c8c7b5b4e53f0ddb5805ec09330a44b092",
    TOOLS / "manifest.json": "3e9e05f830711ff4c93f2694683a66618e218fb2f792acc16d93c42a274c7949",
    LOCK: "663d3f2a02334440ea3f210a56d3be0e2c4c55f0bea647e8cc01304f2145915e",
}


def require(ok, why):
    if not ok:
        raise RuntimeError(why)


def load_supervisor():
    bodies = {}
    for path, digest in PINNED.items():
        require(path.resolve(strict=True) == path and path.is_file(), "canonical input")
        raw = path.read_bytes()
        require(hashlib.sha256(raw).hexdigest() == digest, "input identity: " + str(path))
        bodies[path] = raw
    module = types.ModuleType("early_stop_bounded_leaf")
    module.__file__ = str(SUPERVISOR)
    exec(compile(bodies[SUPERVISOR], str(SUPERVISOR), "exec"), module.__dict__)
    return module


def fixture_transition():
    old_root = "/home/harmenon/ferric-asrock-42/fe2o3"
    original = ORIGINAL / "fixture"
    require(hashlib.sha256((original / "src/lib.rs").read_bytes()).hexdigest()
            == "794bac2a76ced9ddcf3aa3abdaea6c41850e861a1d253837c321efc9c3ceef34", "original caller identity")
    require(hashlib.sha256((PROVIDER / "crates/fe2o3-device/src/finite_join/wave_mlp_tiles_v2.rs").read_bytes()).hexdigest()
            == "f292dd381b827946b037a39b05548ed2a8819420686f2069b5432845530992b6", "original scheduler provider")
    names = sorted(str(p.relative_to(original)) for p in original.rglob("*") if p.is_file())
    require(len(names) == 7, "seven original fixture files")
    for arm in ("fixed", "early"):
        actual = sorted(str(p.relative_to(ROOT / arm)) for p in (ROOT / arm).rglob("*") if p.is_file())
        require(actual == names, "closed fixture roster")
        for name in names:
            raw = (original / name).read_bytes()
            if name == "Cargo.lock":
                raw = LOCK.read_bytes()
                expected = tomllib.loads(raw.decode())
                packages = [p for p in expected["package"] if p["name"] == "ferric-qwen3-tp-guarded-mlp-segment-kernels-device-v1"]
                require(len(packages) == 1 and packages[0]["dependencies"] == ["fe2o3-device", "fe2o3-host"], "same root dependencies")
                packages[0]["name"] = "fe2o3-production-extraction-fixture"
                require(tomllib.loads((ROOT / arm / name).read_text()) == expected, "retained lock semantics")
                token = b"ferric-qwen3-tp-guarded-mlp-segment-kernels-device-v1"
                require(raw.count(token) == 1, "one root package rename")
                raw = raw.replace(token, b"fe2o3-production-extraction-fixture")
            if name == "Cargo.toml":
                require(raw.count(old_root.encode()) == 2, "two dependency relocations")
                raw = raw.replace(old_root.encode(), str(PROVIDER).encode())
            if name == "src/lib.rs":
                closure = b'        #[cfg_attr(target_arch = "amdgpu", inline(always))]\n        |task| execute_task(task),'
                require(raw.count(closure) == 1, "one noncapturing forwarding closure")
                raw = raw.replace(closure, b"        execute_task,")
                forced = b'#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]\nfn execute_task'
                require(raw.count(forced) == 1, "one named callback attribute")
                raw = raw.replace(forced, b"#[inline(always)]\nfn execute_task")
            if arm == "early" and name == "src/lib.rs":
                token = b"WaveMlpTileWorkerV2::run_fixed_rounds("
                require(raw.count(token) == 1, "one scheduler call")
                raw = raw.replace(token, b"WaveMlpTileWorkerV2::run(")
            require((ROOT / arm / name).read_bytes() == raw, "unexpected fixture edit: " + name)


def main(arm):
    require(__debug__ and sys.dont_write_bytecode and arm in ("fixed", "early"), "closed invocation")
    require(Path(__file__).resolve() == ROOT / "probe.py", "owned script path")
    require(os.getuid() == os.geteuid() == 9661 and os.uname().nodename == "smci350-rck-g03-b19-03", "MI350 owner")
    os.umask(0o077)
    os.sched_setaffinity(0, {8, 9})
    nice = os.getpriority(os.PRIO_PROCESS, 0)
    require(nice in (0, 10), "nice envelope")
    if nice == 0:
        os.nice(10)
    for kind, cap in ((resource.RLIMIT_CORE, 0), (resource.RLIMIT_AS, 12 << 30), (resource.RLIMIT_FSIZE, 1 << 30)):
        limits = resource.getrlimit(kind)
        limit = min([cap] + [v for v in limits if v != resource.RLIM_INFINITY])
        resource.setrlimit(kind, (limit, limit))
    require(shutil.disk_usage(ROOT).free >= 40 << 30, "initial 40 GiB free floor")
    fixture_transition()
    h = load_supervisor()
    out = ROOT / (arm + "-run")
    require(not os.path.lexists(out), "fresh output required")
    out.mkdir(mode=0o700)
    h.ROOT, h.OUT, h.TARGET, h.TMP = ROOT, out / "evidence", out / "fe2o3-engineering-v1", out / "tmp"
    h.OUT.mkdir(); h.TMP.mkdir()
    h.CPU_LIMIT = 600
    h.CLEANUP_RESERVE = 50
    h.LEAF_WALL = 600
    require(ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) == 0, "owned subreaper")
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP, signal.SIGALRM):
        signal.signal(sig, h.interrupted)

    def snapshot():
        paths = [ROOT / "probe.py", COMMAND, TOOLS / "manifest.json", SUPERVISOR, LOCK]
        for directory in (PROVIDER, ROOT / "fixed", ROOT / "early", ORIGINAL / "fixture"):
            paths.extend(h.files_below(directory))
        require(len(paths) < 6000, "source roster bound")
        return {str(p): h.pin(p) for p in sorted(set(paths))}

    h.sources = snapshot
    before = snapshot()
    h.save("sources-before.json", before)
    tool_pins = {}
    for name, expected in json.loads((TOOLS / "manifest.json").read_text()).items():
        actual = h.pin(TOOLS / name)
        require(all(actual[k] == expected[k] for k in ("bytes", "sha256")), "compiler tool identity")
        tool_pins[actual["path"]] = actual
    command = json.loads(COMMAND.read_text())
    argv = command["compiler_argv"][:]
    require(argv[0] == str(TOOLS / "bin/cargo-fe2o3") and argv[1:3] == ["engineering", "hsaco"], "checked route")
    for option, value in (("--crate", "fe2o3_production_source_safety_fixture"),
                          ("--output-root", str(h.TARGET)), ("--manifest-path", str(ROOT / arm / "Cargo.toml"))):
        require(argv.count(option) == 1, "unique compiler option")
        argv[argv.index(option) + 1] = value
    require(argv[-4:] == ["--lib", "--no-default-features", "--features", "gfx950"], "old guarded feature tail")
    argv[-4:] = ["--lib"]
    for option in ("--cargo", "--rustc"):
        actual = h.pin(Path(argv[argv.index(option) + 1]))
        require(actual["sha256"] == argv[argv.index(option + "-sha256") + 1], "Rust tool identity")
        tool_pins[actual["path"]] = actual
    env = dict(command["env"], HOME=str(out), TMPDIR=str(h.TMP))
    for key in ("HIP_VISIBLE_DEVICES", "ROCR_VISIBLE_DEVICES", "CUDA_VISIBLE_DEVICES"):
        env[key] = ""
    require(env["CARGO_BUILD_JOBS"] == "2", "two Cargo jobs")
    h.save("inputs.json", dict(arm=arm, argv=argv, env=env, tool_pins=tool_pins,
                              original_image_sha256="b0d1766fdb0fda17ab09496d1120ba3cee29100692b48fcc5b05cf0db0a8d589",
                              current_generation_not_historical=True, gpu_execution=False))
    phases, error, postcheck_errors = [], None, []
    started = time.monotonic()
    deadline = started + 650
    signal.setitimer(signal.ITIMER_REAL, 600)
    try:
        h.run("compile", argv, env, phases, deadline, before, 600, ROOT / arm)
    except BaseException as failure:
        error = repr(failure)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        try:
            after = snapshot()
            h.save("sources-after.json", after)
            require(after == before, "source mutation")
            require({p: h.pin(Path(p)) for p in tool_pins} == tool_pins, "tool mutation")
            fixture_transition()
        except BaseException as failure:
            postcheck_errors.append(repr(failure))
    accepted = error is None and not postcheck_errors and len(phases) == 1
    result = dict(schema="ferric-mlp-early-stop-compile-feasibility-v1", arm=arm,
                  compile_accepted=accepted, error=error, postcheck_errors=postcheck_errors,
                  phases=phases, elapsed_seconds=time.monotonic() - started,
                  gpu_execution=False, numerical_acceptance=False, performance_claim=False,
                  production_authority=False, full_model_acceptance=False)
    h.save("result.json", result)
    print(json.dumps(result), flush=True)
    return 0 if accepted else 1


if __name__ == "__main__":
    require(len(sys.argv) == 2, "fixed or early")
    sys.exit(main(sys.argv[1]))
