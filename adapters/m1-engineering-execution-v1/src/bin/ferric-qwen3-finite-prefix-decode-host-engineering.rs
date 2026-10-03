//! Explicit Four host diagnostic, unchanged protocol and no performance acceptance.
use ferric_m1_engineering_execution_v1::tp_finite_client::prefix_decode::{
    Config, host_observation,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
};
fn options(args: impl Iterator<Item = String>) -> Result<PathBuf, String> {
    let values = args.collect::<Vec<_>>();
    if values.len() != 4
        || values[0] != "--request"
        || values[2] != "--allow-unauthenticated-machine-code"
        || values[3] != "--observe-host"
    {
        return Err(
            "exact --request FILE --allow-unauthenticated-machine-code --observe-host required"
                .into(),
        );
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
    let value = host_observation::run(Config::parse(&raw)?, true)?;
    let mut output = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    output.push(b'\n');
    if output.len() > 65536 {
        return Err("host diagnostic stdout bound".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|e| e.to_string())
}
fn main() {
    if let Err(error) = execute() {
        eprintln!("finite prefix decode host diagnostic failure: {error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefix_host_bin_requires_distinct_optin_and_no_policy_options() {
        let good = [
            "--request",
            "/tmp/request.json",
            "--allow-unauthenticated-machine-code",
            "--observe-host",
        ];
        assert_eq!(
            options(good.iter().map(|s| s.to_string())).unwrap(),
            PathBuf::from(good[1])
        );
        for n in 0..4 {
            assert!(options(good[..n].iter().map(|s| s.to_string())).is_err());
        }
        for extra in [
            "--cache-kernel-admission",
            "--profile",
            "--shared-currentness",
            "--observe-host",
        ] {
            let mut bad = good.to_vec();
            bad.push(extra);
            assert!(options(bad.iter().map(|s| s.to_string())).is_err());
        }
        let mut bad = good;
        bad[1] = "relative";
        assert!(options(bad.iter().map(|s| s.to_string())).is_err());
    }
}
