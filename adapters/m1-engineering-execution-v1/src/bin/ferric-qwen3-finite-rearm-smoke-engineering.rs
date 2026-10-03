//! Explicit fixed rearm smoke engineering command; no queued or two-forward fallback.
use ferric_m1_engineering_execution_v1::tp_finite_client::rearm_smoke::{Config, run};
use std::io::{Read, Write};
use std::path::PathBuf;

fn options(arguments: impl Iterator<Item = String>) -> Result<(PathBuf, bool), String> {
    let mut arguments = arguments;
    let mut request = None;
    let mut consent = false;
    let mut capture_layer0 = false;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--request" if request.is_none() => request = Some(PathBuf::from(arguments.next().ok_or("missing request path")?)),
            "--allow-unauthenticated-machine-code" if !consent => consent = true,
            "--capture-layer0" if !capture_layer0 => capture_layer0 = true,
            _ => return Err("rearm smoke accepts --request FILE, --allow-unauthenticated-machine-code and optional --capture-layer0".into()),
        }
    }
    if !consent {
        return Err("rearm smoke explicit engineering machine-code opt-in required".into());
    }
    let path = request.ok_or("rearm smoke --request is required")?;
    if !path.is_absolute() {
        return Err("rearm smoke request path must be absolute".into());
    }
    Ok((path, capture_layer0))
}
fn execute() -> Result<(), String> {
    let (path, capture_layer0) = options(std::env::args().skip(1))?;
    if path.canonicalize().map_err(|e| e.to_string())? != path
        || !std::fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_file()
    {
        return Err("rearm smoke request must be a canonical regular file".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let config = Config::parse(&bytes)?;
    if config.capture_layer0 != capture_layer0 {
        return Err("rearm smoke CLI/request stage capture differs".into());
    }
    let observation = run(config, true)?;
    // The full controls/captures are separate bounded binary files, never JSON arrays.
    let mut bytes = serde_json::to_vec(&observation).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    if bytes.len() > 65_536 {
        return Err("rearm smoke final stdout bound".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&bytes)
        .map_err(|e| e.to_string())
}
fn main() {
    if let Err(error) = execute() {
        eprintln!("finite-rearm-smoke engineering failure: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(values: &[&str]) -> Result<(PathBuf, bool), String> {
        options(values.iter().map(|v| (*v).to_owned()))
    }
    #[test]
    fn explicit_closed_command_cannot_select_other_profiles_or_counts() {
        assert_eq!(
            parse(&[
                "--request",
                "/task/request.json",
                "--allow-unauthenticated-machine-code"
            ])
            .unwrap(),
            (PathBuf::from("/task/request.json"), false)
        );
        assert_eq!(
            parse(&[
                "--request",
                "/task/request.json",
                "--allow-unauthenticated-machine-code",
                "--capture-layer0"
            ])
            .unwrap(),
            (PathBuf::from("/task/request.json"), true)
        );
        for args in [
            vec!["--request", "/task/request.json"],
            vec!["--allow-unauthenticated-machine-code"],
            vec![
                "--request",
                "relative",
                "--allow-unauthenticated-machine-code",
            ],
            vec![
                "--request",
                "/a",
                "--request",
                "/b",
                "--allow-unauthenticated-machine-code",
            ],
            vec![
                "--request",
                "/a",
                "--allow-unauthenticated-machine-code",
                "--forwards",
                "4",
            ],
            vec![
                "--request",
                "/a",
                "--allow-unauthenticated-machine-code",
                "--input-mode",
                "teacher-forced",
            ],
            vec![
                "--request",
                "/a",
                "--allow-unauthenticated-machine-code",
                "--capture-layer0",
                "--capture-layer0",
            ],
        ] {
            assert!(parse(&args).is_err());
        }
    }
}
