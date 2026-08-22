#![forbid(unsafe_code)]

use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ferric_build::{
    publish_qwen3_m1_generated_runner_v1, render_qwen3_gfx942_runner_source,
    validate_qwen3_m1_generated_runner_sources_v1, M1GeneratedRunnerPublicationStatusV1,
    M1_GENERATED_RUNNER_CRATE_SOURCE_PATH_V1, M1_GENERATED_RUNNER_PATH_V1,
    M1_GENERATED_RUNNER_VALIDATION_SOURCE_PATH_V1,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ferric-m1-generate-runner: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os();
    let executable = arguments
        .next()
        .unwrap_or_else(|| OsString::from("ferric-m1-generate-runner"));
    let first = arguments.next();
    let (check_only, root) = match first.as_deref() {
        Some(value) if value == "--check" => {
            (true, arguments.next().ok_or_else(|| usage(&executable))?)
        }
        Some(value) => (false, value.to_os_string()),
        None => return Err(usage(&executable).into()),
    };
    if arguments.next().is_some() {
        return Err(usage(&executable).into());
    }
    let root = PathBuf::from(root);
    let roadmap_path = root.join(M1_GENERATED_RUNNER_PATH_V1);
    let crate_source = fs::read(root.join(M1_GENERATED_RUNNER_CRATE_SOURCE_PATH_V1))?;
    let validation_source = fs::read(root.join(M1_GENERATED_RUNNER_VALIDATION_SOURCE_PATH_V1))?;
    validate_qwen3_m1_generated_runner_sources_v1(
        &render_qwen3_gfx942_runner_source(),
        &crate_source,
        &validation_source,
    )?;
    if !check_only {
        fs::create_dir_all(root.join("generated"))?;
        let status = publish_qwen3_m1_generated_runner_v1(&roadmap_path)?;
        match status {
            M1GeneratedRunnerPublicationStatusV1::AlreadyCurrent => {
                println!("publication=already-current");
            }
            M1GeneratedRunnerPublicationStatusV1::PublishedAndParentDirectorySynced => {
                println!("publication=parent-directory-synced");
            }
            M1GeneratedRunnerPublicationStatusV1::PublishedButParentDirectorySyncFailed {
                parent_directory,
                source,
            } => {
                println!("publication=visible-parent-directory-sync-failed");
                eprintln!(
                    "warning: output is visible, but syncing parent directory {} failed: {source}",
                    parent_directory.display()
                );
            }
        }
    }
    validate_qwen3_m1_generated_runner_sources_v1(
        &fs::read(&roadmap_path)?,
        &crate_source,
        &validation_source,
    )?;
    println!("validated={}", roadmap_path.display());
    Ok(())
}

fn usage(executable: &OsString) -> String {
    format!(
        "usage: {} [--check] <ferric-repository-root>",
        Path::new(executable).display()
    )
}
