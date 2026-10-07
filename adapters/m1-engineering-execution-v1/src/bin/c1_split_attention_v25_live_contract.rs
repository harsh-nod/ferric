//! Closed V25 partial/merge ablation on the unchanged combined V17 target.

use super::wave_target_v17_live_contract::{self, Mode};
use std::path::PathBuf;

pub struct Options {
    pub base: wave_target_v17_live_contract::Options,
    pub artifact: PathBuf,
    pub enabled: bool,
    pub packed: Option<bool>,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut artifact = None;
        let mut enabled = None;
        let mut packed = None;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--split-attention-artifact" => {
                    if artifact.is_some() {
                        return Err("duplicate --split-attention-artifact".into());
                    }
                    artifact = Some(PathBuf::from(
                        arguments
                            .next()
                            .ok_or("missing --split-attention-artifact value")?,
                    ));
                }
                "--split-attention-mode" => {
                    if enabled.is_some() {
                        return Err("duplicate --split-attention-mode".into());
                    }
                    enabled = Some(match arguments.next().as_deref() {
                        Some("baseline") => false,
                        Some("split8-v21") => true,
                        _ => {
                            return Err(
                                "split attention mode must be baseline or split8-v21".into()
                            );
                        }
                    });
                }
                "--c1-packet-mode" => {
                    if packed.is_some() {
                        return Err("duplicate --c1-packet-mode".into());
                    }
                    packed = Some(match arguments.next().as_deref() {
                        Some("baseline") => false,
                        Some("packed16-v22") => true,
                        _ => return Err("C1 packet mode must be baseline or packed16-v22".into()),
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
            artifact: artifact.ok_or("required --split-attention-artifact")?,
            enabled: enabled.ok_or("required --split-attention-mode")?,
            packed,
        };
        options.validate()?;
        Ok(options)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.base.validate()?;
        if self.base.mode != Mode::Combined
            || self.artifact.as_os_str().is_empty()
            || self.base.live.host_timing.is_some()
        {
            return Err("V25 requires combined V17, its explicit separate V21 image and no diagnostic timing".into());
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
            "--split-attention-artifact",
            "split",
            "--split-attention-mode",
            "split8-v21",
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
    fn both_explicit_arms_require_image_and_preserve_option_shaped_paths() {
        for (mode, enabled) in [("baseline", false), ("split8-v21", true)] {
            let mut args = arguments();
            let index = args
                .iter()
                .position(|arg| arg == "--split-attention-mode")
                .unwrap();
            args[index + 1] = mode.into();
            let options = Options::parse(args.into_iter()).unwrap();
            assert_eq!(options.enabled, enabled);
            assert_eq!(options.packed, None);
            assert_eq!(options.artifact, PathBuf::from("split"));
            assert!(!options.base.live.runtime.profile);
        }
        for flag in [
            "--split-attention-artifact",
            "--source",
            "--attention-artifact",
        ] {
            let mut args = arguments();
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args[index + 1] = "--split-attention-mode".into();
            assert!(Options::parse(args.into_iter()).is_ok());
        }
        for flag in ["--split-attention-artifact", "--split-attention-mode"] {
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
    fn broader_or_hybrid_profiles_are_rejected_without_changing_v17_parser() {
        for (flag, value) in [
            ("--split-attention-mode", "auto"),
            ("--split-attention-artifact", ""),
            ("--wave-target-mode", "baseline"),
            ("--context", "256"),
            ("--pages", "16"),
            ("--submission", "synchronous"),
        ] {
            let mut args = arguments();
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args[index + 1] = value.into();
            assert!(Options::parse(args.into_iter()).is_err(), "{flag}={value}");
        }
        for (flag, value) in [
            ("--host-timing", "file"),
            ("--kv-copy-mode", "parallel-c1-v19"),
            ("--gemv-mode", "prefetch4-v20"),
        ] {
            let mut args = arguments();
            args.extend([flag.into(), value.into()]);
            assert!(Options::parse(args.into_iter()).is_err());
        }
        assert!(wave_target_v17_live_contract::Options::parse(arguments().into_iter()).is_err());
    }

    #[test]
    fn packet_composition_is_explicit_and_rejects_unknown_or_duplicate_selection() {
        for (mode, packed) in [("baseline", false), ("packed16-v22", true)] {
            let mut args = arguments();
            args.extend(["--c1-packet-mode".into(), mode.into()]);
            assert_eq!(
                Options::parse(args.clone().into_iter()).unwrap().packed,
                Some(packed)
            );
            args.extend(["--c1-packet-mode".into(), mode.into()]);
            assert!(Options::parse(args.into_iter()).is_err());
        }
        for value in [None, Some("auto"), Some("")] {
            let mut args = arguments();
            args.push("--c1-packet-mode".into());
            args.extend(value.map(str::to_owned));
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }
}
