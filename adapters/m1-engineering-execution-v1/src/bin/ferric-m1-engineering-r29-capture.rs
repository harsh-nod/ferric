#![forbid(unsafe_code)]

//! Authority-none R29 technical capture over one aggregate engineering observation.

#[allow(dead_code)] // The shared source also contains protected R30/R32 commands unreachable here.
#[allow(
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::semicolon_if_nothing_returned,
    clippy::used_underscore_binding
)] // Mirror only the owning root workspace's lint policy for this shared module.
#[rustfmt::skip] // Skip cross-edition traversal only; the root workspace formats this shared module.
#[path = "../../../../crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs"]
mod qualification_capture;

use ferric_m1_engineering_execution_v1::{
    M1EngineeringAggregateArtifactV1, bind_engineering_structural_m1_physical_runner_v1,
    reopen_m1_engineering_aggregate_artifact_v1, reopen_m1_engineering_mfma_aggregate_artifact_v1,
};
use ferric_spec::Identity;
use qualification_capture::{
    CaptureResult, EngineeringArtifactCoordinatesV1, M1R29CaptureProgramSourceV1,
};
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

struct EngineeringAggregateProgramSourceV1<const MFMA: bool = false>;

impl<const MFMA: bool> M1R29CaptureProgramSourceV1 for EngineeringAggregateProgramSourceV1<MFMA> {
    type Artifact = M1EngineeringAggregateArtifactV1;
    const CAPTURE_COMMAND: &'static str = "ferric-m1-engineering-r29-capture";
    const INVOCATION_FORMAT: &'static str = "FERRIC-M1-TECHNICAL-PREQUALIFICATION-INVOCATIONS-V1";
    const ENGINEERING_CAPTURE_SUBCOMMAND: &'static str = if MFMA {
        "capture-engineering-mfma"
    } else {
        "capture-engineering"
    };

    fn pre_capture(_root: &Path) -> CaptureResult<()> {
        Ok(())
    }

    fn reopen(root: &Path) -> CaptureResult<Self::Artifact> {
        let artifact = if MFMA {
            reopen_m1_engineering_mfma_aggregate_artifact_v1(root)
        } else {
            reopen_m1_engineering_aggregate_artifact_v1(root)
        };
        artifact.map_err(|error| format!("cannot admit aggregate engineering artifact: {error}"))
    }

    fn program_catalog_id(artifact: &Self::Artifact) -> Identity {
        artifact.program_catalog_id()
    }

    fn engineering_coordinates(
        artifact: &Self::Artifact,
    ) -> CaptureResult<EngineeringArtifactCoordinatesV1> {
        Ok(EngineeringArtifactCoordinatesV1 {
            manifest: artifact.manifest_id(),
            hsaco: artifact.hsaco_id(),
            compiler_handoff: artifact.compiler_handoff_id(),
            canonical_descriptor: artifact.canonical_descriptor_id(),
            program_catalog: artifact.program_catalog_id(),
        })
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
        Some("generate-engineering-mfma-inputs") => {
            qualification_capture::generate_engineering_r29_inputs::<
                EngineeringAggregateProgramSourceV1<true>,
            >(&arguments[1..], false)
        }
        Some("validate-engineering-mfma-inputs") => {
            qualification_capture::generate_engineering_r29_inputs::<
                EngineeringAggregateProgramSourceV1<true>,
            >(&arguments[1..], true)
        }
        Some("capture-engineering-mfma") => qualification_capture::run_engineering_r29_capture::<
            EngineeringAggregateProgramSourceV1<true>,
        >(&arguments[1..]),
        Some("generate-engineering-inputs") => {
            qualification_capture::generate_engineering_r29_inputs::<
                EngineeringAggregateProgramSourceV1,
            >(&arguments[1..], false)
        }
        Some("validate-engineering-inputs") => {
            qualification_capture::generate_engineering_r29_inputs::<
                EngineeringAggregateProgramSourceV1,
            >(&arguments[1..], true)
        }
        Some("capture-engineering") => qualification_capture::run_engineering_r29_capture::<
            EngineeringAggregateProgramSourceV1,
        >(&arguments[1..]),
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

#[cfg(test)]
mod mfma_command_tests {
    use super::*;

    #[test]
    fn explicit_mfma_provider_records_a_disjoint_capture_subcommand() {
        assert_eq!(
            EngineeringAggregateProgramSourceV1::<false>::ENGINEERING_CAPTURE_SUBCOMMAND,
            "capture-engineering"
        );
        assert_eq!(
            EngineeringAggregateProgramSourceV1::<true>::ENGINEERING_CAPTURE_SUBCOMMAND,
            "capture-engineering-mfma"
        );
        assert_eq!(
            EngineeringAggregateProgramSourceV1::<false>::CAPTURE_COMMAND,
            EngineeringAggregateProgramSourceV1::<true>::CAPTURE_COMMAND
        );
        let source = include_str!("ferric-m1-engineering-r29-capture.rs");
        let provider = source
            .split("    fn reopen(root:")
            .nth(1)
            .unwrap()
            .split("    fn program_catalog_id(")
            .next()
            .unwrap();
        assert!(provider.contains("if MFMA {\n            reopen_m1_engineering_mfma_aggregate_artifact_v1(root)\n        } else {\n            reopen_m1_engineering_aggregate_artifact_v1(root)"));
        assert!(!provider.contains("std::env"));
    }

    #[test]
    fn explicit_mfma_commands_reject_missing_inputs_before_artifact_or_gpu_use() {
        for command in [
            "generate-engineering-mfma-inputs",
            "validate-engineering-mfma-inputs",
            "capture-engineering-mfma",
        ] {
            assert!(run(&[OsString::from(command)]).is_err());
        }
    }
}
