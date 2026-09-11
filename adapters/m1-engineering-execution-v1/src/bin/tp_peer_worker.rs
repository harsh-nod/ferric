//! Safe rank proxies for one explicitly opted-in disposable peer-memory child.
//! Serial and concurrent-round profiles have distinct, checked handshakes.

#[path = "../../../tp-peer-engineering-worker-v4/src/wire.rs"]
#[allow(dead_code)]
mod peer_wire;

use super::tp_worker::{LoadedKernel, RuntimeOptions, metadata_matches, pack_dispatch};
use fe2o3_kfd::engineering_wire::{CommandV1, ResponseV1, SequenceDispatchV1};
use ferric_m1_engineering_execution_v1::host_timing::{HostTiming, IpcTiming};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpArgumentV1, EngineeringTpBufferAccessV1, EngineeringTpDispatchV1,
    EngineeringTpRankTransportV1, TpResult,
};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_mins(2);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
type Incoming = (peer_wire::Response, Vec<u8>);
type Outgoing = (peer_wire::Request, Vec<u8>);

struct Connection {
    child: Child,
    writer: Option<SyncSender<Outgoing>>,
    written: Receiver<TpResult<()>>,
    reader: Receiver<TpResult<Incoming>>,
    threads: Vec<JoinHandle<()>>,
    next_request: u64,
    pending: Vec<Option<u64>>,
    order: VecDeque<(u64, usize)>,
    completed: Vec<Option<(ResponseV1, Vec<u8>)>>,
    closed: Vec<bool>,
    capacities: BTreeMap<u64, usize>,
    ownership: BTreeMap<u64, (usize, bool)>,
    failed: bool,
    exited: bool,
    timeout: Duration,
    concurrent_rounds: bool,
    queued_round: Vec<(usize, SequenceDispatchV1, Vec<u8>)>,
    timing: HostTiming,
    tickets: BTreeMap<u64, IpcTiming>,
}

impl Connection {
    fn connect_profile(
        mut child: Child,
        ids: &[u64],
        timeout: Duration,
        concurrent_rounds: bool,
        timing: HostTiming,
    ) -> TpResult<Self> {
        let Some(mut input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("peer stdin missing".into());
        };
        let Some(mut output) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("peer stdout missing".into());
        };
        let (writer, outgoing) = mpsc::sync_channel::<Outgoing>(1);
        let (written, acknowledgments) = mpsc::sync_channel(1);
        let writing = thread::spawn(move || {
            while let Ok((header, bytes)) = outgoing.recv() {
                let result = peer_wire::write_request(&mut input, &header, &bytes)
                    .map_err(|e| e.to_string());
                let failed = result.is_err();
                if written.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let (incoming, reader) = mpsc::sync_channel(ids.len() + 1);
        let reading = thread::spawn(move || {
            loop {
                let result = peer_wire::read_response(&mut output).map_err(|e| e.to_string());
                let failed = result.is_err();
                if incoming.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let mut connection = Self {
            child,
            writer: Some(writer),
            written: acknowledgments,
            reader,
            threads: vec![writing, reading],
            next_request: 1,
            pending: vec![None; ids.len()],
            order: VecDeque::new(),
            completed: vec![None; ids.len()],
            closed: vec![false; ids.len()],
            capacities: BTreeMap::new(),
            ownership: BTreeMap::new(),
            failed: false,
            exited: false,
            timeout,
            concurrent_rounds,
            queued_round: Vec::new(),
            timing,
            tickets: BTreeMap::new(),
        };
        let ready = connection.reader.recv_timeout(timeout);
        if !matches!(ready, Ok(Ok((peer_wire::Response::Ready { protocol, mode, target,
            unique_ids, process_id, authority }, payload))) if protocol == peer_wire::PROTOCOL
                && mode == if concurrent_rounds { peer_wire::ROUND_MODE } else { peer_wire::MODE }
                && target == "gfx950:xnack-" && unique_ids == ids
                && process_id == connection.child.id() && authority == "none" && payload.is_empty())
        {
            return connection.reject("peer ready identity, roster, mode, or deadline mismatch");
        }
        Ok(connection)
    }

    fn reject<T>(&mut self, message: impl Into<String>) -> TpResult<T> {
        self.failed = true;
        self.tickets.clear();
        let message = message.into();
        Err(match self.terminate() {
            Ok(()) => message,
            Err(error) => format!("{message}; cleanup: {error}"),
        })
    }

    fn send(
        &mut self,
        rank: usize,
        command: CommandV1,
        peer_readable: bool,
        payload: Vec<u8>,
    ) -> TpResult<()> {
        if self.failed
            || self.exited
            || rank >= self.pending.len()
            || self.closed[rank]
            || self.pending[rank].is_some()
        {
            return self.reject("peer rank is not ready for a request");
        }
        let request = self.next_request;
        let Some(next) = request.checked_add(1) else {
            return self.reject("peer request identity exhausted");
        };
        let (kind, count) = match &command {
            CommandV1::Dispatch { .. } => ("dispatch", 1),
            CommandV1::DispatchSequence { dispatches } => {
                ("dispatch_sequence", dispatches.len() as u64)
            }
            CommandV1::Write { .. } => ("write", 0),
            CommandV1::Read { .. } => ("read", 0),
            CommandV1::Allocate { .. } => ("allocate", 0),
            CommandV1::LoadKernel { .. } => ("load_kernel", 0),
            CommandV1::ConfigurePerformance { .. } => ("configure_performance", 0),
            CommandV1::Close => ("close", 0),
            _ => ("other", 0),
        };
        if self.timing.is_enabled() {
            let ticket = self.timing.request(
                Some(u32::try_from(rank).map_err(|_| "timing rank overflow")?),
                kind,
                payload.len(),
                count,
            );
            self.tickets.insert(request, ticket);
        }
        let header = peer_wire::Request {
            id: request,
            rank: u32::try_from(rank).map_err(|_| "rank overflow")?,
            command,
            peer_readable,
            round_ranks: vec![],
        };
        let length = match header.payload_bytes() {
            Ok(length) => length,
            Err(error) => return self.reject(error.to_string()),
        };
        if length != payload.len() {
            return self.reject("peer outgoing payload mismatch");
        }
        self.next_request = next;
        self.pending[rank] = Some(request);
        self.order.push_back((request, rank));
        let Some(writer) = &self.writer else {
            return self.reject("peer writer closed");
        };
        if writer.try_send((header, payload)).is_err() {
            return self.reject("peer writer unavailable");
        }
        match self.written.recv_timeout(self.timeout) {
            Ok(Ok(())) => {
                if let Some(ticket) = self.tickets.get_mut(&request) {
                    ticket.sent(true);
                }
                Ok(())
            }
            Ok(Err(error)) => self.reject(error),
            Err(error) => self.reject(format!("peer write deadline/disconnect: {error}")),
        }
    }

    fn receive(&mut self, rank: usize) -> TpResult<(ResponseV1, Vec<u8>)> {
        if self.failed || self.exited || self.pending.get(rank).is_none_or(Option::is_none) {
            return self.reject("peer rank has no pending completion");
        }
        if !self.queued_round.is_empty() {
            self.flush_round()?;
        }
        while self.completed[rank].is_none() {
            let _waiting = self.timing.span(
                "ipc_response_wait",
                Some(u32::try_from(rank).map_err(|_| "timing rank overflow")?),
            );
            let incoming = match self.reader.recv_timeout(self.timeout) {
                Ok(Ok(value)) => value,
                Ok(Err(error)) => return self.reject(error),
                Err(error) => {
                    return self.reject(format!("peer response deadline/disconnect: {error}"));
                }
            };
            let (
                peer_wire::Response::Done {
                    request,
                    rank: actual_rank,
                    response,
                },
                payload,
            ) = incoming
            else {
                return self.reject("unexpected peer ready response");
            };
            let actual_rank = actual_rank as usize;
            if self.order.pop_front() != Some((request, actual_rank))
                || self.pending.get(actual_rank) != Some(&Some(request))
                || self.completed[actual_rank].is_some()
            {
                return self.reject("peer response order, request identity, or rank mismatch");
            }
            if let ResponseV1::Error { message, fatal } = response {
                return self.reject(format!("peer group failed (fatal={fatal}): {message}"));
            }
            if let Some(ticket) = self.tickets.remove(&request) {
                ticket.finish(payload.len(), true);
            }
            self.completed[actual_rank] = Some((response, payload));
        }
        self.pending[rank] = None;
        self.completed[rank]
            .take()
            .ok_or_else(|| "peer completion disappeared".into())
    }

    fn queue_round(&mut self, rank: usize, command: CommandV1, payload: Vec<u8>) -> TpResult<()> {
        if !self.concurrent_rounds
            || self.failed
            || self.exited
            || rank >= self.pending.len()
            || self.closed[rank]
            || self.pending[rank].is_some()
            || self.completed.iter().any(Option::is_some)
            || !self.order.is_empty()
            || self.pending.iter().filter(|entry| entry.is_some()).count()
                != self.queued_round.len()
        {
            return self.reject("peer round rank or barrier state mismatch");
        }
        let bytes = match command.payload_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return self.reject(error.to_string()),
        };
        if bytes != payload.len() {
            return self.reject("peer round payload mismatch");
        }
        let CommandV1::Dispatch {
            kernel,
            payload_bytes,
            workgroup,
            grid,
            pointers,
            timeout_ms,
        } = command
        else {
            return self.reject("peer round accepts only individual dispatches");
        };
        let timeout_ms = timeout_ms
            .min(600_000 / u32::try_from(self.pending.len()).map_err(|_| "peer world overflow")?);
        self.pending[rank] = Some(self.next_request);
        self.queued_round.push((
            rank,
            SequenceDispatchV1 {
                kernel,
                payload_bytes,
                workgroup,
                grid,
                pointers,
                timeout_ms,
            },
            payload,
        ));
        Ok(())
    }

    fn flush_round(&mut self) -> TpResult<()> {
        let request = self.next_request;
        let Some(next) = request.checked_add(1) else {
            return self.reject("peer round request identity exhausted");
        };
        let entries = std::mem::take(&mut self.queued_round);
        let mut ticket = self.timing.request(
            None,
            "dispatch_round",
            entries.iter().map(|(_, _, bytes)| bytes.len()).sum(),
            entries.len() as u64,
        );
        let ranks = entries
            .iter()
            .map(|(rank, _, _)| u32::try_from(*rank).map_err(|_| "peer rank overflow"))
            .collect::<Result<Vec<_>, _>>()?;
        let mut payload = Vec::new();
        let dispatches = entries
            .into_iter()
            .map(|(_, dispatch, bytes)| {
                payload.extend_from_slice(&bytes);
                dispatch
            })
            .collect();
        let header = peer_wire::Request {
            id: request,
            rank: ranks[0],
            peer_readable: false,
            command: CommandV1::DispatchSequence { dispatches },
            round_ranks: ranks.clone(),
        };
        let bytes = match header.payload_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return self.reject(error.to_string()),
        };
        if bytes != payload.len() {
            return self.reject("peer round frame bound mismatch");
        }
        self.next_request = next;
        let Some(writer) = &self.writer else {
            return self.reject("peer writer closed");
        };
        if writer.try_send((header, payload)).is_err() {
            return self.reject("peer round writer unavailable");
        }
        match self.written.recv_timeout(self.timeout) {
            Ok(Ok(())) => {
                ticket.sent(true);
            }
            Ok(Err(error)) => return self.reject(error),
            Err(error) => {
                return self.reject(format!("peer round write deadline/disconnect: {error}"));
            }
        }
        let _waiting = self.timing.span("ipc_response_wait", None);
        let incoming = match self.reader.recv_timeout(self.timeout) {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => return self.reject(error),
            Err(error) => {
                return self.reject(format!("peer round response deadline/disconnect: {error}"));
            }
        };
        let (
            peer_wire::Response::RoundDone {
                request: actual_request,
                ranks: actual_ranks,
                response,
            },
            payload,
        ) = incoming
        else {
            return self.reject("peer round expected all-rank completion");
        };
        if actual_request != request
            || actual_ranks != ranks
            || !payload.is_empty()
            || ranks.iter().any(|&rank| {
                self.pending[rank as usize] != Some(request)
                    || self.completed[rank as usize].is_some()
            })
        {
            return self.reject("peer round completion identity or roster mismatch");
        }
        let ResponseV1::DispatchSequenceCompleted { elapsed_ns } = response else {
            return self.reject(format!("peer round failed: {response:?}"));
        };
        if elapsed_ns.len() != ranks.len() {
            return self.reject("peer round completion count mismatch");
        }
        // Validate the entire receipt before making any rank completion available.
        for (rank, elapsed_ns) in ranks.into_iter().zip(elapsed_ns) {
            self.completed[rank as usize] = Some((ResponseV1::Dispatched { elapsed_ns }, vec![]));
        }
        ticket.finish(0, true);
        Ok(())
    }

    fn sync(
        &mut self,
        rank: usize,
        command: CommandV1,
        peers: bool,
        payload: Vec<u8>,
    ) -> TpResult<(ResponseV1, Vec<u8>)> {
        if self.pending.iter().any(Option::is_some) {
            return self.reject("peer lifecycle or host access cannot overlap pending dispatches");
        }
        self.send(rank, command, peers, payload)?;
        self.receive(rank)
    }

    fn check_owned_extent(
        &self,
        rank: usize,
        id: u64,
        offset: usize,
        bytes: usize,
    ) -> TpResult<()> {
        if self
            .ownership
            .get(&id)
            .is_none_or(|&(owner, _)| owner != rank)
            || !self.capacities.get(&id).is_some_and(|&capacity| {
                offset <= capacity && offset.checked_add(bytes).is_some_and(|end| end <= capacity)
            })
        {
            return Err("peer host access ownership or extent mismatch".into());
        }
        Ok(())
    }

    fn configure_performance(&mut self, cache: bool, operational: bool) -> TpResult<()> {
        if !cache && !operational {
            return Ok(());
        }
        let (response, payload) = self.sync(
            0,
            CommandV1::ConfigurePerformance {
                cache_kernel_admission: cache,
                operational_currentness: operational,
                profile: false,
            },
            false,
            vec![],
        )?;
        if !matches!(response, ResponseV1::PerformanceConfigured) || !payload.is_empty() {
            return self.reject("peer performance configuration receipt mismatch");
        }
        Ok(())
    }

    fn terminate(&mut self) -> TpResult<()> {
        self.writer.take();
        if self.exited {
            return Ok(());
        }
        if self.child.try_wait().map_err(|e| e.to_string())?.is_none() {
            self.child
                .kill()
                .map_err(|e| format!("cannot terminate peer child: {e}"))?;
        }
        self.await_exit(false)
    }

    fn await_exit(&mut self, successful: bool) -> TpResult<()> {
        let start = Instant::now();
        loop {
            // Drain bounded read delivery so a failing peer cannot pin an IO thread.
            while self.reader.try_recv().is_ok() {}
            if let Some(status) = self.child.try_wait().map_err(|e| e.to_string())? {
                self.exited = true;
                self.writer.take();
                while self.threads.iter().any(|thread| !thread.is_finished()) {
                    while self.reader.try_recv().is_ok() {}
                    if start.elapsed() >= EXIT_TIMEOUT {
                        return Err("peer child exited but IPC threads did not stop".into());
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                for thread in self.threads.drain(..) {
                    let _ = thread.join();
                }
                return if successful && !status.success() {
                    Err(format!("peer child exited with {status}"))
                } else {
                    Ok(())
                };
            }
            if start.elapsed() >= EXIT_TIMEOUT {
                return Err("peer child exit was not confirmed".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if !self.exited
            && let Err(error) = self.terminate()
        {
            eprintln!("peer worker cleanup: {error}");
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PendingDispatch {
    Single,
    Sequence(usize),
}

pub struct PeerWorker {
    connection: Rc<RefCell<Connection>>,
    rank: usize,
    kernels: BTreeMap<String, LoadedKernel>,
    pending: Option<PendingDispatch>,
    sequences: bool,
}

impl PeerWorker {
    pub fn spawn_with_timing(
        executable: &Path,
        unique_ids: &[u64],
        artifacts: &[&EngineeringTpArtifactV1],
        options: RuntimeOptions,
        concurrent_rounds: bool,
        timing: HostTiming,
    ) -> TpResult<Vec<Self>> {
        if options.rollover || concurrent_rounds && options.sequences {
            return Err("peer rollover and concurrent same-rank sequences are unsupported".into());
        }
        if !matches!(unique_ids.len(), 2 | 8)
            || unique_ids.contains(&0)
            || unique_ids.iter().copied().collect::<BTreeSet<_>>().len() != unique_ids.len()
            || artifacts.len() != 2
        {
            return Err("peer spawn requires exact TP2/8 roster and two admitted artifacts".into());
        }
        let mut command = Command::new(executable);
        command
            .arg("--allow-unauthenticated-machine-code")
            .arg("--device-unique-ids")
            .arg(
                unique_ids
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        if concurrent_rounds {
            command.arg("--concurrent-rounds");
        }
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("spawn peer child: {e}"))?;
        let connection = Rc::new(RefCell::new(Connection::connect_profile(
            child,
            unique_ids,
            TIMEOUT,
            concurrent_rounds,
            timing,
        )?));
        connection
            .borrow_mut()
            .configure_performance(options.cache_admission, options.operational)?;
        let mut workers = Vec::new();
        for rank in 0..unique_ids.len() {
            let mut worker = Self {
                connection: Rc::clone(&connection),
                rank,
                kernels: BTreeMap::new(),
                pending: None,
                sequences: options.sequences,
            };
            for artifact in artifacts {
                worker.load_artifact(artifact)?;
            }
            workers.push(worker);
        }
        Ok(workers)
    }

    fn load_artifact(&mut self, artifact: &EngineeringTpArtifactV1) -> TpResult<()> {
        let hash: [u8; 32] = Sha256::digest(artifact.bytes()).into();
        let mut connection = self.connection.borrow_mut();
        for metadata in artifact.inspection().hsaco().kernels() {
            if self.kernels.contains_key(metadata.name()) {
                return connection.reject("duplicate peer kernel symbol");
            }
            let (response, payload) = connection.sync(
                self.rank,
                CommandV1::LoadKernel {
                    payload_bytes: u32::try_from(artifact.bytes().len())
                        .map_err(|_| "HSACO length overflow")?,
                    object_sha256: hash,
                    symbol: metadata.name().into(),
                },
                false,
                artifact.bytes().to_vec(),
            )?;
            let ResponseV1::LoadedKernel {
                kernel,
                metadata: actual,
            } = response
            else {
                return connection.reject("peer kernel admission response mismatch");
            };
            if kernel == 0
                || !payload.is_empty()
                || self.kernels.values().any(|entry| entry.id == kernel)
                || !metadata_matches(metadata, &actual, hash)
            {
                return connection.reject("peer kernel metadata or identity mismatch");
            }
            self.kernels.insert(
                metadata.name().into(),
                LoadedKernel {
                    id: kernel,
                    metadata: metadata.clone(),
                },
            );
        }
        Ok(())
    }

    pub fn pid(&self) -> u32 {
        self.connection.borrow().child.id()
    }

    fn pack(
        &self,
        connection: &Connection,
        dispatch: &EngineeringTpDispatchV1,
    ) -> TpResult<(CommandV1, Vec<u8>)> {
        for argument in &dispatch.arguments {
            if let EngineeringTpArgumentV1::Buffer { id, access, .. } = argument
                && !connection.ownership.get(id).is_some_and(|&(owner, peers)| {
                    owner == self.rank || peers && *access == EngineeringTpBufferAccessV1::Read
                })
            {
                return Err("peer dispatch attempted foreign or writable peer access".into());
            }
        }
        let kernel = self
            .kernels
            .get(dispatch.kernel)
            .ok_or("peer kernel not admitted")?;
        pack_dispatch(kernel, dispatch, &connection.capacities)
    }

    fn allocate_inner(&mut self, bytes: usize, peers: bool) -> TpResult<u64> {
        let mut connection = self.connection.borrow_mut();
        let (response, payload) = connection.sync(
            self.rank,
            CommandV1::Allocate {
                bytes: u64::try_from(bytes).map_err(|_| "allocation size overflow")?,
            },
            peers,
            vec![],
        )?;
        let ResponseV1::Allocated {
            buffer,
            bytes: actual,
        } = response
        else {
            return connection.reject("peer allocation response mismatch");
        };
        if buffer == 0
            || actual != bytes as u64
            || !payload.is_empty()
            || connection.capacities.contains_key(&buffer)
        {
            return connection.reject("peer allocation identity or extent mismatch");
        }
        connection.capacities.insert(buffer, bytes);
        connection.ownership.insert(buffer, (self.rank, peers));
        Ok(buffer)
    }
}

impl EngineeringTpRankTransportV1 for PeerWorker {
    fn supports_concurrent_rounds(&self) -> bool {
        self.connection.borrow().concurrent_rounds
    }
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        let connection = self.connection.borrow();
        Some((
            connection.child.id(),
            u32::try_from(self.rank).ok()?,
            u32::try_from(connection.pending.len()).ok()?,
        ))
    }
    fn allocate(&mut self, byte_len: usize) -> TpResult<u64> {
        self.allocate_inner(byte_len, false)
    }
    fn allocate_peer_readable(&mut self, byte_len: usize) -> TpResult<u64> {
        self.allocate_inner(byte_len, true)
    }

    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        let mut connection = self.connection.borrow_mut();
        if let Err(error) = connection.check_owned_extent(self.rank, buffer, offset, bytes.len()) {
            return connection.reject(error);
        }
        let (response, payload) = connection.sync(
            self.rank,
            CommandV1::Write {
                buffer,
                offset: offset as u64,
                payload_bytes: u32::try_from(bytes.len()).map_err(|_| "write length overflow")?,
            },
            false,
            bytes.to_vec(),
        )?;
        if !matches!(response, ResponseV1::Written) || !payload.is_empty() {
            return connection.reject("peer write response mismatch");
        }
        Ok(())
    }

    fn read(&mut self, buffer: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        let mut connection = self.connection.borrow_mut();
        if let Err(error) = connection.check_owned_extent(self.rank, buffer, offset, bytes.len()) {
            return connection.reject(error);
        }
        let (response, payload) = connection.sync(
            self.rank,
            CommandV1::Read {
                buffer,
                offset: offset as u64,
                bytes: u32::try_from(bytes.len()).map_err(|_| "read length overflow")?,
            },
            false,
            vec![],
        )?;
        if !matches!(response, ResponseV1::Read { payload_bytes } if payload_bytes as usize == bytes.len())
            || payload.len() != bytes.len()
        {
            return connection.reject("peer read response mismatch");
        }
        bytes.copy_from_slice(&payload);
        Ok(())
    }

    fn submit(&mut self, dispatch: &EngineeringTpDispatchV1) -> TpResult<()> {
        let mut connection = self.connection.borrow_mut();
        if self.pending.is_some() {
            return connection.reject("peer dispatch already pending");
        }
        let (command, payload) = match self.pack(&connection, dispatch) {
            Ok(value) => value,
            Err(error) => return connection.reject(error),
        };
        if connection.concurrent_rounds {
            connection.queue_round(self.rank, command, payload)?;
        } else {
            connection.send(self.rank, command, false, payload)?;
        }
        self.pending = Some(PendingDispatch::Single);
        Ok(())
    }

    fn wait(&mut self) -> TpResult<()> {
        let mut connection = self.connection.borrow_mut();
        if self.pending != Some(PendingDispatch::Single) {
            return connection.reject("peer rank has no pending dispatch");
        }
        let (response, payload) = connection.receive(self.rank)?;
        if !matches!(response, ResponseV1::Dispatched { .. }) || !payload.is_empty() {
            return connection.reject("peer dispatch completion mismatch");
        }
        self.pending = None;
        Ok(())
    }

    fn supports_sequences(&self) -> bool {
        self.sequences
    }

    fn submit_sequence(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        let mut connection = self.connection.borrow_mut();
        if !self.sequences || self.pending.is_some() || !(1..=16).contains(&dispatches.len()) {
            return connection.reject("peer sequence policy, pending state or count mismatch");
        }
        let count = u32::try_from(dispatches.len()).map_err(|_| "peer sequence count overflow")?;
        let mut entries = Vec::with_capacity(dispatches.len());
        let mut payload = Vec::new();
        for dispatch in dispatches {
            let (command, bytes) = match self.pack(&connection, dispatch) {
                Ok(value) => value,
                Err(error) => return connection.reject(error),
            };
            let CommandV1::Dispatch {
                kernel,
                payload_bytes,
                workgroup,
                grid,
                pointers,
                timeout_ms,
            } = command
            else {
                return connection.reject("peer sequence packing drift");
            };
            entries.push(SequenceDispatchV1 {
                kernel,
                payload_bytes,
                workgroup,
                grid,
                pointers,
                timeout_ms: timeout_ms.min(600_000 / count),
            });
            payload.extend_from_slice(&bytes);
        }
        connection.send(
            self.rank,
            CommandV1::DispatchSequence {
                dispatches: entries,
            },
            false,
            payload,
        )?;
        self.pending = Some(PendingDispatch::Sequence(dispatches.len()));
        Ok(())
    }

    fn wait_sequence(&mut self, count: usize) -> TpResult<()> {
        let mut connection = self.connection.borrow_mut();
        if self.pending != Some(PendingDispatch::Sequence(count)) {
            return connection.reject("peer pending sequence count mismatch");
        }
        let (response, payload) = connection.receive(self.rank)?;
        if !matches!(response, ResponseV1::DispatchSequenceCompleted { elapsed_ns } if elapsed_ns.len() == count)
            || !payload.is_empty()
        {
            return connection.reject("peer sequence completion mismatch");
        }
        self.pending = None;
        Ok(())
    }

    fn close(&mut self) -> TpResult<()> {
        let mut connection = self.connection.borrow_mut();
        if connection.exited {
            return Ok(());
        }
        if connection.failed {
            return connection.terminate();
        }
        if connection.pending.iter().any(Option::is_some) {
            return connection.reject("peer close with unacknowledged group work");
        }
        if connection.closed[self.rank] {
            return Ok(());
        }
        connection.closed[self.rank] = true;
        if connection.closed.iter().all(|closed| *closed) {
            connection.closed[0] = false;
            let (response, payload) = connection.sync(0, CommandV1::Close, false, vec![])?;
            connection.closed[0] = true;
            if !matches!(response, ResponseV1::Closed) || !payload.is_empty() {
                return connection.reject("peer group close receipt mismatch");
            }
            if let Err(error) = connection.await_exit(true) {
                return connection.reject(error);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE: &str = r"
import json, os, struct, sys, time
mode = sys.argv[1]
def send(response, payload=b''):
    header = json.dumps(response).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(header)) + header + payload)
    sys.stdout.buffer.flush()
send({'peer_op':'ready','protocol':4,'mode':'device-peer-concurrent-round-v1' if mode.startswith('round') else 'device-peer-serial-v4',
      'target':'gfx950:xnack-','unique_ids':[1,2], 'process_id':os.getpid(), 'authority':'none'})
buffers = {}
while True:
    prefix = sys.stdin.buffer.read(4)
    if not prefix: break
    header = json.loads(sys.stdin.buffer.read(struct.unpack('<I', prefix)[0]))
    command = header['command']
    payload = sys.stdin.buffer.read(sum(entry['payload_bytes'] for entry in command['dispatches'])
        if command['op'] == 'dispatch_sequence' else command.get('payload_bytes',0))
    if mode == 'stall': time.sleep(60)
    op = command['op']
    out = b''
    if op == 'allocate':
        identifier = len(buffers)+1
        buffers[identifier] = bytearray(command['bytes'])
        response = {'op':'allocated','buffer':identifier,'bytes':command['bytes']}
    elif op == 'write':
        buffers[command['buffer']][command['offset']:command['offset']+len(payload)] = payload
        response = {'op':'written'}
    elif op == 'read':
        out = bytes(buffers[command['buffer']][command['offset']:command['offset']+command['bytes']])
        response = {'op':'read','payload_bytes':len(out)}
    elif op == 'dispatch': response = {'op':'dispatched','elapsed_ns':1}
    elif op == 'dispatch_sequence':
        count = len(command['dispatches'])
        response = {'op':'dispatch_sequence_completed','elapsed_ns':[1]*(count-1 if mode == 'short_sequence' else count)}
    elif op == 'configure_performance':
        assert header['rank'] == 0 and not command['profile']
        response = {'op':'written' if mode == 'bad_config' else 'performance_configured'}
    elif op == 'close': response = {'op':'closed'}
    else: response = {'op':'error','message':'unsupported fake command','fatal':True}
    if header.get('round_ranks'):
        ranks = header['round_ranks']
        assert mode.startswith('round')
        assert len(ranks) == len(set(ranks)) == len(command['dispatches'])
        if mode == 'round_short': response['elapsed_ns'].pop()
        if mode == 'round_fatal': response = {'op':'error','message':'late group failure','fatal':True}
        if mode == 'round_wrong_order': ranks = list(reversed(ranks))
        request = header['request'] + (1 if mode == 'round_wrong_request' else 0)
        if mode != 'round_serial_done':
            send({'peer_op':'round_done','request':request,'ranks':ranks,'response':response})
            continue
    rank = 1-header['rank'] if mode == 'wrong' else header['rank']
    send({'peer_op':'done','request':header['request'],'rank':rank,'response':response},out)
    if op == 'close': break
";

    fn fixture(mode: &str) -> Vec<PeerWorker> {
        let child = Command::new("python3")
            .args(["-u", "-c", FAKE, mode])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let connection = Rc::new(RefCell::new(
            Connection::connect_profile(
                child,
                &[1, 2],
                Duration::from_millis(250),
                mode.starts_with("round"),
                HostTiming::default(),
            )
            .unwrap(),
        ));
        (0..2)
            .map(|rank| PeerWorker {
                connection: Rc::clone(&connection),
                rank,
                kernels: BTreeMap::new(),
                pending: None,
                sequences: false,
            })
            .collect()
    }

    fn empty_dispatch() -> CommandV1 {
        CommandV1::Dispatch {
            kernel: 1,
            payload_bytes: 0,
            workgroup: [64, 1, 1],
            grid: [64, 1, 1],
            pointers: vec![],
            timeout_ms: 1000,
        }
    }

    fn queue_fake_round(workers: &mut [PeerWorker], ranks: &[usize]) {
        for &rank in ranks {
            workers[rank]
                .connection
                .borrow_mut()
                .queue_round(rank, empty_dispatch(), vec![])
                .unwrap();
            workers[rank].pending = Some(PendingDispatch::Single);
        }
    }

    #[test]
    fn concurrent_round_has_one_identity_and_releases_only_complete_receipts() {
        let mut workers = fixture("round_normal");
        assert!(
            workers
                .iter()
                .all(EngineeringTpRankTransportV1::supports_concurrent_rounds)
        );
        queue_fake_round(&mut workers, &[0, 1]);
        {
            let connection = workers[0].connection.borrow();
            assert_eq!(connection.pending, vec![Some(1), Some(1)]);
            assert_eq!(connection.queued_round.len(), 2);
            assert!(connection.completed.iter().all(Option::is_none));
        }
        workers[1].wait().unwrap();
        assert!(workers[0].connection.borrow().completed[0].is_some());
        workers[0].wait().unwrap();
        queue_fake_round(&mut workers, &[1]);
        workers[1].wait().unwrap();
        assert_eq!(workers[0].connection.borrow().next_request, 3);
        workers[0].close().unwrap();
        workers[1].close().unwrap();
    }

    #[test]
    fn timing_tickets_cover_serial_and_round_receipts_without_disabled_map_entries() {
        let mut workers = fixture("normal");
        workers[0]
            .connection
            .borrow_mut()
            .send(0, empty_dispatch(), false, vec![])
            .unwrap();
        assert!(workers[0].connection.borrow().tickets.is_empty());
        workers[0].connection.borrow_mut().receive(0).unwrap();
        for worker in &mut workers {
            worker.close().unwrap();
        }

        for mode in ["normal", "round_normal", "round_short"] {
            let mut workers = fixture(mode);
            let timing = HostTiming::enabled();
            workers[0].connection.borrow_mut().timing = timing.clone();
            if mode == "normal" {
                for (rank, worker) in workers.iter().enumerate() {
                    worker
                        .connection
                        .borrow_mut()
                        .send(rank, empty_dispatch(), false, vec![])
                        .unwrap();
                }
                workers[0].connection.borrow_mut().receive(0).unwrap();
                workers[1].connection.borrow_mut().receive(1).unwrap();
            } else {
                queue_fake_round(&mut workers, &[0, 1]);
                assert_eq!(workers[0].wait().is_ok(), mode == "round_normal");
                assert_eq!(workers[1].wait().is_ok(), mode == "round_normal");
            }
            if mode != "round_short" {
                for worker in &mut workers {
                    worker.close().unwrap();
                }
            }
            let snapshot = timing.snapshot();
            assert_eq!(snapshot["incomplete"], false);
            let rows = snapshot["records"].as_array().unwrap();
            let requests = rows
                .iter()
                .filter(|row| {
                    row["category"] == "ipc_roundtrip" && row["dispatches"].as_u64().unwrap() != 0
                })
                .collect::<Vec<_>>();
            assert_eq!(
                requests
                    .iter()
                    .map(|row| row["dispatches"].as_u64().unwrap())
                    .sum::<u64>(),
                2
            );
            if mode == "normal" {
                assert_eq!(requests.len(), 2);
                assert!(
                    requests
                        .iter()
                        .all(|row| row["rank"].is_u64() && row["failed"] == 0)
                );
            } else {
                assert_eq!(requests.len(), 1);
                assert_eq!(requests[0]["label"], "dispatch_round");
                assert!(requests[0]["rank"].is_null());
                assert_eq!(requests[0]["failed"], u64::from(mode == "round_short"));
            }
        }
    }

    #[test]
    fn invalid_round_receipt_never_publishes_partial_success() {
        for mode in [
            "round_short",
            "round_fatal",
            "round_wrong_order",
            "round_wrong_request",
            "round_serial_done",
        ] {
            let mut workers = fixture(mode);
            queue_fake_round(&mut workers, &[0, 1]);
            assert!(workers[0].wait().is_err(), "{mode}");
            assert!(workers[1].wait().is_err(), "{mode}");
            let connection = workers[0].connection.borrow();
            assert!(connection.failed && connection.exited, "{mode}");
            assert!(connection.completed.iter().all(Option::is_none), "{mode}");
        }
    }

    #[test]
    fn pending_round_rejects_duplicate_ranks_and_host_interleaving() {
        let mut workers = fixture("round_normal");
        queue_fake_round(&mut workers, &[0]);
        assert!(
            workers[0]
                .connection
                .borrow_mut()
                .queue_round(0, empty_dispatch(), vec![])
                .is_err()
        );
        let mut workers = fixture("round_normal");
        queue_fake_round(&mut workers, &[1]);
        assert!(workers[0].allocate(32).is_err());
        let mut workers = fixture("round_normal");
        queue_fake_round(&mut workers, &[0, 1]);
        workers[0].wait().unwrap();
        assert!(
            workers[0]
                .connection
                .borrow_mut()
                .queue_round(0, empty_dispatch(), vec![])
                .is_err()
        );
    }

    #[test]
    fn group_ids_are_global_host_access_is_owned_and_close_waits_for_every_rank() {
        let mut workers = fixture("normal");
        assert_eq!(workers[0].pid(), workers[1].pid());
        let first = workers[0].allocate_peer_readable(32).unwrap();
        let second = workers[1].allocate(32).unwrap();
        assert_ne!(first, second);
        workers[0].write(first, 3, &[4, 5, 6]).unwrap();
        let mut bytes = [0; 3];
        workers[0].read(first, 3, &mut bytes).unwrap();
        assert_eq!(bytes, [4, 5, 6]);
        assert_eq!(workers[0].peer_group_rank().unwrap().1, 0);
        assert_eq!(workers[1].peer_group_rank().unwrap().1, 1);
        workers[0].close().unwrap();
        assert!(!workers[0].connection.borrow().exited);
        workers[1].close().unwrap();
        assert!(workers[0].connection.borrow().exited);
    }

    #[test]
    fn rank_completions_can_be_consumed_in_reverse_without_identity_confusion() {
        let mut workers = fixture("normal");
        for (rank, worker) in workers.iter_mut().enumerate() {
            worker
                .connection
                .borrow_mut()
                .send(rank, empty_dispatch(), false, vec![])
                .unwrap();
            worker.pending = Some(PendingDispatch::Single);
        }
        workers[1].wait().unwrap();
        workers[0].wait().unwrap();
        for worker in &mut workers {
            worker.close().unwrap();
        }
    }

    #[test]
    fn sequence_completion_is_exact_and_partial_receipts_poison_every_rank() {
        for mode in ["normal", "short_sequence"] {
            let mut workers = fixture(mode);
            let entries = (0..2)
                .map(|_| SequenceDispatchV1 {
                    kernel: 1,
                    payload_bytes: 0,
                    workgroup: [64, 1, 1],
                    grid: [64, 1, 1],
                    pointers: vec![],
                    timeout_ms: 1000,
                })
                .collect();
            workers[0]
                .connection
                .borrow_mut()
                .send(
                    0,
                    CommandV1::DispatchSequence {
                        dispatches: entries,
                    },
                    false,
                    vec![],
                )
                .unwrap();
            workers[0].pending = Some(PendingDispatch::Sequence(2));
            assert_eq!(workers[0].wait_sequence(2).is_ok(), mode == "normal");
            if mode == "normal" {
                for worker in &mut workers {
                    worker.close().unwrap();
                }
            } else {
                assert!(workers[0].connection.borrow().failed);
                assert!(workers[1].allocate(4).is_err());
            }
        }
    }

    #[test]
    fn sequence_opt_in_and_pending_kind_are_not_silently_ignored() {
        let mut workers = fixture("normal");
        assert!(!workers[0].supports_sequences());
        assert!(workers[0].submit_sequence(&[]).is_err());
        assert!(workers[1].connection.borrow().exited);
        let mut workers = fixture("normal");
        workers[0]
            .connection
            .borrow_mut()
            .send(0, empty_dispatch(), false, vec![])
            .unwrap();
        workers[0].pending = Some(PendingDispatch::Single);
        assert!(workers[0].wait_sequence(1).is_err());
        assert!(workers[1].connection.borrow().failed);
    }

    #[test]
    fn configuration_requires_exact_receipt_and_rollover_rejects_before_spawn() {
        for mode in ["normal", "bad_config"] {
            let mut workers = fixture(mode);
            let configured = workers[0]
                .connection
                .borrow_mut()
                .configure_performance(true, true);
            assert_eq!(configured.is_ok(), mode == "normal");
            if mode == "normal" {
                for worker in &mut workers {
                    worker.close().unwrap();
                }
            } else {
                assert!(workers[1].connection.borrow().exited);
            }
        }
        let result = PeerWorker::spawn_with_timing(
            Path::new("/nonexistent-peer-worker"),
            &[1, 2],
            &[],
            RuntimeOptions {
                rollover: true,
                ..RuntimeOptions::default()
            },
            false,
            HostTiming::default(),
        );
        assert!(
            matches!(result, Err(error) if error == "peer rollover and concurrent same-rank sequences are unsupported")
        );
    }

    #[test]
    fn wrong_rank_timeout_foreign_host_access_and_pending_lifecycle_poison_whole_child() {
        for mode in ["wrong", "stall", "foreign", "pending"] {
            let mut workers = fixture(if matches!(mode, "wrong" | "stall") {
                mode
            } else {
                "normal"
            });
            let failed = match mode {
                "wrong" | "stall" => workers[0].allocate(4).is_err(),
                "foreign" => {
                    let id = workers[0].allocate_peer_readable(4).unwrap();
                    workers[1].read(id, 0, &mut [0; 4]).is_err()
                }
                "pending" => {
                    workers[0]
                        .connection
                        .borrow_mut()
                        .send(0, empty_dispatch(), false, vec![])
                        .unwrap();
                    workers[1].allocate(4).is_err()
                }
                _ => unreachable!(),
            };
            assert!(failed, "{mode}");
            assert!(workers[0].connection.borrow().failed);
            assert!(workers[0].connection.borrow().exited);
            assert!(workers[1].allocate(4).is_err());
        }
    }
}
