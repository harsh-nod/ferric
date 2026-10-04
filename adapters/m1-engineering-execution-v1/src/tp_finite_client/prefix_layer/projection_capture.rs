//! One separately identified projection-boundary candidate, with no legacy fallback.
use super::capture::{CaptureStage, closed_stages_for_profile, pinned_part};
use super::*;

const REQUEST_SCHEMA: &str = "FerricFiniteProjectionResidualLayerCaptureRequestV1";
const LAYER_SCHEMA: &str = "FerricFinitePrefixLayerCaptureRequestV1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionCaptureConfig {
    pub schema: String,
    pub layer: Config,
    pub projection_residual_image: FilePin,
}

impl ProjectionCaptureConfig {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= 65536,
            "projection capture request bound",
        )?;
        let config: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        self.layer.validate_schema(LAYER_SCHEMA)?;
        let image = &self.projection_residual_image;
        require(
            self.schema == REQUEST_SCHEMA
                && image.path.is_absolute()
                && image.bytes > 0
                && image.bytes <= 32 << 20
                && image.sha256 != [0; 32]
                && image.sha256 != self.layer.images.residual.sha256
                && serde_json::to_vec(self).map_err(|e| e.to_string())?.len() <= 16384,
            "projection capture distinct image/request identity",
        )
    }
}

#[derive(Serialize)]
pub struct ProjectionRunRecord {
    pub child_pid: u32,
    pub bootstrap: projection_wire::Bootstrap,
    pub profile_sha256: [u8; 32],
    pub setup_commands: u64,
    pub close: wire::Response,
    pub child_exit_zero: bool,
    pub process_group_absent: bool,
}

#[derive(Serialize)]
pub struct ProjectionCaptureObservation {
    pub schema: &'static str,
    pub request: ProjectionCaptureConfig,
    pub run: ProjectionRunRecord,
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

impl ProjectionCaptureObservation {
    fn closed(
        config: ProjectionCaptureConfig,
        record: RunRecord,
        bootstrap: projection_wire::Bootstrap,
        capture: &[u8],
        files: Vec<evidence::File>,
    ) -> Result<Self> {
        config.validate()?;
        bootstrap.validate().map_err(|e| e.to_string())?;
        require(
            bootstrap.layer == record.bootstrap
                && bootstrap.projection_residual_image
                    == pinned_part(&config.projection_residual_image),
            "projection capture actual extra image/bootstrap",
        )?;
        // The unchanged inner Control/capture checks use the new authenticated
        // profile digest, not a synthetic old response or an old parity result.
        let stages = closed_stages_for_profile(
            &config.layer,
            &record,
            capture,
            bootstrap.sha256().map_err(|e| e.to_string())?,
        )?;
        Ok(Self {
            schema: "FerricFiniteProjectionResidualLayerCaptureObservationV1",
            request: config,
            run: ProjectionRunRecord {
                child_pid: record.child_pid,
                bootstrap,
                profile_sha256: record.profile_sha256,
                setup_commands: record.setup_commands,
                close: record.close,
                child_exit_zero: record.child_exit_zero,
                process_group_absent: record.process_group_absent,
            },
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

/// The original residual image still supplies copy. Only the separate candidate
/// image supplies the projection materialization, under its own wire/profile.
pub fn run_projection_capture(
    config: ProjectionCaptureConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<ProjectionCaptureObservation> {
    require(
        allow_unauthenticated_machine_code,
        "projection capture explicit engineering opt-in required",
    )?;
    config.validate()?;
    let c = &config.layer;
    c.worker.read(512 << 20, false)?;
    let images = c.images.read()?;
    let prefix = c.prefix_tiles_image.read(32 << 20, true)?;
    let mlp = c.mlp_tiles_image.read(32 << 20, true)?;
    let projection = config.projection_residual_image.read(32 << 20, true)?;
    let (text, tokens) = c.prompt.read()?;
    let model = EngineeringQwenModelV1::open(&c.source)?;
    require(
        *model.bundle_id().as_bytes() == c.expected_bundle_id
            && *model.config().model_id.as_bytes() == c.expected_model_id,
        "projection capture original model/bundle mismatch",
    )?;
    require(
        model.encode_with_limits(
            &text,
            ferric_build::TokenizerExecutionLimits::long_context(),
        )? == tokens,
        "projection capture original tokenizer mismatch",
    )?;
    let mut evidence = evidence::Evidence::create(&c.evidence_directory)?;
    evidence.json("request.json", &config, 16384)?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(c.child_deadline_ms))
        .ok_or("projection capture deadline overflow")?;
    let candidate = run_one_with_projection(
        c,
        &model,
        &images,
        &mlp,
        Some(&prefix),
        Some(&projection),
        tokens[0],
        deadline,
        None,
        &mut evidence,
    )?;
    c.worker.read(512 << 20, false)?;
    c.images.read()?;
    c.prefix_tiles_image.read(32 << 20, false)?;
    c.mlp_tiles_image.read(32 << 20, false)?;
    config.projection_residual_image.read(32 << 20, false)?;
    c.prompt.recheck()?;
    let observation = ProjectionCaptureObservation::closed(
        config,
        candidate.record,
        candidate
            .projection_bootstrap
            .ok_or("projection bootstrap not retained")?,
        &candidate.capture,
        evidence.files(),
    )?;
    evidence.finish_capture(&observation)?;
    Ok(observation)
}

#[cfg(test)]
#[path = "projection_capture_tests.rs"]
mod tests;
