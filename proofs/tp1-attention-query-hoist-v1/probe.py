#!/usr/bin/env python3
"""Eight finite TP1 resident-Wave/V14 byte checks; no timing comparison."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
from types import ModuleType

FIXTURE_SHA = "6a7c7baf533a9fa7e0773126519992bb62dfdaefa3ed5f239729319fd6a7f789"
HELPER_SHA = "a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b"
IMAGES = {
    "resident": {
        "root": "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5",
        "sha256": "98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502",
        "observation_sha256": "c559d0533907323aff7dda03215adab6326423c3f151b296e22394c9baf37d00",
        "handoff_sha256": "94807eb1b5f78ce1b9e217b464eb5a98d6a270c6aeb932d8847b5ff4315c7756",
        "source": "eff229bdd8339a47e84d392d3e34b83beea87383",
        "compiler": "3e74a9324a5acd7107e96a4a9b5319d3dd5ecde8",
    },
    "candidate": {
        "root": "ferric_qwen3_tp_batch32_wave_paged_gqa_query_hoist_bf16_v14",
        "sha256": "8f21681fe9103b670ee5666f429a45682e77fc90eb16802a02c4b6fb93a192c8",
        "observation_sha256": "eb058fceb9519c4e9f9fb9d347263dcb80e957c1accd87122c4e139a9e571752",
        "handoff_sha256": "dacfab37c50f78828c329322d34f14dea67a383baf8402b3f7d1bbb843947798",
        "source": "1843d8f174b20ebd6dbc0e72d6fe9044e8d1f244",
        "compiler": "8efd4fd416d1ffae7a718144e4d299fe3c8f7590",
    },
}
ROOTS = tuple(image["root"] for image in IMAGES.values())
TIMING_SCOPE = "worker dispatch latency retained but excluded from all comparisons"


def require(ok, message):
    if not ok:
        raise ValueError(message)


def load_fixture(path):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= 128 * 1024,
                "fixture source extent")
        with os.fdopen(os.dup(fd), "rb") as source:
            raw = source.read(128 * 1024 + 1)
        after = os.fstat(fd)
        identity = lambda value: (value.st_dev, value.st_ino, value.st_size,
                                  value.st_mtime_ns, value.st_ctime_ns)
        require(identity(before) == identity(after) and len(raw) == before.st_size
                and hashlib.sha256(raw).hexdigest() == FIXTURE_SHA, "fixture source pin")
    finally:
        os.close(fd)
    module = ModuleType("ferric_pinned_tp1_attention_fixtures")
    module.__file__ = str(path)
    exec(compile(raw, str(path), "exec", dont_inherit=True), module.__dict__)
    require(module.HELPER_SHA256 == HELPER_SHA, "fixture/core binding")
    return module


def specifications():
    for family, rows in (("uniform", 1), ("uniform", 17), ("uniform", 31), ("selector", 32)):
        for role in IMAGES:
            yield family, rows, role


def make_case(core, fixtures, spec):
    require(type(spec) is tuple and spec in tuple(specifications()) and type(spec[1]) is int,
            "closed two-image specification")
    family, rows, role = spec
    case = fixtures.make_case(core, (family, rows, "wave"))
    fixtures.validate_case(case)
    case["name"] = f"{role}_{family}_rows{rows}_tp1_causal_pages"
    case["symbol"] = IMAGES[role]["root"]
    return case


def validate_case(fixtures, case):
    require(type(case) is dict, "case object")
    matches = [spec for spec in specifications()
               if case.get("name") == f"{spec[2]}_{spec[0]}_rows{spec[1]}_tp1_causal_pages"]
    require(len(matches) == 1, "closed case name")
    family, rows, role = matches[0]
    require(case.get("symbol") == IMAGES[role]["root"], "case root/image role")
    original = dict(case, name=f"wave_{family}_rows{rows}_tp1_causal_pages",
                    symbol=fixtures.ROOTS[1])
    fixtures.validate_case(original)


def image_provenance():
    return [dict(role=role, **record) for role, record in IMAGES.items()]


def self_test(core, fixtures):
    specs = tuple(specifications())
    require(len(specs) == len(set(specs)) == 8, "exact eight-case roster")
    for spec in specs:
        validate_case(fixtures, make_case(core, fixtures, spec))
    print("PASS: eight finite TP1 two-image cases; no GPU")


def run_cases(worker, core, fixtures, artifacts):
    try:
        require(set(artifacts) == set(IMAGES), "exact two held images")
        _, payload = worker.command({"op": "configure_performance", "cache_kernel_admission": False,
                                     "operational_currentness": True, "profile": False},
                                    expected="performance_configured")
        require(not payload, "configuration payload")
        results = []
        for spec in specifications():
            role = spec[2]
            case = make_case(core, fixtures, spec)
            validate_case(fixtures, case)
            result = core.probe(worker, case, artifacts[role], IMAGES[role]["sha256"])
            results.append(dict(result, role=role, artifact_sha256=IMAGES[role]["sha256"]))
            print(case["name"] + ": PASS", flush=True)
        worker.finish()
        return results
    except BaseException:
        worker.abort()
        raise


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--operational", action="store_true")
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--helper", type=Path, required=True)
    parser.add_argument("--worker", type=Path)
    parser.add_argument("--worker-sha256")
    parser.add_argument("--resident", type=Path)
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--device-unique-id", type=int)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    require(args.self_test != args.run, "choose self-test or explicit native run")
    fixtures = load_fixture(args.fixtures)
    core = fixtures.load_helper(args.helper)
    if args.self_test:
        self_test(core, fixtures)
        return
    require(args.operational and all(value is not None for value in
            (args.worker, args.worker_sha256, args.resident, args.candidate,
             args.device_unique_id, args.output)), "explicit operational native inputs required")
    require(0 < args.device_unique_id < 1 << 64 and args.output.is_absolute(), "run bounds")
    available = next(int(line.split()[1]) * 1024 for line in Path("/proc/meminfo").read_text().splitlines()
                     if line.startswith("MemAvailable:"))
    require(available >= 1024**3, "host headroom")
    self_test(core, fixtures)
    args.output.mkdir(mode=0o700)
    source_hash = core.digest(Path(__file__).read_bytes())
    worker_fd, worker_stat, _ = core.held_file(args.worker, args.worker_sha256, 512 * 1024**2, 62)
    held = []
    try:
        artifacts = {}
        for role in IMAGES:
            fd, info, artifact = core.held_file(getattr(args, role), IMAGES[role]["sha256"], 64 * 1024**2, 224)
            held.append((fd, info))
            artifacts[role] = artifact
        worker = core.Worker(worker_fd, worker_stat, args.device_unique_id, args.output)
        results = run_cases(worker, core, fixtures, artifacts)
        require(core.identity(os.fstat(worker_fd)) == core.identity(worker_stat)
                and all(core.identity(os.fstat(fd)) == core.identity(info) for fd, info in held)
                and core.digest(Path(__file__).read_bytes()) == source_hash
                and core.digest(args.fixtures.read_bytes()) == FIXTURE_SHA
                and core.digest(args.helper.read_bytes()) == HELPER_SHA, "retained identity drift")
        report = dict(schema="FerricTp1AttentionQueryHoistProbeV1", authority="none", benchmark=False,
                      performance_qualified=False, model_inference=False, model_parity_qualified=False,
                      same_compiler_ablation=False, checks_pass=True, source_sha256=source_hash,
                      fixture_sha256=FIXTURE_SHA, helper_sha256=HELPER_SHA,
                      worker_sha256=args.worker_sha256, images=image_provenance(), runtime_operational=True,
                      device_unique_id=args.device_unique_id, worker_pid=worker.process.pid,
                      worker_start_ticks=worker.start, clean_teardown=True, timing_scope=TIMING_SCOPE,
                      closed_roots=list(ROOTS), active_inputs_finite=True, results=results)
        with (args.output / "result.json").open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2, allow_nan=False)
            output.write("\n")
    finally:
        for fd, _ in held:
            os.close(fd)
        os.close(worker_fd)


if __name__ == "__main__":
    main()
