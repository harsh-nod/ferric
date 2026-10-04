//! Plain TF4 with a separately pinned projection boundary, never legacy parity.
use super::*;

const REQUEST_SCHEMA: &str = "FerricFiniteProjectionResidualDecodeRequestV1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionDecodeConfig {
    pub schema: String,
    pub decode: Config,
    pub projection_residual_image: FilePin,
}

impl ProjectionDecodeConfig {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(
            !bytes.is_empty() && bytes.len() <= 65536,
            "projection decode request bound",
        )?;
        let config: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        self.decode.validate()?;
        let image = &self.projection_residual_image;
        require(
            self.schema == REQUEST_SCHEMA
                && self.decode.mode == wire::InputMode::TeacherForced
                && image.path.is_absolute()
                && image.bytes > 0
                && image.bytes <= wire::MAX_IMAGE_BYTES as u64
                && image.sha256 != [0; 32]
                && image.sha256 != self.decode.images.residual.sha256
                && serde_json::to_vec(self).map_err(|e| e.to_string())?.len() <= 16384,
            "projection decode distinct image and closed TF4 request",
        )
    }
}

fn pinned_part(image: &FilePin) -> Result<setup_wire::Part> {
    Ok(setup_wire::Part {
        bytes: u32::try_from(image.bytes).map_err(|_| "projection decode image extent overflow")?,
        sha256: image.sha256,
    })
}

pub(super) fn bootstrap(
    decode: wire::Bootstrap,
    image: &FilePin,
) -> Result<projection_wire::Bootstrap> {
    let value = projection_wire::Bootstrap {
        schema: projection_wire::SCHEMA.into(),
        decode,
        projection_residual_image: pinned_part(image)?,
    };
    value
        .validate(
            value.decode.device_ids,
            value.decode.timeout_ms,
            value.decode.scope.child_identity,
            value.decode.mode,
        )
        .map_err(|e| e.to_string())?;
    Ok(value)
}

#[derive(Serialize)]
pub struct ProjectionDecodeObservation {
    pub schema: &'static str,
    pub request: ProjectionDecodeConfig,
    pub child_pid: u32,
    pub registration_sha256: [u8; 32],
    pub source_program_sha256: [u8; 32],
    pub upload_manifest_sha256: [u8; 32],
    pub bootstrap: projection_wire::Bootstrap,
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
    pub paired_comparison_performed: bool,
    pub native_attempts: u32,
    pub retries: u32,
}

impl ProjectionDecodeObservation {
    fn closed(
        config: ProjectionDecodeConfig,
        actual: Observation,
        bootstrap: projection_wire::Bootstrap,
    ) -> Result<Self> {
        config.validate()?;
        bootstrap
            .validate(
                config.decode.device_ids,
                config.decode.dispatch_timeout_ms,
                actual.child_pid,
                config.decode.mode,
            )
            .map_err(|e| e.to_string())?;
        require(
            serde_json::to_value(&config.decode).map_err(|e| e.to_string())?
                == serde_json::to_value(&actual.request).map_err(|e| e.to_string())?
                && bootstrap.decode == actual.bootstrap
                && bootstrap.projection_residual_image
                    == pinned_part(&config.projection_residual_image)?
                && actual.profile_sha256 == bootstrap.sha256().map_err(|e| e.to_string())?
                && actual.registration_sha256 == bootstrap.decode.registration
                && actual.source_program_sha256 == bootstrap.decode.begin.source_program.sha256
                && actual.upload_manifest_sha256 == bootstrap.decode.begin.uploads.sha256
                && actual.input_tokens == INPUT_TOKENS
                && actual.gpu_execution,
            "projection decode actual request/bootstrap/profile/source joins",
        )?;
        // This internal value is never published under the old schema. Its
        // unchanged validators check the actual new-profile Control and Close.
        evidence::validate_closed(&actual)?;
        Ok(Self {
            schema: "FerricFiniteProjectionResidualDecodeObservationV1",
            request: config,
            child_pid: actual.child_pid,
            registration_sha256: actual.registration_sha256,
            source_program_sha256: actual.source_program_sha256,
            upload_manifest_sha256: actual.upload_manifest_sha256,
            bootstrap,
            profile_sha256: actual.profile_sha256,
            setup_commands: actual.setup_commands,
            completed_forwards: actual.completed_forwards,
            input_tokens: actual.input_tokens,
            observed_output_tokens: actual.observed_output_tokens,
            page_permutation: actual.page_permutation,
            transcript_sha256: actual.transcript_sha256,
            request_stream_bytes: actual.request_stream_bytes,
            response_stream_bytes: actual.response_stream_bytes,
            files: actual.files,
            close: actual.close,
            child_exit_zero: actual.child_exit_zero,
            process_group_absent: actual.process_group_absent,
            native_closed: actual.native_closed,
            gpu_execution: actual.gpu_execution,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
            full_long_workload: false,
            paired_comparison_performed: false,
            native_attempts: 1,
            retries: 0,
        })
    }

    fn publish(&mut self) -> Result<()> {
        let directory = self.request.decode.evidence_directory.clone();
        evidence::publish_summary(
            &directory,
            self.files.bytes_before_summary,
            |summary, total| {
                self.files.summary_bytes = summary;
                self.files.total_bytes = total;
                serde_json::to_vec(&self).map_err(|e| e.to_string())
            },
        )
    }
}

pub fn run_projection_decode(
    config: ProjectionDecodeConfig,
    allow_unauthenticated_machine_code: bool,
) -> Result<ProjectionDecodeObservation> {
    require(
        allow_unauthenticated_machine_code,
        "projection decode explicit engineering opt-in required",
    )?;
    config.validate()?;
    let (actual, diagnostic, bootstrap) = run_selected(
        config.decode.clone(),
        true,
        None,
        Some(&config.projection_residual_image),
    )?;
    require(
        diagnostic.is_none(),
        "projection decode cannot publish legacy diagnostics",
    )?;
    let mut observation = ProjectionDecodeObservation::closed(
        config,
        actual,
        bootstrap.ok_or("projection decode bootstrap missing")?,
    )?;
    observation.publish()?;
    Ok(observation)
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
