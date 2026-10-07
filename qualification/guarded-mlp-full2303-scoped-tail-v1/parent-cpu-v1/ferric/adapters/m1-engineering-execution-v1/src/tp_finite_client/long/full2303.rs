//! Explicit 2048-prompt/256-output guarded request; bounded source route only.
use super::*;
use crate::finite_guarded_mlp_decode_wire_v1 as four;
use crate::finite_guarded_mlp_full2303_wire_v1 as full;
use crate::finite_guarded_mlp_long_wire_v2 as long;
use std::io::Write;

#[path = "full2303_evidence.rs"]
mod retained;
#[path = "full2303_scoped.rs"]
mod scoped;
pub use scoped::{ScopedWarmObservation, run_scoped_warm};
#[path = "full2303_bank_scoped_census.rs"]
mod bank_scoped_census;
pub use bank_scoped_census::{BankScopedCensusObservation, run_bank_scoped_census};
#[path = "full2303_bank_scoped_census_tail.rs"]
mod bank_scoped_census_tail;
pub use bank_scoped_census_tail::{BankScopedCensusTailObservation, run_bank_scoped_census_tail};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativePolicy {
    Full,
    ScopedWarm,
    BankScopedCensus,
    BankScopedCensusTail,
}
pub const REQUEST_SCHEMA: &str = "FerricGuardedMlpFull2303RequestV1";
pub const OBSERVATION_SCHEMA: &str = "FerricGuardedMlpFull2303ObservationV1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Full2303Config {
    pub schema: String,
    pub base: super::Config,
    pub tiles_image: FilePin,
    pub prefix_image: FilePin,
    pub projection_image: FilePin,
    pub guarded_image: FilePin,
}
impl Full2303Config {
    pub fn parse(raw: &[u8]) -> Result<Self> {
        require(
            !raw.is_empty() && raw.len() <= 65_536,
            "full2303 request bound",
        )?;
        let value: Self = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        self.base.validate()?;
        require(
            self.schema == REQUEST_SCHEMA
                && self.base.child_deadline_ms <= full::MAX_DEADLINE_MS
                && self.guarded_image.sha256 == four::GUARDED_IMAGE
                && self.projection_image.sha256 != self.base.images.residual.sha256
                && serde_json::to_vec(self).map_err(|e| e.to_string())?.len() <= 24 << 10,
            "full2303 closed request/profile/image",
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
                    && pin.bytes <= full::MAX_IMAGE_BYTES as u64
                    && pin.sha256 != [0; 32],
                "full2303 image pin bound",
            )?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct Observation {
    pub schema: &'static str,
    pub request: Full2303Config,
    pub child_pid: u32,
    pub bootstrap: full::Bootstrap,
    pub profile_sha256: [u8; 32],
    pub registration_sha256: [u8; 32],
    pub source_program_sha256: [u8; 32],
    pub upload_manifest_sha256: [u8; 32],
    pub setup_commands: u64,
    pub completed_forwards: u32,
    pub prompt_positions_executed: u32,
    pub decode_positions_executed: u32,
    pub decoded_special_token_policy: &'static str,
    pub generated_tokens: Vec<u32>,
    pub page_permutation: Vec<u32>,
    pub transcript_sha256: [u8; 32],
    pub request_stream_bytes: usize,
    pub response_stream_bytes: usize,
    pub files: retained::Files,
    pub close: full::Closed,
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
        value.schema == OBSERVATION_SCHEMA
            && b.profile == long::Profile::Full2303
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
            && value.completed_forwards == long::FORWARDS
            && value.prompt_positions_executed == long::PROMPT_TOKENS as u32
            && value.decode_positions_executed == long::OUTPUT_TOKENS as u32 - 1
            && value.decoded_special_token_policy == "skip"
            && value.generated_tokens.len() == long::OUTPUT_TOKENS
            && value
                .generated_tokens
                .iter()
                .all(|token| *token < VOCABULARY)
            && value.page_permutation.len() == 144
            && value.files.rows == long::FORWARDS
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
            && value.full_long_workload
            && !value.numerical_acceptance
            && !value.performance_claim
            && !value.production_authority
            && value.request_stream_bytes <= long::STREAM_BYTES
            && value.response_stream_bytes <= long::STREAM_BYTES,
        "full2303 complete observation boundary",
    )?;
    require(
        value
            .page_permutation
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            == (0..144).collect(),
        "full2303 complete physical page permutation",
    )?;
    value
        .close
        .validate(
            &value.close.request,
            value.transcript_sha256,
            &value.generated_tokens,
        )
        .map_err(|e| e.to_string())?;
    require(
        value.close.request.device_ids == b.device_ids
            && value.close.request.session == b.scope.session
            && value.close.request.registration == b.registration
            && value.close.request.profile_sha256 == profile,
        "full2303 final Close profile join",
    )
}

/// One authenticated child and exactly one full request. This source abort cap
/// does not establish launch feasibility or numerical/performance acceptance.
pub fn run(
    config: Full2303Config,
    allow_unauthenticated_machine_code: bool,
) -> Result<Observation> {
    require(
        allow_unauthenticated_machine_code,
        "full2303 explicit machine-code opt-in required",
    )?;
    run_inner(config, NativePolicy::Full)
}

fn run_inner(config: Full2303Config, policy: NativePolicy) -> Result<Observation> {
    config.validate()?;
    let selected_profile = long::Profile::Full2303;
    let worker_flag = match policy {
        NativePolicy::Full => full::WORKER_FLAG,
        NativePolicy::ScopedWarm => crate::finite_guarded_mlp_full2303_scoped_v1::WORKER_FLAG,
        NativePolicy::BankScopedCensus => {
            crate::finite_guarded_mlp_full2303_bank_scoped_census_v1::WORKER_FLAG
        }
        NativePolicy::BankScopedCensusTail => {
            crate::finite_guarded_mlp_full2303_bank_scoped_census_tail_v1::WORKER_FLAG
        }
    };
    let base = &config.base;
    base.worker.read(512 << 20, false)?;
    let images = base.images.read()?;
    let mlp = config
        .tiles_image
        .read(full::MAX_IMAGE_BYTES as u64, true)?;
    let prefix = config
        .prefix_image
        .read(full::MAX_IMAGE_BYTES as u64, true)?;
    let projection = config
        .projection_image
        .read(full::MAX_IMAGE_BYTES as u64, true)?;
    let guarded = config
        .guarded_image
        .read(full::MAX_IMAGE_BYTES as u64, true)?;
    let (prompt_text, prompt) = base.prompt.read()?;
    let model = EngineeringQwenModelV1::open(&base.source)?;
    require(
        *model.bundle_id().as_bytes() == base.expected_bundle_id
            && *model.config().model_id.as_bytes() == base.expected_model_id,
        "full2303 authenticated model/bundle",
    )?;
    require(
        model.encode_with_limits(
            &prompt_text,
            ferric_build::TokenizerExecutionLimits::long_context(),
        )? == prompt,
        "full2303 authenticated raw prompt/tokenizer",
    )?;
    let mut evidence = retained::Evidence::create(&base.evidence_directory)?;
    let scope = EngineeringTpPoolScopeV1 {
        model: base.expected_model_id,
        session: base.session,
    };
    let limits =
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).map_err(|e| format!("{e:?}"))?;
    let mut pool = EngineeringTpPagedPoolV1::new(scope, limits).map_err(|e| format!("{e:?}"))?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(base.child_deadline_ms))
        .ok_or("full2303 deadline overflow")?;
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
    let mut child = process::OwnedChild::spawn_command(command, deadline)?;
    let pid = child.id();
    eprintln!("finite guarded full2303 owned child pid={pid} pgid={pid}; setup pending");
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
        let b = full::Bootstrap {
            schema: full::SCHEMA.into(),
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
        full::write_bootstrap(
            child.input.as_mut().ok_or("full2303 stdin absent")?,
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
                require(offset == total, "full2303 head contiguous offset")?;
                stream.write_tail(offset, bytes)?;
                total += bytes.len();
                digest.update(bytes);
                Ok(())
            },
        )?;
        let actual: [u8; 32] = digest.finalize().into();
        require(
            total == head.bytes() && actual == head.sha256(),
            "full2303 head recipe changed",
        )?;
        Ok((b, stream.seal()?))
    })?;
    let b = &bootstrap.sequence;
    let profile = b.sha256().map_err(|e| e.to_string())?;
    let sequence = pool
        .open_sequence(scope, &[prompt[0]], 0)
        .map_err(|e| format!("{e:?}"))?;
    require(
        sequence.hit_tokens() == 0 && sequence.physical_pages().is_empty(),
        "full2303 fresh history",
    )?;
    let mut transcript = long::Transcript::new(b.clone()).map_err(|e| e.to_string())?;
    let mut stable = None;
    for position in 0..long::FORWARDS {
        let token = super::input_token(
            &prompt,
            position,
            transcript.output_tokens().last().copied(),
        )?;
        require(
            pool.committed_position(sequence.sequence())
                .map_err(|e| format!("{e:?}"))?
                == position,
            "full2303 host/native committed position",
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
                    .ok_or("full2303 forward stdin absent")?,
                &mut sent,
                &request,
            )
            .map_err(|e| e.to_string())?;
            child
                .input
                .as_mut()
                .ok_or("full2303 stdin absent")?
                .flush()
                .map_err(|e| e.to_string())?;
            let (frame, capture) = long::read_frame(
                child.output.as_mut().ok_or("full2303 stdout absent")?,
                &mut received,
                selected_profile,
            )
            .map_err(|e| e.to_string())?
            .ok_or("full2303 EOF before completion")?;
            require(
                frame.request == request,
                "full2303 actual request/completion join",
            )?;
            child.check_deadline()?;
            evidence.append(&frame, capture.as_ref())?;
            transcript.advance(&frame).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if let Err(error) = result {
            pool.quarantine_batch(&batch)
                .map_err(|e| format!("full2303 quarantine: {e:?}; original: {error}"))?;
            return Err(error);
        }
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .map_err(|e| format!("{e:?}"))?;
        eprintln!(
            "finite guarded full2303 completed position={position} generated={}",
            transcript.output_tokens().len(),
        );
    }
    require(
        transcript.completed() == long::FORWARDS
            && transcript.output_tokens().len() == long::OUTPUT_TOKENS,
        "full2303 complete own-output history",
    )?;
    let close_request = long::Request {
        protocol: 1,
        id: u64::from(long::FORWARDS) + 1,
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
        child.input.as_mut().ok_or("full2303 Close stdin absent")?,
        &mut sent,
        &close_request,
    )
    .map_err(|e| e.to_string())?;
    child
        .input
        .as_mut()
        .ok_or("full2303 stdin absent")?
        .flush()
        .map_err(|e| e.to_string())?;
    let close = long::read_record::<full::Closed>(
        child
            .output
            .as_mut()
            .ok_or("full2303 Close stdout absent")?,
        &mut received,
    )
    .map_err(|e| e.to_string())?
    .ok_or("full2303 EOF before Close")?;
    close
        .validate(&close_request, digest, transcript.output_tokens())
        .map_err(|e| e.to_string())?;
    let stderr = child.finish()?;
    match policy {
        NativePolicy::Full => {}
        NativePolicy::ScopedWarm => {
            scoped::validate_original(
                &stderr,
                &bootstrap,
                digest,
                transcript.output_tokens(),
                base.worker.sha256,
            )?;
            retained::check_deadline(Some(deadline))?;
        }
        NativePolicy::BankScopedCensus => {
            bank_scoped_census::validate_original(
                &stderr,
                &bootstrap,
                digest,
                transcript.output_tokens(),
                base.worker.sha256,
            )?;
            retained::check_deadline(Some(deadline))?;
        }
        NativePolicy::BankScopedCensusTail => {
            bank_scoped_census_tail::validate_original(
                &stderr,
                &bootstrap,
                digest,
                transcript.output_tokens(),
                base.worker.sha256,
            )?;
            retained::check_deadline(Some(deadline))?;
        }
    }
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
    let generated_tokens = transcript.output_tokens().to_vec();
    let decoded = model.decode(&generated_tokens)?;
    require(decoded.len() <= 128 << 10, "full2303 decoded byte bound")?;
    let files = evidence.finish(&stderr, &decoded)?;
    let mut observation = Observation {
        schema: OBSERVATION_SCHEMA,
        child_pid: pid,
        registration_sha256: b.registration,
        source_program_sha256: b.begin.source_program.sha256,
        upload_manifest_sha256: b.begin.uploads.sha256,
        profile_sha256: profile,
        setup_commands,
        completed_forwards: long::FORWARDS,
        prompt_positions_executed: long::PROMPT_TOKENS as u32,
        decode_positions_executed: long::OUTPUT_TOKENS as u32 - 1,
        decoded_special_token_policy: "skip",
        generated_tokens,
        page_permutation: stable.ok_or("full2303 page permutation missing")?.to_vec(),
        transcript_sha256: digest,
        request_stream_bytes: sent.used(),
        response_stream_bytes: received.used(),
        files,
        close,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        full_long_workload: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        bootstrap,
        request: config,
    };
    validate_summary(&observation)?;
    match policy {
        NativePolicy::Full => retained::publish(&mut observation)?,
        NativePolicy::ScopedWarm => retained::publish_scoped(&mut observation, deadline)?,
        NativePolicy::BankScopedCensus => {
            retained::publish_bank_scoped_census(&mut observation, deadline)?
        }
        NativePolicy::BankScopedCensusTail => {
            retained::publish_bank_scoped_census_tail(&mut observation, deadline)?
        }
    }
    Ok(observation)
}

#[cfg(test)]
#[path = "full2303_tests.rs"]
mod tests;
