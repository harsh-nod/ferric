//! Explicit non-production two-forward client. No queued-runtime fallback.

use crate::finite_forward_wire_v1 as forward;
use crate::finite_setup_wire_v1 as setup_wire;
use crate::tp_execution::batched::EngineeringTp2FiniteSourceRecorderV1;
use crate::tp_execution::{
    EngineeringTp2Finite2304MetadataV1, EngineeringTp2GraphGeometryV1, EngineeringTp2GraphInputV1,
};
use crate::tp_model::EngineeringQwenModelV1;
use crate::tp_paged::{
    EngineeringTpBatchCompletionV1, EngineeringTpPageRowV1, EngineeringTpPagedLimitsV1,
    EngineeringTpPagedPoolV1, EngineeringTpPoolScopeV1, EngineeringTpPreparedBatchV1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub mod long;
pub mod mlp_tiles_comparison;
pub mod prefix_decode;
pub mod prefix_layer;
mod process;
pub mod queued_mlp_comparison;
pub mod queued_projection_comparison;
pub mod rearm_smoke;
mod setup;
pub mod tiles_decode;
type Result<T> = std::result::Result<T, String>;

fn require(value: bool, message: &str) -> Result<()> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn mode_label(mode: forward::InputMode) -> &'static str {
    match mode {
        forward::InputMode::TeacherForced => "teacher-forced",
        forward::InputMode::Autoregressive => "autoregressive",
    }
}

/// Exact regular-file identity. Paths must already be canonical without symlinks.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilePin {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: [u8; 32],
}
impl FilePin {
    fn read(&self, maximum: u64, retain: bool) -> Result<Vec<u8>> {
        require(
            self.path.is_absolute()
                && self.path.canonicalize().map_err(|e| e.to_string())? == self.path
                && self.bytes > 0
                && self.bytes <= maximum
                && self.sha256 != [0; 32],
            "finite input pin/path/bound",
        )?;
        let mut file = File::open(&self.path).map_err(|e| e.to_string())?;
        let before = file.metadata().map_err(|e| e.to_string())?;
        require(
            before.is_file() && before.len() == self.bytes,
            "finite input regular-file extent",
        )?;
        let mut hasher = Sha256::new();
        let mut output = Vec::new();
        let mut total = 0_u64;
        let mut block = [0; 65_536];
        loop {
            let count = file.read(&mut block).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            total = total
                .checked_add(count as u64)
                .ok_or("finite input size overflow")?;
            require(total <= self.bytes, "finite input grew during intake")?;
            hasher.update(&block[..count]);
            if retain {
                output.extend_from_slice(&block[..count]);
            }
        }
        let after = file.metadata().map_err(|e| e.to_string())?;
        let path = std::fs::symlink_metadata(&self.path).map_err(|e| e.to_string())?;
        let digest: [u8; 32] = hasher.finalize().into();
        let identity = |m: &std::fs::Metadata| {
            (
                m.dev(),
                m.ino(),
                m.len(),
                m.mtime(),
                m.mtime_nsec(),
                m.ctime(),
                m.ctime_nsec(),
            )
        };
        require(
            total == self.bytes
                && digest == self.sha256
                && identity(&before) == identity(&after)
                && path.is_file()
                && identity(&path) == identity(&after),
            "finite input changed or digest mismatch",
        )?;
        Ok(output)
    }
}

/// Artifact and executable pins are evidence inputs, not production admission.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePins {
    pub prefix: FilePin,
    pub mlp: FilePin,
    pub residual: FilePin,
    pub tail: FilePin,
}
struct Images {
    prefix: Vec<u8>,
    mlp: Vec<u8>,
    residual: Vec<u8>,
    tail: Vec<u8>,
}
impl ImagePins {
    fn read(&self) -> Result<Images> {
        Ok(Images {
            prefix: self.prefix.read(64 << 20, true)?,
            mlp: self.mlp.read(64 << 20, true)?,
            residual: self.residual.read(64 << 20, true)?,
            tail: self.tail.read(64 << 20, true)?,
        })
    }
}

/// Closed two-position engineering request. Whole-parent CPU/address-space
/// limits must additionally be supplied by the root's bounded outer supervisor.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub source: PathBuf,
    pub worker: FilePin,
    pub images: ImagePins,
    pub expected_bundle_id: [u8; 32],
    pub expected_model_id: [u8; 32],
    pub device_ids: [u64; 2],
    pub session: [u8; 32],
    pub mode: forward::InputMode,
    pub input_tokens: Vec<u32>,
    pub dispatch_timeout_ms: u32,
    pub child_deadline_ms: u64,
}
impl Config {
    /// Parse only the bounded closed request. No file or device is opened.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= 65_536,
            "finite request byte bound",
        )?;
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        let tokens = match self.mode {
            forward::InputMode::TeacherForced => 2,
            forward::InputMode::Autoregressive => 1,
        };
        require(
            self.schema == "FerricFiniteTwoForwardRequestV1"
                && self.source.is_absolute()
                && self.expected_bundle_id != [0; 32]
                && self.expected_model_id != [0; 32]
                && self.session != [0; 32]
                && self.device_ids[0] != 0
                && self.device_ids[1] != 0
                && self.device_ids[0] != self.device_ids[1]
                && self.input_tokens.len() == tokens
                && self.input_tokens.iter().all(|v| *v < 151_936)
                && (1..=10_000).contains(&self.dispatch_timeout_ms)
                && (1_000..=3_600_000).contains(&self.child_deadline_ms),
            "finite closed request scope/mode/bounds",
        )
    }
}

#[derive(Serialize)]
pub struct ForwardObservation {
    pub response: forward::Response,
    /// Exact child payload bytes in wire order; not independently accepted numerics.
    pub payload: Vec<u8>,
}
#[derive(Serialize)]
pub struct Observation {
    pub schema: &'static str,
    pub request: Config,
    pub child_pid: u32,
    pub registration_sha256: [u8; 32],
    pub source_program_sha256: [u8; 32],
    pub upload_manifest_sha256: [u8; 32],
    pub setup_commands: u64,
    pub forwards: Vec<ForwardObservation>,
    pub close: forward::Response,
    pub child_stderr: Vec<u8>,
    pub child_exit_zero: bool,
    pub process_group_absent: bool,
    pub native_closed: bool,
    pub gpu_execution: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}

fn state_valid<const N: usize>(words: &[u32; N], tasks: usize) -> bool {
    let mask = (1_u32 << tasks) - 1;
    N == 6 + tasks
        && words[..4] == [1, 0, mask, mask]
        && words[5] == 0
        && words[6..].iter().all(|v| *v == 64)
        && (0..tasks).all(|task| matches!((words[4] >> (2 * task)) & 3, 1 | 2))
        && (tasks == 16 || words[4] >> (2 * tasks) == 0)
}

fn validate_completion(
    request: &forward::Request,
    response: &forward::Response,
    payload: &[u8],
) -> Result<u32> {
    request.validate().map_err(|e| e.to_string())?;
    require(
        response.protocol == request.protocol
            && response.id == request.id
            && response.device_ids == request.device_ids
            && response.session == request.session
            && response.registration == request.registration
            && response.gpu_execution
            && !response.native_closed
            && !response.numerical_acceptance
            && !response.performance_claim
            && !response.production_authority,
        "finite completed response identity/claims",
    )?;
    let forward::Command::Forward {
        generation, token, ..
    } = &request.command
    else {
        return Err("completion for non-forward".into());
    };
    let forward::Event::Completed(c) = &response.event else {
        return Err("finite expected Completed".into());
    };
    require(
        c.generation == *generation
            && c.position as u64 + 1 == *generation
            && c.input_token == *token
            && c.output_token < 151_936
            && c.layers.len() == 36
            && c.payload == forward::Payload::from_bytes(payload).map_err(|e| e.to_string())?,
        "finite completed payload/case",
    )?;
    require(
        c.layers.iter().all(|layer| {
            layer.prefix_states.iter().all(|w| state_valid(w, 16))
                && layer.mlp_states.iter().all(|w| state_valid(w, 5))
        }),
        "finite incomplete/error/owner/arrival state",
    )?;
    require(
        payload
            .chunks_exact(2)
            .all(|b| u16::from_le_bytes([b[0], b[1]]) & 0x7f80 != 0x7f80),
        "finite nonfinite BF16 observation",
    )?;
    Ok(c.output_token)
}

fn validate_close(
    request: &forward::Request,
    response: &forward::Response,
    payload: &[u8],
) -> Result<()> {
    request.validate().map_err(|e| e.to_string())?;
    require(
        matches!(request.command, forward::Command::Close),
        "finite Close request kind",
    )?;
    require(
        response.protocol == request.protocol
            && response.id == 3
            && request.id == 3
            && response.device_ids == request.device_ids
            && response.session == request.session
            && response.registration == request.registration
            && payload.is_empty()
            && response.event
                == (forward::Event::Closed {
                    completed_forwards: 2,
                })
            && response.native_closed
            && response.gpu_execution
            && !response.numerical_acceptance
            && !response.performance_claim
            && !response.production_authority,
        "finite Close acknowledgement scope",
    )
}

fn second_token(mode: forward::InputMode, inputs: &[u32], output0: u32) -> Result<u32> {
    require(output0 < 151_936, "first output vocabulary")?;
    match mode {
        forward::InputMode::TeacherForced => inputs
            .get(1)
            .copied()
            .ok_or("teacher-forced second input missing".into()),
        forward::InputMode::Autoregressive => Ok(output0),
    }
}

fn metadata(
    batch: &EngineeringTpPreparedBatchV1,
    program_sha256: [u8; 32],
    theta: u32,
) -> Result<EngineeringTp2Finite2304MetadataV1> {
    require(
        batch.rows().len() == 1
            && theta == 1_000_000
            && batch.context_tokens() == 2304
            && batch.physical_page_count() == 144
            && batch.page_table_stride() == 144,
        "finite metadata requires one authentic long-geometry prepared row",
    )?;
    let row = &batch.rows()[0];
    require(
        row.position() < 2 && row.physical_pages().len() == 1,
        "finite two-forward row/history bound",
    )?;
    let mut page_table = vec![u32::MAX; 144];
    page_table[..row.physical_pages().len()].copy_from_slice(row.physical_pages());
    let (mut cos_sin, sin) = crate::tp_execution::rope_bytes(row.position(), theta);
    cos_sin.extend(sin);
    // This GraphInput is consumed only by the inert scalar converter. The source
    // snapshot digest here is not a legacy registered graph capability.
    let input = EngineeringTp2GraphInputV1 {
        geometry: EngineeringTp2GraphGeometryV1::Long2304,
        plan_sha256: program_sha256,
        generation: u64::from(row.position()) + 1,
        epoch: u64::from(row.position()),
        token: row.token(),
        position: row.position(),
        page_table,
        cos_sin,
    };
    EngineeringTp2Finite2304MetadataV1::prepare(&input)
}

/// Authenticate the model, spawn one owned current-generation child, upload the
/// fixed resident model and run exactly positions0/1. This is an engineering
/// observation API, never a production execution-capability factory.
///
/// The caller must provide bounded outer CPU/address-space supervision. The
/// owned child's complete lifetime additionally has a kill/reap/join deadline.
/// # Errors
/// Every setup, identity, native/state, process or stream error rejects the whole
/// observation. There are no retries or fallback kernels.
pub fn run(config: Config, allow_unauthenticated_machine_code: bool) -> Result<Observation> {
    require(
        allow_unauthenticated_machine_code,
        "explicit unauthenticated engineering machine-code opt-in required",
    )?;
    config.validate()?;
    config.worker.read(512 << 20, false)?;
    let images = config.images.read()?;
    let model = EngineeringQwenModelV1::open(&config.source)?;
    require(
        *model.bundle_id().as_bytes() == config.expected_bundle_id
            && *model.config().model_id.as_bytes() == config.expected_model_id,
        "authenticated model/bundle differs from request",
    )?;
    let scope = EngineeringTpPoolScopeV1 {
        model: config.expected_model_id,
        session: config.session,
    };
    let limits =
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).map_err(|e| format!("{e:?}"))?;
    let mut pool = EngineeringTpPagedPoolV1::new(scope, limits).map_err(|e| format!("{e:?}"))?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(config.child_deadline_ms))
        .ok_or("child deadline overflow")?;
    let mut child = process::OwnedChild::spawn(&config, deadline)?;
    let pid = child.id();
    eprintln!("finite engineering owned child pid={pid} pgid={pid}; no native setup acknowledged");
    config.worker.read(512 << 20, false)?;
    let recorder = EngineeringTp2FiniteSourceRecorderV1::new(&model, &pool, pid)?;
    child.check_deadline()?;
    let (registration_sha256, program_sha256, manifest_sha256, setup_commands) = recorder
        .with_plan(|composition, source, uploads| {
            let registration = composition.wire_registration_v1()?;
            let program = composition.source_program_snapshot_v1()?;
            let head = source.head_transpose_upload()?;
            let manifest = setup::manifest(
                uploads,
                setup_wire::TailHeadTranspose {
                    source: setup_wire::Key::Source {
                        rank: 0,
                        id: head.source_id(),
                    },
                    source_sha256: head.source_sha256(),
                    bytes: head.bytes() as u64,
                    sha256: head.sha256(),
                },
            )?;
            let registration_sha =
                hash(&serde_json::to_vec(&registration).map_err(|e| e.to_string())?);
            let manifest_sha = hash(&serde_json::to_vec(&manifest).map_err(|e| e.to_string())?);
            let bootstrap = forward::Bootstrap {
                protocol: forward::PROTOCOL,
                device_ids: config.device_ids,
                scope: setup::scope(&registration),
                mode: config.mode,
                timeout_ms: config.dispatch_timeout_ms,
            };
            bootstrap
                .validate(
                    config.device_ids,
                    config.mode,
                    config.dispatch_timeout_ms,
                    pid,
                )
                .map_err(|e| e.to_string())?;
            child.check_deadline()?;
            forward::write_bootstrap(
                child.input.as_mut().ok_or("closed bootstrap stdin")?,
                &bootstrap,
            )
            .map_err(|e| e.to_string())?;
            let mut stream = setup::Stream::begin(
                &mut child,
                config.device_ids,
                &registration,
                &program,
                &manifest,
                &images,
            )?;
            for upload in uploads {
                stream.upload(upload)?;
            }
            for key in setup::mutable_keys(&registration)? {
                stream.zero(key)?;
            }
            stream.allocate_tail()?;
            let mut total = 0;
            let mut digest = Sha256::new();
            head.visit_chunks(
                crate::finite_composition_wire::MAX_TRANSFER,
                |offset, bytes| {
                    require(offset == total, "head recipe contiguous offset")?;
                    stream.write_tail(offset, bytes)?;
                    total += bytes.len();
                    digest.update(bytes);
                    Ok(())
                },
            )?;
            let observed: [u8; 32] = digest.finalize().into();
            require(
                total == head.bytes() && observed == head.sha256(),
                "head recipe changed while streaming",
            )?;
            Ok((
                registration_sha,
                hash(&program),
                manifest_sha,
                stream.seal()?,
            ))
        })?;
    let sequence = pool
        .open_sequence(scope, &[config.input_tokens[0]], 0)
        .map_err(|e| format!("{e:?}"))?;
    require(
        sequence.hit_tokens() == 0 && sequence.physical_pages().is_empty(),
        "finite setup requires fresh sequence history",
    )?;
    let mut observations = Vec::with_capacity(2);
    let mut token = config.input_tokens[0];
    for position in 0..2_u32 {
        require(
            pool.committed_position(sequence.sequence())
                .map_err(|e| format!("{e:?}"))?
                == position,
            "finite native history/host committed position",
        )?;
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence: sequence.sequence(),
                token,
                position,
            }])
            .map_err(|e| format!("{e:?}"))?;
        let metadata = metadata(&batch, program_sha256, model.config().rope_theta)?;
        let request = forward::Request {
            protocol: forward::PROTOCOL,
            id: u64::from(position) + 1,
            device_ids: config.device_ids,
            session: config.session,
            registration: registration_sha256,
            command: forward::Command::Forward {
                generation: u64::from(position) + 1,
                token,
                cache_metadata: metadata.cache_metadata().to_vec(),
                rotary_bits: metadata.rotary().iter().map(|v| v.to_bits()).collect(),
            },
        };
        request.validate().map_err(|e| e.to_string())?;
        child.check_deadline()?;
        pool.begin_submission(&batch)
            .map_err(|e| format!("{e:?}"))?;
        let result: Result<(u32, ForwardObservation)> = (|| {
            forward::write_request(
                child.input.as_mut().ok_or("closed forward stdin")?,
                &request,
            )
            .map_err(|e| e.to_string())?;
            let (response, payload) =
                forward::read_response(child.output.as_mut().ok_or("closed forward stdout")?)
                    .map_err(|e| e.to_string())?
                    .ok_or("EOF before forward completion")?;
            let output = validate_completion(&request, &response, &payload)?;
            child.check_deadline()?;
            Ok((output, ForwardObservation { response, payload }))
        })();
        let (output, observation) = match result {
            Ok(value) => value,
            Err(error) => {
                pool.quarantine_batch(&batch)
                    .map_err(|e| format!("quarantine: {e:?}; original: {error}"))?;
                return Err(error);
            }
        };
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .map_err(|e| format!("{e:?}"))?;
        if position == 0 {
            token = second_token(config.mode, &config.input_tokens, output)?;
        }
        observations.push(observation);
    }
    let close_request = forward::Request {
        protocol: forward::PROTOCOL,
        id: 3,
        device_ids: config.device_ids,
        session: config.session,
        registration: registration_sha256,
        command: forward::Command::Close,
    };
    forward::write_request(
        child.input.as_mut().ok_or("closed close stdin")?,
        &close_request,
    )
    .map_err(|e| e.to_string())?;
    let (close, payload) =
        forward::read_response(child.output.as_mut().ok_or("closed close stdout")?)
            .map_err(|e| e.to_string())?
            .ok_or("EOF before native close")?;
    validate_close(&close_request, &close, &payload)?;
    let stderr = child.finish()?;
    // Worker/image files remain explicit evidence pins. The retained buffers,
    // not reread files, were sent to the child during setup.
    config.worker.read(512 << 20, false)?;
    for pin in [
        &config.images.prefix,
        &config.images.mlp,
        &config.images.residual,
        &config.images.tail,
    ] {
        pin.read(64 << 20, false)?;
    }
    Ok(Observation {
        schema: "FerricFiniteTwoForwardObservationV1",
        request: config,
        child_pid: pid,
        registration_sha256,
        source_program_sha256: program_sha256,
        upload_manifest_sha256: manifest_sha256,
        setup_commands,
        forwards: observations,
        close,
        child_stderr: stderr,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    })
}

#[cfg(test)]
#[path = "tp_finite_client/tests.rs"]
mod tests;
