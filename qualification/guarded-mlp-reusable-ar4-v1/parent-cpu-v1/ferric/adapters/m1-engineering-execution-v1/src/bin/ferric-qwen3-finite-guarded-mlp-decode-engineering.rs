//! Explicit guarded TF4/AR4 observation. No default or old-route fallback.
use ferric_m1_engineering_execution_v1::tp_finite_client::prefix_decode::guarded::{
    GuardedConfig, run_guarded, run_guarded_capture, run_guarded_host, run_guarded_host_shared,
    run_guarded_reuse,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Observe,
    Capture,
    Host,
    HostShared,
    Reuse,
}

fn options(args: impl Iterator<Item = String>) -> Result<(PathBuf, Mode), String> {
    let values = args.collect::<Vec<_>>();
    if values.len() != 4
        || values[0] != "--request"
        || !matches!(
            values[2].as_str(),
            "--observe-guarded-mlp-decode"
                | "--capture-guarded-layer-zero"
                | "--observe-guarded-host"
                | "--observe-guarded-host-shared-currentness"
                | "--observe-guarded-reusable-ar4"
        )
        || values[3] != "--allow-unauthenticated-machine-code"
    {
        return Err("exact --request FILE --observe-guarded-mlp-decode --allow-unauthenticated-machine-code required".into());
    }
    let path = PathBuf::from(&values[1]);
    if !path.is_absolute() {
        return Err("absolute request required".into());
    }
    let mode = match values[2].as_str() {
        "--capture-guarded-layer-zero" => Mode::Capture,
        "--observe-guarded-host" => Mode::Host,
        "--observe-guarded-host-shared-currentness" => Mode::HostShared,
        "--observe-guarded-reusable-ar4" => Mode::Reuse,
        _ => Mode::Observe,
    };
    Ok((path, mode))
}

fn execute() -> Result<(), String> {
    let (path, mode) = options(std::env::args().skip(1))?;
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
    let config = GuardedConfig::parse(&raw)?;
    let value = match mode {
        Mode::Observe => run_guarded(config, true)?,
        Mode::Capture => run_guarded_capture(config, true)?,
        Mode::Host => run_guarded_host(config, true)?,
        Mode::HostShared => run_guarded_host_shared(config, true)?,
        Mode::Reuse => run_guarded_reuse(config, true)?,
    };
    let mut output = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    output.push(b'\n');
    if output.len() > 65536 {
        return Err("guarded decode stdout bound".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|e| e.to_string())
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("finite guarded MLP decode engineering failure: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guarded_reuse_parent_cli_is_explicit_and_cannot_combine_diagnostics() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-reusable-ar4",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), Mode::Reuse)
        );
        for flag in [
            "--observe-guarded-mlp-decode",
            "--capture-guarded-layer-zero",
            "--observe-guarded-host",
            "--observe-guarded-host-shared-currentness",
            "--profile",
        ] {
            let mut extra = good.to_vec();
            extra.push(flag);
            assert!(options(extra.into_iter().map(str::to_owned)).is_err());
        }
        for n in 0..4 {
            assert!(options(good[..n].iter().map(|s| s.to_string())).is_err());
        }
        let mut wrong = good;
        wrong[3] = "--production";
        assert!(options(wrong.into_iter().map(str::to_owned)).is_err());
    }
    #[test]
    fn guarded_decode_parent_cli_refuses_legacy_fallback_or_extra_modes() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-mlp-decode",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), Mode::Observe)
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

    #[test]
    fn guarded_capture_parent_cli_is_separate_and_explicit() {
        let good = [
            "--request",
            "/task/request.json",
            "--capture-guarded-layer-zero",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), Mode::Capture)
        );
        let mut extra = good.to_vec();
        extra.push("--observe-guarded-mlp-decode");
        assert!(options(extra.into_iter().map(str::to_owned)).is_err());
    }
    #[test]
    fn guarded_host_parent_cli_is_explicit_and_exclusive() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-host",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), Mode::Host)
        );
        for flag in [
            "--capture-guarded-layer-zero",
            "--observe-guarded-mlp-decode",
        ] {
            let mut extra = good.to_vec();
            extra.push(flag);
            assert!(options(extra.into_iter().map(str::to_owned)).is_err());
        }
    }

    #[test]
    fn guarded_shared_host_parent_cli_cannot_select_or_combine_other_modes() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-host-shared-currentness",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), Mode::HostShared)
        );
        for flag in [
            "--observe-guarded-host",
            "--capture-guarded-layer-zero",
            "--observe-guarded-mlp-decode",
            "--profile",
            "--cache-kernel-admission",
        ] {
            let mut extra = good.to_vec();
            extra.push(flag);
            assert!(options(extra.into_iter().map(str::to_owned)).is_err());
        }
        let mut unpermitted = good;
        unpermitted[3] = "--profile";
        assert!(options(unpermitted.into_iter().map(str::to_owned)).is_err());
    }
}
