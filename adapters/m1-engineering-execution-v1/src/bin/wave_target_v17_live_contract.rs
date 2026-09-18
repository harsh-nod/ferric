//! Closed same-image target ablations; no legacy CLI or default is widened.

use super::layer_c1_wave_live_contract::{self, LayerProjection};
use super::wave_argmax_live_contract::{self, Submission};
pub use ferric_m1_engineering_execution_v1::tp_execution::batched::EngineeringTpWaveTargetModeV17 as Mode;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Options {
    pub live: wave_argmax_live_contract::Options,
    pub mode: Mode,
    pub attention_artifact: PathBuf,
    pub rmsnorm_artifact: PathBuf,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut mode = None;
        let mut attention_artifact = None;
        let mut rmsnorm_artifact = None;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--wave-target-mode" => {
                    if mode.is_some() {
                        return Err("duplicate --wave-target-mode".into());
                    }
                    mode = Some(match arguments.next().as_deref() {
                        Some("baseline") => Mode::Baseline,
                        Some("query-hoist-v14") => Mode::QueryHoist,
                        Some("wave-rmsnorm-v15") => Mode::RmsNorm,
                        Some("combined") => Mode::Combined,
                        _ => return Err("wave target mode must be baseline, query-hoist-v14, wave-rmsnorm-v15 or combined".into()),
                    });
                }
                "--attention-artifact" | "--rmsnorm-artifact" => {
                    let slot = if flag == "--attention-artifact" {
                        &mut attention_artifact
                    } else {
                        &mut rmsnorm_artifact
                    };
                    if slot.is_some() {
                        return Err(format!("duplicate {flag}"));
                    }
                    *slot = Some(PathBuf::from(
                        arguments
                            .next()
                            .ok_or_else(|| format!("missing value for {flag}"))?,
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
                    let value = arguments
                        .next()
                        .ok_or_else(|| format!("missing value for {flag}"))?;
                    forwarded.extend([flag, value]);
                }
            }
        }
        let layer = layer_c1_wave_live_contract::Options::parse(forwarded.into_iter())?;
        if layer.layer_projection != LayerProjection::C1Wave {
            return Err("wave target requires fixed c1-wave layer projection".into());
        }
        let options = Self {
            live: layer.live,
            mode: mode.ok_or("required --wave-target-mode")?,
            attention_artifact: attention_artifact.ok_or("required --attention-artifact")?,
            rmsnorm_artifact: rmsnorm_artifact.ok_or("required --rmsnorm-artifact")?,
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
            || self.attention_artifact.as_os_str().is_empty()
            || self.rmsnorm_artifact.as_os_str().is_empty()
        {
            return Err("wave target requires ordered C1, context8192/pages512 and both explicit candidate images".into());
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) const MODES: [Mode; 4] = [
    Mode::Baseline,
    Mode::QueryHoist,
    Mode::RmsNorm,
    Mode::Combined,
];

#[cfg(test)]
pub(super) fn fixture(mode: Mode) -> Options {
    Options {
        live: wave_argmax_live_contract::fixture(Submission::Ordered),
        mode,
        attention_artifact: "attention".into(),
        rmsnorm_artifact: "rmsnorm".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(mode: &str) -> Vec<String> {
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
            mode,
            "--attention-artifact",
            "attention",
            "--rmsnorm-artifact",
            "rmsnorm",
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
    fn v17_all_modes_load_identical_images_with_the_same_closed_runtime_envelope() {
        let mut baseline = None;
        for mode in MODES {
            let options = Options::parse(arguments(mode.label()).into_iter()).unwrap();
            assert_eq!(options.mode, mode);
            assert_eq!(options.live.context, 8192);
            assert_eq!(options.live.pages, 512);
            assert!(options.live.runtime.ordered_batches);
            assert!(!options.live.runtime.sequences && !options.live.runtime.profile);
            let identity = (
                options.live.source,
                options.live.target_artifact,
                options.live.target_head_artifact,
                options.live.argmax_artifact,
                options.attention_artifact,
                options.rmsnorm_artifact,
                options.live.worker,
                options.live.worker_sha256,
                options.live.device,
            );
            if let Some(expected) = &baseline {
                assert_eq!(&identity, expected);
            } else {
                baseline = Some(identity);
            }
        }
    }

    #[test]
    fn v17_requires_all_options_and_rejects_duplicate_selectors() {
        let args = arguments("combined");
        for (index, flag) in args
            .iter()
            .enumerate()
            .filter(|(_, value)| value.starts_with("--"))
        {
            let mut missing = args.clone();
            missing.remove(index);
            assert!(Options::parse(missing.into_iter()).is_err(), "{flag}");
            let mut duplicate = args.clone();
            duplicate.push(flag.clone());
            assert!(Options::parse(duplicate.into_iter()).is_err(), "{flag}");
        }
        for (flag, value) in [
            ("--wave-target-mode", "baseline"),
            ("--attention-artifact", "other"),
            ("--rmsnorm-artifact", "other"),
        ] {
            let mut args = args.clone();
            args.extend([flag.into(), value.into()]);
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn v17_rejects_profile_drift_unknown_modes_and_implicit_routing() {
        for mode in ["", "auto", "wave-v15", "COMBINED", "query-hoist"] {
            assert!(Options::parse(arguments(mode).into_iter()).is_err());
        }
        for (flag, value) in [
            ("--layer-projection", "mfma"),
            ("--submission", "synchronous"),
            ("--context", "256"),
            ("--pages", "16"),
            ("--attention-artifact", ""),
            ("--rmsnorm-artifact", ""),
        ] {
            let mut args = arguments("combined");
            let index = args.iter().position(|value| value == flag).unwrap();
            args[index + 1] = value.into();
            assert!(Options::parse(args.into_iter()).is_err(), "{flag}");
        }
        for extra in [
            vec!["--attention", "wave"],
            vec!["--rmsnorm-mode", "wave-v15"],
            vec!["--runtime-profile"],
            vec!["--enable-prefix-cache"],
            vec!["--runtime-sequences"],
        ] {
            let mut args = arguments("combined");
            args.extend(extra.into_iter().map(str::to_owned));
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn v17_preserves_option_shaped_path_values_and_rejects_invalid_direct_options() {
        for flag in [
            "--source",
            "--worker",
            "--attention-artifact",
            "--rmsnorm-artifact",
        ] {
            let mut args = arguments("baseline");
            let index = args.iter().position(|value| value == flag).unwrap();
            args[index + 1] = "--wave-target-mode".into();
            assert!(Options::parse(args.into_iter()).is_ok());
        }
        for mode in MODES {
            for mutation in 0..10 {
                let mut options = fixture(mode);
                match mutation {
                    0 => options.live.runtime.ordered_batches = false,
                    1 => options.live.runtime.sequences = true,
                    2 => options.live.runtime.profile = true,
                    3 => options.live.runtime.operational = false,
                    4 => options.live.runtime.shared_full_currentness = true,
                    5 => options.live.context = 256,
                    6 => options.live.pages = 16,
                    7 => options.attention_artifact = PathBuf::new(),
                    8 => options.rmsnorm_artifact = PathBuf::new(),
                    9 => options.live.runtime.cache_admission = false,
                    _ => unreachable!(),
                }
                assert!(options.validate().is_err(), "{mode:?}, mutation {mutation}");
            }
        }
    }
}
