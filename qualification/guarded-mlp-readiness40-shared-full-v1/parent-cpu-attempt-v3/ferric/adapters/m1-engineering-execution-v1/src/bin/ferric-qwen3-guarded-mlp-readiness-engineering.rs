//! Explicit native Readiness40, never full2303 or a four-forward fallback.
use ferric_m1_engineering_execution_v1::tp_finite_client::long::readiness::{
    ReadinessConfig, run, run_causal, run_position5, run_position5_host_timing,
    run_position5_shared_full, run_position5_shared_full_host_timing,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Ordinary,
    Position5,
    Position5HostTiming,
    Position5SharedFull,
    Position5SharedFullHostTiming,
    Causal,
}
fn options(args: impl Iterator<Item = String>) -> Result<(PathBuf, Mode), String> {
    let args = args.collect::<Vec<_>>();
    if args.len() != 4
        || args[0] != "--request"
        || !matches!(
            args[2].as_str(),
            "--observe-guarded-readiness40"
                | "--observe-guarded-readiness40-position5"
                | "--observe-guarded-readiness40-causal-layer0"
                | "--observe-guarded-readiness40-position5-host-timing"
                | "--observe-guarded-readiness40-position5-shared-full"
                | "--observe-guarded-readiness40-position5-shared-full-host-timing"
        )
        || args[3] != "--allow-unauthenticated-machine-code"
    {
        return Err("exact --request FILE --observe-guarded-readiness40 --allow-unauthenticated-machine-code required".into());
    }
    let path = PathBuf::from(&args[1]);
    if !path.is_absolute() {
        return Err("absolute readiness request required".into());
    }
    Ok((
        path,
        match args[2].as_str() {
            "--observe-guarded-readiness40" => Mode::Ordinary,
            "--observe-guarded-readiness40-position5" => Mode::Position5,
            "--observe-guarded-readiness40-position5-host-timing" => Mode::Position5HostTiming,
            "--observe-guarded-readiness40-position5-shared-full" => Mode::Position5SharedFull,
            "--observe-guarded-readiness40-position5-shared-full-host-timing" => {
                Mode::Position5SharedFullHostTiming
            }
            _ => Mode::Causal,
        },
    ))
}
fn execute() -> Result<(), String> {
    let (path, mode) = options(std::env::args().skip(1))?;
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
    let mut raw = match mode {
        Mode::Ordinary => serde_json::to_vec(&run(config, true)?),
        Mode::Position5 => serde_json::to_vec(&run_position5(config, true)?),
        Mode::Causal => serde_json::to_vec(&run_causal(config, true)?),
        Mode::Position5HostTiming => serde_json::to_vec(&run_position5_host_timing(config, true)?),
        Mode::Position5SharedFull => serde_json::to_vec(&run_position5_shared_full(config, true)?),
        Mode::Position5SharedFullHostTiming => {
            serde_json::to_vec(&run_position5_shared_full_host_timing(config, true)?)
        }
    }
    .map_err(|e| e.to_string())?;
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
    fn shared_full_host_timing_parent_cli_is_one_separate_opt_in() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-readiness40-position5-shared-full-host-timing",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap().1,
            Mode::Position5SharedFullHostTiming,
        );
        for (flag, mode) in [
            ("--observe-guarded-readiness40", Mode::Ordinary),
            ("--observe-guarded-readiness40-position5", Mode::Position5),
            ("--observe-guarded-readiness40-causal-layer0", Mode::Causal),
            (
                "--observe-guarded-readiness40-position5-host-timing",
                Mode::Position5HostTiming,
            ),
            (
                "--observe-guarded-readiness40-position5-shared-full",
                Mode::Position5SharedFull,
            ),
        ] {
            let mut old = good;
            old[2] = flag;
            assert_eq!(options(old.into_iter().map(str::to_owned)).unwrap().1, mode);
            let mut both = good.to_vec();
            both.push(flag);
            assert!(options(both.into_iter().map(str::to_owned)).is_err());
        }
        for flag in [
            "--full2303",
            "--observe-guarded-host-shared-full",
            "--shared-full-host-timing",
        ] {
            let mut bad = good;
            bad[2] = flag;
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        for n in 0..good.len() {
            assert!(options(good[..n].iter().map(|s| s.to_string())).is_err());
        }
        let mut bad = good;
        bad[1] = "relative";
        assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        let mut bad = good;
        bad[3] = "--missing-opt-in";
        assert!(options(bad.into_iter().map(str::to_owned)).is_err());
    }
    #[test]
    fn shared_full_parent_selector_is_separate_from_default_timing_and_full() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-readiness40-position5-shared-full",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap().1,
            Mode::Position5SharedFull
        );
        for (flag, mode) in [
            ("--observe-guarded-readiness40", Mode::Ordinary),
            ("--observe-guarded-readiness40-position5", Mode::Position5),
            (
                "--observe-guarded-readiness40-position5-host-timing",
                Mode::Position5HostTiming,
            ),
            ("--observe-guarded-readiness40-causal-layer0", Mode::Causal),
        ] {
            let mut old = good;
            old[2] = flag;
            assert_eq!(options(old.into_iter().map(str::to_owned)).unwrap().1, mode);
        }
        for flag in [
            "--full2303",
            "--observe-guarded-host-shared-full",
            "--observe-guarded-reusable-ar4",
        ] {
            let mut bad = good;
            bad[2] = flag;
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        for n in 0..good.len() {
            assert!(options(good[..n].iter().map(|s| s.to_string())).is_err());
        }
        let mut bad = good.to_vec();
        bad.push("--observe-guarded-readiness40-position5-host-timing");
        assert!(options(bad.into_iter().map(str::to_owned)).is_err());
    }
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
            (PathBuf::from(good[1]), Mode::Ordinary)
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
            (PathBuf::from(good[1]), Mode::Position5)
        );
        let mut bad = good.to_vec();
        bad.push("--observe-guarded-readiness40");
        assert!(options(bad.into_iter().map(str::to_owned)).is_err());
    }
    #[test]
    fn causal_parent_cli_has_a_separate_closed_mode() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-readiness40-causal-layer0",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap().1,
            Mode::Causal
        );
        let mut bad = good.to_vec();
        bad.push("--observe-guarded-readiness40-position5");
        assert!(options(bad.into_iter().map(str::to_owned)).is_err());
    }

    #[test]
    fn readiness_parent_host_timing_has_one_explicit_position5_selector() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-readiness40-position5-host-timing",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap().1,
            Mode::Position5HostTiming,
        );
        for flag in [
            "--observe-guarded-readiness40-position5",
            "--observe-guarded-readiness40",
            "--observe-guarded-readiness40-causal-layer0",
            "--full2303",
        ] {
            let mut bad = good.to_vec();
            bad.push(flag);
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        let mut bad = good;
        bad[3] = "--missing-opt-in";
        assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        assert!(options(good[..3].iter().map(|s| s.to_string())).is_err());
    }
}
