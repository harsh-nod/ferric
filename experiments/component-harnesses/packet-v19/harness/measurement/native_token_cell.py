#!/usr/bin/env python3
"""One ordered64 packet-tick cell inside the existing bounded GPU supervisor.

This module intentionally has no standalone GPU launch entry. The integration
owner supplies pinned legacy helpers and the unchanged outer resource guard.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import time


_SPEC = importlib.util.spec_from_file_location("v14_ledger", Path(__file__).with_name("abba_ledger.py"))
ledger = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(ledger)
require = ledger.require
_LIFECYCLE = importlib.util.spec_from_file_location("v14_lifecycle", Path(__file__).with_name("native_lifecycle.py"))
lifecycle = importlib.util.module_from_spec(_LIFECYCLE)
_LIFECYCLE.loader.exec_module(lifecycle)
_TICKS = importlib.util.spec_from_file_location("ordered64_packet_ticks", Path(__file__).with_name("packet_ticks.py"))
ticks = importlib.util.module_from_spec(_TICKS)
_TICKS.loader.exec_module(ticks)

BACKENDS = {"A": {"profile": ticks.PROFILE, "groups": 11}}
COMMON = {"wave_target_mode": "combined", "layer_projection": "c1-wave", "rmsnorm_mode": "wave-v15",
          "submission": "ordered", "benchmark_admitted": False, "serving_admitted": False,
          "requested_prefill_kv_mode": "parallel-prefill16-v27", "prefill_kv_mode": "parallel-prefill16-v27",
          "requested_split_attention_mode": "split8-v21", "split_attention_mode": "split8-v21",
          "requested_c1_packet_mode": "packed64-v29", "c1_packet_mode": "packed64-v29",
          "ordered_wire_mode": "ordered64", "packed_c1_group_bound": 64, "non_c1_group_bound": 16,
          "requested_kv_copy_mode": "parallel-c1-v19", "kv_copy_mode": "parallel-c1-v19",
          "kv_append_mode": "parallel-c1-v19"}
FORBIDDEN = ("token_program", "host_breakdown", "host_preparation", "runtime_profile", "runtime_profile_snapshot", "dispatch_timestamps",
             "host_timing_schema", "host_diagnostic_scope", "runtime_counter_schema", "runtime_counter_scope",
             "diagnostic_max_model_batches", "ordered64_runtime_counters", "ordered64_packet_ticks",
             "active_poll", "wait_policy", "ordered64_wait_policy", "prefill32_pages_mode",
             "requested_prefill32_pages_mode", "prefill_scheduling", "prefill16_ordered_mode",
             "requested_prefill16_ordered_mode", "packed_gate_up_mode", "packed_down_mode", "prefill_program")


def exact(actual, wanted, message):
    require(type(actual) is dict and all(type(actual.get(k)) is type(v) and actual[k] == v for k, v in wanted.items()), message)


def shape(spec):
    require(spec.get("schema") == "FerricPacketTicksCellPlanV1" and spec.get("arm") in BACKENDS
            and spec.get("mode") == "counters", "exact native token cell required")
    require(spec.get("timeouts") == {"setup_seconds": 600, "request_seconds": 180, "cell_seconds": 1200},
            "unchanged bounded cell timeouts required")
    for key in ("controller", "worker"):
        value = spec.get(key)
        require(type(value) is dict and set(value) == {"path", "sha256"} and Path(value["path"]).is_absolute(),
                "exact absolute binary binding required")
        ledger.hash_string(value["sha256"])
    argv = spec.get("argv")
    require(type(argv) is list and argv and all(type(v) is str for v in argv)
            and argv[0] == spec["controller"]["path"], "explicit controller argv required")
    require(type(spec.get("device_unique_id")) is int and spec["device_unique_id"] > 0, "physical device identity required")
    requests = 1
    required = {"--worker": spec["worker"]["path"], "--worker-sha256": spec["worker"]["sha256"],
                "--device-unique-id": str(spec["device_unique_id"]), "--max-batches": str(requests * 135),
                "--context": "8192", "--pages": "512", "--submission": "ordered",
                "--wave-target-mode": "combined", "--layer-projection": "c1-wave",
                "--prefill-kv-mode": "parallel-prefill16-v27", "--split-attention-mode": "split8-v21",
                "--c1-packet-mode": "packed64-v29", "--gemv-mode": "baseline",
                "--ordered64-kv-copy-mode": "parallel-c1-v19"}
    require("--token-program-fence-mode" not in argv, "boundary-fence composition is forbidden")
    require("--token-program-backend" not in argv, "dedicated ordered64 packet-tick executable required")
    require(not set(argv) & {"--ordered64-baseline-packet-ticks", "--native-prefill-rows",
        "--prefill32-pages-mode", "--ordered64-runtime-counters", "--ordered64-host-timing",
        "--model-timestamps-output", "--host-timing", "--runtime-profile",
        "--ordered64-active-poll-10ms", "--diagnostic-active-poll-10ms"},
        "unrelated baseline, native-width, or diagnostic selector")
    sidecar = spec.get("packet_sidecar")
    require(type(sidecar) is str and Path(sidecar).is_absolute()
            and Path(sidecar).parts[-4:] == ("cells", "counter-A", "cell-results", "packet-ticks.json"),
            "fixed private per-cell packet sidecar required")
    required["--ordered64-packet-ticks"] = sidecar
    copy_path = spec.get("setup_expected", {}).get("kv_copy_artifact_path")
    require(type(copy_path) is str and Path(copy_path).is_absolute(), "actual V19 image path")
    required["--ordered64-kv-copy-artifact"] = copy_path
    require(type(spec.get("kernel_catalog")) is dict and spec["kernel_catalog"], "bound image kernel catalog required")
    for key, wanted in required.items():
        require(argv.count(key) == 1 and argv.index(key) + 1 < len(argv)
                and argv[argv.index(key) + 1] == wanted, "exact argv selector required: " + key)
    for key in ("--live-stdin", "--runtime-cache-admission", "--runtime-operational", "--queue-rollover",
                "--disable-prefix-cache", "--prune-output-head"):
        require(argv.count(key) == 1, "exact runtime flag required: " + key)
    require(type(spec.get("prompt")) is str and spec["prompt"], "frozen prompt required")
    reference = spec.get("reference")
    require(type(reference) is dict and type(reference.get("generated_token_ids")) is list
            and len(reference["generated_token_ids"]) == 128 and all(type(v) is int and v >= 0 for v in reference["generated_token_ids"]),
            "exact128 reference IDs required")
    bytes.fromhex(reference["generated_utf8_hex"]).decode("utf-8")
    for key in ("setup_expected", "profile_expected", "closed_expected"):
        require(type(spec.get(key)) is dict and spec[key], "explicit frozen runtime metadata required")
    require(all(key in spec["setup_expected"] for key in ("model_bundle_id", "target_model_id")),
            "actual frozen model IDs required")
    return (1, 0)


def common_argv(spec):
    return ["<diagnostic-controller>", *spec["argv"][1:]]


def composition(value, spec):
    exact(value, COMMON, "fixed652 graph composition drift")
    exact(value, {"live_profile": ticks.PROFILE}, "ordinary ordered64 packet-tick entry required")
    require(not any(key in value for key in FORBIDDEN), "unrelated optimization/instrumentation in token cell")


def check_setup(value, spec):
    count, _ = shape(spec)
    composition(value, spec)
    profile = value.get("performance_profile")
    composition(profile, spec)
    exact(value, {"schema": "FerricQwen3TpBatchSetupV2", "authority": "none", "attention_mode": "query-hoist-v14",
                 "tensor_parallel": 1, "model": "Qwen/Qwen3-8B", "dtype": "BF16", "target": "gfx950:xnack-",
                 "head_precision": "fp32-v8", "argmax_mode": "wave-v11", "runtime_ordered_batches": True,
                 "prefix_cache": False, "context_tokens": 8192, "physical_pages": 512, "prefill_chunk": 16,
                 "batch_tokens": 32, "performance_qualified": False, "serving_qualified": False,
                 "output_head_pruning": True, "max_batches": count * 135,
                 "controller_sha256": spec["controller"]["sha256"], "worker_sha256": spec["worker"]["sha256"],
                 "running_worker_sha256": [spec["worker"]["sha256"]], "device_unique_ids": [spec["device_unique_id"]]},
          "actual token cell setup drift")
    exact(profile, {"attention": "query-hoist-v14", "runtime_profiling": False,
                    "dispatch_sequences": False, "runtime_cache_admission": True, "runtime_operational": True,
                    "queue_rollover": True, "runtime_ordered_batches": True, "projection": "mfma",
                    "argmax_mode": "wave-v11"}, "runtime policy drift")
    require("model_timestamps" not in value
            and ledger.canonical(profile.get("model_timestamps")) == ledger.canonical(ticks.METADATA),
            "exact frequency-unspecified packet-tick metadata required")
    exact(value, spec["setup_expected"], "frozen setup metadata changed")
    exact(profile, spec["profile_expected"], "frozen profile metadata changed")
    pids = value.get("worker_pids")
    require(type(pids) is list and len(pids) == 1 and type(pids[0]) is int and 1 < pids[0] <= 0xffffffff,
            "one actual worker PID required")
    return {key: ledger.hash_string(value[key]) for key in ("model_bundle_id", "target_model_id")}


def check_closed(value, setup, dispatches, spec):
    count, _ = shape(spec)
    composition(value, spec)
    exact(value, {"schema": "FerricQwen3TpBatchClosedV2", "authority": "none", "attention_mode": "query-hoist-v14",
                 "performance_qualified": False, "worker_pids": setup["worker_pids"], "execution_completed": True,
                 "all_workers_exited": True, "rank_dispatch_counts": [count * 87711]}, "unclean or incomplete token close")
    exact(value, spec["closed_expected"], "frozen close metadata changed")
    require(dispatches == count * 87711, "complete token cell dispatch count changed")


def summarize(records, warmups):
    measured = records[warmups:]
    require(measured, "nonempty request cohort required")
    for row in records:
        require(row["output_tokens"] == 128 and row["batches"] == 135 and row["dispatches"] == 87711,
                "complete exact128 native request required")
        require(all(type(row.get(key)) is int and row[key] > 0
                    for key in ("arrival_ns", "completed_ns", "ttft_ns", "tpot_ns")), "positive native ingress timing required")
        require(row["completed_ns"] > row["arrival_ns"], "native request clock order")
    require(all(after["arrival_ns"] > before["completed_ns"] for before, after in zip(records, records[1:])),
            "native requests overlap")
    span = measured[-1]["completed_ns"] - measured[0]["arrival_ns"]
    return {"requests": [{"ttft_ms": r["ttft_ns"] / 1e6, "tpot_ms": r["tpot_ns"] / 1e6,
                           "started_ns": r["arrival_ns"], "completed_ns": r["completed_ns"], "output_tokens": 128}
                          for r in measured], "window_ns": span,
            "warmups_excluded": warmups, "finite_output_tokens_per_second": len(measured) * 128e9 / span,
            "latency_admitted": False, "scope": "instrumented native ingress diagnostics; not a benchmark"}


def consume(controller, spec, *, runner, legacy, evidence, deadline,
            before_requests=lambda _: None, before_request=lambda _: None, after_requests=lambda _: None):
    count, warmups = shape(spec)
    setup = controller.next(deadline)
    model = check_setup(setup, spec)
    events = legacy.Events(runner, "split8-v21")
    ready = controller.next(deadline)
    events.validate(ready)
    require(ready.get("event") == "ready" and ready.get("clock") == "monotonic_ns_since_live_start"
            and ready.get("eos_policy") == "fixed output count" and ready.get("context_tokens") == 8192
            and ready.get("physical_pages") == 512, "exact native readiness required")
    before_requests(setup)
    records = []
    for index in range(count):
        before_request(index)
        request_id = index + 1
        name = "packet-ticks-diagnostic-0"
        parity = evidence.StreamParity(controller, spec["reference"])
        parity.send({"schema": runner.COMMAND, "op": "submit", "request_id": request_id, "name": name,
                     "prompt": spec["prompt"], "new_tokens": 128}, deadline)
        before = events.batches, events.dispatches
        record = runner.collect_request(parity, events, request_id, name, spec["reference"], deadline)
        require((events.batches - before[0], events.dispatches - before[1]) == (135, 87711)
                and record["output_tokens"] == 128 and parity.complete and parity.tokens == 128,
                "independent token IDs/streamed/final bytes did not match")
        record.update(warmup=index < warmups, batches=135, dispatches=87711,
                      streamed_utf8_sha256=hashlib.sha256(parity.decoded).hexdigest())
        records.append(record)
    after_requests(setup)
    controller.send({"schema": runner.COMMAND, "op": "drain"}, deadline)
    draining = controller.next(deadline)
    events.validate(draining)
    require(draining.get("event") == "draining" and draining.get("reason") == "command", "clean drain required")
    stopped = controller.next(deadline)
    require(stopped.get("schema") == runner.EVENT and stopped.get("authority") == "none"
            and stopped.get("event") == "stopped" and stopped.get("reason") == "drained"
            and stopped.get("batches") == count * 135
            and runner.integer(stopped.get("emission_started_ns"), "stopped emission") >= events.emission,
            "complete clean stopped event required")
    closed = controller.next(deadline)
    check_closed(closed, setup, events.dispatches, spec)
    require(events.batches == count * 135 and events.dispatches == count * 87711, "cell totals changed")
    terminal = controller.next(deadline)
    sidecar = ticks.read_sidecar(spec["packet_sidecar"])
    packet_replay = ticks.validate(sidecar, terminal, sidecar_path=spec["packet_sidecar"],
                                  device=spec["device_unique_id"], allowed_kernels=spec["kernel_catalog"],
                                  copy_identity={key: closed[key] for key in ticks.COPY_KEYS})
    controller.finish(deadline)
    return {"setup": setup, "closed": closed, "model_identity": model, "requests": records,
            "batches": count * 135, "dispatches": count * 87711, "exact_output_tokens_checked": count * 128,
            "summary": summarize(records, warmups), "packet_ticks": packet_replay, "packet_terminal": terminal}


class Replay:
    def __init__(self, output, runner, count):
        self.runner = runner
        for name in ("stdin.raw", "stdout.raw"):
            runner.identity(output / name, maximum=runner.MAX_STREAM)
        self.commands = (output / "stdin.raw").read_bytes().splitlines()
        self.events = (output / "stdout.raw").read_bytes().splitlines()
        require(len(self.commands) == count + 1 and 0 < len(self.events) <= count * 512, "bounded exact native transcript")

    def next(self, _deadline):
        require(self.events, "truncated native replay")
        return self.runner.decode(self.events.pop(0))

    def send(self, value, _deadline):
        require(self.commands and self.runner.decode(self.commands.pop(0)) == value, "raw command drift")

    def finish(self, _deadline):
        require(not self.commands and not self.events, "trailing native raw transcript")


def counter_replay(raw, spec, setup):
    shape(spec)
    require(not raw, "packet ticks exclude stderr counters or diagnostics")
    return None


def retained_bytes(path, maximum):
    path = Path(path)
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_uid == os.getuid()
            and before.st_size <= maximum, "owned bounded regular transcript required")
    with path.open("rb") as stream:
        raw = stream.read(maximum + 1)
    after = path.lstat()
    require(len(raw) == before.st_size and len(raw) <= maximum
            and all(getattr(before, key) == getattr(after, key) for key in
                    ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")),
            "transcript changed during bounded read")
    return raw


def run_cell(spec, output, *, runner, legacy, counter, evidence, admission):
    with lifecycle.handling_stop():
        return _run_cell(spec, output, runner=runner, legacy=legacy, counter=counter,
                         evidence=evidence, admission=admission)


def _run_cell(spec, output, *, runner, legacy, counter, evidence, admission):
    count, _ = shape(spec)
    require(callable(admission), "outer resource/device admission hook is mandatory")
    output = Path(output)
    output.mkdir(mode=0o700)
    require(Path(spec["packet_sidecar"]).parent == output and not os.path.lexists(spec["packet_sidecar"]),
            "fresh sidecar inside owned cell output required")
    row = {"schema": "FerricPacketTicksCellResultV1", "spec_sha256": ledger.digest(spec), "accepted": False,
           "arm": spec["arm"], "mode": spec["mode"], "instrumented": spec["mode"] == "counters",
           "latency_admitted": False, "started_ns": time.monotonic_ns(), "admission": []}
    controller = None
    try:
        for key in ("controller", "worker"):
            require(runner.identity(Path(spec[key]["path"]))["sha256"] == spec[key]["sha256"], "binary custody drift")
        def admitted(phase, setup):
            receipt = admission(phase, setup)
            row["admission"].append(receipt)
            require(type(receipt) is dict and receipt.get("accepted") is True, "outer admission did not accept cell")

        admitted("preflight", None)
        deadline = time.monotonic() + spec["timeouts"]["cell_seconds"]
        with lifecycle.deferred_stop():
            controller = lifecycle.controller_class(runner)(spec["argv"], output, deadline)
        controller.deadline = min(deadline, time.monotonic() + spec["timeouts"]["setup_seconds"])

        def before(setup):
            admitted("active", setup)

        def before_request(_index):
            controller.deadline = min(deadline, time.monotonic() + spec["timeouts"]["request_seconds"])

        def after(setup):
            admitted("active", setup)
            controller.deadline = min(deadline, time.monotonic() + 15)

        result = consume(controller, spec, runner=runner, legacy=legacy, evidence=evidence, deadline=deadline,
                         before_requests=before, before_request=before_request, after_requests=after)
        cleanup = controller.close()
        controller = None
        require(cleanup.get("cleanup_ok") is True and cleanup.get("child_reaped") is True
                and cleanup.get("owned_descendants_absent") is True and cleanup.get("returncode") == 0
                and cleanup.get("errors") == [] and cleanup.get("term_sent") is False
                and cleanup.get("kill_sent") is False, "clean unsignaled child teardown required")
        replay = consume(Replay(output, runner, count), spec, runner=runner, legacy=legacy,
                         evidence=evidence, deadline=0)
        require(replay == result, "raw native replay disagrees")
        stderr = retained_bytes(output / "stderr.raw", 65536)
        if spec["mode"] == "counters":
            counter_replay(stderr, spec, result["setup"])
        else:
            require(not stderr, "uninstrumented controller stderr must be empty")
        for key in ("controller", "worker"):
            require(runner.identity(Path(spec[key]["path"]))["sha256"] == spec[key]["sha256"], "postflight binary drift")
        admitted("postflight", None)
        require(ledger.sha(ticks.read_sidecar(spec["packet_sidecar"]))
                == result["packet_ticks"]["raw_capture_sha256"], "postflight sidecar changed")
        row.update(result, cleanup=cleanup, raw_replay_passed=True, accepted=True,
                   latency_admitted=spec["mode"] == "latency",
                   raw_sha256={name: ledger.sha(retained_bytes(output / name, runner.MAX_STREAM))
                               for name in ("stdin.raw", "stdout.raw", "stderr.raw")})
    except BaseException as error:
        row["error"] = type(error).__name__ + ": " + str(error)
        raise
    finally:
        with lifecycle.deferred_stop(deliver=False):
            if controller is not None:
                row["failed_cleanup"] = controller.close()
            row["finished_ns"] = time.monotonic_ns()
            runner.save(output / "result.json", row)
    return row


def evaluate_campaign(plan, cells, counter_cells):
    require(type(plan) is dict and set(plan) == {"schema", "cell_spec_sha256", "latency_admitted", "scope"}
            and plan["schema"] == "FerricPacketTicksDiagnosticPlanV1"
            and plan["latency_admitted"] is False
            and plan["scope"] == "one instrumented ordered64 request; uncalibrated non-shader-only packet ticks",
            "exact non-comparative diagnostic plan required")
    require(cells == [] and type(counter_cells) is list and len(counter_cells) == 1,
            "one replayed diagnostic cell and no latency cohort required")
    entry = counter_cells[0]
    require(entry.get("cell_id") == "counter-A" and ledger.digest(entry["spec"]) == plan["cell_spec_sha256"],
            "diagnostic result must bind its predeclared cell")
    result = entry["result"]
    exact(result, {"accepted": True, "raw_replay_passed": True, "instrumented": True,
                   "latency_admitted": False, "exact_output_tokens_checked": 128,
                   "batches": 135, "dispatches": 87711}, "complete instrumented diagnostic required")
    spec = entry["spec"]
    replay = ticks.validate(ticks.read_sidecar(spec["packet_sidecar"]), result["packet_terminal"],
                           sidecar_path=spec["packet_sidecar"], device=spec["device_unique_id"],
                           allowed_kernels=spec["kernel_catalog"],
                           copy_identity={key: result["closed"][key] for key in ticks.COPY_KEYS})
    require(ledger.canonical(replay) == ledger.canonical(result["packet_ticks"]),
            "packet summary differs from retained exact sidecar")
    return {"schema": "FerricPacketTicksDiagnosticReportV1", "accepted": True,
            "latency_admitted": False, "performance_qualified": False, "default_promotion_allowed": False,
            "scope": plan["scope"], "model_identity": result["model_identity"],
            "exact_output_tokens_checked": 128, "batches": 135, "dispatches": 87711,
            "packet_ticks": replay, "instrumented_ingress": result["summary"],
            "cell_custody": entry["custody"], "raw_sha256": result["raw_sha256"]}
