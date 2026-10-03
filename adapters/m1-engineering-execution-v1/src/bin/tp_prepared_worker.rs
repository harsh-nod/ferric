//! Separate synchronous whole-token transport; no GPU scheduling semantics change.

use super::tp_peer_worker::{dependency_wire, peer_wire as wire};
#[path = "../../../tp-peer-engineering-worker-v4/src/prepared_forward_wire.rs"]
#[allow(dead_code)]
mod prepared_forward_wire;

use self::{dependency_wire as dep, prepared_forward_wire as protocol};
use super::tp_worker::{LoadedKernel, RuntimeOptions, metadata_matches, pack_dispatch};
use fe2o3_kfd::engineering_wire::{CommandV1, ResponseV1, SequenceDispatchV1};
use ferric_engine::tensor_parallel::Qwen3TensorParallelCollectiveV1;
use ferric_m1_engineering_execution_v1::host_timing::HostTiming;
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTp2CollectiveReceiptV1, EngineeringTp2GraphGeometryV1 as GraphGeometry,
    EngineeringTp2GraphInputV1 as GraphInput, EngineeringTp2GraphKernelProfileV1 as GraphKernels,
    EngineeringTp2GraphPolicyV1 as GraphPolicy,
    EngineeringTp2PreparedGraphReceiptV1 as GraphReceipt, EngineeringTp2PreparedInputV1 as Input,
    EngineeringTp2PreparedProgramV1 as Program, EngineeringTp2PreparedReceiptV1 as Receipt,
    EngineeringTp2PreparedStepV1 as Step, EngineeringTpArgumentV1 as Arg,
    EngineeringTpBufferAccessV1 as Access, EngineeringTpDispatchV1, EngineeringTpRankTransportV1,
    TpResult,
};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const IPC_TIMEOUT: Duration = Duration::from_mins(2);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);

fn setup_upload_chunk_bytes(mode: PreparedMode) -> usize {
    if mode.graph_policy().is_some() {
        // Setup framing delegates to CommandV1; this is its existing ceiling.
        usize::try_from(fe2o3_kfd::engineering_wire::MAX_TRANSFER_BYTES_V1).unwrap_or(0)
    } else {
        1 << 20
    }
}

#[path = "tp_prepared_scope.rs"]
mod scope;
pub(crate) use scope::Mode as PreparedMode;
pub use scope::graph::MetadataUploadMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphLaunchOptions {
    pub kernel_profile: GraphKernels,
    pub metadata_upload_mode: MetadataUploadMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphGeometryLaunchOptions {
    pub kernel_profile: GraphKernels,
    pub metadata_upload_mode: MetadataUploadMode,
    pub geometry: GraphGeometry,
}

fn graph_geometry(mode: PreparedMode) -> GraphGeometry {
    match mode.geometry() {
        scope::graph::GraphGeometry::Short64 => GraphGeometry::Short64,
        scope::graph::GraphGeometry::Long2304 => GraphGeometry::Long2304,
    }
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(HEX[usize::from(byte >> 4)]),
                char::from(HEX[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

fn validate_options(ids: &[u64], options: RuntimeOptions, timing: bool) -> TpResult<[u64; 2]> {
    if ids.len() != 2
        || ids.contains(&0)
        || ids[0] == ids[1]
        || options.operational
        || options.sequences
        || options.ordered_batches
        || options.full_forward
        || options.rollover
        || options.shared_full_currentness
        || options.profile
        || timing
    {
        return Err(
            "prepared TP2 requires exact two devices and the baseline full-currentness profile"
                .into(),
        );
    }
    Ok([ids[0], ids[1]])
}

fn validate_admission_option(mode: PreparedMode, cache_admission: bool) -> TpResult<()> {
    if mode
        .graph_policy()
        .is_some_and(|policy| cache_admission != policy.admission_cache())
        || (mode == PreparedMode::NativeProgram && cache_admission)
    {
        return Err("native graph admission option must match its immutable launch policy".into());
    }
    Ok(())
}

fn validate_metadata_option(mode: PreparedMode) -> TpResult<()> {
    if mode.graph_policy().is_some_and(GraphPolicy::decode_token)
        && mode.metadata_upload_mode() != MetadataUploadMode::Batched
    {
        return Err("closed-token graph requires explicit batched metadata uploads".into());
    }
    if mode.graph_policy().is_some_and(GraphPolicy::finite_request)
        && mode.kernel_profile() != scope::graph::KernelProfile::WaveStackNormAttentionKvMlp
    {
        return Err("finite request requires full-wave kernels".into());
    }
    Ok(())
}

fn valid_ready(response: &protocol::Response, ids: [u64; 2], pid: u32) -> bool {
    matches!(response, protocol::Response::Ready {
        protocol, mode, profile, unique_ids, process_id, authority, currentness,
        control_allocation_flags,
    } if *protocol == protocol::PROTOCOL && mode == protocol::MODE
        && profile == protocol::PROFILE && *unique_ids == ids && *process_id == pid
        && authority == "none" && currentness == "full"
        && *control_allocation_flags == dep::CONTROL_ALLOCATION_FLAGS)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Setup,
    Ready,
    Executing,
    Terminal,
    Closed,
}

struct Registration {
    program: Program,
    hash: [u8; 32],
    generation: u64,
    pages: Vec<u32>,
    geometry: GraphGeometry,
}

struct Connection {
    child: Child,
    writer: Option<SyncSender<(scope::Request, Vec<u8>)>>,
    written: Receiver<TpResult<()>>,
    reader: Receiver<TpResult<scope::Reply>>,
    finite_clean_eof: Option<Receiver<()>>,
    threads: Vec<JoinHandle<()>>,
    ids: [u64; 2],
    next_request: u64,
    next_native_request: u64,
    closed: [bool; 2],
    exited: bool,
    phase: Phase,
    timeout: Duration,
    mode: PreparedMode,
    capacities: BTreeMap<u64, usize>,
    catalog: protocol::Catalog,
    kernels: [BTreeMap<String, LoadedKernel>; 2],
    registration: Option<Registration>,
    _timing: HostTiming,
}

impl Connection {
    #[cfg(test)]
    fn connect(
        child: Child,
        ids: [u64; 2],
        timeout: Duration,
        timing: HostTiming,
    ) -> TpResult<Self> {
        Self::connect_mode(child, ids, timeout, timing, PreparedMode::Interpreter)
    }

    fn connect_mode(
        mut child: Child,
        ids: [u64; 2],
        timeout: Duration,
        timing: HostTiming,
        mode: PreparedMode,
    ) -> TpResult<Self> {
        let Some(mut input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("prepared stdin missing".into());
        };
        let Some(mut output) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("prepared stdout missing".into());
        };
        let (writer, outgoing) = mpsc::sync_channel::<(scope::Request, Vec<u8>)>(1);
        let (written, acknowledgments) = mpsc::sync_channel(1);
        let writing = thread::spawn(move || {
            while let Ok((header, payload)) = outgoing.recv() {
                let result = header
                    .write(&mut input, &payload)
                    .map_err(|error| error.to_string());
                let failed = result.is_err();
                if written.send(result).is_err() || failed {
                    break;
                }
            }
        });
        let (incoming, reader) = mpsc::sync_channel(1);
        let (eof_sender, finite_clean_eof) =
            if mode.graph_policy().is_some_and(GraphPolicy::finite_request) {
                let (sender, receiver) = mpsc::sync_channel(1);
                (Some(sender), Some(receiver))
            } else {
                (None, None)
            };
        let reading = thread::spawn(move || {
            loop {
                let response = scope::read_response(&mut output, mode);
                if matches!(&response, Ok(None))
                    && let Some(sender) = &eof_sender
                {
                    let _ = sender.send(());
                    break;
                }
                let result = response
                    .map_err(|error| error.to_string())
                    .and_then(|value| value.ok_or("prepared child closed response stream".into()));
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
            finite_clean_eof,
            threads: vec![writing, reading],
            ids,
            next_request: 1,
            next_native_request: 1,
            closed: [false; 2],
            exited: false,
            phase: Phase::Setup,
            timeout,
            mode,
            capacities: BTreeMap::new(),
            catalog: protocol::Catalog {
                kernels: vec![],
                buffers: vec![],
            },
            kernels: std::array::from_fn(|_| BTreeMap::new()),
            registration: None,
            _timing: timing,
        };
        if !matches!(connection.reader.recv_timeout(timeout), Ok(Ok(response))
            if response.valid_ready(mode, ids, connection.child.id()))
        {
            return connection.reject("prepared explicit ready identity/policy mismatch");
        }
        Ok(connection)
    }

    fn reject<T>(&mut self, error: impl Into<String>) -> TpResult<T> {
        self.phase = Phase::Terminal;
        let error = error.into();
        Err(match self.terminate() {
            Ok(()) => error,
            Err(cleanup) => format!("{error}; prepared teardown: {cleanup}"),
        })
    }

    fn exchange_reply(
        &mut self,
        header: protocol::Request,
        bytes: Vec<u8>,
    ) -> TpResult<scope::Reply> {
        let header = match scope::Request::for_mode(self.mode, header) {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        self.exchange_wire_reply(header, bytes)
    }

    fn exchange_wire_reply(
        &mut self,
        header: scope::Request,
        bytes: Vec<u8>,
    ) -> TpResult<scope::Reply> {
        if matches!(self.phase, Phase::Terminal | Phase::Closed)
            || self.exited
            || self.closed.iter().any(|value| *value) && !header.is_close()
            || header.id() != self.next_request
            || header.geometry() != self.mode.geometry()
        {
            return self.reject("prepared request on sealed/terminal connection");
        }
        let Some(next) = self.next_request.checked_add(1) else {
            return self.reject("prepared request counter overflow");
        };
        match header.payload_bytes() {
            Ok(length) if length == bytes.len() => {}
            _ => return self.reject("prepared frame payload mismatch"),
        }
        let deadline = Instant::now() + self.timeout;
        let Some(writer) = &self.writer else {
            return self.reject("prepared writer closed");
        };
        if writer.try_send((header, bytes)).is_err() {
            return self.reject("prepared writer unavailable");
        }
        match self
            .written
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return self.reject(error),
            Err(error) => {
                return self.reject(format!("prepared write deadline/disconnect: {error}"));
            }
        }
        let response = match self
            .reader
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => return self.reject(error),
            Err(error) => {
                return self.reject(format!("prepared response deadline/disconnect: {error}"));
            }
        };
        self.next_request = next;
        Ok(response)
    }

    fn exchange(
        &mut self,
        header: protocol::Request,
        bytes: Vec<u8>,
    ) -> TpResult<(protocol::Response, Option<scope::NativeProgramCompletion>)> {
        let (response, native) = match self.exchange_reply(header, bytes)?.into_response() {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        if let protocol::Response::Fatal { message, .. } = response {
            return self.reject(format!("prepared child terminal: {message}"));
        }
        Ok((response, native))
    }

    fn setup(
        &mut self,
        rank: usize,
        peer_readable: bool,
        command: CommandV1,
        bytes: Vec<u8>,
    ) -> TpResult<ResponseV1> {
        if self.phase != Phase::Setup || rank >= 2 {
            return self.reject("prepared registry already sealed");
        }
        let id = self.next_request;
        let Some(next_native) = self
            .next_native_request
            .checked_add(u64::from(self.mode == PreparedMode::Interpreter))
        else {
            return self.reject("prepared native identity overflow");
        };
        let (response, _) = self.exchange(
            protocol::Request::Setup {
                id,
                rank: u32::try_from(rank).map_err(|_| "prepared setup rank overflow")?,
                peer_readable,
                command,
            },
            bytes,
        )?;
        if let protocol::Response::Setup {
            id: actual,
            response,
        } = response
            && actual == id
        {
            self.next_native_request = next_native;
            return Ok(response);
        }
        self.reject("prepared setup response identity mismatch")
    }

    fn check_extent(&self, rank: usize, id: u64, offset: usize, bytes: usize) -> TpResult<()> {
        let entry = self
            .catalog
            .buffers
            .iter()
            .find(|entry| entry.id == id)
            .ok_or("prepared unknown buffer")?;
        if entry.rank as usize != rank
            || offset
                .checked_add(bytes)
                .is_none_or(|end| end as u64 > entry.bytes)
        {
            return Err("prepared buffer owner/extent mismatch".into());
        }
        Ok(())
    }

    fn pack(
        &self,
        rank: usize,
        dispatch: &EngineeringTpDispatchV1,
        collective: bool,
    ) -> TpResult<protocol::Template> {
        if rank >= 2 || !collective && dep::is_collective_root(dispatch.kernel) {
            return Err("prepared dispatch rank or collective role mismatch".into());
        }
        for argument in &dispatch.arguments {
            if let Arg::Buffer { id, access, .. } = argument {
                let entry = self
                    .catalog
                    .buffers
                    .iter()
                    .find(|entry| entry.id == *id)
                    .ok_or("prepared unregistered buffer")?;
                if entry.rank as usize != rank
                    && (!entry.peer_readable
                        || *access != Access::Read
                        || ![dep::CONSUMER_ROOT, "ferric_qwen3_tp_peer_copy_bf16_v4"]
                            .contains(&dispatch.kernel))
                {
                    return Err("prepared typed peer-read admission mismatch".into());
                }
            }
        }
        let kernel = self.kernels[rank]
            .get(dispatch.kernel)
            .ok_or("prepared kernel not loaded")?;
        let (command, bytes) = pack_dispatch(kernel, dispatch, &self.capacities)?;
        let CommandV1::Dispatch {
            kernel,
            payload_bytes,
            workgroup,
            grid,
            pointers,
            timeout_ms,
        } = command
        else {
            return Err("prepared dispatch packing changed".into());
        };
        Ok(protocol::Template {
            dispatch: SequenceDispatchV1 {
                kernel,
                payload_bytes,
                workgroup,
                grid,
                pointers,
                timeout_ms: if collective {
                    dep::TIMEOUT_MS
                } else {
                    timeout_ms
                },
            },
            bytes,
        })
    }

    fn load_graph_argmax_v22(&mut self, artifact: &EngineeringTpArtifactV1) -> TpResult<()> {
        if !self.mode.kernel_profile().has_v22()
            || self.mode.graph_policy().is_none()
            || self.catalog.kernels.len() != 34
            || !artifact.is_graph_bf16_argmax_v22()
        {
            return self
                .reject("graph V22 sidecar must follow original34 descriptors on rank zero");
        }
        let metadata = &artifact.inspection().hsaco().kernels()[0];
        let hash = digest(artifact.bytes());
        let response = self.setup(
            0,
            false,
            CommandV1::LoadKernel {
                payload_bytes: u32::try_from(artifact.bytes().len())
                    .map_err(|_| "V22 HSACO length")?,
                object_sha256: hash,
                symbol: metadata.name().into(),
            },
            artifact.bytes().to_vec(),
        )?;
        let ResponseV1::LoadedKernel {
            kernel,
            metadata: actual,
        } = response
        else {
            return self.reject("graph V22 actual kernel admission response missing");
        };
        if kernel == 0
            || self.catalog.kernels.iter().any(|entry| entry.id == kernel)
            || self.kernels[0].contains_key(metadata.name())
            || !metadata_matches(metadata, &actual, hash)
        {
            return self.reject("graph V22 actual kernel identity or metadata mismatch");
        }
        self.catalog.kernels.push(protocol::KernelBinding {
            id: kernel,
            rank: 0,
            metadata: actual,
        });
        self.kernels[0].insert(
            metadata.name().into(),
            LoadedKernel {
                id: kernel,
                image: hash,
                metadata: metadata.clone(),
            },
        );
        Ok(())
    }

    fn load_graph_norm_v15(
        &mut self,
        artifact: &EngineeringTpArtifactV1,
        rank: usize,
    ) -> TpResult<()> {
        if rank >= 2
            || !self.mode.kernel_profile().has_v15()
            || self.mode.graph_policy().is_none()
            || self.catalog.kernels.len() != 35 + rank
            || !artifact.is_graph_wave_rmsnorm_v15()
        {
            return self
                .reject("graph V15 sidecars must follow V22 in rank-zero then rank-one order");
        }
        let metadata = &artifact.inspection().hsaco().kernels()[0];
        let hash = digest(artifact.bytes());
        let response = self.setup(
            rank,
            false,
            CommandV1::LoadKernel {
                payload_bytes: u32::try_from(artifact.bytes().len())
                    .map_err(|_| "V15 HSACO length")?,
                object_sha256: hash,
                symbol: metadata.name().into(),
            },
            artifact.bytes().to_vec(),
        )?;
        let ResponseV1::LoadedKernel {
            kernel,
            metadata: actual,
        } = response
        else {
            return self.reject("graph V15 actual kernel admission response missing");
        };
        if kernel == 0
            || self.catalog.kernels.iter().any(|entry| entry.id == kernel)
            || self.kernels[rank].contains_key(metadata.name())
            || !metadata_matches(metadata, &actual, hash)
        {
            return self.reject("graph V15 actual kernel identity or metadata mismatch");
        }
        self.catalog.kernels.push(protocol::KernelBinding {
            id: kernel,
            rank: u32::try_from(rank).map_err(|_| "V15 rank bound")?,
            metadata: actual,
        });
        self.kernels[rank].insert(
            metadata.name().into(),
            LoadedKernel {
                id: kernel,
                image: hash,
                metadata: metadata.clone(),
            },
        );
        Ok(())
    }

    fn load_graph_split_attention(
        &mut self,
        artifact: &EngineeringTpArtifactV1,
        rank: usize,
    ) -> TpResult<()> {
        if rank >= 2
            || !self.mode.kernel_profile().split_attention()
            || self.mode.graph_policy().is_none()
            || self.catalog.kernels.len() != 37 + rank * 2
            || !artifact.is_graph_split_attention_v1()
        {
            return self.reject("split sidecar must follow V15 in rank/partial/merge order");
        }
        let hash = digest(artifact.bytes());
        for root in protocol::SPLIT_ATTENTION_ROOTS {
            let metadata = artifact
                .inspection()
                .hsaco()
                .kernels()
                .iter()
                .find(|kernel| kernel.name() == root)
                .ok_or("split root missing")?;
            let response = self.setup(
                rank,
                false,
                CommandV1::LoadKernel {
                    payload_bytes: u32::try_from(artifact.bytes().len())
                        .map_err(|_| "split HSACO length")?,
                    object_sha256: hash,
                    symbol: root.into(),
                },
                artifact.bytes().to_vec(),
            )?;
            let ResponseV1::LoadedKernel {
                kernel,
                metadata: actual,
            } = response
            else {
                return self.reject("split actual kernel admission response missing");
            };
            if kernel == 0
                || self.catalog.kernels.iter().any(|entry| entry.id == kernel)
                || self.kernels[rank].contains_key(root)
                || !metadata_matches(metadata, &actual, hash)
            {
                return self.reject("split actual kernel identity or metadata mismatch");
            }
            self.catalog.kernels.push(protocol::KernelBinding {
                id: kernel,
                rank: u32::try_from(rank).map_err(|_| "split rank bound")?,
                metadata: actual,
            });
            self.kernels[rank].insert(
                root.into(),
                LoadedKernel {
                    id: kernel,
                    image: hash,
                    metadata: metadata.clone(),
                },
            );
        }
        Ok(())
    }

    fn pack_program(&self, program: &Program) -> TpResult<protocol::Program> {
        if self.phase != Phase::Setup
            || self.registration.is_some()
            || program.steps.len() != self.mode.kernel_profile().steps()
            || self.catalog.kernels.len() != self.mode.kernel_profile().descriptors()
        {
            return Err("prepared fresh exact program registration required".into());
        }
        let expected_argmax = if self.mode.kernel_profile().wave_argmax() {
            protocol::V22_ROOT
        } else {
            "ferric_qwen3_tp_batch_argmax_bf16_v2"
        };
        if !matches!(program.steps.last(), Some(Step::Rank { rank: 0, dispatch }) if dispatch.kernel == expected_argmax)
        {
            return Err("prepared final argmax does not match immutable kernel profile".into());
        }
        let mut steps = Vec::with_capacity(self.mode.kernel_profile().steps());
        let mut collective_index = 0;
        let mut counts = [0; 2];
        for step in &program.steps {
            match step {
                Step::Rank { rank, dispatch } => {
                    let template = self.pack(*rank as usize, dispatch, false)?;
                    counts[*rank as usize] += 1;
                    steps.push(protocol::Step::Rank {
                        rank: *rank,
                        template,
                    });
                }
                Step::Collective(request) => {
                    request.validate()?;
                    let expected_operation = if collective_index % 2 == 0 {
                        Qwen3TensorParallelCollectiveV1::AttentionOutputSum
                    } else {
                        Qwen3TensorParallelCollectiveV1::FeedForwardDownSum
                    };
                    let expected_producer = if self.mode.kernel_profile().wave_mlp()
                        && expected_operation == Qwen3TensorParallelCollectiveV1::FeedForwardDownSum
                    {
                        "ferric_qwen3_tp_wave_gemv_partial_f32_v3"
                    } else {
                        dep::PRODUCER_ROOT
                    };
                    if request.key.group_id != program.group_id
                        || request.key.epoch != 0
                        || request.key.layer != collective_index / 2
                        || request.key.operation != expected_operation
                        || request
                            .producers
                            .iter()
                            .any(|value| value.kernel != expected_producer)
                        || request
                            .consumers
                            .iter()
                            .any(|value| value.kernel != dep::CONSUMER_ROOT)
                    {
                        return Err(
                            "prepared exact72 profile collective keys/roots required".into()
                        );
                    }
                    let producers = [
                        self.pack(0, &request.producers[0], true)?,
                        self.pack(1, &request.producers[1], true)?,
                    ];
                    let consumers = [
                        self.pack(0, &request.consumers[0], true)?,
                        self.pack(1, &request.consumers[1], true)?,
                    ];
                    steps.push(protocol::Step::Collective {
                        layer: request.key.layer,
                        operation: operation(request.key.operation),
                        producers,
                        consumers: Box::new(consumers),
                    });
                    for count in &mut counts {
                        *count += 2;
                    }
                    collective_index += 1;
                }
            }
        }
        if collective_index != 72 || counts != self.mode.kernel_profile().kernel_counts() {
            return Err("prepared complete kernel/collective counts required".into());
        }
        Ok(protocol::Program {
            profile: self
                .mode
                .geometry()
                .program_profile(self.mode.kernel_profile()),
            reference_source_sha256: protocol::REFERENCE_SOURCE.into(),
            native_source_sha256: self.mode.source().into(),
            unique_ids: self.ids,
            group_id: program.group_id,
            catalog: self.catalog.clone(),
            token_buffer: program.token_buffer,
            result_buffer: program.result_buffer,
            metadata: program.metadata.map(|value| protocol::RankMetadata {
                positions: value.positions,
                page_table: value.page_table,
                cos: value.cos,
                sin: value.sin,
            }),
            steps,
        })
    }

    fn register(&mut self, program: &Program) -> TpResult<[u8; 32]> {
        let packed = match self.pack_program(program) {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        let finite = self
            .mode
            .graph_policy()
            .is_some_and(GraphPolicy::finite_request);
        let bytes = if finite {
            serde_json::to_vec(&scope::graph::FiniteProgram {
                geometry: self.mode.geometry(),
                budget: scope::graph::FiniteBudget::for_geometry(self.mode.geometry()),
                program: packed.clone(),
            })
        } else {
            match self.mode.geometry() {
                scope::graph::GraphGeometry::Short64 => serde_json::to_vec(&packed),
                scope::graph::GraphGeometry::Long2304 => {
                    serde_json::to_vec(&scope::graph::LongProgram {
                        geometry: scope::graph::GraphGeometry::Long2304,
                        program: packed.clone(),
                    })
                }
            }
        }
        .map_err(|error| error.to_string())?;
        let hash = digest(&bytes);
        let catalog =
            digest(&serde_json::to_vec(&packed.catalog).map_err(|error| error.to_string())?);
        if bytes.is_empty() || bytes.len() > protocol::MAX_PROGRAM_BYTES {
            return self.reject("prepared program exceeds framing bound");
        }
        let id = self.next_request;
        let header = protocol::Request::Register {
            id,
            program_sha256: hash,
            program_bytes: u32::try_from(bytes.len())
                .map_err(|_| "prepared program length overflow")?,
        };
        let response = if finite {
            let reply = self.exchange_reply(header, bytes)?;
            let response = match reply {
                scope::Reply::Graph(response)
                    if self.mode.geometry() == scope::graph::GraphGeometry::Short64 =>
                {
                    response
                }
                scope::Reply::LongGraph(envelope) if envelope.geometry == self.mode.geometry() => {
                    envelope.response
                }
                _ => return self.reject("finite begin wrong response envelope"),
            };
            match validate_finite_registered(
                &response,
                id,
                hash,
                catalog,
                packed.group_id,
                self.mode,
            ) {
                Ok(response) => response,
                Err(error) => return self.reject(error),
            }
        } else {
            self.exchange(header, bytes)?.0
        };
        if !matches!(response, protocol::Response::Registered {
            id: actual, plan_sha256, catalog_sha256, steps, kernel_counts,
        } if actual == id && plan_sha256 == hash && catalog_sha256 == catalog
            && steps as usize == self.mode.kernel_profile().steps()
            && kernel_counts == self.mode.kernel_profile().kernel_counts())
        {
            return self.reject("prepared registration identity/cardinality mismatch");
        }
        self.registration = Some(Registration {
            program: program.clone(),
            hash,
            generation: 1,
            pages: vec![
                u32::MAX;
                usize::try_from(self.mode.geometry().pages())
                    .map_err(|_| "graph pages bound")?
            ],
            geometry: graph_geometry(self.mode),
        });
        self.phase = Phase::Ready;
        Ok(hash)
    }

    fn execute(&mut self, input: &Input) -> TpResult<Receipt> {
        if self.phase != Phase::Ready || self.mode.graph_policy().is_some() {
            return self.reject("prepared execute without live registration");
        }
        let registered = self
            .registration
            .as_ref()
            .ok_or("prepared registration missing")?;
        if let Err(error) = validate_input(registered, input) {
            return self.reject(error);
        }
        let Some(next_native) =
            self.next_native_request
                .checked_add(if self.mode == PreparedMode::Interpreter {
                    1023
                } else {
                    0
                })
        else {
            return self.reject("prepared internal request counter overflow");
        };
        let id = self.next_request;
        self.phase = Phase::Executing;
        let (response, native) = self.exchange(
            protocol::Request::Execute {
                execution: protocol::Execute {
                    id,
                    plan_sha256: input.plan_sha256,
                    generation: input.generation,
                    epoch: input.epoch,
                    token: input.token,
                    position: input.position,
                    page_table: input.page_table,
                },
            },
            input.cos_sin.clone(),
        )?;
        let protocol::Response::Executed { receipt: actual } = response else {
            return self.reject("prepared whole-token completion missing");
        };
        let registered = self
            .registration
            .as_ref()
            .ok_or("prepared registration lost")?;
        match (self.mode, native) {
            (PreparedMode::Interpreter, None) => {}
            (PreparedMode::NativeProgram, Some(actual)) => {
                if let Err(error) =
                    scope::validate_completion(&actual, &registered.program, input, self.ids)
                {
                    return self.reject(error);
                }
            }
            _ => return self.reject("prepared native completion mode mismatch"),
        }
        let receipt = match validate_receipt_mode(
            &registered.program,
            input,
            &actual,
            self.ids,
            id,
            self.next_native_request,
            self.mode,
        ) {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        let registered = self
            .registration
            .as_mut()
            .ok_or("prepared registration lost")?;
        registered.generation += 1;
        registered.pages = input.page_table.to_vec();
        self.next_native_request = next_native;
        self.phase = Phase::Ready;
        Ok(receipt)
    }

    fn execute_graph(&mut self, input: &Input) -> TpResult<GraphReceipt> {
        self.execute_geometry_graph(&GraphInput::from(input))
    }

    fn execute_geometry_graph(&mut self, input: &GraphInput) -> TpResult<GraphReceipt> {
        let Some(policy) = self.mode.graph_policy() else {
            return self.reject("prepared graph requires its immutable launch mode");
        };
        if self.phase != Phase::Ready {
            return self.reject("prepared graph execute without live registration");
        }
        let Some(registered) = self.registration.as_ref() else {
            return self.reject("prepared graph registration missing");
        };
        if input.geometry != graph_geometry(self.mode) {
            return self.reject("graph input differs from immutable launch geometry");
        }
        if policy.finite_request()
            && input.generation
                > u64::from(scope::graph::FiniteBudget::for_geometry(self.mode.geometry()).forwards)
        {
            return self.reject("finite request exhausted before IPC");
        }
        if let Err(error) = validate_graph_input(registered, input) {
            return self.reject(error);
        }
        let Some(next_generation) = registered.generation.checked_add(1) else {
            return self.reject("prepared graph generation overflow");
        };
        let id = self.next_request;
        self.phase = Phase::Executing;
        let header = match input.geometry {
            GraphGeometry::Short64 => scope::Request::Short(protocol::Request::Execute {
                execution: protocol::Execute {
                    id,
                    plan_sha256: input.plan_sha256,
                    generation: input.generation,
                    epoch: input.epoch,
                    token: input.token,
                    position: input.position,
                    page_table: input
                        .page_table
                        .as_slice()
                        .try_into()
                        .map_err(|_| "short graph pages")?,
                },
            }),
            GraphGeometry::Long2304 => scope::Request::Long(scope::graph::LongRequest {
                geometry: scope::graph::GraphGeometry::Long2304,
                request: scope::graph::LongRequestBody::Execute {
                    execution: scope::graph::LongExecute {
                        id,
                        plan_sha256: input.plan_sha256,
                        generation: input.generation,
                        epoch: input.epoch,
                        token: input.token,
                        position: input.position,
                        page_table: input.page_table.clone(),
                    },
                },
            }),
        };
        let response = self.exchange_wire_reply(header, input.cos_sin.clone())?;
        let actual = match response {
            scope::Reply::Graph(scope::graph::Response::Executed { receipt })
                if input.geometry == GraphGeometry::Short64 =>
            {
                receipt
            }
            scope::Reply::LongGraph(scope::graph::LongResponse {
                geometry: scope::graph::GraphGeometry::Long2304,
                response: scope::graph::Response::Executed { receipt },
            }) if input.geometry == GraphGeometry::Long2304 => receipt,
            scope::Reply::Graph(scope::graph::Response::Fatal { message, .. }) => {
                return self.reject(format!("prepared graph child terminal: {message}"));
            }
            _ => return self.reject("prepared graph completion missing or wrong wire mode"),
        };
        let Some(registered) = self.registration.as_ref() else {
            return self.reject("prepared graph registration lost");
        };
        let receipt = match scope::validate_geometry_graph_receipt(
            actual,
            &registered.program,
            input,
            self.ids,
            id,
            policy,
            GraphKernels::ALL
                .into_iter()
                .find(|profile| profile.argument() == self.mode.kernel_profile().argument())
                .ok_or("graph profile mapping mismatch")?,
        ) {
            Ok(value) => value,
            Err(error) => return self.reject(error),
        };
        let Some(registered) = self.registration.as_mut() else {
            return self.reject("prepared graph registration lost before commit");
        };
        registered.generation = next_generation;
        registered.pages.clone_from(&input.page_table);
        self.phase = Phase::Ready;
        Ok(receipt)
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
                .map_err(|error| format!("prepared child kill: {error}"))?;
        }
        self.await_exit(false)
    }

    fn await_exit(&mut self, successful: bool) -> TpResult<()> {
        let started = Instant::now();
        let finite = successful
            && self
                .mode
                .graph_policy()
                .is_some_and(GraphPolicy::finite_request);
        let mut trailing = false;
        loop {
            while self.reader.try_recv().is_ok() {
                trailing = true;
            }
            if let Some(status) = self.child.try_wait().map_err(|error| error.to_string())? {
                self.exited = true;
                self.writer.take();
                while self.threads.iter().any(|thread| !thread.is_finished()) {
                    while self.reader.try_recv().is_ok() {
                        trailing = true;
                    }
                    if started.elapsed() >= EXIT_TIMEOUT {
                        return Err("prepared IPC thread exit unconfirmed".into());
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                let mut join_failed = false;
                for thread in self.threads.drain(..) {
                    join_failed |= thread.join().is_err();
                }
                while self.reader.try_recv().is_ok() {
                    trailing = true;
                }
                return if finite
                    && (trailing
                        || join_failed
                        || self
                            .finite_clean_eof
                            .as_ref()
                            .is_none_or(|eof| eof.try_recv().is_err()))
                {
                    Err(
                        "finite close requires exact clean EOF and successful IPC thread joins"
                            .into(),
                    )
                } else if successful && !status.success() {
                    Err(format!("prepared child exited with {status}"))
                } else {
                    Ok(())
                };
            }
            if started.elapsed() >= EXIT_TIMEOUT {
                return Err("prepared child exit unconfirmed".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn validate_finite_registered(
    response: &scope::graph::Response,
    id: u64,
    hash: [u8; 32],
    catalog: [u8; 32],
    group: u64,
    mode: PreparedMode,
) -> TpResult<protocol::Response> {
    let scope::graph::Response::RegisteredFinite {
        id: actual,
        plan_sha256,
        catalog_sha256,
        group_id,
        geometry,
        budget,
        full_boundaries,
        steps,
        kernel_counts,
    } = *response
    else {
        return Err("finite begin acknowledgement missing".into());
    };
    if !mode.graph_policy().is_some_and(GraphPolicy::finite_request)
        || actual != id
        || plan_sha256 != hash
        || catalog_sha256 != catalog
        || group_id != group
        || geometry != mode.geometry()
        || budget != scope::graph::FiniteBudget::for_geometry(geometry)
        || full_boundaries != 1
        || steps as usize != mode.kernel_profile().steps()
        || kernel_counts != mode.kernel_profile().kernel_counts()
    {
        return Err("finite begin identity/budget/cardinality substitution".into());
    }
    Ok(protocol::Response::Registered {
        id: actual,
        plan_sha256,
        catalog_sha256,
        steps,
        kernel_counts,
    })
}

impl Drop for Connection {
    fn drop(&mut self) {
        if !self.exited
            && let Err(error) = self.terminate()
        {
            eprintln!("prepared child cleanup: {error}");
        }
    }
}

fn operation(value: Qwen3TensorParallelCollectiveV1) -> dep::Operation {
    match value {
        Qwen3TensorParallelCollectiveV1::AttentionOutputSum => dep::Operation::AttentionOutputSum,
        Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => dep::Operation::FeedForwardDownSum,
    }
}

fn validate_input(registered: &Registration, input: &Input) -> TpResult<()> {
    validate_graph_input(registered, &GraphInput::from(input))
}

fn validate_graph_input(registered: &Registration, input: &GraphInput) -> TpResult<()> {
    let pages = usize::try_from(input.geometry.pages()).map_err(|_| "graph page extent")?;
    if input.geometry != registered.geometry
        || input.page_table.len() != pages
        || registered.pages.len() != pages
        || input.plan_sha256 != registered.hash
        || input.generation != registered.generation
        || input.epoch.checked_add(1) != Some(input.generation)
        || u64::from(input.position) != input.epoch
        || input.position >= input.geometry.context_tokens()
        || input.token >= protocol::VOCABULARY
        || input.cos_sin.len() != 512
        || !input.cos_sin.chunks_exact(4).all(|bytes| {
            let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            value.is_finite() && value.abs() <= 1.0
        })
    {
        return Err("prepared execute identity, token or metadata bound".into());
    }
    let active = input.position as usize / 16 + 1;
    let previous =
        input.position.saturating_sub(1) as usize / 16 + usize::from(input.position != 0);
    if input.page_table[..active]
        .iter()
        .any(|value| *value >= input.geometry.pages())
        || input.page_table[active..]
            .iter()
            .any(|value| *value != u32::MAX)
        || input.page_table[..previous] != registered.pages[..previous]
        || input.page_table[..active]
            .iter()
            .enumerate()
            .any(|(index, value)| input.page_table[..index].contains(value))
    {
        return Err("prepared page progression mismatch".into());
    }
    Ok(())
}

#[cfg(test)]
fn validate_receipt(
    program: &Program,
    input: &Input,
    actual: &protocol::ExecutionReceipt,
    ids: [u64; 2],
    request_id: u64,
    native_request: u64,
) -> TpResult<Receipt> {
    validate_receipt_mode(
        program,
        input,
        actual,
        ids,
        request_id,
        native_request,
        PreparedMode::Interpreter,
    )
}

fn validate_receipt_mode(
    program: &Program,
    input: &Input,
    actual: &protocol::ExecutionReceipt,
    ids: [u64; 2],
    request_id: u64,
    native_request: u64,
    mode: PreparedMode,
) -> TpResult<Receipt> {
    if actual.id != request_id
        || actual.unique_ids != ids
        || actual.ordinary_completions != 941
        || actual.collectives.len() != 72
    {
        return Err("prepared response transport identity mismatch".into());
    }
    let mut collectives = Vec::with_capacity(72);
    let mut next_native = if mode == PreparedMode::Interpreter {
        native_request
            .checked_add(9)
            .ok_or("prepared native cursor overflow")?
    } else {
        0
    };
    for step in &program.steps {
        if let Step::Collective(template) = step {
            let record = actual
                .collectives
                .get(collectives.len())
                .ok_or("prepared extra collective template")?;
            record.validate().map_err(|error| error.to_string())?;
            let generation = input
                .epoch
                .checked_mul(72)
                .and_then(|value| value.checked_add(collectives.len() as u64 + 1))
                .ok_or("prepared native generation overflow")?;
            if record.identity
                != (dep::Identity {
                    request_id: if mode == PreparedMode::NativeProgram {
                        generation
                    } else {
                        next_native
                    },
                    generation,
                    group_id: program.group_id,
                    model_role: dep::ModelRole::Target8b,
                    epoch: input.epoch,
                    layer: template.key.layer,
                    operation: operation(template.key.operation),
                })
                || record.queues.map(|queue| queue.unique_id) != ids
            {
                return Err("prepared actual collective request/device substitution".into());
            }
            let mut key = template.key;
            key.epoch = input.epoch;
            collectives.push(EngineeringTp2CollectiveReceiptV1 {
                key,
                generation,
                kernel_dispatches: record.kernel_counts,
                barrier_packets: record.barrier_counts,
                packet_counts: record.packet_counts,
                completion_values: std::array::from_fn(|slot| {
                    record.completion_values[slot / 3][slot % 3]
                }),
                queue_epochs: record.queues.map(|queue| queue.queue_epoch),
                first_packet_ids: record.queues.map(|queue| queue.first_packet),
                frontiers: record.final_frontiers,
            });
        }
        if mode == PreparedMode::Interpreter {
            next_native = next_native
                .checked_add(1)
                .ok_or("prepared native cursor overflow")?;
        }
    }
    let receipt = Receipt {
        plan_sha256: actual.plan_sha256,
        generation: actual.generation,
        epoch: actual.epoch,
        position: actual.position,
        input_token: actual.input_token,
        output_token: actual.output_token,
        kernel_counts: actual.kernel_counts,
        barrier_counts: actual.barrier_counts,
        packet_counts: actual.packet_counts,
        collectives,
    };
    receipt.validate_for(program, input)?;
    Ok(receipt)
}

pub struct PreparedWorker {
    connection: Rc<RefCell<Connection>>,
    rank: usize,
}

impl PreparedWorker {
    pub fn spawn_graph_with_options(
        executable: &Path,
        unique_ids: &[u64],
        artifacts: &[&EngineeringTpArtifactV1],
        options: RuntimeOptions,
        timing: HostTiming,
        policy: GraphPolicy,
        launch: GraphLaunchOptions,
    ) -> TpResult<Vec<Self>> {
        Self::spawn_graph_with_geometry_options(
            executable,
            unique_ids,
            artifacts,
            options,
            timing,
            policy,
            GraphGeometryLaunchOptions {
                kernel_profile: launch.kernel_profile,
                metadata_upload_mode: launch.metadata_upload_mode,
                geometry: GraphGeometry::Short64,
            },
        )
    }

    pub fn spawn_graph_with_geometry_options(
        executable: &Path,
        unique_ids: &[u64],
        artifacts: &[&EngineeringTpArtifactV1],
        options: RuntimeOptions,
        timing: HostTiming,
        policy: GraphPolicy,
        launch: GraphGeometryLaunchOptions,
    ) -> TpResult<Vec<Self>> {
        let mode = match policy {
            GraphPolicy::QueuedBaseline => scope::graph::ExecutionMode::QueuedBaseline,
            GraphPolicy::TransactionFences => scope::graph::ExecutionMode::TransactionFences,
            GraphPolicy::TransactionFencesAdmissionCache => {
                scope::graph::ExecutionMode::TransactionFencesAdmissionCache
            }
            GraphPolicy::TransactionFencesAdmissionCacheScopedObservations => {
                scope::graph::ExecutionMode::TransactionFencesAdmissionCacheScopedObservations
            }
            GraphPolicy::ClosedTokenAdmissionCache => {
                scope::graph::ExecutionMode::ClosedTokenAdmissionCache
            }
            GraphPolicy::FiniteRequestAdmissionCache => {
                scope::graph::ExecutionMode::FiniteRequestAdmissionCache
            }
        };
        let profile = match launch.kernel_profile {
            GraphKernels::Baseline => protocol::KernelProfile::Baseline,
            GraphKernels::V22Scalar => protocol::KernelProfile::V22Scalar,
            GraphKernels::V22Wave => protocol::KernelProfile::V22Wave,
            GraphKernels::WaveStackControl => protocol::KernelProfile::WaveStackControl,
            GraphKernels::WaveStackNorm => protocol::KernelProfile::WaveStackNorm,
            GraphKernels::WaveStackNormAttention => protocol::KernelProfile::WaveStackNormAttention,
            GraphKernels::WaveStackNormAttentionKv => {
                protocol::KernelProfile::WaveStackNormAttentionKv
            }
            GraphKernels::WaveStackNormAttentionKvMlp => {
                protocol::KernelProfile::WaveStackNormAttentionKvMlp
            }
            GraphKernels::WaveStackNormSplitAttentionKvMlp => {
                protocol::KernelProfile::WaveStackNormSplitAttentionKvMlp
            }
        };
        Self::spawn_with_mode(
            executable,
            unique_ids,
            artifacts,
            options,
            timing,
            match launch.geometry {
                GraphGeometry::Short64 => {
                    PreparedMode::GraphOptions(mode, profile, launch.metadata_upload_mode)
                }
                GraphGeometry::Long2304 => {
                    PreparedMode::LongGraph(mode, profile, launch.metadata_upload_mode)
                }
            },
        )
    }

    pub fn spawn_graph_with_timing(
        executable: &Path,
        unique_ids: &[u64],
        artifacts: &[&EngineeringTpArtifactV1],
        options: RuntimeOptions,
        timing: HostTiming,
        policy: GraphPolicy,
    ) -> TpResult<Vec<Self>> {
        let mode = match policy {
            GraphPolicy::QueuedBaseline => scope::graph::ExecutionMode::QueuedBaseline,
            GraphPolicy::TransactionFences => scope::graph::ExecutionMode::TransactionFences,
            GraphPolicy::TransactionFencesAdmissionCache => {
                scope::graph::ExecutionMode::TransactionFencesAdmissionCache
            }
            GraphPolicy::TransactionFencesAdmissionCacheScopedObservations => {
                scope::graph::ExecutionMode::TransactionFencesAdmissionCacheScopedObservations
            }
            GraphPolicy::ClosedTokenAdmissionCache => {
                scope::graph::ExecutionMode::ClosedTokenAdmissionCache
            }
            GraphPolicy::FiniteRequestAdmissionCache => {
                scope::graph::ExecutionMode::FiniteRequestAdmissionCache
            }
        };
        Self::spawn_with_mode(
            executable,
            unique_ids,
            artifacts,
            options,
            timing,
            PreparedMode::Graph(mode),
        )
    }

    pub fn spawn_program_with_timing(
        executable: &Path,
        unique_ids: &[u64],
        artifacts: &[&EngineeringTpArtifactV1],
        options: RuntimeOptions,
        timing: HostTiming,
    ) -> TpResult<Vec<Self>> {
        Self::spawn_with_mode(
            executable,
            unique_ids,
            artifacts,
            options,
            timing,
            PreparedMode::NativeProgram,
        )
    }

    pub fn spawn_with_timing(
        executable: &Path,
        unique_ids: &[u64],
        artifacts: &[&EngineeringTpArtifactV1],
        options: RuntimeOptions,
        timing: HostTiming,
    ) -> TpResult<Vec<Self>> {
        Self::spawn_with_mode(
            executable,
            unique_ids,
            artifacts,
            options,
            timing,
            PreparedMode::Interpreter,
        )
    }

    fn spawn_with_mode(
        executable: &Path,
        unique_ids: &[u64],
        artifacts: &[&EngineeringTpArtifactV1],
        options: RuntimeOptions,
        timing: HostTiming,
        mode: PreparedMode,
    ) -> TpResult<Vec<Self>> {
        let ids = validate_options(unique_ids, options, timing.is_enabled())?;
        validate_admission_option(mode, options.cache_admission)?;
        validate_metadata_option(mode)?;
        let profile = mode.kernel_profile();
        if artifacts.len()
            != 2 + usize::from(profile.has_v22())
                + usize::from(profile.has_v15())
                + usize::from(profile.split_attention())
            || hex(&digest(artifacts[0].bytes())) != protocol::MAIN_IMAGE
            || hex(&digest(artifacts[1].bytes())) != protocol::PEER_IMAGE
            || artifacts[0].inspection().hsaco().kernels().len() != 15
            || artifacts[1].inspection().hsaco().kernels().len() != 2
        {
            return Err("prepared requires the exact admitted main15/peer2 images in order".into());
        }
        if profile.has_v22() && !artifacts[2].is_graph_bf16_argmax_v22() {
            return Err(
                "graph V22 requires the exact ordinarily admitted rank-zero sidecar".into(),
            );
        }
        if profile.has_v15() && !artifacts[3].is_graph_wave_rmsnorm_v15() {
            return Err(
                "wave-stack requires the exact ordinarily admitted V15 sidecar on both ranks"
                    .into(),
            );
        }
        if profile.split_attention() && !artifacts[4].is_graph_split_attention_v1() {
            return Err(
                "split attention requires the exact ordinarily admitted two-root sidecar".into(),
            );
        }
        let mut command = Command::new(executable);
        command.args([
            "--allow-unauthenticated-machine-code",
            "--device-unique-ids",
            &format!("{},{}", ids[0], ids[1]),
        ]);
        command.args(mode.launch_arguments());
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("spawn prepared child: {error}"))?;
        let connection = Rc::new(RefCell::new(Connection::connect_mode(
            child,
            ids,
            IPC_TIMEOUT,
            timing,
            mode,
        )?));
        {
            let mut state = connection.borrow_mut();
            let response = state.setup(
                0,
                false,
                CommandV1::ConfigurePerformance {
                    cache_kernel_admission: options.cache_admission,
                    operational_currentness: false,
                    profile: false,
                },
                vec![],
            )?;
            if !matches!(response, ResponseV1::PerformanceConfigured) {
                return state.reject("prepared setup policy receipt mismatch");
            }
            for rank in 0..2 {
                for artifact in &artifacts[..2] {
                    let hash = digest(artifact.bytes());
                    for metadata in artifact.inspection().hsaco().kernels() {
                        let response = state.setup(
                            rank,
                            false,
                            CommandV1::LoadKernel {
                                payload_bytes: u32::try_from(artifact.bytes().len())
                                    .map_err(|_| "prepared HSACO length")?,
                                object_sha256: hash,
                                symbol: metadata.name().into(),
                            },
                            artifact.bytes().to_vec(),
                        )?;
                        let ResponseV1::LoadedKernel {
                            kernel,
                            metadata: actual,
                        } = response
                        else {
                            return state
                                .reject("prepared actual kernel admission response missing");
                        };
                        if kernel == 0
                            || state.catalog.kernels.iter().any(|entry| entry.id == kernel)
                            || state.kernels[rank].contains_key(metadata.name())
                            || !metadata_matches(metadata, &actual, hash)
                        {
                            return state
                                .reject("prepared actual kernel metadata/identity mismatch");
                        }
                        state.catalog.kernels.push(protocol::KernelBinding {
                            id: kernel,
                            rank: u32::try_from(rank)
                                .map_err(|_| "prepared kernel rank overflow")?,
                            metadata: actual,
                        });
                        state.kernels[rank].insert(
                            metadata.name().into(),
                            LoadedKernel {
                                id: kernel,
                                image: hash,
                                metadata: metadata.clone(),
                            },
                        );
                    }
                }
            }
            if profile.has_v22() {
                state.load_graph_argmax_v22(artifacts[2])?;
            }
            if profile.has_v15() {
                state.load_graph_norm_v15(artifacts[3], 0)?;
                state.load_graph_norm_v15(artifacts[3], 1)?;
            }
            if profile.split_attention() {
                state.load_graph_split_attention(artifacts[4], 0)?;
                state.load_graph_split_attention(artifacts[4], 1)?;
            }
            state.catalog.kernels.sort_by_key(|value| value.id);
        }
        Ok((0..2)
            .map(|rank| Self {
                connection: Rc::clone(&connection),
                rank,
            })
            .collect())
    }

    pub fn pid(&self) -> u32 {
        self.connection.borrow().child.id()
    }

    fn allocate_inner(&mut self, bytes: usize, peer: bool) -> TpResult<u64> {
        let mut state = self.connection.borrow_mut();
        if bytes == 0 {
            return state.reject("prepared empty allocation");
        }
        let response = state.setup(
            self.rank,
            peer,
            CommandV1::Allocate {
                bytes: bytes as u64,
            },
            vec![],
        )?;
        let ResponseV1::Allocated {
            buffer,
            bytes: actual,
        } = response
        else {
            return state.reject("prepared allocation receipt missing");
        };
        if buffer == 0 || actual != bytes as u64 || state.capacities.insert(buffer, bytes).is_some()
        {
            return state.reject("prepared allocation identity/size mismatch");
        }
        state.catalog.buffers.push(protocol::BufferBinding {
            id: buffer,
            rank: u32::try_from(self.rank).map_err(|_| "prepared allocation rank overflow")?,
            bytes: actual,
            peer_readable: peer,
        });
        state.catalog.buffers.sort_by_key(|value| value.id);
        Ok(buffer)
    }
}

impl EngineeringTpRankTransportV1 for PreparedWorker {
    fn setup_upload_chunk_bytes(&self) -> usize {
        setup_upload_chunk_bytes(self.connection.borrow().mode)
    }
    fn supports_peer_dependency_collectives(&self) -> bool {
        let state = self.connection.borrow();
        matches!(state.phase, Phase::Setup | Phase::Ready)
            && !state.exited
            && !state.closed.iter().any(|value| *value)
    }
    fn supports_prepared_peer(&self) -> bool {
        self.connection.borrow().mode.graph_policy().is_none()
            && self.supports_peer_dependency_collectives()
    }
    fn supports_prepared_peer_graph(&self, policy: GraphPolicy) -> bool {
        self.connection.borrow().mode.graph_policy() == Some(policy)
            && self.supports_peer_dependency_collectives()
    }
    fn supports_prepared_peer_graph_profile(
        &self,
        policy: GraphPolicy,
        profile: GraphKernels,
    ) -> bool {
        self.supports_prepared_peer_graph(policy)
            && graph_geometry(self.connection.borrow().mode) == GraphGeometry::Short64
            && self.connection.borrow().mode.kernel_profile().argument() == profile.argument()
    }
    fn supports_prepared_peer_graph_geometry(
        &self,
        policy: GraphPolicy,
        profile: GraphKernels,
        geometry: GraphGeometry,
    ) -> bool {
        self.supports_prepared_peer_graph(policy)
            && graph_geometry(self.connection.borrow().mode) == geometry
            && self.connection.borrow().mode.kernel_profile().argument() == profile.argument()
    }
    fn register_prepared_peer_graph(
        &mut self,
        program: &Program,
        policy: GraphPolicy,
    ) -> TpResult<[u8; 32]> {
        let mut state = self.connection.borrow_mut();
        if self.rank != 0
            || state.mode.graph_policy() != Some(policy)
            || graph_geometry(state.mode) != GraphGeometry::Short64
        {
            return state.reject("prepared graph registration rank or immutable policy mismatch");
        }
        match state.register(program) {
            Ok(value) => Ok(value),
            Err(error) => state.reject(error),
        }
    }
    fn execute_prepared_peer_graph(&mut self, input: &Input) -> TpResult<GraphReceipt> {
        let mut state = self.connection.borrow_mut();
        if self.rank != 0 || state.mode.graph_policy().is_none() {
            return state.reject("prepared graph execute rank or launch mode mismatch");
        }
        match state.execute_graph(input) {
            Ok(value) => Ok(value),
            Err(error) => state.reject(error),
        }
    }
    fn register_prepared_peer_graph_geometry(
        &mut self,
        program: &Program,
        policy: GraphPolicy,
        geometry: GraphGeometry,
    ) -> TpResult<[u8; 32]> {
        let mut state = self.connection.borrow_mut();
        if self.rank != 0
            || state.mode.graph_policy() != Some(policy)
            || graph_geometry(state.mode) != geometry
        {
            return state.reject("graph registration rank, policy or geometry mismatch");
        }
        match state.register(program) {
            Ok(value) => Ok(value),
            Err(error) => state.reject(error),
        }
    }
    fn execute_prepared_peer_graph_geometry(
        &mut self,
        input: &GraphInput,
    ) -> TpResult<GraphReceipt> {
        let mut state = self.connection.borrow_mut();
        if self.rank != 0
            || state.mode.graph_policy().is_none()
            || graph_geometry(state.mode) != input.geometry
        {
            return state.reject("graph execution rank, policy or geometry mismatch");
        }
        match state.execute_geometry_graph(input) {
            Ok(value) => Ok(value),
            Err(error) => state.reject(error),
        }
    }
    fn register_prepared_peer(&mut self, program: &Program) -> TpResult<[u8; 32]> {
        let mut state = self.connection.borrow_mut();
        if self.rank != 0 || state.mode.graph_policy().is_some() {
            return state.reject("prepared registration belongs to rank zero");
        }
        match state.register(program) {
            Ok(value) => Ok(value),
            Err(error) => state.reject(error),
        }
    }
    fn execute_prepared_peer(&mut self, input: &Input) -> TpResult<Receipt> {
        let mut state = self.connection.borrow_mut();
        if self.rank != 0 || state.mode.graph_policy().is_some() {
            return state.reject("prepared execute belongs to rank zero");
        }
        match state.execute(input) {
            Ok(value) => Ok(value),
            Err(error) => state.reject(error),
        }
    }
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        Some((self.pid(), u32::try_from(self.rank).ok()?, 2))
    }
    fn require_loaded_image(&mut self, image: [u8; 32], kernels: &[&str]) -> TpResult<()> {
        let mut state = self.connection.borrow_mut();
        if !matches!(state.phase, Phase::Setup | Phase::Ready)
            || kernels.is_empty()
            || kernels.iter().any(|name| {
                state.kernels[self.rank]
                    .get(*name)
                    .is_none_or(|kernel| kernel.image != image)
            })
        {
            return state.reject("prepared loaded image requirement mismatch");
        }
        Ok(())
    }
    fn allocate(&mut self, bytes: usize) -> TpResult<u64> {
        self.allocate_inner(bytes, false)
    }
    fn allocate_peer_readable(&mut self, bytes: usize) -> TpResult<u64> {
        self.allocate_inner(bytes, true)
    }
    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()> {
        let mut state = self.connection.borrow_mut();
        if let Err(error) = state.check_extent(self.rank, buffer, offset, bytes.len()) {
            return state.reject(error);
        }
        let Ok(length) = u32::try_from(bytes.len()) else {
            return state.reject("prepared write length");
        };
        let response = state.setup(
            self.rank,
            false,
            CommandV1::Write {
                buffer,
                offset: offset as u64,
                payload_bytes: length,
            },
            bytes.to_vec(),
        )?;
        if !matches!(response, ResponseV1::Written) {
            return state.reject("prepared write receipt mismatch");
        }
        Ok(())
    }
    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        self.connection
            .borrow_mut()
            .reject("prepared result read is part of Execute only")
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        self.connection
            .borrow_mut()
            .reject("prepared ordinary dispatch requires registered program")
    }
    fn wait(&mut self) -> TpResult<()> {
        self.connection
            .borrow_mut()
            .reject("prepared has no standalone pending dispatch")
    }
    fn close(&mut self) -> TpResult<()> {
        let mut state = self.connection.borrow_mut();
        if state
            .mode
            .graph_policy()
            .is_some_and(GraphPolicy::finite_request)
            && state.phase == Phase::Terminal
        {
            return Err("finite close is permanently terminal".into());
        }
        if state.exited {
            if state
                .mode
                .graph_policy()
                .is_some_and(GraphPolicy::finite_request)
                && state.phase != Phase::Closed
            {
                return Err("finite owner exited without accepted clean close".into());
            }
            return Ok(());
        }
        if state.closed[self.rank] {
            return Ok(());
        }
        state.closed[self.rank] = true;
        if !state.closed.iter().all(|value| *value) {
            return Ok(());
        }
        if state
            .mode
            .graph_policy()
            .is_some_and(GraphPolicy::finite_request)
            && state.registration.as_ref().is_none_or(|registered| {
                registered.generation
                    != u64::from(
                        scope::graph::FiniteBudget::for_geometry(state.mode.geometry()).forwards,
                    ) + 1
            })
        {
            return state.reject("finite request close before validated exact completion");
        }
        let id = state.next_request;
        let (response, native) = state.exchange(protocol::Request::Close { id }, vec![])?;
        if native.is_some() {
            return state.reject("unexpected native execution metadata on Close");
        }
        if !matches!(response, protocol::Response::Closed { id: actual } if actual == id) {
            return state.reject("prepared close receipt mismatch");
        }
        state.phase = Phase::Closed;
        match state.await_exit(true) {
            Ok(()) => Ok(()),
            Err(error) => state.reject(error),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn graph_admission_option_matches_only_its_closed_launch_mode() {
        for mode in scope::graph::ExecutionMode::PRE_FINITE {
            for cache in [false, true] {
                assert_eq!(
                    validate_admission_option(PreparedMode::Graph(mode), cache).is_ok(),
                    cache == mode.admission_cache()
                );
                for profile in scope::graph::KernelProfile::ALL {
                    for metadata in MetadataUploadMode::ALL {
                        let selected = PreparedMode::GraphOptions(mode, profile, metadata);
                        assert_eq!(
                            validate_admission_option(selected, cache).is_ok(),
                            cache == mode.admission_cache()
                        );
                        assert_eq!(setup_upload_chunk_bytes(selected), 4 << 20);
                    }
                }
            }
        }
        assert!(validate_admission_option(PreparedMode::NativeProgram, false).is_ok());
        assert!(validate_admission_option(PreparedMode::NativeProgram, true).is_err());
        assert!(validate_admission_option(PreparedMode::Interpreter, false).is_ok());
        assert!(validate_admission_option(PreparedMode::Interpreter, true).is_ok());
    }

    #[test]
    fn setup_upload_capability_is_graph_only_and_obeys_existing_wire_limit() {
        use super::{PreparedMode, scope, setup_upload_chunk_bytes};
        for mode in [PreparedMode::Interpreter, PreparedMode::NativeProgram] {
            assert_eq!(setup_upload_chunk_bytes(mode), 1 << 20);
        }
        for mode in scope::graph::ExecutionMode::PRE_FINITE {
            let bytes = setup_upload_chunk_bytes(PreparedMode::Graph(mode));
            assert_eq!(bytes, 4 << 20);
            let bytes = u32::try_from(bytes).unwrap();
            let command = |payload_bytes| super::CommandV1::Write {
                buffer: 1,
                offset: 0,
                payload_bytes,
            };
            assert_eq!(
                command(bytes).payload_bytes().unwrap(),
                usize::try_from(bytes).unwrap()
            );
            assert!(command(bytes + 1).payload_bytes().is_err());
        }
    }

    include!("tp_prepared_worker_fixture.rs");
    include!("tp_prepared_graph_tests.rs");
    include!("tp_prepared_closed_token_tests.rs");
    include!("tp_prepared_finite_request_tests.rs");
    include!("tp_prepared_finite_begin_tests.rs");
    use super::*;
    use ferric_engine::tensor_parallel::Qwen3TensorParallelCollectiveKeyV1;
    use ferric_m1_engineering_execution_v1::tp_execution::{
        EngineeringTp2CollectiveRequestV1, EngineeringTp2PreparedMetadataV1,
    };
    use ferric_spec::Qwen3ModelRole;

    fn slice(id: u64, elements: usize, element_bytes: u32, access: Access) -> Arg {
        Arg::Buffer {
            id,
            offset: 0,
            elements,
            element_bytes,
            access,
        }
    }

    fn collective(
        layer: u32,
        operation: Qwen3TensorParallelCollectiveV1,
    ) -> EngineeringTp2CollectiveRequestV1 {
        let (k, tag) = if operation == Qwen3TensorParallelCollectiveV1::AttentionOutputSum {
            (2048, 1)
        } else {
            (6144, 2)
        };
        EngineeringTp2CollectiveRequestV1 {
            key: Qwen3TensorParallelCollectiveKeyV1 {
                group_id: 7,
                model_role: Qwen3ModelRole::Target8B,
                epoch: 0,
                layer,
                operation,
            },
            rows: 1,
            producers: std::array::from_fn(|rank| EngineeringTpDispatchV1 {
                kernel: dep::PRODUCER_ROOT,
                grid_workgroups: 256,
                workgroup_size: 64,
                arguments: vec![
                    slice(1 + rank as u64 * 3, 16 * k, 2, Access::Read),
                    slice(2 + rank as u64 * 3, 4096 * k, 2, Access::Read),
                    slice(3 + rank as u64 * 3, 16 * 4096, 4, Access::Write),
                    Arg::U32(1),
                    Arg::U32(4096),
                    Arg::U32(k as u32),
                    Arg::U32(2),
                    Arg::U32(tag),
                ],
            }),
            consumers: std::array::from_fn(|rank| {
                let mut arguments = vec![
                    slice(3, 4096, 4, Access::Read),
                    slice(6, 4096, 4, Access::Read),
                ];
                arguments.extend([slice(3, 0, 4, Access::Read); 6]);
                arguments.extend([
                    slice(7 + rank as u64 * 2, 4096, 2, Access::Read),
                    slice(8 + rank as u64 * 2, 4096, 2, Access::Write),
                    Arg::U32(1),
                    Arg::U32(2),
                ]);
                EngineeringTpDispatchV1 {
                    kernel: dep::CONSUMER_ROOT,
                    grid_workgroups: 64,
                    workgroup_size: 64,
                    arguments,
                }
            }),
        }
    }

    // Placeholder ordinary roots exercise receipts, not executable program admission.
    fn ordinary(rank: u32) -> Step {
        Step::Rank {
            rank,
            dispatch: EngineeringTpDispatchV1 {
                kernel: "cpu-receipt-fixture-only",
                grid_workgroups: 1,
                workgroup_size: 64,
                arguments: vec![],
            },
        }
    }

    fn receipt_fixture(epoch: u64) -> (Program, Input, protocol::ExecutionReceipt) {
        let mut steps = vec![ordinary(0), ordinary(1)];
        for layer in 0..36 {
            for _ in 0..9 {
                steps.extend([ordinary(0), ordinary(1)]);
            }
            steps.push(Step::Collective(collective(
                layer,
                Qwen3TensorParallelCollectiveV1::AttentionOutputSum,
            )));
            for _ in 0..4 {
                steps.extend([ordinary(0), ordinary(1)]);
            }
            steps.push(Step::Collective(collective(
                layer,
                Qwen3TensorParallelCollectiveV1::FeedForwardDownSum,
            )));
        }
        steps.extend([ordinary(0), ordinary(0), ordinary(0)]);
        let program = Program {
            group_id: 7,
            token_buffer: 20,
            result_buffer: 21,
            metadata: [EngineeringTp2PreparedMetadataV1 {
                positions: 22,
                page_table: 23,
                cos: 24,
                sin: 25,
            }; 2],
            steps,
        };
        let mut pages = [u32::MAX; 4];
        for (index, page) in pages.iter_mut().enumerate().take(epoch as usize / 16 + 1) {
            *page = index as u32;
        }
        let input = Input {
            plan_sha256: [9; 32],
            generation: epoch + 1,
            epoch,
            token: 42,
            position: epoch as u32,
            page_table: pages,
            cos_sin: vec![0; 512],
        };
        let mut cursors = [epoch * 688, epoch * 685];
        let mut collectives = Vec::new();
        let mut request_id = 100 + 9;
        for step in &program.steps {
            match step {
                Step::Rank { rank, .. } => cursors[*rank as usize] += 1,
                Step::Collective(template) => {
                    collectives.push(dep::Receipt {
                        identity: dep::Identity {
                            request_id,
                            generation: epoch * 72 + collectives.len() as u64 + 1,
                            group_id: 7,
                            model_role: dep::ModelRole::Target8b,
                            epoch,
                            layer: template.key.layer,
                            operation: operation(template.key.operation),
                        },
                        queues: std::array::from_fn(|rank| dep::RankQueue {
                            unique_id: rank as u64 + 1,
                            queue_epoch: 0,
                            first_packet: cursors[rank],
                            next_packet: cursors[rank] + 3,
                        }),
                        kernel_counts: [2; 2],
                        barrier_counts: [1; 2],
                        packet_counts: [3; 2],
                        completion_values: [[0; 3]; 2],
                        final_frontiers: cursors.map(|cursor| [cursor + 3; 2]),
                    });
                    for cursor in &mut cursors {
                        *cursor += 3;
                    }
                }
            }
            request_id += 1;
        }
        let receipt = protocol::ExecutionReceipt {
            id: 50,
            plan_sha256: input.plan_sha256,
            generation: input.generation,
            epoch,
            position: input.position,
            input_token: input.token,
            output_token: 12_095,
            unique_ids: [1, 2],
            kernel_counts: [616, 613],
            barrier_counts: [72; 2],
            packet_counts: [688, 685],
            ordinary_completions: 941,
            collectives,
        };
        (program, input, receipt)
    }

    #[test]
    fn native_program_parent_keeps_program_calls_distinct_from_logical_dispatches() {
        for epoch in [0, 1, 15, 16, 63] {
            let (program, input, mut receipt) = receipt_fixture(epoch);
            for record in &mut receipt.collectives {
                record.identity.request_id = record.identity.generation;
            }
            let native = scope::NativeProgramCompletion {
                program_id: epoch + 1,
                group_id: program.group_id,
                epoch,
                unique_ids: [1, 2],
                program_api_calls: 1,
                logical_steps: 1013,
                rank_dispatches: 941,
                kernel_counts: [616, 613],
                barrier_counts: [72; 2],
                packet_counts: [688, 685],
                first_frontiers: [epoch * 688, epoch * 685],
                final_frontiers: [[(epoch + 1) * 688; 2], [(epoch + 1) * 685; 2]],
            };
            scope::validate_completion(&native, &program, &input, [1, 2]).unwrap();
            validate_receipt_mode(
                &program,
                &input,
                &receipt,
                [1, 2],
                50,
                u64::MAX,
                PreparedMode::NativeProgram,
            )
            .unwrap();
            assert!(validate_receipt(&program, &input, &receipt, [1, 2], 50, 100).is_err());
            let mutations: [fn(&mut scope::NativeProgramCompletion); 14] = [
                |v| v.program_id += 1,
                |v| v.group_id += 1,
                |v| v.epoch += 1,
                |v| v.unique_ids.swap(0, 1),
                |v| v.program_api_calls = 1013,
                |v| v.logical_steps -= 1,
                |v| v.rank_dispatches -= 1,
                |v| v.kernel_counts[1] += 1,
                |v| v.barrier_counts[0] += 1,
                |v| v.packet_counts[1] += 1,
                |v| v.first_frontiers[0] += 1,
                |v| v.first_frontiers[1] += 1,
                |v| v.final_frontiers[0][0] += 1,
                |v| v.final_frontiers[1][1] += 1,
            ];
            for mutate in mutations {
                let mut changed = native.clone();
                mutate(&mut changed);
                assert!(scope::validate_completion(&changed, &program, &input, [1, 2]).is_err());
            }
            for index in 0..72 {
                let mut changed = receipt.clone();
                changed.collectives[index].identity.request_id += 1;
                assert!(
                    validate_receipt_mode(
                        &program,
                        &input,
                        &changed,
                        [1, 2],
                        50,
                        0,
                        PreparedMode::NativeProgram
                    )
                    .is_err()
                );
            }
        }
    }

    #[test]
    fn prepared_parent_options_close_all_other_modes() {
        assert_eq!(
            validate_options(&[1, 2], RuntimeOptions::default(), false).unwrap(),
            [1, 2]
        );
        assert!(
            validate_options(
                &[1, 2],
                RuntimeOptions {
                    cache_admission: true,
                    ..RuntimeOptions::default()
                },
                false
            )
            .is_ok()
        );
        for ids in [vec![], vec![1], vec![1, 1], vec![0, 2], vec![1, 2, 3]] {
            assert!(validate_options(&ids, RuntimeOptions::default(), false).is_err());
        }
        let mutations: [fn(&mut RuntimeOptions); 7] = [
            |o| o.operational = true,
            |o| o.sequences = true,
            |o| o.ordered_batches = true,
            |o| o.full_forward = true,
            |o| o.rollover = true,
            |o| o.shared_full_currentness = true,
            |o| o.profile = true,
        ];
        for mutate in mutations {
            let mut options = RuntimeOptions::default();
            mutate(&mut options);
            assert!(validate_options(&[1, 2], options, false).is_err());
        }
        assert!(validate_options(&[1, 2], RuntimeOptions::default(), true).is_err());
    }

    fn ready(pid: u32) -> protocol::Response {
        protocol::Response::Ready {
            protocol: 1,
            mode: protocol::MODE.into(),
            profile: protocol::PROFILE.into(),
            unique_ids: [1, 2],
            process_id: pid,
            authority: "none".into(),
            currentness: "full".into(),
            control_allocation_flags: dep::CONTROL_ALLOCATION_FLAGS,
        }
    }

    #[test]
    fn prepared_parent_ready_binds_exact_child_and_control_policy() {
        assert!(valid_ready(&ready(7), [1, 2], 7));
        assert!(!valid_ready(&ready(7), [2, 1], 7));
        assert!(!valid_ready(&ready(7), [1, 2], 8));
        for key in [
            "protocol",
            "mode",
            "profile",
            "authority",
            "currentness",
            "control_allocation_flags",
        ] {
            let mut value = serde_json::to_value(ready(7)).unwrap();
            value[key] = if ["protocol", "control_allocation_flags"].contains(&key) {
                0.into()
            } else {
                "wrong".into()
            };
            let changed = serde_json::from_value(value).unwrap();
            assert!(!valid_ready(&changed, [1, 2], 7));
        }
    }

    #[test]
    fn prepared_parent_checks_input_before_ipc() {
        let (program, input, _) = receipt_fixture(0);
        let registered = Registration {
            program,
            hash: input.plan_sha256,
            generation: 1,
            pages: vec![u32::MAX; 4],
            geometry: GraphGeometry::Short64,
        };
        validate_input(&registered, &input).unwrap();
        let mutations: [fn(&mut Input); 9] = [
            |v| v.plan_sha256[0] ^= 1,
            |v| v.generation += 1,
            |v| v.epoch += 1,
            |v| v.position = 64,
            |v| v.token = protocol::VOCABULARY,
            |v| {
                v.cos_sin.pop();
            },
            |v| v.cos_sin[..4].copy_from_slice(&f32::NAN.to_le_bytes()),
            |v| v.page_table[0] = 4,
            |v| v.page_table[1] = 0,
        ];
        for mutate in mutations {
            let mut value = input.clone();
            mutate(&mut value);
            assert!(validate_input(&registered, &value).is_err());
        }
        for number in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.0001, -1.0001] {
            let mut value = input.clone();
            value.cos_sin[..4].copy_from_slice(&number.to_le_bytes());
            assert!(validate_input(&registered, &value).is_err());
        }
        for number in [-1.0_f32, -0.0, 0.0, 0.95, 1.0] {
            let mut value = input.clone();
            value.cos_sin[..4].copy_from_slice(&number.to_le_bytes());
            validate_input(&registered, &value).unwrap();
        }
        let (program, mut input, _) = receipt_fixture(17);
        let registered = Registration {
            program,
            hash: input.plan_sha256,
            generation: 18,
            pages: vec![0, 1, u32::MAX, u32::MAX],
            geometry: GraphGeometry::Short64,
        };
        validate_input(&registered, &input).unwrap();
        input.page_table.swap(0, 1);
        assert!(validate_input(&registered, &input).is_err());
    }

    #[test]
    fn prepared_parent_receipts_bind_all_native_ids_and_two_token_frontiers() {
        for epoch in [0, 1, 63] {
            let (program, input, receipt) = receipt_fixture(epoch);
            let public = validate_receipt(&program, &input, &receipt, [1, 2], 50, 100).unwrap();
            assert_eq!(public.output_token, 12_095);
            assert_eq!(public.input_token, 42);
            assert_eq!(public.collectives.len(), 72);
            for index in 0..72 {
                for field in 0..5 {
                    let mut changed = receipt.clone();
                    let record = &mut changed.collectives[index];
                    match field {
                        0 => record.identity.request_id += 1,
                        1 => record.identity.generation += 1,
                        2 => record.queues[0].unique_id = 2,
                        3 => record.final_frontiers[1][1] -= 1,
                        _ => record.completion_values[0][0] = 1,
                    }
                    assert!(validate_receipt(&program, &input, &changed, [1, 2], 50, 100).is_err());
                }
            }
        }
    }

    #[test]
    fn prepared_parent_full_late_receipt_fits_unchanged_header_bound() {
        let (program, input, mut receipt) = receipt_fixture(63);
        let ids = [16_366_993_098_680_759_275, 10_838_076_764_495_710_945];
        let native_start = u64::MAX - 2048;
        let request_id = u64::MAX - 1;
        receipt.id = request_id;
        receipt.unique_ids = ids;
        for record in &mut receipt.collectives {
            record.identity.request_id = record.identity.request_id - 100 + native_start;
            for (rank, queue) in record.queues.iter_mut().enumerate() {
                queue.unique_id = ids[rank];
            }
        }
        validate_receipt(&program, &input, &receipt, ids, request_id, native_start).unwrap();
        let response = protocol::Response::Executed { receipt };
        let mut bytes = Vec::new();
        protocol::write_response(&mut bytes, &response).unwrap();
        let header_bytes = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        assert_eq!(header_bytes + 4, bytes.len());
        assert!(header_bytes <= 65_536);
        let decoded = protocol::read_response(&mut bytes.as_slice())
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::to_value(decoded).unwrap(),
            serde_json::to_value(response).unwrap()
        );
        eprintln!("prepared epoch63 complete72-receipt header bytes: {header_bytes}");
    }

    #[test]
    fn prepared_parent_rejects_outer_receipt_substitution_and_truncation() {
        let (program, input, receipt) = receipt_fixture(0);
        let mutations: [fn(&mut protocol::ExecutionReceipt); 8] = [
            |v| v.id += 1,
            |v| v.unique_ids.swap(0, 1),
            |v| v.input_token += 1,
            |v| v.output_token = protocol::VOCABULARY,
            |v| v.plan_sha256[0] ^= 1,
            |v| v.ordinary_completions -= 1,
            |v| v.kernel_counts[0] -= 1,
            |v| {
                v.collectives.pop();
            },
        ];
        for mutate in mutations {
            let mut changed = receipt.clone();
            mutate(&mut changed);
            assert!(validate_receipt(&program, &input, &changed, [1, 2], 50, 100).is_err());
        }
        let mut bytes = vec![];
        protocol::write_response(&mut bytes, &protocol::Response::Executed { receipt }).unwrap();
        assert!(matches!(
            protocol::read_response(&mut bytes.as_slice()).unwrap(),
            Some(protocol::Response::Executed { .. })
        ));
        bytes.pop();
        assert!(protocol::read_response(&mut bytes.as_slice()).is_err());
    }

    const FAKE: &str = r"
import json, os, struct, sys, time
mode = sys.argv[1]
ready = json.loads(sys.argv[2]); long = 'response' in ready
body = ready['response'] if long else ready
body['process_id'] = os.getpid()
response_tag = sys.argv[3]
def send(value):
    if long and 'geometry' not in value: value = {'geometry':'long2304','response':value}
    data = json.dumps(value).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(data)) + data); sys.stdout.buffer.flush()
if mode == 'bad_ready': body['currentness'] = 'operational'
send(ready)
count = 0
while True:
    prefix = sys.stdin.buffer.read(4)
    if not prefix: break
    header = json.loads(sys.stdin.buffer.read(struct.unpack('<I', prefix)[0]))
    if long: header = header['request']
    if mode == 'stall': time.sleep(60)
    if mode == 'fatal': send({response_tag:'fatal','id':header['id'],'message':'injected'}); break
    if header.get('prepared_long_graph_op' if long else 'prepared_op') == 'close':
        send({response_tag:'closed','id':header['id']}); break
    command = header['command']
    sys.stdin.buffer.read(command.get('payload_bytes',0))
    if command['op'] == 'allocate':
        count += 1
        response = {'op':'allocated','buffer':1 if mode == 'reuse' else count,'bytes':command['bytes']}
    elif command['op'] == 'write': response = {'op':'written'}
    else: response = {'op':'performance_configured'}
    send({response_tag:'setup','id':header['id'] + (1 if mode == 'wrong_id' else 0),'response':response})
";

    fn fake(mode: &str) -> TpResult<Vec<PreparedWorker>> {
        fake_with_mode(mode, PreparedMode::Interpreter)
    }

    fn fake_with_mode(mode: &str, selected: PreparedMode) -> TpResult<Vec<PreparedWorker>> {
        let (ready, response_tag) = match selected {
            PreparedMode::Interpreter => (serde_json::to_value(ready(0)).unwrap(), "prepared_op"),
            PreparedMode::NativeProgram => (scope::ready_fixture([1, 2], 0), "prepared_program_op"),
            PreparedMode::Graph(policy) => (
                serde_json::to_value(scope::graph_ready_fixture([1, 2], 0, policy)).unwrap(),
                "prepared_graph_op",
            ),
            PreparedMode::GraphOptions(policy, profile, metadata) => (
                serde_json::to_value(scope::graph_ready_fixture_with_options(
                    [1, 2],
                    0,
                    policy,
                    profile,
                    metadata,
                ))
                .unwrap(),
                "prepared_graph_op",
            ),
            PreparedMode::LongGraph(policy, profile, metadata) => (
                serde_json::to_value(scope::long_graph_ready_fixture(
                    [1, 2],
                    0,
                    policy,
                    profile,
                    metadata,
                ))
                .unwrap(),
                "prepared_graph_op",
            ),
        };
        let child = Command::new("python3")
            .args([
                "-u",
                "-c",
                FAKE,
                mode,
                &serde_json::to_string(&ready).unwrap(),
                response_tag,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let connection = match selected {
            PreparedMode::Interpreter => Connection::connect(
                child,
                [1, 2],
                Duration::from_millis(500),
                HostTiming::default(),
            ),
            PreparedMode::NativeProgram
            | PreparedMode::Graph(_)
            | PreparedMode::GraphOptions(_, _, _)
            | PreparedMode::LongGraph(_, _, _) => Connection::connect_mode(
                child,
                [1, 2],
                Duration::from_millis(500),
                HostTiming::default(),
                selected,
            ),
        }?;
        let connection = Rc::new(RefCell::new(connection));
        Ok((0..2)
            .map(|rank| PreparedWorker {
                connection: Rc::clone(&connection),
                rank,
            })
            .collect())
    }

    #[test]
    fn prepared_parent_setup_typed_ownership_and_two_rank_close_are_real_ipc() {
        let mut ranks = fake("normal").unwrap();
        let first = ranks[0].allocate_peer_readable(64).unwrap();
        let second = ranks[1].allocate(64).unwrap();
        assert_ne!(first, second);
        ranks[0].write(first, 0, &[1, 2]).unwrap();
        assert!(ranks[0].supports_prepared_peer());
        ranks[0].close().unwrap();
        assert!(!ranks[1].supports_prepared_peer());
        ranks[1].close().unwrap();
        assert!(ranks[0].connection.borrow().exited);
        assert_eq!(ranks[0].connection.borrow().phase, Phase::Closed);
    }

    #[test]
    fn prepared_parent_failure_paths_terminate_owned_mock_child() {
        assert!(fake("bad_ready").is_err());
        for mode in ["wrong_id", "fatal", "stall", "reuse", "normal"] {
            let mut ranks = fake(mode).unwrap();
            if mode == "reuse" {
                ranks[0].allocate(64).unwrap();
            }
            if mode == "normal" {
                let id = ranks[0].allocate(64).unwrap();
                assert!(ranks[1].write(id, 0, &[0]).is_err());
            } else {
                assert!(ranks[0].allocate(64).is_err());
            }
            let state = ranks[0].connection.borrow();
            assert!(state.exited);
            assert_eq!(state.phase, Phase::Terminal);
            drop(state);
            assert!(ranks[1].allocate(1).is_err());
            ranks[0].close().unwrap();
            ranks[1].close().unwrap();
        }
    }

    #[test]
    fn prepared_parent_seal_and_unsupported_dispatch_are_terminal() {
        let mut ranks = fake("normal").unwrap();
        ranks[0].connection.borrow_mut().phase = Phase::Ready;
        assert!(ranks[0].allocate(4).is_err());
        assert!(ranks[0].connection.borrow().exited);
        let mut ranks = fake("normal").unwrap();
        let Step::Rank { dispatch, .. } = ordinary(0) else {
            panic!()
        };
        assert!(ranks[0].submit(&dispatch).is_err());
        assert!(ranks[0].connection.borrow().exited);
    }
}
