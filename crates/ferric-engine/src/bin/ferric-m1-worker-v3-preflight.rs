//! Authority-free host admission preflight for exact M1 Worker V3 publications.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ferric_build::M1_PHYSICAL_PROGRAM_COUNT_V1;
use ferric_engine::{
    decode_m1_worker_v3_selector_manifest_v1, recover_m1_worker_v3_rosters_v1,
    M1_WORKER_V3_SELECTOR_MANIFEST_MAX_BYTES_V1,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const RESULT_FORMAT: &str = "ferric.m1-worker-v3-host-admission-preflight.v1";
const USAGE: &str = "usage: ferric-m1-worker-v3-preflight SELECTOR-MANIFEST";

fn main() -> ExitCode {
    match parse_command(std::env::args_os().skip(1).collect()).and_then(execute) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ferric-m1-worker-v3-preflight: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_command(arguments: Vec<OsString>) -> Result<PathBuf, String> {
    match arguments.as_slice() {
        [manifest] => Ok(PathBuf::from(manifest)),
        _ => Err(USAGE.to_owned()),
    }
}

fn execute(manifest_path: PathBuf) -> Result<(), String> {
    let bytes = read_bounded_manifest(&manifest_path)?;
    let manifest_sha256 = hex(&Sha256::digest(&bytes));
    let selectors =
        decode_m1_worker_v3_selector_manifest_v1(&bytes).map_err(|error| error.to_string())?;
    let rosters = recover_m1_worker_v3_rosters_v1(selectors).map_err(|error| error.to_string())?;
    let program_count = rosters.program_count();
    if program_count != M1_PHYSICAL_PROGRAM_COUNT_V1 {
        return Err(format!(
            "host-admitted program count {program_count} differs from the exact M1 count {M1_PHYSICAL_PROGRAM_COUNT_V1}"
        ));
    }
    if rosters.authenticates_verification_authority() {
        return Err("host admission unexpectedly reported verification authority".to_owned());
    }
    drop(rosters);
    write_canonical_stdout(&json!({
        "authentication_authority": false,
        "format": RESULT_FORMAT,
        "grants_launch_authority": false,
        "grants_load_authority": false,
        "host_admitted": true,
        "program_count": program_count,
        "selector_manifest_sha256": manifest_sha256,
        "submitted_gpu_work": false,
    }))
}

fn read_bounded_manifest(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path)
        .map_err(|error| format!("cannot open selector manifest {}: {error}", path.display()))?;
    let limit = u64::try_from(M1_WORKER_V3_SELECTOR_MANIFEST_MAX_BYTES_V1)
        .expect("selector manifest bound fits u64")
        + 1;
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read selector manifest {}: {error}", path.display()))?;
    if bytes.len() > M1_WORKER_V3_SELECTOR_MANIFEST_MAX_BYTES_V1 {
        return Err(format!(
            "selector manifest exceeds {M1_WORKER_V3_SELECTOR_MANIFEST_MAX_BYTES_V1} bytes"
        ));
    }
    Ok(bytes)
}

fn write_canonical_stdout(value: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("cannot encode preflight result: {error}"))?;
    if !bytes.is_ascii() {
        return Err("preflight result is not ASCII".to_owned());
    }
    bytes.push(b'\n');
    std::io::stdout()
        .lock()
        .write_all(&bytes)
        .map_err(|error| format!("cannot write preflight result: {error}"))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_requires_one_exact_manifest_path() {
        assert_eq!(
            parse_command(vec![OsString::from("selectors.json")]),
            Ok(PathBuf::from("selectors.json"))
        );
        assert_eq!(parse_command(Vec::new()), Err(USAGE.to_owned()));
        assert_eq!(
            parse_command(vec![OsString::from("a"), OsString::from("b")]),
            Err(USAGE.to_owned())
        );
    }
}
