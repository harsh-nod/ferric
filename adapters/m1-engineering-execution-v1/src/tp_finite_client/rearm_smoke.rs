//! Four authentic teacher-forced forwards to exercise two-bank rearm, not a long run.

use super::{FilePin, ImagePins, Result, hash, process, require, setup};
use crate::finite_forward_wire_v1 as old;
use crate::finite_rearm_smoke_wire_v1 as wire;
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

mod evidence;

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
            "smoke profile requires the exact retained raw 2048-token prompt",
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
            "smoke prompt manifest/token projection",
        )?;
        require(
            tokens.len() == 2048
                && tokens.iter().all(|v| *v < VOCABULARY)
                && tokens[..4] == INPUT_TOKENS,
            "smoke prompt vocabulary or extent",
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
    pub capture_layer0: bool,
    pub evidence_directory: PathBuf,
    pub dispatch_timeout_ms: u32,
    pub child_deadline_ms: u64,
}
impl Config {
    /// Parse without opening files or devices.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= 65_536,
            "smoke request byte bound",
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
            "smoke retained request/path bound",
        )?;
        require(
            self.schema == "FerricFiniteRearmSmokeRequestV1"
                && self.source.is_absolute()
                && self.evidence_directory.is_absolute()
                && self.evidence_directory.file_name().is_some()
                && self.expected_bundle_id != [0; 32]
                && self.expected_model_id != [0; 32]
                && self.session != [0; 32]
                && self.device_ids[0] != 0
                && self.device_ids[1] != 0
                && self.device_ids[0] != self.device_ids[1]
                && (1..=10_000).contains(&self.dispatch_timeout_ms)
                && (1_000..=3_600_000).contains(&self.child_deadline_ms),
            "smoke request scope or bounds",
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
    pub input_tokens: [u32; 4],
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

fn input_token(position: u32) -> Result<u32> {
    INPUT_TOKENS
        .get(position as usize)
        .copied()
        .ok_or("smoke requires exactly four teacher-forced inputs".into())
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
        "smoke metadata requires one authentic prepared row",
    )?;
    let row = &batch.rows()[0];
    require(
        row.position() < wire::FORWARDS
            && row.physical_pages().len() == (row.position() / 16 + 1) as usize,
        "smoke authentic page/history extent",
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
    let pages: [u32; 144] = words[1..].try_into().map_err(|_| "smoke page extent")?;
    match stable {
        Some(first) => require(*first == pages, "smoke physical page permutation changed"),
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
        "smoke response identity or claim boundary",
    )
}

// Projection only after the shared wire decoder has checked the exact closed
// schema, all 34 parts, offsets, hashes, finite values and diagnostic-only flags.
#[derive(Deserialize)]
struct StageProjection {
    parts: Vec<StagePart>,
    payload: Vec<u8>,
}
#[derive(Deserialize)]
struct StagePart {
    boundary: String,
    rank: u32,
    role: String,
    offset: usize,
    bytes: usize,
}
fn stage_part<'a>(
    stage: &'a StageProjection,
    boundary: &str,
    rank: u32,
    role: &str,
) -> Result<&'a [u8]> {
    let mut found = stage
        .parts
        .iter()
        .filter(|p| p.boundary == boundary && p.rank == rank && p.role == role);
    let part = found.next().ok_or("smoke missing stage join part")?;
    require(found.next().is_none(), "smoke duplicate stage join part")?;
    let end = part
        .offset
        .checked_add(part.bytes)
        .ok_or("smoke stage part overflow")?;
    stage
        .payload
        .get(part.offset..end)
        .ok_or("smoke stage part bounds".into())
}
fn validate_stage_join(request: &wire::Request, bytes: &[u8], main: &[u8]) -> Result<()> {
    wire::validate_stage_capture(bytes).map_err(|e| e.to_string())?;
    let wire::Command::Forward {
        generation,
        cache_metadata,
        rotary_bits,
        ..
    } = &request.command
    else {
        return Err("smoke stage attached to non-forward".into());
    };
    require(
        request.id == 1 && *generation == 1 && main.len() == old::OBSERVATION_BYTES,
        "smoke stage requires exact generation1 main capture",
    )?;
    let stage: StageProjection = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let metadata: Vec<u8> = cache_metadata
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let rotary: Vec<u8> = rotary_bits.iter().flat_map(|v| v.to_le_bytes()).collect();
    for rank in 0..2 {
        require(
            stage_part(&stage, "before_prefix", rank, "cache_metadata")? == metadata
                && stage_part(&stage, "before_prefix", rank, "rotary")? == rotary,
            "smoke stage metadata/rotary differs from actual request",
        )?;
        require(
            stage_part(&stage, "after_final_residual", rank, "final_hidden")? == &main[..8192],
            "smoke layer0 final residual differs from main hidden capture",
        )?;
    }
    require(
        stage_part(&stage, "before_prefix", 0, "input")?
            == stage_part(&stage, "before_prefix", 1, "input")?,
        "smoke layer0 replicated input differs",
    )
}

fn validate_completion<'a>(
    request: &wire::Request,
    response: &'a wire::Response,
    control: &wire::Control,
    payload: &[u8],
    stage: &[u8],
    capture_layer0: bool,
    chain: &mut wire::Chain,
) -> Result<&'a wire::Completion> {
    response_identity(request, response)?;
    let wire::Command::Forward {
        generation, token, ..
    } = &request.command
    else {
        return Err("smoke completion for non-forward".into());
    };
    let wire::Event::Completed(c) = &response.event else {
        return Err("smoke expected Completed".into());
    };
    control.validate().map_err(|e| e.to_string())?;
    require(
        !response.native_closed
            && c.generation == *generation
            && c.position as u64 + 1 == *generation
            && c.input_token == *token
            && c.output_token < VOCABULARY
            && c.control == old::part(&control.encode())
            && c.observation.bytes as usize == old::OBSERVATION_BYTES
            && c.observation.sha256 != [0; 32]
            && c.stage_capture.is_some() == (capture_layer0 && *generation == 1),
        "smoke completion token, state, payload or position",
    )?;
    require(
        c.capture == old::Payload::from_bytes(payload).map_err(|e| e.to_string())?
            && c.capture.total == c.observation,
        "smoke full capture binding",
    )?;
    require(
        payload
            .chunks_exact(2)
            .all(|b| u16::from_le_bytes([b[0], b[1]]) & 0x7f80 != 0x7f80),
        "smoke nonfinite captured BF16",
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
        "smoke captured logits/lowest-index argmax",
    )?;
    if let Some(expected) = &c.stage_capture {
        require(old::part(stage) == *expected, "smoke stage annex digest")?;
        validate_stage_join(request, stage, payload)?;
    } else {
        require(stage.is_empty(), "smoke undeclared stage annex")?;
    }
    require(
        chain.advance(c) == c.chain,
        "smoke transcript chain mismatch",
    )?;
    Ok(c)
}

fn validate_close(
    request: &wire::Request,
    response: &wire::Response,
    control: Option<&wire::Control>,
    payload: &[u8],
    stage: &[u8],
    chain: [u8; 32],
) -> Result<()> {
    response_identity(request, response)?;
    require(
        matches!(request.command, wire::Command::Close)
            && response.native_closed
            && control.is_none()
            && payload.is_empty()
            && stage.is_empty()
            && response.event
                == (wire::Event::Closed {
                    completed_forwards: wire::FORWARDS,
                    transcript_sha256: chain,
                }),
        "smoke close count, transcript or lifecycle",
    )
}

/// Run four authenticated teacher-forced prompt tokens. The third/fourth calls
/// exercise state-bank rearm; this is not an autoregressive or full-long result.
/// No retry, fallback or numerical acceptance; provisional files survive failure.
pub fn run(config: Config, allow_unauthenticated_machine_code: bool) -> Result<Observation> {
    require(
        allow_unauthenticated_machine_code,
        "smoke explicit engineering machine-code opt-in required",
    )?;
    config.validate()?;
    config.worker.read(512 << 20, false)?;
    let images = config.images.read()?;
    let (prompt_text, prompt) = config.prompt.read()?;
    let model = EngineeringQwenModelV1::open(&config.source)?;
    require(
        *model.bundle_id().as_bytes() == config.expected_bundle_id
            && *model.config().model_id.as_bytes() == config.expected_model_id,
        "smoke authenticated model/bundle differs",
    )?;
    require(
        model.encode_with_limits(
            &prompt_text,
            ferric_build::TokenizerExecutionLimits::long_context(),
        )? == prompt,
        "smoke authenticated tokenizer/raw prompt differs",
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
        .ok_or("smoke deadline overflow")?;
    let mut command = Command::new(&config.worker.path);
    command
        .args([
            "--engineering-native-rearm-smoke-v1",
            "--allow-unauthenticated-machine-code",
            "--devices",
        ])
        .arg(format!("{},{}", config.device_ids[0], config.device_ids[1]))
        .arg("--timeout-ms")
        .arg(config.dispatch_timeout_ms.to_string())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C");
    if config.capture_layer0 {
        command.arg("--capture-layer0");
    }
    let mut child = process::OwnedChild::spawn_command(command, deadline)?;
    let pid = child.id();
    eprintln!("finite engineering owned child pid={pid} pgid={pid}; no native setup acknowledged");
    eprintln!(
        "finite explicit profile=rearm-four-forward-v1 capture_layer0={}",
        config.capture_layer0
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
        let registration_sha = hash(&serde_json::to_vec(&registration).map_err(|e| e.to_string())?);
        let manifest_sha = hash(&serde_json::to_vec(&manifest).map_err(|e| e.to_string())?);
        let bootstrap = wire::Bootstrap {
            protocol: wire::PROTOCOL,
            profile: wire::Profile::RearmFourForwardV1,
            device_ids: config.device_ids,
            scope: setup::scope(&registration),
            timeout_ms: config.dispatch_timeout_ms,
            prompt_tokens: INPUT_TOKENS.to_vec(),
            capture_layer0: config.capture_layer0,
        };
        bootstrap
            .validate(
                config.device_ids,
                config.dispatch_timeout_ms,
                pid,
                config.capture_layer0,
            )
            .map_err(|e| e.to_string())?;
        let profile = bootstrap.sha256().map_err(|e| e.to_string())?;
        child.check_deadline()?;
        wire::write_bootstrap(
            child.input.as_mut().ok_or("smoke closed bootstrap stdin")?,
            &mut sent,
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
                require(offset == total, "smoke head recipe contiguous offset")?;
                stream.write_tail(offset, bytes)?;
                total += bytes.len();
                digest.update(bytes);
                Ok(())
            },
        )?;
        let actual: [u8; 32] = digest.finalize().into();
        require(
            total == head.bytes() && actual == head.sha256(),
            "smoke head recipe changed",
        )?;
        Ok((
            registration_sha,
            hash(&program),
            manifest_sha,
            bootstrap,
            profile,
            stream.seal()?,
        ))
    })?;
    let sequence = pool
        .open_sequence(scope, &[prompt[0]], 0)
        .map_err(|e| format!("{e:?}"))?;
    require(
        sequence.hit_tokens() == 0 && sequence.physical_pages().is_empty(),
        "smoke requires fresh history",
    )?;
    let mut stable = None;
    let mut chain = wire::Chain::new(registration_sha256, profile_sha256);
    let mut outputs = Vec::with_capacity(4);
    for position in 0..wire::FORWARDS {
        let token = input_token(position)?;
        require(
            pool.committed_position(sequence.sequence())
                .map_err(|e| format!("{e:?}"))?
                == position,
            "smoke native/host committed position",
        )?;
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence: sequence.sequence(),
                token,
                position,
            }])
            .map_err(|e| format!("{e:?}"))?;
        pool.begin_submission(&batch)
            .map_err(|e| format!("{e:?}"))?;
        let result: Result<u32> = (|| {
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
                child.input.as_mut().ok_or("smoke closed forward stdin")?,
                &mut sent,
                &request,
            )
            .map_err(|e| e.to_string())?;
            let (response, control, payload, stage) = wire::read_response(
                child.output.as_mut().ok_or("smoke closed stdout")?,
                &mut received,
            )
            .map_err(|e| e.to_string())?
            .ok_or("smoke EOF before completion")?;
            let control = control.ok_or("smoke missing completion control")?;
            let completion = validate_completion(
                &request,
                &response,
                &control,
                &payload,
                &stage,
                config.capture_layer0,
                &mut chain,
            )?;
            child.check_deadline()?;
            evidence.append(&response, &control, &payload, &stage)?;
            Ok(completion.output_token)
        })();
        let output = match result {
            Ok(value) => value,
            Err(error) => {
                pool.quarantine_batch(&batch)
                    .map_err(|e| format!("smoke quarantine: {e:?}; original: {error}"))?;
                return Err(error);
            }
        };
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .map_err(|e| format!("{e:?}"))?;
        outputs.push(output);
        eprintln!(
            "finite rearm smoke completed position={position} forwards={}",
            position + 1
        );
    }
    require(outputs.len() == 4, "smoke observed output count")?;
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
        child.input.as_mut().ok_or("smoke closed close stdin")?,
        &mut sent,
        &close_request,
    )
    .map_err(|e| e.to_string())?;
    let (close, control, payload, stage) = wire::read_response(
        child.output.as_mut().ok_or("smoke closed close stdout")?,
        &mut received,
    )
    .map_err(|e| e.to_string())?
    .ok_or("smoke EOF before Close")?;
    validate_close(
        &close_request,
        &close,
        control.as_ref(),
        &payload,
        &stage,
        chain.digest(),
    )?;
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
    let files = evidence.finish(&stderr)?;
    let mut observation = Observation {
        schema: "FerricFiniteRearmSmokeObservationV1",
        request: config,
        child_pid: pid,
        registration_sha256,
        source_program_sha256: program_sha256,
        upload_manifest_sha256: manifest_sha256,
        bootstrap,
        profile_sha256,
        setup_commands,
        completed_forwards: wire::FORWARDS,
        input_tokens: INPUT_TOKENS,
        observed_output_tokens: outputs,
        page_permutation: stable.ok_or("smoke missing page permutation")?.to_vec(),
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
    evidence::publish(&mut observation)?;
    Ok(observation)
}

#[cfg(test)]
mod tests;
