//! Explicit same-image `RMSNorm` selection on the fixed ordered C1 live profile.

use super::layer_c1_wave_live_contract::{self, LayerProjection};
use super::wave_argmax_live_contract::{self, Submission};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RmsNormMode {
    Baseline,
    WaveV15,
}

impl RmsNormMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::WaveV15 => "wave-v15",
        }
    }
}

#[derive(Clone)]
pub struct Options {
    pub live: wave_argmax_live_contract::Options,
    pub rmsnorm_mode: RmsNormMode,
    pub rmsnorm_artifact: PathBuf,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut rmsnorm_mode = None;
        let mut rmsnorm_artifact = None;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--rmsnorm-mode" => {
                    if rmsnorm_mode.is_some() {
                        return Err("duplicate option --rmsnorm-mode".into());
                    }
                    rmsnorm_mode = Some(match arguments.next().as_deref() {
                        Some("baseline") => RmsNormMode::Baseline,
                        Some("wave-v15") => RmsNormMode::WaveV15,
                        _ => return Err("RMSNorm mode must be baseline or wave-v15".into()),
                    });
                }
                "--rmsnorm-artifact" => {
                    if rmsnorm_artifact.is_some() {
                        return Err("duplicate option --rmsnorm-artifact".into());
                    }
                    rmsnorm_artifact = Some(PathBuf::from(
                        arguments.next().ok_or("missing value for --rmsnorm-artifact")?,
                    ));
                }
                "--live-stdin"
                | "--allow-unauthenticated-machine-code"
                | "--runtime-cache-admission"
                | "--runtime-operational"
                | "--queue-rollover"
                | "--disable-prefix-cache"
                | "--prune-output-head" => forwarded.push(flag),
                _ => {
                    // Preserve option-shaped path values for the existing parser.
                    let value = arguments
                        .next()
                        .ok_or_else(|| format!("missing value for {flag}"))?;
                    forwarded.extend([flag, value]);
                }
            }
        }
        let rmsnorm_mode = rmsnorm_mode.ok_or("required option --rmsnorm-mode")?;
        let rmsnorm_artifact = rmsnorm_artifact.ok_or("required option --rmsnorm-artifact")?;
        let layer = layer_c1_wave_live_contract::Options::parse(forwarded.into_iter())?;
        if layer.layer_projection != LayerProjection::C1Wave {
            return Err("V15 live profile requires fixed layer projection c1-wave".into());
        }
        let options = Self {
            live: layer.live,
            rmsnorm_mode,
            rmsnorm_artifact,
        };
        options.validate()?;
        Ok(options)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.live.validate()?;
        if self.live.submission != Submission::Ordered
            || !self.live.runtime.ordered_batches
            || self.live.context != 8192
            || self.live.pages != 512
            || self.rmsnorm_artifact.as_os_str().is_empty()
        {
            return Err("V15 live requires ordered C1, context8192/pages512 and an explicit RMSNorm image".into());
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn fixture(rmsnorm_mode: RmsNormMode) -> Options {
    Options {
        live: wave_argmax_live_contract::fixture(Submission::Ordered),
        rmsnorm_mode,
        rmsnorm_artifact: "/tmp/fe2o3-engineering-v1/322c87ef9bdbf6d4e45f31b5d389e7664db8beadcb9c3f85dbe2a4758aaa87e9".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave_argmax_live_contract::{CHUNK, ROWS};

    fn arguments(mode: &str) -> Vec<String> {
        [
            "--source", "source",
            "--target-artifact", "target",
            "--target-head-artifact", "head",
            "--argmax-artifact", "argmax",
            "--worker", "worker",
            "--worker-sha256", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--device-unique-id", "1",
            "--submission", "ordered",
            "--context", "8192",
            "--pages", "512",
            "--max-batches", "1000000",
            "--layer-projection", "c1-wave",
            "--rmsnorm-mode", mode,
            "--rmsnorm-artifact", "rmsnorm",
            "--live-stdin",
            "--allow-unauthenticated-machine-code",
            "--runtime-cache-admission",
            "--runtime-operational",
            "--queue-rollover",
            "--disable-prefix-cache",
            "--prune-output-head",
        ].map(str::to_owned).to_vec()
    }

    #[test]
    fn v15_live_modes_preserve_the_same_images_and_fixed_8192_envelope() {
        let mut snapshots = Vec::new();
        for mode in [RmsNormMode::Baseline, RmsNormMode::WaveV15] {
            let options = Options::parse(arguments(mode.label()).into_iter()).unwrap();
            assert_eq!(options.rmsnorm_mode, mode);
            let live = options.live;
            assert_eq!((ROWS, CHUNK, live.context, live.pages), (32, 16, 8192, 512));
            assert_eq!(live.submission, Submission::Ordered);
            assert!(live.runtime.cache_admission && live.runtime.operational
                && live.runtime.rollover && live.runtime.ordered_batches);
            assert!(!live.runtime.sequences && !live.runtime.profile
                && !live.runtime.shared_full_currentness);
            assert_eq!(live.limits().unwrap().physical_token_capacity().unwrap(), 8192);
            snapshots.push((live.source, live.target_artifact, live.target_head_artifact,
                live.argmax_artifact, options.rmsnorm_artifact, live.worker,
                live.worker_sha256, live.device, live.max_batches, live.host_timing));
        }
        assert_eq!(snapshots[0], snapshots[1]);
    }

    #[test]
    fn v15_live_requires_explicit_modes_images_and_every_common_flag() {
        let args = arguments("baseline");
        for index in 0..args.len() {
            if !args[index].starts_with("--") { continue; }
            let mut missing = args.clone();
            missing.remove(index);
            assert!(Options::parse(missing.into_iter()).is_err(), "{}", args[index]);
            let mut duplicate = args.clone();
            duplicate.push(args[index].clone());
            assert!(Options::parse(duplicate.into_iter()).is_err(), "{}", args[index]);
        }
        for (flag, value) in [("--rmsnorm-mode", "wave-v15"),
            ("--rmsnorm-artifact", "another"), ("--layer-projection", "c1-wave")] {
            let mut duplicate = args.clone();
            duplicate.extend([flag.into(), value.into()]);
            assert!(Options::parse(duplicate.into_iter()).is_err());
        }
    }

    #[test]
    fn v15_live_rejects_other_arithmetic_geometry_and_runtime_profiles() {
        for (flag, value) in [
            ("--rmsnorm-mode", ""), ("--rmsnorm-mode", "wave"),
            ("--rmsnorm-mode", "Wave-v15"), ("--rmsnorm-mode", "auto"),
            ("--rmsnorm-artifact", ""), ("--layer-projection", "mfma"),
            ("--submission", "synchronous"), ("--context", "256"),
            ("--context", "8193"), ("--pages", "16"), ("--pages", "513"),
        ] {
            let mut args = arguments("wave-v15");
            let index = args.iter().position(|argument| argument == flag).unwrap();
            args[index + 1] = value.into();
            assert!(Options::parse(args.into_iter()).is_err(), "{flag} {value}");
        }
        for extra in [vec!["--runtime-ordered-batches"], vec!["--runtime-sequences"],
            vec!["--runtime-profile"], vec!["--peer-shared-full-currentness"],
            vec!["--attention", "query-hoist-v14"], vec!["--argmax-mode", "wave-v11"],
            vec!["--query-hoist-artifact", "v14"], vec!["--prefix-cache"]] {
            let mut args = arguments("wave-v15");
            args.extend(extra.into_iter().map(str::to_owned));
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn v15_live_preserves_option_shaped_path_values() {
        for flag in ["--source", "--target-artifact", "--target-head-artifact",
            "--argmax-artifact", "--rmsnorm-artifact", "--worker", "--host-timing"] {
            for value in ["--rmsnorm-mode", "--rmsnorm-artifact", "--layer-projection", "--live-stdin"] {
                let mut args = arguments("wave-v15");
                if flag == "--host-timing" {
                    args.extend([flag.into(), value.into()]);
                } else {
                    let index = args.iter().position(|argument| argument == flag).unwrap();
                    args[index + 1] = value.into();
                }
                let options = Options::parse(args.into_iter()).unwrap();
                assert_eq!(options.rmsnorm_mode, RmsNormMode::WaveV15);
                let actual = match flag {
                    "--source" => options.live.source,
                    "--target-artifact" => options.live.target_artifact,
                    "--target-head-artifact" => options.live.target_head_artifact,
                    "--argmax-artifact" => options.live.argmax_artifact,
                    "--rmsnorm-artifact" => options.rmsnorm_artifact,
                    "--worker" => options.live.worker,
                    "--host-timing" => options.live.host_timing.unwrap(),
                    _ => unreachable!(),
                };
                assert_eq!(actual, PathBuf::from(value));
            }
        }
    }

    #[test]
    fn v15_live_keeps_both_older_live_parsers_closed_and_unchanged() {
        for mode in ["baseline", "wave-v15"] {
            let args = arguments(mode);
            assert!(layer_c1_wave_live_contract::Options::parse(args.clone().into_iter()).is_err());
            assert!(wave_argmax_live_contract::Options::parse(args.clone().into_iter()).is_err());
            let mut old = args;
            for flag in ["--rmsnorm-mode", "--rmsnorm-artifact"] {
                let index = old.iter().position(|argument| argument == flag).unwrap();
                old.drain(index..index + 2);
            }
            for layer in ["mfma", "c1-wave"] {
                let mut old = old.clone();
                let index = old.iter().position(|argument| argument == "--layer-projection").unwrap();
                old[index + 1] = layer.into();
                let parsed = layer_c1_wave_live_contract::Options::parse(old.into_iter()).unwrap();
                assert_eq!(parsed.layer_projection.label(), layer);
                assert_eq!((parsed.live.context, parsed.live.pages), (8192, 512));
            }
        }
    }
}
