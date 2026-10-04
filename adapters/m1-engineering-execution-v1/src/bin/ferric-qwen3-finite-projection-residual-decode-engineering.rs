//! Explicit additional-image four-forward TF capture, never a legacy-route fallback.
use ferric_m1_engineering_execution_v1::tp_finite_client::prefix_decode::projection::{
    ProjectionDecodeConfig, run_projection_decode,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

fn options(args: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    let values = args.collect::<Vec<_>>();
    if values.len() != 4
        || values[0] != "--request"
        || values[2] != "--observe-projection-residual-decode"
        || values[3] != "--allow-unauthenticated-machine-code"
    {
        return Err("exact --request FILE --observe-projection-residual-decode --allow-unauthenticated-machine-code required".into());
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
    let value = run_projection_decode(ProjectionDecodeConfig::parse(&raw)?, true)?;
    let mut output = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    output.push(b'\n');
    if output.len() > 65536 {
        return Err("projection decode stdout bound".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|e| e.to_string())
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("finite projection residual decode engineering failure: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_decode_cli_refuses_legacy_fallback_or_extra_modes() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-projection-residual-decode",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            PathBuf::from(good[1])
        );
        for size in 0..4 {
            assert!(options(good[..size].iter().map(|s| s.to_string())).is_err());
        }
        for which in 0..4 {
            let mut bad = good;
            bad[which] = if which == 1 {
                "relative"
            } else {
                "--capture-layer-zero"
            };
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        let mut extra = good.to_vec();
        extra.push("--profile");
        assert!(options(extra.into_iter().map(str::to_owned)).is_err());
    }
}
