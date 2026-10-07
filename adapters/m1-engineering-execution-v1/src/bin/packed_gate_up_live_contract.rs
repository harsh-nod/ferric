//! Explicit packed-image arguments; the existing V28/KV parsers are unchanged.

use super::ordered64_kv_copy_live_contract;
use super::wave_target_v17_runner::{
    PackedGateUp, Variant, packed_digest_argument, validate_packed_host_diagnostic,
};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpPackedBf16ImageIdsR2;
use std::collections::BTreeMap;
use std::path::PathBuf;

const FLAGS: [&str; 7] = [
    "--packed-gate-up-artifact",
    "--packed-gate-up-roster",
    "--packed-gate-up-roster-sha256",
    "--packed-gate-up-hsaco-sha256",
    "--packed-gate-up-manifest-sha256",
    "--packed-gate-up-handoff-sha256",
    "--packed-gate-up-mode",
];

pub struct Options {
    pub base: ordered64_kv_copy_live_contract::Options,
    pub selection: PackedGateUp,
    pub host_timing: Option<PathBuf>,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut selected = BTreeMap::new();
        let mut host_timing = None;
        while let Some(flag) = arguments.next() {
            if flag == "--packed-host-timing" {
                if host_timing.is_some() {
                    return Err("duplicate --packed-host-timing".into());
                }
                host_timing = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or("missing --packed-host-timing value")?,
                ));
            } else if FLAGS.contains(&flag.as_str()) {
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
        let selection = PackedGateUp {
            artifact: get(FLAGS[0])?.into(),
            roster: get(FLAGS[1])?.into(),
            roster_sha256: packed_digest_argument(get(FLAGS[2])?)?,
            image_ids: EngineeringTpPackedBf16ImageIdsR2 {
                hsaco: packed_digest_argument(get(FLAGS[3])?)?,
                manifest: packed_digest_argument(get(FLAGS[4])?)?,
                handoff: packed_digest_argument(get(FLAGS[5])?)?,
            },
            enabled: match get(FLAGS[6])? {
                "baseline" => false,
                "packed-gate-up-u32-r2" => true,
                _ => {
                    return Err(
                        "packed gate/up mode must be baseline or packed-gate-up-u32-r2".into(),
                    );
                }
            },
        };
        let options = Self {
            base: ordered64_kv_copy_live_contract::Options::parse(forwarded.into_iter())?,
            selection,
            host_timing,
        };
        if !options.base.selection().enabled {
            return Err("packed gate/up requires parallel-c1-v19 in both arms".into());
        }
        options
            .selection
            .validate(&options.base.base.base, options.variant())?;
        if let Some(output) = &options.host_timing {
            validate_packed_host_diagnostic(
                &options.base.base.base,
                options.variant(),
                options.base.selection(),
                &options.selection,
                output,
            )?;
        }
        Ok(options)
    }

    pub fn variant(&self) -> Variant<'_> {
        self.base.variant()
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
            "--live-stdin",
            "--allow-unauthenticated-machine-code",
            "--gemv-artifact",
            "gemv",
            "--gemv-mode",
            "baseline",
            "--ordered64-kv-copy-artifact",
            "copy",
            "--ordered64-kv-copy-mode",
            "parallel-c1-v19",
            "--runtime-cache-admission",
            "--runtime-operational",
            "--queue-rollover",
            "--disable-prefix-cache",
            "--prune-output-head",
        ]
        .map(str::to_owned)
        .to_vec();
        for (flag, value) in FLAGS.into_iter().zip([
            "/unused/packed".into(),
            "/unused/roster.json".into(),
            "11".repeat(32),
            "22".repeat(32),
            "33".repeat(32),
            "44".repeat(32),
            "packed-gate-up-u32-r2".into(),
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
    fn packed_cli_requires_explicit_selection_without_changing_old_parser() {
        let args = arguments();
        assert!(ordered64_kv_copy_live_contract::Options::parse(args.clone().into_iter()).is_err());
        assert_eq!(
            Options::parse(args.into_iter()).is_ok(),
            cfg!(all(
                feature = "c1-ordered64",
                not(feature = "model-timestamps")
            ))
        );
    }

    #[test]
    fn packed_cli_rejects_missing_duplicate_empty_and_bad_image_identity_flags() {
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
        }
        for flag in &FLAGS[2..6] {
            for bad in ["0".repeat(64), "AA".repeat(32), "f".repeat(63)] {
                let mut args = arguments();
                replace(&mut args, flag, &bad);
                assert!(Options::parse(args.into_iter()).is_err());
            }
        }
    }

    #[test]
    fn packed_cli_requires_current_kv_composition_and_excludes_instrumentation() {
        for (flag, value) in [
            ("--prefill-kv-mode", "baseline"),
            ("--split-attention-mode", "baseline"),
            ("--c1-packet-mode", "packed16-v22"),
            ("--packed-gate-up-mode", "auto"),
            ("--packed-gate-up-artifact", "relative"),
            ("--packed-gate-up-roster", "relative"),
            ("--ordered64-kv-copy-mode", "baseline"),
            ("--gemv-mode", "partial-prefetch4-v20"),
        ] {
            let mut args = arguments();
            replace(&mut args, flag, value);
            assert!(Options::parse(args.into_iter()).is_err());
        }
        for extension in [
            vec!["--prefill32-pages-mode", "parallel-prefill32-two-pages-v27"],
            vec!["--host-timing", "timing"],
            vec!["--model-timestamps", "ticks"],
            vec!["--ordered64-runtime-counters", "counters"],
        ] {
            let mut args = arguments();
            args.extend(extension.into_iter().map(str::to_owned));
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    #[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
    fn packed_cli_baseline_and_candidate_share_explicit_images_and_unprofiled_runtime() {
        let mut baseline = arguments();
        replace(&mut baseline, "--packed-gate-up-mode", "baseline");
        let baseline = Options::parse(baseline.into_iter()).unwrap();
        let candidate = Options::parse(arguments().into_iter()).unwrap();
        assert!(!baseline.selection.enabled && candidate.selection.enabled);
        assert_eq!(baseline.selection.artifact, candidate.selection.artifact);
        assert_eq!(baseline.selection.roster, candidate.selection.roster);
        assert_eq!(
            baseline.selection.roster_sha256,
            candidate.selection.roster_sha256
        );
        assert_eq!(baseline.selection.image_ids, candidate.selection.image_ids);
        for options in [baseline, candidate] {
            assert!(options.host_timing.is_none());
            assert_eq!(options.base.base.base.live.max_batches, 1_000_000);
            assert!(options.base.base.base.live.host_timing.is_none());
            assert!(!options.base.base.base.live.runtime.profile);
            assert!(options.base.selection().enabled);
            assert!(
                options
                    .base
                    .base
                    .decode
                    .unwrap()
                    .gemv
                    .is_some_and(|(_, enabled)| !enabled)
            );
        }
    }

    #[test]
    fn packed_host_diagnostic_requires_a_separate_bounded_selector() {
        let mut args = arguments();
        replace(&mut args, "--max-batches", "135");
        args.extend(["--packed-host-timing", "/unused/host.json"].map(str::to_owned));
        assert_eq!(
            Options::parse(args.clone().into_iter()).is_ok(),
            cfg!(all(
                feature = "c1-ordered64",
                not(feature = "model-timestamps")
            ))
        );
        for bad in ["", "relative.json"] {
            let mut malformed = args.clone();
            replace(&mut malformed, "--packed-host-timing", bad);
            assert!(Options::parse(malformed.into_iter()).is_err());
        }
        for limit in ["257", "1000000"] {
            let mut unbounded = args.clone();
            replace(&mut unbounded, "--max-batches", limit);
            assert!(Options::parse(unbounded.into_iter()).is_err());
        }
        let mut duplicate = args.clone();
        duplicate.extend(["--packed-host-timing", "/unused/second.json"].map(str::to_owned));
        assert!(Options::parse(duplicate.into_iter()).is_err());
        args.pop();
        assert!(Options::parse(args.into_iter()).is_err());
    }

    #[test]
    #[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
    fn packed_host_diagnostic_preserves_both_modes_and_rejects_other_instrumentation() {
        for mode in ["baseline", "packed-gate-up-u32-r2"] {
            let mut args = arguments();
            replace(&mut args, "--max-batches", "256");
            replace(&mut args, "--packed-gate-up-mode", mode);
            args.extend(["--packed-host-timing", "/unused/host.json"].map(str::to_owned));
            let options = Options::parse(args.clone().into_iter()).unwrap();
            assert_eq!(options.selection.enabled, mode != "baseline");
            assert_eq!(
                options.host_timing,
                Some(PathBuf::from("/unused/host.json"))
            );
            assert!(options.base.base.base.live.host_timing.is_none());
            assert!(!options.base.base.base.live.runtime.profile);
            for extra in [
                vec!["--host-timing", "/unused/legacy.json"],
                vec!["--model-timestamps", "/unused/ticks.json"],
                vec!["--ordered64-runtime-counters"],
                vec!["--runtime-profile"],
            ] {
                let mut combined = args.clone();
                combined.extend(extra.into_iter().map(str::to_owned));
                assert!(Options::parse(combined.into_iter()).is_err());
            }
        }
    }
}
