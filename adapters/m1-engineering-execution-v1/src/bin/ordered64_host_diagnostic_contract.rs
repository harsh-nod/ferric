//! Explicit bounded host diagnostic over the unchanged ordered64 composition.

use super::prefill_kv_copy_v28_live_contract;
use super::wave_target_v17_runner::{self, DecodeComposition, Variant};
use std::path::PathBuf;

pub struct Options {
    pub live: prefill_kv_copy_v28_live_contract::Options,
    pub output: PathBuf,
    pub runtime_counters: bool,
    pub active_poll_10ms: bool,
    pub prefill16_ordered: bool,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut output = None;
        let mut runtime_counters = false;
        let mut active_poll_10ms = false;
        let mut prefill16_ordered = false;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--prefill16-ordered64" => {
                    if prefill16_ordered {
                        return Err("duplicate --prefill16-ordered64".into());
                    }
                    prefill16_ordered = true;
                }
                "--ordered64-active-poll-10ms" => {
                    if active_poll_10ms {
                        return Err("duplicate --ordered64-active-poll-10ms".into());
                    }
                    active_poll_10ms = true;
                }
                "--ordered64-runtime-counters" => {
                    if runtime_counters {
                        return Err("duplicate --ordered64-runtime-counters".into());
                    }
                    runtime_counters = true;
                }
                "--ordered64-host-timing" => {
                    if output.is_some() {
                        return Err("duplicate --ordered64-host-timing".into());
                    }
                    output = Some(PathBuf::from(
                        arguments
                            .next()
                            .ok_or("missing --ordered64-host-timing value")?,
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
        let options = Self {
            live: prefill_kv_copy_v28_live_contract::Options::parse(forwarded.into_iter())?,
            output: output.ok_or("required --ordered64-host-timing")?,
            runtime_counters,
            active_poll_10ms,
            prefill16_ordered,
        };
        options.validate()?;
        Ok(options)
    }

    #[must_use]
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

    pub fn validate(&self) -> Result<(), String> {
        self.live.validate()?;
        if self.output.as_os_str().is_empty() {
            return Err("ordered64 host diagnostic requires an explicit output path".into());
        }
        if self.active_poll_10ms && !self.runtime_counters {
            return Err("ordered64 active polling requires explicit runtime counters".into());
        }
        if self.prefill16_ordered && self.active_poll_10ms {
            return Err("prefill scheduling and active polling are separate experiments".into());
        }
        wave_target_v17_runner::validate_ordered64_host_diagnostic(&self.live.base, self.variant())
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
            "copy",
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
            "--ordered64-host-timing",
            "timing.json",
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

    fn replace(args: &mut [String], flag: &str, value: &str) {
        let at = args.iter().position(|arg| arg == flag).unwrap();
        args[at + 1] = value.into();
    }

    #[test]
    fn diagnostic_requires_explicit_selector_and_preserves_unprofiled_runtime() {
        let args = arguments();
        assert!(
            prefill_kv_copy_v28_live_contract::Options::parse(args.clone().into_iter()).is_err()
        );
        let result = Options::parse(args.into_iter());
        if !cfg!(feature = "c1-ordered64") || cfg!(feature = "model-timestamps") {
            assert!(result.is_err());
            return;
        }
        let options = result.unwrap();
        assert_eq!(options.output, PathBuf::from("timing.json"));
        assert!(!options.runtime_counters);
        assert!(!options.active_poll_10ms);
        assert!(!options.prefill16_ordered);
        assert_eq!(options.live.base.live.max_batches, 135);
        assert!(options.live.base.live.host_timing.is_none());
        assert!(!options.live.base.live.runtime.profile);
        assert!(!options.live.base.live.runtime.sequences);
        assert!(options.live.decode.as_ref().unwrap().ordered64);
    }

    #[test]
    fn diagnostic_output_is_required_unique_nonempty_and_consumed_as_a_value() {
        for mutation in 0..4 {
            let mut args = arguments();
            let at = args
                .iter()
                .position(|arg| arg == "--ordered64-host-timing")
                .unwrap();
            match mutation {
                0 => {
                    args.drain(at..at + 2);
                }
                1 => args.extend(["--ordered64-host-timing", "duplicate"].map(str::to_owned)),
                2 => args[at + 1] = String::new(),
                3 => {
                    args.remove(at + 1);
                }
                _ => unreachable!(),
            }
            assert!(Options::parse(args.into_iter()).is_err());
        }
        if !cfg!(feature = "c1-ordered64") || cfg!(feature = "model-timestamps") {
            return;
        }
        for flag in ["--source", "--gemv-artifact", "--ordered64-host-timing"] {
            let mut args = arguments();
            replace(&mut args, flag, "--ordered64-host-timing");
            let options = Options::parse(args.into_iter()).unwrap();
            match flag {
                "--source" => assert_eq!(
                    options.live.base.live.source,
                    PathBuf::from("--ordered64-host-timing")
                ),
                "--gemv-artifact" => assert_eq!(
                    options.live.decode.unwrap().gemv.unwrap().0,
                    PathBuf::from("--ordered64-host-timing")
                ),
                _ => assert_eq!(options.output, PathBuf::from("--ordered64-host-timing")),
            }
        }
    }

    #[test]
    fn prefill16_ordered_selector_is_explicit_unique_and_isolated() {
        let mut args = arguments();
        args.push("--prefill16-ordered64".into());
        assert!(
            prefill_kv_copy_v28_live_contract::Options::parse(args.clone().into_iter()).is_err()
        );
        let parsed = Options::parse(args.clone().into_iter());
        if cfg!(feature = "c1-ordered64") && !cfg!(feature = "model-timestamps") {
            let selected = parsed.unwrap();
            assert!(selected.prefill16_ordered);
            assert!(!selected.active_poll_10ms);
        } else {
            assert!(parsed.is_err());
        }
        let mut combined = args.clone();
        combined.extend(
            [
                "--ordered64-runtime-counters",
                "--ordered64-active-poll-10ms",
            ]
            .map(str::to_owned),
        );
        assert!(Options::parse(combined.into_iter()).is_err());
        args.push("--prefill16-ordered64".into());
        assert!(Options::parse(args.into_iter()).is_err());
    }

    #[test]
    fn prefill16_ordered_flag_as_option_value_does_not_select_policy() {
        if !cfg!(feature = "c1-ordered64") || cfg!(feature = "model-timestamps") {
            return;
        }
        for flag in ["--source", "--ordered64-host-timing"] {
            let mut args = arguments();
            replace(&mut args, flag, "--prefill16-ordered64");
            assert!(!Options::parse(args.into_iter()).unwrap().prefill16_ordered);
        }
    }

    #[test]
    fn runtime_counters_require_the_dedicated_unique_selector() {
        let mut args = arguments();
        args.push("--ordered64-runtime-counters".into());
        assert!(
            prefill_kv_copy_v28_live_contract::Options::parse(args.clone().into_iter()).is_err()
        );
        let selected = Options::parse(args.clone().into_iter());
        if cfg!(feature = "c1-ordered64") && !cfg!(feature = "model-timestamps") {
            let selected = selected.unwrap();
            assert!(selected.runtime_counters);
            assert!(!selected.live.base.live.runtime.profile);
            assert!(!selected.live.base.live.runtime.ordered64_runtime_counters);
        } else {
            assert!(selected.is_err());
        }
        args.push("--ordered64-runtime-counters".into());
        assert!(Options::parse(args.into_iter()).is_err());
    }

    #[test]
    fn active_poll_selector_requires_explicit_counters_and_is_default_off() {
        let mut args = arguments();
        args.push("--ordered64-active-poll-10ms".into());
        assert!(Options::parse(args.clone().into_iter()).is_err());
        args.push("--ordered64-runtime-counters".into());
        assert!(
            prefill_kv_copy_v28_live_contract::Options::parse(args.clone().into_iter()).is_err()
        );
        let parsed = Options::parse(args.clone().into_iter());
        if cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps")
        )) {
            let parsed = parsed.unwrap();
            assert!(parsed.active_poll_10ms && parsed.runtime_counters);
            assert!(!parsed.live.base.live.runtime.profile);
            let control = arguments()
                .into_iter()
                .chain(["--ordered64-runtime-counters".into()]);
            let control = Options::parse(control).unwrap();
            assert!(control.runtime_counters && !control.active_poll_10ms);
        } else {
            assert!(parsed.is_err());
        }
        args.push("--ordered64-active-poll-10ms".into());
        assert!(Options::parse(args.into_iter()).is_err());
    }

    #[test]
    fn active_poll_selector_does_not_consume_values_or_relax_existing_bounds() {
        if cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps")
        )) {
            for flag in ["--source", "--ordered64-host-timing"] {
                let mut args = arguments();
                replace(&mut args, flag, "--ordered64-active-poll-10ms");
                let parsed = Options::parse(args.into_iter()).unwrap();
                assert!(!parsed.active_poll_10ms && !parsed.runtime_counters);
            }
        }
        for (flag, value) in [
            ("--max-batches", "257"),
            ("--submission", "sequential"),
            ("--c1-packet-mode", "packed16-v22"),
            ("--gemv-mode", "partial-prefetch4-v20"),
        ] {
            let mut args = arguments();
            args.extend(
                [
                    "--ordered64-runtime-counters",
                    "--ordered64-active-poll-10ms",
                ]
                .map(str::to_owned),
            );
            replace(&mut args, flag, value);
            assert!(Options::parse(args.into_iter()).is_err());
        }
        for forbidden in [
            "--runtime-profile",
            "--runtime-sequences",
            "--diagnostic-token-program-v1",
            "--diagnostic-active-poll-10ms",
            "--ordered64-packet-ticks",
        ] {
            let mut args = arguments();
            args.extend(
                [
                    "--ordered64-runtime-counters",
                    "--ordered64-active-poll-10ms",
                    forbidden,
                ]
                .map(str::to_owned),
            );
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn runtime_counter_selector_does_not_consume_option_values_or_relax_bounds() {
        if cfg!(feature = "c1-ordered64") && !cfg!(feature = "model-timestamps") {
            let mut args = arguments();
            replace(
                &mut args,
                "--ordered64-host-timing",
                "--ordered64-runtime-counters",
            );
            let parsed = Options::parse(args.into_iter()).unwrap();
            assert!(!parsed.runtime_counters);
            assert_eq!(parsed.output, PathBuf::from("--ordered64-runtime-counters"));
        }
        for (flag, value) in [
            ("--max-batches", "257"),
            ("--c1-packet-mode", "packed16-v22"),
            ("--gemv-mode", "partial-prefetch4-v20"),
        ] {
            let mut args = arguments();
            args.push("--ordered64-runtime-counters".into());
            replace(&mut args, flag, value);
            assert!(Options::parse(args.into_iter()).is_err());
        }
        for extras in [
            vec!["--runtime-profile"],
            vec!["--runtime-sequences"],
            vec!["--host-timing", "legacy.json"],
            vec!["--model-timestamps", "timestamps.json"],
        ] {
            let mut args = arguments();
            args.push("--ordered64-runtime-counters".into());
            args.extend(extras.into_iter().map(str::to_owned));
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn diagnostic_rejects_selector_and_batch_drift_before_output_creation() {
        for (flag, value) in [
            ("--prefill-kv-mode", "baseline"),
            ("--split-attention-mode", "baseline"),
            ("--c1-packet-mode", "packed16-v22"),
            ("--gemv-mode", "partial-prefetch4-v20"),
            ("--max-batches", "0"),
            ("--max-batches", "257"),
            ("--max-batches", "18446744073709551615"),
        ] {
            let mut args = arguments();
            replace(&mut args, flag, value);
            assert!(Options::parse(args.into_iter()).is_err(), "{flag}={value}");
        }
        let mut no_gemv = arguments();
        for flag in ["--gemv-artifact", "--gemv-mode"] {
            let at = no_gemv.iter().position(|arg| arg == flag).unwrap();
            no_gemv.drain(at..at + 2);
        }
        assert!(Options::parse(no_gemv.into_iter()).is_err());
        if cfg!(feature = "c1-ordered64") && !cfg!(feature = "model-timestamps") {
            let mut args = arguments();
            replace(&mut args, "--max-batches", "256");
            assert_eq!(
                Options::parse(args.into_iter())
                    .unwrap()
                    .live
                    .base
                    .live
                    .max_batches,
                256
            );
        }
    }

    #[test]
    fn diagnostic_does_not_grant_legacy_timing_runtime_profile_or_timestamps() {
        for extras in [
            vec!["--host-timing", "legacy.json"],
            vec!["--model-timestamps", "timestamps.json"],
            vec!["--runtime-profile"],
            vec!["--runtime-sequences"],
        ] {
            let mut args = arguments();
            args.extend(extras.into_iter().map(str::to_owned));
            assert!(Options::parse(args.into_iter()).is_err());
        }
        if cfg!(feature = "c1-ordered64") && !cfg!(feature = "model-timestamps") {
            let mut args = arguments();
            let at = args
                .iter()
                .position(|arg| arg == "--ordered64-host-timing")
                .unwrap();
            args.drain(at..at + 2);
            assert!(
                prefill_kv_copy_v28_live_contract::Options::parse(args.clone().into_iter()).is_ok()
            );
            args.extend(["--host-timing", "legacy.json"].map(str::to_owned));
            assert!(prefill_kv_copy_v28_live_contract::Options::parse(args.into_iter()).is_err());
        }
    }
}
