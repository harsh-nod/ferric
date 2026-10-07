"""Ordered submission using the unchanged component fixtures and parity gates."""
import time


def run_component(component, adapter, worker, fixture, v5, candidate, mode, device,
                  retain=lambda _row: None, check_alive=lambda: None,
                  clock=time.perf_counter_ns):
    session = adapter.OrderedSession(worker, mode, device)
    session.configure()
    records = component.records_for(fixture)
    component.allocate_shared(worker, records)
    plans = component.component_plans(worker, records, v5, candidate)
    component.require(set(plans) == {"wave", "mfma", "splitk"}
                      and [len(plans[name]) for name in ("wave", "mfma", "splitk")] == [1, 1, 2],
                      "exact one/one/two dispatch arms")
    before = component.check_inputs(worker, records)
    samples = []
    for cell in component.schedule():
        check_alive()
        component.reset_outputs(worker, records)
        timing = {**cell, **session.dispatch(plans[cell["arm"]],
            post_warmup=cell["phase"] == "sample", clock=clock)}
        retain({"event": "completed_unchecked", **timing})
        check_alive()
        output = component.checked_read(worker, records["output"],
            fixture.output_f32 + component.TAIL, "output")
        expected = (fixture.partials_f32 if cell["arm"] == "splitk"
                    else records["partials"]["data"])
        partials = component.checked_read(worker, records["partials"], expected, "partials")
        row = {**timing, "output_check": output, "partial_check": partials}
        retain({"event": "verified_sample", **row})
        samples.append(row)
    component.require(len(samples) == 30 and session.frontier == 44,
                      "fixed 30-cell/44-packet campaign")
    after = component.check_inputs(worker, records)
    component.require(before == after, "input custody changed")
    for record in records.values():
        _, payload = worker.command({"op": "free", "buffer": record["id"]}, expected="freed")
        component.require(not payload, "free response payload")
    return {"mode": mode, "inputs_before": before, "inputs_after": after, "samples": samples,
            "metadata": {arm: [plan["metadata"] for plan in entries] for arm, entries in plans.items()},
            "allocation_ids": {name: record["id"] for name, record in records.items()},
            "allocation_reuse": "one fixed shared set; no allocation within timed samples",
            "dispatch_packets": session.frontier, "queue_epoch": session.epoch,
            "performance_configuration": {"cache_kernel_admission": True,
                "operational_currentness": True, "profile": mode != "latency"},
            "counter_scope": (None if mode == "latency" else
                "previous snapshot plus batch; overlapping durations are not additive"),
            "host_timing_scope": "encoding, process check, trace and synchronous ordered IPC; excludes snapshots and parity reads",
            "worker_timing_scope": "ordered publication, completion and exit fence; excludes preparation/staging; not GPU-only"}
