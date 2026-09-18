//! Owned-byte IPC to a separately opted-in, non-authoritative KFD process.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use fe2o3_hsaco::{
    ArgumentAccess, ExplicitArgument, ExplicitValueKind, ExplicitValueType, InspectedKernel,
};
use fe2o3_kfd::engineering_wire::{
    self as wire, BufferAccessV1, CommandV1, KernelMetadataV1, OrderedBatchDispatchV1,
    PointerFixupV1, ResponseV1, SequenceDispatchV1,
};
use ferric_m1_engineering_execution_v1::host_timing::{HostTiming, IpcTiming};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpArgumentV1, EngineeringTpBufferAccessV1, EngineeringTpDispatchV1,
    EngineeringTpRankTransportV1, TpResult,
};
use sha2::{Digest, Sha256};

const OP_TIMEOUT: Duration = Duration::from_mins(2);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const DISPATCH_TIMEOUT_MS: u32 = 60_000;

#[derive(Clone, Copy, Default)]
#[allow(clippy::struct_excessive_bools)] // Independent ablation flags, not lifecycle state.
pub struct RuntimeOptions {
    pub cache_admission: bool,
    pub operational: bool,
    pub sequences: bool,
    pub ordered_batches: bool,
    pub full_forward: bool,
    pub rollover: bool,
    pub shared_full_currentness: bool,
    pub profile: bool,
}

struct Outgoing {
    header: CommandV1,
    payload: Vec<u8>,
}

struct Incoming {
    header: ResponseV1,
    payload: Vec<u8>,
}

pub(super) struct LoadedKernel {
    pub(super) id: u64,
    pub(super) image: [u8; 32],
    pub(super) metadata: InspectedKernel,
}

#[derive(Eq, PartialEq)]
enum PendingRequest {
    Other,
    Dispatch,
    Sequence(usize),
    OrderedBatch(usize),
    FullForward(usize),
}

struct WorkerTiming {
    timing: HostTiming,
    rank: u32,
    pending: Option<IpcTiming>,
}

pub struct Worker {
    child: Child,
    writer: Option<SyncSender<Outgoing>>,
    written: Receiver<TpResult<()>>,
    reader: Receiver<TpResult<Incoming>>,
    io_threads: Vec<JoinHandle<()>>,
    buffers: BTreeMap<u64, usize>,
    kernels: BTreeMap<String, LoadedKernel>,
    pending: Option<PendingRequest>,
    failed: bool,
    exited: bool,
    timeout: Duration,
    options: RuntimeOptions,
    queue_epoch: u64,
    queue_packets: u64,
    timing: Option<Box<WorkerTiming>>,
    diagnostic_identity: (u64, u32),
    diagnostic_snapshots: u32,
    last_diagnostic: Option<Box<wire::PerformanceCountersV1>>,
}

impl Worker {
    // Shared with the legacy token-at-a-time executable.
    #[allow(dead_code)]
    pub fn spawn(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
    ) -> TpResult<Self> {
        Self::spawn_with_options(executable, unique_id, artifact, RuntimeOptions::default())
    }

    pub fn spawn_with_options(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
    ) -> TpResult<Self> {
        Self::spawn_with_timing(
            executable,
            unique_id,
            artifact,
            options,
            HostTiming::default(),
            0,
        )
    }

    pub fn spawn_with_timing(
        executable: &Path,
        unique_id: u64,
        artifact: &EngineeringTpArtifactV1,
        options: RuntimeOptions,
        timing: HostTiming,
        rank: u32,
    ) -> TpResult<Self> {
        if options.shared_full_currentness {
            return Err("shared full currentness requires the explicit peer worker".into());
        }
        if options.sequences && options.ordered_batches {
            return Err("ordered batches and legacy dispatch sequences are distinct modes".into());
        }
        if options.full_forward
            && (options.sequences || options.ordered_batches || options.rollover || options.profile)
        {
            return Err("full-forward submission excludes sequences, ordered batches, rollover and profiling".into());
        }
        let child = Command::new(executable)
            .arg("--device-unique-id")
            .arg(unique_id.to_string())
            .arg("--allow-unauthenticated-machine-code")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("spawn GPU worker: {error}"))?;
        let mut worker = Self::connect_with_timing(child, unique_id, OP_TIMEOUT, timing, rank)?;
        worker.configure_options(options)?;
        worker.load_artifact(artifact)?;
        Ok(worker)
    }

    fn configure_options(&mut self, options: RuntimeOptions) -> TpResult<()> {
        if options.sequences && options.ordered_batches {
            return self.reject("ordered batches and legacy dispatch sequences are incompatible");
        }
        if options.full_forward
            && (options.sequences
                || options.ordered_batches
                || options.rollover
                || options.profile
                || options.shared_full_currentness)
        {
            return self.reject("full-forward submission has incompatible runtime options");
        }
        self.options = options;
        if options.cache_admission || options.operational || options.profile {
            self.send(
                CommandV1::ConfigurePerformance {
                    cache_kernel_admission: options.cache_admission,
                    operational_currentness: options.operational,
                    profile: options.profile,
                },
                vec![],
            )?;
            if !matches!(self.receive()?.header, ResponseV1::PerformanceConfigured) {
                return self.reject("runtime performance configuration was not acknowledged");
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn connect(child: Child, unique_id: u64, timeout: Duration) -> TpResult<Self> {
        Self::connect_with_timing(child, unique_id, timeout, HostTiming::default(), 0)
    }

    fn connect_with_timing(
        mut child: Child,
        unique_id: u64,
        timeout: Duration,
        timing: HostTiming,
        timing_rank: u32,
    ) -> TpResult<Self> {
        let Some(mut input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("worker stdin missing".into());
        };
        let Some(mut output) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("worker stdout missing".into());
        };
        let (writer, pending_frames) = mpsc::sync_channel::<Outgoing>(1);
        let (acknowledge, written) = mpsc::sync_channel(1);
        let (responses, reader) = mpsc::sync_channel(1);
        // Both pipe directions have independent bounded queues. The controller
        // never blocks in a pipe write when a child stops consuming input.
        let write_thread = thread::spawn(move || {
            while let Ok(frame) = pending_frames.recv() {
                let result = (|| {
                    wire::write_header_v1(&mut input, &frame.header)?;
                    input.write_all(&frame.payload)?;
                    input.flush()
                })()
                .map_err(|error: std::io::Error| format!("worker pipe write: {error}"));
                let failed = result.is_err();
                if acknowledge.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let read_thread = thread::spawn(move || {
            loop {
                let result = read_response(&mut output);
                let failed = result.is_err();
                if responses.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let mut worker = Self {
            child,
            writer: Some(writer),
            written,
            reader,
            io_threads: vec![write_thread, read_thread],
            buffers: BTreeMap::new(),
            kernels: BTreeMap::new(),
            pending: Some(PendingRequest::Other),
            failed: false,
            exited: false,
            timeout,
            options: RuntimeOptions::default(),
            queue_epoch: 0,
            queue_packets: 0,
            diagnostic_identity: (unique_id, timing_rank),
            diagnostic_snapshots: 0,
            last_diagnostic: None,
            timing: timing.is_enabled().then(|| {
                Box::new(WorkerTiming {
                    timing,
                    rank: timing_rank,
                    pending: None,
                })
            }),
        };
        let ready = worker.receive()?;
        if !matches!(ready.header, ResponseV1::Ready { protocol, target, device_unique_id, authority }
            if protocol == wire::PROTOCOL_VERSION_V1 && target == wire::TARGET_V1
                && device_unique_id == unique_id && authority == "none")
        {
            return worker.reject("worker ready identity mismatch");
        }
        Ok(worker)
    }

    /// Adds a disjoint admitted image only before any buffer allocation or dispatch.
    #[allow(dead_code)] // The legacy single-sequence controller shares this module.
    pub fn load_additional_artifact(&mut self, artifact: &EngineeringTpArtifactV1) -> TpResult<()> {
        let names = artifact
            .inspection()
            .hsaco()
            .kernels()
            .iter()
            .map(InspectedKernel::name)
            .collect::<Vec<_>>();
        self.with_additional_image(&names, |worker| worker.load_artifact(artifact))
    }

    fn with_additional_image(
        &mut self,
        names: &[&str],
        load: impl FnOnce(&mut Self) -> TpResult<()>,
    ) -> TpResult<()> {
        if self.failed
            || self.exited
            || self.pending.is_some()
            || !self.buffers.is_empty()
            || self.queue_packets != 0
            || self.queue_epoch != 0
            || names.is_empty()
            || names.iter().any(|name| self.kernels.contains_key(*name))
            || names
                .iter()
                .enumerate()
                .any(|(index, name)| names[..index].contains(name))
        {
            return Err(
                "additional artifact requires a fresh worker and disjoint kernel symbols".into(),
            );
        }
        let result = load(self);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn load_artifact(&mut self, artifact: &EngineeringTpArtifactV1) -> TpResult<()> {
        let hash: [u8; 32] = Sha256::digest(artifact.bytes()).into();
        for metadata in artifact.inspection().hsaco().kernels() {
            let length =
                u32::try_from(artifact.bytes().len()).map_err(|_| "HSACO length overflow")?;
            self.send(
                CommandV1::LoadKernel {
                    payload_bytes: length,
                    object_sha256: hash,
                    symbol: metadata.name().into(),
                },
                artifact.bytes().to_vec(),
            )?;
            let response = self.receive()?;
            let ResponseV1::LoadedKernel {
                kernel,
                metadata: actual,
            } = response.header
            else {
                return self.reject("worker did not load kernel");
            };
            if kernel == 0
                || self.kernels.values().any(|loaded| loaded.id == kernel)
                || !metadata_matches(metadata, &actual, hash)
            {
                return self.reject("worker kernel metadata or identity mismatch");
            }
            self.kernels.insert(
                metadata.name().into(),
                LoadedKernel {
                    id: kernel,
                    image: hash,
                    metadata: metadata.clone(),
                },
            );
        }
        Ok(())
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    fn reject<T>(&mut self, message: impl Into<String>) -> TpResult<T> {
        self.failed = true;
        if let Some(timing) = &mut self.timing {
            timing.pending.take();
        }
        let message = message.into();
        let cleanup = self.terminate();
        Err(match cleanup {
            Ok(()) => message,
            Err(error) => format!("{message}; {error}"),
        })
    }

    fn send(&mut self, header: CommandV1, payload: Vec<u8>) -> TpResult<()> {
        if self.failed || self.exited || self.pending.is_some() {
            return self.reject("worker is not ready for a request");
        }
        if header.payload_bytes().map_err(|error| error.to_string())? != payload.len() {
            return self.reject("outgoing payload length mismatch");
        }
        let (command, dispatches) = match &header {
            CommandV1::Dispatch { .. } => ("dispatch", 1),
            CommandV1::DispatchSequence { dispatches } => {
                ("dispatch_sequence", dispatches.len() as u64)
            }
            CommandV1::DispatchOrderedBatch { dispatches, .. } => {
                ("dispatch_ordered_batch", dispatches.len() as u64)
            }
            CommandV1::DispatchFullForward { dispatch_count, .. } => {
                ("dispatch_full_forward", u64::from(*dispatch_count))
            }
            CommandV1::Allocate { .. } => ("allocate", 0),
            CommandV1::Write { .. } => ("write", 0),
            CommandV1::Read { .. } => ("read", 0),
            CommandV1::LoadKernel { .. } => ("load_kernel", 0),
            CommandV1::ConfigurePerformance { .. } => ("configure_performance", 0),
            CommandV1::RolloverQueue { .. } => ("rollover_queue", 0),
            CommandV1::Close => ("close", 0),
            _ => ("other", 0),
        };
        if let Some(timing) = &mut self.timing {
            timing.pending = Some(timing.timing.request(
                Some(timing.rank),
                command,
                payload.len(),
                dispatches,
            ));
        }
        self.pending = Some(match &header {
            CommandV1::Dispatch { .. } => PendingRequest::Dispatch,
            CommandV1::DispatchSequence { dispatches } => {
                PendingRequest::Sequence(dispatches.len())
            }
            CommandV1::DispatchOrderedBatch { dispatches, .. } => {
                PendingRequest::OrderedBatch(dispatches.len())
            }
            CommandV1::DispatchFullForward { dispatch_count, .. } => {
                PendingRequest::FullForward(*dispatch_count as usize)
            }
            _ => PendingRequest::Other,
        });
        let Some(writer) = &self.writer else {
            return self.reject("worker writer closed");
        };
        if writer.try_send(Outgoing { header, payload }).is_err() {
            return self.reject("worker writer unavailable");
        }
        let written = self.written.recv_timeout(self.timeout);
        if let Some(ticket) = self
            .timing
            .as_mut()
            .and_then(|timing| timing.pending.as_mut())
        {
            ticket.sent(matches!(&written, Ok(Ok(()))));
        }
        match written {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => self.reject(error),
            Err(error) => self.reject(format!("worker write deadline/disconnect: {error}")),
        }
    }

    fn receive(&mut self) -> TpResult<Incoming> {
        let _timing = self
            .timing
            .as_ref()
            .map(|timing| timing.timing.span("ipc_response_wait", Some(timing.rank)));
        if self.pending.is_none() || self.failed || self.exited {
            return self.reject("worker has no admissible pending response");
        }
        match self.reader.recv_timeout(self.timeout) {
            Ok(Ok(Incoming {
                header: ResponseV1::Error { message, fatal },
                ..
            })) => self.reject(format!("GPU worker failed (fatal={fatal}): {message}")),
            Ok(Ok(response)) => {
                self.pending = None;
                if let Some(ticket) = self
                    .timing
                    .as_mut()
                    .and_then(|timing| timing.pending.take())
                {
                    ticket.finish(response.payload.len(), true);
                }
                Ok(response)
            }
            Ok(Err(error)) => self.reject(error),
            Err(error) => self.reject(format!("worker response deadline/disconnect: {error}")),
        }
    }

    fn extent(&self, buffer: u64, offset: usize, bytes: usize) -> TpResult<()> {
        let capacity = self.buffers.get(&buffer).ok_or("unknown owned buffer")?;
        let end = offset.checked_add(bytes).ok_or("buffer extent overflow")?;
        if offset > *capacity || end > *capacity {
            return Err("buffer extent outside allocation".into());
        }
        Ok(())
    }

    fn terminate(&mut self) -> TpResult<()> {
        self.writer.take();
        if self.exited {
            return Ok(());
        }
        if self
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_none()
        {
            self.child
                .kill()
                .map_err(|error| format!("cannot kill worker {}: {error}", self.pid()))?;
        }
        self.await_exit(false)
    }

    fn await_exit(&mut self, successful: bool) -> TpResult<()> {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().map_err(|error| error.to_string())? {
                self.exited = true;
                self.writer.take();
                while self.io_threads.iter().any(|thread| !thread.is_finished()) {
                    if start.elapsed() >= EXIT_TIMEOUT {
                        return Err(format!(
                            "worker {} exited but IPC threads did not stop",
                            self.pid()
                        ));
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                for handle in self.io_threads.drain(..) {
                    if handle.is_finished() {
                        let _ = handle.join();
                    }
                }
                return if successful && !status.success() {
                    Err(format!("worker exited with {status}"))
                } else {
                    Ok(())
                };
            }
            if start.elapsed() >= EXIT_TIMEOUT {
                return Err(format!(
                    "worker {} did not exit; resources are not confirmed released",
                    self.pid()
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl EngineeringTpRankTransportV1 for Worker {
    fn require_loaded_image(&mut self, image: [u8; 32], kernels: &[&str]) -> TpResult<()> {
        if self.failed
            || self.exited
            || self.pending.is_some()
            || !self.buffers.is_empty()
            || self.queue_packets != 0
            || self.queue_epoch != 0
            || image == [0; 32]
            || kernels.is_empty()
            || kernels.len() > 32
            || kernels.iter().enumerate().any(|(index, name)| {
                kernels[..index].contains(name)
                    || self
                        .kernels
                        .get(*name)
                        .is_none_or(|loaded| loaded.image != image)
            })
        {
            return self.reject("additional image binding is not fresh or complete");
        }
        Ok(())
    }
    fn runtime_diagnostic_snapshot(&mut self) -> TpResult<serde_json::Value> {
        if !self.options.profile
            || self.failed
            || self.exited
            || self.pending.is_some()
            || self.diagnostic_snapshots >= 2
        {
            return self.reject("runtime diagnostic snapshot is disabled or unavailable");
        }
        self.send(CommandV1::PerformanceSnapshot, vec![])?;
        let incoming = self.receive()?;
        let ResponseV1::PerformanceSnapshot { counters } = incoming.header else {
            return self.reject("runtime diagnostic snapshot response mismatch");
        };
        if !incoming.payload.is_empty() {
            return self.reject("runtime diagnostic snapshot has an unexpected payload");
        }
        let values = serde_json::to_value(&counters).map_err(|error| error.to_string())?;
        if let Some(previous) = &self.last_diagnostic {
            let before = serde_json::to_value(previous).map_err(|error| error.to_string())?;
            if before.as_object().is_none_or(|object| {
                object.iter().any(|(key, value)| {
                    values.get(key).and_then(serde_json::Value::as_u64) < value.as_u64()
                })
            }) {
                return self.reject("runtime diagnostic counters regressed");
            }
        }
        let ordinal = self.diagnostic_snapshots;
        self.diagnostic_snapshots += 1;
        self.last_diagnostic = Some(Box::new(counters));
        Ok(serde_json::json!({
            "schema": "FerricRuntimeDiagnosticSnapshotV1",
            "authority": "none",
            "performance_qualified": false,
            "scope": "cumulative overlapping worker host-wall counters, not GPU timestamps",
            "process_id": self.child.id(),
            "device_unique_id": self.diagnostic_identity.0,
            "rank": self.diagnostic_identity.1,
            "ordinal": ordinal,
            "counters": values,
        }))
    }
    fn allocate(&mut self, byte_len: usize) -> TpResult<u64> {
        if byte_len == 0 {
            return self.reject("zero allocation");
        }
        self.send(
            CommandV1::Allocate {
                bytes: byte_len as u64,
            },
            vec![],
        )?;
        let response = self.receive()?;
        match response.header {
            ResponseV1::Allocated { buffer, bytes }
                if buffer != 0
                    && bytes == byte_len as u64
                    && !self.buffers.contains_key(&buffer) =>
            {
                self.buffers.insert(buffer, byte_len);
                Ok(buffer)
            }
            _ => self.reject("allocation response mismatch"),
        }
    }

    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        self.extent(buffer, offset, bytes.len())?;
        for (index, chunk) in bytes
            .chunks(wire::MAX_TRANSFER_BYTES_V1 as usize)
            .enumerate()
        {
            let position = offset + index * wire::MAX_TRANSFER_BYTES_V1 as usize;
            self.send(
                CommandV1::Write {
                    buffer,
                    offset: position as u64,
                    payload_bytes: u32::try_from(chunk.len()).map_err(|_| "write chunk length")?,
                },
                chunk.to_vec(),
            )?;
            if !matches!(self.receive()?.header, ResponseV1::Written) {
                return self.reject("write response mismatch");
            }
        }
        Ok(())
    }

    fn read(&mut self, buffer: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()> {
        self.extent(buffer, offset, bytes.len())?;
        for (index, chunk) in bytes
            .chunks_mut(wire::MAX_TRANSFER_BYTES_V1 as usize)
            .enumerate()
        {
            let position = offset + index * wire::MAX_TRANSFER_BYTES_V1 as usize;
            self.send(
                CommandV1::Read {
                    buffer,
                    offset: position as u64,
                    bytes: u32::try_from(chunk.len()).map_err(|_| "read chunk length")?,
                },
                vec![],
            )?;
            let response = self.receive()?;
            if !matches!(response.header, ResponseV1::Read { payload_bytes } if payload_bytes as usize == chunk.len())
                || response.payload.len() != chunk.len()
            {
                return self.reject("read response mismatch");
            }
            chunk.copy_from_slice(&response.payload);
        }
        Ok(())
    }

    fn submit(&mut self, dispatch: &EngineeringTpDispatchV1) -> TpResult<()> {
        let loaded = self.kernels.get(dispatch.kernel).ok_or("unloaded kernel")?;
        let (header, bytes) = pack_dispatch(loaded, dispatch, &self.buffers)?;
        self.send(header, bytes)
    }

    fn wait(&mut self) -> TpResult<()> {
        if self.pending != Some(PendingRequest::Dispatch) {
            return self.reject("no pending GPU dispatch");
        }
        if !matches!(self.receive()?.header, ResponseV1::Dispatched { .. }) {
            return self.reject("dispatch completion response mismatch");
        }
        self.queue_packets = self
            .queue_packets
            .checked_add(1)
            .ok_or("queue packet overflow")?;
        Ok(())
    }

    fn supports_sequences(&self) -> bool {
        self.options.sequences
    }

    fn submit_sequence(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        if !self.options.sequences || !(1..=16).contains(&dispatches.len()) {
            return self.reject("dispatch sequence policy or bound mismatch");
        }
        let mut entries = Vec::with_capacity(dispatches.len());
        let mut payload = Vec::new();
        for dispatch in dispatches {
            let loaded = self
                .kernels
                .get(dispatch.kernel)
                .ok_or("unloaded sequence kernel")?;
            let (header, bytes) = pack_dispatch(loaded, dispatch, &self.buffers)?;
            let CommandV1::Dispatch {
                kernel,
                payload_bytes,
                workgroup,
                grid,
                pointers,
                timeout_ms,
            } = header
            else {
                return self.reject("sequence dispatch packing drifted");
            };
            let timeout_ms = timeout_ms.min(
                600_000
                    / u32::try_from(dispatches.len()).map_err(|_| "sequence length overflow")?,
            );
            entries.push(SequenceDispatchV1 {
                kernel,
                payload_bytes,
                workgroup,
                grid,
                pointers,
                timeout_ms,
            });
            payload.extend_from_slice(&bytes);
        }
        self.send(
            CommandV1::DispatchSequence {
                dispatches: entries,
            },
            payload,
        )
    }

    fn wait_sequence(&mut self, count: usize) -> TpResult<()> {
        if self.pending != Some(PendingRequest::Sequence(count)) {
            return self.reject("pending sequence count mismatch");
        }
        match self.receive()?.header {
            ResponseV1::DispatchSequenceCompleted { elapsed_ns } if elapsed_ns.len() == count => {
                self.queue_packets = self
                    .queue_packets
                    .checked_add(count as u64)
                    .ok_or("queue packet overflow")?;
                Ok(())
            }
            response => self.reject(format!(
                "dispatch sequence failed or completion drifted: {response:?}"
            )),
        }
    }

    fn supports_ordered_batches(&self) -> bool {
        self.options.ordered_batches
    }

    fn submit_ordered_batch(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        if !self.options.ordered_batches
            || self.options.sequences
            || !(1..=wire::MAX_ORDERED_BATCH_DISPATCHES_V1).contains(&dispatches.len())
            || self
                .queue_packets
                .checked_add(dispatches.len() as u64)
                .is_none_or(|end| end > wire::MAX_UNRETIRED_RING_PACKETS_V1)
        {
            return self.reject("ordered batch policy, count or packet budget mismatch");
        }
        // Every existing metadata, ownership and scalar pack check precedes publication.
        let prepared = (|| -> TpResult<_> {
            let mut entries = Vec::with_capacity(dispatches.len());
            let mut payload = Vec::new();
            for dispatch in dispatches {
                let loaded = self
                    .kernels
                    .get(dispatch.kernel)
                    .ok_or("unloaded ordered batch kernel")?;
                let (header, bytes) = pack_dispatch(loaded, dispatch, &self.buffers)?;
                let CommandV1::Dispatch {
                    kernel,
                    payload_bytes,
                    workgroup,
                    grid,
                    pointers,
                    ..
                } = header
                else {
                    return Err("ordered batch dispatch packing drifted".into());
                };
                entries.push(OrderedBatchDispatchV1 {
                    kernel,
                    payload_bytes,
                    workgroup,
                    grid,
                    pointers,
                });
                payload.extend_from_slice(&bytes);
            }
            let header = CommandV1::DispatchOrderedBatch {
                dispatches: entries,
                timeout_ms: DISPATCH_TIMEOUT_MS,
            };
            if header.payload_bytes().map_err(|error| error.to_string())? != payload.len() {
                return Err("ordered batch cumulative payload mismatch".into());
            }
            Ok((header, payload))
        })();
        let (header, payload) = match prepared {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        self.send(header, payload)
    }

    fn wait_ordered_batch(&mut self, count: usize) -> TpResult<()> {
        if !(1..=wire::MAX_ORDERED_BATCH_DISPATCHES_V1).contains(&count)
            || self.pending != Some(PendingRequest::OrderedBatch(count))
        {
            return self.reject("pending ordered batch count mismatch");
        }
        let next = match self.queue_packets.checked_add(count as u64) {
            Some(value) if value <= wire::MAX_UNRETIRED_RING_PACKETS_V1 => value,
            _ => return self.reject("ordered batch completed packet count overflow"),
        };
        let response = self.receive()?;
        if !response.payload.is_empty()
            || !matches!(response.header, ResponseV1::DispatchOrderedBatchCompleted {
                completed_dispatches, ..
            } if completed_dispatches as usize == count)
        {
            return self.reject("ordered batch failed or aggregate completion drifted");
        }
        self.queue_packets = next;
        Ok(())
    }

    fn supports_full_forward(&self) -> bool {
        self.options.full_forward
    }

    fn submit_full_forward(&mut self, dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        if !self.options.full_forward
            || self.options.sequences
            || self.options.ordered_batches
            || self.options.rollover
            || self.options.profile
            || self.options.shared_full_currentness
            || dispatches.len() != wire::FULL_FORWARD_DISPATCHES_V1
            || self
                .queue_packets
                .checked_add(dispatches.len() as u64)
                .is_none_or(|end| end > wire::MAX_UNRETIRED_RING_PACKETS_V1)
        {
            return self.reject("full-forward policy, count or packet budget mismatch");
        }
        // Validate and pack the entire forward before publishing any command.
        let prepared = (|| -> TpResult<_> {
            let mut entries = Vec::with_capacity(wire::FULL_FORWARD_DISPATCHES_V1);
            let mut kernargs = Vec::new();
            for dispatch in dispatches {
                let loaded = self
                    .kernels
                    .get(dispatch.kernel)
                    .ok_or("unloaded full-forward kernel")?;
                let (header, bytes) = pack_dispatch(loaded, dispatch, &self.buffers)?;
                let CommandV1::Dispatch {
                    kernel,
                    payload_bytes,
                    workgroup,
                    grid,
                    pointers,
                    ..
                } = header
                else {
                    return Err("full-forward dispatch packing drifted".into());
                };
                let length = kernargs
                    .len()
                    .checked_add(bytes.len())
                    .ok_or("full-forward kernarg length overflow")?;
                if length > wire::MAX_FULL_FORWARD_KERNARG_BYTES_V1 as usize {
                    return Err("full-forward kernargs exceed payload bound".into());
                }
                entries.push(OrderedBatchDispatchV1 {
                    kernel,
                    payload_bytes,
                    workgroup,
                    grid,
                    pointers,
                });
                kernargs.extend_from_slice(&bytes);
            }
            let payload = wire::encode_full_forward_payload_v1(&entries, &kernargs)
                .map_err(|error| format!("full-forward payload: {error}"))?;
            Ok((
                CommandV1::DispatchFullForward {
                    dispatch_count: u32::try_from(dispatches.len())
                        .map_err(|_| "full-forward dispatch count overflow")?,
                    plan_bytes: payload.plan_bytes,
                    kernarg_bytes: payload.kernarg_bytes,
                    timeout_ms: DISPATCH_TIMEOUT_MS,
                },
                payload.bytes,
            ))
        })();
        let (header, payload) = match prepared {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        self.send(header, payload)
    }

    fn wait_full_forward(&mut self, count: usize) -> TpResult<()> {
        if count != wire::FULL_FORWARD_DISPATCHES_V1
            || self.pending != Some(PendingRequest::FullForward(count))
        {
            return self.reject("pending full-forward count mismatch");
        }
        let next = match self.queue_packets.checked_add(count as u64) {
            Some(value) if value <= wire::MAX_UNRETIRED_RING_PACKETS_V1 => value,
            _ => return self.reject("full-forward completed packet count overflow"),
        };
        let response = self.receive()?;
        if !response.payload.is_empty()
            || !matches!(response.header, ResponseV1::DispatchFullForwardCompleted {
                completed_dispatches, ..
            } if completed_dispatches as usize == count)
        {
            return self.reject("full-forward failed or aggregate completion drifted");
        }
        self.queue_packets = next;
        Ok(())
    }

    fn supports_queue_rollover(&self) -> bool {
        self.options.rollover
    }

    fn prepare_packets(&mut self, count: u64) -> TpResult<()> {
        let limit = wire::MAX_UNRETIRED_RING_PACKETS_V1;
        if count == 0 || count > limit {
            return self.reject("invalid next packet reservation");
        }
        if self
            .queue_packets
            .checked_add(count)
            .is_some_and(|end| end <= limit)
        {
            return Ok(());
        }
        if !self.options.rollover {
            return self.reject("queue requires explicitly enabled rollover");
        }
        let next = self
            .queue_epoch
            .checked_add(1)
            .ok_or("queue epoch overflow")?;
        self.send(
            CommandV1::RolloverQueue {
                expected_epoch: self.queue_epoch,
                expected_completed_packets: self.queue_packets,
            },
            vec![],
        )?;
        if !matches!(self.receive()?.header, ResponseV1::QueueRolledOver { retired_packets, queue_epoch }
            if retired_packets == self.queue_packets && queue_epoch == next)
        {
            return self.reject("queue rollover receipt mismatch");
        }
        self.queue_epoch = next;
        self.queue_packets = 0;
        Ok(())
    }

    fn close(&mut self) -> TpResult<()> {
        if self.exited {
            return Ok(());
        }
        if self.failed {
            return self.terminate();
        }
        if self.pending == Some(PendingRequest::Dispatch) {
            self.wait()?;
        } else if let Some(PendingRequest::Sequence(count)) = self.pending {
            self.wait_sequence(count)?;
        } else if let Some(PendingRequest::OrderedBatch(count)) = self.pending {
            self.wait_ordered_batch(count)?;
        } else if let Some(PendingRequest::FullForward(count)) = self.pending {
            self.wait_full_forward(count)?;
        }
        self.send(CommandV1::Close, vec![])?;
        if !matches!(self.receive()?.header, ResponseV1::Closed) {
            return self.reject("close response mismatch");
        }
        let result = self.await_exit(true);
        if result.is_err() {
            let _ = self.terminate();
        }
        result
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if !self.exited
            && let Err(error) = self.terminate()
        {
            eprintln!("GPU worker cleanup: {error}");
        }
    }
}

#[cfg(test)]
#[path = "tp_worker_diagnostics_tests.rs"]
mod diagnostic_tests;

fn read_response(input: &mut impl Read) -> TpResult<Incoming> {
    let header = wire::read_header_v1(input)
        .map_err(|error| format!("worker pipe header: {error}"))?
        .ok_or("worker closed its response pipe")?;
    let length = match &header {
        ResponseV1::Read { payload_bytes } if *payload_bytes <= wire::MAX_TRANSFER_BYTES_V1 => {
            *payload_bytes as usize
        }
        ResponseV1::Read { .. } => return Err("oversized worker payload".into()),
        _ => 0,
    };
    let mut payload = vec![0; length];
    input
        .read_exact(&mut payload)
        .map_err(|error| format!("worker pipe payload: {error}"))?;
    Ok(Incoming { header, payload })
}

fn wire_access(access: ArgumentAccess) -> BufferAccessV1 {
    match access {
        ArgumentAccess::ReadOnly => BufferAccessV1::Read,
        ArgumentAccess::WriteOnly => BufferAccessV1::Write,
        ArgumentAccess::ReadWrite => BufferAccessV1::ReadWrite,
    }
}

pub(super) fn metadata_matches(
    expected: &InspectedKernel,
    actual: &KernelMetadataV1,
    hash: [u8; 32],
) -> bool {
    if expected.name() != actual.symbol
        || hash != actual.object_sha256
        || expected.kernarg_segment_size() != u64::from(actual.kernarg_bytes)
        || expected.kernarg_segment_alignment() != u64::from(actual.kernarg_alignment)
        || expected.group_segment_fixed_size() != u64::from(actual.group_segment_bytes)
        || expected.private_segment_fixed_size() != u64::from(actual.private_segment_bytes)
        || expected.wavefront_size() != actual.wavefront_size
        || expected.implicit_argument_offset() != actual.implicit_argument_offset.map(u64::from)
        || expected.implicit_argument_size() != u64::from(actual.implicit_argument_bytes)
        || expected.explicit_arguments().len() != actual.explicit_arguments.len()
    {
        return false;
    }
    expected
        .explicit_arguments()
        .iter()
        .zip(&actual.explicit_arguments)
        .all(|(left, right)| {
            left.offset() == u64::from(right.offset)
                && left.size() == u64::from(right.bytes)
                && (left.value_kind() == ExplicitValueKind::GlobalBuffer) == right.global_buffer
                && left.pointee_alignment() == right.pointee_alignment.map(u64::from)
                && left.access().map(wire_access) == right.access
        })
}

fn pointee_size_matches(value_type: Option<ExplicitValueType>, element_bytes: u32) -> bool {
    use ExplicitValueType::{F16, F32, F64, I8, I16, I32, I64, Struct, U8, U16, U32, U64};
    match value_type {
        None => matches!(element_bytes, 2 | 4),
        Some(I8 | U8) => element_bytes == 1,
        Some(I16 | U16 | F16) => element_bytes == 2,
        Some(I32 | U32 | F32) => element_bytes == 4,
        Some(I64 | U64 | F64) => element_bytes == 8,
        Some(Struct) => false,
    }
}

pub(super) fn pack_dispatch(
    loaded: &LoadedKernel,
    dispatch: &EngineeringTpDispatchV1,
    buffers: &BTreeMap<u64, usize>,
) -> TpResult<(CommandV1, Vec<u8>)> {
    let metadata = &loaded.metadata;
    let workgroup = [dispatch.workgroup_size, 1, 1];
    let grid_x = dispatch
        .grid_workgroups
        .checked_mul(u32::from(dispatch.workgroup_size))
        .ok_or("grid overflow")?;
    if grid_x == 0
        || dispatch.workgroup_size == 0
        || u32::from(dispatch.workgroup_size) > metadata.max_flat_workgroup_size()
        || metadata
            .required_workgroup_size()
            .is_some_and(|required| required != workgroup.map(u32::from))
        || metadata
            .max_workgroups()
            .iter()
            .zip([dispatch.grid_workgroups, 1, 1])
            .any(|(limit, actual)| limit.is_some_and(|limit| actual > limit))
    {
        return Err("dispatch geometry contradicts compiler metadata".into());
    }
    let length =
        u32::try_from(metadata.kernarg_segment_size()).map_err(|_| "kernarg length overflow")?;
    if length > wire::MAX_KERNARG_BYTES_V1 {
        return Err("kernarg bound exceeded".into());
    }
    let mut bytes = vec![0; length as usize];
    let mut explicit = metadata.explicit_arguments().iter();
    let mut pointers = Vec::new();
    for argument in &dispatch.arguments {
        let field = explicit
            .next()
            .ok_or("missing explicit argument metadata")?;
        match *argument {
            EngineeringTpArgumentV1::Buffer {
                id,
                offset,
                elements,
                element_bytes,
                access,
            } => {
                let wanted_access = match access {
                    EngineeringTpBufferAccessV1::Read => BufferAccessV1::Read,
                    EngineeringTpBufferAccessV1::Write => BufferAccessV1::Write,
                    EngineeringTpBufferAccessV1::ReadWrite => BufferAccessV1::ReadWrite,
                };
                let extent = elements
                    .checked_mul(element_bytes as usize)
                    .ok_or("slice extent overflow")?;
                let end = offset.checked_add(extent).ok_or("slice offset overflow")?;
                if element_bytes == 0
                    || !offset.is_multiple_of(element_bytes as usize)
                    || !buffers
                        .get(&id)
                        .is_some_and(|capacity| offset <= *capacity && end <= *capacity)
                    || field.value_kind() != ExplicitValueKind::GlobalBuffer
                    || field.size() != 8
                    || !pointee_size_matches(field.value_type(), element_bytes)
                    || field
                        .access()
                        .map(wire_access)
                        .is_some_and(|actual| actual != wanted_access)
                    || field.pointee_alignment().is_some_and(|alignment| {
                        alignment == 0 || !(offset as u64).is_multiple_of(alignment)
                    })
                {
                    return Err("slice ownership, access or physical ABI mismatch".into());
                }
                pointers.push(PointerFixupV1 {
                    kernarg_offset: u32::try_from(field.offset())
                        .map_err(|_| "pointer offset overflow")?,
                    buffer: id,
                    buffer_offset: offset as u64,
                    extent_bytes: extent as u64,
                    access: wanted_access,
                });
                let count = explicit.next().ok_or("missing slice length ABI field")?;
                put_scalar(
                    &mut bytes,
                    count,
                    &(elements as u64).to_le_bytes(),
                    ExplicitValueType::U64,
                )?;
            }
            EngineeringTpArgumentV1::U32(value) => put_scalar(
                &mut bytes,
                field,
                &value.to_le_bytes(),
                ExplicitValueType::U32,
            )?,
            EngineeringTpArgumentV1::F32(value) => {
                if !value.is_finite() {
                    return Err("nonfinite dispatch scalar".into());
                }
                put_scalar(
                    &mut bytes,
                    field,
                    &value.to_le_bytes(),
                    ExplicitValueType::F32,
                )?;
            }
        }
    }
    if explicit.next().is_some() {
        return Err("extra explicit compiler arguments".into());
    }
    let header = CommandV1::Dispatch {
        kernel: loaded.id,
        payload_bytes: length,
        workgroup,
        grid: [grid_x, 1, 1],
        pointers,
        timeout_ms: DISPATCH_TIMEOUT_MS,
    };
    header.payload_bytes().map_err(|error| error.to_string())?;
    Ok((header, bytes))
}

fn put_scalar(
    bytes: &mut [u8],
    field: &ExplicitArgument,
    value: &[u8],
    expected_type: ExplicitValueType,
) -> TpResult<()> {
    if field.value_kind() != ExplicitValueKind::ByValue
        || field.size() != value.len() as u64
        || field.value_type().is_some_and(|kind| kind != expected_type)
    {
        return Err("scalar physical ABI mismatch".into());
    }
    let start = usize::try_from(field.offset()).map_err(|_| "scalar offset overflow")?;
    let end = start
        .checked_add(value.len())
        .ok_or("scalar end overflow")?;
    bytes
        .get_mut(start..end)
        .ok_or("scalar exceeds kernarg extent")?
        .copy_from_slice(value);
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    const FAKE_WORKER: &str = r"
import json, struct, sys, time
mode = sys.argv[1]
def send(value, payload=b''):
    header = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(header)) + header + payload)
    sys.stdout.buffer.flush()
send({'op':'ready','protocol':1,'target':'gfx950:xnack-',
      'device_unique_id':2 if mode == 'wrong' else 1,'authority':'none'})
if mode in ('wrong', 'stall'):
    time.sleep(60)
buffers = {}
while True:
    prefix = sys.stdin.buffer.read(4)
    if len(prefix) != 4: sys.exit(3)
    command = json.loads(sys.stdin.buffer.read(struct.unpack('<I',prefix)[0]))
    op = command['op']
    if op == 'allocate':
        identity = len(buffers) + 1
        buffers[identity] = bytearray(command['bytes'])
        send({'op':'allocated','buffer':identity,'bytes':command['bytes']})
    elif op == 'write':
        size = command['payload_bytes']
        value = sys.stdin.buffer.read(size)
        if len(value) != size: sys.exit(4)
        start = command['offset']
        buffers[command['buffer']][start:start+size] = value
        send({'op':'written'})
    elif op == 'read':
        start, size = command['offset'], command['bytes']
        send({'op':'read','payload_bytes':size}, buffers[command['buffer']][start:start+size])
    elif op == 'close':
        send({'op':'closed'})
        sys.exit(0)
    elif op == 'rollover_queue':
        send({'op':'queue_rolled_over', 'retired_packets':command['expected_completed_packets'],
              'queue_epoch':command['expected_epoch'] + (2 if mode == 'badrollover' else 1)})
    elif op == 'dispatch_sequence':
        entries = command['dispatches']
        sys.stdin.buffer.read(sum(entry['payload_bytes'] for entry in entries))
        send({'op':'dispatch_sequence_completed', 'elapsed_ns':[1] * (len(entries) - (1 if mode == 'shortsequence' else 0))})
    elif op == 'dispatch_full_forward':
        if command['dispatch_count'] != 616 or command['timeout_ms'] != 60000: sys.exit(6)
        plan = json.loads(sys.stdin.buffer.read(command['plan_bytes']))
        entries = plan['dispatches']
        size = command['kernarg_bytes']
        if len(entries) != 616 or sum(entry['payload_bytes'] for entry in entries) != size: sys.exit(7)
        if len(sys.stdin.buffer.read(size)) != size: sys.exit(4)
        if mode == 'fullerror': send({'op':'error','message':'injected','fatal':True})
        elif mode == 'fullordered': send({'op':'dispatch_ordered_batch_completed','completed_dispatches':616,'elapsed_ns':7})
        elif mode == 'fullpayload': send({'op':'read','payload_bytes':1}, b'x')
        elif mode == 'fullstall': time.sleep(60)
        else:
            send({'op':'dispatch_full_forward_completed',
                  'completed_dispatches':616 - (1 if mode == 'fullshort' else 0),
                  'elapsed_ns':7})
    elif op == 'dispatch_ordered_batch':
        entries = command['dispatches']
        if command['timeout_ms'] != 60000 or not 1 <= len(entries) <= 16: sys.exit(6)
        if any('timeout_ms' in entry for entry in entries): sys.exit(7)
        size = sum(entry['payload_bytes'] for entry in entries)
        if len(sys.stdin.buffer.read(size)) != size: sys.exit(4)
        if mode == 'orderederror': send({'op':'error','message':'injected','fatal':True})
        elif mode == 'orderedsequence': send({'op':'dispatch_sequence_completed','elapsed_ns':[1]*len(entries)})
        elif mode == 'orderedpayload': send({'op':'read','payload_bytes':1}, b'x')
        elif mode == 'orderedstall': time.sleep(60)
        else:
            send({'op':'dispatch_ordered_batch_completed',
                  'completed_dispatches':len(entries) - (1 if mode == 'orderedshort' else 0),
                  'elapsed_ns':7})
    else:
        sys.exit(5)
";

    fn fake(mode: &str) -> Child {
        Command::new("python3")
            .arg("-u")
            .arg("-c")
            .arg(FAKE_WORKER)
            .arg(mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
    }

    pub(crate) fn full_forward_wrapper_fixture(
        enabled: bool,
        artifact: Option<&EngineeringTpArtifactV1>,
    ) -> Worker {
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        worker.options.full_forward = enabled;
        if let Some(artifact) = artifact {
            let symbol = "ferric_qwen3_tp_batch32_argmax_f32_v8";
            let metadata = artifact
                .inspection()
                .hsaco()
                .kernels()
                .iter()
                .find(|kernel| kernel.name() == symbol)
                .unwrap();
            worker.kernels.insert(
                symbol.into(),
                LoadedKernel {
                    id: 1,
                    image: *artifact.hsaco_id().as_bytes(),
                    metadata: metadata.clone(),
                },
            );
        }
        worker
    }

    #[test]
    #[ignore = "requires FERRIC_V7_TEST_ARTIFACT pointing to a separately emitted closed image; no GPU"]
    #[cfg(feature = "tp-batch-engineering")]
    fn actual_v7_image_fake_worker_loads_rejects_duplicates_and_reaps_partial_failures() {
        let path = std::env::var_os("FERRIC_V7_TEST_ARTIFACT").expect("explicit v7 artifact path");
        let artifact = EngineeringTpArtifactV1::open_fp32_head(
            Path::new(&path),
            &ferric_qwen3_tp_fp32_head_kernels_device_v7::compiler_expectation_roster_v7(),
        )
        .unwrap();
        actual_image_fake_worker(&artifact, false);
    }

    #[test]
    #[ignore = "requires FERRIC_V9_TEST_ARTIFACT pointing to the emitted v9 image; no GPU"]
    #[cfg(feature = "tp-batch-engineering")]
    fn actual_v9_image_binding_is_fresh_exact_and_precedes_allocation() {
        use ferric_m1_engineering_execution_v1::tp_paged::{
            EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1, EngineeringTpPoolScopeV1,
        };
        let path = std::env::var_os("FERRIC_V9_TEST_ARTIFACT").expect("explicit v9 image");
        let expected = ferric_qwen3_tp_large_kv_kernels_device_v9::compiler_expectation_roster_v9();
        let artifact =
            EngineeringTpArtifactV1::open_large_kv32(Path::new(&path), &expected).unwrap();
        assert!(EngineeringTpArtifactV1::open_fp32_head32(Path::new(&path), &expected).is_err());
        let scope = EngineeringTpPoolScopeV1 {
            model: [1; 32],
            session: [2; 32],
        };
        for pages in [1, 512, 513, 16384] {
            let limits = EngineeringTpPagedLimitsV1::new_large_kv32(8192, 32, pages, 100).unwrap();
            assert!(EngineeringTpPagedPoolV1::new_wide32(scope, limits).is_err());
            let pool = EngineeringTpPagedPoolV1::new_large_kv32(scope, limits, &artifact).unwrap();
            assert_eq!(pool.limits(), limits);
        }
        actual_image_fake_worker(&artifact, true);
    }

    #[cfg(feature = "tp-batch-engineering")]
    fn actual_image_fake_worker(artifact: &EngineeringTpArtifactV1, large: bool) {
        let hash: [u8; 32] = Sha256::digest(artifact.bytes()).into();
        let metadata = artifact.inspection().hsaco().kernels().iter().map(|kernel| {
            let arguments = kernel.explicit_arguments().iter().map(|arg| serde_json::json!({
                "offset":arg.offset(), "bytes":arg.size(),
                "global_buffer":arg.value_kind() == ExplicitValueKind::GlobalBuffer,
                "pointee_alignment":arg.pointee_alignment(), "access":arg.access().map(wire_access)
            })).collect::<Vec<_>>();
            (kernel.name().to_owned(), serde_json::json!({
                "symbol":kernel.name(), "object_sha256":hash,
                "kernarg_bytes":kernel.kernarg_segment_size(), "kernarg_alignment":kernel.kernarg_segment_alignment(),
                "group_segment_bytes":kernel.group_segment_fixed_size(), "private_segment_bytes":kernel.private_segment_fixed_size(),
                "wavefront_size":kernel.wavefront_size(), "implicit_argument_offset":kernel.implicit_argument_offset(),
                "implicit_argument_bytes":kernel.implicit_argument_size(), "explicit_arguments":arguments
            }))
        }).collect::<serde_json::Map<_, _>>();
        let script = r"
import copy, json, struct, sys
metadata, mode = json.loads(sys.argv[1]), sys.argv[2]
def send(value):
    data = json.dumps(value, separators=(',', ':')).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(data)) + data)
    sys.stdout.buffer.flush()
send({'op':'ready','protocol':1,'target':'gfx950:xnack-','device_unique_id':1,'authority':'none'})
count = 0
while True:
    prefix = sys.stdin.buffer.read(4)
    if len(prefix) != 4: sys.exit(3)
    command = json.loads(sys.stdin.buffer.read(struct.unpack('<I', prefix)[0]))
    payload = sys.stdin.buffer.read(command.get('payload_bytes', 0))
    if len(payload) != command.get('payload_bytes', 0): sys.exit(4)
    if command['op'] == 'load_kernel':
        count += 1
        item = copy.deepcopy(metadata[command['symbol']])
        if mode == 'metadata' and count == 2: item['wavefront_size'] = 32
        send({'op':'loaded_kernel','kernel':1 if mode == 'duplicate_id' and count == 2 else count,'metadata':item})
    elif command['op'] == 'close':
        send({'op':'closed'})
        sys.exit(0)
    else: sys.exit(5)
";
        let modes: &[&str] = if large {
            &[
                "normal",
                "duplicate_id",
                "metadata",
                "wrong_image",
                "missing_root",
                "duplicate_root",
                "allocated",
                "pending",
                "used_queue",
                "old_epoch",
                "failed",
            ]
        } else {
            &["normal", "duplicate_id", "metadata"]
        };
        for &mode in modes {
            let child = Command::new("python3")
                .args([
                    "-u",
                    "-c",
                    script,
                    &serde_json::to_string(&metadata).unwrap(),
                    mode,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let mut worker = Worker::connect(child, 1, Duration::from_secs(5)).unwrap();
            let result = worker.load_additional_artifact(artifact);
            if matches!(mode, "duplicate_id" | "metadata") {
                assert!(result.is_err() && worker.failed);
                assert_eq!(worker.kernels.len(), 1);
                assert!(worker.load_additional_artifact(artifact).is_err());
            } else {
                result.unwrap();
                assert_eq!(worker.kernels.len(), if large { 2 } else { 3 });
                assert!(!worker.failed);
                if large {
                    use ferric_m1_engineering_execution_v1::tp_artifact::ENGINEERING_TP_LARGE_KV_EXPORTS_V9;
                    let mut names = ENGINEERING_TP_LARGE_KV_EXPORTS_V9;
                    let mut expected = hash;
                    match mode {
                        "wrong_image" => expected[0] ^= 1,
                        "missing_root" => names[0] = "missing",
                        "duplicate_root" => names[1] = names[0],
                        "allocated" => {
                            worker.buffers.insert(999, 4);
                        }
                        "pending" => worker.pending = Some(PendingRequest::Other),
                        "used_queue" => worker.queue_packets = 1,
                        "old_epoch" => worker.queue_epoch = 1,
                        "failed" => worker.failed = true,
                        _ => {}
                    }
                    assert_eq!(
                        worker.require_loaded_image(expected, &names).is_ok(),
                        mode == "normal"
                    );
                    if mode != "normal" {
                        assert!(worker.failed);
                        worker.pending = None;
                        worker.buffers.clear();
                        worker.close().unwrap();
                        assert!(worker.exited && worker.child.try_wait().unwrap().is_some());
                        continue;
                    }
                }
                assert!(worker.load_additional_artifact(artifact).is_err());
                assert!(!worker.failed);
            }
            worker.close().unwrap();
            assert!(worker.exited && worker.io_threads.is_empty());
            assert!(worker.child.try_wait().unwrap().is_some());
        }
    }

    #[test]
    fn additional_image_is_fresh_only_and_partial_failure_poisoned_and_reaped() {
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        assert!(
            worker
                .with_additional_image(&[], |_| panic!("empty roster loaded"))
                .is_err()
        );
        assert!(
            worker
                .with_additional_image(&["new", "new"], |_| panic!("duplicate loaded"))
                .is_err()
        );
        worker.queue_packets = 1;
        assert!(
            worker
                .with_additional_image(&["new"], |_| panic!("post-dispatch load"))
                .is_err()
        );
        worker.queue_packets = 0;
        worker.queue_epoch = 1;
        assert!(
            worker
                .with_additional_image(&["new"], |_| panic!("post-rollover load"))
                .is_err()
        );
        worker.queue_epoch = 0;
        worker.pending = Some(PendingRequest::Other);
        assert!(
            worker
                .with_additional_image(&["new"], |_| panic!("pending load"))
                .is_err()
        );
        worker.pending = None;
        worker.with_additional_image(&["new"], |_| Ok(())).unwrap();
        let result = worker.with_additional_image(&["another"], |worker| {
            // An acknowledged IPC operation precedes the injected later load failure.
            worker.allocate(4)?;
            Err("injected second-symbol descriptor failure".into())
        });
        assert!(result.is_err() && worker.failed);
        assert!(
            worker
                .with_additional_image(&["later"], |_| panic!("poisoned load"))
                .is_err()
        );
        worker.close().unwrap();
        assert!(worker.exited && worker.io_threads.is_empty());
        assert!(worker.child.try_wait().unwrap().is_some());
        let mut allocated = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        allocated.allocate(1).unwrap();
        assert!(
            allocated
                .with_additional_image(&["new"], |_| panic!("allocated load"))
                .is_err()
        );
        allocated.close().unwrap();
        assert!(
            allocated
                .with_additional_image(&["new"], |_| panic!("closed load"))
                .is_err()
        );
    }

    #[test]
    fn owned_child_chunked_roundtrip_closes_threads_and_reaps_process() {
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        let size = wire::MAX_TRANSFER_BYTES_V1 as usize + 17;
        let buffer = worker.allocate(size + 1).unwrap();
        let source = vec![0xa5; size];
        worker.write(buffer, 1, &source).unwrap();
        let mut actual = vec![0; size];
        worker.read(buffer, 1, &mut actual).unwrap();
        assert_eq!(actual, source);
        let mut first = [1];
        worker.read(buffer, 0, &mut first).unwrap();
        assert_eq!(first, [0]);
        worker.close().unwrap();
        assert!(worker.exited);
        assert!(worker.writer.is_none());
        assert!(worker.io_threads.is_empty());
        assert!(worker.child.try_wait().unwrap().unwrap().success());
    }

    #[test]
    fn host_tickets_preserve_roundtrip_payloads_and_explicit_failure() {
        let timing = HostTiming::enabled();
        let mut worker = Worker::connect_with_timing(
            fake("normal"),
            1,
            Duration::from_secs(5),
            timing.clone(),
            1,
        )
        .unwrap();
        let buffer = worker.allocate(4).unwrap();
        worker.write(buffer, 0, &[1, 2, 3, 4]).unwrap();
        let mut bytes = [0; 4];
        worker.read(buffer, 0, &mut bytes).unwrap();
        assert_eq!(bytes, [1, 2, 3, 4]);
        worker.close().unwrap();
        let snapshot = timing.snapshot();
        assert_eq!(snapshot["incomplete"], false);
        let rows = snapshot["records"].as_array().unwrap();
        let requests = rows
            .iter()
            .filter(|row| row["category"] == "ipc_roundtrip")
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 4);
        assert!(
            requests
                .iter()
                .all(|row| row["rank"] == 1 && row["count"] == 1 && row["failed"] == 0)
        );
        assert_eq!(
            requests.iter().find(|row| row["label"] == "write").unwrap()["request_payload_bytes"],
            4
        );
        assert_eq!(
            requests.iter().find(|row| row["label"] == "read").unwrap()["response_payload_bytes"],
            4
        );

        let timing = HostTiming::enabled();
        let mut worker = Worker::connect_with_timing(
            fake("stall"),
            1,
            Duration::from_millis(100),
            timing.clone(),
            0,
        )
        .unwrap();
        assert!(worker.allocate(4).is_err());
        assert!(worker.exited);
        let snapshot = timing.snapshot();
        assert!(
            snapshot["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["label"] == "allocate" && row["failed"] == 1)
        );
    }

    #[test]
    fn wrong_ready_identity_is_rejected_and_child_is_reaped() {
        let child = fake("wrong");
        let pid = child.id();
        assert!(Worker::connect(child, 1, Duration::from_secs(5)).is_err());
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
    }

    #[test]
    fn missing_response_has_a_deadline_and_kills_owned_child() {
        let mut worker = Worker::connect(fake("stall"), 1, Duration::from_secs(5)).unwrap();
        worker.timeout = Duration::from_millis(100);
        let start = Instant::now();
        assert!(worker.allocate(8).is_err());
        assert!(start.elapsed() < Duration::from_secs(10));
        assert!(worker.failed && worker.exited);
        assert!(worker.io_threads.is_empty());
    }

    #[test]
    fn blocked_pipe_write_has_a_deadline_and_kills_owned_child() {
        let mut worker = Worker::connect(fake("stall"), 1, Duration::from_secs(5)).unwrap();
        worker.timeout = Duration::from_millis(100);
        worker
            .buffers
            .insert(1, wire::MAX_TRANSFER_BYTES_V1 as usize);
        let start = Instant::now();
        assert!(
            worker
                .write(1, 0, &vec![7; wire::MAX_TRANSFER_BYTES_V1 as usize])
                .is_err()
        );
        assert!(start.elapsed() < Duration::from_secs(10));
        assert!(worker.failed && worker.exited);
        assert!(worker.io_threads.is_empty());
    }

    #[test]
    fn bounded_reader_requires_exact_payload_and_rejects_oversized_frames() {
        let mut bytes = Vec::new();
        wire::write_header_v1(&mut bytes, &ResponseV1::Read { payload_bytes: 3 }).unwrap();
        bytes.extend_from_slice(&[1, 2, 3]);
        let response = read_response(&mut bytes.as_slice()).unwrap();
        assert_eq!(response.payload, [1, 2, 3]);
        bytes.pop();
        assert!(read_response(&mut bytes.as_slice()).is_err());
        bytes.clear();
        wire::write_header_v1(
            &mut bytes,
            &ResponseV1::Read {
                payload_bytes: wire::MAX_TRANSFER_BYTES_V1 + 1,
            },
        )
        .unwrap();
        assert!(read_response(&mut bytes.as_slice()).is_err());
        assert!(read_response(&mut [].as_slice()).is_err());
    }

    #[test]
    fn compiler_pointee_width_cannot_be_substituted_by_caller() {
        assert!(pointee_size_matches(Some(ExplicitValueType::U16), 2));
        assert!(!pointee_size_matches(Some(ExplicitValueType::U16), 4));
        assert!(pointee_size_matches(Some(ExplicitValueType::F32), 4));
        assert!(!pointee_size_matches(Some(ExplicitValueType::F32), 2));
        assert!(!pointee_size_matches(Some(ExplicitValueType::Struct), 4));
        assert!(!pointee_size_matches(None, 0));
    }

    #[test]
    fn rollover_requires_exact_receipt_and_retains_owned_buffers() {
        for mode in ["normal", "badrollover"] {
            let mut worker = Worker::connect(fake(mode), 1, Duration::from_secs(5)).unwrap();
            worker.options.rollover = true;
            let buffer = worker.allocate(16).unwrap();
            worker.write(buffer, 0, &[7; 16]).unwrap();
            worker.queue_packets = wire::MAX_UNRETIRED_RING_PACKETS_V1 - 10;
            worker.prepare_packets(10).unwrap();
            assert_eq!(worker.queue_epoch, 0);
            let result = worker.prepare_packets(11);
            if mode == "badrollover" {
                assert!(result.is_err());
                assert!(worker.failed && worker.exited);
            } else {
                result.unwrap();
                assert_eq!((worker.queue_epoch, worker.queue_packets), (1, 0));
                let mut bytes = [0; 16];
                worker.read(buffer, 0, &mut bytes).unwrap();
                assert_eq!(bytes, [7; 16]);
                worker.close().unwrap();
            }
        }
    }

    #[test]
    fn sequence_completion_is_counted_only_when_exact() {
        for mode in ["normal", "shortsequence"] {
            let mut worker = Worker::connect(fake(mode), 1, Duration::from_secs(5)).unwrap();
            let dispatches = (0..2)
                .map(|_| SequenceDispatchV1 {
                    kernel: 1,
                    payload_bytes: 0,
                    workgroup: [64, 1, 1],
                    grid: [64, 1, 1],
                    pointers: vec![],
                    timeout_ms: 1000,
                })
                .collect();
            worker
                .send(CommandV1::DispatchSequence { dispatches }, vec![])
                .unwrap();
            let result = worker.wait_sequence(2);
            if mode == "shortsequence" {
                assert!(result.is_err());
                assert_eq!(worker.queue_packets, 0);
                assert!(worker.failed && worker.exited);
            } else {
                result.unwrap();
                assert_eq!(worker.queue_packets, 2);
                worker.close().unwrap();
            }
        }
    }

    fn full_forward_frame() -> (CommandV1, Vec<u8>) {
        let entries = vec![
            OrderedBatchDispatchV1 {
                kernel: 1,
                payload_bytes: 4,
                workgroup: [64, 1, 1],
                grid: [64, 1, 1],
                pointers: vec![],
            };
            wire::FULL_FORWARD_DISPATCHES_V1
        ];
        let payload = wire::encode_full_forward_payload_v1(
            &entries,
            &vec![0; wire::FULL_FORWARD_DISPATCHES_V1 * 4],
        )
        .unwrap();
        (
            CommandV1::DispatchFullForward {
                dispatch_count: wire::FULL_FORWARD_DISPATCHES_V1 as u32,
                plan_bytes: payload.plan_bytes,
                kernarg_bytes: payload.kernarg_bytes,
                timeout_ms: DISPATCH_TIMEOUT_MS,
            },
            payload.bytes,
        )
    }

    #[test]
    fn full_forward_completion_is_distinct_exact_and_terminal_on_failure() {
        for mode in [
            "normal",
            "fullshort",
            "fullordered",
            "fullpayload",
            "fullerror",
            "fullstall",
        ] {
            let mut worker = Worker::connect(fake(mode), 1, Duration::from_secs(5)).unwrap();
            if mode == "fullstall" {
                worker.timeout = Duration::from_millis(100);
            }
            let (header, payload) = full_forward_frame();
            worker.send(header, payload).unwrap();
            let result = worker.wait_full_forward(wire::FULL_FORWARD_DISPATCHES_V1);
            if mode == "normal" {
                result.unwrap();
                assert_eq!(worker.queue_packets, 616);
                worker.close().unwrap();
            } else {
                assert!(result.is_err());
                assert_eq!(worker.queue_packets, 0);
                assert!(worker.failed && worker.exited);
                assert!(worker.allocate(4).is_err());
            }
        }
    }

    #[test]
    fn full_forward_pending_excludes_wrong_wait_and_host_commands() {
        for operation in 0..7 {
            let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
            let buffer = worker.allocate(4).unwrap();
            let (header, payload) = full_forward_frame();
            worker.send(header, payload).unwrap();
            let result = match operation {
                0 => worker.wait_full_forward(615),
                1 => worker.wait_full_forward(617),
                2 => worker.wait_sequence(616),
                3 => worker.wait_ordered_batch(616),
                4 => worker.write(buffer, 0, &[0; 4]),
                5 => worker.read(buffer, 0, &mut [0; 4]),
                _ => worker.runtime_diagnostic_snapshot().map(|_| ()),
            };
            assert!(result.is_err());
            assert!(worker.failed && worker.exited);
            assert_eq!(worker.queue_packets, 0);
        }
    }

    #[test]
    fn full_forward_close_drains_and_counts_packets_not_forwards() {
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        worker.queue_packets = wire::MAX_UNRETIRED_RING_PACKETS_V1 - 616;
        let (header, payload) = full_forward_frame();
        worker.send(header, payload).unwrap();
        worker.close().unwrap();
        assert_eq!(worker.queue_packets, wire::MAX_UNRETIRED_RING_PACKETS_V1);
        assert!(worker.exited);
        let mut overflow = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        overflow.queue_packets = wire::MAX_UNRETIRED_RING_PACKETS_V1 - 615;
        let (header, payload) = full_forward_frame();
        overflow.send(header, payload).unwrap();
        assert!(overflow.wait_full_forward(616).is_err());
        assert_eq!(
            overflow.queue_packets,
            wire::MAX_UNRETIRED_RING_PACKETS_V1 - 615
        );
        assert!(overflow.failed && overflow.exited);
    }

    #[test]
    fn full_forward_policy_rejects_conflicting_runtime_modes() {
        for option in 0..5 {
            let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
            let mut options = RuntimeOptions {
                full_forward: true,
                ..RuntimeOptions::default()
            };
            match option {
                0 => options.ordered_batches = true,
                1 => options.sequences = true,
                2 => options.rollover = true,
                3 => options.profile = true,
                _ => options.shared_full_currentness = true,
            }
            assert!(worker.configure_options(options).is_err());
            assert!(worker.failed && worker.exited);
        }
    }

    #[test]
    fn full_forward_pack_rejects_disabled_wrong_count_unloaded_and_budget() {
        for (enabled, count, packets) in [
            (false, 616, 0),
            (true, 0, 0),
            (true, 615, 0),
            (true, 617, 0),
            (true, 616, 0),
            (true, 616, wire::MAX_UNRETIRED_RING_PACKETS_V1 - 615),
        ] {
            let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
            worker.options.full_forward = enabled;
            worker.queue_packets = packets;
            let commands = vec![
                EngineeringTpDispatchV1 {
                    kernel: "not_loaded",
                    grid_workgroups: 1,
                    workgroup_size: 64,
                    arguments: vec![],
                };
                count
            ];
            assert!(worker.submit_full_forward(&commands).is_err());
            assert_eq!(worker.queue_packets, packets);
            assert!(worker.failed && worker.exited);
        }
    }

    fn ordered_header(count: usize) -> CommandV1 {
        CommandV1::DispatchOrderedBatch {
            dispatches: vec![
                OrderedBatchDispatchV1 {
                    kernel: 1,
                    payload_bytes: 0,
                    workgroup: [64, 1, 1],
                    grid: [64, 1, 1],
                    pointers: vec![],
                };
                count
            ],
            timeout_ms: DISPATCH_TIMEOUT_MS,
        }
    }

    #[test]
    fn ordered_completion_is_distinct_exact_and_terminal_on_failure() {
        for mode in [
            "normal",
            "orderedshort",
            "orderedsequence",
            "orderedpayload",
            "orderederror",
            "orderedstall",
        ] {
            let mut worker = Worker::connect(fake(mode), 1, Duration::from_secs(5)).unwrap();
            if mode == "orderedstall" {
                worker.timeout = Duration::from_millis(100);
            }
            worker.send(ordered_header(10), vec![]).unwrap();
            let result = worker.wait_ordered_batch(10);
            if mode == "normal" {
                result.unwrap();
                assert_eq!(worker.queue_packets, 10);
                worker.close().unwrap();
            } else {
                assert!(result.is_err());
                assert_eq!(worker.queue_packets, 0);
                assert!(worker.failed && worker.exited);
                assert!(worker.send(ordered_header(1), vec![]).is_err());
            }
        }
    }

    #[test]
    fn ordered_pending_rejects_mismatched_wait_and_host_commands() {
        for operation in 0..5 {
            let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
            let buffer = worker.allocate(4).unwrap();
            worker.send(ordered_header(5), vec![]).unwrap();
            let result = match operation {
                0 => worker.wait_ordered_batch(4),
                1 => worker.wait_sequence(5),
                2 => worker.write(buffer, 0, &[0; 4]),
                3 => worker.read(buffer, 0, &mut [0; 4]),
                _ => worker.runtime_diagnostic_snapshot().map(|_| ()),
            };
            assert!(result.is_err());
            assert!(worker.failed && worker.exited);
            assert_eq!(worker.queue_packets, 0);
        }
    }

    #[test]
    fn ordered_packets_not_groups_are_retired_and_pending_close_is_drained() {
        let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
        worker.options.rollover = true;
        worker.queue_packets = wire::MAX_UNRETIRED_RING_PACKETS_V1 - 15;
        for count in [10, 5] {
            worker.send(ordered_header(count), vec![]).unwrap();
            worker.wait_ordered_batch(count).unwrap();
        }
        assert_eq!(worker.queue_packets, wire::MAX_UNRETIRED_RING_PACKETS_V1);
        worker.prepare_packets(616).unwrap();
        assert_eq!((worker.queue_epoch, worker.queue_packets), (1, 0));
        worker.send(ordered_header(10), vec![]).unwrap();
        worker.close().unwrap();
        assert_eq!(worker.queue_packets, 10);
        assert!(worker.exited);
    }

    #[test]
    fn ordered_pack_rejects_disabled_empty_oversized_and_unloaded_before_publication() {
        for (enabled, count) in [(false, 1), (true, 0), (true, 17), (true, 1)] {
            let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
            worker.options.ordered_batches = enabled;
            let commands = vec![
                EngineeringTpDispatchV1 {
                    kernel: "not_loaded",
                    grid_workgroups: 1,
                    workgroup_size: 64,
                    arguments: vec![],
                };
                count
            ];
            assert!(worker.submit_ordered_batch(&commands).is_err());
            assert_eq!(worker.queue_packets, 0);
            assert!(worker.failed && worker.exited);
        }
    }

    #[test]
    #[ignore = "requires FERRIC_V8_TEST_ARTIFACT; real metadata packing with a CPU fake worker, no GPU"]
    #[cfg(feature = "tp-batch-engineering")]
    fn actual_v8_metadata_ordered_pack_preserves_bounds_and_aggregate_timeout() {
        actual_v8_metadata_batch_pack(false);
    }

    #[test]
    #[ignore = "requires FERRIC_V8_TEST_ARTIFACT; real metadata packing with a CPU fake worker, no GPU"]
    #[cfg(feature = "tp-batch-engineering")]
    fn actual_v8_metadata_full_forward_pack_rejects_late_invalid_operand_before_send() {
        actual_v8_metadata_batch_pack(true);
    }

    #[cfg(feature = "tp-batch-engineering")]
    fn actual_v8_metadata_batch_pack(full_forward: bool) {
        let path = std::env::var_os("FERRIC_V8_TEST_ARTIFACT").expect("explicit v8 artifact path");
        let artifact = EngineeringTpArtifactV1::open_fp32_head32(
            Path::new(&path),
            &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(),
        )
        .unwrap();
        let symbol = "ferric_qwen3_tp_batch32_argmax_f32_v8";
        let metadata = artifact
            .inspection()
            .hsaco()
            .kernels()
            .iter()
            .find(|kernel| kernel.name() == symbol)
            .unwrap();
        for malformed in [false, true] {
            let mut worker = Worker::connect(fake("normal"), 1, Duration::from_secs(5)).unwrap();
            worker.options.ordered_batches = !full_forward;
            worker.options.full_forward = full_forward;
            worker.kernels.insert(
                symbol.into(),
                LoadedKernel {
                    id: 1,
                    image: *artifact.hsaco_id().as_bytes(),
                    metadata: metadata.clone(),
                },
            );
            let logits = worker.allocate(32 * 151_936 * 4).unwrap();
            let choice = worker.allocate(32 * 4).unwrap();
            let command = EngineeringTpDispatchV1 {
                kernel: symbol,
                grid_workgroups: 1,
                workgroup_size: 64,
                arguments: vec![
                    EngineeringTpArgumentV1::Buffer {
                        id: logits,
                        offset: 0,
                        elements: 32 * 151_936,
                        element_bytes: 4,
                        access: EngineeringTpBufferAccessV1::Read,
                    },
                    EngineeringTpArgumentV1::Buffer {
                        id: choice,
                        offset: 0,
                        elements: 32,
                        element_bytes: 4,
                        access: EngineeringTpBufferAccessV1::Write,
                    },
                    EngineeringTpArgumentV1::U32(1),
                ],
            };
            let count = if full_forward {
                wire::FULL_FORWARD_DISPATCHES_V1
            } else {
                16
            };
            let mut commands = vec![command; count];
            if malformed {
                commands.last_mut().unwrap().arguments[1] = EngineeringTpArgumentV1::Buffer {
                    id: choice,
                    offset: 0,
                    elements: 33,
                    element_bytes: 4,
                    access: EngineeringTpBufferAccessV1::Write,
                };
            }
            let result = if full_forward {
                worker.submit_full_forward(&commands)
            } else {
                worker.submit_ordered_batch(&commands)
            };
            if malformed {
                assert!(result.is_err() && worker.failed && worker.exited);
                assert_eq!(worker.queue_packets, 0);
            } else {
                result.unwrap();
                if full_forward {
                    worker.wait_full_forward(count).unwrap();
                } else {
                    worker.wait_ordered_batch(count).unwrap();
                }
                assert_eq!(worker.queue_packets, count as u64);
                worker.close().unwrap();
            }
        }
    }
}
