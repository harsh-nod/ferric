//! Closed, explicit V28 copy ablation over the unchanged combined V17 target.

use super::wave_target_v17_live_contract::{self, Mode};
use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Independent explicit kernel and packet selectors.
pub struct DecodeOptions {
    pub artifact: PathBuf,
    #[allow(dead_code)] // The diagnostic executable rejects the entire decode bundle.
    pub split: bool,
    #[allow(dead_code)] // The diagnostic executable rejects the entire decode bundle.
    pub packed: bool,
    pub ordered64: bool,
    #[allow(dead_code)] // The diagnostic executable rejects the entire decode bundle.
    pub gemv: Option<(PathBuf, bool)>,
}

pub struct Options {
    pub base: wave_target_v17_live_contract::Options,
    pub artifact: PathBuf,
    pub enabled: bool,
    pub decode: Option<DecodeOptions>,
}

impl Options {
    #[allow(dead_code)] // The packet-tick binary selects the separate parser below.
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        Self::parse_with_packet_ticks(arguments, false)
    }

    #[allow(dead_code)] // Only the dedicated packet-tick diagnostic selects this entry.
    pub(super) fn parse_ordered64_packet_ticks(
        arguments: impl Iterator<Item = String>,
    ) -> Result<Self, String> {
        Self::parse_with_packet_ticks(arguments, true)
    }

    fn parse_with_packet_ticks(
        arguments: impl Iterator<Item = String>,
        packet_ticks: bool,
    ) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut artifact = None;
        let mut enabled = None;
        let mut split_artifact = None;
        let mut split = None;
        let mut packed = None;
        let mut ordered64 = false;
        let mut gemv_artifact = None;
        let mut gemv_enabled = None;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--gemv-artifact" => {
                    if gemv_artifact.is_some() {
                        return Err("duplicate --gemv-artifact".into());
                    }
                    gemv_artifact = Some(PathBuf::from(
                        arguments.next().ok_or("missing --gemv-artifact value")?,
                    ));
                }
                "--gemv-mode" => {
                    if gemv_enabled.is_some() {
                        return Err("duplicate --gemv-mode".into());
                    }
                    gemv_enabled = Some(match arguments.next().as_deref() {
                        Some("baseline") => false,
                        Some("partial-prefetch4-v20") => true,
                        _ => {
                            return Err(
                                "GEMV mode must be baseline or partial-prefetch4-v20".into()
                            );
                        }
                    });
                }
                "--prefill-kv-artifact" => {
                    if artifact.is_some() {
                        return Err("duplicate --prefill-kv-artifact".into());
                    }
                    artifact = Some(PathBuf::from(
                        arguments
                            .next()
                            .ok_or("missing --prefill-kv-artifact value")?,
                    ));
                }
                "--prefill-kv-mode" => {
                    if enabled.is_some() {
                        return Err("duplicate --prefill-kv-mode".into());
                    }
                    enabled = Some(match arguments.next().as_deref() {
                        Some("baseline") => false,
                        Some("parallel-prefill16-v27") => true,
                        _ => {
                            return Err(
                                "kv copy mode must be baseline or parallel-prefill16-v27".into()
                            );
                        }
                    });
                }
                "--split-attention-artifact" => {
                    if split_artifact.is_some() {
                        return Err("duplicate --split-attention-artifact".into());
                    }
                    split_artifact = Some(PathBuf::from(
                        arguments
                            .next()
                            .ok_or("missing --split-attention-artifact value")?,
                    ));
                }
                "--split-attention-mode" => {
                    if split.is_some() {
                        return Err("duplicate --split-attention-mode".into());
                    }
                    split = Some(match arguments.next().as_deref() {
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
                        Some("packed64-v29") => {
                            ordered64 = true;
                            true
                        }
                        _ => {
                            return Err(
                                "C1 packet mode must be baseline, packed16-v22 or packed64-v29"
                                    .into(),
                            );
                        }
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
        let gemv = match (gemv_artifact, gemv_enabled) {
            (None, None) => None,
            (Some(artifact), Some(enabled)) if !artifact.as_os_str().is_empty() => {
                Some((artifact, enabled))
            }
            _ => return Err("partial GEMV requires an explicit image and mode".into()),
        };
        let decode = match (split_artifact, split, packed) {
            (None, None, None) => None,
            (Some(artifact), Some(split), Some(packed)) => Some(DecodeOptions { artifact, split, packed, ordered64, gemv: gemv.clone() }),
            _ => return Err("V28 decode composition requires all three explicit split image, split mode and packet mode options".into()),
        };
        if gemv.is_some() && decode.is_none() {
            return Err("partial GEMV requires the explicit V28 decode composition bundle".into());
        }
        let mut options = Self {
            base: wave_target_v17_live_contract::Options::parse(forwarded.into_iter())?,
            artifact: artifact.ok_or("required --prefill-kv-artifact")?,
            enabled: enabled.ok_or("required --prefill-kv-mode")?,
            decode,
        };
        options.base.live.runtime.ordered64_packet_ticks = packet_ticks;
        options.validate()?;
        Ok(options)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.base.validate()?;
        if self.base.live.runtime.ordered64_packet_ticks
            && (!cfg!(all(feature = "c1-ordered64", feature = "model-timestamps"))
                || !self.decode.as_ref().is_some_and(|decode| decode.ordered64)
                || self.base.live.runtime.profile
                || self.base.live.runtime.ordered64_runtime_counters)
        {
            return Err("packet ticks require the dedicated ordered64 diagnostic build".into());
        }
        if self.base.mode != Mode::Combined || self.artifact.as_os_str().is_empty() {
            return Err("V28 copy requires combined V17 and its explicit separate artifact".into());
        }
        if let Some(decode) = &self.decode
            && decode.ordered64
            && (!cfg!(feature = "c1-ordered64")
                || (cfg!(feature = "model-timestamps")
                    && !self.base.live.runtime.ordered64_packet_ticks)
                || !decode.packed)
        {
            return Err("packed64-v29 requires the explicit c1-ordered64 non-diagnostic build and packed C1 composition".into());
        }
        if let Some(decode) = &self.decode
            && (decode.artifact.as_os_str().is_empty()
                || self.base.live.host_timing.is_some()
                || decode
                    .gemv
                    .as_ref()
                    .is_some_and(|(path, _)| path.as_os_str().is_empty()))
        {
            return Err(
                "V28 decode composition requires an explicit split image and no diagnostic timing"
                    .into(),
            );
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
            "--prefill-kv-artifact",
            "copy",
            "--prefill-kv-mode",
            "parallel-prefill16-v27",
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
    fn v28_requires_explicit_mode_and_image_and_preserves_option_shaped_paths() {
        for (mode, enabled) in [("baseline", false), ("parallel-prefill16-v27", true)] {
            let mut args = arguments();
            let index = args
                .iter()
                .position(|arg| arg == "--prefill-kv-mode")
                .unwrap();
            args[index + 1] = mode.into();
            let options = Options::parse(args.into_iter()).unwrap();
            assert_eq!(options.enabled, enabled);
            assert_eq!(options.base.mode, Mode::Combined);
            assert!(!options.base.live.runtime.profile);
            assert_eq!(options.artifact, PathBuf::from("copy"));
            assert!(options.decode.is_none());
        }
        for flag in ["--prefill-kv-artifact", "--source", "--attention-artifact"] {
            let mut args = arguments();
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args[index + 1] = "--prefill-kv-mode".into();
            assert!(Options::parse(args.into_iter()).is_ok());
        }
        for flag in ["--prefill-kv-artifact", "--prefill-kv-mode"] {
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
    fn ordered64_packet_ticks_are_a_separate_explicit_parser_admission() {
        let mut args = arguments();
        args.extend(
            [
                "--split-attention-artifact",
                "split",
                "--split-attention-mode",
                "split8-v21",
                "--c1-packet-mode",
                "packed64-v29",
                "--gemv-artifact",
                "gemv",
                "--gemv-mode",
                "baseline",
            ]
            .map(str::to_owned),
        );
        if cfg!(all(feature = "c1-ordered64", feature = "model-timestamps")) {
            assert!(Options::parse(args.clone().into_iter()).is_err());
            let selected = Options::parse_ordered64_packet_ticks(args.clone().into_iter()).unwrap();
            assert!(selected.base.live.runtime.ordered64_packet_ticks);
            assert!(!selected.base.live.runtime.profile);
            assert!(selected.decode.as_ref().unwrap().ordered64);
        } else {
            assert!(Options::parse_ordered64_packet_ticks(args.clone().into_iter()).is_err());
        }
        let index = args
            .iter()
            .position(|value| value == "--c1-packet-mode")
            .unwrap();
        args[index + 1] = "packed16-v22".into();
        assert!(Options::parse_ordered64_packet_ticks(args.into_iter()).is_err());
    }

    #[test]
    fn v28_rejects_unmatched_target_modes_or_broader_runtime_policies() {
        for (flag, value) in [
            ("--prefill-kv-mode", "auto"),
            ("--prefill-kv-mode", "parallel"),
            ("--prefill-kv-artifact", ""),
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
        for (flag, value) in [
            ("--c1-packet-mode", "packed16-v22"),
            ("--kv-copy-mode", "parallel-c1-v19"),
            ("--gemv-mode", "prefetch4-v20"),
            ("--split-attention-mode", "split8-v21"),
        ] {
            let mut args = arguments();
            args.extend([flag.into(), value.into()]);
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn v28_composition_requires_a_complete_closed_bundle_for_all_eight_modes() {
        for copy in ["baseline", "parallel-prefill16-v27"] {
            for split in ["baseline", "split8-v21"] {
                for packed in ["baseline", "packed16-v22"] {
                    let mut args = arguments();
                    let at = args
                        .iter()
                        .position(|value| value == "--prefill-kv-mode")
                        .unwrap();
                    args[at + 1] = copy.into();
                    args.extend(
                        [
                            "--split-attention-artifact",
                            "--prefill-kv-mode",
                            "--split-attention-mode",
                            split,
                            "--c1-packet-mode",
                            packed,
                        ]
                        .map(str::to_owned),
                    );
                    let parsed = Options::parse(args.clone().into_iter()).unwrap();
                    assert_eq!(parsed.enabled, copy != "baseline");
                    assert_eq!(
                        parsed.decode,
                        Some(DecodeOptions {
                            artifact: "--prefill-kv-mode".into(),
                            split: split != "baseline",
                            packed: packed != "baseline",
                            ordered64: false,
                            gemv: None
                        })
                    );
                    for flag in [
                        "--split-attention-artifact",
                        "--split-attention-mode",
                        "--c1-packet-mode",
                    ] {
                        let mut missing = args.clone();
                        let at = missing.iter().position(|value| value == flag).unwrap();
                        missing.drain(at..at + 2);
                        assert!(Options::parse(missing.into_iter()).is_err());
                        let mut duplicate = args.clone();
                        duplicate.extend([flag.into(), "baseline".into()]);
                        assert!(Options::parse(duplicate.into_iter()).is_err());
                    }
                    for (flag, value) in [
                        ("--split-attention-artifact", ""),
                        ("--split-attention-mode", "auto"),
                        ("--c1-packet-mode", "auto"),
                    ] {
                        let mut invalid = args.clone();
                        let at = invalid.iter().position(|item| item == flag).unwrap();
                        invalid[at + 1] = value.into();
                        assert!(Options::parse(invalid.into_iter()).is_err());
                    }
                    let mut diagnostic = args;
                    diagnostic.extend(["--host-timing".into(), "timing.json".into()]);
                    assert!(Options::parse(diagnostic.into_iter()).is_err());
                }
            }
        }
    }

    #[test]
    fn ordered64_cli_is_explicit_feature_gated_and_keeps_legacy_defaults() {
        assert!(
            Options::parse(arguments().into_iter())
                .unwrap()
                .decode
                .is_none()
        );
        for packet in ["baseline", "packed16-v22", "packed64-v29"] {
            let mut args = arguments();
            args.extend(
                [
                    "--split-attention-artifact",
                    "split",
                    "--split-attention-mode",
                    "split8-v21",
                    "--c1-packet-mode",
                    packet,
                ]
                .map(str::to_owned),
            );
            let result = Options::parse(args.clone().into_iter());
            if packet == "packed64-v29"
                && (!cfg!(feature = "c1-ordered64") || cfg!(feature = "model-timestamps"))
            {
                assert!(result.is_err());
            } else {
                let decode = result.unwrap().decode.unwrap();
                assert_eq!(decode.ordered64, packet == "packed64-v29");
                assert_eq!(decode.packed, packet != "baseline");
                assert!(decode.gemv.is_none());
            }
            args.extend(["--c1-packet-mode".into(), "packed64-v29".into()]);
            assert!(Options::parse(args.into_iter()).is_err());
        }
        let mut missing = arguments();
        missing.extend(["--c1-packet-mode", "packed64-v29"].map(str::to_owned));
        assert!(Options::parse(missing.into_iter()).is_err());
    }

    #[test]
    fn v28_partial_gemv_requires_both_explicit_options_and_the_composition_bundle() {
        for (mode, enabled) in [("baseline", false), ("partial-prefetch4-v20", true)] {
            let mut args = arguments();
            args.extend(
                [
                    "--split-attention-artifact",
                    "split",
                    "--split-attention-mode",
                    "split8-v21",
                    "--c1-packet-mode",
                    "packed16-v22",
                    "--gemv-artifact",
                    "gemv",
                    "--gemv-mode",
                    mode,
                ]
                .map(str::to_owned),
            );
            let options = Options::parse(args.clone().into_iter()).unwrap();
            assert_eq!(options.decode.unwrap().gemv, Some(("gemv".into(), enabled)));
            for flag in [
                "--gemv-artifact",
                "--gemv-mode",
                "--split-attention-artifact",
                "--split-attention-mode",
                "--c1-packet-mode",
            ] {
                let mut missing = args.clone();
                let at = missing.iter().position(|value| value == flag).unwrap();
                missing.drain(at..at + 2);
                assert!(Options::parse(missing.into_iter()).is_err());
            }
            for flag in ["--gemv-artifact", "--gemv-mode"] {
                let mut duplicate = args.clone();
                duplicate.extend([flag.into(), mode.into()]);
                assert!(Options::parse(duplicate.into_iter()).is_err());
            }
            for (flag, value) in [
                ("--gemv-artifact", ""),
                ("--gemv-mode", "auto"),
                ("--gemv-mode", "prefetch4-v20"),
            ] {
                let mut invalid = args.clone();
                let at = invalid.iter().position(|item| item == flag).unwrap();
                invalid[at + 1] = value.into();
                assert!(Options::parse(invalid.into_iter()).is_err());
            }
            let mut option_shaped_path = args.clone();
            let at = option_shaped_path
                .iter()
                .position(|value| value == "--gemv-artifact")
                .unwrap();
            option_shaped_path[at + 1] = "--gemv-mode".into();
            assert_eq!(
                Options::parse(option_shaped_path.into_iter())
                    .unwrap()
                    .decode
                    .unwrap()
                    .gemv,
                Some(("--gemv-mode".into(), enabled))
            );
            args.extend(["--host-timing".into(), "timing".into()]);
            assert!(Options::parse(args.into_iter()).is_err());
            let mut without_composition = arguments();
            without_composition
                .extend(["--gemv-artifact", "gemv", "--gemv-mode", mode].map(str::to_owned));
            assert!(Options::parse(without_composition.into_iter()).is_err());
        }
    }
}
