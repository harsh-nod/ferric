#![forbid(unsafe_code)]

//! Authority-none R29 technical capture over one aggregate engineering observation.

#[allow(dead_code)] // The shared source also contains protected R30/R32 commands unreachable here.
#[allow(
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::semicolon_if_nothing_returned
)] // Mirror only the owning root workspace's lint policy for this shared module.
#[rustfmt::skip] // Skip cross-edition traversal only; the root workspace formats this shared module.
#[path = "../../../../crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs"]
mod qualification_capture;

use ferric_m1_engineering_execution_v1::{
    M1EngineeringAggregateArtifactV1, bind_engineering_structural_m1_physical_runner_v1,
    reopen_m1_engineering_aggregate_artifact_v1,
};
use ferric_spec::Identity;
use qualification_capture::{CaptureResult, M1R29CaptureProgramSourceV1};
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

struct EngineeringAggregateProgramSourceV1;

impl M1R29CaptureProgramSourceV1 for EngineeringAggregateProgramSourceV1 {
    type Artifact = M1EngineeringAggregateArtifactV1;
    const CAPTURE_COMMAND: &'static str = "ferric-m1-engineering-r29-capture";
    const INVOCATION_FORMAT: &'static str = "FERRIC-M1-TECHNICAL-PREQUALIFICATION-INVOCATIONS-V1";

    fn pre_capture(_root: &Path) -> CaptureResult<()> {
        Ok(())
    }

    fn reopen(root: &Path) -> CaptureResult<Self::Artifact> {
        reopen_m1_engineering_aggregate_artifact_v1(root)
            .map_err(|error| format!("cannot admit aggregate engineering artifact: {error}"))
    }

    fn program_catalog_id(artifact: &Self::Artifact) -> Identity {
        artifact.program_catalog_id()
    }

    fn bind(
        artifact: Self::Artifact,
        publication: ferric_build::PublishedRunnerDeclaration,
    ) -> CaptureResult<ferric_engine::M1PhysicalRunnerV1> {
        bind_engineering_structural_m1_physical_runner_v1(artifact, publication)
            .map_err(|error| format!("cannot bind aggregate engineering runner: {error:?}"))
    }
}

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[OsString]) -> CaptureResult<()> {
    match arguments.first().and_then(|argument| argument.to_str()) {
        Some("generate-inputs") => qualification_capture::generate_technical_r29_inputs::<
            EngineeringAggregateProgramSourceV1,
        >(&arguments[1..]),
        Some("validate-inputs") => qualification_capture::validate_technical_r29_inputs::<
            EngineeringAggregateProgramSourceV1,
        >(&arguments[1..]),
        _ => {
            qualification_capture::run_technical_r29_capture::<EngineeringAggregateProgramSourceV1>(
                arguments,
            )
        }
    }
}
