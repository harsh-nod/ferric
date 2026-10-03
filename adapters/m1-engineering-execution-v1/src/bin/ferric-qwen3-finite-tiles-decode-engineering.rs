//! Explicit four-forward typed548 route; no implicit fallback or mode selection.
use ferric_m1_engineering_execution_v1::finite_tiles_decode_wire_v1::InputMode;
use ferric_m1_engineering_execution_v1::tp_finite_client::tiles_decode::{Config, run};
use std::io::{Read, Write};
use std::path::PathBuf;

fn options(arguments: impl Iterator<Item = String>) -> Result<(PathBuf, InputMode), String> {
    let mut args = arguments;
    let mut request = None;
    let mut mode = None;
    let mut consent = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--request" if request.is_none()=>request=Some(PathBuf::from(args.next().ok_or("missing request")?)),
            "--mode" if mode.is_none()=>mode=Some(match args.next().as_deref() {
                Some("teacher-forced")=>InputMode::TeacherForced, Some("autoregressive")=>InputMode::Autoregressive,
                _=>return Err("exact teacher-forced/autoregressive mode required".into())
            }),
            "--allow-unauthenticated-machine-code" if !consent=>consent=true,
            _=>return Err("tiles decode accepts --request FILE --mode MODE --allow-unauthenticated-machine-code only".into()),
        }
    }
    let request = request.ok_or("tiles decode request required")?;
    if !consent || !request.is_absolute() {
        return Err(
            "tiles decode absolute request and explicit engineering opt-in required".into(),
        );
    }
    Ok((request, mode.ok_or("tiles decode mode required")?))
}
fn execute() -> Result<(), String> {
    let (path, mode) = options(std::env::args().skip(1))?;
    if path.canonicalize().map_err(|e| e.to_string())? != path
        || !std::fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_file()
    {
        return Err("tiles decode request requires canonical regular file".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let config = Config::parse(&bytes)?;
    if config.mode != mode {
        return Err("tiles decode CLI/request mode differs".into());
    }
    let observation = run(config, true)?;
    let mut bytes = serde_json::to_vec(&observation).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    if bytes.len() > 65_536 {
        return Err("tiles decode final stdout bound".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&bytes)
        .map_err(|e| e.to_string())
}
fn main() {
    if let Err(error) = execute() {
        eprintln!("finite-tiles-decode engineering failure: {error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<(PathBuf, InputMode), String> {
        options(args.iter().map(|s| s.to_string()))
    }
    #[test]
    fn explicit_mode_optin_and_closed_arguments() {
        for (name, mode) in [
            ("teacher-forced", InputMode::TeacherForced),
            ("autoregressive", InputMode::Autoregressive),
        ] {
            let args = [
                "--request",
                "/task/request.json",
                "--mode",
                name,
                "--allow-unauthenticated-machine-code",
            ];
            assert_eq!(
                parse(&args).unwrap(),
                (PathBuf::from("/task/request.json"), mode)
            );
            for suffix in [
                "--capture-layer0",
                "--forwards",
                "--mode",
                "--allow-unauthenticated-machine-code",
            ] {
                let mut bad = args.to_vec();
                bad.push(suffix);
                assert!(parse(&bad).is_err());
            }
        }
        assert!(
            parse(&[
                "--request",
                "/task/request.json",
                "--allow-unauthenticated-machine-code"
            ])
            .is_err()
        );
        assert!(
            parse(&[
                "--request",
                "/task/request.json",
                "--mode",
                "teacher-forced"
            ])
            .is_err()
        );
        assert!(
            parse(&[
                "--request",
                "relative",
                "--mode",
                "autoregressive",
                "--allow-unauthenticated-machine-code"
            ])
            .is_err()
        );
    }
}
