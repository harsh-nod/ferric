//! Separate, bounded decode attribution. All durations are host wall time.
use super::{CommandV1, EngineeringTpRankTransportV1, ResponseV1, TpResult, Worker, wire};
use serde_json::{Value, json};
use std::io::Write;
use std::time::Instant;

const EXECUTIONS: usize = 127;

fn ns(start: Instant) -> TpResult<u64> {
    u64::try_from(start.elapsed().as_nanos()).map_err(|_| "diagnostic duration overflow".into())
}

fn monotonic_ns() -> TpResult<u64> {
    let time = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    u64::try_from(time.tv_sec).ok().and_then(|seconds| seconds.checked_mul(1_000_000_000))
        .and_then(|base| u64::try_from(time.tv_nsec).ok().and_then(|part| base.checked_add(part)))
        .ok_or_else(|| "diagnostic clock overflow".into())
}

fn delta(before: &Value, after: &Value, fields: usize) -> TpResult<Value> {
    let a = before.as_object().ok_or("counter object")?;
    let b = after.as_object().ok_or("counter object")?;
    if a.len() != fields || a.keys().ne(b.keys()) { return Err("counter roster changed".into()); }
    let mut result = serde_json::Map::new();
    for (key, start) in a {
        let difference = b[key].as_u64().and_then(|end| start.as_u64().and_then(|start| end.checked_sub(start)))
            .ok_or("counter is noninteger or regressed")?;
        result.insert(key.clone(), json!(difference));
    }
    Ok(Value::Object(result))
}

fn sum(rows: &[Value], key: &str) -> TpResult<u64> {
    rows.iter().try_fold(0_u64, |total, row| {
        total.checked_add(row[key].as_u64().ok_or("duration missing")?).ok_or_else(|| "duration sum overflow".into())
    })
}

#[derive(Default)]
pub(super) struct Recorder {
    program: u64,
    epoch: u64,
    packets: u64,
    start_frontier: u64,
    before: Option<Value>,
    partial_before: Value,
    partial_after: Value,
    start: Option<Instant>,
    start_monotonic_ns: u64,
    execute_start: Option<Instant>,
    execute_monotonic_ns: u64,
    planners: Vec<Value>,
    executions: Vec<Value>,
    reads: Vec<Value>,
    writes: Vec<Value>,
    complete: bool,
    incomplete_emitted: bool,
}

impl Recorder {
    fn active(&self) -> bool { self.start.is_some() && !self.complete }

    fn validate_totals(&self, general: &Value, token: &Value, end_frontier: u64) -> TpResult<()> {
        let dispatches = EXECUTIONS as u64 * self.packets;
        if self.executions.len() != EXECUTIONS || self.reads.len() != EXECUTIONS
            || self.planners.len() != EXECUTIONS || self.writes.len() != 630
            || end_frontier.checked_sub(self.start_frontier) != Some(dispatches)
            || general["dispatches"] != dispatches || general["reads"] != 127
            || general["read_bytes"] != 508 || general["writes"] != 630
            || token["executions"] != 127 || token["publications"] != 127
            || token["final_waits"] != 127 || token["dispatches"] != dispatches
            || token["retirement_signals"] != dispatches {
            return Err("incomplete or unexpected decode mechanism".into());
        }
        Ok(())
    }
}

impl Worker {
    pub(crate) fn enable_decode_diagnostic(&mut self) -> TpResult<()> {
        let state = self.token_program.as_mut().ok_or("token mode required")?;
        if !self.options.profile || !self.options.ordered64_runtime_counters
            || state.gate_up_selected.is_none() || state.prefill_rows != 32
            || state.completed_executions != 0 || state.completed_prefills != 0
            || state.counter_snapshots != 1
            || state.registered.is_some() || state.diagnostic.is_some() {
            return self.reject("decode diagnostic requires fresh profiled gate/up route");
        }
        state.diagnostic = Some(Box::default());
        Ok(())
    }

    pub(super) fn diagnostic_planner_start(&self) -> Option<Instant> {
        self.token_program.as_ref()?.diagnostic.as_ref().map(|_| Instant::now())
    }

    pub(super) fn diagnostic_planner_end(&mut self, start: Option<Instant>) -> TpResult<()> {
        if let Some(start) = start {
            let row = json!({"host_ns":ns(start)?});
            let d = self.token_program.as_mut().and_then(|s| s.diagnostic.as_mut()).ok_or("diagnostic state")?;
            if d.planners.len() >= EXECUTIONS || d.complete { return self.reject("extra decode planner"); }
            d.planners.push(row);
        }
        Ok(())
    }

    fn diagnostic_token_snapshot(&mut self) -> TpResult<Value> {
        let expected = self.token_program_backend().ok_or("token backend")?.identity();
        self.send(CommandV1::TokenProgramSnapshotV1 {}, vec![])?;
        let incoming = self.receive()?;
        let ResponseV1::TokenProgramSnapshotV1 { backend, counters } = incoming.header else {
            return self.reject("decode token snapshot response");
        };
        if backend != expected || !incoming.payload.is_empty() {
            return self.reject("decode token snapshot identity or payload");
        }
        serde_json::to_value(counters).map_err(|error| error.to_string())
    }

    fn diagnostic_endpoint(&mut self, before: bool) -> TpResult<Value> {
        let start = Instant::now();
        let (general, token) = if before {
            let token = self.diagnostic_token_snapshot()?;
            self.retain_diagnostic_endpoint(true, "token", token.clone())?;
            let general = self.runtime_diagnostic_snapshot()?;
            self.retain_diagnostic_endpoint(true, "general", general.clone())?;
            (general, token)
        } else {
            let general = self.runtime_diagnostic_snapshot()?;
            self.retain_diagnostic_endpoint(false, "general", general.clone())?;
            let token = self.diagnostic_token_snapshot()?;
            self.retain_diagnostic_endpoint(false, "token", token.clone())?;
            (general, token)
        };
        // Revalidate complete typed rosters before computing generic deltas.
        serde_json::from_value::<wire::PerformanceCountersV1>(general["counters"].clone())
            .map_err(|error| error.to_string())?;
        serde_json::from_value::<wire::TokenProgramCountersV1>(token.clone())
            .map_err(|error| error.to_string())?;
        Ok(json!({"general":general,"token":token,"snapshot_roundtrip_host_ns":ns(start)?}))
    }

    fn retain_diagnostic_endpoint(&mut self, before: bool, key: &str, value: Value) -> TpResult<()> {
        let d = self.token_program.as_mut().and_then(|s| s.diagnostic.as_mut()).ok_or("diagnostic state")?;
        let target = if before { &mut d.partial_before } else { &mut d.partial_after };
        target[key] = value;
        Ok(())
    }

    pub(super) fn diagnostic_execute_start(&mut self) -> TpResult<()> {
        let state = self.token_program.as_ref().ok_or("token state")?;
        let registered = state.registered.as_ref().ok_or("registered token")?;
        if registered.shape.is_prefill() || state.diagnostic.is_none() { return Ok(()); }
        let (program, epoch, packets) = (registered.program, registered.epoch, registered.shape.packets() as u64);
        let d = state.diagnostic.as_ref().ok_or("diagnostic state")?;
        if d.complete || d.execute_start.is_some() || d.executions.len() >= EXECUTIONS {
            return self.reject("extra or overlapping decode execution");
        }
        let fresh = d.before.is_none();
        if fresh {
            if state.completed_prefills != 4 || state.completed_executions != 0 || self.queue_packets != 2599 {
                return self.reject("fixed 128-token prefill boundary required");
            }
            let before = self.diagnostic_endpoint(true)?;
            if before["token"]["executions"] != 4 || before["token"]["dispatches"] != 2596 {
                return self.reject("prefill token counters differ");
            }
            let start_monotonic_ns = monotonic_ns()?;
            let d = self.token_program.as_mut().and_then(|s| s.diagnostic.as_mut()).ok_or("diagnostic state")?;
            d.program = program;
            d.epoch = epoch;
            d.packets = packets;
            d.start_frontier = self.queue_packets;
            d.before = Some(before);
            d.start_monotonic_ns = start_monotonic_ns;
            d.start = Some(Instant::now());
        }
        let d = self.token_program.as_mut().and_then(|s| s.diagnostic.as_mut()).ok_or("diagnostic state")?;
        if (d.program, d.epoch, d.packets) != (program, epoch, packets)
            || d.reads.len() != d.executions.len()
            || self.queue_packets != d.start_frontier + d.executions.len() as u64 * packets {
            return self.reject("decode program, epoch, readback or frontier changed");
        }
        d.execute_monotonic_ns = monotonic_ns()?;
        d.execute_start = Some(Instant::now());
        Ok(())
    }

    pub(super) fn diagnostic_execute_end(&mut self, program: u64, epoch: u64, frontier: u64) -> TpResult<()> {
        let Some(d) = self.token_program.as_mut().and_then(|s| s.diagnostic.as_mut()) else { return Ok(()); };
        if !d.active() { return Ok(()); }
        let start = d.execute_start.take().ok_or("decode execute ticket missing")?;
        if (program, epoch) != (d.program, d.epoch)
            || frontier != d.start_frontier + (d.executions.len() as u64 + 1) * d.packets {
            return self.reject("validated decode completion drift");
        }
        d.executions.push(json!({"ordinal":d.executions.len(),"frontier":frontier,
            "host_ns":ns(start)?,"started_monotonic_ns":d.execute_monotonic_ns,
            "finished_monotonic_ns":monotonic_ns()?}));
        Ok(())
    }

    pub(crate) fn diagnostic_io_start(&self) -> Option<Instant> {
        self.token_program.as_ref()?.diagnostic.as_ref().filter(|d| d.active()).map(|_| Instant::now())
    }

    pub(crate) fn diagnostic_io_end(&mut self, start: Option<Instant>, bytes: usize, read: bool) -> TpResult<()> {
        let Some(start) = start else { return Ok(()); };
        let row = json!({"host_ns":ns(start)?,"bytes":bytes,"finished_monotonic_ns":monotonic_ns()?});
        let d = self.token_program.as_mut().and_then(|s| s.diagnostic.as_mut()).ok_or("diagnostic state")?;
        if read {
            if bytes != 4 || d.execute_start.is_some() || d.reads.len() + 1 != d.executions.len() {
                return self.reject("decode readback order or size differs");
            }
            d.reads.push(row);
        } else {
            if d.writes.len() >= 630 || d.executions.len() != d.reads.len() || d.executions.len() >= EXECUTIONS {
                return self.reject("extra or unordered decode metadata write");
            }
            d.writes.push(row);
        }
        if read && d.reads.len() == EXECUTIONS { self.finish_decode_diagnostic()?; }
        Ok(())
    }

    fn finish_decode_diagnostic(&mut self) -> TpResult<()> {
        let d = self.token_program.as_ref().and_then(|s| s.diagnostic.as_ref()).ok_or("diagnostic state")?;
        let span_ns = ns(d.start.ok_or("diagnostic start")?)?;
        let end_monotonic_ns = monotonic_ns()?;
        let after = self.diagnostic_endpoint(false)?;
        let d = self.token_program.as_mut().and_then(|s| s.diagnostic.as_mut()).ok_or("diagnostic state")?;
        let before = d.before.as_ref().ok_or("diagnostic before endpoint")?;
        let general = delta(&before["general"]["counters"], &after["general"]["counters"], 19)?;
        let token = delta(&before["token"], &after["token"], 7)?;
        d.validate_totals(&general, &token, self.queue_packets)?;
        let execute_ns = sum(&d.executions, "host_ns")?;
        let read_ns = sum(&d.reads, "host_ns")?;
        let write_ns = sum(&d.writes, "host_ns")?;
        let warm_planner_ns = sum(&d.planners[1..], "host_ns")?;
        let worker_scope = [general["dispatch_prepare_ns"].as_u64(), general["dispatch_publish_ns"].as_u64(),
            general["dispatch_wait_ns"].as_u64(), token["staging_ns"].as_u64()]
            .into_iter().try_fold(0_i128, |n, v| Ok::<_, String>(n + i128::from(v.ok_or("missing counter")?)))?;
        let record = json!({"schema":"FerricNativeGateUpDecodeBreakdownR1","authority":"none",
            "performance_qualified":false,"latency_sample_admitted":false,"complete":true,
            "worker_pid":self.child.id(),"device_unique_id":self.diagnostic_identity.0,
            "program":d.program,"epoch":d.epoch,"packets_per_execute":d.packets,
            "start_frontier":d.start_frontier,"end_frontier":self.queue_packets,
            "started_monotonic_ns":d.start_monotonic_ns,"finished_monotonic_ns":end_monotonic_ns,
            "registered_decode_span_host_ns":span_ns,"before":before,"after":after,
            "general_counter_delta":general,"token_counter_delta":token,
            "planner_host_ns":d.planners,"executions":d.executions,"reads":d.reads,"writes":d.writes,
            "execute_host_ns":execute_ns,"readback_host_ns":read_ns,"metadata_write_host_ns":write_ns,
            "warm_transport_planner_host_ns":warm_planner_ns,
            "execute_minus_worker_scopes_host_ns":i128::from(execute_ns)-worker_scope,
            "span_residual_host_ns":i128::from(span_ns)-i128::from(execute_ns)-i128::from(read_ns)-i128::from(write_ns)-i128::from(warm_planner_ns),
            "scope":"Host-wall only; general counters overlap. Not GPU time or pure IPC. Pdelta includes the before-snapshot command, excludes the after-snapshot command.",
            "excludes":"First decode metadata uploads, cold transport plan and registration; setup, prefill, endpoint snapshot roundtrips and teardown.",
            "residual_scope":"Includes upstream graph construction, scheduler/JSON, instrumentation and inter-token gaps; signed, not clamped."});
        writeln!(std::io::stderr().lock(), "{record}").map_err(|error| error.to_string())?;
        d.complete = true;
        Ok(())
    }

    pub(crate) fn require_decode_diagnostic_complete(&self) -> TpResult<()> {
        if self.token_program.as_ref().and_then(|s| s.diagnostic.as_ref()).is_some_and(|d| !d.complete) {
            return Err("incomplete decode diagnostic; no accepted breakdown".into());
        }
        Ok(())
    }

    pub(crate) fn emit_incomplete_decode_diagnostic(&mut self, phase: &str) -> TpResult<()> {
        let Some(d) = self.token_program.as_mut().and_then(|s| s.diagnostic.as_mut()) else { return Ok(()); };
        if d.complete || d.incomplete_emitted { return Ok(()); }
        d.incomplete_emitted = true;
        let record = json!({"schema":"FerricNativeGateUpDecodeIncompleteR1","complete":false,
            "performance_qualified":false,"latency_sample_admitted":false,"phase":phase,
            "worker_pid":self.child.id(),"device_unique_id":self.diagnostic_identity.0,
            "program":d.program,"epoch":d.epoch,"frontier":self.queue_packets,"before":d.before,
            "partial_before":d.partial_before,"partial_after":d.partial_after,
            "planner_host_ns":d.planners,"executions":d.executions,"reads":d.reads,"writes":d.writes,
            "unfinished_execute":d.execute_start.is_some()});
        writeln!(std::io::stderr().lock(), "{record}").map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{Shape, TokenProgramBackend};

    fn run_wire_fixture(mode: &str, candidate: bool) -> (Worker, std::path::PathBuf, TpResult<()>) {
        let (mut worker, path) = super::super::tests::worker(mode);
        worker.options.profile = true;
        worker.options.ordered64_runtime_counters = true;
        let shape = if candidate { Shape::DecodeGateUp724 } else { Shape::Decode };
        let state = worker.token_program.as_mut().unwrap();
        state.backend = TokenProgramBackend::NativeWholeProgramSlots512V1;
        state.prefill_enabled = true;
        state.prefill_rows = 32;
        state.gate_up_selected = Some(candidate);
        state.decode_shape = shape;
        state.counter_snapshots = 1;
        worker.enable_decode_diagnostic().unwrap();
        worker.token_program.as_mut().unwrap().completed_prefills = 4;
        worker.queue_packets = 2599;
        worker.buffers.insert(100,4);
        let run = (|| {
            for ordinal in 0..127 {
                if ordinal != 0 {
                    for _ in 0..5 { worker.write(100,0,&[0;4])?; }
                }
                let start = worker.diagnostic_planner_start();
                let mut plan = super::super::tests::fixture();
                plan.definition.dispatches.resize(shape.packets(),plan.definition.dispatches[0].clone());
                plan.kernargs.resize(shape.packets()*4,0);
                let immutable = plan.immutable()?;
                worker.diagnostic_planner_end(start)?;
                worker.submit_shape(plan,immutable,shape)?;
                worker.wait_fixed_token(shape.packets())?;
                worker.read(100,0,&mut [0;4])?;
            }
            worker.require_decode_diagnostic_complete()?;
            worker.close()
        })();
        (worker,path,run)
    }

    #[test]
    fn successful_wire_path_has_exact_endpoints_and_frontiers_for_both_arms() {
        for candidate in [false,true] {
            let (worker,path,result) = run_wire_fixture("decode_diag_ok",candidate);
            result.unwrap();
            assert!(worker.exited);
            let d = worker.token_program.as_ref().unwrap().diagnostic.as_ref().unwrap();
            assert!(d.complete && !d.incomplete_emitted);
            assert_eq!(worker.queue_packets,if candidate {94547} else {85403});
            assert_eq!(worker.diagnostic_snapshots,2);
            assert_eq!(worker.token_program.as_ref().unwrap().counter_snapshots,2);
            let trace = std::fs::read_to_string(&path).unwrap();
            let ops:Vec<_> = trace.lines().collect();
            assert_eq!(&ops[..4],&["register_token_program","token_program_snapshot_v1","performance_snapshot","execute_token_program"]);
            assert_eq!(&ops[ops.len()-5..],&["performance_snapshot","token_program_snapshot_v1","release_token_program","token_program_snapshot_v1","close"]);
            assert_eq!(ops.iter().filter(|op| **op == "execute_token_program").count(),127);
            assert_eq!(ops.iter().filter(|op| **op == "read").count(),127);
            assert_eq!(ops.iter().filter(|op| **op == "write").count(),630);
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn failed_last_read_or_snapshot_retains_incomplete_acquired_endpoints() {
        for mode in ["decode_diag_last_read_bad","decode_diag_last_token_bad","decode_diag_first_general_bad"] {
            let (worker,path,result) = run_wire_fixture(mode,false);
            assert!(result.is_err() && worker.exited);
            let d = worker.token_program.as_ref().unwrap().diagnostic.as_ref().unwrap();
            assert!(!d.complete && d.incomplete_emitted);
            assert!(d.partial_before["token"].is_object());
            if mode == "decode_diag_last_token_bad" { assert!(d.partial_after["general"].is_object()); }
            if mode == "decode_diag_first_general_bad" { assert_eq!(d.executions.len(),0); }
            else { assert_eq!(d.executions.len(),127); }
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn checked_delta_rejects_missing_extra_regressed_and_noninteger_fields() {
        let before = json!({"a":2,"b":3});
        assert_eq!(delta(&before,&json!({"a":4,"b":8}),2).unwrap(),json!({"a":2,"b":5}));
        for wrong in [json!({"a":3}),json!({"a":3,"b":4,"c":0}),json!({"a":1,"b":4}),json!({"a":"3","b":4})] {
            assert!(delta(&before,&wrong,2).is_err());
        }
    }

    #[test]
    fn sums_reject_overflow_and_missing_fields() {
        assert_eq!(sum(&[json!({"n":2}),json!({"n":3})],"n").unwrap(),5);
        assert!(sum(&[json!({"n":u64::MAX}),json!({"n":1})],"n").is_err());
        assert!(sum(&[json!({})],"n").is_err());
    }

    #[test]
    fn monotonic_clock_is_nonzero_and_ordered() {
        let first = monotonic_ns().unwrap();
        assert!(first > 0 && monotonic_ns().unwrap() >= first);
    }

    #[test]
    fn typed_snapshots_require_complete_exact_integer_rosters() {
        let general = serde_json::to_value(wire::PerformanceCountersV1::default()).unwrap();
        let token = serde_json::to_value(wire::TokenProgramCountersV1::default()).unwrap();
        for (original, field, general_type) in [(general,"commands",true),(token,"executions",false)] {
            for mode in 0..3 {
                let mut wrong = original.clone();
                match mode {
                    0 => { wrong.as_object_mut().unwrap().remove(field); }
                    1 => { wrong["unexpected"] = json!(0); }
                    _ => { wrong[field] = json!("0"); }
                }
                if general_type {
                    assert!(serde_json::from_value::<wire::PerformanceCountersV1>(wrong).is_err());
                } else {
                    assert!(serde_json::from_value::<wire::TokenProgramCountersV1>(wrong).is_err());
                }
            }
        }
    }

    #[test]
    fn exact_decode_counts_for_both_arms_reject_incomplete_and_extra_work() {
        for packets in [652,724] {
            let mut d = Recorder { packets,start_frontier:2599,
                executions:vec![Value::Null;127],reads:vec![Value::Null;127],
                planners:vec![Value::Null;127],writes:vec![Value::Null;630],..Recorder::default() };
            let dispatches = packets*127;
            let general = json!({"dispatches":dispatches,"reads":127,"read_bytes":508,"writes":630});
            let token = json!({"executions":127,"publications":127,"final_waits":127,
                "dispatches":dispatches,"retirement_signals":dispatches});
            let frontier = 2599+dispatches;
            assert!(d.validate_totals(&general,&token,frontier).is_ok());
            assert!(d.validate_totals(&general,&token,frontier+1).is_err());
            for key in ["executions","publications","final_waits","dispatches","retirement_signals"] {
                let mut wrong = token.clone();
                wrong[key] = json!(wrong[key].as_u64().unwrap()+1);
                assert!(d.validate_totals(&general,&wrong,frontier).is_err());
            }
            for key in ["dispatches","reads","read_bytes","writes"] {
                let mut wrong = general.clone();
                wrong[key] = json!(wrong[key].as_u64().unwrap()+1);
                assert!(d.validate_totals(&wrong,&token,frontier).is_err());
            }
            d.executions.pop();
            assert!(d.validate_totals(&general,&token,frontier).is_err());
            d.executions.extend([Value::Null,Value::Null]);
            assert!(d.validate_totals(&general,&token,frontier).is_err());
        }
    }
}
