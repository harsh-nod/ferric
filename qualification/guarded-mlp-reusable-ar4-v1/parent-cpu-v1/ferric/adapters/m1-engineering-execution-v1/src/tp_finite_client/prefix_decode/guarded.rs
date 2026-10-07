//! Bounded, separately selected real-model guarded TF4/AR4 parent.
use super::*;
use crate::finite_guarded_mlp_decode_wire_v1 as gwire;

#[path = "guarded_capture.rs"]
mod capture;
#[path = "guarded_evidence.rs"]
mod evidence;

const REQUEST_SCHEMA: &str = "FerricFiniteGuardedMlpDecodeRequestV1";
const REUSE_REQUEST_SCHEMA: &str = "FerricFiniteGuardedMlpReusableAr4RequestV1";
const REUSE_OBSERVATION_SCHEMA: &str = "FerricFiniteGuardedMlpReusableAr4ObservationV1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardedConfig {
    pub schema: String,
    pub decode: Config,
    pub projection_image: FilePin,
    pub guarded_image: FilePin,
}
impl GuardedConfig {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= 65536,
            "guarded parent request bound",
        )?;
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        self.decode.validate()?;
        require(
            (self.schema == REQUEST_SCHEMA
                || (self.schema == REUSE_REQUEST_SCHEMA
                    && self.decode.mode == wire::InputMode::Autoregressive))
                && serde_json::to_vec(self).map_err(|e| e.to_string())?.len() <= 16384,
            "guarded parent schema/request bound",
        )?;
        for pin in [&self.projection_image, &self.guarded_image] {
            require(
                pin.path.is_absolute()
                    && pin.bytes > 0
                    && pin.bytes <= gwire::MAX_IMAGE_BYTES as u64
                    && pin.sha256 != [0; 32],
                "guarded parent image bounds",
            )?;
        }
        require(
            self.guarded_image.sha256 == gwire::GUARDED_IMAGE
                && self.projection_image.sha256 != self.decode.images.residual.sha256,
            "guarded parent selected image identity",
        )?;
        Ok(())
    }
}
fn pinned_part(pin: &FilePin) -> Result<setup_wire::Part> {
    Ok(setup_wire::Part {
        bytes: u32::try_from(pin.bytes).map_err(|_| "guarded image extent")?,
        sha256: pin.sha256,
    })
}
fn bootstrap(decode: wire::Bootstrap, config: &GuardedConfig) -> Result<gwire::Bootstrap> {
    let value = gwire::Bootstrap {
        schema: if config.schema == REUSE_REQUEST_SCHEMA {
            gwire::REUSE_SCHEMA
        } else {
            gwire::SCHEMA
        }
        .into(),
        decode,
        projection_image: pinned_part(&config.projection_image)?,
        guarded_image: pinned_part(&config.guarded_image)?,
    };
    value
        .validate(
            config.decode.device_ids,
            config.decode.dispatch_timeout_ms,
            value.decode.scope.child_identity,
            config.decode.mode,
        )
        .map_err(|e| e.to_string())?;
    Ok(value)
}
#[derive(Serialize)]
pub struct GuardedObservation {
    pub schema: &'static str,
    pub request: GuardedConfig,
    pub child_pid: u32,
    pub registration_sha256: [u8; 32],
    pub source_program_sha256: [u8; 32],
    pub upload_manifest_sha256: [u8; 32],
    pub bootstrap: gwire::Bootstrap,
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
    pub close: gwire::Response,
    pub child_exit_zero: bool,
    pub process_group_absent: bool,
    pub native_closed: bool,
    pub gpu_execution: bool,
    pub numerical_acceptance: bool,
    pub full_model_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
    pub full_long_workload: bool,
    pub native_attempts: u32,
    pub retries: u32,
}
fn trajectory(b: &gwire::Bootstrap, inputs: &[u32], outputs: &[u32]) -> Result<()> {
    require(
        inputs.len() == 4 && outputs.len() == 4,
        "guarded four-token observation",
    )?;
    let mut previous = None;
    for (position, (&input, &output)) in inputs.iter().zip(outputs).enumerate() {
        require(
            output < VOCABULARY
                && input
                    == b.input(position as u32, previous)
                        .map_err(|e| e.to_string())?,
            "guarded own-output trajectory",
        )?;
        previous = Some(output);
    }
    Ok(())
}
fn validate_observation(observation: &GuardedObservation) -> Result<()> {
    let config = &observation.request;
    config.validate()?;
    let b = &observation.bootstrap;
    b.validate(
        config.decode.device_ids,
        config.decode.dispatch_timeout_ms,
        observation.child_pid,
        config.decode.mode,
    )
    .map_err(|e| e.to_string())?;
    let (expected_observation, expected_bootstrap) = if config.schema == REUSE_REQUEST_SCHEMA {
        (REUSE_OBSERVATION_SCHEMA, gwire::REUSE_SCHEMA)
    } else {
        ("FerricFiniteGuardedMlpDecodeObservationV1", gwire::SCHEMA)
    };
    require(
        observation.schema == expected_observation
            && b.schema == expected_bootstrap
            && b.decode.scope.bundle_id == config.decode.expected_bundle_id
            && b.decode.scope.model_id == config.decode.expected_model_id
            && b.decode.scope.session == config.decode.session
            && b.projection_image == pinned_part(&config.projection_image)?
            && b.guarded_image == pinned_part(&config.guarded_image)?
            && b.decode.prefix_image == pinned_part(&config.decode.prefix_image)?
            && b.decode.tiles_image == pinned_part(&config.decode.tiles_image)?
            && b.decode.begin.prefix_image == pinned_part(&config.decode.images.prefix)?
            && b.decode.begin.mlp_image == pinned_part(&config.decode.images.mlp)?
            && b.decode.begin.residual_image == pinned_part(&config.decode.images.residual)?
            && b.decode.begin.tail_image == Some(pinned_part(&config.decode.images.tail)?)
            && observation.registration_sha256 == b.decode.registration
            && observation.source_program_sha256 == b.decode.begin.source_program.sha256
            && observation.upload_manifest_sha256 == b.decode.begin.uploads.sha256
            && observation.profile_sha256 == b.sha256().map_err(|e| e.to_string())?
            && observation.setup_commands > 0
            && observation.gpu_execution,
        "guarded parent actual source/profile/image joins",
    )?;
    let seed = match config.decode.mode {
        wire::InputMode::TeacherForced => INPUT_TOKENS.as_slice(),
        wire::InputMode::Autoregressive => &INPUT_TOKENS[..1],
    };
    require(
        b.decode.input_tokens.as_slice() == seed && observation.page_permutation.len() == 144,
        "guarded parent seed/pages",
    )?;
    let mut pages = observation.page_permutation.clone();
    pages.sort_unstable();
    require(
        pages == (0..144).collect::<Vec<_>>(),
        "guarded parent page permutation",
    )?;
    if config.schema == REUSE_REQUEST_SCHEMA {
        let raw = observation
            .files
            .child_stderr
            .read(gwire::REUSE_CENSUS_BYTES as u64, true)?;
        gwire::ReuseArenaCensus::decode(&raw, b).map_err(|e| e.to_string())?;
    }
    trajectory(
        b,
        &observation.input_tokens,
        &observation.observed_output_tokens,
    )
}
fn check_frontiers(previous: &mut [(u64, u64); 2], control: &gwire::Control) -> Result<()> {
    for rank in 0..2 {
        let first = control.layers[0].observed_queue_frontiers[rank];
        require(
            first.0 > previous[rank].0 && first.1 >= previous[rank].1,
            "guarded parent cross-forward queue ordering",
        )?;
    }
    *previous = control.layers[35].observed_queue_frontiers;
    Ok(())
}
fn response_identity(request: &gwire::Request, response: &gwire::Response) -> Result<()> {
    request.validate().map_err(|e| e.to_string())?;
    require(
        response.schema == gwire::RESPONSE_SCHEMA
            && response.protocol == request.protocol
            && response.id == request.id
            && response.device_ids == request.device_ids
            && response.session == request.session
            && response.registration == request.registration
            && response.profile_sha256 == request.profile_sha256
            && response.gpu_execution
            && !response.full_model_acceptance
            && !response.numerical_acceptance
            && !response.performance_claim
            && !response.production_authority,
        "guarded decode response identity or claim boundary",
    )
}

fn validate_completion<'a>(
    request: &gwire::Request,
    response: &'a gwire::Response,
    control: &gwire::Control,
    payload: &[u8],
    chain: &mut gwire::Chain,
) -> Result<&'a gwire::Completion> {
    control.validate(request.id).map_err(|e| e.to_string())?;
    validate_completion_bytes(request, response, &control.encode(), payload, chain)
}
fn validate_completion_bytes<'a>(
    request: &gwire::Request,
    response: &'a gwire::Response,
    control: &[u8],
    payload: &[u8],
    chain: &mut gwire::Chain,
) -> Result<&'a gwire::Completion> {
    response_identity(request, response)?;
    let gwire::Command::Forward {
        generation, token, ..
    } = &request.command
    else {
        return Err("guarded decode completion for non-forward".into());
    };
    let gwire::Event::Completed(c) = &response.event else {
        return Err("guarded decode expected Completed".into());
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
        "guarded decode completion token, state, payload or position",
    )?;
    require(
        c.capture == old::Payload::from_bytes(payload).map_err(|e| e.to_string())?
            && c.capture.total == c.observation,
        "guarded decode full capture binding",
    )?;
    require(
        payload
            .chunks_exact(2)
            .all(|b| u16::from_le_bytes([b[0], b[1]]) & 0x7f80 != 0x7f80),
        "guarded decode nonfinite captured BF16",
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
        "guarded decode captured logits/lowest-index argmax",
    )?;
    require(
        chain.advance(c) == c.chain,
        "guarded decode transcript chain mismatch",
    )?;
    Ok(c)
}

fn validate_close(
    request: &gwire::Request,
    response: &gwire::Response,
    control: Option<&gwire::Control>,
    payload: &[u8],
    chain: [u8; 32],
) -> Result<()> {
    response_identity(request, response)?;
    require(
        matches!(request.command, gwire::Command::Close)
            && response.native_closed
            && control.is_none()
            && payload.is_empty()
            && response.event
                == (gwire::Event::Closed {
                    completed_forwards: gwire::FORWARDS,
                    transcript_sha256: chain,
                }),
        "guarded decode close count, transcript or lifecycle",
    )
}

/// One child, exactly four forwards. No retry, alternate kernel, parity or acceptance.
pub fn run_guarded(
    config: GuardedConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<GuardedObservation> {
    run_selected(
        config,
        allow_unauthenticated_machine_code,
        false,
        None,
        false,
    )
}

/// Same four-forward protocol, with bounded layer-zero diagnostics in child stderr.
pub fn run_guarded_capture(
    config: GuardedConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<GuardedObservation> {
    run_selected(
        config,
        allow_unauthenticated_machine_code,
        true,
        None,
        false,
    )
}

/// Unchanged policy and wire, with inclusive host-counter diagnostics after Close.
pub fn run_guarded_host(
    config: GuardedConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<GuardedObservation> {
    run_selected(
        config,
        allow_unauthenticated_machine_code,
        false,
        Some(crate::guarded_mlp_host_observation_v1::Policy::DefaultFull),
        false,
    )
}

/// Explicit fresh shared-full fences, with unchanged owned wire and Close gates.
pub fn run_guarded_host_shared(
    config: GuardedConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<GuardedObservation> {
    run_selected(
        config,
        allow_unauthenticated_machine_code,
        false,
        Some(crate::guarded_mlp_host_observation_v1::Policy::SharedFull),
        false,
    )
}

/// Separate AR4 proof of reusable paired arena ownership; no long-workload claim.
pub fn run_guarded_reuse(
    config: GuardedConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<GuardedObservation> {
    run_selected(
        config,
        allow_unauthenticated_machine_code,
        false,
        None,
        true,
    )
}

fn run_selected(
    config: GuardedConfig,
    allow_unauthenticated_machine_code: bool,
    capture_layer_zero: bool,
    host_policy: Option<crate::guarded_mlp_host_observation_v1::Policy>,
    reusable: bool,
) -> Result<GuardedObservation> {
    require(
        usize::from(capture_layer_zero)
            + usize::from(host_policy.is_some())
            + usize::from(reusable)
            <= 1,
        "guarded diagnostic modes are exclusive",
    )?;
    require(
        allow_unauthenticated_machine_code,
        "guarded explicit machine-code opt-in required",
    )?;
    config.validate()?;
    require(
        (config.schema == REUSE_REQUEST_SCHEMA) == reusable,
        "guarded request/invocation arena policy mismatch",
    )?;
    let decode = &config.decode;
    decode.worker.read(512 << 20, false)?;
    let images = decode.images.read()?;
    let tiles_image = decode.read_tiles_image()?;
    let prefix_image = decode.read_prefix_image()?;
    let projection_image = config
        .projection_image
        .read(gwire::MAX_IMAGE_BYTES as u64, true)?;
    let guarded_image = config
        .guarded_image
        .read(gwire::MAX_IMAGE_BYTES as u64, true)?;
    let (prompt_text, prompt) = decode.prompt.read()?;
    let model = EngineeringQwenModelV1::open(&decode.source)?;
    require(
        *model.bundle_id().as_bytes() == decode.expected_bundle_id
            && *model.config().model_id.as_bytes() == decode.expected_model_id,
        "guarded authenticated model/bundle differs",
    )?;
    require(
        model.encode_with_limits(
            &prompt_text,
            ferric_build::TokenizerExecutionLimits::long_context(),
        )? == prompt,
        "guarded authenticated tokenizer/raw prompt differs",
    )?;
    let mut evidence = evidence::Evidence::create(&decode.evidence_directory)?;
    let scope = EngineeringTpPoolScopeV1 {
        model: decode.expected_model_id,
        session: decode.session,
    };
    let limits =
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).map_err(|e| format!("{e:?}"))?;
    let mut pool = EngineeringTpPagedPoolV1::new(scope, limits).map_err(|e| format!("{e:?}"))?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(decode.child_deadline_ms))
        .ok_or("guarded deadline overflow")?;
    let mut command = Command::new(&decode.worker.path);
    command
        .args([
            if reusable {
                gwire::REUSE_WORKER_FLAG
            } else if let Some(policy) = host_policy {
                policy.worker_flag()
            } else {
                capture::worker_flag(capture_layer_zero)
            },
            "--allow-unauthenticated-machine-code",
            "--devices",
        ])
        .arg(format!("{},{}", decode.device_ids[0], decode.device_ids[1]))
        .arg("--timeout-ms")
        .arg(decode.dispatch_timeout_ms.to_string())
        .arg("--mode")
        .arg(match decode.mode {
            wire::InputMode::TeacherForced => "teacher-forced",
            wire::InputMode::Autoregressive => "autoregressive",
        })
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C");
    let mut child = process::OwnedChild::spawn_command(command, deadline)?;
    let pid = child.id();
    eprintln!("finite guarded owned child pid={pid} pgid={pid}; no native setup acknowledged");
    decode.worker.read(512 << 20, false)?;
    let mut sent = gwire::FrameBudget::new();
    let mut received = gwire::FrameBudget::new();
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
        let registration_raw = serde_json::to_vec(&registration).map_err(|e| e.to_string())?;
        let manifest_raw = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
        let registration_sha = hash(&registration_raw);
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
        let recipe = wire::Bootstrap {
            protocol: wire::PROTOCOL,
            profile: wire::Profile::Prefix284Mlp548FourForwardV1,
            device_ids: decode.device_ids,
            scope: setup::scope(&registration),
            registration: registration_sha,
            begin,
            timeout_ms: decode.dispatch_timeout_ms,
            mode: decode.mode,
            input_tokens: match decode.mode {
                wire::InputMode::TeacherForced => INPUT_TOKENS.to_vec(),
                wire::InputMode::Autoregressive => vec![INPUT_TOKENS[0]],
            },
            tiles_image: old::part(&tiles_image),
            prefix_image: old::part(&prefix_image),
        };
        let selected = bootstrap(recipe, &config)?;
        selected
            .validate(
                decode.device_ids,
                decode.dispatch_timeout_ms,
                pid,
                decode.mode,
            )
            .map_err(|e| e.to_string())?;
        let profile = selected.sha256().map_err(|e| e.to_string())?;
        child.check_deadline()?;
        gwire::write_bootstrap(
            child.input.as_mut().ok_or("guarded stdin absent")?,
            &mut sent,
            &selected,
            [
                &tiles_image,
                &prefix_image,
                &projection_image,
                &guarded_image,
            ],
        )
        .map_err(|e| e.to_string())?;
        gwire::account_begin(&mut sent, &selected).map_err(|e| e.to_string())?;
        let mut stream = setup::Stream::begin(
            &mut child,
            decode.device_ids,
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
                require(offset == total, "guarded head contiguous offset")?;
                stream.write_tail(offset, bytes)?;
                total += bytes.len();
                digest.update(bytes);
                Ok(())
            },
        )?;
        let actual: [u8; 32] = digest.finalize().into();
        require(
            total == head.bytes() && actual == head.sha256(),
            "guarded head recipe changed",
        )?;
        Ok((
            registration_sha,
            hash(&program),
            hash(&manifest_raw),
            selected,
            profile,
            stream.seal()?,
        ))
    })?;
    let sequence = pool
        .open_sequence(scope, &[prompt[0]], 0)
        .map_err(|e| format!("{e:?}"))?;
    require(
        sequence.hit_tokens() == 0 && sequence.physical_pages().is_empty(),
        "guarded fresh history required",
    )?;
    let mut stable = None;
    let mut chain = gwire::Chain::new(registration_sha256, profile_sha256);
    let mut outputs = Vec::with_capacity(4);
    let mut inputs = Vec::with_capacity(4);
    let mut frontiers = [(0, 0); 2];
    for position in 0..gwire::FORWARDS {
        let token = bootstrap
            .input(position, outputs.last().copied())
            .map_err(|e| e.to_string())?;
        require(
            pool.committed_position(sequence.sequence())
                .map_err(|e| format!("{e:?}"))?
                == position,
            "guarded host/native committed position",
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
            let request = gwire::Request {
                protocol: gwire::PROTOCOL,
                id: u64::from(position) + 1,
                device_ids: decode.device_ids,
                session: decode.session,
                registration: registration_sha256,
                profile_sha256,
                command: gwire::Command::Forward {
                    generation: u64::from(position) + 1,
                    token,
                    cache_metadata: metadata.cache_metadata().to_vec(),
                    rotary_bits: metadata.rotary().iter().map(|v| v.to_bits()).collect(),
                },
            };
            child.check_deadline()?;
            gwire::write_request(
                child.input.as_mut().ok_or("guarded forward stdin absent")?,
                &mut sent,
                &request,
            )
            .map_err(|e| e.to_string())?;
            let (response, control, payload) = gwire::read_response(
                child.output.as_mut().ok_or("guarded stdout absent")?,
                &mut received,
            )
            .map_err(|e| e.to_string())?
            .ok_or("guarded EOF before completion")?;
            let control = control.ok_or("guarded control absent")?;
            let completion =
                validate_completion(&request, &response, &control, &payload, &mut chain)?;
            check_frontiers(&mut frontiers, &control)?;
            child.check_deadline()?;
            evidence.append(&request, &response, &control, &payload)?;
            Ok(completion.output_token)
        })();
        let output = commit_completed(&mut pool, &batch, result)?;
        inputs.push(token);
        outputs.push(output);
        eprintln!("finite guarded completed position={position}");
    }
    trajectory(&bootstrap, &inputs, &outputs)?;
    let close_request = gwire::Request {
        protocol: gwire::PROTOCOL,
        id: 5,
        device_ids: decode.device_ids,
        session: decode.session,
        registration: registration_sha256,
        profile_sha256,
        command: gwire::Command::Close,
    };
    child.check_deadline()?;
    gwire::write_request(
        child.input.as_mut().ok_or("guarded Close stdin absent")?,
        &mut sent,
        &close_request,
    )
    .map_err(|e| e.to_string())?;
    let (close, control, payload) = gwire::read_response(
        child.output.as_mut().ok_or("guarded Close stdout absent")?,
        &mut received,
    )
    .map_err(|e| e.to_string())?
    .ok_or("guarded EOF before Close")?;
    validate_close(
        &close_request,
        &close,
        control.as_ref(),
        &payload,
        chain.digest(),
    )?;
    let stderr = child.finish()?;
    decode.worker.read(512 << 20, false)?;
    for pin in [
        &decode.images.prefix,
        &decode.images.mlp,
        &decode.images.residual,
        &decode.images.tail,
    ] {
        pin.read(64 << 20, false)?;
    }
    decode.prompt.recheck()?;
    for pin in [
        &decode.tiles_image,
        &decode.prefix_image,
        &config.projection_image,
        &config.guarded_image,
    ] {
        pin.read(gwire::MAX_IMAGE_BYTES as u64, false)?;
    }
    let files = evidence.finish(&stderr)?;
    if capture_layer_zero {
        let first = files
            .frames
            .first()
            .ok_or("guarded capture first forward absent")?;
        let request =
            serde_json::from_slice::<gwire::Request>(&first.request.read(16 << 10, true)?)
                .map_err(|e| e.to_string())?;
        capture::validate(
            &stderr,
            &bootstrap,
            &request,
            &first
                .observation
                .read(old::OBSERVATION_BYTES as u64, true)?,
        )?;
    }
    if let Some(policy) = host_policy {
        use crate::guarded_mlp_host_observation_v1::{Policy, Report};
        let report = match policy {
            Policy::DefaultFull => Report::decode(&stderr),
            Policy::SharedFull => Report::decode_shared(&stderr),
        }
        .map_err(|e| e.to_string())?;
        let completions = files
            .frames
            .iter()
            .map(|frame| match &frame.response.event {
                gwire::Event::Completed(done) => Ok(done.clone()),
                _ => Err("guarded host expected actual completed frame".into()),
            })
            .collect::<Result<Vec<_>>>()?;
        match policy {
            Policy::DefaultFull => {
                report.validate_expected(&bootstrap, decode.worker.sha256, pid, &completions)
            }
            Policy::SharedFull => {
                report.validate_shared_expected(&bootstrap, decode.worker.sha256, pid, &completions)
            }
        }
        .map_err(|e| e.to_string())?;
    }
    if reusable {
        gwire::ReuseArenaCensus::decode(&stderr, &bootstrap).map_err(|e| e.to_string())?;
    }
    let mut observation = GuardedObservation {
        schema: if reusable {
            REUSE_OBSERVATION_SCHEMA
        } else {
            "FerricFiniteGuardedMlpDecodeObservationV1"
        },
        request: config,
        child_pid: pid,
        registration_sha256,
        source_program_sha256: program_sha256,
        upload_manifest_sha256: manifest_sha256,
        bootstrap,
        profile_sha256,
        setup_commands,
        completed_forwards: 4,
        input_tokens: inputs,
        observed_output_tokens: outputs,
        page_permutation: stable.ok_or("guarded page permutation absent")?.to_vec(),
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
        full_model_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_long_workload: false,
        native_attempts: 1,
        retries: 0,
    };
    evidence::publish(&mut observation)?;
    Ok(observation)
}
#[cfg(test)]
#[path = "guarded_tests.rs"]
mod tests;
