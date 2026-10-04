//! Explicit one-owner layer-zero capture; no paired or numerical acceptance.
use ferric_m1_engineering_execution_v1::tp_finite_client::prefix_layer::{
    CaptureConfig, run_capture,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

fn options(args: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    let values = args.collect::<Vec<_>>();
    if values.len() != 4
        || values[0] != "--request"
        || values[2] != "--capture-layer-zero"
        || values[3] != "--allow-unauthenticated-machine-code"
    {
        return Err("exact --request FILE --capture-layer-zero --allow-unauthenticated-machine-code required".into());
    }
    let path = PathBuf::from(&values[1]);
    if !path.is_absolute() {
        return Err("absolute request required".into());
    }
    Ok(path)
}

fn execute() -> Result<(), String> {
    let path = options(std::env::args().skip(1))?;
    if path.canonicalize().map_err(|e| e.to_string())? != path
        || !std::fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_file()
    {
        return Err("canonical regular request required".into());
    }
    let mut raw = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    let value = run_capture(CaptureConfig::parse(&raw)?, true)?;
    let mut output = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    output.push(b'\n');
    if output.len() > 65536 {
        return Err("layer capture stdout bound".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|e| e.to_string())
}

fn main() {
    if let Err(e) = execute() {
        eprintln!("finite prefix layer capture engineering failure: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_cli_is_exact_optin_and_rejects_old_or_additional_modes() {
        let good = [
            "--request",
            "/task/request.json",
            "--capture-layer-zero",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.iter().map(|v| v.to_string())).unwrap(),
            PathBuf::from(good[1])
        );
        for n in 0..4 {
            assert!(options(good[..n].iter().map(|v| v.to_string())).is_err());
        }
        for extra in [
            "--mode",
            "--forwards",
            "--kernel-admission",
            "--capture-layer-zero",
        ] {
            let mut bad = good.to_vec();
            bad.push(extra);
            assert!(options(bad.iter().map(|v| v.to_string())).is_err());
        }
        for which in 0..4 {
            let mut bad = good;
            bad[which] = if which == 1 { "relative" } else { "--wrong" };
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        assert!(options([good[0], good[1], good[3]].into_iter().map(str::to_owned)).is_err());
    }
}
