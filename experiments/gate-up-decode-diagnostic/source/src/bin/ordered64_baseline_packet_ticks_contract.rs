//! New selector preserves the eight-image default; the V19 diagnostic is unchanged.

use super::prefill_kv_copy_v28_live_contract;
use super::wave_target_v17_runner::{self, DecodeComposition, Variant};
use std::path::PathBuf;

pub struct Options {
    pub live: prefill_kv_copy_v28_live_contract::Options,
    pub output: PathBuf,
}

fn split_output(arguments: impl Iterator<Item = String>) -> Result<(PathBuf, Vec<String>), String> {
    let mut arguments = arguments;
    let mut output = None;
    let mut forwarded = Vec::new();
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--ordered64-baseline-packet-ticks" => {
                if output.is_some() {
                    return Err("duplicate baseline packet selector".into());
                }
                let value = arguments.next().ok_or("missing baseline packet sidecar")?;
                if value.is_empty() || value.starts_with('-') {
                    return Err("baseline packet sidecar must be nonempty".into());
                }
                output = Some(PathBuf::from(value));
            }
            "--ordered64-packet-ticks"
            | "--ordered64-kv-copy-artifact"
            | "--ordered64-kv-copy-mode"
            | "--prefill32-pages-mode"
            | "--ordered64-runtime-counters"
            | "--ordered64-host-timing"
            | "--model-timestamps-output"
            | "--host-timing"
            | "--runtime-profile"
            | "--ordered64-active-poll-10ms"
            | "--diagnostic-active-poll-10ms" => {
                return Err(
                    "baseline packet ticks exclude other compositions and diagnostics".into(),
                );
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
    Ok((
        output.ok_or("required --ordered64-baseline-packet-ticks")?,
        forwarded,
    ))
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let (output, forwarded) = split_output(arguments)?;
        let options = Self {
            live: prefill_kv_copy_v28_live_contract::Options::parse_ordered64_packet_ticks(
                forwarded.into_iter(),
            )?,
            output,
        };
        wave_target_v17_runner::validate_ordered64_host_diagnostic(
            &options.live.base,
            options.variant(),
        )?;
        Ok(options)
    }

    pub fn variant(&self) -> Variant<'_> {
        Variant::PrefillKvCopyV28 {
            artifact: &self.live.artifact,
            enabled: self.live.enabled,
            decode: self.live.decode.as_ref().map(|decode| DecodeComposition {
                artifact: &decode.artifact,
                split: decode.split,
                packed: decode.packed,
                ordered64: decode.ordered64,
                gemv: decode
                    .gemv
                    .as_ref()
                    .map(|(path, enabled)| (path.as_path(), *enabled)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(values: &[&str]) -> Result<(PathBuf, Vec<String>), String> {
        split_output(values.iter().map(|value| (*value).to_owned()))
    }

    #[test]
    fn baseline_packet_selector_is_unique_required_and_nonempty() {
        for values in [
            vec![],
            vec!["--ordered64-baseline-packet-ticks"],
            vec!["--ordered64-baseline-packet-ticks", ""],
            vec!["--ordered64-baseline-packet-ticks", "--bad"],
            vec![
                "--ordered64-baseline-packet-ticks",
                "a",
                "--ordered64-baseline-packet-ticks",
                "b",
            ],
        ] {
            assert!(split(&values).is_err());
        }
    }

    #[test]
    fn baseline_packet_rejects_v19_and_other_diagnostics() {
        for option in [
            "--ordered64-packet-ticks",
            "--ordered64-kv-copy-artifact",
            "--ordered64-kv-copy-mode",
            "--prefill32-pages-mode",
            "--ordered64-runtime-counters",
            "--ordered64-host-timing",
            "--model-timestamps-output",
            "--host-timing",
            "--runtime-profile",
            "--ordered64-active-poll-10ms",
            "--diagnostic-active-poll-10ms",
        ] {
            assert!(
                split(&[
                    "--ordered64-baseline-packet-ticks",
                    "raw.json",
                    option,
                    "value"
                ])
                .is_err()
            );
        }
    }

    #[test]
    fn baseline_packet_parser_preserves_values_and_base_flags() {
        let (output, forwarded) = split(&[
            "--source",
            "--ordered64-baseline-packet-ticks",
            "--ordered64-baseline-packet-ticks",
            "raw.json",
            "--live-stdin",
            "--runtime-operational",
        ])
        .unwrap();
        assert_eq!(output, PathBuf::from("raw.json"));
        assert_eq!(
            forwarded,
            [
                "--source",
                "--ordered64-baseline-packet-ticks",
                "--live-stdin",
                "--runtime-operational"
            ]
        );
    }

    #[test]
    fn full_baseline_packet_parser_preserves_exact_eight_image_composition() {
        let arguments = [
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
            "135",
            "--layer-projection",
            "c1-wave",
            "--wave-target-mode",
            "combined",
            "--attention-artifact",
            "attention",
            "--rmsnorm-artifact",
            "rmsnorm",
            "--prefill-kv-artifact",
            "prefill",
            "--prefill-kv-mode",
            "parallel-prefill16-v27",
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
            "--ordered64-baseline-packet-ticks",
            "ticks.json",
            "--live-stdin",
            "--allow-unauthenticated-machine-code",
            "--runtime-cache-admission",
            "--runtime-operational",
            "--queue-rollover",
            "--disable-prefix-cache",
            "--prune-output-head",
        ]
        .map(str::to_owned)
        .to_vec();
        assert!(
            prefill_kv_copy_v28_live_contract::Options::parse(arguments.clone().into_iter())
                .is_err()
        );
        let options = Options::parse(arguments.clone().into_iter()).unwrap();
        assert_eq!(options.output, PathBuf::from("ticks.json"));
        assert!(options.live.base.live.runtime.ordered64_packet_ticks);
        assert!(
            !options.live.base.live.runtime.profile && options.live.base.live.host_timing.is_none()
        );
        for (flag, value) in [
            ("--prefill-kv-mode", "baseline"),
            ("--split-attention-mode", "baseline"),
            ("--gemv-mode", "partial-prefetch4-v20"),
            ("--c1-packet-mode", "baseline"),
        ] {
            let mut wrong = arguments.clone();
            let offset = wrong.iter().position(|item| item == flag).unwrap();
            wrong[offset + 1] = value.to_owned();
            assert!(Options::parse(wrong.into_iter()).is_err());
        }
    }
}
