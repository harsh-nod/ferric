//! Explicit split-K selection around the unchanged ordinary V19 CLI.

use super::ordered64_kv_copy_live_contract;
use super::wave_target_v17_runner::{SplitKDown, Variant, packed_digest_argument};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpSplitKDownImageIdsR1;
use std::collections::BTreeMap;

const FLAGS: [&str; 7] = [
    "--splitk-down-artifact",
    "--splitk-down-roster",
    "--splitk-down-roster-sha256",
    "--splitk-down-hsaco-sha256",
    "--splitk-down-manifest-sha256",
    "--splitk-down-handoff-sha256",
    "--splitk-down-mode",
];

pub struct Options {
    pub base: ordered64_kv_copy_live_contract::Options,
    pub selection: SplitKDown,
}

impl Options {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments;
        let mut selected = BTreeMap::new();
        let mut forwarded = Vec::new();
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
        let get = |flag: &str| {
            selected
                .get(flag)
                .map(String::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("required nonempty {flag}"))
        };
        let selection = SplitKDown {
            artifact: get(FLAGS[0])?.into(),
            roster: get(FLAGS[1])?.into(),
            roster_sha256: packed_digest_argument(get(FLAGS[2])?)?,
            image_ids: EngineeringTpSplitKDownImageIdsR1 {
                hsaco: packed_digest_argument(get(FLAGS[3])?)?,
                manifest: packed_digest_argument(get(FLAGS[4])?)?,
                handoff: packed_digest_argument(get(FLAGS[5])?)?,
            },
            enabled: match get(FLAGS[6])? {
                "baseline" => false,
                "splitk8-down-mfma-r1" => true,
                _ => {
                    return Err("split-K down mode must be baseline or splitk8-down-mfma-r1".into());
                }
            },
        };
        let options = Self {
            base: ordered64_kv_copy_live_contract::Options::parse(forwarded.into_iter())?,
            selection,
        };
        options.selection.validate(
            &options.base.base.base,
            options.variant(),
            options.base.selection(),
        )?;
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
        let mut values = "--source source --target-artifact target --target-head-artifact head \
            --argmax-artifact argmax --worker worker --worker-sha256 \
            aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
            --device-unique-id 1 --submission ordered --context 8192 --pages 512 --max-batches 1000000 \
            --layer-projection c1-wave --wave-target-mode combined --attention-artifact attention \
            --rmsnorm-artifact rmsnorm --prefill-kv-artifact prefill --prefill-kv-mode parallel-prefill16-v27 \
            --split-attention-artifact split --split-attention-mode split8-v21 --c1-packet-mode packed64-v29 \
            --gemv-artifact gemv --gemv-mode baseline --ordered64-kv-copy-artifact copy \
            --ordered64-kv-copy-mode parallel-c1-v19 --live-stdin --allow-unauthenticated-machine-code \
            --runtime-cache-admission --runtime-operational --queue-rollover --disable-prefix-cache --prune-output-head"
            .split_whitespace().map(str::to_owned).collect::<Vec<_>>();
        for (flag, value) in FLAGS.into_iter().zip([
            "/unused/splitk".into(),
            "/unused/roster.json".into(),
            "11".repeat(32),
            "22".repeat(32),
            "33".repeat(32),
            "44".repeat(32),
            "splitk8-down-mfma-r1".into(),
        ]) {
            values.extend([flag.into(), value]);
        }
        values
    }

    fn replace(args: &mut [String], flag: &str, value: &str) {
        let index = args.iter().position(|item| item == flag).unwrap();
        args[index + 1] = value.into();
    }

    #[test]
    fn splitk_cli_uses_same_binary_composition_and_assets_in_both_arms() {
        let ordinary = cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps"),
            not(feature = "c1-token-program")
        ));
        for mode in ["baseline", "splitk8-down-mfma-r1"] {
            let mut args = arguments();
            replace(&mut args, "--splitk-down-mode", mode);
            assert!(
                ordered64_kv_copy_live_contract::Options::parse(args.clone().into_iter()).is_err()
            );
            let parsed = Options::parse(args.into_iter());
            assert_eq!(parsed.is_ok(), ordinary);
            if let Ok(options) = parsed {
                assert!(options.base.selection().enabled);
                assert_eq!(options.selection.enabled, mode != "baseline");
                assert_eq!(options.selection.image_ids.hsaco, [0x22; 32]);
                assert_eq!(options.selection.roster_sha256, [0x11; 32]);
            }
        }
    }

    #[test]
    fn splitk_cli_refuses_missing_duplicate_unpinned_and_hybrid_options() {
        for flag in FLAGS {
            let mut args = arguments();
            let index = args.iter().position(|item| item == flag).unwrap();
            args.drain(index..index + 2);
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
        for (flag, value) in [
            ("--ordered64-kv-copy-mode", "baseline"),
            ("--prefill-kv-mode", "baseline"),
            ("--split-attention-mode", "baseline"),
            ("--c1-packet-mode", "packed16-v22"),
            ("--gemv-mode", "partial-prefetch4-v20"),
            ("--splitk-down-mode", "auto"),
            ("--splitk-down-artifact", "relative"),
            ("--splitk-down-roster", "relative"),
        ] {
            let mut args = arguments();
            replace(&mut args, flag, value);
            assert!(Options::parse(args.into_iter()).is_err());
        }
        for flag in [
            "--runtime-profile",
            "--ordered64-packet-ticks",
            "--host-timing",
            "--prefill32-pages-mode",
            "--native-prefill-rows",
        ] {
            let mut args = arguments();
            args.extend([flag.into(), "32".into()]);
            assert!(Options::parse(args.into_iter()).is_err());
        }
    }
}
