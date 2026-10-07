//! Explicit same-image packet grouping over the unchanged combined V17 target.

use super::wave_target_v17_live_contract::{self, Mode};

pub struct Options {
    pub base: wave_target_v17_live_contract::Options,
    pub enabled: bool,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut enabled = None;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--c1-packet-mode" => {
                    if enabled.is_some() {
                        return Err("duplicate --c1-packet-mode".into());
                    }
                    enabled = Some(match arguments.next().as_deref() {
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
            enabled: enabled.ok_or("required --c1-packet-mode")?,
        };
        options.validate()?;
        Ok(options)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.base.validate()?;
        if self.base.mode != Mode::Combined {
            return Err("C1 packet packing requires combined V17".into());
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
            "--c1-packet-mode",
            "packed16-v22",
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
    fn v22_requires_explicit_selector_and_preserves_option_shaped_paths() {
        for (mode, enabled) in [("baseline", false), ("packed16-v22", true)] {
            let mut args = arguments();
            let index = args
                .iter()
                .position(|arg| arg == "--c1-packet-mode")
                .unwrap();
            args[index + 1] = mode.into();
            let options = Options::parse(args.into_iter()).unwrap();
            assert_eq!(options.enabled, enabled);
            assert_eq!(options.base.mode, Mode::Combined);
            assert!(!options.base.live.runtime.profile);
        }
        let mut args = arguments();
        let index = args
            .iter()
            .position(|arg| arg == "--c1-packet-mode")
            .unwrap();
        args.drain(index..index + 2);
        assert!(Options::parse(args.into_iter()).is_err());
        let mut args = arguments();
        args.extend(["--c1-packet-mode".into(), "baseline".into()]);
        assert!(Options::parse(args.into_iter()).is_err());
        for flag in [
            "--source",
            "--target-artifact",
            "--attention-artifact",
            "--rmsnorm-artifact",
        ] {
            let mut args = arguments();
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args[index + 1] = "--c1-packet-mode".into();
            assert!(Options::parse(args.into_iter()).is_ok());
        }
    }

    #[test]
    fn v22_rejects_broader_policies_and_does_not_widen_the_v17_cli() {
        for (flag, value) in [
            ("--c1-packet-mode", "auto"),
            ("--c1-packet-mode", "packed32"),
            ("--wave-target-mode", "baseline"),
            ("--wave-target-mode", "query-hoist-v14"),
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
