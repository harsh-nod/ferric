//! Closed forty-position native readiness client, not a generated-token workload.
use super::*;
use crate::finite_guarded_mlp_decode_wire_v1 as four;
use crate::finite_guarded_mlp_long_wire_v2 as long;
use crate::finite_guarded_mlp_readiness_wire_v1 as ready;
use std::io::Write;

#[path = "readiness_causal.rs"]
mod causal;
#[path = "readiness_host_timing.rs"]
mod host_timing;
#[path = "readiness_evidence.rs"]
mod retained;
#[path = "readiness_shared_full.rs"]
mod shared_full;
pub use host_timing::{TimedObservation, TimingStatus, run_position5_host_timing};
pub use shared_full::{SharedFullObservation, run_position5_shared_full};
pub const REQUEST_SCHEMA: &str = "FerricGuardedMlpReadiness40RequestV1";
pub const POSITION5_REQUEST_SCHEMA: &str = "FerricGuardedMlpReadiness40Position5RequestV1";
pub const CAUSAL_REQUEST_SCHEMA: &str = "FerricGuardedMlpReadiness40CausalLayerZeroRequestV1";
const CAUSAL_OBSERVATION_SCHEMA: &str = "FerricGuardedMlpReadiness40CausalLayerZeroObservationV1";
fn observation_schema(profile: long::Profile) -> Result<&'static str> {
    match profile {
        long::Profile::Readiness40 => Ok("FerricGuardedMlpReadiness40ObservationV1"),
        long::Profile::Readiness40Position5 => {
            Ok("FerricGuardedMlpReadiness40Position5ObservationV1")
        }
        long::Profile::Full2303 => Err("readiness refuses Full2303".into()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessConfig {
    pub schema: String,
    pub base: super::Config,
    pub tiles_image: FilePin,
    pub prefix_image: FilePin,
    pub projection_image: FilePin,
    pub guarded_image: FilePin,
}
impl ReadinessConfig {
    pub fn parse(raw: &[u8]) -> Result<Self> {
        require(
            !raw.is_empty() && raw.len() <= 65_536,
            "readiness request bound",
        )?;
        let value: Self = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    fn profile(&self) -> Result<long::Profile> {
        match self.schema.as_str() {
            REQUEST_SCHEMA => Ok(long::Profile::Readiness40),
            POSITION5_REQUEST_SCHEMA | CAUSAL_REQUEST_SCHEMA => {
                Ok(long::Profile::Readiness40Position5)
            }
            _ => Err("readiness closed request profile".into()),
        }
    }
    fn validate(&self) -> Result<()> {
        self.base.validate()?;
        require(
            observation_schema(self.profile()?).is_ok()
                && self.guarded_image.sha256 == four::GUARDED_IMAGE
                && self.projection_image.sha256 != self.base.images.residual.sha256
                && serde_json::to_vec(self).map_err(|e| e.to_string())?.len() <= 24 << 10,
            "readiness closed request/profile/image",
        )?;
        for pin in [
            &self.tiles_image,
            &self.prefix_image,
            &self.projection_image,
            &self.guarded_image,
        ] {
            require(
                pin.path.is_absolute()
                    && pin.bytes != 0
                    && pin.bytes <= ready::MAX_IMAGE_BYTES as u64
                    && pin.sha256 != [0; 32],
                "readiness image pin bound",
            )?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct Observation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_layer_zero: Option<causal::Summary>,
    pub schema: &'static str,
    pub request: ReadinessConfig,
    pub child_pid: u32,
    pub bootstrap: ready::Bootstrap,
    pub profile_sha256: [u8; 32],
    pub registration_sha256: [u8; 32],
    pub source_program_sha256: [u8; 32],
    pub upload_manifest_sha256: [u8; 32],
    pub setup_commands: u64,
    pub completed_forwards: u32,
    pub prompt_positions_executed: u32,
    pub generated_tokens: Vec<u32>,
    pub page_permutation: Vec<u32>,
    pub transcript_sha256: [u8; 32],
    pub request_stream_bytes: usize,
    pub response_stream_bytes: usize,
    pub files: retained::Files,
    pub close: ready::Closed,
    pub child_exit_zero: bool,
    pub process_group_absent: bool,
    pub native_closed: bool,
    pub gpu_execution: bool,
    pub full_long_workload: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
}

fn validate_summary(value: &Observation) -> Result<()> {
    value.request.validate()?;
    value
        .bootstrap
        .validate(
            value.request.base.device_ids,
            value.request.base.dispatch_timeout_ms,
            value.child_pid,
        )
        .map_err(|e| e.to_string())?;
    let b = &value.bootstrap.sequence;
    let profile = b.sha256().map_err(|e| e.to_string())?;
    require(
        value.schema
            == if value.request.schema == CAUSAL_REQUEST_SCHEMA {
                CAUSAL_OBSERVATION_SCHEMA
            } else {
                observation_schema(value.request.profile()?)?
            }
            && b.profile == value.request.profile()?
            && b.scope.session == value.request.base.session
            && b.scope.model_id == value.request.base.expected_model_id
            && b.scope.bundle_id == value.request.base.expected_bundle_id
            && value.bootstrap.child_deadline_ms == value.request.base.child_deadline_ms
            && u64::from(b.mlp_image.bytes) == value.request.tiles_image.bytes
            && b.mlp_image.sha256 == value.request.tiles_image.sha256
            && u64::from(b.prefix_image.bytes) == value.request.prefix_image.bytes
            && b.prefix_image.sha256 == value.request.prefix_image.sha256
            && u64::from(b.projection_image.bytes) == value.request.projection_image.bytes
            && b.projection_image.sha256 == value.request.projection_image.sha256
            && u64::from(b.guarded_image.bytes) == value.request.guarded_image.bytes
            && b.guarded_image.sha256 == value.request.guarded_image.sha256
            && value.profile_sha256 == profile
            && value.registration_sha256 == b.registration
            && value.source_program_sha256 == b.begin.source_program.sha256
            && value.upload_manifest_sha256 == b.begin.uploads.sha256
            && value.completed_forwards == 40
            && value.prompt_positions_executed == 40
            && value.generated_tokens.is_empty()
            && value.page_permutation.len() == 144
            && value.files.rows == 40
            && value
                .files
                .captures
                .iter()
                .map(|c| c.position)
                .eq(b.profile.capture_positions())
            && value.child_exit_zero
            && value.process_group_absent
            && value.native_closed
            && value.gpu_execution
            && !value.full_long_workload
            && !value.numerical_acceptance
            && !value.performance_claim
            && !value.production_authority
            && value.request_stream_bytes <= long::STREAM_BYTES
            && value.response_stream_bytes <= long::STREAM_BYTES,
        "readiness complete observation boundary",
    )?;
    require(
        value
            .page_permutation
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            == (0..144).collect(),
        "readiness complete physical page permutation",
    )?;
    if value.request.schema == CAUSAL_REQUEST_SCHEMA {
        let checked = causal::validate_file(
            &value.bootstrap,
            value.transcript_sha256,
            &value.page_permutation,
            &value.files,
        )?;
        require(
            value.causal_layer_zero.as_ref() == Some(&checked),
            "causal checked sidecar summary",
        )?;
    } else {
        require(
            value.causal_layer_zero.is_none(),
            "ordinary readiness cannot claim causal diagnostics",
        )?;
    }
    value
        .close
        .validate_for(b.profile, &value.close.request, value.transcript_sha256)
        .map_err(|e| e.to_string())?;
    require(
        value.close.request.device_ids == b.device_ids
            && value.close.request.session == b.scope.session
            && value.close.request.registration == b.registration
            && value.close.request.profile_sha256 == profile,
        "readiness final Close profile join",
    )
}

/// One authenticated owned child; no fallback, retry or full-workload admission.
pub fn run(
    config: ReadinessConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<Observation> {
    run_for(
        config,
        allow_unauthenticated_machine_code,
        long::Profile::Readiness40,
        false,
    )
}
/// Explicit four-capture position-5 diagnostic; unchanged forty-forward execution.
pub fn run_position5(
    config: ReadinessConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<Observation> {
    run_for(
        config,
        allow_unauthenticated_machine_code,
        long::Profile::Readiness40Position5,
        false,
    )
}
/// Separate operation-boundary diagnostic on the genuine first six prefixes.
pub fn run_causal(
    config: ReadinessConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<Observation> {
    run_for(
        config,
        allow_unauthenticated_machine_code,
        long::Profile::Readiness40Position5,
        true,
    )
}
fn run_for(
    config: ReadinessConfig,
    allow_unauthenticated_machine_code: bool,
    selected_profile: long::Profile,
    causal_selected: bool,
) -> Result<Observation> {
    run_inner(
        config,
        allow_unauthenticated_machine_code,
        selected_profile,
        causal_selected,
        &mut None,
    )
}
fn run_inner(
    config: ReadinessConfig,
    allow_unauthenticated_machine_code: bool,
    selected_profile: long::Profile,
    causal_selected: bool,
    timing: &mut Option<host_timing::Recorder>,
) -> Result<Observation> {
    run_inner_policy(
        config,
        allow_unauthenticated_machine_code,
        selected_profile,
        causal_selected,
        timing,
        false,
    )
}
fn run_inner_policy(
    config: ReadinessConfig,
    allow_unauthenticated_machine_code: bool,
    selected_profile: long::Profile,
    causal_selected: bool,
    timing: &mut Option<host_timing::Recorder>,
    shared_selected: bool,
) -> Result<Observation> {
    if shared_selected {
        shared_full::admit_entry(&config.schema, allow_unauthenticated_machine_code)?;
        require(
            selected_profile == long::Profile::Readiness40Position5
                && !causal_selected
                && timing.is_none(),
            "shared full cannot combine causal, timing or another profile",
        )?;
    }
    require(
        allow_unauthenticated_machine_code,
        "readiness explicit machine-code opt-in required",
    )?;
    config.validate()?;
    require(
        config.profile()? == selected_profile
            && (config.schema == CAUSAL_REQUEST_SCHEMA) == causal_selected,
        "readiness explicit parent entry/profile mismatch",
    )?;
    let (bootstrap_schema, _, ordinary_worker_flag) =
        ready::schemas(selected_profile).map_err(|e| e.to_string())?;
    let worker_flag = if shared_selected {
        crate::finite_guarded_mlp_readiness_shared_v1::WORKER_FLAG
    } else if causal_selected {
        "--engineering-native-guarded-mlp-readiness40-causal-layer0-v1"
    } else {
        ordinary_worker_flag
    };
    let base = &config.base;
    base.worker.read(512 << 20, false)?;
    let images = base.images.read()?;
    let mlp = config
        .tiles_image
        .read(ready::MAX_IMAGE_BYTES as u64, true)?;
    let prefix = config
        .prefix_image
        .read(ready::MAX_IMAGE_BYTES as u64, true)?;
    let projection = config
        .projection_image
        .read(ready::MAX_IMAGE_BYTES as u64, true)?;
    let guarded = config
        .guarded_image
        .read(ready::MAX_IMAGE_BYTES as u64, true)?;
    let (prompt_text, prompt) = base.prompt.read()?;
    let model = EngineeringQwenModelV1::open(&base.source)?;
    require(
        *model.bundle_id().as_bytes() == base.expected_bundle_id
            && *model.config().model_id.as_bytes() == base.expected_model_id,
        "readiness authenticated model/bundle",
    )?;
    require(
        model.encode_with_limits(
            &prompt_text,
            ferric_build::TokenizerExecutionLimits::long_context(),
        )? == prompt,
        "readiness authenticated raw prompt/tokenizer",
    )?;
    let mut evidence = retained::Evidence::create_for(&base.evidence_directory, selected_profile)?;
    let scope = EngineeringTpPoolScopeV1 {
        model: base.expected_model_id,
        session: base.session,
    };
    let limits =
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).map_err(|e| format!("{e:?}"))?;
    let mut pool = EngineeringTpPagedPoolV1::new(scope, limits).map_err(|e| format!("{e:?}"))?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(base.child_deadline_ms))
        .ok_or("readiness deadline overflow")?;
    let mut command = Command::new(&base.worker.path);
    command
        .args([
            worker_flag,
            "--allow-unauthenticated-machine-code",
            "--devices",
        ])
        .arg(format!("{},{}", base.device_ids[0], base.device_ids[1]))
        .arg("--timeout-ms")
        .arg(base.dispatch_timeout_ms.to_string())
        .args(["--mode", "autoregressive"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C");
    host_timing::mark(timing, host_timing::Event::Spawn)?;
    let mut child = process::OwnedChild::spawn_command(command, deadline)?;
    let pid = child.id();
    eprintln!("finite guarded readiness owned child pid={pid} pgid={pid}; setup pending");
    base.worker.read(512 << 20, false)?;
    let mut sent = long::FrameBudget::new();
    let mut received = long::FrameBudget::new();
    let recorder = EngineeringTp2FiniteSourceRecorderV1::new(&model, &pool, pid)?;
    child.check_deadline()?;
    let (bootstrap, setup_commands) = recorder.with_plan(|composition, source, uploads| {
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
        let b = ready::Bootstrap {
            schema: bootstrap_schema.into(),
            child_deadline_ms: base.child_deadline_ms,
            sequence: long::Bootstrap {
                protocol: 1,
                profile: selected_profile,
                device_ids: base.device_ids,
                scope: setup::scope(&registration),
                registration: hash(&registration_raw),
                begin,
                timeout_ms: base.dispatch_timeout_ms,
                prompt_tokens: prompt.clone(),
                prefix_image: old::part(&prefix),
                mlp_image: old::part(&mlp),
                projection_image: old::part(&projection),
                guarded_image: old::part(&guarded),
            },
        };
        b.validate(base.device_ids, base.dispatch_timeout_ms, pid)
            .map_err(|e| e.to_string())?;
        child.check_deadline()?;
        ready::write_bootstrap(
            child.input.as_mut().ok_or("readiness stdin absent")?,
            &mut sent,
            &b,
            [&mlp, &prefix, &projection, &guarded],
        )
        .map_err(|e| e.to_string())?;
        four::account_begin(&mut sent, &b.setup().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let mut stream = setup::Stream::begin(
            &mut child,
            base.device_ids,
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
                require(offset == total, "readiness head contiguous offset")?;
                stream.write_tail(offset, bytes)?;
                total += bytes.len();
                digest.update(bytes);
                Ok(())
            },
        )?;
        let actual: [u8; 32] = digest.finalize().into();
        require(
            total == head.bytes() && actual == head.sha256(),
            "readiness head recipe changed",
        )?;
        Ok((b, stream.seal()?))
    })?;
    host_timing::mark(timing, host_timing::Event::Sealed)?;
    let b = &bootstrap.sequence;
    let profile = b.sha256().map_err(|e| e.to_string())?;
    let sequence = pool
        .open_sequence(scope, &[prompt[0]], 0)
        .map_err(|e| format!("{e:?}"))?;
    require(
        sequence.hit_tokens() == 0 && sequence.physical_pages().is_empty(),
        "readiness fresh history",
    )?;
    let mut transcript = long::Transcript::new(b.clone()).map_err(|e| e.to_string())?;
    let mut stable = None;
    for position in 0..long::READINESS_FORWARDS {
        let token = prompt[position as usize];
        require(
            pool.committed_position(sequence.sequence())
                .map_err(|e| format!("{e:?}"))?
                == position,
            "readiness host/native committed position",
        )?;
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence: sequence.sequence(),
                token,
                position,
            }])
            .map_err(|e| format!("{e:?}"))?;
        let result: Result<()> = (|| {
            pool.begin_submission(&batch)
                .map_err(|e| format!("{e:?}"))?;
            let metadata = super::metadata(
                &batch,
                b.begin.source_program.sha256,
                model.config().rope_theta,
            )?;
            super::stable_pages(&mut stable, metadata.cache_metadata())?;
            let request = long::Request {
                protocol: 1,
                id: u64::from(position) + 1,
                device_ids: base.device_ids,
                session: base.session,
                registration: b.registration,
                profile_sha256: profile,
                command: long::Command::Forward {
                    generation: u64::from(position) + 1,
                    token,
                    cache_metadata: metadata.cache_metadata().to_vec(),
                    rotary_bits: metadata.rotary().iter().map(|v| v.to_bits()).collect(),
                },
            };
            transcript.begin(&request).map_err(|e| e.to_string())?;
            child.check_deadline()?;
            long::write_record(
                child
                    .input
                    .as_mut()
                    .ok_or("readiness forward stdin absent")?,
                &mut sent,
                &request,
            )
            .map_err(|e| e.to_string())?;
            child
                .input
                .as_mut()
                .ok_or("readiness stdin absent")?
                .flush()
                .map_err(|e| e.to_string())?;
            host_timing::mark(timing, host_timing::Event::Flushed(position))?;
            let (frame, capture) = long::read_frame(
                child.output.as_mut().ok_or("readiness stdout absent")?,
                &mut received,
                selected_profile,
            )
            .map_err(|e| e.to_string())?
            .ok_or("readiness EOF before completion")?;
            host_timing::mark(timing, host_timing::Event::Read(position))?;
            require(
                frame.request == request,
                "readiness actual request/completion join",
            )?;
            child.check_deadline()?;
            evidence.append(&frame, capture.as_ref())?;
            transcript.advance(&frame).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if let Err(error) = result {
            pool.quarantine_batch(&batch)
                .map_err(|e| format!("readiness quarantine: {e:?}; original: {error}"))?;
            return Err(error);
        }
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .map_err(|e| format!("{e:?}"))?;
        host_timing::mark(timing, host_timing::Event::Committed(position))?;
        eprintln!("finite guarded readiness completed position={position} generated=0");
    }
    require(
        transcript.completed() == 40 && transcript.output_tokens().is_empty(),
        "readiness no generated outputs",
    )?;
    let close_request = long::Request {
        protocol: 1,
        id: 41,
        device_ids: base.device_ids,
        session: base.session,
        registration: b.registration,
        profile_sha256: profile,
        command: long::Command::Close,
    };
    let digest = transcript.digest();
    transcript
        .close(&close_request, digest)
        .map_err(|e| e.to_string())?;
    child.check_deadline()?;
    long::write_record(
        child.input.as_mut().ok_or("readiness Close stdin absent")?,
        &mut sent,
        &close_request,
    )
    .map_err(|e| e.to_string())?;
    child
        .input
        .as_mut()
        .ok_or("readiness stdin absent")?
        .flush()
        .map_err(|e| e.to_string())?;
    let close = long::read_record::<ready::Closed>(
        child
            .output
            .as_mut()
            .ok_or("readiness Close stdout absent")?,
        &mut received,
    )
    .map_err(|e| e.to_string())?
    .ok_or("readiness EOF before Close")?;
    close
        .validate_for(selected_profile, &close_request, digest)
        .map_err(|e| e.to_string())?;
    let stderr = child.finish()?;
    host_timing::mark(timing, host_timing::Event::Retired)?;
    base.worker.read(512 << 20, false)?;
    base.prompt.recheck()?;
    for pin in [
        &base.images.prefix,
        &base.images.mlp,
        &base.images.residual,
        &base.images.tail,
        &config.tiles_image,
        &config.prefix_image,
        &config.projection_image,
        &config.guarded_image,
    ] {
        pin.read(64 << 20, false)?;
    }
    let files = evidence.finish(&stderr)?;
    let causal_layer_zero = if causal_selected {
        Some(causal::validate_file(
            &bootstrap,
            digest,
            &stable
                .as_ref()
                .ok_or("readiness page permutation missing")?[..],
            &files,
        )?)
    } else {
        None
    };
    let mut observation = Observation {
        causal_layer_zero,
        schema: if causal_selected {
            CAUSAL_OBSERVATION_SCHEMA
        } else {
            observation_schema(selected_profile)?
        },
        child_pid: pid,
        registration_sha256: b.registration,
        source_program_sha256: b.begin.source_program.sha256,
        upload_manifest_sha256: b.begin.uploads.sha256,
        profile_sha256: profile,
        setup_commands,
        completed_forwards: 40,
        prompt_positions_executed: 40,
        generated_tokens: Vec::new(),
        page_permutation: stable.ok_or("readiness page permutation missing")?.to_vec(),
        transcript_sha256: digest,
        request_stream_bytes: sent.used(),
        response_stream_bytes: received.used(),
        files,
        close,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        full_long_workload: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        bootstrap,
        request: config,
    };
    validate_summary(&observation)?;
    if shared_selected {
        shared_full::validate_file(&observation)?;
    }
    retained::publish(&mut observation)?;
    Ok(observation)
}

#[cfg(test)]
#[path = "readiness_tests.rs"]
mod tests;
