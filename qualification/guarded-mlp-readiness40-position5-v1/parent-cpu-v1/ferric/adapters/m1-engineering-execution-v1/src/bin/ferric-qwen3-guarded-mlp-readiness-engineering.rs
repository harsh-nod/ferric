//! Explicit native Readiness40, never full2303 or a four-forward fallback.
use ferric_m1_engineering_execution_v1::tp_finite_client::long::readiness::{
    ReadinessConfig, run, run_position5,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
};
fn options(args: impl Iterator<Item = String>) -> Result<(PathBuf, bool), String> {
    let args = args.collect::<Vec<_>>();
    if args.len() != 4
        || args[0] != "--request"
        || !matches!(
            args[2].as_str(),
            "--observe-guarded-readiness40" | "--observe-guarded-readiness40-position5"
        )
        || args[3] != "--allow-unauthenticated-machine-code"
    {
        return Err("exact --request FILE --observe-guarded-readiness40 --allow-unauthenticated-machine-code required".into());
    }
    let path = PathBuf::from(&args[1]);
    if !path.is_absolute() {
        return Err("absolute readiness request required".into());
    }
    Ok((path, args[2] == "--observe-guarded-readiness40-position5"))
}
fn execute() -> Result<(), String> {
    let (path, position5) = options(std::env::args().skip(1))?;
    if path.canonicalize().map_err(|e| e.to_string())? != path
        || !std::fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_file()
    {
        return Err("canonical regular readiness request required".into());
    }
    let mut raw = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    let config = ReadinessConfig::parse(&raw)?;
    let value = if position5 {
        run_position5(config, true)?
    } else {
        run(config, true)?
    };
    let mut raw = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    raw.push(b'\n');
    if raw.len() > 128 << 10 {
        return Err("readiness summary stdout bound".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&raw)
        .map_err(|e| e.to_string())
}
fn main() {
    if let Err(error) = execute() {
        eprintln!("finite guarded readiness engineering failure: {error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_parent_cli_is_explicit_and_has_no_long_or_ar4_fallback() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-readiness40",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), false)
        );
        for flag in [
            "--observe-guarded-mlp-decode",
            "--observe-guarded-reusable-ar4",
            "--observe-guarded-host-paired-read",
            "--full2303",
            "--observe-long",
        ] {
            let mut bad = good;
            bad[2] = flag;
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        for count in 0..good.len() {
            assert!(options(good[..count].iter().map(|s| s.to_string())).is_err());
        }
        let mut bad = good.to_vec();
        bad.push("--full2303");
        assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        let mut bad = good;
        bad[1] = "relative";
        assert!(options(bad.into_iter().map(str::to_owned)).is_err());
    }
    #[test]
    fn position5_parent_cli_requires_distinct_exact_flag() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-readiness40-position5",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), true)
        );
        let mut bad = good.to_vec();
        bad.push("--observe-guarded-readiness40");
        assert!(options(bad.into_iter().map(str::to_owned)).is_err());
    }
}
