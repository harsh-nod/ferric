//! Fixed 2048-prompt/256-output engineering stream, separate from the two-forward API.

use super::{FilePin, ImagePins, Result, hash, process, require, setup};
use crate::finite_forward_wire_v1 as old;
use crate::finite_long_wire_v1 as wire;
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
#[cfg(feature = "guarded-mlp-full2303-engineering")]
#[path = "long/full2303.rs"]
pub mod full2303;
#[cfg(feature = "guarded-mlp-model-engineering")]
#[path = "long/readiness.rs"]
pub mod readiness;

const VOCABULARY: u32 = 151_936;
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
            "long profile requires the exact retained raw 2048-token prompt",
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
            "long prompt manifest/token projection",
        )?;
        require(
            tokens.len() == wire::PROMPT_TOKENS && tokens.iter().all(|v| *v < VOCABULARY),
            "long prompt vocabulary or extent",
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
    pub evidence_directory: PathBuf,
    pub dispatch_timeout_ms: u32,
    pub child_deadline_ms: u64,
}
impl Config {
    /// Parse without opening files or devices.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= 65_536,
            "long request byte bound",
        )?;
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        self.prompt.validate()?;
        require(
            serde_json::to_vec(self).map_err(|e| e.to_string())?.len() <= 16_384
                && self.evidence_directory.as_os_str().len() <= 1024,
            "long retained request/path bound",
        )?;
        require(
            self.schema == "FerricFiniteLongRequestV1"
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
            "long request scope or bounds",
        )
    }
}

#[derive(Serialize)]
pub struct Observation {
    pub schema: &'static str,
    pub request: Config,
    pub child_pid: u32,
    /// Original two-bank source/setup custody, not 2303-forward source proof.
    pub registration_sha256: [u8; 32],
    pub source_program_sha256: [u8; 32],
    pub upload_manifest_sha256: [u8; 32],
    pub bootstrap: wire::Bootstrap,
    pub profile_sha256: [u8; 32],
    pub setup_commands: u64,
    pub completed_forwards: u32,
    pub generated_tokens: Vec<u32>,
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
}

fn input_token(prompt: &[u32], position: u32, prior: Option<u32>) -> Result<u32> {
    require(
        prompt.len() == wire::PROMPT_TOKENS && position < wire::FORWARDS,
        "long input schedule bounds",
    )?;
    let token = if (position as usize) < wire::PROMPT_TOKENS {
        prompt[position as usize]
    } else {
        prior.ok_or("long decode requires actual preceding output")?
    };
    require(token < VOCABULARY, "long input vocabulary")?;
    Ok(token)
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
        "long metadata requires one authentic prepared row",
    )?;
    let row = &batch.rows()[0];
    require(
        row.position() < wire::FORWARDS
            && row.physical_pages().len() == (row.position() / 16 + 1) as usize,
        "long authentic page/history extent",
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
    let pages: [u32; 144] = words[1..].try_into().map_err(|_| "long page extent")?;
    match stable {
        Some(first) => require(*first == pages, "long physical page permutation changed"),
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
        "long response identity or claim boundary",
    )
}

fn validate_completion<'a>(
    request: &wire::Request,
    response: &'a wire::Response,
    control: &wire::Control,
    payload: &[u8],
    chain: &mut wire::Chain,
) -> Result<&'a wire::Completion> {
    response_identity(request, response)?;
    let wire::Command::Forward {
        generation, token, ..
    } = &request.command
    else {
        return Err("long completion for non-forward".into());
    };
    let wire::Event::Completed(c) = &response.event else {
        return Err("long expected Completed".into());
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
            && c.capture.is_some() == wire::capture_position(c.position),
        "long completion token, state, payload or position",
    )?;
    if let Some(expected) = &c.capture {
        require(
            *expected == old::Payload::from_bytes(payload).map_err(|e| e.to_string())?
                && expected.total == c.observation,
            "long selected capture binding",
        )?;
        require(
            payload
                .chunks_exact(2)
                .all(|b| u16::from_le_bytes([b[0], b[1]]) & 0x7f80 != 0x7f80),
            "long nonfinite captured BF16",
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
            "long captured logits/lowest-index argmax",
        )?;
    } else {
        require(payload.is_empty(), "long unexpected uncaptured payload")?;
    }
    require(
        chain.advance(c) == c.chain,
        "long transcript chain mismatch",
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
        "long close count, transcript or lifecycle",
    )
}

/// Run the closed prompt then consume the child's actual preceding output on
/// each decode step. No retry, token substitution, old-profile fallback or
/// numerical acceptance is performed. Provisional evidence survives failures.
pub fn run(config: Config, allow_unauthenticated_machine_code: bool) -> Result<Observation> {
    require(
        allow_unauthenticated_machine_code,
        "long explicit engineering machine-code opt-in required",
    )?;
    config.validate()?;
    config.worker.read(512 << 20, false)?;
    let images = config.images.read()?;
    let (prompt_text, prompt) = config.prompt.read()?;
    let model = EngineeringQwenModelV1::open(&config.source)?;
    require(
        *model.bundle_id().as_bytes() == config.expected_bundle_id
            && *model.config().model_id.as_bytes() == config.expected_model_id,
        "long authenticated model/bundle differs",
    )?;
    require(
        model.encode_with_limits(
            &prompt_text,
            ferric_build::TokenizerExecutionLimits::long_context(),
        )? == prompt,
        "long authenticated tokenizer/raw prompt differs",
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
        .ok_or("long deadline overflow")?;
    let mut command = Command::new(&config.worker.path);
    command
        .args([
            "--engineering-native-long-v1",
            "--allow-unauthenticated-machine-code",
            "--devices",
        ])
        .arg(format!("{},{}", config.device_ids[0], config.device_ids[1]))
        .arg("--timeout-ms")
        .arg(config.dispatch_timeout_ms.to_string())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C");
    let mut child = process::OwnedChild::spawn_command(command, deadline)?;
    let pid = child.id();
    eprintln!("finite long owned child pid={pid} pgid={pid}; setup pending");
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
            profile: wire::Profile::Prompt2048Output256V1,
            device_ids: config.device_ids,
            scope: setup::scope(&registration),
            timeout_ms: config.dispatch_timeout_ms,
            prompt_tokens: prompt.clone(),
        };
        bootstrap
            .validate(config.device_ids, config.dispatch_timeout_ms, pid)
            .map_err(|e| e.to_string())?;
        let profile = bootstrap.sha256().map_err(|e| e.to_string())?;
        child.check_deadline()?;
        wire::write_bootstrap(
            child.input.as_mut().ok_or("long closed bootstrap stdin")?,
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
                require(offset == total, "long head recipe contiguous offset")?;
                stream.write_tail(offset, bytes)?;
                total += bytes.len();
                digest.update(bytes);
                Ok(())
            },
        )?;
        let actual: [u8; 32] = digest.finalize().into();
        require(
            total == head.bytes() && actual == head.sha256(),
            "long head recipe changed",
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
        "long requires fresh history",
    )?;
    let mut stable = None;
    let mut chain = wire::Chain::new(registration_sha256, profile_sha256);
    let mut prior = None;
    let mut outputs = Vec::with_capacity(wire::OUTPUT_TOKENS);
    for position in 0..wire::FORWARDS {
        let token = input_token(&prompt, position, prior)?;
        require(
            pool.committed_position(sequence.sequence())
                .map_err(|e| format!("{e:?}"))?
                == position,
            "long native/host committed position",
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
                child.input.as_mut().ok_or("long closed forward stdin")?,
                &mut sent,
                &request,
            )
            .map_err(|e| e.to_string())?;
            let (response, control, payload) = wire::read_response(
                child.output.as_mut().ok_or("long closed stdout")?,
                &mut received,
            )
            .map_err(|e| e.to_string())?
            .ok_or("long EOF before completion")?;
            let control = control.ok_or("long missing completion control")?;
            let completion =
                validate_completion(&request, &response, &control, &payload, &mut chain)?;
            child.check_deadline()?;
            evidence.append(completion, &control, &payload)?;
            Ok(completion.output_token)
        })();
        let output = match result {
            Ok(value) => value,
            Err(error) => {
                pool.quarantine_batch(&batch)
                    .map_err(|e| format!("long quarantine: {e:?}; original: {error}"))?;
                return Err(error);
            }
        };
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .map_err(|e| format!("{e:?}"))?;
        prior = Some(output);
        if position >= wire::PROMPT_TOKENS as u32 - 1 {
            outputs.push(output);
        }
        if position % 64 == 0 || position + 1 == wire::FORWARDS {
            eprintln!(
                "finite long completed position={position} forwards={} generated={}",
                position + 1,
                outputs.len()
            );
        }
    }
    require(
        outputs.len() == wire::OUTPUT_TOKENS,
        "long generated output count",
    )?;
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
        child.input.as_mut().ok_or("long closed close stdin")?,
        &mut sent,
        &close_request,
    )
    .map_err(|e| e.to_string())?;
    let (close, control, payload) = wire::read_response(
        child.output.as_mut().ok_or("long closed close stdout")?,
        &mut received,
    )
    .map_err(|e| e.to_string())?
    .ok_or("long EOF before Close")?;
    validate_close(
        &close_request,
        &close,
        control.as_ref(),
        &payload,
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
        schema: "FerricFiniteLongObservationV1",
        request: config,
        child_pid: pid,
        registration_sha256,
        source_program_sha256: program_sha256,
        upload_manifest_sha256: manifest_sha256,
        bootstrap,
        profile_sha256,
        setup_commands,
        completed_forwards: wire::FORWARDS,
        generated_tokens: outputs,
        page_permutation: stable.ok_or("long missing page permutation")?.to_vec(),
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
    };
    evidence::publish(&mut observation)?;
    Ok(observation)
}

#[cfg(test)]
mod tests;
