//! Separate diagnostic opt-in over the unchanged ordered C1 live contract.

use super::layer_c1_wave_live_contract;
use super::tp_worker::RuntimeOptions;

#[derive(Clone)]
pub struct Options {
    pub base: layer_c1_wave_live_contract::Options,
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
                    // Preserve option-shaped path values for the unchanged parser.
                    let value = arguments
                        .next()
                        .ok_or_else(|| format!("missing value for {flag}"))?;
                    forwarded.extend([flag, value]);
                }
            }
        }
        let options = Self {
            base: layer_c1_wave_live_contract::Options::parse(forwarded.into_iter())?,
            profiling_requested,
        };
        options.validate()?;
        Ok(options)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.base.validate()?;
        if !self.profiling_requested
            || self.base.live.host_timing.is_none()
            || self.base.live.context != 8192
            || self.base.live.pages != 512
        {
            return Err("runtime diagnostic requires explicit --runtime-profile, --host-timing, context 8192 and 512 pages".into());
        }
        Ok(())
    }

    pub fn worker_runtime(&self) -> Result<RuntimeOptions, String> {
        self.validate()?;
        let mut runtime = self.base.live.runtime;
        runtime.profile = true;
        Ok(runtime)
    }
}

#[cfg(test)]
pub(super) fn fixture(layer: layer_c1_wave_live_contract::LayerProjection) -> Options {
    let mut base = layer_c1_wave_live_contract::fixture(layer);
    base.live.host_timing = Some("host-timing.json".into());
    Options {
        base,
        profiling_requested: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer_c1_wave_live_contract::LayerProjection;
    use crate::wave_argmax_live_contract::Submission;

    fn arguments(layer: &str) -> Vec<String> {
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
            layer,
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
    fn both_diagnostic_modes_derive_only_the_profiling_bit() {
        for layer in [LayerProjection::Mfma, LayerProjection::C1Wave] {
            let options = Options::parse(arguments(layer.label()).into_iter()).unwrap();
            assert_eq!(options.base.layer_projection, layer);
            assert_eq!(options.base.live.context, 8192);
            assert_eq!(options.base.live.pages, 512);
            assert!(!options.base.live.runtime.profile);
            let derived = options.worker_runtime().unwrap();
            assert!(derived.profile);
            let base = options.base.live.runtime;
            assert_eq!(derived.cache_admission, base.cache_admission);
            assert_eq!(derived.operational, base.operational);
            assert_eq!(derived.sequences, base.sequences);
            assert_eq!(derived.ordered_batches, base.ordered_batches);
            assert_eq!(derived.rollover, base.rollover);
            assert_eq!(
                derived.shared_full_currentness,
                base.shared_full_currentness
            );
        }
    }

    #[test]
    fn diagnostic_flags_are_required_and_duplicates_or_unknown_options_reject() {
        for layer in ["mfma", "c1-wave"] {
            for (flag, has_value) in [("--runtime-profile", false), ("--host-timing", true)] {
                let mut missing = arguments(layer);
                let index = missing
                    .iter()
                    .position(|argument| argument == flag)
                    .unwrap();
                missing.remove(index);
                if has_value {
                    missing.remove(index);
                }
                assert!(Options::parse(missing.into_iter()).is_err());
                let mut duplicate = arguments(layer);
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
                let mut invalid = arguments(layer);
                invalid.extend(extra.into_iter().map(str::to_owned));
                assert!(Options::parse(invalid.into_iter()).is_err());
            }
        }
    }

    #[test]
    fn diagnostic_preserves_ordered_only_layer_and_fixed_live_geometry() {
        for (flag, value) in [
            ("--submission", "synchronous"),
            ("--layer-projection", "auto"),
            ("--context", "256"),
            ("--context", "8193"),
            ("--pages", "16"),
            ("--pages", "513"),
            ("--host-timing", ""),
            ("--max-batches", "0"),
        ] {
            let mut invalid = arguments("c1-wave");
            replace(&mut invalid, flag, value);
            assert!(
                Options::parse(invalid.into_iter()).is_err(),
                "{flag} {value}"
            );
        }
    }

    #[test]
    fn profile_shaped_path_values_do_not_enable_diagnostics() {
        let mut args = arguments("mfma");
        args.retain(|value| value != "--runtime-profile");
        replace(&mut args, "--source", "--runtime-profile");
        assert!(Options::parse(args.clone().into_iter()).is_err());
        args.push("--runtime-profile".into());
        let options = Options::parse(args.into_iter()).unwrap();
        assert_eq!(options.base.live.source.to_str(), Some("--runtime-profile"));
    }

    #[test]
    fn old_live_parser_remains_unprofiled_and_rejects_the_diagnostic_flag() {
        for layer in ["mfma", "c1-wave"] {
            let args = arguments(layer);
            assert!(layer_c1_wave_live_contract::Options::parse(args.clone().into_iter()).is_err());
            let ordinary = args
                .into_iter()
                .filter(|value| value != "--runtime-profile");
            let old = layer_c1_wave_live_contract::Options::parse(ordinary).unwrap();
            assert!(!old.live.runtime.profile);
            assert_eq!(old.live.submission, Submission::Ordered);
        }
    }

    #[test]
    fn direct_invalid_options_cannot_derive_a_worker_configuration() {
        for layer in [LayerProjection::Mfma, LayerProjection::C1Wave] {
            for mutation in 0..6 {
                let mut options = fixture(layer);
                match mutation {
                    0 => options.profiling_requested = false,
                    1 => options.base.live.host_timing = None,
                    2 => options.base.live.runtime.profile = true,
                    3 => options.base.live.runtime.ordered_batches = false,
                    4 => options.base.live.runtime.sequences = true,
                    5 => options.base.live.runtime.shared_full_currentness = true,
                    _ => unreachable!(),
                }
                assert!(options.worker_runtime().is_err());
            }
        }
    }
}
