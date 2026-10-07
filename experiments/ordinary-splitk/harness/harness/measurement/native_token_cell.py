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
_SELECTION = importlib.util.spec_from_file_location("splitk_selection", Path(__file__).with_name("splitk_selection.py"))
selection = importlib.util.module_from_spec(_SELECTION)
_SELECTION.loader.exec_module(selection)

BACKENDS = {arm: {"backend": "ordinary-ordered64", "profile": selection.PROFILE}
            for arm in selection.ARMS}

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
             "requested_prefill16_ordered_mode", "packed_gate_up_mode", "packed_down_mode",
             "token_program", "prefill_program")


def exact(actual, wanted, message):
    require(type(actual) is dict and all(type(actual.get(k)) is type(v) and actual[k] == v for k, v in wanted.items()), message)


def shape(spec):
    require(spec.get("schema") == "FerricSplitKModelCellPlanV1" and spec.get("arm") in BACKENDS
            and spec.get("mode") in ("correctness", "latency"), "exact native token cell required")
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
    selection.cli(spec)
    require("--token-program-backend" not in argv, "ordinary same-binary down selector required")
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


def expected_mechanism(arm):
    return selection.expected_mechanism(arm)


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
    exact(value, COMMON, "ordinary V19 prefill16/split8/packed64 composition changed")
    exact(value, {"live_profile": selection.PROFILE,
                  "requested_gemv_mode": "baseline", "gemv_mode": "baseline"},
          "explicit same-binary baseline GEMV composition changed")
    selection.metadata(value.get("splitk_down"), spec)
    require(not any(key in value for key in FORBIDDEN),
            "unrelated program/width/fence/instrumentation in split-K cell")


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
                 "all_workers_exited": True, "rank_dispatch_counts": [count * expected_mechanism(spec["arm"])["model_dispatches"]]}, "unclean or incomplete token close")
    exact(value, spec["closed_expected"], "frozen close metadata changed")
    require(dispatches == count * expected_mechanism(spec["arm"])["model_dispatches"], "complete token cell dispatch count changed")


def summarize(records, warmups, arm):
    measured = records[warmups:]
    require(measured, "nonempty request cohort required")
    for row in records:
        require(row["output_tokens"] == 128 and row["batches"] == 135 and row["dispatches"] == expected_mechanism(arm)["model_dispatches"],
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
    events = selection.Events(runner, legacy, spec["arm"])
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
        require((events.batches - before[0], events.dispatches - before[1]) == (135, expected_mechanism(spec["arm"])["model_dispatches"])
                and record["output_tokens"] == 128 and parity.complete and parity.tokens == 128,
                "independent token IDs/streamed/final bytes did not match")
        record.update(warmup=index < warmups, batches=135,
                      dispatches=expected_mechanism(spec["arm"])["model_dispatches"],
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
    require(events.batches == count * 135 and events.dispatches == count * expected_mechanism(spec["arm"])["model_dispatches"], "cell totals changed")
    controller.finish(deadline)
    return {"setup": setup, "closed": closed, "model_identity": model, "requests": records,
            "batches": count * 135, "dispatches": count * expected_mechanism(spec['arm'])['model_dispatches'],
            "exact_output_tokens_checked": count * 128,
            "summary": summarize(records, warmups, spec["arm"])}


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
    row = {"schema": "FerricSplitKModelCellResultV1", "spec_sha256": ledger.digest(spec), "accepted": False,
           "arm": spec["arm"], "mode": spec["mode"], "instrumented": False,
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
        require(not stderr, "uninstrumented controller stderr must be empty")
        if spec["mode"] == "correctness":
            row["mechanism"] = expected_mechanism(spec["arm"])
        setup_raw = retained_bytes(output / "stdout.raw", runner.MAX_STREAM).splitlines(keepends=True)[0]
        row["setup_provenance"] = selection.setup_provenance(setup_raw, spec["experimental_retention"])
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


def evaluate_campaign(plan, cells, mechanism_cells):
    """Only raw-replayed exact same-binary cells can enter the frozen ABBA arithmetic."""
    ledger.validate_plan(plan)
    require(plan["timing_semantics"] == "native-ingress-v1", "native ingress timing only")
    require(type(cells) is list and len(cells) == 12 and type(mechanism_cells) is list
            and len(mechanism_cells) == 2, "twelve latency plus two untimed parity cells")
    common = None
    mechanisms, profiles, results = {}, {}, []
    for entry in mechanism_cells + cells:
        spec, result = entry["spec"], entry["result"]
        outer, completion = entry.get("outer"), entry.get("completion")
        exact(outer, {"status": 0, "cleanup_ok": True, "child_reaped": True, "errors": [],
            "term_sent": False, "kill_sent": False, "termination_reason": "completed"},
            "clean unsignaled outer completion")
        require(outer.get("postflight", {}).get("accepted") is True, "accepted final admission")
        exact(completion, {"accepted": True, "model_stable": True, "input_files_stable": True},
              "stable completed inputs")
        retained_hash = lambda value: ledger.sha((json.dumps(value, indent=2, sort_keys=True,
                                                              allow_nan=False) + "\n").encode())
        require(outer.get("completion_sha256") == retained_hash(completion)
                and completion.get("cell_result_sha256") == retained_hash(result),
                "exact retained completion and result chain")
        count, warmups = shape(spec)
        exact(result, {"schema": "FerricSplitKModelCellResultV1", "spec_sha256": ledger.digest(spec),
                       "accepted": True, "raw_replay_passed": True, "instrumented": False},
              "completed uninstrumented raw-replayed cell")
        check_setup(result["setup"], spec)
        check_closed(result["closed"], result["setup"], result["dispatches"], spec)
        require(result["exact_output_tokens_checked"] == count * 128
                and result["batches"] == count * 135
                and result["summary"] == summarize(result["requests"], warmups, spec["arm"]),
                "complete request count and actual ingress metrics")
        raw = result.get("raw_sha256")
        require(type(raw) is dict and set(raw) == {"stdin.raw", "stdout.raw", "stderr.raw"},
                "closed raw transcript bindings")
        for digest in raw.values():
            ledger.hash_string(digest)
        provenance = result["setup_provenance"]
        require(provenance["strict_clippy"] == "failed"
                and provenance["binary_lint_coverage"] == "incomplete"
                and provenance["experimental_retention"] == spec["experimental_retention"]
                and provenance["production_qualified"] is False
                and provenance["default_promotion"] is False,
                "failed lint scope must remain explicit")
        require(result["finished_ns"] > result["started_ns"] > 0, "positive cell lifetime")
        comparable = {key: spec[key] for key in ("controller", "worker", "device_unique_id",
            "prompt", "reference", "splitk", "experimental_retention", 'setup_expected',
            'profile_expected', 'closed_expected')}
        comparable['argv'] = selection.common_argv(spec)
        comparable['argv'][comparable['argv'].index('--max-batches') + 1] = '<request-budget>'
        require(common is None or comparable == common, "same binaries/images/scratch/provenance")
        common = comparable
        require(plan["reference_sha256"] == ledger.digest(spec["reference"]), "fixed reference")
        arm = spec["arm"]
        if spec["mode"] == "correctness":
            require(arm not in mechanisms and result["latency_admitted"] is False,
                    "one untimed exact-output graph mechanism cell per arm")
            selection.same(result["mechanism"], expected_mechanism(arm), "actual raw graph totals")
            mechanisms[arm] = result["mechanism"]
            continue
        require(spec["mode"] == "latency" and result["latency_admitted"] is True,
                "only uninstrumented measured cohort")
        require(plan["profiles"][arm] == profile_for_spec(spec), "predeclared profile")
        normalized = selection.common_argv(spec)
        require(not profiles or normalized == next(iter(profiles.values())), "only mode argv differs")
        profiles[arm] = normalized
        results.append({"arm": arm, "cell_id": entry["cell_id"], "requests": result["summary"]["requests"],
                        "window_ns": result["summary"]["window_ns"],
                        "median_tpot_ms": statistics.median(row["tpot_ms"]
                            for row in result["summary"]["requests"])})
    require(set(mechanisms) == set(profiles) == {"A", "B"}, "complete both-arm evidence")
    require(len({cell["result"]["raw_sha256"]["stdout.raw"] for cell in cells}) == 12,
            "no raw transcript reused across cells")
    require(len({cell["result"]["raw_sha256"]["stdin.raw"] for cell in cells}) == 1,
            "identical raw workload commands")
    require(all(after["result"]["started_ns"] >= before["result"]["finished_ns"]
                for before, after in zip(cells, cells[1:])), "serial nonoverlapping cells")
    report = ledger.compare_results(plan, ledger.digest({"cells": cells, "mechanism_cells": mechanism_cells}), results)
    report["mechanism"] = mechanisms
    report["experimental_provenance"] = mechanism_cells[0]["result"]["setup_provenance"]
    report["production_qualified"] = False
    report["default_promotion"] = False
    report["scope"] = "Within ordinary ordered64/V19, unpaired down-only experiment; no vendor/backend promotion"
    report["raw_custody"] = [{"cell_id": cell["cell_id"], "spec_sha256": cell["result"]["spec_sha256"],
                              "raw_sha256": cell["result"]["raw_sha256"]}
                             for cell in mechanism_cells + cells]
    return report
