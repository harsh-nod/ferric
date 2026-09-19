//! Explicit combined-only profiling wrapper around the unchanged V17 contract.

use super::wave_target_v17_live_contract::{self, Mode};

pub struct Options {
    pub base: wave_target_v17_live_contract::Options,
    profiling_requested: bool,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut profiling_requested = false;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--runtime-profile" => {
                    if profiling_requested {
                        return Err("duplicate option --runtime-profile".into());
                    }
                    profiling_requested = true;
                }
                "--live-stdin"
                | "--allow-unauthenticated-machine-code"
                | "--runtime-cache-admission"
                | "--runtime-operational"
                | "--queue-rollover"
                | "--disable-prefix-cache"
                | "--prune-output-head" => forwarded.push(flag),
                _ => {
                    // An option-shaped path remains a value, never a profiling grant.
                    let value = arguments
                        .next()
                        .ok_or_else(|| format!("missing value for {flag}"))?;
                    forwarded.extend([flag, value]);
                }
            }
        }
        let options = Self {
            base: wave_target_v17_live_contract::Options::parse(forwarded.into_iter())?,
            profiling_requested,
        };
        options.validate()?;
        Ok(options)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.base.validate()?;
        if !self.profiling_requested
            || self.base.mode != Mode::Combined
            || self.base.live.host_timing.is_none()
        {
            return Err("V17 diagnostic requires explicit --runtime-profile, --host-timing and combined mode".into());
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
            "270",
            "--layer-projection",
            "c1-wave",
            "--wave-target-mode",
            "combined",
            "--attention-artifact",
            "attention",
            "--rmsnorm-artifact",
            "rmsnorm",
            "--host-timing",
            "host-timing.json",
            "--runtime-profile",
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

    fn replace(arguments: &mut [String], flag: &str, value: &str) {
        let index = arguments
            .iter()
            .position(|argument| argument == flag)
            .unwrap();
        arguments[index + 1] = value.into();
    }

    #[test]
    fn diagnostic_keeps_base_unprofiled_and_requires_explicit_combined_opt_in() {
        let options = Options::parse(arguments().into_iter()).unwrap();
        assert_eq!(options.base.mode, Mode::Combined);
        assert!(!options.base.live.runtime.profile);
        assert!(options.base.live.runtime.ordered_batches);
        for (flag, value) in [
            ("--wave-target-mode", "baseline"),
            ("--wave-target-mode", "query-hoist-v14"),
            ("--wave-target-mode", "wave-rmsnorm-v15"),
            ("--context", "256"),
            ("--pages", "16"),
            ("--submission", "synchronous"),
            ("--host-timing", ""),
        ] {
            let mut invalid = arguments();
            replace(&mut invalid, flag, value);
            assert!(
                Options::parse(invalid.into_iter()).is_err(),
                "{flag} {value}"
            );
        }
    }

    #[test]
    fn missing_duplicate_unknown_and_old_live_profile_options_reject() {
        for (flag, has_value) in [("--runtime-profile", false), ("--host-timing", true)] {
            let mut missing = arguments();
            let index = missing.iter().position(|value| value == flag).unwrap();
            missing.remove(index);
            if has_value {
                missing.remove(index);
            }
            assert!(Options::parse(missing.into_iter()).is_err());
            let mut duplicate = arguments();
            duplicate.push(flag.into());
            if has_value {
                duplicate.push("other.json".into());
            }
            assert!(Options::parse(duplicate.into_iter()).is_err());
        }
        for extra in [
            vec!["--runtime-profile", "true"],
            vec!["--benchmark-control"],
            vec!["--unknown", "x"],
        ] {
            let mut invalid = arguments();
            invalid.extend(extra.into_iter().map(str::to_owned));
            assert!(Options::parse(invalid.into_iter()).is_err());
        }
        assert!(wave_target_v17_live_contract::Options::parse(arguments().into_iter()).is_err());
    }

    #[test]
    fn profile_shaped_path_does_not_enable_diagnostic() {
        let mut args = arguments();
        args.retain(|value| value != "--runtime-profile");
        replace(&mut args, "--source", "--runtime-profile");
        assert!(Options::parse(args.clone().into_iter()).is_err());
        args.push("--runtime-profile".into());
        assert_eq!(
            Options::parse(args.into_iter())
                .unwrap()
                .base
                .live
                .source
                .to_str(),
            Some("--runtime-profile")
        );
    }
}
