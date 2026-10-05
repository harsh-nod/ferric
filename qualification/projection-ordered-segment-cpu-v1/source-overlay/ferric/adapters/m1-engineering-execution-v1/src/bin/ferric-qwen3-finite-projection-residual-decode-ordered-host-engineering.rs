//! Explicit ordered residual/MLP AR4 shared-full diagnostic; no performance acceptance.
use ferric_m1_engineering_execution_v1::tp_finite_client::prefix_decode::{
    ordered_host, projection_ordered::OrderedDecodeConfig,
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
        || values[3] != "--observe-projection-residual-mlp-ordered"
    {
        return Err(
            "exact --request FILE --allow-unauthenticated-machine-code --observe-projection-residual-mlp-ordered required"
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
    let value = ordered_host::run(OrderedDecodeConfig::parse(&raw)?, true)?;
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
        eprintln!("finite projection decode host diagnostic failure: {error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_ordered_bin_requires_distinct_optin_and_no_policy_options() {
        // The source contract complements the separate default-feature cargo check.
        let source: String = include_str!("../lib.rs")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        for name in [
            "finite_projection_residual_mlp_ordered_wire_v1",
            "projection_residual_mlp_ordered_observation_v1",
        ] {
            let declaration = format!("pubmod {name};").replace(' ', "");
            let guarded = format!(
                "#[cfg(feature=\"tp-batch-engineering\")]#[path=\"../../tp-peer-finite-engineering-worker-v1/src/{name}.rs\"]{declaration}"
            );
            assert_eq!(source.matches(&declaration).count(), 1);
            assert!(
                source.contains(&guarded),
                "ungated ordered parent export: {name}"
            );
        }
        let manifest: toml::Value = toml::from_str(include_str!("../../Cargo.toml")).unwrap();
        let bins = manifest["bin"].as_array().unwrap();
        let selected: Vec<_> = bins
            .iter()
            .filter(|bin| {
                bin["name"].as_str()
                    == Some(
                        "ferric-qwen3-finite-projection-residual-decode-ordered-host-engineering",
                    )
            })
            .collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(
            selected[0]["required-features"].as_array().unwrap(),
            &vec![toml::Value::String("tp-batch-engineering".into())]
        );
        let good = [
            "--request",
            "/tmp/request.json",
            "--allow-unauthenticated-machine-code",
            "--observe-projection-residual-mlp-ordered",
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
            "--observe-projection-residual-mlp-ordered",
        ] {
            let mut bad = good.to_vec();
            bad.push(extra);
            assert!(options(bad.iter().map(|s| s.to_string())).is_err());
        }
        let mut bad = good;
        bad[3] = "--observe-projection-host";
        assert!(options(bad.iter().map(|s| s.to_string())).is_err());
        let mut bad = good;
        bad[1] = "relative";
        assert!(options(bad.iter().map(|s| s.to_string())).is_err());
    }
}
