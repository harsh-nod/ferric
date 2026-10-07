//! Closed, explicit V19 copy ablation over the unchanged combined V17 target.

use super::wave_target_v17_live_contract::{self, Mode};
use std::path::PathBuf;

pub struct Options {
    pub base: wave_target_v17_live_contract::Options,
    pub artifact: PathBuf,
    pub enabled: bool,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut artifact = None;
        let mut enabled = None;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--kv-copy-artifact" => {
                    if artifact.is_some() {
                        return Err("duplicate --kv-copy-artifact".into());
                    }
                    artifact = Some(PathBuf::from(
                        arguments.next().ok_or("missing --kv-copy-artifact value")?,
                    ));
                }
                "--kv-copy-mode" => {
                    if enabled.is_some() {
                        return Err("duplicate --kv-copy-mode".into());
                    }
                    enabled = Some(match arguments.next().as_deref() {
                        Some("baseline") => false,
                        Some("parallel-c1-v19") => true,
                        _ => return Err("kv copy mode must be baseline or parallel-c1-v19".into()),
                    });
                }
                "--live-stdin"
                | "--allow-unauthenticated-machine-code"
                | "--runtime-cache-admission"
                | "--runtime-operational"
                | "--queue-rollover"
                | "--disable-prefix-cache"
                | "--prune-output-head" => forwarded.push(flag),
                _ => {
                    let value = arguments
                        .next()
                        .ok_or_else(|| format!("missing value for {flag}"))?;
                    forwarded.extend([flag, value]);
                }
            }
        }
        let options = Self {
            base: wave_target_v17_live_contract::Options::parse(forwarded.into_iter())?,
            artifact: artifact.ok_or("required --kv-copy-artifact")?,
            enabled: enabled.ok_or("required --kv-copy-mode")?,
        };
        options.validate()?;
        Ok(options)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.base.validate()?;
        if self.base.mode != Mode::Combined || self.artifact.as_os_str().is_empty() {
            return Err("V19 copy requires combined V17 and its explicit separate artifact".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments() -> Vec<String> {
        [
            "--source",
            "source",
            "--target-artifact",
            "target",
            "--target-head-artifact",
            "head",
            "--argmax-artifact",
            "argmax",
            "--worker",
            "worker",
            "--worker-sha256",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--device-unique-id",
            "1",
            "--submission",
            "ordered",
            "--context",
            "8192",
            "--pages",
            "512",
            "--max-batches",
            "1000000",
            "--layer-projection",
            "c1-wave",
            "--wave-target-mode",
            "combined",
            "--attention-artifact",
            "attention",
            "--rmsnorm-artifact",
            "rmsnorm",
            "--kv-copy-artifact",
            "copy",
            "--kv-copy-mode",
            "parallel-c1-v19",
            "--live-stdin",
            "--allow-unauthenticated-machine-code",
            "--runtime-cache-admission",
            "--runtime-operational",
            "--queue-rollover",
            "--disable-prefix-cache",
            "--prune-output-head",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    #[test]
    fn v19_requires_explicit_mode_and_image_and_preserves_option_shaped_paths() {
        for (mode, enabled) in [("baseline", false), ("parallel-c1-v19", true)] {
            let mut args = arguments();
            let index = args.iter().position(|arg| arg == "--kv-copy-mode").unwrap();
            args[index + 1] = mode.into();
            let options = Options::parse(args.into_iter()).unwrap();
            assert_eq!(options.enabled, enabled);
            assert_eq!(options.base.mode, Mode::Combined);
            assert!(!options.base.live.runtime.profile);
            assert_eq!(options.artifact, PathBuf::from("copy"));
        }
        for flag in ["--kv-copy-artifact", "--source", "--attention-artifact"] {
            let mut args = arguments();
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args[index + 1] = "--kv-copy-mode".into();
            assert!(Options::parse(args.into_iter()).is_ok());
        }
        for flag in ["--kv-copy-artifact", "--kv-copy-mode"] {
            let mut args = arguments();
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args.drain(index..index + 2);
            assert!(Options::parse(args.into_iter()).is_err());
            let mut args = arguments();
            args.extend([flag.into(), "baseline".into()]);
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn v19_rejects_unmatched_target_modes_or_broader_runtime_policies() {
        for (flag, value) in [
            ("--kv-copy-mode", "auto"),
            ("--kv-copy-mode", "parallel"),
            ("--kv-copy-artifact", ""),
            ("--wave-target-mode", "baseline"),
            ("--wave-target-mode", "wave-rmsnorm-v15"),
            ("--context", "256"),
            ("--pages", "16"),
            ("--submission", "synchronous"),
        ] {
            let mut args = arguments();
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args[index + 1] = value.into();
            assert!(Options::parse(args.into_iter()).is_err(), "{flag}={value}");
        }
        for flag in [
            "--runtime-profile",
            "--runtime-sequences",
            "--enable-prefix-cache",
        ] {
            let mut args = arguments();
            args.push(flag.into());
            assert!(Options::parse(args.into_iter()).is_err());
        }
        assert!(wave_target_v17_live_contract::Options::parse(arguments().into_iter()).is_err());
    }
}
