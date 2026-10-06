//! Four all-layer Prefix284 + MLP548 forwards with explicit TF/own-output AR.

use super::{FilePin, ImagePins, Result, hash, process, require, setup};
use crate::finite_forward_wire_v1 as old;
use crate::finite_prefix_decode_wire_v1 as wire;
use crate::finite_projection_residual_decode_wire_v1 as projection_wire;
use crate::finite_projection_residual_mlp_ordered_wire_v1 as ordered_wire;
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
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

pub mod device_clock_v2;
pub mod device_v1;
mod evidence;
pub mod guarded;
pub mod host_observation;
pub mod host_policy_v2;
pub mod projection;

const VOCABULARY: u32 = 151_936;
const INPUT_TOKENS: [u32; 4] = [9112, 2190, 3772, 220];
const MANIFEST_SHA: &str = "30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600";
const PROMPT_SHA: &str = "a43ef3619cb96c9c23d020e07630493ced7a588088033a53f1d612448375751a";
const TOKENS_SHA: &str = "2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02";

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut text, "{byte:02x}").expect("String formatting");
    }
    text
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptPins {
    pub manifest: FilePin,
    pub text: FilePin,
    pub tokens: FilePin,
}
impl PromptPins {
    fn validate(&self) -> Result<()> {
        require(
            self.manifest.bytes == 21_318
                && hex(&self.manifest.sha256) == MANIFEST_SHA
                && self.text.bytes == 11_224
                && hex(&self.text.sha256) == PROMPT_SHA
                && self.tokens.bytes == 8192
                && hex(&self.tokens.sha256) == TOKENS_SHA,
            "prefix decode profile requires the exact retained raw 2048-token prompt",
        )
    }
    fn read(&self) -> Result<(String, Vec<u32>)> {
        self.validate()?;
        let manifest = self.manifest.read(32 << 10, true)?;
        let text = String::from_utf8(self.text.read(16 << 10, true)?).map_err(|e| e.to_string())?;
        let bytes = self.tokens.read(8192, true)?;
        let tokens: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
            .collect();
        // The entire historical manifest is pinned above; this is a projection,
        // not an extensible request schema or fresh source authority.
        let value: serde_json::Value =
            serde_json::from_slice(&manifest).map_err(|e| e.to_string())?;
        require(
            value["schema"] == "FerricQwen3LongPromptV1"
                && value["revision"] == "b968826d9c46dd6066d109eabc6255188de91218"
                && value["input_tokens"] == 2048
                && value["output_tokens"] == 256
                && value["add_special_tokens"] == false
                && value["chat_template"].is_null()
                && value["round_trip_verified"] == true
                && value["input_token_ids"]
                    == serde_json::to_value(&tokens).map_err(|e| e.to_string())?,
            "prefix decode prompt manifest/token projection",
        )?;
        require(
            tokens.len() == 2048
                && tokens.iter().all(|v| *v < VOCABULARY)
                && tokens[..4] == INPUT_TOKENS,
            "prefix decode prompt vocabulary or extent",
        )?;
        Ok((text, tokens))
    }
    fn recheck(&self) -> Result<()> {
        self.manifest.read(32 << 10, false)?;
        self.text.read(16 << 10, false)?;
        self.tokens.read(8192, false)?;
        Ok(())
    }
}

/// Closed engineering request. Outer CPU, address-space and idle-device
/// supervision remains required; these pins do not grant production authority.
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
    pub prompt: PromptPins,
    pub mode: wire::InputMode,
    /// Actual reviewed tiled V2 image; distinct from the historical setup MLP image.
    pub tiles_image: FilePin,
    /// Actual reviewed PrefixV6 image, not the original setup prefix image.
    pub prefix_image: FilePin,
    pub evidence_directory: PathBuf,
    pub dispatch_timeout_ms: u32,
    pub child_deadline_ms: u64,
}
impl Config {
    fn read_tiles_image(&self) -> Result<Vec<u8>> {
        self.tiles_image.read(wire::MAX_IMAGE_BYTES as u64, true)
    }
    fn read_prefix_image(&self) -> Result<Vec<u8>> {
        self.prefix_image.read(wire::MAX_IMAGE_BYTES as u64, true)
    }

    /// Parse without opening files or devices.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= 65_536,
            "prefix decode request byte bound",
        )?;
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        self.prompt.validate()?;
        require(
            serde_json::to_vec(self).map_err(|e| e.to_string())?.len() <= 16_384
                && serde_json::to_vec(&self.evidence_directory)
                    .map_err(|e| e.to_string())?
                    .len()
                    <= 512,
            "prefix decode retained request/path bound",
        )?;
        require(
            self.schema == "FerricFinitePrefixDecodeRequestV1"
                && self.source.is_absolute()
                && self.evidence_directory.is_absolute()
                && self.evidence_directory.file_name().is_some()
                && self.expected_bundle_id != [0; 32]
                && self.expected_model_id != [0; 32]
                && self.session != [0; 32]
                && self.tiles_image.path.is_absolute()
                && self.tiles_image.bytes > 0
                && self.tiles_image.bytes <= wire::MAX_IMAGE_BYTES as u64
                && self.tiles_image.sha256 != [0; 32]
                && self.prefix_image.path.is_absolute()
                && self.prefix_image.bytes > 0
                && self.prefix_image.bytes <= wire::MAX_IMAGE_BYTES as u64
                && self.prefix_image.sha256 != [0; 32]
                && self.device_ids[0] != 0
                && self.device_ids[1] != 0
                && self.device_ids[0] != self.device_ids[1]
                && (1..=10_000).contains(&self.dispatch_timeout_ms)
                && (1_000..=3_600_000).contains(&self.child_deadline_ms),
            "prefix decode request scope or bounds",
        )
    }
}

#[derive(Serialize)]
pub struct Observation {
    pub schema: &'static str,
    pub request: Config,
    pub child_pid: u32,
    /// Original two-bank source/setup custody, not four-forward source proof.
    pub registration_sha256: [u8; 32],
    pub source_program_sha256: [u8; 32],
    pub upload_manifest_sha256: [u8; 32],
    pub bootstrap: wire::Bootstrap,
    pub profile_sha256: [u8; 32],
    pub setup_commands: u64,
    pub completed_forwards: u32,
    pub input_tokens: Vec<u32>,
    pub observed_output_tokens: Vec<u32>,
    pub page_permutation: Vec<u32>,
    pub transcript_sha256: [u8; 32],
    pub request_stream_bytes: usize,
    pub response_stream_bytes: usize,
    pub files: evidence::Files,
    pub close: wire::Response,
    pub child_exit_zero: bool,
    pub process_group_absent: bool,
    pub native_closed: bool,
    pub gpu_execution: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
    pub full_long_workload: bool,
}

fn metadata(
    batch: &EngineeringTpPreparedBatchV1,
    program: [u8; 32],
    theta: u32,
) -> Result<EngineeringTp2Finite2304MetadataV1> {
    require(
        batch.rows().len() == 1
            && theta == 1_000_000
            && batch.context_tokens() == 2304
            && batch.physical_page_count() == 144
            && batch.page_table_stride() == 144,
        "prefix decode metadata requires one authentic prepared row",
    )?;
    let row = &batch.rows()[0];
    require(
        row.position() < wire::FORWARDS
            && row.physical_pages().len() == (row.position() / 16 + 1) as usize,
        "prefix decode authentic page/history extent",
    )?;
    let mut page_table = vec![u32::MAX; 144];
    page_table[..row.physical_pages().len()].copy_from_slice(row.physical_pages());
    let (mut cos_sin, sin) = crate::tp_execution::rope_bytes(row.position(), theta);
    cos_sin.extend(sin);
    EngineeringTp2Finite2304MetadataV1::prepare(&EngineeringTp2GraphInputV1 {
        geometry: EngineeringTp2GraphGeometryV1::Long2304,
        plan_sha256: program,
        generation: u64::from(row.position()) + 1,
        epoch: u64::from(row.position()),
        token: row.token(),
        position: row.position(),
        page_table,
        cos_sin,
    })
}

fn stable_pages(stable: &mut Option<[u32; 144]>, words: &[u32; 145]) -> Result<()> {
    let pages: [u32; 144] = words[1..]
        .try_into()
        .map_err(|_| "prefix decode page extent")?;
    match stable {
        Some(first) => require(
            *first == pages,
            "prefix decode physical page permutation changed",
        ),
        None => {
            *stable = Some(pages);
            Ok(())
        }
    }
}

fn response_identity(request: &wire::Request, response: &wire::Response) -> Result<()> {
    request.validate().map_err(|e| e.to_string())?;
    require(
        response.protocol == request.protocol
            && response.id == request.id
            && response.device_ids == request.device_ids
            && response.session == request.session
            && response.registration == request.registration
            && response.profile_sha256 == request.profile_sha256
            && response.gpu_execution
            && !response.numerical_acceptance
            && !response.performance_claim
            && !response.production_authority,
        "prefix decode response identity or claim boundary",
    )
}

fn validate_completion<'a>(
    request: &wire::Request,
    response: &'a wire::Response,
    control: &wire::Control,
    payload: &[u8],
    chain: &mut wire::Chain,
) -> Result<&'a wire::Completion> {
    control.validate().map_err(|e| e.to_string())?;
    validate_completion_bytes(request, response, &control.encode(), payload, chain)
}
fn validate_completion_bytes<'a>(
    request: &wire::Request,
    response: &'a wire::Response,
    control: &[u8],
    payload: &[u8],
    chain: &mut wire::Chain,
) -> Result<&'a wire::Completion> {
    response_identity(request, response)?;
    let wire::Command::Forward {
        generation, token, ..
    } = &request.command
    else {
        return Err("prefix decode completion for non-forward".into());
    };
    let wire::Event::Completed(c) = &response.event else {
        return Err("prefix decode expected Completed".into());
    };
    require(
        !response.native_closed
            && c.generation == *generation
            && c.position as u64 + 1 == *generation
            && c.input_token == *token
            && c.output_token < VOCABULARY
            && c.control == old::part(control)
            && c.observation.bytes as usize == old::OBSERVATION_BYTES
            && c.observation.sha256 != [0; 32],
        "prefix decode completion token, state, payload or position",
    )?;
    require(
        c.capture == old::Payload::from_bytes(payload).map_err(|e| e.to_string())?
            && c.capture.total == c.observation,
        "prefix decode full capture binding",
    )?;
    require(
        payload
            .chunks_exact(2)
            .all(|b| u16::from_le_bytes([b[0], b[1]]) & 0x7f80 != 0x7f80),
        "prefix decode nonfinite captured BF16",
    )?;
    let logits = &payload[old::OBSERVATION_BYTES - 303_872..];
    let mut maximum = f32::NEG_INFINITY;
    let mut winner = 0_u32;
    for (index, bits) in logits.chunks_exact(2).enumerate() {
        let value = f32::from_bits(u32::from(u16::from_le_bytes([bits[0], bits[1]])) << 16);
        if value > maximum {
            maximum = value;
            winner = index as u32;
        }
    }
    require(
        winner == c.output_token,
        "prefix decode captured logits/lowest-index argmax",
    )?;
    require(
        chain.advance(c) == c.chain,
        "prefix decode transcript chain mismatch",
    )?;
    Ok(c)
}

fn validate_close(
    request: &wire::Request,
    response: &wire::Response,
    control: Option<&wire::Control>,
    payload: &[u8],
    chain: [u8; 32],
) -> Result<()> {
    response_identity(request, response)?;
    require(
        matches!(request.command, wire::Command::Close)
            && response.native_closed
            && control.is_none()
            && payload.is_empty()
            && response.event
                == (wire::Event::Closed {
                    completed_forwards: wire::FORWARDS,
                    transcript_sha256: chain,
                }),
        "prefix decode close count, transcript or lifecycle",
    )
}

/// A successful value is produced only after full validation and file retention.
fn commit_completed(
    pool: &mut EngineeringTpPagedPoolV1,
    batch: &EngineeringTpPreparedBatchV1,
    result: Result<u32>,
) -> Result<u32> {
    let outcome = result.and_then(|value| {
        pool.commit_batch(
            batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(batch),
        )
        .map_err(|e| format!("{e:?}"))?;
        Ok(value)
    });
    match outcome {
        Ok(value) => Ok(value),
        Err(error) => {
            pool.quarantine_batch(batch)
                .map_err(|e| format!("prefix decode quarantine: {e:?}; original: {error}"))?;
            Err(error)
        }
    }
}

/// Run four authenticated TF or own-output AR inputs with all-layer Prefix284/MLP548.
/// The third/fourth calls exercise state-bank rearm; this is not a full-long result.
/// No retry, fallback or numerical acceptance; provisional files survive failure.
pub fn run(config: Config, allow_unauthenticated_machine_code: bool) -> Result<Observation> {
    run_inner(config, allow_unauthenticated_machine_code, None).map(|(observation, _)| observation)
}
fn run_inner(
    config: Config,
    allow_unauthenticated_machine_code: bool,
    diagnostic: Option<PathBuf>,
) -> Result<(Observation, Option<FilePin>)> {
    run_with_diagnostic(
        config,
        allow_unauthenticated_machine_code,
        diagnostic.map(HostDiagnostic::V1),
    )
}
enum HostDiagnostic {
    Projection(PathBuf, FilePin),
    ProjectionShared(PathBuf, FilePin),
    Ordered(PathBuf, FilePin),
    Device(PathBuf),
    DeviceClock(PathBuf),
    V1(PathBuf),
    V2(PathBuf, crate::prefix_decode_host_observation_v2::Policy),
}
impl HostDiagnostic {
    fn path(&self) -> &std::path::Path {
        match self {
            Self::V1(path)
            | Self::V2(path, _)
            | Self::Device(path)
            | Self::DeviceClock(path)
            | Self::Projection(path, _)
            | Self::ProjectionShared(path, _)
            | Self::Ordered(path, _) => path,
        }
    }
    fn launch_flag(&self) -> &'static str {
        match self {
            Self::Ordered(_, _) => "--engineering-native-projection-residual-mlp-ordered-v1",
            Self::Projection(_, _) => "--engineering-native-projection-residual-decode-host-v1",
            Self::ProjectionShared(_, _) => {
                "--engineering-native-projection-residual-decode-shared-host-v1"
            }
            Self::Device(_) => "--engineering-native-prefix-decode-device-v1",
            Self::DeviceClock(_) => "--engineering-native-prefix-decode-device-clock-v2",
            Self::V1(_) => "--engineering-native-prefix-decode-host-v1",
            Self::V2(_, _) => "--engineering-native-prefix-decode-host-v2",
        }
    }
    fn append_args(&self, command: &mut Command) {
        command
            .arg(if matches!(self, Self::Device(_) | Self::DeviceClock(_)) {
                "--device-sidecar"
            } else {
                "--host-sidecar"
            })
            .arg(self.path());
        if let Self::V2(_, policy) = self {
            command.arg("--host-policy").arg(policy.name());
        }
    }
    fn validate(&self, observation: &Observation) -> Result<FilePin> {
        match self {
            Self::Ordered(path, image) => ordered_host::validate(path, observation, image),
            Self::Projection(path, image) => projection_host::validate(path, observation, image),
            Self::ProjectionShared(path, image) => {
                projection_host::validate_shared(path, observation, image)
            }
            Self::Device(path) => device_v1::validate(path, observation),
            Self::DeviceClock(path) => device_clock_v2::validate(path, observation),
            Self::V1(path) => host_observation::validate(path, observation),
            Self::V2(path, policy) => host_policy_v2::validate(path, observation, *policy),
        }
    }
}
fn run_with_diagnostic(
    config: Config,
    allow_unauthenticated_machine_code: bool,
    diagnostic: Option<HostDiagnostic>,
) -> Result<(Observation, Option<FilePin>)> {
    run_selected(config, allow_unauthenticated_machine_code, diagnostic, None)
        .map(|(observation, host, _)| (observation, host))
}

fn worker_flag(diagnostic: Option<&HostDiagnostic>, projection: bool) -> Result<&'static str> {
    match (projection, diagnostic) {
        (true, None) => Ok("--engineering-native-projection-residual-decode-v1"),
        (
            true,
            Some(
                value @ (HostDiagnostic::Projection(_, _)
                | HostDiagnostic::ProjectionShared(_, _)
                | HostDiagnostic::Ordered(_, _)),
            ),
        ) => Ok(value.launch_flag()),
        (
            false,
            Some(
                HostDiagnostic::Projection(_, _)
                | HostDiagnostic::ProjectionShared(_, _)
                | HostDiagnostic::Ordered(_, _),
            ),
        )
        | (true, Some(_)) => Err("projection decode cannot mix legacy diagnostics".into()),
        (false, diagnostic) => Ok(diagnostic
            .map(HostDiagnostic::launch_flag)
            .unwrap_or("--engineering-native-prefix-decode-v1")),
    }
}

fn run_selected(
    config: Config,
    allow_unauthenticated_machine_code: bool,
    diagnostic: Option<HostDiagnostic>,
    projection: Option<&FilePin>,
) -> Result<(
    Observation,
    Option<FilePin>,
    Option<projection_wire::Bootstrap>,
)> {
    require(
        allow_unauthenticated_machine_code,
        "prefix decode explicit engineering machine-code opt-in required",
    )?;
    config.validate()?;
    let launch_flag = worker_flag(diagnostic.as_ref(), projection.is_some())?;
    let ordered = matches!(&diagnostic, Some(HostDiagnostic::Ordered(_, _)));
    require(
        !ordered || config.mode == wire::InputMode::Autoregressive,
        "ordered route is AR4 only",
    )?;
    if let Some(diagnostic) = &diagnostic {
        host_observation::preflight(diagnostic.path())?;
    }
    config.worker.read(512 << 20, false)?;
    let images = config.images.read()?;
    let tiles_image = config.read_tiles_image()?;
    let prefix_image = config.read_prefix_image()?;
    let projection_image = projection
        .map(|pin| pin.read(wire::MAX_IMAGE_BYTES as u64, true))
        .transpose()?;
    let (prompt_text, prompt) = config.prompt.read()?;
    let model = EngineeringQwenModelV1::open(&config.source)?;
    require(
        *model.bundle_id().as_bytes() == config.expected_bundle_id
            && *model.config().model_id.as_bytes() == config.expected_model_id,
        "prefix decode authenticated model/bundle differs",
    )?;
    require(
        model.encode_with_limits(
            &prompt_text,
            ferric_build::TokenizerExecutionLimits::long_context(),
        )? == prompt,
        "prefix decode authenticated tokenizer/raw prompt differs",
    )?;
    let mut evidence = evidence::Evidence::create(&config.evidence_directory)?;
    let scope = EngineeringTpPoolScopeV1 {
        model: config.expected_model_id,
        session: config.session,
    };
    let limits =
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).map_err(|e| format!("{e:?}"))?;
    let mut pool = EngineeringTpPagedPoolV1::new(scope, limits).map_err(|e| format!("{e:?}"))?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(config.child_deadline_ms))
        .ok_or("prefix decode deadline overflow")?;
    let mut command = Command::new(&config.worker.path);
    command
        .args([
            launch_flag,
            "--allow-unauthenticated-machine-code",
            "--devices",
        ])
        .arg(format!("{},{}", config.device_ids[0], config.device_ids[1]))
        .arg("--timeout-ms")
        .arg(config.dispatch_timeout_ms.to_string())
        .arg("--mode")
        .arg(match config.mode {
            wire::InputMode::TeacherForced => "teacher-forced",
            wire::InputMode::Autoregressive => "autoregressive",
        })
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C");
    if let Some(diagnostic) = &diagnostic {
        diagnostic.append_args(&mut command);
    }
    let mut child = process::OwnedChild::spawn_command(command, deadline)?;
    let pid = child.id();
    eprintln!("finite engineering owned child pid={pid} pgid={pid}; no native setup acknowledged");
    eprintln!(
        "finite explicit profile={} mode={:?}",
        if ordered {
            "projection-residual-mlp-ordered-ar4-shared-v1"
        } else if projection.is_some() {
            "projection-residual-prefix284-mlp548-four-forward-v1"
        } else {
            "prefix284-mlp548-four-forward-v1"
        },
        config.mode
    );
    config.worker.read(512 << 20, false)?;
    let mut sent = wire::FrameBudget::new();
    let mut received = wire::FrameBudget::new();
    let recorder = EngineeringTp2FiniteSourceRecorderV1::new(&model, &pool, pid)?;
    child.check_deadline()?;
    let (
        registration_sha256,
        program_sha256,
        manifest_sha256,
        bootstrap,
        projection_bootstrap,
        profile_sha256,
        setup_commands,
    ) = recorder.with_plan(|composition, source, uploads| {
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
        let registration_raw = serde_json::to_vec(&registration).map_err(|e| e.to_string())?;
        let manifest_raw = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
        let registration_sha = hash(&registration_raw);
        let manifest_sha = hash(&manifest_raw);
        let begin = setup_wire::Begin {
            scope: setup::scope(&registration),
            registration: old::part(&registration_raw),
            source_program: old::part(&program),
            uploads: old::part(&manifest_raw),
            prefix_image: old::part(&images.prefix),
            mlp_image: old::part(&images.mlp),
            residual_image: old::part(&images.residual),
            tail_image: Some(old::part(&images.tail)),
        };
        let bootstrap = wire::Bootstrap {
            protocol: wire::PROTOCOL,
            profile: wire::Profile::Prefix284Mlp548FourForwardV1,
            device_ids: config.device_ids,
            scope: setup::scope(&registration),
            timeout_ms: config.dispatch_timeout_ms,
            registration: registration_sha,
            begin,
            mode: config.mode,
            input_tokens: match config.mode {
                wire::InputMode::TeacherForced => INPUT_TOKENS.to_vec(),
                wire::InputMode::Autoregressive => vec![INPUT_TOKENS[0]],
            },
            tiles_image: old::part(&tiles_image),
            prefix_image: old::part(&prefix_image),
        };
        bootstrap
            .validate(
                config.device_ids,
                config.dispatch_timeout_ms,
                pid,
                config.mode,
            )
            .map_err(|e| e.to_string())?;
        let projection_bootstrap = projection
            .map(|pin| projection::bootstrap(bootstrap.clone(), pin))
            .transpose()?;
        let profile = match &projection_bootstrap {
            Some(selected) if ordered => projection_ordered::selected(selected).sha256(),
            Some(selected) => selected.sha256(),
            None => bootstrap.sha256(),
        }
        .map_err(|e| e.to_string())?;
        child.check_deadline()?;
        let input = child
            .input
            .as_mut()
            .ok_or("prefix decode closed bootstrap stdin")?;
        match &projection_bootstrap {
            Some(selected) if ordered => ordered_wire::write_bootstrap(
                input,
                &mut sent,
                &projection_ordered::selected(selected),
                &tiles_image,
                &prefix_image,
                projection_image.as_deref().ok_or("ordered image missing")?,
            ),
            Some(selected) => projection_wire::write_bootstrap(
                input,
                &mut sent,
                selected,
                &tiles_image,
                &prefix_image,
                projection_image
                    .as_deref()
                    .ok_or("projection decode image missing")?,
            ),
            None => {
                wire::write_bootstrap(input, &mut sent, &bootstrap, &tiles_image, &prefix_image)
            }
        }
        .map_err(|e| e.to_string())?;
        wire::account_begin(&mut sent, &bootstrap).map_err(|e| e.to_string())?;
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
                require(
                    offset == total,
                    "prefix decode head recipe contiguous offset",
                )?;
                stream.write_tail(offset, bytes)?;
                total += bytes.len();
                digest.update(bytes);
                Ok(())
            },
        )?;
        let actual: [u8; 32] = digest.finalize().into();
        require(
            total == head.bytes() && actual == head.sha256(),
            "prefix decode head recipe changed",
        )?;
        Ok((
            registration_sha,
            hash(&program),
            manifest_sha,
            bootstrap,
            projection_bootstrap,
            profile,
            stream.seal()?,
        ))
    })?;
    let sequence = pool
        .open_sequence(scope, &[prompt[0]], 0)
        .map_err(|e| format!("{e:?}"))?;
    require(
        sequence.hit_tokens() == 0 && sequence.physical_pages().is_empty(),
        "prefix decode requires fresh history",
    )?;
    let mut stable = None;
    let mut chain = wire::Chain::new(registration_sha256, profile_sha256);
    let mut outputs = Vec::with_capacity(4);
    let mut inputs = Vec::with_capacity(4);
    for position in 0..wire::FORWARDS {
        let token = bootstrap
            .input(position, outputs.last().copied())
            .map_err(|e| e.to_string())?;
        require(
            pool.committed_position(sequence.sequence())
                .map_err(|e| format!("{e:?}"))?
                == position,
            "prefix decode native/host committed position",
        )?;
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence: sequence.sequence(),
                token,
                position,
            }])
            .map_err(|e| format!("{e:?}"))?;
        let result: Result<u32> = (|| {
            pool.begin_submission(&batch)
                .map_err(|e| format!("{e:?}"))?;
            let metadata = metadata(&batch, program_sha256, model.config().rope_theta)?;
            stable_pages(&mut stable, metadata.cache_metadata())?;
            let request = wire::Request {
                protocol: wire::PROTOCOL,
                id: u64::from(position) + 1,
                device_ids: config.device_ids,
                session: config.session,
                registration: registration_sha256,
                profile_sha256,
                command: wire::Command::Forward {
                    generation: u64::from(position) + 1,
                    token,
                    cache_metadata: metadata.cache_metadata().to_vec(),
                    rotary_bits: metadata.rotary().iter().map(|v| v.to_bits()).collect(),
                },
            };
            child.check_deadline()?;
            wire::write_request(
                child
                    .input
                    .as_mut()
                    .ok_or("prefix decode closed forward stdin")?,
                &mut sent,
                &request,
            )
            .map_err(|e| e.to_string())?;
            if ordered {
                let (response, control, payload) = ordered_wire::read_response(
                    child.output.as_mut().ok_or("ordered stdout closed")?,
                    &mut received,
                )
                .map_err(|e| e.to_string())?
                .ok_or("ordered EOF before completion")?;
                let control = control.ok_or("ordered control absent")?;
                control.validate().map_err(|e| e.to_string())?;
                let raw = control.encode();
                let completion =
                    validate_completion_bytes(&request, &response, &raw, &payload, &mut chain)?;
                child.check_deadline()?;
                evidence.append_encoded(&request, &response, &raw, &payload)?;
                return Ok(completion.output_token);
            }
            let (response, control, payload) = wire::read_response(
                child.output.as_mut().ok_or("prefix decode closed stdout")?,
                &mut received,
            )
            .map_err(|e| e.to_string())?
            .ok_or("prefix decode EOF before completion")?;
            let control = control.ok_or("prefix decode missing completion control")?;
            let completion =
                validate_completion(&request, &response, &control, &payload, &mut chain)?;
            child.check_deadline()?;
            evidence.append(&request, &response, &control, &payload)?;
            Ok(completion.output_token)
        })();
        let output = commit_completed(&mut pool, &batch, result)?;
        inputs.push(token);
        outputs.push(output);
        eprintln!(
            "finite prefix decode completed position={position} forwards={}",
            position + 1
        );
    }
    require(outputs.len() == 4, "prefix decode observed output count")?;
    let close_request = wire::Request {
        protocol: wire::PROTOCOL,
        id: u64::from(wire::FORWARDS) + 1,
        device_ids: config.device_ids,
        session: config.session,
        registration: registration_sha256,
        profile_sha256,
        command: wire::Command::Close,
    };
    child.check_deadline()?;
    wire::write_request(
        child
            .input
            .as_mut()
            .ok_or("prefix decode closed close stdin")?,
        &mut sent,
        &close_request,
    )
    .map_err(|e| e.to_string())?;
    let close = if ordered {
        let (close, control, payload) = ordered_wire::read_response(
            child.output.as_mut().ok_or("ordered Close stdout absent")?,
            &mut received,
        )
        .map_err(|e| e.to_string())?
        .ok_or("ordered EOF before Close")?;
        require(control.is_none(), "ordered Close control must be absent")?;
        validate_close(&close_request, &close, None, &payload, chain.digest())?;
        close
    } else {
        let (close, control, payload) = wire::read_response(
            child
                .output
                .as_mut()
                .ok_or("prefix decode closed close stdout")?,
            &mut received,
        )
        .map_err(|e| e.to_string())?
        .ok_or("prefix decode EOF before Close")?;
        validate_close(
            &close_request,
            &close,
            control.as_ref(),
            &payload,
            chain.digest(),
        )?;
        close
    };
    let stderr = child.finish()?;
    config.worker.read(512 << 20, false)?;
    for pin in [
        &config.images.prefix,
        &config.images.mlp,
        &config.images.residual,
        &config.images.tail,
    ] {
        pin.read(64 << 20, false)?;
    }
    config.prompt.recheck()?;
    config
        .tiles_image
        .read(wire::MAX_IMAGE_BYTES as u64, false)?;
    config
        .prefix_image
        .read(wire::MAX_IMAGE_BYTES as u64, false)?;
    if let Some(pin) = projection {
        pin.read(wire::MAX_IMAGE_BYTES as u64, false)?;
    }
    let files = evidence.finish(&stderr)?;
    let mut observation = Observation {
        schema: "FerricFinitePrefixDecodeObservationV1",
        request: config,
        child_pid: pid,
        registration_sha256,
        source_program_sha256: program_sha256,
        upload_manifest_sha256: manifest_sha256,
        bootstrap,
        profile_sha256,
        setup_commands,
        completed_forwards: wire::FORWARDS,
        input_tokens: inputs,
        observed_output_tokens: outputs,
        page_permutation: stable
            .ok_or("prefix decode missing page permutation")?
            .to_vec(),
        transcript_sha256: chain.digest(),
        request_stream_bytes: sent.used(),
        response_stream_bytes: received.used(),
        files,
        close,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_long_workload: false,
    };
    let host = diagnostic
        .as_ref()
        .map(|diagnostic| diagnostic.validate(&observation))
        .transpose()?;
    if diagnostic.is_none() && projection.is_none() {
        evidence::publish(&mut observation)?;
    }
    Ok((observation, host, projection_bootstrap))
}

#[cfg(test)]
mod tests;

pub mod projection_host;

pub mod ordered_host;
pub mod projection_ordered;
