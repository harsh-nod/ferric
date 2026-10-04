//! One genuine layer-zero capture, independent of the legacy paired comparison.
use super::*;

const REQUEST_SCHEMA: &str = "FerricFinitePrefixLayerCaptureRequestV1";

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct CaptureConfig(Config);

impl CaptureConfig {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= 65536,
            "layer capture request bound",
        )?;
        let config: Config = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        config.validate_schema(REQUEST_SCHEMA)?;
        Ok(Self(config))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CaptureStage {
    pub rank: u32,
    pub stage: &'static str,
    pub offset: usize,
    pub bytes: usize,
    pub elements: usize,
    pub element_bytes: usize,
    pub sha256: [u8; 32],
}

#[derive(Serialize)]
pub struct CaptureObservation {
    pub schema: &'static str,
    pub request: CaptureConfig,
    pub run: RunRecord,
    pub stages: Vec<CaptureStage>,
    pub files: Vec<evidence::File>,
    pub native_attempts: u32,
    pub retries: u32,
    pub completed_layers: u32,
    pub native_closed: bool,
    pub gpu_execution: bool,
    pub paired_comparison_performed: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
    pub full_forward: bool,
}

fn pinned_part(pin: &FilePin) -> setup_wire::Part {
    // Config and ImagePins bounds are checked before this private conversion.
    setup_wire::Part {
        bytes: pin.bytes as u32,
        sha256: pin.sha256,
    }
}

fn closed_stages(config: &Config, record: &RunRecord, capture: &[u8]) -> Result<Vec<CaptureStage>> {
    let b = &record.bootstrap;
    b.validate().map_err(|e| e.to_string())?;
    let scope = &b.begin.scope;
    require(
        record.child_pid != 0
            && record.child_exit_zero
            && record.process_group_absent
            && record.setup_commands != 0
            && b.profile == wire::Profile::Prefix284Mlp548
            && b.device_ids == config.device_ids
            && b.timeout_ms == config.dispatch_timeout_ms
            && scope.child_identity == record.child_pid
            && scope.bundle_id == config.expected_bundle_id
            && scope.model_id == config.expected_model_id
            && scope.session == config.session
            && b.input.token == 9112
            && b.prefix_image == Some(pinned_part(&config.prefix_tiles_image))
            && b.mlp_image == pinned_part(&config.mlp_tiles_image)
            && b.begin.prefix_image == pinned_part(&config.images.prefix)
            && b.begin.mlp_image == pinned_part(&config.images.mlp)
            && b.begin.residual_image == pinned_part(&config.images.residual)
            && b.begin.tail_image == Some(pinned_part(&config.images.tail))
            && record.profile_sha256 == b.sha256().map_err(|e| e.to_string())?,
        "layer capture actual request/child/images/input",
    )?;
    response(
        &wire::Request {
            protocol: wire::PROTOCOL,
            id: 2,
            profile_sha256: record.profile_sha256,
            command: wire::Command::Close,
        },
        b,
        &record.close,
    )?;
    require(
        record.close.capture == Some(wire::part(capture)),
        "layer capture actual Close payload",
    )?;
    untouched_kv(&b.input, capture)?;
    let rows = wire::capture_rows(capture).map_err(|e| e.to_string())?;
    let mut offset = 0;
    let mut stages = Vec::with_capacity(28);
    for (i, bytes) in rows.into_iter().enumerate() {
        let (stage, count, width) = wire::STAGES[i % 14];
        stages.push(CaptureStage {
            rank: (i / 14) as u32,
            stage,
            offset,
            bytes: count,
            elements: count / width,
            element_bytes: width,
            sha256: hash(bytes),
        });
        offset += count;
    }
    Ok(stages)
}

impl CaptureObservation {
    fn closed(
        config: CaptureConfig,
        record: RunRecord,
        capture: &[u8],
        files: Vec<evidence::File>,
    ) -> Result<Self> {
        let stages = closed_stages(&config.0, &record, capture)?;
        Ok(Self {
            schema: "FerricFinitePrefixLayerCaptureObservationV1",
            request: config,
            run: record,
            stages,
            files,
            native_attempts: 1,
            retries: 0,
            completed_layers: 1,
            native_closed: true,
            gpu_execution: true,
            paired_comparison_performed: false,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
            full_forward: false,
        })
    }
}

/// A single candidate owner with a genuine model embedding, not injected stage inputs.
/// Finite captures and successful Close/reap are observations, not numerical acceptance.
pub fn run_capture(
    config: CaptureConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<CaptureObservation> {
    require(
        allow_unauthenticated_machine_code,
        "layer capture explicit engineering opt-in required",
    )?;
    let c = &config.0;
    c.validate_schema(REQUEST_SCHEMA)?;
    c.worker.read(512 << 20, false)?;
    let images = c.images.read()?;
    let prefix = c.prefix_tiles_image.read(32 << 20, true)?;
    let mlp = c.mlp_tiles_image.read(32 << 20, true)?;
    let (text, tokens) = c.prompt.read()?;
    let model = EngineeringQwenModelV1::open(&c.source)?;
    require(
        *model.bundle_id().as_bytes() == c.expected_bundle_id
            && *model.config().model_id.as_bytes() == c.expected_model_id,
        "layer capture original model/bundle mismatch",
    )?;
    require(
        model.encode_with_limits(
            &text,
            ferric_build::TokenizerExecutionLimits::long_context(),
        )? == tokens,
        "layer capture original tokenizer mismatch",
    )?;
    let mut evidence = evidence::Evidence::create(&c.evidence_directory)?;
    evidence.json("request.json", &config, 16384)?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(c.child_deadline_ms))
        .ok_or("layer capture deadline overflow")?;
    let candidate = run_one(
        c,
        &model,
        &images,
        &mlp,
        Some(&prefix),
        tokens[0],
        deadline,
        None,
        &mut evidence,
    )?;
    c.worker.read(512 << 20, false)?;
    c.images.read()?;
    c.prefix_tiles_image.read(32 << 20, false)?;
    c.mlp_tiles_image.read(32 << 20, false)?;
    c.prompt.recheck()?;
    let observation = CaptureObservation::closed(
        config,
        candidate.record,
        &candidate.capture,
        evidence.files(),
    )?;
    evidence.finish_capture(&observation)?;
    Ok(observation)
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
