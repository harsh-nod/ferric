//! Explicit Full2303 engineering entry, not a readiness or AR4 fallback.
use ferric_m1_engineering_execution_v1::tp_finite_client::long::full2303::{
    Full2303Config, run, run_bank_scoped_census, run_bank_scoped_census_tail, run_scoped_warm,
};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Full,
    ScopedWarm,
    BankScopedCensus,
    BankScopedCensusTail,
}

fn options(args: impl Iterator<Item = String>) -> Result<(PathBuf, Mode), String> {
    let args = args.collect::<Vec<_>>();
    if args.len() != 4
        || args[0] != "--request"
        || args[3] != "--allow-unauthenticated-machine-code"
    {
        return Err("exact --request FILE --observe-guarded-full2303 --allow-unauthenticated-machine-code required".into());
    }
    let mode = match args[2].as_str() {
        "--observe-guarded-full2303" => Mode::Full,
        "--observe-guarded-full2303-scoped-warm" => Mode::ScopedWarm,
        "--observe-guarded-full2303-bank-scoped-census-v1" => Mode::BankScopedCensus,
        "--observe-guarded-full2303-bank-scoped-census-tail-v1" => Mode::BankScopedCensusTail,
        _ => return Err("exact ordinary or scoped full2303 selector required".into()),
    };
    let path = PathBuf::from(&args[1]);
    if !path.is_absolute() {
        return Err("absolute full2303 request required".into());
    }
    Ok((path, mode))
}
fn execute() -> Result<(), String> {
    let (path, mode) = options(std::env::args().skip(1))?;
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
    let config = Full2303Config::parse(&raw)?;
    let mut raw = match mode {
        Mode::Full => serde_json::to_vec(&run(config, true)?),
        Mode::ScopedWarm => serde_json::to_vec(&run_scoped_warm(config, true)?),
        Mode::BankScopedCensus => serde_json::to_vec(&run_bank_scoped_census(config, true)?),
        Mode::BankScopedCensusTail => {
            serde_json::to_vec(&run_bank_scoped_census_tail(config, true)?)
        }
    }
    .map_err(|e| e.to_string())?;
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
    fn full2303_bank_scoped_census_tail_parent_cli_requires_one_distinct_selector() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-full2303-bank-scoped-census-tail-v1",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), Mode::BankScopedCensusTail)
        );
        for (flag, expected) in [
            ("--observe-guarded-full2303", Mode::Full),
            ("--observe-guarded-full2303-scoped-warm", Mode::ScopedWarm),
            (
                "--observe-guarded-full2303-bank-scoped-census-v1",
                Mode::BankScopedCensus,
            ),
        ] {
            let mut old = good;
            old[2] = flag;
            assert_eq!(
                options(old.into_iter().map(str::to_owned)).unwrap().1,
                expected
            );
        }
        for flag in [
            "--observe-guarded-readiness40-position5-bank-scoped-warm-host-timing-v2",
            "--observe-guarded-readiness40-position5-bank-scoped-census-tail-host-timing-v4",
            "--observe-guarded-readiness40-position5-bank-scoped-census-host-timing-v3",
            "--engineering-native-guarded-mlp-full2303-bank-scoped-census-tail-v1",
            "--observe-guarded-full2303-bank-scoped-census-tail",
            "--observe-guarded-reusable-ar4",
            "",
        ] {
            let mut bad = good;
            bad[2] = flag;
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        for count in 0..good.len() {
            assert!(options(good[..count].iter().map(|s| s.to_string())).is_err());
        }
        let mut duplicate = good.to_vec();
        duplicate.push(good[2]);
        assert!(options(duplicate.into_iter().map(str::to_owned)).is_err());
        let mut mixed = good.to_vec();
        mixed.push("--observe-guarded-full2303-scoped-warm");
        assert!(options(mixed.into_iter().map(str::to_owned)).is_err());
        let mut relative = good;
        relative[1] = "relative";
        assert!(options(relative.into_iter().map(str::to_owned)).is_err());
        let mut no_opt_in = good;
        no_opt_in[3] = good[2];
        assert!(options(no_opt_in.into_iter().map(str::to_owned)).is_err());
    }
    #[test]
    fn full2303_bank_scoped_census_parent_cli_requires_one_distinct_selector() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-full2303-bank-scoped-census-v1",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), Mode::BankScopedCensus)
        );
        for (flag, expected) in [
            ("--observe-guarded-full2303", Mode::Full),
            ("--observe-guarded-full2303-scoped-warm", Mode::ScopedWarm),
        ] {
            let mut old = good;
            old[2] = flag;
            assert_eq!(
                options(old.into_iter().map(str::to_owned)).unwrap().1,
                expected
            );
        }
        for flag in [
            "--observe-guarded-readiness40-position5-bank-scoped-warm-host-timing-v2",
            "--observe-guarded-readiness40-position5-bank-scoped-census-host-timing-v3",
            "--engineering-native-guarded-mlp-full2303-bank-scoped-census-v1",
            "--observe-guarded-full2303-bank-scoped-census",
            "--observe-guarded-reusable-ar4",
            "",
        ] {
            let mut bad = good;
            bad[2] = flag;
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        for count in 0..good.len() {
            assert!(options(good[..count].iter().map(|s| s.to_string())).is_err());
        }
        let mut duplicate = good.to_vec();
        duplicate.push(good[2]);
        assert!(options(duplicate.into_iter().map(str::to_owned)).is_err());
        let mut mixed = good.to_vec();
        mixed.push("--observe-guarded-full2303-scoped-warm");
        assert!(options(mixed.into_iter().map(str::to_owned)).is_err());
        let mut relative = good;
        relative[1] = "relative";
        assert!(options(relative.into_iter().map(str::to_owned)).is_err());
        let mut no_opt_in = good;
        no_opt_in[3] = good[2];
        assert!(options(no_opt_in.into_iter().map(str::to_owned)).is_err());
    }
    #[test]
    fn full2303_scoped_parent_cli_has_one_closed_explicit_selector() {
        let good = [
            "--request",
            "/task/request.json",
            "--observe-guarded-full2303-scoped-warm",
            "--allow-unauthenticated-machine-code",
        ];
        assert_eq!(
            options(good.into_iter().map(str::to_owned)).unwrap(),
            (PathBuf::from(good[1]), Mode::ScopedWarm)
        );
        let mut ordinary = good;
        ordinary[2] = "--observe-guarded-full2303";
        assert_eq!(
            options(ordinary.into_iter().map(str::to_owned)).unwrap().1,
            Mode::Full
        );
        for count in 0..good.len() {
            assert!(options(good[..count].iter().map(|s| s.to_string())).is_err());
        }
        for flag in [
            "--observe-guarded-readiness40",
            "--observe-guarded-readiness40-position5",
            "--observe-guarded-readiness40-position5-scoped-warm",
            "--observe-guarded-readiness40-position5-scoped-warm-host-timing",
            "--observe-guarded-readiness40-position5-shared-full",
            "--observe-long",
            "--observe-guarded-mlp-decode",
            "",
        ] {
            let mut bad = good;
            bad[2] = flag;
            assert!(options(bad.into_iter().map(str::to_owned)).is_err());
        }
        let mut mixed = good.to_vec();
        mixed.push("--observe-guarded-full2303");
        assert!(options(mixed.into_iter().map(str::to_owned)).is_err());
        let mut relative = good;
        relative[1] = "relative";
        assert!(options(relative.into_iter().map(str::to_owned)).is_err());
        let mut no_opt_in = good;
        no_opt_in[3] = "--observe-guarded-full2303";
        assert!(options(no_opt_in.into_iter().map(str::to_owned)).is_err());
    }
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
            (PathBuf::from(good[1]), Mode::Full),
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
