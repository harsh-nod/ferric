//! Command-line extraction of one authority-free aggregate source-pin projection.

#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fe2o3_runtime_protocol::MAX_WORKER_V3_LOAD_ENVELOPE_BYTES_V2;
use ferric_qwen3_all_kernels_worker_v3_source_pin_v1::extract_m1_aggregate_source_pin_v1;

const USAGE: &str = "usage: ferric-qwen3-all-kernels-worker-v3-source-pin-v1 ENVELOPE|-";

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match parse_input(&arguments).and_then(|path| execute(&path)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ferric-qwen3-all-kernels-worker-v3-source-pin-v1: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_input(arguments: &[OsString]) -> Result<PathBuf, String> {
    match arguments {
        [path] => Ok(PathBuf::from(path)),
        _ => Err(USAGE.to_owned()),
    }
}

fn execute(path: &Path) -> Result<(), String> {
    let bytes = if path.as_os_str() == OsStr::new("-") {
        let mut input = io::stdin().lock();
        read_bounded(&mut input, MAX_WORKER_V3_LOAD_ENVELOPE_BYTES_V2)?
    } else {
        let mut input = File::open(path)
            .map_err(|error| format!("cannot open envelope {}: {error}", path.display()))?;
        read_bounded(&mut input, MAX_WORKER_V3_LOAD_ENVELOPE_BYTES_V2)?
    };
    let projection =
        extract_m1_aggregate_source_pin_v1(&bytes).map_err(|error| error.to_string())?;
    let output = projection
        .to_canonical_json()
        .map_err(|error| error.to_string())?;
    io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|error| format!("cannot write source-pin JSON: {error}"))
}

fn read_bounded<R: Read>(reader: &mut R, maximum: usize) -> Result<Vec<u8>, String> {
    let limit = u64::try_from(maximum)
        .map_err(|_| "envelope byte bound exceeds u64".to_owned())?
        .checked_add(1)
        .ok_or_else(|| "envelope byte bound overflowed".to_owned())?;
    let mut bytes = Vec::new();
    reader
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read envelope: {error}"))?;
    if bytes.len() > maximum {
        return Err(format!("envelope exceeds {maximum} bytes"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn command_requires_one_exact_input() {
        assert_eq!(
            parse_input(&[OsString::from("envelope.bin")]),
            Ok(PathBuf::from("envelope.bin"))
        );
        assert_eq!(parse_input(&[OsString::from("-")]), Ok(PathBuf::from("-")));
        assert_eq!(parse_input(&[]), Err(USAGE.to_owned()));
        assert_eq!(
            parse_input(&[OsString::from("a"), OsString::from("b")]),
            Err(USAGE.to_owned())
        );
    }

    #[test]
    fn bounded_reader_rejects_one_byte_over_limit() {
        let mut exact = Cursor::new([1_u8, 2, 3, 4]);
        assert_eq!(read_bounded(&mut exact, 4), Ok(vec![1, 2, 3, 4]));

        let mut oversized = Cursor::new([1_u8, 2, 3, 4, 5]);
        assert_eq!(
            read_bounded(&mut oversized, 4),
            Err("envelope exceeds 4 bytes".to_owned())
        );
    }

    #[test]
    fn malformed_input_never_emits_a_projection() {
        let path = Path::new("this-path-does-not-exist");
        let error = execute(path).expect_err("missing input must fail");
        assert!(error.contains("cannot open envelope"));
    }
}
