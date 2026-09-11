//! Bounded, local JSONL ingress for one already-resident engineering TP session.
//! No network listener, execution authority, or serving qualification is implied.

use std::collections::VecDeque;
use std::os::fd::AsFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::host_timing::HostTiming;
use crate::tp_batch_runtime::{EngineeringTpBatchRunnerV2, EngineeringTpBatchRuntimeV2};
use crate::tp_model::EngineeringQwenModelV1;
use crate::tp_scheduler::{
    TP_MAX_REQUESTS_V1, TpRequestAdmissionV1, TpRequestIdV1, TpRequestRecordV1, TpRequestStateV1,
};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use serde::Deserialize;
use serde_json::{Value, json};

const COMMAND_SCHEMA: &str = "FerricQwen3TpLiveCommandV1";
const EVENT_SCHEMA: &str = "FerricQwen3TpLiveEventV1";
const MAX_LINE_BYTES: usize = 524_288;
const MAX_PROMPT_BYTES: usize = ferric_build::MAX_TOKENIZER_INPUT_BYTES;
const MAX_PENDING: usize = 32;
const INPUT_CAPACITY: usize = 8;
const INPUTS_PER_BATCH: usize = 32;
const POLL_TIMEOUT: Timespec = Timespec {
    tv_sec: 0,
    tv_nsec: 100_000_000,
};

fn elapsed_ns(clock: Instant) -> Result<u64, String> {
    u64::try_from(clock.elapsed().as_nanos()).map_err(|_| "live elapsed clock overflow".into())
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Submit {
        schema: String,
        request_id: u64,
        name: String,
        prompt: String,
        new_tokens: u32,
    },
    Cancel {
        schema: String,
        request_id: u64,
    },
    Drain {
        schema: String,
    },
    Shutdown {
        schema: String,
    },
}

impl Command {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_LINE_BYTES {
            return Err("command must contain 1..=524288 JSON bytes".into());
        }
        let command: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let schema = match &command {
            Self::Submit { schema, .. }
            | Self::Cancel { schema, .. }
            | Self::Drain { schema }
            | Self::Shutdown { schema } => schema,
        };
        if schema != COMMAND_SCHEMA {
            return Err("wrong live command schema".into());
        }
        match &command {
            Self::Submit {
                request_id,
                name,
                prompt,
                new_tokens,
                ..
            } if *request_id == 0
                || name.is_empty()
                || name.len() > 64
                || prompt.is_empty()
                || prompt.len() > MAX_PROMPT_BYTES
                || !(1..=8192).contains(new_tokens) =>
            {
                return Err("invalid request identity, prompt, or output bound".into());
            }
            Self::Cancel { request_id: 0, .. } => {
                return Err("cancel request_id must be positive".into());
            }
            _ => {}
        }
        Ok(command)
    }
}

#[derive(Debug)]
enum Input {
    Command {
        arrival_ns: u64,
        command: Result<Command, String>,
    },
    Eof,
    Failed(String),
}

#[derive(Default)]
struct Framer {
    bytes: Vec<u8>,
    oversized: bool,
}

impl Framer {
    fn push(&mut self, byte: u8) -> Option<Result<Command, String>> {
        if byte == b'\n' {
            return Some(self.finish());
        }
        if !self.oversized {
            if self.bytes.len() == MAX_LINE_BYTES {
                self.bytes.clear();
                self.oversized = true;
            } else {
                self.bytes.push(byte);
            }
        }
        None
    }

    fn finish(&mut self) -> Result<Command, String> {
        let result = if self.oversized {
            Err("command exceeds 524288 JSON bytes".into())
        } else {
            Command::parse(&self.bytes)
        };
        self.bytes.clear();
        self.oversized = false;
        result
    }

    fn has_partial(&self) -> bool {
        self.oversized || !self.bytes.is_empty()
    }
}

struct LiveInput {
    receiver: Receiver<Input>,
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
}

impl LiveInput {
    fn start(input: impl AsFd + Send + 'static, clock: Instant) -> Result<Self, String> {
        let (sender, receiver) = mpsc::sync_channel(INPUT_CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let reader_stop = Arc::clone(&stop);
        let reader = thread::Builder::new()
            .name("ferric-live-stdin".into())
            .spawn(move || {
                if let Err(error) = read_input(input, &sender, &reader_stop, clock) {
                    send_input(&sender, &reader_stop, Input::Failed(error));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            receiver,
            stop,
            reader: Some(reader),
        })
    }

    fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::Release);
        self.reader.take().map_or(Ok(()), |reader| {
            reader
                .join()
                .map_err(|_| "live stdin reader panicked".into())
        })
    }
}

impl Drop for LiveInput {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn send_input(sender: &SyncSender<Input>, stop: &AtomicBool, mut input: Input) -> bool {
    loop {
        if stop.load(Ordering::Acquire) {
            return false;
        }
        match sender.try_send(input) {
            Ok(()) => return true,
            Err(TrySendError::Disconnected(_)) => return false,
            Err(TrySendError::Full(value)) => {
                input = value;
                thread::sleep(Duration::from_millis(5));
            }
        }
    }
}

fn read_input(
    input: impl AsFd,
    sender: &SyncSender<Input>,
    stop: &AtomicBool,
    clock: Instant,
) -> Result<(), String> {
    let mut framer = Framer::default();
    let mut bytes = [0; 4096];
    while !stop.load(Ordering::Acquire) {
        let mut fds = [PollFd::new(&input, PollFlags::IN)];
        match poll(&mut fds, Some(&POLL_TIMEOUT)) {
            Ok(0) | Err(rustix::io::Errno::INTR) => continue,
            Ok(_) => {}
            Err(error) => return Err(format!("stdin poll: {error}")),
        }
        // This thread exclusively reads the descriptor. Polling bounds shutdown
        // without changing flags on the caller's shared stdin description.
        let count = match rustix::io::read(&input, &mut bytes) {
            Ok(count) => count,
            Err(rustix::io::Errno::INTR | rustix::io::Errno::AGAIN) => continue,
            Err(error) => return Err(format!("stdin read: {error}")),
        };
        let arrival_ns = elapsed_ns(clock)?;
        if count == 0 {
            if framer.has_partial()
                && !send_input(
                    sender,
                    stop,
                    Input::Command {
                        arrival_ns,
                        command: framer.finish(),
                    },
                )
            {
                return Ok(());
            }
            send_input(sender, stop, Input::Eof);
            return Ok(());
        }
        for &byte in &bytes[..count] {
            if let Some(command) = framer.push(byte)
                && !send_input(
                    sender,
                    stop,
                    Input::Command {
                        arrival_ns,
                        command,
                    },
                )
            {
                return Ok(());
            }
        }
    }
    Ok(())
}

trait TokenCodec {
    fn encode(&self, prompt: &str) -> Result<Vec<u32>, String>;
    fn decode(&self, tokens: &[u32]) -> Result<Vec<u8>, String>;
}

impl TokenCodec for EngineeringQwenModelV1 {
    fn encode(&self, prompt: &str) -> Result<Vec<u32>, String> {
        self.encode(prompt)
    }
    fn decode(&self, tokens: &[u32]) -> Result<Vec<u8>, String> {
        self.decode(tokens)
    }
}

struct Pending {
    request_id: u64,
    name: String,
    tokens: Vec<u32>,
    new_tokens: u32,
    arrival_ns: u64,
    pages: u32,
}

struct Active {
    request_id: u64,
    name: String,
    id: TpRequestIdV1,
    pages: u32,
}

struct Session<'a, G: EngineeringTpBatchRunnerV2, M, E> {
    runtime: &'a mut EngineeringTpBatchRuntimeV2<G>,
    codec: &'a M,
    emit: E,
    pending: VecDeque<Pending>,
    active: Vec<Active>,
    context: u32,
    pages: u32,
    last_request_id: u64,
    tick: u64,
    batches: u64,
    draining: bool,
}

impl<G: EngineeringTpBatchRunnerV2, M: TokenCodec, E: FnMut(Value) -> Result<(), String>>
    Session<'_, G, M, E>
{
    fn event(&mut self, event: &str, mut fields: Value) -> Result<(), String> {
        fields["schema"] = json!(EVENT_SCHEMA);
        fields["authority"] = json!("none");
        fields["event"] = json!(event);
        (self.emit)(fields)
    }

    fn input(&mut self, input: Input, now_ns: u64) -> Result<(), String> {
        match input {
            Input::Command {
                arrival_ns,
                command: Ok(command),
            } => self.command(command, arrival_ns, now_ns),
            Input::Command {
                arrival_ns,
                command: Err(error),
            } => self.event(
                "rejected",
                json!({"request_id":null,"arrival_ns":arrival_ns,
                    "reason":"invalid_command","detail":error}),
            ),
            Input::Eof => self.drain("eof"),
            Input::Failed(error) => Err(error),
        }
    }

    fn command(&mut self, command: Command, arrival_ns: u64, now_ns: u64) -> Result<(), String> {
        match command {
            Command::Submit {
                request_id,
                name,
                prompt,
                new_tokens,
                ..
            } => {
                let reason = if request_id <= self.last_request_id {
                    Some("request_id_not_increasing")
                } else {
                    self.last_request_id = request_id;
                    if self.draining {
                        Some("draining")
                    } else if self.pending.len() == MAX_PENDING {
                        Some("overloaded")
                    } else {
                        None
                    }
                };
                if let Some(reason) = reason {
                    return self.event(
                        "rejected",
                        json!({"request_id":request_id,
                        "name":name,"arrival_ns":arrival_ns,"reason":reason}),
                    );
                }
                let tokens = match self.codec.encode(&prompt) {
                    Ok(tokens) => tokens,
                    Err(error) => return self.event("rejected", json!({"request_id":request_id,
                        "name":name,"arrival_ns":arrival_ns,"reason":"tokenization","detail":error})),
                };
                let positions = u32::try_from(tokens.len())
                    .ok()
                    .and_then(|length| length.checked_add(new_tokens))
                    .and_then(|length| length.checked_sub(1));
                if tokens.is_empty() || positions.is_none_or(|n| n > self.context) {
                    return self.event(
                        "rejected",
                        json!({"request_id":request_id,
                        "name":name,"arrival_ns":arrival_ns,"reason":"context_limit"}),
                    );
                }
                let pages = positions
                    .ok_or("missing request position bound")?
                    .div_ceil(16);
                if pages > self.pages {
                    return self.event(
                        "rejected",
                        json!({"request_id":request_id,
                        "name":name,"arrival_ns":arrival_ns,"reason":"page_capacity"}),
                    );
                }
                self.event(
                    "queued",
                    json!({"request_id":request_id,"name":name,
                    "arrival_ns":arrival_ns,"queued_ns":now_ns,"prompt_tokens":tokens.len()}),
                )?;
                self.pending.push_back(Pending {
                    request_id,
                    name,
                    tokens,
                    new_tokens,
                    arrival_ns,
                    pages,
                });
                Ok(())
            }
            Command::Cancel { request_id, .. } => self.cancel(request_id, now_ns),
            Command::Drain { .. } => self.drain("command"),
            Command::Shutdown { .. } => {
                self.drain("shutdown")?;
                self.cancel_all(now_ns, "shutdown")
            }
        }
    }

    fn drain(&mut self, reason: &str) -> Result<(), String> {
        if !self.draining {
            self.draining = true;
            self.event("draining", json!({"reason":reason}))?;
        }
        Ok(())
    }

    fn cancel_pending(&mut self, index: usize, now_ns: u64, reason: &str) -> Result<(), String> {
        let request = self
            .pending
            .remove(index)
            .ok_or("pending request disappeared")?;
        self.event(
            "request",
            json!({"request_id":request.request_id,"name":request.name,
            "state":"Cancelled","reason":reason,"arrival_ns":request.arrival_ns,
            "cancelled_ns":now_ns,"admitted":false,"generated_tokens":[],
            "output_timestamps_ns":[],"ttft_ns":null,"tpot_ns":null}),
        )
    }

    fn cancel(&mut self, request_id: u64, now_ns: u64) -> Result<(), String> {
        if let Some(index) = self.pending.iter().position(|r| r.request_id == request_id) {
            self.cancel_pending(index, now_ns, "cancel")?;
            return self.event(
                "cancel",
                json!({"request_id":request_id,"status":"cancelled_pending"}),
            );
        }
        if let Some(active) = self.active.iter().find(|r| r.request_id == request_id) {
            self.runtime.cancel(active.id, now_ns)?;
            self.retire()?;
            return self.event(
                "cancel",
                json!({"request_id":request_id,"status":"cancelled_active"}),
            );
        }
        self.event(
            "cancel",
            json!({"request_id":request_id,"status":"not_found"}),
        )
    }

    fn cancel_all(&mut self, now_ns: u64, reason: &str) -> Result<(), String> {
        while !self.pending.is_empty() {
            self.cancel_pending(0, now_ns, reason)?;
        }
        for active in &self.active {
            self.runtime.cancel(active.id, now_ns)?;
        }
        self.retire()
    }

    fn admit(&mut self, now_ns: u64) -> Result<(), String> {
        let mut reserved = self.active.iter().map(|r| r.pages).sum::<u32>();
        while self.active.len() < TP_MAX_REQUESTS_V1 {
            let Some(request) = self.pending.front() else {
                break;
            };
            // Reserve each request's worst-case private KV footprint. Cache hits
            // may save work, but never justify overcommitting live page capacity.
            if reserved + request.pages > self.pages {
                break;
            }
            let hit = self.runtime.admit(
                TpRequestAdmissionV1 {
                    prompt_tokens: request.tokens.clone(),
                    max_new_tokens: request.new_tokens,
                    cached_prefix_tokens: 0,
                    arrival_tick: self.tick,
                    arrival_ns: request.arrival_ns,
                },
                self.tick,
                now_ns,
            )?;
            let request = self
                .pending
                .pop_front()
                .ok_or("pending admission disappeared")?;
            reserved += request.pages;
            self.event(
                "admission",
                json!({"request_id":request.request_id,"name":request.name,
                "slot":hit.request.slot,"generation":hit.request.generation,
                "arrival_ns":request.arrival_ns,"admitted_ns":now_ns,
                "queue_wait_ns":now_ns - request.arrival_ns,"prompt_tokens":request.tokens,
                "prompt_token_count":request.tokens.len(),
                "cached_tokens":hit.cached_tokens,"cached_pages":hit.cached_pages}),
            )?;
            self.active.push(Active {
                request_id: request.request_id,
                name: request.name,
                id: hit.request,
                pages: request.pages,
            });
        }
        Ok(())
    }

    fn retired_event(&mut self, active: &Active, record: &TpRequestRecordV1) -> Result<(), String> {
        let decoded = self.codec.decode(&record.generated_tokens)?;
        let intervals = record
            .output_timestamps_ns
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .collect::<Vec<_>>();
        let tpot = if intervals.is_empty() {
            None
        } else {
            Some(
                intervals.iter().sum::<u64>()
                    / u64::try_from(intervals.len()).map_err(|_| "interval count")?,
            )
        };
        self.event(
            "request",
            json!({"request_id":active.request_id,"name":active.name,
            "slot":record.id.slot,"generation":record.id.generation,"admitted":true,
            "state":format!("{:?}", record.state()),"prompt_tokens":record.prompt_tokens,
            "prompt_token_count":record.prompt_tokens.len(),
            "generated_tokens":record.generated_tokens,"generated_utf8_bytes":decoded,
            "generated_text":String::from_utf8(decoded.clone()).ok(),
            "cached_prefix_tokens":record.cached_prefix_tokens,"arrival_ns":record.arrival_ns,
            "output_timestamps_ns":record.output_timestamps_ns,"cancelled_ns":record.cancelled_ns,
            "ttft_ns":record.output_timestamps_ns.first().map(|t| t - record.arrival_ns),
            "decode_intervals_ns":intervals,"tpot_ns":tpot}),
        )
    }

    fn retire(&mut self) -> Result<(), String> {
        let mut index = 0;
        while index < self.active.len() {
            let id = self.active[index].id;
            if matches!(
                self.runtime.request(id)?.state(),
                TpRequestStateV1::Completed | TpRequestStateV1::Cancelled
            ) {
                let record = self.runtime.retire(id, self.tick)?;
                let active = self.active.remove(index);
                self.retired_event(&active, &record)?;
            } else {
                index += 1;
            }
        }
        Ok(())
    }

    fn step(
        &mut self,
        mut now: impl FnMut() -> Result<u64, String>,
        timing: &HostTiming,
    ) -> Result<(), String> {
        let report = {
            let _batch_timing = timing.scope("controller_batch");
            let started = now()?;
            let mut clock_error = None;
            let report = self.runtime.step(self.tick, started, || match now() {
                Ok(completed) => completed,
                Err(error) => {
                    clock_error = Some(error);
                    started
                }
            });
            if let Some(error) = clock_error {
                return Err(error);
            }
            report?.ok_or("live active requests produced no batch")?
        };
        for output in &report.outputs {
            let active = self
                .active
                .iter()
                .find(|r| r.id == output.request)
                .ok_or("live output has no admitted request")?;
            let decoded = self.codec.decode(&[output.token_id])?;
            self.event(
                "token",
                json!({"request_id":active.request_id,"name":active.name,
                "slot":output.request.slot,"generation":output.request.generation,
                "token":output.token_id,"index":output.output_index,"decoded_bytes":decoded,
                "completed_ns":output.completed_ns,"finished":output.finished}),
            )?;
        }
        self.event("batch", json!({"batch_id":report.batch_id,"tick":self.tick,
            "rows":report.rows.len(),"outputs":report.outputs.len(),
            "started_ns":report.started_ns,"completed_ns":report.completed_ns,
            "rank_dispatch_counts":report.rank_dispatch_counts,"output_head_rows":report.output_head_rows}))?;
        self.retire()?;
        self.tick = self.tick.checked_add(1).ok_or("live tick overflow")?;
        self.batches = self.batches.checked_add(1).ok_or("live batch overflow")?;
        Ok(())
    }

    fn enforce_budget(&mut self, max_batches: u64, now_ns: u64) -> Result<(), String> {
        if self.batches < max_batches || self.active.is_empty() && self.pending.is_empty() {
            return Ok(());
        }
        self.drain("batch_budget")?;
        self.cancel_all(now_ns, "batch_budget")?;
        self.event(
            "stopped",
            json!({"reason":"batch_budget","batches":self.batches}),
        )?;
        Err("live session exhausted its conservative batch budget".into())
    }
}

/// Streams local JSONL requests through one already-initialized resident runtime.
/// The caller retains ownership of closing workers after success or failure.
/// # Errors
/// Rejects I/O failures, exhausted batch budgets, and any runtime state error.
pub fn run<G: EngineeringTpBatchRunnerV2>(
    runtime: &mut EngineeringTpBatchRuntimeV2<G>,
    model: &EngineeringQwenModelV1,
    context: u32,
    pages: u32,
    max_batches: u64,
    timing: &HostTiming,
    mut output: impl FnMut(&Value) -> Result<(), String>,
) -> Result<(), String> {
    let clock = Instant::now();
    let mut input = LiveInput::start(std::io::stdin(), clock)?;
    let mut session = Session {
        runtime,
        codec: model,
        emit: |mut value: Value| {
            value["emission_started_ns"] = json!(elapsed_ns(clock)?);
            output(&value)
        },
        pending: VecDeque::new(),
        active: Vec::new(),
        context,
        pages,
        last_request_id: 0,
        tick: 0,
        batches: 0,
        draining: false,
    };
    let result = (|| {
        session.event(
            "ready",
            json!({"clock":"monotonic_ns_since_live_start",
            "arrival_policy":"complete command received; includes input and pending queue wait",
            "max_active_requests":TP_MAX_REQUESTS_V1,"max_pending_requests":MAX_PENDING,
            "input_channel_capacity":INPUT_CAPACITY,"max_command_bytes":MAX_LINE_BYTES,
            "max_prompt_bytes":MAX_PROMPT_BYTES,
            "max_batches":max_batches,"context_tokens":context,"physical_pages":pages,
            "admission_policy":"FIFO with worst-case private KV reservation",
            "cancellation_policy":"between completed GPU batches","eos_policy":"fixed output count",
            "output_policy":"synchronous JSONL; consumer backpressure stalls scheduling"}),
        )?;
        loop {
            if session.active.is_empty() && session.pending.is_empty() && session.draining {
                return session.event(
                    "stopped",
                    json!({"reason":"drained","batches":session.batches}),
                );
            }
            for _ in 0..INPUTS_PER_BATCH {
                match input.receiver.try_recv() {
                    Ok(command) => session.input(command, elapsed_ns(clock)?)?,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        session.drain("eof")?;
                        break;
                    }
                }
            }
            session.enforce_budget(max_batches, elapsed_ns(clock)?)?;
            session.admit(elapsed_ns(clock)?)?;
            if !session.active.is_empty() {
                session.step(|| elapsed_ns(clock), timing)?;
            } else if !session.draining {
                match input.receiver.recv() {
                    Ok(command) => session.input(command, elapsed_ns(clock)?)?,
                    Err(_) => session.drain("eof")?,
                }
            }
        }
    })();
    let joined = input.stop();
    match (result, joined) {
        (Ok(()), result) | (result, Ok(())) => result,
        (Err(error), Err(join)) => Err(format!("{error}; {join}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::io::Write;
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;

    use crate::tp_execution::batched::EngineeringTpBatchOutputV2;
    use crate::tp_paged::{
        EngineeringTpBatchCompletionV1, EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1,
        EngineeringTpPoolScopeV1, EngineeringTpPreparedBatchV1,
    };
    use crate::tp_scheduler::EngineeringTpSchedulerV1;

    struct FakeRunner {
        count: u64,
        fail: bool,
    }

    impl EngineeringTpBatchRunnerV2 for FakeRunner {
        fn execute_batch(
            &mut self,
            batch: &EngineeringTpPreparedBatchV1,
            output_rows: &[usize],
        ) -> Result<EngineeringTpBatchOutputV2, String> {
            if self.fail {
                return Err("injected submitted failure".into());
            }
            self.count += 544;
            Ok(EngineeringTpBatchOutputV2 {
                choices: vec![3; output_rows.len()],
                completion: EngineeringTpBatchCompletionV1::after_all_ranks(batch),
            })
        }
        fn dispatch_counts(&self) -> Vec<u64> {
            vec![self.count]
        }
        fn close(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    struct FakeCodec;
    impl TokenCodec for FakeCodec {
        fn encode(&self, prompt: &str) -> Result<Vec<u32>, String> {
            Ok(prompt.bytes().map(|b| u32::from(b % 100)).collect())
        }
        fn decode(&self, tokens: &[u32]) -> Result<Vec<u8>, String> {
            Ok(vec![b'x'; tokens.len()])
        }
    }

    fn runtime(pages: u32, fail: bool) -> EngineeringTpBatchRuntimeV2<FakeRunner> {
        let pool = EngineeringTpPagedPoolV1::new(
            EngineeringTpPoolScopeV1 {
                model: [1; 32],
                session: [2; 32],
            },
            EngineeringTpPagedLimitsV1::new(64, 32, pages, 1000).unwrap(),
        )
        .unwrap();
        EngineeringTpBatchRuntimeV2::new(
            FakeRunner { count: 0, fail },
            pool,
            EngineeringTpSchedulerV1::new(100, 64, 16, 16).unwrap(),
            16,
            true,
        )
        .unwrap()
    }

    fn session<'a>(
        runtime: &'a mut EngineeringTpBatchRuntimeV2<FakeRunner>,
        events: &'a RefCell<Vec<Value>>,
        pages: u32,
    ) -> Session<'a, FakeRunner, FakeCodec, impl FnMut(Value) -> Result<(), String> + 'a> {
        Session {
            runtime,
            codec: &FakeCodec,
            emit: |event| {
                events.borrow_mut().push(event);
                Ok(())
            },
            pending: VecDeque::new(),
            active: Vec::new(),
            context: 64,
            pages,
            last_request_id: 0,
            tick: 0,
            batches: 0,
            draining: false,
        }
    }

    fn submit(request_id: u64, prompt: &str, new_tokens: u32) -> Command {
        Command::Submit {
            schema: COMMAND_SCHEMA.into(),
            request_id,
            name: format!("r{request_id}"),
            prompt: prompt.into(),
            new_tokens,
        }
    }

    fn cancel(request_id: u64) -> Command {
        Command::Cancel {
            schema: COMMAND_SCHEMA.into(),
            request_id,
        }
    }

    #[test]
    fn commands_use_closed_schemas_and_bounded_fields() {
        let valid = json!({"schema":COMMAND_SCHEMA,"op":"submit","request_id":1,
            "name":"first","prompt":"a","new_tokens":1});
        assert!(Command::parse(&serde_json::to_vec(&valid).unwrap()).is_ok());
        for (field, value) in [
            ("schema", json!("wrong")),
            ("op", json!("unknown")),
            ("request_id", json!(0)),
            ("name", json!("")),
            ("prompt", json!("")),
            ("new_tokens", json!(0)),
            ("new_tokens", json!(8193)),
            ("extra", json!(true)),
            ("prompt", json!("a".repeat(MAX_PROMPT_BYTES + 1))),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(Command::parse(&serde_json::to_vec(&invalid).unwrap()).is_err());
        }
        for op in ["cancel", "drain", "shutdown"] {
            let mut command = json!({"schema":COMMAND_SCHEMA,"op":op});
            if op == "cancel" {
                command["request_id"] = json!(1);
            }
            assert!(Command::parse(&serde_json::to_vec(&command).unwrap()).is_ok());
            command["prompt"] = json!("unexpected");
            assert!(Command::parse(&serde_json::to_vec(&command).unwrap()).is_err());
        }
        assert!(Command::parse(b"{").is_err());
        assert!(Command::parse(&vec![b' '; MAX_LINE_BYTES + 1]).is_err());
    }

    #[test]
    fn oversized_framing_discards_to_boundary_and_recovers_without_growing() {
        let mut framer = Framer::default();
        for _ in 0..MAX_LINE_BYTES + 4096 {
            assert!(framer.push(b'x').is_none());
            assert!(framer.bytes.len() <= MAX_LINE_BYTES);
        }
        assert!(framer.has_partial());
        assert!(framer.push(b'\n').unwrap().is_err());
        let valid = format!("{{\"schema\":\"{COMMAND_SCHEMA}\",\"op\":\"drain\"}}\r\n");
        let results = valid
            .bytes()
            .filter_map(|byte| framer.push(byte))
            .collect::<Vec<_>>();
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], Ok(Command::Drain { .. })));
        assert!(!framer.has_partial());
    }

    #[test]
    fn stdin_eof_accepts_final_unterminated_json_and_joins() {
        let (mut writer, reader) = UnixStream::pair().unwrap();
        let mut input = LiveInput::start(reader, Instant::now()).unwrap();
        write!(
            writer,
            "{{\"schema\":\"{COMMAND_SCHEMA}\",\"op\":\"drain\"}}"
        )
        .unwrap();
        writer.shutdown(Shutdown::Write).unwrap();
        assert!(matches!(
            input.receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
            Input::Command {
                command: Ok(Command::Drain { .. }),
                ..
            }
        ));
        assert!(matches!(
            input.receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
            Input::Eof
        ));
        input.stop().unwrap();
        assert!(input.reader.is_none());
    }

    #[test]
    fn reader_shutdown_does_not_wait_for_idle_stdin_or_a_full_channel() {
        for flood in [false, true] {
            let (mut writer, reader) = UnixStream::pair().unwrap();
            let mut input = LiveInput::start(reader, Instant::now()).unwrap();
            if flood {
                for _ in 0..INPUT_CAPACITY * 4 {
                    writer.write_all(b"{}\n").unwrap();
                }
                thread::sleep(Duration::from_millis(30));
            }
            let clock = Instant::now();
            input.stop().unwrap();
            assert!(clock.elapsed() < Duration::from_secs(5));
        }
    }

    #[test]
    fn queued_wait_is_in_ttft_and_pages_are_not_overcommitted() {
        let mut runtime = runtime(1, false);
        let events = RefCell::new(Vec::new());
        let mut session = session(&mut runtime, &events, 1);
        session.command(submit(1, "a", 2), 5, 10).unwrap();
        session.command(submit(2, "b", 1), 6, 10).unwrap();
        session.admit(20).unwrap();
        assert_eq!(session.active.len(), 1);
        assert_eq!(session.pending.len(), 1);
        session.step(|| Ok(30), &HostTiming::default()).unwrap();
        session.step(|| Ok(40), &HostTiming::default()).unwrap();
        session.admit(50).unwrap();
        session.step(|| Ok(60), &HostTiming::default()).unwrap();
        let events = events.borrow();
        let admission = events
            .iter()
            .find(|e| e["event"] == "admission" && e["request_id"] == 2)
            .unwrap();
        assert_eq!(admission["arrival_ns"], 6);
        assert_eq!(admission["queue_wait_ns"], 44);
        let request = events
            .iter()
            .find(|e| e["event"] == "request" && e["request_id"] == 2)
            .unwrap();
        assert_eq!(request["ttft_ns"], 54);
        assert_eq!(request["output_timestamps_ns"], json!([60]));
        assert_eq!(request["generated_tokens"], json!([3]));
    }

    #[test]
    fn streamed_token_bytes_concatenate_to_the_final_decoded_bytes() {
        let mut runtime = runtime(1, false);
        let events = RefCell::new(Vec::new());
        let mut session = session(&mut runtime, &events, 1);
        session.command(submit(1, "a", 3), 1, 1).unwrap();
        session.admit(2).unwrap();
        for now in 3..=5 {
            session.step(|| Ok(now), &HostTiming::default()).unwrap();
        }
        let events = events.borrow();
        let bytes = events
            .iter()
            .filter(|e| e["event"] == "token")
            .flat_map(|e| e["decoded_bytes"].as_array().unwrap().iter().cloned())
            .collect::<Vec<_>>();
        let request = events.iter().find(|e| e["event"] == "request").unwrap();
        assert_eq!(json!(bytes), request["generated_utf8_bytes"]);
        assert_eq!(bytes.len(), 3);
    }

    #[test]
    fn continuous_arrivals_reuse_slots_without_restarting_runtime() {
        let mut runtime = runtime(1, false);
        let events = RefCell::new(Vec::new());
        let mut session = session(&mut runtime, &events, 1);
        for id in 1..=65 {
            session
                .command(submit(id, "a", 1), id * 10, id * 10)
                .unwrap();
            session.admit(id * 10 + 1).unwrap();
            session
                .step(|| Ok(id * 10 + 2), &HostTiming::default())
                .unwrap();
            assert!(session.active.is_empty());
        }
        assert_eq!(session.batches, 65);
        let events = events.borrow();
        let admissions = events
            .iter()
            .filter(|e| e["event"] == "admission")
            .collect::<Vec<_>>();
        assert_eq!(admissions.len(), 65);
        assert_eq!(admissions[0]["slot"], admissions[64]["slot"]);
        assert_eq!(admissions[64]["generation"], 65);
        assert_eq!(events.iter().filter(|e| e["event"] == "token").count(), 65);
    }

    #[test]
    fn later_live_requests_reuse_retained_complete_prefix_pages() {
        let mut runtime = runtime(2, false);
        let events = RefCell::new(Vec::new());
        let mut session = session(&mut runtime, &events, 2);
        let prompt = "a".repeat(17);
        session.command(submit(1, &prompt, 2), 1, 1).unwrap();
        session.admit(2).unwrap();
        for now in 3..=5 {
            session.step(|| Ok(now), &HostTiming::default()).unwrap();
        }
        assert!(session.active.is_empty());
        session.command(submit(2, &prompt, 1), 6, 6).unwrap();
        session.admit(7).unwrap();
        session.step(|| Ok(8), &HostTiming::default()).unwrap();
        assert!(session.active.is_empty());
        let events = events.borrow();
        let admission = events
            .iter()
            .find(|e| e["event"] == "admission" && e["request_id"] == 2)
            .unwrap();
        assert_eq!(admission["cached_tokens"], 16);
        assert_eq!(admission["cached_pages"], 1);
        assert_eq!(events.iter().filter(|e| e["event"] == "token").count(), 3);
    }

    #[test]
    fn queued_active_and_stale_cancellation_cannot_cross_generations() {
        let mut runtime = runtime(1, false);
        let events = RefCell::new(Vec::new());
        let mut session = session(&mut runtime, &events, 1);
        session.command(submit(1, "a", 2), 1, 2).unwrap();
        session.command(submit(2, "b", 2), 2, 3).unwrap();
        session.admit(4).unwrap();
        session.command(cancel(2), 5, 5).unwrap();
        assert!(session.pending.is_empty());
        session.command(cancel(1), 6, 6).unwrap();
        assert!(session.active.is_empty());
        session.command(submit(3, "c", 2), 7, 7).unwrap();
        session.admit(8).unwrap();
        let next = session.active[0].id;
        session.command(cancel(1), 9, 9).unwrap();
        assert_eq!(session.active[0].id, next);
        assert_ne!(
            session.runtime.request(next).unwrap().state(),
            TpRequestStateV1::Cancelled
        );
        session.command(submit(1, "d", 1), 10, 10).unwrap();
        assert!(session.pending.is_empty());
        let events = events.borrow();
        assert!(
            events
                .iter()
                .any(|e| e["event"] == "cancel" && e["status"] == "not_found")
        );
        assert!(
            events
                .iter()
                .any(|e| e["reason"] == "request_id_not_increasing")
        );
    }

    #[test]
    fn overload_and_context_capacity_rejections_are_per_request() {
        let mut runtime = runtime(1, false);
        let events = RefCell::new(Vec::new());
        let mut session = session(&mut runtime, &events, 1);
        session
            .command(submit(1, &"a".repeat(65), 1), 1, 1)
            .unwrap();
        session
            .command(submit(2, &"a".repeat(17), 1), 2, 2)
            .unwrap();
        for id in 3..=35 {
            session.command(submit(id, "a", 1), id, id).unwrap();
        }
        assert_eq!(session.pending.len(), MAX_PENDING);
        let events = events.borrow();
        assert!(
            events
                .iter()
                .any(|e| e["request_id"] == 1 && e["reason"] == "context_limit")
        );
        assert!(
            events
                .iter()
                .any(|e| e["request_id"] == 2 && e["reason"] == "page_capacity")
        );
        assert!(
            events
                .iter()
                .any(|e| e["request_id"] == 35 && e["reason"] == "overloaded")
        );
    }

    #[test]
    fn drain_and_eof_finish_accepted_work_but_shutdown_cancels_it() {
        for shutdown in [false, true] {
            let mut runtime = runtime(1, false);
            let events = RefCell::new(Vec::new());
            let mut session = session(&mut runtime, &events, 1);
            session.command(submit(1, "a", 2), 1, 1).unwrap();
            session.admit(2).unwrap();
            session.command(submit(2, "b", 2), 3, 3).unwrap();
            if shutdown {
                session
                    .command(
                        Command::Shutdown {
                            schema: COMMAND_SCHEMA.into(),
                        },
                        4,
                        4,
                    )
                    .unwrap();
                assert!(session.active.is_empty() && session.pending.is_empty());
            } else {
                session.input(Input::Eof, 4).unwrap();
                assert_eq!(session.active.len(), 1);
                assert_eq!(session.pending.len(), 1);
                session.step(|| Ok(5), &HostTiming::default()).unwrap();
                session.step(|| Ok(6), &HostTiming::default()).unwrap();
                session.admit(7).unwrap();
                session.step(|| Ok(8), &HostTiming::default()).unwrap();
                session.step(|| Ok(9), &HostTiming::default()).unwrap();
                assert!(session.active.is_empty() && session.pending.is_empty());
            }
            assert!(session.draining);
            session.command(submit(3, "c", 1), 10, 10).unwrap();
            assert!(session.pending.is_empty());
            assert!(
                events
                    .borrow()
                    .iter()
                    .any(|e| e["request_id"] == 3 && e["reason"] == "draining")
            );
        }
    }

    #[test]
    fn batch_cap_cancels_and_reports_all_accepted_work() {
        let mut runtime = runtime(1, false);
        let events = RefCell::new(Vec::new());
        let mut session = session(&mut runtime, &events, 1);
        session.command(submit(1, "a", 3), 1, 1).unwrap();
        session.command(submit(2, "b", 3), 2, 2).unwrap();
        session.admit(3).unwrap();
        session.step(|| Ok(4), &HostTiming::default()).unwrap();
        assert!(session.enforce_budget(1, 5).is_err());
        assert!(session.active.is_empty() && session.pending.is_empty());
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|e| e["event"] == "request")
                .count(),
            2
        );
        assert!(
            events
                .borrow()
                .iter()
                .any(|e| e["event"] == "stopped" && e["reason"] == "batch_budget")
        );
    }

    #[test]
    fn submitted_failure_publishes_no_token_and_runtime_stays_poisoned() {
        let mut runtime = runtime(1, true);
        let events = RefCell::new(Vec::new());
        let mut session = session(&mut runtime, &events, 1);
        session.command(submit(1, "a", 1), 1, 1).unwrap();
        session.admit(2).unwrap();
        assert!(session.step(|| Ok(3), &HostTiming::default()).is_err());
        assert!(session.step(|| Ok(4), &HostTiming::default()).is_err());
        assert!(!events.borrow().iter().any(|e| e["event"] == "token"));
    }
}
