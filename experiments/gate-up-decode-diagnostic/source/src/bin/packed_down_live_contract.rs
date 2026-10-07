//! Explicit packed-down arguments over the unchanged eight-image V28 parser.

use super::prefill_kv_copy_v28_live_contract;
use super::wave_target_v17_runner::{
    DecodeComposition, PackedDown, Variant, packed_digest_argument,
};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpPackedDownImageIdsR1;
use std::collections::BTreeMap;

const FLAGS: [&str; 7] = [
    "--packed-down-artifact",
    "--packed-down-roster",
    "--packed-down-roster-sha256",
    "--packed-down-hsaco-sha256",
    "--packed-down-manifest-sha256",
    "--packed-down-handoff-sha256",
    "--packed-down-mode",
];

pub struct Options {
    pub base: prefill_kv_copy_v28_live_contract::Options,
    pub selection: PackedDown,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut selected = BTreeMap::new();
        while let Some(flag) = arguments.next() {
            if FLAGS.contains(&flag.as_str()) {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("missing value for {flag}"))?;
                if selected.insert(flag.clone(), value).is_some() {
                    return Err(format!("duplicate {flag}"));
                }
            } else {
                match flag.as_str() {
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
        }
        let get = |flag: &str| -> Result<&str, String> {
            selected
                .get(flag)
                .map(String::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("required nonempty {flag}"))
        };
        let selection = PackedDown {
            artifact: get(FLAGS[0])?.into(),
            roster: get(FLAGS[1])?.into(),
            roster_sha256: packed_digest_argument(get(FLAGS[2])?)?,
            image_ids: EngineeringTpPackedDownImageIdsR1 {
                hsaco: packed_digest_argument(get(FLAGS[3])?)?,
                manifest: packed_digest_argument(get(FLAGS[4])?)?,
                handoff: packed_digest_argument(get(FLAGS[5])?)?,
            },
            enabled: match get(FLAGS[6])? {
                "baseline" => false,
                "packed-down-u32-r1" => true,
                _ => return Err("packed down mode must be baseline or packed-down-u32-r1".into()),
            },
        };
        let options = Self {
            base: prefill_kv_copy_v28_live_contract::Options::parse(forwarded.into_iter())?,
            selection,
        };
        options
            .selection
            .validate(&options.base.base, options.variant())?;
        Ok(options)
    }

    pub fn variant(&self) -> Variant<'_> {
        Variant::PrefillKvCopyV28 {
            artifact: &self.base.artifact,
            enabled: self.base.enabled,
            decode: self.base.decode.as_ref().map(|decode| DecodeComposition {
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

    fn arguments() -> Vec<String> {
        let mut args = [
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
        for (flag, value) in FLAGS.into_iter().zip([
            "/unused/packed-down".into(),
            "/unused/roster.json".into(),
            "11".repeat(32),
            "22".repeat(32),
            "33".repeat(32),
            "44".repeat(32),
            "packed-down-u32-r1".into(),
        ]) {
            args.extend([flag.into(), value]);
        }
        args
    }

    fn replace(args: &mut [String], flag: &str, value: &str) {
        let position = args.iter().position(|item| item == flag).unwrap();
        args[position + 1] = value.into();
    }

    #[test]
    fn packed_down_cli_requires_explicit_selection_without_changing_v28_parser() {
        let args = arguments();
        assert!(prefill_kv_copy_v28_live_contract::Options::parse(args.clone().into_iter()).is_err());
        assert_eq!(
            Options::parse(args.into_iter()).is_ok(),
            cfg!(all(
                feature = "c1-ordered64",
                not(feature = "model-timestamps"),
                not(feature = "c1-token-program")
            ))
        );
    }

    #[test]
    fn packed_down_cli_rejects_missing_duplicate_empty_and_bad_identity_flags() {
        for flag in FLAGS {
            let mut args = arguments();
            let position = args.iter().position(|item| item == flag).unwrap();
            args.drain(position..position + 2);
            assert!(Options::parse(args.into_iter()).is_err());
            let mut args = arguments();
            args.extend([flag.into(), "duplicate".into()]);
            assert!(Options::parse(args.into_iter()).is_err());
            let mut args = arguments();
            replace(&mut args, flag, "");
            assert!(Options::parse(args.into_iter()).is_err());
            assert!(Options::parse([flag.into()].into_iter()).is_err());
        }
        for flag in &FLAGS[2..6] {
            for bad in [
                "0".repeat(64),
                "AA".repeat(32),
                "f".repeat(63),
                "gg".repeat(32),
            ] {
                let mut args = arguments();
                replace(&mut args, flag, &bad);
                assert!(Options::parse(args.into_iter()).is_err());
            }
        }
    }

    #[test]
    fn packed_down_cli_excludes_mixed_composition_and_instrumentation() {
        for (flag, value) in [
            ("--prefill-kv-mode", "baseline"),
            ("--split-attention-mode", "baseline"),
            ("--c1-packet-mode", "packed16-v22"),
            ("--packed-down-mode", "auto"),
            ("--packed-down-artifact", "relative"),
            ("--packed-down-roster", "relative"),
            ("--gemv-mode", "partial-prefetch4-v20"),
            ("--wave-target-mode", "baseline"),
        ] {
            let mut args = arguments();
            replace(&mut args, flag, value);
            assert!(Options::parse(args.into_iter()).is_err());
        }
        for extension in [
            vec![
                "--ordered64-kv-copy-artifact",
                "copy",
                "--ordered64-kv-copy-mode",
                "parallel-c1-v19",
            ],
            vec!["--packed-gate-up-mode", "packed-gate-up-u32-r2"],
            vec!["--prefill32-pages-mode", "parallel-prefill32-two-pages-v27"],
            vec!["--host-timing", "timing"],
            vec!["--model-timestamps", "ticks"],
            vec!["--ordered64-runtime-counters"],
            vec!["--active-poll-10ms"],
            vec!["--prefill16-ordered64"],
            vec!["--packed-host-timing", "timing"],
            vec!["--token-program"],
        ] {
            let mut args = arguments();
            args.extend(extension.into_iter().map(str::to_owned));
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    #[cfg(all(
        feature = "c1-ordered64",
        not(feature = "model-timestamps"),
        not(feature = "c1-token-program")
    ))]
    fn packed_down_cli_both_arms_preserve_pins_and_base_options() {
        let candidate = Options::parse(arguments().into_iter()).unwrap();
        let mut args = arguments();
        replace(&mut args, "--packed-down-mode", "baseline");
        let baseline = Options::parse(args.into_iter()).unwrap();
        assert!(candidate.selection.enabled);
        assert!(!baseline.selection.enabled);
        assert_eq!(candidate.selection.artifact, baseline.selection.artifact);
        assert_eq!(candidate.selection.roster, baseline.selection.roster);
        assert_eq!(
            candidate.selection.roster_sha256,
            baseline.selection.roster_sha256
        );
        assert_eq!(candidate.selection.image_ids, baseline.selection.image_ids);
        assert_eq!(candidate.variant(), baseline.variant());
        assert_eq!(candidate.base.decode, baseline.base.decode);
        assert_eq!(
            candidate.base.base.live.worker_sha256,
            baseline.base.base.live.worker_sha256
        );
        assert_eq!(
            candidate.base.base.live.source,
            baseline.base.base.live.source
        );
    }
}
