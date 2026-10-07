#!/usr/bin/env python3
"""Token-program cell execution/replay inside the existing bounded GPU supervisor.

This module intentionally has no standalone GPU launch entry. The integration
owner supplies pinned legacy helpers and the unchanged outer resource guard.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import statistics
import time


_SPEC = importlib.util.spec_from_file_location("v14_ledger", Path(__file__).with_name("abba_ledger.py"))
ledger = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(ledger)
require = ledger.require
_LIFECYCLE = importlib.util.spec_from_file_location("v14_lifecycle", Path(__file__).with_name("native_lifecycle.py"))
lifecycle = importlib.util.module_from_spec(_LIFECYCLE)
_LIFECYCLE.loader.exec_module(lifecycle)

BACKENDS = {
    "A": {"backend": "ordered64-groups-v1", "worker_entry": "--diagnostic-token-program-v1",
          "profile": "prefill16-decode-fixed-token-program-v1", "groups": 11},
    "B": {"backend": "native-whole-program-v1", "worker_entry": "--diagnostic-token-program-native-v1",
          "profile": "prefill16-decode-native-whole-token-program-v1", "groups": 1},
}
COMMON = {"wave_target_mode": "combined", "layer_projection": "c1-wave", "rmsnorm_mode": "wave-v15",
          "submission": "ordered", "benchmark_admitted": False, "serving_admitted": False,
          "requested_prefill_kv_mode": "parallel-prefill16-v27", "prefill_kv_mode": "parallel-prefill16-v27",
          "requested_split_attention_mode": "split8-v21", "split_attention_mode": "split8-v21",
          "requested_c1_packet_mode": "packed64-v29", "c1_packet_mode": "packed64-v29",
          "ordered_wire_mode": "ordered64", "packed_c1_group_bound": 64, "non_c1_group_bound": 16,
          "requested_kv_copy_mode": "parallel-c1-v19", "kv_copy_mode": "parallel-c1-v19",
          "kv_append_mode": "parallel-c1-v19"}
FORBIDDEN = ("model_timestamps", "runtime_profile", "runtime_profile_snapshot", "dispatch_timestamps",
             "host_timing_schema", "host_diagnostic_scope", "runtime_counter_schema", "runtime_counter_scope",
             "diagnostic_max_model_batches", "ordered64_runtime_counters", "ordered64_packet_ticks",
             "active_poll", "wait_policy", "ordered64_wait_policy", "prefill32_pages_mode",
             "requested_prefill32_pages_mode", "prefill_scheduling", "prefill16_ordered_mode",
             "requested_prefill16_ordered_mode", "packed_gate_up_mode", "packed_down_mode")


def exact(actual, wanted, message):
    require(type(actual) is dict and all(type(actual.get(k)) is type(v) and actual[k] == v for k, v in wanted.items()), message)


def shape(spec):
    require(spec.get("schema") == "FerricNativeTokenCellPlanV1" and spec.get("arm") in BACKENDS
            and spec.get("mode") in ("correctness", "counters", "latency"), "exact native token cell required")
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
    requests = 6 if spec["mode"] == "latency" else 1
    required = {"--worker": spec["worker"]["path"], "--worker-sha256": spec["worker"]["sha256"],
                "--device-unique-id": str(spec["device_unique_id"]), "--max-batches": str(requests * 135),
                "--context": "8192", "--pages": "512", "--submission": "ordered",
                "--wave-target-mode": "combined", "--layer-projection": "c1-wave",
                "--prefill-kv-mode": "parallel-prefill16-v27", "--split-attention-mode": "split8-v21",
                "--c1-packet-mode": "packed64-v29", "--gemv-mode": "baseline",
                "--ordered64-kv-copy-mode": "parallel-c1-v19"}
    if spec["mode"] == "counters":
        required["--token-program-backend"] = BACKENDS[spec["arm"]]["backend"]
    else:
        require("--token-program-backend" not in argv, "counter selector in latency binary")
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
    return (6, 2) if spec["mode"] == "latency" else (1, 0)


def token_metadata(arm, counters=False):
    backend = BACKENDS[arm]
    result = {"command": "execute_token_program", "c1_dispatches": 652, "dynamic_slots": 180,
            "worker_entry": backend["worker_entry"], "backend": backend["backend"],
            "backend_identity_checked": True, "backend_fallback": False, "counter_diagnostic": counters,
            "context_range": [128, 256],
            "ordinary_fallback": True, "prefill_unchanged": True, "active_poll": False,
            "performance_qualified": False}
    if counters:
        result.update(counter_schema="FerricTokenProgramCountersV1", counter_stream="controller stderr",
                      counter_phases=["worker_start", "before_close"], latency_sample_admitted=False,
                      max_model_batches=256)
    return result


def profile_for_spec(spec):
    shape(spec)
    require(spec["mode"] == "latency", "profile manifest describes uninstrumented latency arm")
    return {"artifacts": {"controller": spec["controller"]["sha256"], "worker": spec["worker"]["sha256"]},
            "configuration": {"backend": BACKENDS[spec["arm"]]["backend"],
                              "argv": ["<backend-controller>", *spec["argv"][1:]],
                              "device_unique_id": spec["device_unique_id"],
                              "reference_sha256": ledger.digest(spec["reference"]),
                              "prompt_sha256": ledger.sha(spec["prompt"].encode()),
                              "setup_expected": spec["setup_expected"], "profile_expected": spec["profile_expected"],
                              "closed_expected": spec["closed_expected"]}}


def composition(value, spec):
    exact(value, COMMON, "fixed652 graph composition drift")
    backend = BACKENDS[spec["arm"]]
    profile = backend["profile"]
    if spec["mode"] == "counters":
        profile = profile.removesuffix("-v1") + "-counters-v1"
    exact(value, {"live_profile": profile, "token_program": token_metadata(spec["arm"], spec["mode"] == "counters")},
          "positive backend identity or complete token graph changed")
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
    exact(profile, {"attention": "query-hoist-v14", "runtime_profiling": spec["mode"] == "counters",
                    "dispatch_sequences": False, "runtime_cache_admission": True, "runtime_operational": True,
                    "queue_rollover": True, "runtime_ordered_batches": True, "projection": "mfma",
                    "argmax_mode": "wave-v11"}, "runtime policy drift")
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
            "warmups_excluded": warmups, "finite_output_tokens_per_second": len(measured) * 128e9 / span}


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
        name = ("warmup-" + str(index)) if index < warmups else ("measured-" + str(index - warmups))
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
    controller.finish(deadline)
    return {"setup": setup, "closed": closed, "model_identity": model, "requests": records,
            "batches": count * 135, "dispatches": count * 87711, "exact_output_tokens_checked": count * 128,
            "summary": summarize(records, warmups)}


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
    require(spec["mode"] == "counters" and len(raw) <= 65536, "bounded counter-only stderr required")
    lines = raw.splitlines()
    require(len(lines) == 2, "exact start/close counter pair required")
    rows = [ledger.parse(line) for line in lines]
    backend = BACKENDS[spec["arm"]]
    fields = {"executions", "dispatches", "publications", "final_waits", "retirement_signals",
              "staging_ns", "kernarg_initialized_bytes"}
    for index, row in enumerate(rows):
        exact(row, {"schema": "FerricTokenProgramCountersV1", "authority": "none", "performance_qualified": False,
                    "runtime_profiling": True, "latency_sample_admitted": False, "backend": backend["backend"],
                    "worker_entry": backend["worker_entry"],
                    "live_profile": backend["profile"].removesuffix("-v1") + "-counters-v1",
                    "process_id": setup["worker_pids"][0], "device_unique_id": spec["device_unique_id"],
                    "ordinal": index, "phase": "worker_start" if index == 0 else "before_close",
                    "scope": "successful token executions only; excludes registration, ordinary prefill and readback",
                    "staging_scope": "host staging wall time; not GPU time",
                    "kernarg_bytes_scope": "initialized stores; baseline includes slot clear and exact copy"},
              "counter identity/scope changed")
        counters = row.get("counters")
        require(type(counters) is dict and set(counters) == fields
                and all(type(v) is int and v >= 0 for v in counters.values()), "exact nonnegative counters required")
        if index == 0:
            require(all(v == 0 for v in counters.values()), "start counters must be zero")
        else:
            exact(counters, {"executions": 127, "dispatches": 82804,
                             "publications": 127 * backend["groups"], "final_waits": 127 * backend["groups"],
                             "retirement_signals": 82804}, "whole-request decode mechanism failed")
            require(counters["staging_ns"] > 0 and counters["kernarg_initialized_bytes"] > 0,
                    "staging counters missing")
            if spec["arm"] == "A":
                require(counters["kernarg_initialized_bytes"] > 127 * 652 * 65536,
                        "grouped baseline must account slot clearing plus copied arguments")
    return {"start": rows[0], "end": rows[1], "delta": rows[1]["counters"],
            "latency_admitted": False, "mechanism_qualified": True}


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
    row = {"schema": "FerricNativeTokenCellResultV1", "spec_sha256": ledger.digest(spec), "accepted": False,
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
            row["mechanism"] = counter_replay(stderr, spec, result["setup"])
        else:
            require(not stderr, "uninstrumented controller stderr must be empty")
        for key in ("controller", "worker"):
            require(runner.identity(Path(spec[key]["path"]))["sha256"] == spec[key]["sha256"], "postflight binary drift")
        admitted("postflight", None)
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
    """Summarize already replayed run_cell receipts; not a replacement for custody.

    Call this in the guarded driver that owns run_cell results. Offline users
    must replay bound raw transcripts first, not trust a supplied true boolean.
    """
    ledger.validate_plan(plan)
    require(plan["timing_semantics"] == "native-ingress-v1", "native timing plan required")
    require(type(cells) is list and len(cells) == 12 and type(counter_cells) is list and len(counter_cells) == 2,
            "twelve latency cells and separate two-arm mechanism cells required")
    normalized = None
    normalized_counters = None
    actual_by_arm = {}
    mechanisms = {}
    results = []
    for entry in counter_cells + cells:
        spec, result = entry["spec"], entry["result"]
        outer, completion = entry.get("outer"), entry.get("completion")
        exact(outer, {"status": 0, "cleanup_ok": True, "child_reaped": True, "errors": [],
                      "term_sent": False, "kill_sent": False, "termination_reason": "completed"},
              "final outer supervisor did not accept this cell")
        require(outer.get("postflight", {}).get("accepted") is True, "final outer postflight not accepted")
        exact(completion, {"accepted": True, "model_stable": True, "input_files_stable": True},
              "outer completion/input stability required")
        retained_hash = lambda value: ledger.sha((json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n").encode())
        require(outer.get("completion_sha256") == retained_hash(completion)
                and completion.get("cell_result_sha256") == retained_hash(result),
                "outer completion must bind this exact retained cell result")
        count, warmups = shape(spec)
        require(result.get("schema") == "FerricNativeTokenCellResultV1"
                and result.get("spec_sha256") == ledger.digest(spec)
                and result.get("accepted") is True and result.get("raw_replay_passed") is True,
                "bound completed raw-replayed native cell required")
        check_setup(result["setup"], spec)
        check_closed(result["closed"], result["setup"], result["dispatches"], spec)
        require(result.get("exact_output_tokens_checked") == count * 128 and result.get("batches") == count * 135,
                "complete request count required")
        require(result["summary"] == summarize(result["requests"], warmups), "native summary changed")
        raw = result.get("raw_sha256")
        require(type(raw) is dict and set(raw) == {"stdin.raw", "stdout.raw", "stderr.raw"}, "raw custody hashes required")
        for value in raw.values():
            ledger.hash_string(value)
        require(result.get("finished_ns", 0) > result.get("started_ns", 0) > 0, "positive actual cell lifetime required")
        common = {key: spec[key] for key in ("worker", "device_unique_id", "prompt", "reference")}
        require(plan["reference_sha256"] == ledger.digest(spec["reference"]), "predeclared canonical native reference changed")
        require(normalized_counters is None or common == normalized_counters, "model/worker/device drift between mechanism and latency cells")
        normalized_counters = common
        arm = spec["arm"]
        if spec["mode"] == "counters":
            require(arm not in mechanisms and result.get("instrumented") is True
                    and result.get("latency_admitted") is False, "separate counter arm required")
            mechanism = result["mechanism"]
            raw_counter = b"\n".join(ledger.canonical(mechanism[key]) for key in ("start", "end")) + b"\n"
            require(counter_replay(raw_counter, spec, result["setup"]) == mechanism, "counter summary changed")
            mechanisms[arm] = mechanism["delta"]
            continue
        require(spec["mode"] == "latency" and result.get("instrumented") is False
                and result.get("latency_admitted") is True, "only full uninstrumented cohort can promote")
        require(plan["profiles"][arm] == profile_for_spec(spec), "predeclared native profile differs from actual spec")
        actual = {"controller": spec["controller"], "worker": spec["worker"]}
        require(arm not in actual_by_arm or actual_by_arm[arm] == actual, "binary changed within arm")
        actual_by_arm[arm] = actual
        comparable = {**spec, "arm": "<backend>", "controller": "<backend-controller>",
                      "argv": ["<backend-controller>", *spec["argv"][1:]]}
        require(normalized is None or comparable == normalized, "undeclared native composition change")
        normalized = comparable
        summary = result["summary"]
        results.append({"arm": arm, "cell_id": entry["cell_id"], "requests": summary["requests"],
                        "window_ns": summary["window_ns"],
                        "median_tpot_ms": statistics.median(r["tpot_ms"] for r in summary["requests"])})
    require(set(mechanisms) == {"A", "B"} and set(actual_by_arm) == {"A", "B"}, "both distinct backends required")
    require(actual_by_arm["A"]["controller"] != actual_by_arm["B"]["controller"], "different explicit backend controllers required")
    require(len({cell["result"]["raw_sha256"]["stdout.raw"] for cell in cells}) == 12,
            "native raw run cannot be reused across cells")
    require(len({cell["result"]["raw_sha256"]["stdin.raw"] for cell in cells}) == 1,
            "latency cells must receive identical raw commands")
    require(mechanisms["B"]["kernarg_initialized_bytes"] < mechanisms["A"]["kernarg_initialized_bytes"],
            "compact initialized-byte mechanism did not reduce stores")
    for arm in ("A", "B"):
        require(plan["mechanism"][arm] == {"dispatches_per_decode_token": 652,
                "publications_per_decode_token": BACKENDS[arm]["groups"],
                "waits_per_decode_token": BACKENDS[arm]["groups"], "retired_signals_per_decode_token": 652},
                "predeclared native mechanism differs from actual backend")
    require(all(after["result"]["started_ns"] >= before["result"]["finished_ns"]
                for before, after in zip(cells, cells[1:])), "cell starts overlap or are out of order")
    report = ledger.compare_results(plan, ledger.digest({"cells": cells, "counter_cells": counter_cells}), results)
    report["mechanism"] = mechanisms
    report["raw_custody"] = [{"cell_id": cell["cell_id"], "spec_sha256": cell["result"]["spec_sha256"],
                              "raw_sha256": cell["result"]["raw_sha256"]} for cell in counter_cells + cells]
    return report
