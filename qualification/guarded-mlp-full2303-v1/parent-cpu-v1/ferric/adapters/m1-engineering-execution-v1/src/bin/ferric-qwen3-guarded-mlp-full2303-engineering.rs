//! Explicit Full2303 engineering entry, not a readiness or AR4 fallback.
use ferric_m1_engineering_execution_v1::tp_finite_client::long::full2303::{Full2303Config, run};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

fn options(args: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    let args = args.collect::<Vec<_>>();
    if args.len() != 4
        || args[0] != "--request"
        || args[2] != "--observe-guarded-full2303"
        || args[3] != "--allow-unauthenticated-machine-code"
    {
        return Err("exact --request FILE --observe-guarded-full2303 --allow-unauthenticated-machine-code required".into());
    }
    let path = PathBuf::from(&args[1]);
    if !path.is_absolute() {
        return Err("absolute full2303 request required".into());
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
        return Err("canonical regular full2303 request required".into());
    }
    let mut raw = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    let value = run(Full2303Config::parse(&raw)?, true)?;
    let mut raw = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    raw.push(b'\n');
    if raw.len() > 128 << 10 {
        return Err("full2303 summary stdout bound".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&raw)
        .map_err(|e| e.to_string())
}
fn main() {
    if let Err(error) = execute() {
        eprintln!("finite guarded full2303 engineering failure: {error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full2303_parent_cli_requires_the_distinct_exact_opt_in() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-full2303",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            PathBuf::from(good[1]),
        );
        for flag in [
            "--observe-guarded-readiness40",
            "--observe-guarded-readiness40-position5",
            "--observe-guarded-readiness40-causal-layer0",
            "--observe-guarded-mlp-decode",
            "--observe-guarded-reusable-ar4",
            "--observe-guarded-host-paired-read",
            "--observe-long",
        ] {
            let mut bad = good;
            bad[2] = flag;
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        for count in 0..good.len() {
            assert!(options(good[..count].iter().map(|s| s.to_string())).is_err());
        }
        let mut extra = good.to_vec();
        extra.push("--observe-guarded-readiness40");
        assert!(options(extra.into_iter().map(str::to_owned)).is_err());
        let mut relative = good;
        relative[1] = "relative";
        assert!(options(relative.into_iter().map(str::to_owned)).is_err());
    }
}
