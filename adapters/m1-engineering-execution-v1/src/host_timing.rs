//! Opt-in, bounded controller wall-clock diagnostics. No device clock is sampled.
//!
//! Nested scopes and concurrent rank round trips overlap and must not be added
//! together as elapsed workload time. Tickets describe parent transport receipt,
//! not successful execution or validation of the returned GPU result.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

use serde::Serialize;

/// Maximum distinct aggregate keys, independent of the number of dispatches.
pub const MAX_RECORDS: usize = 65_536;

#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Context {
    batch: Option<u64>,
    phase: &'static str,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Key {
    #[serde(flatten)]
    context: Context,
    category: &'static str,
    label: &'static str,
    rank: Option<u32>,
}

#[derive(Default, Serialize)]
struct Totals {
    count: u64,
    failed: u64,
    elapsed_ns: u64,
    max_ns: u64,
    request_payload_bytes: u64,
    response_payload_bytes: u64,
    dispatches: u64,
}

struct State {
    context: Context,
    records: BTreeMap<Key, Totals>,
    incomplete: bool,
    active: u64,
    limit: usize,
    ordered64_diagnostic: bool,
}

/// Explicit shared controller-thread recorder. Default is allocation/clock free.
#[derive(Clone, Default)]
pub struct HostTiming(Option<Rc<RefCell<State>>>);

impl HostTiming {
    /// Enables bounded aggregation for one controller invocation.
    #[must_use]
    pub fn enabled() -> Self {
        Self::with_ordered64_diagnostic(false)
    }

    /// Enables a separate bounded host/worker diagnostic, never GPU timing.
    #[must_use]
    pub fn ordered64_diagnostic() -> Self {
        Self::with_ordered64_diagnostic(true)
    }

    fn with_ordered64_diagnostic(ordered64_diagnostic: bool) -> Self {
        Self(Some(Rc::new(RefCell::new(State {
            context: Context {
                batch: None,
                phase: "controller",
            },
            records: BTreeMap::new(),
            incomplete: false,
            active: 0,
            limit: MAX_RECORDS,
            ordered64_diagnostic,
        }))))
    }

    /// Whether instrumentation was explicitly selected.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.0.is_some()
    }

    /// Whether the separate ordered64 diagnostic selected worker duration capture.
    #[must_use]
    pub fn is_ordered64_diagnostic(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|state| state.borrow().ordered64_diagnostic)
    }

    /// Times controller-side argument packing only for the explicit diagnostic.
    #[must_use]
    pub fn ordered_packing_span(&self, rank: u32) -> Option<HostSpan> {
        self.is_ordered64_diagnostic()
            .then(|| self.span("ordered_dispatch_pack", Some(rank)))
    }

    /// Retains a validated ordered64 reply's worker wall duration without a clock read.
    /// It excludes worker preparation/staging and includes publication/wait/fences.
    pub fn ordered_worker_elapsed(&self, rank: u32, dispatches: u64, elapsed_ns: u64) {
        let Some(state) = &self.0 else {
            return;
        };
        let mut state = state.borrow_mut();
        if !state.ordered64_diagnostic {
            return;
        }
        if rank != 0 || !(1..=64).contains(&dispatches) {
            state.incomplete = true;
            return;
        }
        let key = Key {
            context: state.context.clone(),
            category: "worker_reported_elapsed",
            label: "dispatch_ordered_batch64",
            rank: Some(rank),
        };
        state.record(key, elapsed_ns, false, 0, 0, dispatches);
    }

    /// Starts a nested host scope and attributes enclosed transport to this phase.
    #[must_use]
    pub fn scope(&self, phase: &'static str) -> HostSpan {
        let restore = self.0.as_ref().map(|state| {
            let mut state = state.borrow_mut();
            let restore = state.context.clone();
            state.context.phase = phase;
            restore
        });
        HostSpan {
            token: self.start("span", phase, None),
            restore,
        }
    }

    /// Starts a physical pool batch scope. The supplied ID is not a new authority.
    #[must_use]
    pub fn batch(&self, id: u64) -> HostSpan {
        let mut span = self.scope("batch");
        if let Some(state) = &self.0 {
            state.borrow_mut().context.batch = Some(id);
        }
        if let Some(token) = &mut span.token {
            token.key.context.batch = Some(id);
        }
        span
    }

    /// Measures an overlapping host operation without changing phase attribution.
    #[must_use]
    pub fn span(&self, label: &'static str, rank: Option<u32>) -> HostSpan {
        HostSpan {
            token: self.start("span", label, rank),
            restore: None,
        }
    }

    /// Starts a parent request ticket; payload counts exclude JSON framing bytes.
    #[must_use]
    pub fn request(
        &self,
        rank: Option<u32>,
        command: &'static str,
        payload_bytes: usize,
        dispatches: u64,
    ) -> IpcTiming {
        let token = self.start("ipc_roundtrip", command, rank);
        IpcTiming {
            token,
            sent: false,
            payload_bytes,
            dispatches,
        }
    }

    fn start(
        &self,
        category: &'static str,
        label: &'static str,
        rank: Option<u32>,
    ) -> Option<Token> {
        let state = self.0.as_ref()?;
        let mut current = state.borrow_mut();
        current.active += 1;
        Some(Token {
            state: Rc::clone(state),
            started: Instant::now(),
            key: Key {
                context: current.context.clone(),
                category,
                label,
                rank,
            },
        })
    }

    /// Produces a bounded sidecar body. Active, overflowed or truncated data fail qualification.
    #[must_use]
    pub fn snapshot(&self) -> serde_json::Value {
        let Some(state) = &self.0 else {
            return serde_json::Value::Null;
        };
        let state = state.borrow();
        let records = state
            .records
            .iter()
            .map(|(key, totals)| {
                #[derive(Serialize)]
                struct Record<'a> {
                    #[serde(flatten)]
                    key: &'a Key,
                    #[serde(flatten)]
                    totals: &'a Totals,
                }
                Record { key, totals }
            })
            .collect::<Vec<_>>();
        let mut snapshot = serde_json::json!({"schema":"FerricHostTimingV1", "clock":"controller-std-instant",
            "measurement":"host-wall-latency-not-gpu-duration", "aggregation":"overlapping-not-additive",
            "payload_accounting":"payload-only-excludes-wire-headers", "record_limit":MAX_RECORDS,
            "incomplete":state.incomplete || state.active != 0, "active_records":state.active,
            "records":records});
        if state.ordered64_diagnostic {
            snapshot["schema"] = serde_json::json!("FerricOrdered64HostTimingV1");
            snapshot["clock"] = serde_json::json!("controller-and-worker-std-instant-durations");
            snapshot["worker_reported_scope"] = serde_json::json!(
                "publication through completion checks; excludes preparation/staging; includes GPU work, polling and fences; not GPU duration"
            );
            snapshot["scope_relationship"] = serde_json::json!(
                "packing is inside flush; send and worker elapsed are inside roundtrip; response wait overlaps worker elapsed; never sum nested scopes"
            );
        }
        snapshot
    }
}

struct Token {
    state: Rc<RefCell<State>>,
    started: Instant,
    key: Key,
}

impl Token {
    fn record(
        &self,
        category: &'static str,
        failed: bool,
        request: usize,
        response: usize,
        dispatches: u64,
    ) {
        let mut state = self.state.borrow_mut();
        let mut key = self.key.clone();
        key.category = category;
        let Ok(elapsed) = u64::try_from(self.started.elapsed().as_nanos()) else {
            state.incomplete = true;
            return;
        };
        state.record(key, elapsed, failed, request, response, dispatches);
    }
}

impl State {
    fn record(
        &mut self,
        key: Key,
        elapsed: u64,
        failed: bool,
        request: usize,
        response: usize,
        dispatches: u64,
    ) {
        if key.label.len() > 96
            || key.context.phase.len() > 96
            || key.rank.is_some_and(|rank| rank >= 8)
            || (!self.records.contains_key(&key) && self.records.len() >= self.limit)
        {
            self.incomplete = true;
            return;
        }
        let totals = self.records.entry(key).or_default();
        let updated = (|| {
            Some(Totals {
                count: totals.count.checked_add(1)?,
                failed: totals.failed.checked_add(u64::from(failed))?,
                elapsed_ns: totals.elapsed_ns.checked_add(elapsed)?,
                max_ns: totals.max_ns.max(elapsed),
                request_payload_bytes: totals
                    .request_payload_bytes
                    .checked_add(u64::try_from(request).ok()?)?,
                response_payload_bytes: totals
                    .response_payload_bytes
                    .checked_add(u64::try_from(response).ok()?)?,
                dispatches: totals.dispatches.checked_add(dispatches)?,
            })
        })();
        if let Some(updated) = updated {
            *totals = updated;
        } else {
            self.incomplete = true;
        }
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        self.state.borrow_mut().active -= 1;
    }
}

/// RAII wall span. Failed runs are identified by the sidecar's final status.
pub struct HostSpan {
    token: Option<Token>,
    restore: Option<Context>,
}

impl Drop for HostSpan {
    fn drop(&mut self) {
        if let Some(token) = &self.token {
            token.record("span", std::thread::panicking(), 0, 0, 0);
            if let Some(context) = self.restore.take() {
                token.state.borrow_mut().context = context;
            }
        }
    }
}

/// Diagnostic transport lifetime; dropping before a terminal receipt marks failure.
pub struct IpcTiming {
    token: Option<Token>,
    sent: bool,
    payload_bytes: usize,
    dispatches: u64,
}

impl IpcTiming {
    /// Records controller enqueue through writer acknowledgement, including blocking.
    pub fn sent(&mut self, ok: bool) {
        if let Some(token) = &self.token {
            if self.sent {
                token.state.borrow_mut().incomplete = true;
                return;
            }
            token.record("ipc_send", !ok, self.payload_bytes, 0, 0);
        }
        self.sent = true;
    }

    /// Records parent receipt. `ok` means transport success, not model correctness.
    pub fn finish(mut self, response_payload_bytes: usize, ok: bool) {
        if let Some(token) = self.token.take() {
            token.record(
                "ipc_roundtrip",
                !ok,
                self.payload_bytes,
                response_payload_bytes,
                self.dispatches,
            );
        }
    }
}

impl Drop for IpcTiming {
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            token.record(
                "ipc_roundtrip",
                true,
                self.payload_bytes,
                0,
                self.dispatches,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_has_no_state_or_tickets() {
        let timing = HostTiming::default();
        assert!(!timing.is_enabled());
        assert!(timing.span("ignored", None).token.is_none());
        assert!(timing.request(Some(0), "dispatch", 5, 1).token.is_none());
        assert!(timing.snapshot().is_null());
    }

    #[test]
    fn tickets_keep_submission_context_and_fail_when_abandoned() {
        let timing = HostTiming::enabled();
        let batch = timing.batch(9);
        let phase = timing.scope("attention");
        let mut ticket = timing.request(Some(1), "dispatch", 10, 1);
        ticket.sent(true);
        drop(phase);
        ticket.finish(20, true);
        drop(timing.request(Some(0), "write", 5, 0));
        drop(batch);
        let snapshot = timing.snapshot();
        assert_eq!(snapshot["incomplete"], false);
        let records = snapshot["records"].as_array().unwrap();
        let request = records
            .iter()
            .find(|r| r["category"] == "ipc_roundtrip" && r["label"] == "dispatch")
            .unwrap();
        assert_eq!(request["phase"], "attention");
        assert_eq!(request["batch"], 9);
        assert_eq!(request["request_payload_bytes"], 10);
        assert_eq!(request["response_payload_bytes"], 20);
        assert_eq!(request["dispatches"], 1);
        let abandoned = records
            .iter()
            .find(|r| r["category"] == "ipc_roundtrip" && r["label"] == "write")
            .unwrap();
        assert_eq!(abandoned["failed"], 1);
    }

    #[test]
    fn bounds_and_live_snapshots_fail_closed() {
        let timing = HostTiming::enabled();
        timing.0.as_ref().unwrap().borrow_mut().limit = 1;
        let span = timing.scope("first");
        assert_eq!(timing.snapshot()["incomplete"], true);
        drop(span);
        drop(timing.scope("second"));
        let snapshot = timing.snapshot();
        assert_eq!(snapshot["records"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot["incomplete"], true);
    }

    #[test]
    fn aggregate_overflow_marks_snapshot_incomplete_without_wrapping() {
        let timing = HostTiming::enabled();
        drop(timing.span("overflow", None));
        timing
            .0
            .as_ref()
            .unwrap()
            .borrow_mut()
            .records
            .values_mut()
            .next()
            .unwrap()
            .count = u64::MAX;
        drop(timing.span("overflow", None));
        let snapshot = timing.snapshot();
        assert_eq!(snapshot["incomplete"], true);
        assert_eq!(snapshot["records"][0]["count"], u64::MAX);
    }

    #[test]
    fn ordered_durations_are_absent_when_disabled_or_legacy_timing_is_selected() {
        for timing in [HostTiming::default(), HostTiming::enabled()] {
            assert!(!timing.is_ordered64_diagnostic());
            assert!(timing.ordered_packing_span(0).is_none());
            timing.ordered_worker_elapsed(0, 64, 123);
            let value = timing.snapshot();
            if !value.is_null() {
                assert_eq!(value["schema"], "FerricHostTimingV1");
                assert_eq!(value["records"].as_array().unwrap().len(), 0);
                assert!(value.get("worker_reported_scope").is_none());
            }
        }
    }

    #[test]
    fn ordered_worker_durations_preserve_context_exact_sum_max_and_packet_count() {
        let timing = HostTiming::ordered64_diagnostic();
        let batch = timing.batch(5);
        let phase = timing.scope("attention");
        timing.ordered_worker_elapsed(0, 64, 1_234);
        timing.ordered_worker_elapsed(0, 12, 567);
        drop(phase);
        drop(batch);
        let value = timing.snapshot();
        assert_eq!(value["schema"], "FerricOrdered64HostTimingV1");
        assert_eq!(value["incomplete"], false);
        let row = value["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["category"] == "worker_reported_elapsed")
            .unwrap();
        assert_eq!(row["batch"], 5);
        assert_eq!(row["phase"], "attention");
        assert_eq!(row["rank"], 0);
        assert_eq!(row["count"], 2);
        assert_eq!(row["elapsed_ns"], 1_801);
        assert_eq!(row["max_ns"], 1_234);
        assert_eq!(row["dispatches"], 76);
        assert_eq!(row["request_payload_bytes"], 0);
        assert_eq!(row["response_payload_bytes"], 0);
        assert!(
            value["worker_reported_scope"]
                .as_str()
                .unwrap()
                .contains("not GPU duration")
        );
    }

    #[test]
    fn ordered_worker_duration_bounds_and_overflow_fail_closed() {
        for (rank, count) in [(1, 1), (0, 0), (0, 65)] {
            let timing = HostTiming::ordered64_diagnostic();
            timing.ordered_worker_elapsed(rank, count, 1);
            assert_eq!(timing.snapshot()["incomplete"], true);
            assert_eq!(timing.snapshot()["records"].as_array().unwrap().len(), 0);
        }
        let timing = HostTiming::ordered64_diagnostic();
        timing.ordered_worker_elapsed(0, 1, u64::MAX);
        timing.ordered_worker_elapsed(0, 1, 1);
        let value = timing.snapshot();
        assert_eq!(value["incomplete"], true);
        assert_eq!(value["records"][0]["elapsed_ns"], u64::MAX);
        assert_eq!(value["records"][0]["count"], 1);
        let bounded = HostTiming::ordered64_diagnostic();
        bounded.0.as_ref().unwrap().borrow_mut().limit = 0;
        bounded.ordered_worker_elapsed(0, 1, 1);
        assert_eq!(bounded.snapshot()["incomplete"], true);
        assert_eq!(bounded.snapshot()["records"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn ordered_packing_span_uses_the_existing_bounded_host_recorder() {
        let timing = HostTiming::ordered64_diagnostic();
        let span = timing.ordered_packing_span(0).unwrap();
        assert_eq!(timing.snapshot()["incomplete"], true);
        drop(span);
        let value = timing.snapshot();
        assert_eq!(value["incomplete"], false);
        assert_eq!(value["records"][0]["label"], "ordered_dispatch_pack");
        assert_eq!(value["records"][0]["category"], "span");
        assert_eq!(value["records"][0]["dispatches"], 0);
    }
}
