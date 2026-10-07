//! A closed opt-in wrapper; ordinary V28 parsing and defaults are unchanged.

use super::prefill_kv_copy_v28_live_contract;
use super::wave_target_v17_runner::{DecodeComposition, Ordered64KvCopy, Variant};
use std::path::PathBuf;

pub struct Options {
    pub base: prefill_kv_copy_v28_live_contract::Options,
    artifact: PathBuf,
    enabled: bool,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut artifact = None;
        let mut enabled = None;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--ordered64-kv-copy-artifact" => {
                    if artifact.is_some() {
                        return Err("duplicate --ordered64-kv-copy-artifact".into());
                    }
                    artifact = Some(PathBuf::from(
                        arguments
                            .next()
                            .ok_or("missing --ordered64-kv-copy-artifact value")?,
                    ));
                }
                "--ordered64-kv-copy-mode" => {
                    if enabled.is_some() {
                        return Err("duplicate --ordered64-kv-copy-mode".into());
                    }
                    enabled = Some(match arguments.next().as_deref() {
                        Some("baseline") => false,
                        Some("parallel-c1-v19") => true,
                        _ => {
                            return Err(
                                "ordered64 KV copy mode must be baseline or parallel-c1-v19".into(),
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
        let options = Self {
            base: prefill_kv_copy_v28_live_contract::Options::parse(forwarded.into_iter())?,
            artifact: artifact.ok_or("required --ordered64-kv-copy-artifact")?,
            enabled: enabled.ok_or("required --ordered64-kv-copy-mode")?,
        };
        options
            .selection()
            .validate(&options.base.base, options.variant())?;
        Ok(options)
    }

    pub fn selection(&self) -> Ordered64KvCopy<'_> {
        Ordered64KvCopy {
            artifact: &self.artifact,
            enabled: self.enabled,
            prefill32_pages: None,
        }
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
        let index = args.iter().position(|arg| arg == flag).unwrap();
        args[index + 1] = value.into();
    }

    #[test]
    fn ordered64_copy_cli_feature_matrix_and_old_parser_are_closed() {
        let args = arguments();
        assert!(
            prefill_kv_copy_v28_live_contract::Options::parse(args.clone().into_iter()).is_err()
        );
        assert_eq!(
            Options::parse(args.into_iter()).is_ok(),
            cfg!(all(
                feature = "c1-ordered64",
                not(feature = "model-timestamps")
            ))
        );
    }

    #[test]
    fn ordered64_copy_cli_rejects_missing_duplicate_empty_unknown_and_hybrid_options() {
        for flag in ["--ordered64-kv-copy-artifact", "--ordered64-kv-copy-mode"] {
            let mut args = arguments();
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args.drain(index..index + 2);
            assert!(Options::parse(args.into_iter()).is_err());
            let mut args = arguments();
            args.extend([flag.into(), "baseline".into()]);
            assert!(Options::parse(args.into_iter()).is_err());
            let mut args = arguments();
            replace(&mut args, flag, "");
            assert!(Options::parse(args.into_iter()).is_err());
        }
        for (flag, value) in [
            ("--ordered64-kv-copy-mode", "auto"),
            ("--prefill-kv-mode", "baseline"),
            ("--split-attention-mode", "baseline"),
            ("--c1-packet-mode", "packed16-v22"),
            ("--gemv-mode", "partial-prefetch4-v20"),
        ] {
            let mut args = arguments();
            replace(&mut args, flag, value);
            assert!(Options::parse(args.into_iter()).is_err());
        }
        for flag in [
            "--ordered64-runtime-counters",
            "--ordered64-host-timing",
            "--model-timestamps",
            "--host-timing",
        ] {
            let mut args = arguments();
            args.extend([flag.into(), "output".into()]);
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    #[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
    fn ordered64_copy_cli_preserves_option_shaped_paths_and_has_no_diagnostic_batch_cap() {
        for (mode, enabled) in [("baseline", false), ("parallel-c1-v19", true)] {
            let mut args = arguments();
            replace(&mut args, "--ordered64-kv-copy-mode", mode);
            replace(
                &mut args,
                "--ordered64-kv-copy-artifact",
                "--ordered64-kv-copy-mode",
            );
            let options = Options::parse(args.into_iter()).unwrap();
            assert_eq!(options.enabled, enabled);
            assert_eq!(options.artifact, PathBuf::from("--ordered64-kv-copy-mode"));
            assert_eq!(options.base.base.live.max_batches, 1_000_000);
            assert!(!options.base.base.live.runtime.profile);
            assert!(options.base.base.live.host_timing.is_none());
        }
    }
}
