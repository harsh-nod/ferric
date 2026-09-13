"""Root-only finite TP1 two-image parity; launch binding deliberately pending."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import socket
import sys
import time
from types import ModuleType

PAIRED_SHA = "26eaae58d6b707a1ad4608cfb9a177cf2b36438ae4dc16f06bd257438c6a37b7"
CHECKER_SHA = "2568a3dce9e6031012f548608deb6ce5c1de8914d53612ac38539ab576af5cf9"
PROBE_SHA = "e48895c1dccc3db523d3e76e18440a852041ac4d3a59c325be7f59bf60add9af"
# Root must separately review exact native paths and source identity before use.
STAGE = None
WORKER = None
IMAGE_DIRECTORIES = None
PROBE_SOURCE = None


def require(ok, message):
    if not ok:
        raise ValueError(message)


def load_pinned(path, expected_sha, name):
    require(path.resolve(strict=True) == path and path.is_file()
            and path.stat().st_uid == os.getuid(), "owned pinned source path")
    with path.open("rb") as source:
        raw = source.read(65537)
    require(0 < len(raw) <= 65536 and hashlib.sha256(raw).hexdigest() == expected_sha,
            "pinned source snapshot")
    module = ModuleType(name)
    module.__file__ = str(path)
    exec(compile(raw, str(path), "exec", dont_inherit=True), module.__dict__)
    return module


def validate_metadata(metadata, case, image_sha, core, checker):
    require(type(metadata) is dict, "metadata object")
    arguments = metadata.get("explicit_arguments")
    require(type(arguments) is list and len(arguments) == 17, "six slices/five scalars")
    expected_arguments = []
    for index, buffer in enumerate(case["buffers"]):
        offset = index * 16
        pointer = arguments[index * 2]
        require(type(pointer) is dict and core.pointer_matches_source(pointer, offset, buffer),
                "source pointer contract")
        expected_arguments.extend([
            dict(offset=offset, bytes=8, global_buffer=True, access=pointer["access"],
                 pointee_alignment=pointer["pointee_alignment"]),
            dict(offset=offset + 8, bytes=8, global_buffer=False, access=None, pointee_alignment=None),
        ])
    expected_arguments.extend(dict(offset=offset, bytes=4, global_buffer=False,
                                   access=None, pointee_alignment=None) for offset in range(96, 116, 4))
    checker.exact(metadata, dict(symbol=case["symbol"], object_sha256=list(bytes.fromhex(image_sha)),
        wavefront_size=64, private_segment_bytes=0, group_segment_bytes=0, kernarg_alignment=8,
        kernarg_bytes=376, implicit_argument_offset=120, implicit_argument_bytes=256,
        explicit_arguments=expected_arguments), "complete source-bound ABI/resource metadata")


def validate(report, probe, fixtures, core, checker, device):
    require(type(report) is dict, "probe report object")
    rows = report.get("results")
    require(type(rows) is list and len(rows) == 8, "complete eight-case roster")
    pid = checker.positive(report.get("worker_pid"), 2**31 - 1)
    start = checker.positive(report.get("worker_start_ticks"))
    checker.exact(report, dict(schema="FerricTp1AttentionQueryHoistProbeV1", authority="none",
        benchmark=False, performance_qualified=False, model_inference=False, model_parity_qualified=False,
        same_compiler_ablation=False, checks_pass=True, source_sha256=PROBE_SHA,
        fixture_sha256=probe.FIXTURE_SHA, helper_sha256=probe.HELPER_SHA,
        worker_sha256=checker.WORKER_SHA, images=probe.image_provenance(), runtime_operational=True,
        device_unique_id=device, worker_pid=pid, worker_start_ticks=start, clean_teardown=True,
        timing_scope=probe.TIMING_SCOPE, closed_roots=list(probe.ROOTS), active_inputs_finite=True,
        results=rows), "exact report identity and nonclaims")
    for spec, result in zip(probe.specifications(), rows, strict=True):
        case = probe.make_case(core, fixtures, spec)
        probe.validate_case(fixtures, case)
        image_sha = probe.IMAGES[spec[2]]["sha256"]
        require(type(result) is dict and set(result) ==
                {"name", "symbol", "role", "artifact_sha256", "elapsed_ns", "metadata", "checks"},
                "closed per-case report")
        checker.exact({key: result[key] for key in ("name", "symbol", "role", "artifact_sha256")},
                      dict(name=case["name"], symbol=case["symbol"], role=spec[2], artifact_sha256=image_sha),
                      "ordered root/image identity")
        checker.positive(result["elapsed_ns"])
        checks = [dict(name=buffer["name"], access=buffer["access"], bytes=len(buffer["expected"]),
                       sha256=hashlib.sha256(buffer["expected"]).hexdigest(),
                       guard_bytes_each_side=64, guards_unchanged=True) for buffer in case["buffers"]]
        checker.exact(result["checks"], checks, "all six complete inputs/output/tail/guards")
        validate_metadata(result["metadata"], case, image_sha, core, checker)
    return pid


def validate_observation(manifest, image, checker):
    require(manifest["schema"] == "EngineeringHsacoObservationV1" and manifest["authority"] == "none"
            and manifest["hsaco"]["identity"]["sha256"] == image["sha256"]
            and manifest["compiler_handoff"]["sha256"] == image["handoff_sha256"]
            and image["root"] in manifest["hsaco"]["kernel_names"]
            and manifest["execution"]["exact_output_replay"] is True, "pinned engineering image record")
    checker.exact(manifest["grants"], dict(publication=False, load=False, launch=False), "no engineering grants")


def require_normal_exit(status, forced):
    require(type(status) is int and status == 0 and forced is False, "native process or cleanup failed")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--runner-sha256", required=True)
    args = parser.parse_args(argv)
    require(STAGE is not None and WORKER is not None and IMAGE_DIRECTORIES is not None
            and PROBE_SOURCE is not None, "native launch binding pending root review")
    require(__debug__ and socket.gethostname() == "smci350-rck-g03-b19-03", "root mi350 only")
    require(re.fullmatch("[a-z0-9][a-z0-9-]{0,63}", args.tag), "fresh bounded tag")
    require(re.fullmatch("[0-9a-f]{40}", PROBE_SOURCE), "reviewed fixture source identity")
    here = Path(__file__).resolve()
    require(here.is_relative_to(STAGE), "owned wrapper path")
    frozen = STAGE / "paired-paged-wrapper-v1"
    lifecycle = load_pinned(frozen / "run_paired.py", PAIRED_SHA, "hoist_lifecycle")
    require(lifecycle.CHECKER_SHA == CHECKER_SHA, "frozen checker binding")
    checker = load_pinned(frozen / "check_paired.py", CHECKER_SHA, "hoist_checker")
    base = lifecycle.load_pinned(STAGE / "run_draft_paged_canary.py", lifecycle.BASE_SHA, "hoist_base")
    checker.sha(args.runner_sha256)
    require(STAGE.resolve(strict=True) == STAGE and STAGE.stat().st_uid == os.getuid()
            and STAGE.stat().st_mode & 0o777 == 0o700, "private owned stage")
    os.umask(0o077)
    probe = load_pinned(here.parent / "probe.py", PROBE_SHA, "hoist_probe")
    require(set(IMAGE_DIRECTORIES) == set(probe.IMAGES), "exact image paths")
    fixture_path = here.parent / "frozen-fixtures.py"
    helper_path = STAGE / "large-kv-frozen-helper.py"
    pins = {STAGE / name: pin for name, pin in base.SUPPORT_PINS.items()}
    pins.update({here: args.runner_sha256, frozen / "run_paired.py": PAIRED_SHA,
        frozen / "check_paired.py": CHECKER_SHA, STAGE / "run_draft_paged_canary.py": lifecycle.BASE_SHA,
        here.parent / "probe.py": PROBE_SHA, fixture_path: probe.FIXTURE_SHA,
        helper_path: probe.HELPER_SHA, WORKER: checker.WORKER_SHA,
        STAGE / base.ROSTER_FILE: base.ROSTER_SHA})
    for role, directory in IMAGE_DIRECTORIES.items():
        pins[directory / "observation.hsaco"] = probe.IMAGES[role]["sha256"]
        pins[directory / "observation.json"] = probe.IMAGES[role]["observation_sha256"]
    lifecycle.verify_pins(pins, base)
    support = base.load_support()
    with support.inputs.LOCK.open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        lifecycle.verify_pins(pins, base)
        fixtures = probe.load_fixture(fixture_path)
        core = fixtures.load_helper(helper_path)
        devices = checker.parse(base.read_bound(STAGE / base.ROSTER_FILE, 65536))["physical_gpu_ids"]
        require(type(devices) is list and len(devices) == len(set(devices)) == 8, "exact eight devices")
        for device in devices:
            checker.positive(device)
        for role, directory in IMAGE_DIRECTORIES.items():
            validate_observation(checker.parse(base.read_bound(directory / "observation.json", 65536)),
                                 probe.IMAGES[role], checker)
        out = STAGE / args.tag
        out.mkdir(mode=0o700)
        python_path = Path(sys.executable).resolve(strict=True)
        python_sha = base.digest(python_path)
        command = [sys.executable, "-I", "-B", str(here.parent / "probe.py"), "--run", "--operational",
            "--fixtures", str(fixture_path), "--helper", str(helper_path), "--worker", str(WORKER),
            "--worker-sha256", checker.WORKER_SHA, "--resident", str(IMAGE_DIRECTORIES["resident"] / "observation.hsaco"),
            "--candidate", str(IMAGE_DIRECTORIES["candidate"] / "observation.hsaco"),
            "--device-unique-id", str(devices[0]), "--output", str(out / "probe")]
        support.inputs.write(out / "prelaunch.json", dict(schema="FerricTp1AttentionQueryHoistPrelaunchV1",
            authority="none", performance_qualified=False, same_compiler_ablation=False,
            argv=command, timeout_seconds=600, probe_source=PROBE_SOURCE, images=probe.image_provenance(),
            worker_source=checker.WORKER_SOURCE, python_executable=str(python_path), python_sha256=python_sha,
            pins={str(path): pin for path, pin in pins.items()}))
        before = after = False
        errors = []
        handlers = {sig: signal.signal(sig, support.interrupt) for sig in (signal.SIGINT, signal.SIGTERM)}
        limit = resource.getrlimit(resource.RLIMIT_FSIZE)
        try:
            checker.exact(support.inputs.gpu_snapshot(out / "gpu-before.json"), devices, "pre-run all-eight idle")
            before = True
            resources = support.resource_snapshot(devices)
            support.inputs.write(out / "resources-before.json", resources)
            require(resources["host_available_bytes"] >= 2 * 1024**3
                    and resources["vram_free_bytes"] >= 1024**3, "native memory headroom")
            resource.setrlimit(resource.RLIMIT_FSIZE, (8 * 1024**2, limit[1]))
            status, forced = support.execute(command, out, timeout=600,
                                             executable_hashes={checker.WORKER_SHA, python_sha})
            require_normal_exit(status, forced)
            result = checker.parse(base.read_bound(out / "probe/result.json", 2 * 1024**2))
            pid = validate(result, probe, fixtures, core, checker, devices[0])
            require(not Path(f"/proc/{pid}").exists(), "native worker still present")
        except (Exception, KeyboardInterrupt) as error:
            errors.append(f"{type(error).__name__}: {error}")
        finally:
            with support.cleanup_signals():
                resource.setrlimit(resource.RLIMIT_FSIZE, limit)
                try:
                    time.sleep(2)
                    checker.exact(support.inputs.gpu_snapshot(out / "gpu-after.json"), devices, "post-run all-eight idle")
                    after = True
                    lifecycle.verify_pins(pins, base)
                    require(base.digest(python_path) == python_sha, "Python executable changed")
                except Exception as error:
                    errors.append(f"postflight: {type(error).__name__}: {error}")
            for sig, handler in handlers.items():
                signal.signal(sig, handler)
        receipt = dict(schema="FerricTp1AttentionQueryHoistWrapperV1", authority="none", performance_qualified=False,
                       same_compiler_ablation=False, all_eight_idle_before=before, all_eight_idle_after=after,
                       errors=errors, passed=before and after and not errors)
        support.inputs.write(out / "wrapper-result.json", receipt)
        print(json.dumps(receipt, sort_keys=True), flush=True)
        return 0 if receipt["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
