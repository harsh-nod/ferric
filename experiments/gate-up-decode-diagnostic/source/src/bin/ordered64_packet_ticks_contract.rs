//! Dedicated closed opt-in; legacy model and host diagnostics stay separate.

use super::prefill_kv_copy_v28_live_contract;
use super::wave_target_v17_runner::{self, DecodeComposition, Ordered64KvCopy, Variant};
use std::path::PathBuf;

pub struct Options {
    pub live: prefill_kv_copy_v28_live_contract::Options,
    pub output: PathBuf,
    copy_artifact: PathBuf,
}

fn split_output(
    arguments: impl Iterator<Item = String>,
) -> Result<(PathBuf, PathBuf, Vec<String>), String> {
    let mut arguments = arguments;
    let mut output = None;
    let mut copy_artifact = None;
    let mut copy_mode = false;
    let mut forwarded = Vec::new();
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--ordered64-packet-ticks" => {
                if output.is_some() {
                    return Err("duplicate --ordered64-packet-ticks".into());
                }
                let value = arguments
                    .next()
                    .ok_or("missing --ordered64-packet-ticks value")?;
                if value.is_empty() || value.starts_with('-') {
                    return Err("packet ticks require a nonempty sidecar path".into());
                }
                output = Some(PathBuf::from(value));
            }
            "--ordered64-runtime-counters"
            | "--ordered64-host-timing"
            | "--model-timestamps-output"
            | "--host-timing"
            | "--runtime-profile"
            | "--diagnostic-active-poll-10ms" => {
                return Err("packet ticks exclude other diagnostics and active polling".into());
            }
            "--ordered64-kv-copy-artifact" => {
                if copy_artifact.is_some() {
                    return Err("duplicate --ordered64-kv-copy-artifact".into());
                }
                let value = arguments
                    .next()
                    .ok_or("missing --ordered64-kv-copy-artifact value")?;
                if value.is_empty() {
                    return Err("packet ticks require the actual V19 artifact".into());
                }
                copy_artifact = Some(PathBuf::from(value));
            }
            "--ordered64-kv-copy-mode" => {
                if copy_mode || arguments.next().as_deref() != Some("parallel-c1-v19") {
                    return Err(
                        "packet ticks require one explicit parallel-c1-v19 selection".into(),
                    );
                }
                copy_mode = true;
            }
            "--prefill32-pages-mode" => {
                return Err("packet ticks retain the measured prefill16 composition".into());
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
    if !copy_mode {
        return Err("required --ordered64-kv-copy-mode parallel-c1-v19".into());
    }
    Ok((
        output.ok_or("required --ordered64-packet-ticks")?,
        copy_artifact.ok_or("required --ordered64-kv-copy-artifact")?,
        forwarded,
    ))
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let (output, copy_artifact, forwarded) = split_output(arguments)?;
        let options = Self {
            live: prefill_kv_copy_v28_live_contract::Options::parse_ordered64_packet_ticks(
                forwarded.into_iter(),
            )?,
            output,
            copy_artifact,
        };
        wave_target_v17_runner::validate_ordered64_host_diagnostic(
            &options.live.base,
            options.variant(),
        )?;
        Ok(options)
    }

    pub fn selection(&self) -> Ordered64KvCopy<'_> {
        Ordered64KvCopy {
            artifact: &self.copy_artifact,
            enabled: true,
            prefill32_pages: None,
        }
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

    #[test]
    fn selector_is_required_unique_nonempty_and_value_aware() {
        for values in [
            vec![],
            vec!["--ordered64-packet-ticks"],
            vec!["--ordered64-packet-ticks", ""],
            vec![
                "--ordered64-packet-ticks",
                "a",
                "--ordered64-packet-ticks",
                "b",
            ],
        ] {
            assert!(split_output(values.into_iter().map(str::to_owned)).is_err());
        }
        let (path, copy, remaining) = split_output(
            [
                "--source",
                "--ordered64-packet-ticks",
                "--ordered64-packet-ticks",
                "raw.json",
                "--ordered64-kv-copy-mode",
                "parallel-c1-v19",
                "--ordered64-kv-copy-artifact",
                "--ordered64-kv-copy-mode",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(path, PathBuf::from("raw.json"));
        assert_eq!(copy, PathBuf::from("--ordered64-kv-copy-mode"));
        assert_eq!(remaining, ["--source", "--ordered64-packet-ticks"]);
    }

    #[test]
    fn other_diagnostics_and_active_polling_cannot_be_mixed() {
        for flag in [
            "--ordered64-runtime-counters",
            "--ordered64-host-timing",
            "--model-timestamps-output",
            "--host-timing",
            "--runtime-profile",
            "--diagnostic-active-poll-10ms",
        ] {
            assert!(
                split_output(
                    ["--ordered64-packet-ticks", "raw.json", flag]
                        .into_iter()
                        .map(str::to_owned)
                )
                .is_err()
            );
        }
    }

    #[test]
    fn v19_candidate_is_required_and_wrong_compositions_are_rejected() {
        let valid = [
            "--ordered64-packet-ticks",
            "raw.json",
            "--ordered64-kv-copy-mode",
            "parallel-c1-v19",
            "--ordered64-kv-copy-artifact",
            "copy",
        ];
        assert!(split_output(valid.into_iter().map(str::to_owned)).is_ok());
        for flag in ["--ordered64-kv-copy-mode", "--ordered64-kv-copy-artifact"] {
            let index = valid.iter().position(|value| *value == flag).unwrap();
            let mut missing = valid.to_vec();
            missing.drain(index..index + 2);
            assert!(split_output(missing.into_iter().map(str::to_owned)).is_err());
            let mut duplicate = valid.to_vec();
            duplicate.extend([flag, valid[index + 1]]);
            assert!(split_output(duplicate.into_iter().map(str::to_owned)).is_err());
            let mut empty = valid.to_vec();
            empty[index + 1] = "";
            assert!(split_output(empty.into_iter().map(str::to_owned)).is_err());
        }
        for mode in ["baseline", "auto", "partial-prefetch4-v20"] {
            let mut wrong = valid.to_vec();
            wrong[3] = mode;
            assert!(split_output(wrong.into_iter().map(str::to_owned)).is_err());
        }
        let mut prefill32 = valid.to_vec();
        prefill32.extend(["--prefill32-pages-mode", "parallel-prefill32-two-pages-v27"]);
        assert!(split_output(prefill32.into_iter().map(str::to_owned)).is_err());
    }

    #[test]
    fn full_tick_parser_preserves_the_nine_image_candidate_and_closes_ordinary_parser() {
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
            "--ordered64-kv-copy-artifact",
            "copy",
            "--ordered64-kv-copy-mode",
            "parallel-c1-v19",
            "--ordered64-packet-ticks",
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
        assert_eq!(options.selection().artifact, std::path::Path::new("copy"));
        assert!(options.selection().enabled && options.selection().prefill32_pages.is_none());
        assert!(options.live.base.live.runtime.ordered64_packet_ticks);
        assert!(
            !options.live.base.live.runtime.profile && options.live.base.live.host_timing.is_none()
        );
        for (flag, value) in [
            ("--prefill-kv-mode", "baseline"),
            ("--split-attention-mode", "baseline"),
            ("--gemv-mode", "partial-prefetch4-v20"),
            ("--max-batches", "257"),
        ] {
            let mut wrong = arguments.clone();
            let index = wrong.iter().position(|arg| arg == flag).unwrap();
            wrong[index + 1] = value.into();
            assert!(Options::parse(wrong.into_iter()).is_err());
        }
    }
}
