#![forbid(unsafe_code)]

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use ferric_build::{build_and_publish_m1_kernel_artifacts_v1, M1KernelArtifactPublicationStatusV1};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ferric-m1-kernel-artifacts: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os();
    let executable = arguments
        .next()
        .unwrap_or_else(|| OsString::from("ferric-m1-kernel-artifacts"));
    let worker = arguments.next().ok_or_else(|| usage(&executable))?;
    let output = arguments.next().ok_or_else(|| usage(&executable))?;
    if arguments.next().is_some() {
        return Err(usage(&executable).into());
    }

    let built = build_and_publish_m1_kernel_artifacts_v1(Path::new(&worker), Path::new(&output))?;
    println!("output={}", built.output_directory().display());
    println!(
        "manifest_sha256={}",
        hex(built.manifest().identity().sha256())
    );
    println!("manifest_bytes={}", built.manifest().identity().byte_len());
    match built.publication_status() {
        M1KernelArtifactPublicationStatusV1::ParentDirectorySynced => {
            println!("publication=parent-directory-synced");
        }
        M1KernelArtifactPublicationStatusV1::PublishedButParentDirectorySyncFailed {
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
    for entry in built.manifest().entries() {
        println!(
            "artifact={} sha256={} bytes={} path={}",
            entry.family().name(),
            hex(entry.artifact().sha256()),
            entry.artifact().byte_len(),
            entry.object_path()
        );
    }
    Ok(())
}

fn usage(executable: &OsString) -> String {
    format!(
        "usage: {} <exact-reviewed-worker-path> <new-output-directory>",
        Path::new(executable).display()
    )
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}
