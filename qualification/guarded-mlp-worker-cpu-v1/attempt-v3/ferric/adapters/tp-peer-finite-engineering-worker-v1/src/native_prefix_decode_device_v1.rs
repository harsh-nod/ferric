//! Separate raw completion-tick diagnostic; never changes ordinary queue modes.
use crate::native_prefix_decode_cli_v1::{self as plain, NativeOptions};
use crate::native_prefix_decode_host_v1::{executable_sha, preflight_path};
use crate::native_prefix_device_recorder_v1::ClosedReport;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub struct Options {
    native: NativeOptions,
    path: PathBuf,
}
pub fn parse_args(args: &[OsString]) -> io::Result<Options> {
    if args.len() != 10
        || args[0] != "--engineering-native-prefix-decode-device-v1"
        || args[8] != "--device-sidecar"
    {
        return Err(io::Error::other(
            "exact device diagnostic arguments required",
        ));
    }
    let mut plain_args = args[..8].to_vec();
    plain_args[0] = "--engineering-native-prefix-decode-v1".into();
    let native = plain::parse_args(&plain_args)?;
    let path = PathBuf::from(&args[9]);
    if !path.is_absolute() || path.file_name().is_none() || path.as_os_str().len() > 512 {
        return Err(io::Error::other("device sidecar absolute bounded path"));
    }
    Ok(Options { native, path })
}

/// Only called after the ordinary Closed reply was successfully written.
pub(crate) fn publish(closed: ClosedReport, path: &Path) -> io::Result<()> {
    if executable_sha()? != closed.worker_sha256() {
        return Err(io::Error::other("device observer executable drift"));
    }
    let raw = closed.encode()?;
    preflight_path(path)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    output.write_all(&raw)?;
    output.sync_all()
}

/// # Safety
/// The trusted parent authenticates model/source/image provenance and owns this
/// disposable process through its unchanged deadline, reap and audit contract.
/// Raw ticks are not calibrated time or cross-device overlap evidence.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: Options,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    preflight_path(&options.path)?;
    let worker = executable_sha()?;
    unsafe { plain::run_device_observed(options.native, r, w, options.path, worker) }
}

#[cfg(test)]
#[path = "native_prefix_decode_device_v1_tests.rs"]
mod tests;
