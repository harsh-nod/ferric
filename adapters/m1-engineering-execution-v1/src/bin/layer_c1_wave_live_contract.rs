//! Adds only an explicit layer policy to the unchanged ordered live contract.

use super::wave_argmax_live_contract::{self, Submission};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerProjection {
    Mfma,
    C1Wave,
}

impl LayerProjection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Mfma => "mfma",
            Self::C1Wave => "c1-wave",
        }
    }
}

#[derive(Clone)]
pub struct Options {
    pub live: wave_argmax_live_contract::Options,
    pub layer_projection: LayerProjection,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut layer_projection = None;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--layer-projection" => {
                    if layer_projection.is_some() {
                        return Err("duplicate option --layer-projection".into());
                    }
                    layer_projection = Some(match arguments.next().as_deref() {
                        Some("mfma") => LayerProjection::Mfma,
                        Some("c1-wave") => LayerProjection::C1Wave,
                        _ => return Err("layer projection must be mfma or c1-wave".into()),
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
                    // Preserve option-shaped path values for the existing parser.
                    let value = arguments
                        .next()
                        .ok_or_else(|| format!("missing value for {flag}"))?;
                    forwarded.extend([flag, value]);
                }
            }
        }
        let layer_projection = layer_projection.ok_or("required option --layer-projection")?;
        let live = wave_argmax_live_contract::Options::parse(forwarded.into_iter())?;
        let options = Self {
            live,
            layer_projection,
        };
        options.validate()?;
        Ok(options)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.live.validate()?;
        if self.live.submission != Submission::Ordered || !self.live.runtime.ordered_batches {
            return Err("layer live profile requires fixed submission ordered".into());
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn fixture(layer_projection: LayerProjection) -> Options {
    Options {
        live: wave_argmax_live_contract::fixture(Submission::Ordered),
        layer_projection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave_argmax_live_contract::{CHUNK, ROWS};

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
    fn layer_live_modes_preserve_the_closed_8192_common_envelope() {
        let mut snapshots = Vec::new();
        for layer in [LayerProjection::Mfma, LayerProjection::C1Wave] {
            let options = Options::parse(arguments(layer.label()).into_iter()).unwrap();
            assert_eq!(options.layer_projection, layer);
            let live = options.live;
            assert_eq!(live.submission, Submission::Ordered);
            assert_eq!((ROWS, CHUNK, live.context, live.pages), (32, 16, 8192, 512));
            assert_eq!(live.limits().unwrap().page_table_stride(), 512);
            assert_eq!(
                live.limits().unwrap().physical_token_capacity().unwrap(),
                8192
            );
            assert!(
                live.runtime.cache_admission
                    && live.runtime.operational
                    && live.runtime.rollover
                    && live.runtime.ordered_batches
            );
            assert!(
                !live.runtime.sequences
                    && !live.runtime.profile
                    && !live.runtime.shared_full_currentness
            );
            snapshots.push((
                live.source,
                live.target_artifact,
                live.target_head_artifact,
                live.argmax_artifact,
                live.worker,
                live.worker_sha256,
                live.device,
                live.max_batches,
                live.host_timing,
            ));
        }
        assert_eq!(snapshots[0], snapshots[1]);
    }

    #[test]
    fn layer_live_requires_every_flag_and_rejects_duplicate_selectors() {
        let args = arguments("mfma");
        for index in 0..args.len() {
            if !args[index].starts_with("--") {
                continue;
            }
            let mut missing = args.clone();
            missing.remove(index);
            assert!(
                Options::parse(missing.into_iter()).is_err(),
                "{}",
                args[index]
            );
            let mut duplicate = args.clone();
            duplicate.push(args[index].clone());
            assert!(
                Options::parse(duplicate.into_iter()).is_err(),
                "{}",
                args[index]
            );
        }
        for (flag, value) in [
            ("--layer-projection", "c1-wave"),
            ("--submission", "ordered"),
        ] {
            let mut duplicate = args.clone();
            duplicate.extend([flag.into(), value.into()]);
            assert!(Options::parse(duplicate.into_iter()).is_err());
        }
    }

    #[test]
    fn layer_live_rejects_other_submission_arithmetic_and_runtime_profiles() {
        for layer in ["", "auto", "wave", "baseline", "MFMA", "c1-Wave"] {
            assert!(Options::parse(arguments(layer).into_iter()).is_err());
        }
        for submission in ["synchronous", "", "auto", "Ordered"] {
            let mut args = arguments("c1-wave");
            let index = args.iter().position(|flag| flag == "--submission").unwrap();
            args[index + 1] = submission.into();
            assert!(Options::parse(args.into_iter()).is_err());
        }
        for extra in [
            vec!["--runtime-ordered-batches"],
            vec!["--runtime-sequences"],
            vec!["--runtime-profile"],
            vec!["--peer-shared-full-currentness"],
            vec!["--projection", "auto"],
            vec!["--attention", "wave"],
            vec!["--head-precision", "fp32-v8"],
            vec!["--argmax-mode", "wave-v11"],
            vec!["--devices", "1,2"],
            vec!["--kv-pool-profile", "large-v9"],
            vec!["--prefix-cache"],
            vec!["--unknown", "value"],
        ] {
            let mut args = arguments("c1-wave");
            args.extend(extra.into_iter().map(str::to_owned));
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn layer_live_preserves_option_shaped_path_values() {
        for flag in [
            "--source",
            "--target-artifact",
            "--target-head-artifact",
            "--argmax-artifact",
            "--worker",
            "--host-timing",
        ] {
            for value in [
                "--layer-projection",
                "--submission",
                "--live-stdin",
                "--runtime-operational",
            ] {
                let mut args = arguments("c1-wave");
                if flag == "--host-timing" {
                    args.extend([flag.into(), value.into()]);
                } else {
                    let index = args.iter().position(|argument| argument == flag).unwrap();
                    args[index + 1] = value.into();
                }
                let options = Options::parse(args.into_iter()).unwrap();
                assert_eq!(options.layer_projection, LayerProjection::C1Wave);
                let actual = match flag {
                    "--source" => options.live.source,
                    "--target-artifact" => options.live.target_artifact,
                    "--target-head-artifact" => options.live.target_head_artifact,
                    "--argmax-artifact" => options.live.argmax_artifact,
                    "--worker" => options.live.worker,
                    "--host-timing" => options.live.host_timing.unwrap(),
                    _ => unreachable!(),
                };
                assert_eq!(actual, std::path::PathBuf::from(value));
            }
        }
    }

    #[test]
    fn layer_live_leaves_the_old_live_parser_closed_and_modes_unchanged() {
        for layer in ["mfma", "c1-wave"] {
            let args = arguments(layer);
            assert!(wave_argmax_live_contract::Options::parse(args.clone().into_iter()).is_err());
            for submission in [Submission::Synchronous, Submission::Ordered] {
                let mut args = args.clone();
                let index = args
                    .iter()
                    .position(|flag| flag == "--layer-projection")
                    .unwrap();
                args.drain(index..index + 2);
                let index = args.iter().position(|flag| flag == "--submission").unwrap();
                args[index + 1] = submission.label().into();
                let live = wave_argmax_live_contract::Options::parse(args.into_iter()).unwrap();
                assert_eq!(live.submission, submission);
                assert_eq!(live.runtime.ordered_batches, submission.ordered());
                assert_eq!((live.context, live.pages), (8192, 512));
            }
        }
    }

    #[test]
    fn layer_live_direct_options_validate_without_normalizing_the_common_contract() {
        for layer in [LayerProjection::Mfma, LayerProjection::C1Wave] {
            for (context, pages) in [(1, 1), (256, 16), (8192, 16), (8192, 512)] {
                let mut options = fixture(layer);
                options.live.context = context;
                options.live.pages = pages;
                options.validate().unwrap();
                assert_eq!((options.live.context, options.live.pages), (context, pages));
            }
            for mutation in 0..12 {
                let mut options = fixture(layer);
                match mutation {
                    0 => {
                        options.live.submission = Submission::Synchronous;
                        options.live.runtime.ordered_batches = false;
                    }
                    1 => options.live.runtime.ordered_batches = false,
                    2 => options.live.context = 8193,
                    3 => options.live.pages = 513,
                    4 => options.live.max_batches = 0,
                    5 => options.live.device = 0,
                    6 => options.live.worker_sha256 = "0".repeat(64),
                    7 => options.live.runtime.sequences = true,
                    8 => options.live.runtime.profile = true,
                    9 => options.live.runtime.shared_full_currentness = true,
                    10 => options.live.runtime.rollover = false,
                    11 => options.live.source = std::path::PathBuf::new(),
                    _ => unreachable!(),
                }
                assert!(options.validate().is_err(), "mutation {mutation}");
            }
        }
    }
}
