//! Explicit chunk32 policy over the independently selected nine-image composition.

use super::ordered64_kv_copy_live_contract;
use super::wave_target_v17_runner::Ordered64KvCopy;

pub struct Options {
    pub base: ordered64_kv_copy_live_contract::Options,
    enabled: bool,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut forwarded = Vec::new();
        let mut enabled = None;
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--prefill32-pages-mode" => {
                    if enabled.is_some() {
                        return Err("duplicate --prefill32-pages-mode".into());
                    }
                    enabled = Some(match arguments.next().as_deref() {
                        Some("baseline") => false,
                        Some("parallel-prefill32-two-pages-v27") => true,
                        _ => return Err("prefill32 pages mode must be baseline or parallel-prefill32-two-pages-v27".into()),
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
            base: ordered64_kv_copy_live_contract::Options::parse(forwarded.into_iter())?,
            enabled: enabled.ok_or("required --prefill32-pages-mode")?,
        };
        options
            .selection()
            .validate(&options.base.base.base, options.base.variant())?;
        Ok(options)
    }

    pub fn selection(&self) -> Ordered64KvCopy<'_> {
        Ordered64KvCopy {
            prefill32_pages: Some(self.enabled),
            ..self.base.selection()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments() -> Vec<String> {
        let mut args: Vec<String> = "--source source --target-artifact target
            --target-head-artifact head --argmax-artifact argmax --worker worker
            --device-unique-id 1 --submission ordered --context 8192 --pages 512
            --max-batches 1000000 --layer-projection c1-wave --wave-target-mode combined
            --attention-artifact attention --rmsnorm-artifact rmsnorm
            --prefill-kv-artifact prefill --prefill-kv-mode parallel-prefill16-v27
            --split-attention-artifact split --split-attention-mode split8-v21
            --c1-packet-mode packed64-v29 --gemv-artifact gemv --gemv-mode baseline
            --ordered64-kv-copy-artifact copy --ordered64-kv-copy-mode parallel-c1-v19
            --prefill32-pages-mode parallel-prefill32-two-pages-v27
            --live-stdin --allow-unauthenticated-machine-code --runtime-cache-admission
            --runtime-operational --queue-rollover --disable-prefix-cache --prune-output-head"
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        args.extend(["--worker-sha256".into(), "a".repeat(64)]);
        args
    }

    fn replace(args: &mut [String], flag: &str, value: &str) {
        let index = args.iter().position(|arg| arg == flag).unwrap();
        args[index + 1] = value.into();
    }

    #[test]
    fn prefill32_cli_requires_explicit_policy_and_preserves_legacy_parser() {
        for mode in ["baseline", "parallel-prefill32-two-pages-v27"] {
            let mut args = arguments();
            replace(&mut args, "--prefill32-pages-mode", mode);
            assert!(
                ordered64_kv_copy_live_contract::Options::parse(args.clone().into_iter()).is_err()
            );
            let result = Options::parse(args.into_iter());
            assert_eq!(
                result.is_ok(),
                cfg!(all(
                    feature = "c1-ordered64",
                    not(feature = "model-timestamps")
                ))
            );
            if let Ok(options) = result {
                assert_eq!(
                    options.selection().prefill32_pages,
                    Some(mode != "baseline")
                );
                assert!(options.selection().enabled);
                assert!(options.base.selection().prefill32_pages.is_none());
            }
        }
    }

    #[test]
    fn prefill32_cli_rejects_missing_duplicate_unknown_and_hybrid_policy() {
        let mut args = arguments();
        let index = args
            .iter()
            .position(|arg| arg == "--prefill32-pages-mode")
            .unwrap();
        args.drain(index..index + 2);
        assert!(Options::parse(args.into_iter()).is_err());
        let mut args = arguments();
        args.extend(["--prefill32-pages-mode".into(), "baseline".into()]);
        assert!(Options::parse(args.into_iter()).is_err());
        for (flag, value) in [
            ("--prefill32-pages-mode", ""),
            ("--prefill32-pages-mode", "auto"),
            ("--prefill32-pages-mode", "parallel-prefill16-v27"),
            ("--ordered64-kv-copy-mode", "baseline"),
            ("--prefill-kv-mode", "baseline"),
            ("--split-attention-mode", "baseline"),
            ("--context", "31"),
            ("--pages", "1"),
        ] {
            let mut args = arguments();
            replace(&mut args, flag, value);
            assert!(Options::parse(args.into_iter()).is_err(), "{flag} {value}");
        }
        for flag in [
            "--host-timing",
            "--model-timestamps",
            "--ordered64-runtime-counters",
        ] {
            let mut args = arguments();
            args.extend([flag.into(), "output".into()]);
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }

    #[test]
    #[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
    fn prefill32_cli_preserves_option_shaped_paths() {
        let mut args = arguments();
        replace(
            &mut args,
            "--ordered64-kv-copy-artifact",
            "--prefill32-pages-mode",
        );
        let options = Options::parse(args.into_iter()).unwrap();
        assert_eq!(
            options.selection().artifact,
            std::path::Path::new("--prefill32-pages-mode")
        );
    }
}
