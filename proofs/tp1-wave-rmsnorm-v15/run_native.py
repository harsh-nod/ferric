"""Root-only eight-case V15 check; no model, performance or serving admission."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import socket
import stat
import sys
import time
from types import ModuleType

STAGE = Path("/tmp/ferric-compete-gpu.VabkOGCx")
LOCK = Path("/tmp/ferric-performance-v2-gpu.GLodmvU0/gpu.lock")
STAGE_ID = (9661, 0o700, 66307, 12596054)
LOCK_ID = (9661, 0o600, 66307, 12596034)
PAIRED_SHA = "26eaae58d6b707a1ad4608cfb9a177cf2b36438ae4dc16f06bd257438c6a37b7"
CHECKER_SHA = "2568a3dce9e6031012f548608deb6ce5c1de8914d53612ac38539ab576af5cf9"
FIXTURE_SHA = "35368711baaaac27ce6612c120875a8afb33540b38db0daa7135e05863c02d90"
HELPER_SHA = "a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b"
ROOT = "ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15"
WORKER = STAGE / "worker-ordered-currentness-ae26717"
WORKER_SHA = "526cc6bf8902128767a5318c90d1c0206a9e435957612c398eeb28ff2ae71032"
WORKER_SOURCE = "ae26717922b1fb7ad62fdd5ad70814d83eb01177"
WORKER_TREE = "2ef28f830df9832d53a62525e20aa33815c164fa"
WORKER_RECEIPT = "aaed3d109764d09f2701983e5cba85089358e51fb64aae26c3b7e4335305a238"
COMPILER_SOURCE = "c9c7036cf98034b638886f54a745701e70ac3571"
COMPILER_TREE = "27142fd1003494aac211bc8250584228ff09525c"
IMAGE_SOURCE = "f393390350b15a0a050b1025eda6476d738ac98a"
IMAGE_TREE = "9a22571657d6479d0d6a9898655e4561ab5fc4b4"
TIMEOUT, LOG_LIMIT = 600, 8 * 1024**2
DISK_FLOOR = 64 * 1024**3


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest_text(value, length=64):
    require(type(value) is str and re.fullmatch(f"[0-9a-f]{{{length}}}", value)
            and value != "0" * length, "nonzero exact digest")
    return value


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


def identity(info):
    return info.st_uid, stat.S_IMODE(info.st_mode), info.st_dev, info.st_ino


def require_identity(path, expected, directory=False):
    require(path.resolve(strict=True) == path and identity(path.stat()) == expected
            and (path.is_dir() if directory else path.is_file()), "exact owned path identity")


def validate_bindings(binding):
    require(type(binding) is dict and set(binding) == {
        "schema", "authority", "native_probe_sha256", "source", "tree", "compiler_source",
        "compiler_tree", "emission_receipt_sha256", "worker", "image"}, "closed V15 bindings")
    require(binding["schema"] == "FerricTp1WaveRmsnormV15BindingsV1"
            and binding["authority"] == "none", "V15 binding schema/nonclaim")
    for key in ("native_probe_sha256", "emission_receipt_sha256"):
        digest_text(binding[key])
    for key in ("source", "tree", "compiler_source", "compiler_tree"):
        digest_text(binding[key], 40)
    require(binding["compiler_source"] == COMPILER_SOURCE and binding["compiler_tree"] == COMPILER_TREE,
            "current compiler source/tree")
    require(binding["source"] == IMAGE_SOURCE and binding["tree"] == IMAGE_TREE, "current image source/tree")
    require(binding["worker"] == dict(path=str(WORKER), sha256=WORKER_SHA, byte_len=1591144,
        source=WORKER_SOURCE, tree=WORKER_TREE, host_receipt_sha256=WORKER_RECEIPT)
        and type(binding["worker"].get("byte_len")) is int, "actual current worker provenance")
    image = binding["image"]
    require(type(image) is dict and set(image) == {
        "path", "sha256", "byte_len", "observation_sha256", "handoff_sha256", "root"},
        "one closed image binding")
    for key in ("sha256", "observation_sha256", "handoff_sha256"):
        digest_text(image[key])
    require(type(image["byte_len"]) is int and 0 < image["byte_len"] <= 4 * 1024**2
            and image["root"] == ROOT, "single bounded V15 image")
    require(type(image["path"]) is str, "image path text")
    directory = Path(image["path"])
    require(str(directory) == image["path"] and directory.parent == STAGE / "fe2o3-engineering-v1",
            "held engineering image namespace")
    digest_text(directory.name)
    return directory


def validate_observation(manifest, image, checker):
    require(type(manifest) is dict and manifest.get("schema") == "EngineeringHsacoObservationV1"
            and manifest.get("namespace") == "fe2o3-engineering-v1" and manifest.get("authority") == "none"
            and manifest.get("artifact") == "observation.hsaco"
            and manifest.get("crate_name") == "ferric_qwen3_tp_wave_rmsnorm_kernels_device_v15"
            and manifest.get("target") == "gfx950:xnack-"
            and type(manifest.get("code_object_version")) is int
            and manifest["code_object_version"] == 6, "V15 engineering observation")
    checker.exact(manifest["hsaco"]["identity"], dict(sha256=image["sha256"], byte_len=image["byte_len"]),
                  "image bytes and extent")
    checker.exact(manifest["hsaco"]["kernel_names"], [ROOT], "sole V15 image root")
    require(manifest["compiler_handoff"]["sha256"] == image["handoff_sha256"]
            and manifest["execution"]["exact_output_replay"] is True, "held handoff and compiler replay")
    revisions = [row["rev"] for row in manifest["tools"]["cargo_vendor"]["git_sources"]
                 if row["url"] == "https://github.com/harsh-nod/fe2o3.git"]
    require(revisions == [COMPILER_SOURCE], "current image SDK source")
    require(manifest["options"]["optimization"] == "O2" and manifest["options"]["verify_each"] is True,
            "verified image options")
    checker.exact(manifest["providers"], [], "no image providers")
    checker.exact(manifest["grants"], dict(publication=False, load=False, launch=False), "no engineering grants")


def validate_image(directory, image, base, checker):
    require(directory.resolve(strict=True) == directory and directory.stat().st_uid == os.getuid()
            and {path.name for path in directory.iterdir()} == {"observation.hsaco", "observation.json"},
            "exact held two-file image leaf")
    manifest = base.read_bound(directory / "observation.json", 65536)
    binary = base.read_bound(directory / "observation.hsaco", 4 * 1024**2)
    require(hashlib.sha256(manifest).hexdigest() == image["observation_sha256"]
            and hashlib.sha256(binary).hexdigest() == image["sha256"]
            and len(binary) == image["byte_len"], "held image bytes")
    content = hashlib.sha256(b"FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0")
    for raw in (manifest, binary):
        content.update(len(raw).to_bytes(8, "little"))
        content.update(raw)
    require(content.hexdigest() == directory.name, "engineering content directory identity")
    validate_observation(checker.parse(manifest), image, checker)


def options(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--bindings", type=Path, required=True)
    for name in ("bindings-sha256", "runner-sha256", "tag"):
        parser.add_argument("--" + name, required=True)
    arguments = sys.argv[1:] if argv is None else argv
    flags = [arg for arg in arguments if arg.startswith("--")]
    require(len(flags) == len(set(flags)) and not any("=" in flag for flag in flags),
            "duplicate/noncanonical option")
    args = parser.parse_args(arguments)
    digest_text(args.bindings_sha256)
    digest_text(args.runner_sha256)
    require(re.fullmatch("[a-z0-9][a-z0-9-]{0,63}", args.tag), "fresh bounded tag")
    return args


def command_for(here, fixture_path, helper_path, image, device, out):
    return [sys.executable, "-I", "-B", str(here.parent / "native_probe.py"),
        "--fixtures", str(fixture_path), "--helper", str(helper_path), "--worker", str(WORKER),
        "--worker-sha256", WORKER_SHA, "--artifact", str(Path(image["path"]) / "observation.hsaco"),
        "--artifact-sha256", image["sha256"], "--device-unique-id", str(device),
        "--output", str(out / "probe"), "--run", "--operational"]


def require_normal_exit(status, forced):
    require(type(status) is int and status == 0 and forced is False, "native process or cleanup failed")


def main(argv=None):
    args = options(argv)
    require(__debug__ and socket.gethostname() == "smci350-rck-g03-b19-03"
            and os.getuid() == 9661, "root mi350 only")
    require_identity(STAGE, STAGE_ID, directory=True)
    require_identity(LOCK, LOCK_ID)
    here = Path(__file__).absolute()
    require(here.is_relative_to(STAGE) and here.resolve(strict=True) == here
            and args.bindings == here.parent / "bindings.json", "owned wrapper and exact bindings path")
    os.umask(0o077)
    frozen = STAGE / "paired-paged-wrapper-v1"
    lifecycle = load_pinned(frozen / "run_paired.py", PAIRED_SHA, "rmsnorm_lifecycle")
    require(lifecycle.CHECKER_SHA == CHECKER_SHA, "frozen checker binding")
    checker = load_pinned(frozen / "check_paired.py", CHECKER_SHA, "rmsnorm_checker")
    base = lifecycle.load_pinned(STAGE / "run_draft_paged_canary.py", lifecycle.BASE_SHA, "rmsnorm_base")
    base.owned_path(args.bindings)
    raw = base.read_bound(args.bindings, 16384)
    require(hashlib.sha256(raw).hexdigest() == args.bindings_sha256, "bindings byte pin")
    binding = checker.parse(raw)
    directory = validate_bindings(binding)
    fixture_path, helper_path = here.parent / "probe.py", STAGE / "large-kv-frozen-helper.py"
    probe_path = here.parent / "native_probe.py"
    pins = {STAGE / name: pin for name, pin in base.SUPPORT_PINS.items()}
    pins.update({here: args.runner_sha256, args.bindings: args.bindings_sha256,
        frozen / "run_paired.py": PAIRED_SHA, frozen / "check_paired.py": CHECKER_SHA,
        STAGE / "run_draft_paged_canary.py": lifecycle.BASE_SHA,
        probe_path: binding["native_probe_sha256"], fixture_path: FIXTURE_SHA, helper_path: HELPER_SHA,
        WORKER: WORKER_SHA, STAGE / base.ROSTER_FILE: base.ROSTER_SHA,
        directory / "observation.hsaco": binding["image"]["sha256"],
        directory / "observation.json": binding["image"]["observation_sha256"]})
    lifecycle.verify_pins(pins, base)
    support = base.load_support()
    require(support.inputs.LOCK == LOCK, "exact shared GPU lease")
    with LOCK.open("rb") as lock:
        require(identity(os.fstat(lock.fileno())) == LOCK_ID, "opened lease identity")
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        require_identity(STAGE, STAGE_ID, directory=True)
        require_identity(LOCK, LOCK_ID)
        lifecycle.verify_pins(pins, base)
        require(WORKER.stat().st_size == binding["worker"]["byte_len"], "actual worker extent")
        validate_image(directory, binding["image"], base, checker)
        probe = load_pinned(probe_path, binding["native_probe_sha256"], "rmsnorm_native_probe")
        require(probe.FIXTURE_SHA == FIXTURE_SHA and probe.HELPER_SHA == HELPER_SHA, "frozen fixture/core pins")
        fixtures = probe.load_fixture(fixture_path)
        core = fixtures.load_helper(helper_path)
        devices = checker.parse(base.read_bound(STAGE / base.ROSTER_FILE, 65536))["physical_gpu_ids"]
        require(type(devices) is list and len(devices) == len(set(devices)) == 8, "exact eight devices")
        for device in devices:
            checker.positive(device)
        require(shutil.disk_usage(STAGE).free >= DISK_FLOOR, "64 GiB disk admission")
        out = STAGE / args.tag
        out.mkdir(mode=0o700)
        python_path = Path(sys.executable).resolve(strict=True)
        python_sha = base.digest(python_path)
        command = command_for(here, fixture_path, helper_path, binding["image"], devices[0], out)
        support.inputs.write(out / "prelaunch.json", dict(schema="FerricTp1WaveRmsnormV15PrelaunchV1",
            authority="none", benchmark=False, performance_qualified=False, model_inference=False,
            model_parity_qualified=False, serving_qualified=False, same_compiler_ablation=False,
            argv=command, timeout_seconds=TIMEOUT, maximum_child_file_bytes=LOG_LIMIT,
            bindings=binding, bindings_sha256=args.bindings_sha256,
            python_executable=str(python_path), python_sha256=python_sha,
            pins={str(path): pin for path, pin in pins.items()}))
        before = after = False
        errors = []
        handlers = {sig: signal.signal(sig, support.interrupt) for sig in (signal.SIGINT, signal.SIGTERM)}
        limit = resource.getrlimit(resource.RLIMIT_FSIZE)
        try:
            checker.exact(support.inputs.gpu_snapshot(out / "gpu-before.json"), devices, "pre-run all-eight idle")
            before = True
            resources = support.resource_snapshot(devices)
            resources.update(disk_free_bytes=shutil.disk_usage(STAGE).free,
                minimum_disk_free_bytes=DISK_FLOOR, minimum_host_available_bytes=2 * 1024**3,
                minimum_vram_free_bytes=1024**3)
            support.inputs.write(out / "resources-before.json", resources)
            require(resources["disk_free_bytes"] >= DISK_FLOOR
                    and resources["host_available_bytes"] >= 2 * 1024**3
                    and resources["vram_free_bytes"] >= 1024**3, "finite native resource headroom")
            require(limit[1] == resource.RLIM_INFINITY or limit[1] >= LOG_LIMIT, "child log hard limit")
            resource.setrlimit(resource.RLIMIT_FSIZE, (LOG_LIMIT, limit[1]))
            status, forced = support.execute(command, out, timeout=TIMEOUT,
                                             executable_hashes={WORKER_SHA, python_sha})
            require_normal_exit(status, forced)
            report = checker.parse(base.read_bound(out / "probe/result.json", 2 * 1024**2))
            pid = probe.validate_report(report, fixtures, core, binding["image"]["sha256"],
                                        WORKER_SHA, devices[0], binding["native_probe_sha256"])
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
                    require_identity(STAGE, STAGE_ID, directory=True)
                    require_identity(LOCK, LOCK_ID)
                    validate_image(directory, binding["image"], base, checker)
                    require(base.digest(python_path) == python_sha, "Python executable changed")
                except Exception as error:
                    errors.append(f"postflight: {type(error).__name__}: {error}")
            for sig, handler in handlers.items():
                signal.signal(sig, handler)
        receipt = dict(schema="FerricTp1WaveRmsnormV15WrapperV1", authority="none", benchmark=False,
            performance_qualified=False, model_inference=False, model_parity_qualified=False,
            serving_qualified=False, same_compiler_ablation=False, bindings_sha256=args.bindings_sha256,
            all_eight_idle_before=before, all_eight_idle_after=after,
            errors=errors, passed=before and after and not errors)
        support.inputs.write(out / "wrapper-result.json", receipt)
        print(json.dumps(receipt, sort_keys=True), flush=True)
        return 0 if receipt["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
