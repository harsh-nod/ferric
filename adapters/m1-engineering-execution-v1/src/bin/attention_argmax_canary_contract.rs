//! Adds only an explicit attention selector to the unchanged fixed argmax contract.

use super::argmax_canary_contract::{ArgmaxMode, Options};
use super::argmax_canary_runtime::CanaryProfile;

pub fn parse(
    arguments: impl Iterator<Item = String>,
) -> Result<(Options, CanaryProfile), String> {
    let mut arguments = arguments;
    let mut forwarded = Vec::new();
    let mut profile = None;
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--attention" => {
                if profile.is_some() {
                    return Err("duplicate option --attention".into());
                }
                profile = Some(match arguments.next().as_deref() {
                    Some("baseline") => CanaryProfile::AttentionBaseline,
                    Some("wave") => CanaryProfile::AttentionWave,
                    _ => return Err("attention must be baseline or wave".into()),
                });
            }
            "--allow-unauthenticated-machine-code"
            | "--runtime-cache-admission"
            | "--runtime-operational"
            | "--runtime-rollover" => forwarded.push(flag),
            _ => {
                // Preserve option/value boundaries, including paths beginning with '--'.
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("missing value for {flag}"))?;
                forwarded.extend([flag, value]);
            }
        }
    }
    let profile = profile.ok_or("required option --attention")?;
    let options = Options::parse(forwarded.into_iter())?;
    if options.mode != ArgmaxMode::WaveV11 {
        return Err("attention comparison requires fixed argmax mode wave-v11".into());
    }
    Ok((options, profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::argmax_canary_contract::{CHUNK, CONTEXT, PAGES, PROMPT};

    fn arguments(attention: &str, mode: &str, outputs: &str) -> Vec<String> {
        [
            "--source", "source",
            "--target-artifact", "target",
            "--target-head-artifact", "head",
            "--argmax-artifact", "argmax",
            "--worker", "worker",
            "--worker-sha256", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--device-unique-id", "1",
            "--reference", "reference",
            "--prefix-reference", "prefix",
            "--host-timing-output", "timing",
            "--argmax-mode", mode,
            "--max-new-tokens", outputs,
            "--attention", attention,
            "--allow-unauthenticated-machine-code",
            "--runtime-cache-admission",
            "--runtime-operational",
            "--runtime-rollover",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    #[test]
    fn attention_modes_keep_the_fixed_workload_runtime_and_v11() {
        assert_eq!((PROMPT, CHUNK, CONTEXT, PAGES), (128, 16, 256, 16));
        for (attention, profile) in [
            ("baseline", CanaryProfile::AttentionBaseline),
            ("wave", CanaryProfile::AttentionWave),
        ] {
            for (outputs, packets) in [("8", 9_219), ("128", 83_139)] {
                let (options, actual) = parse(arguments(attention, "wave-v11", outputs).into_iter())
                    .unwrap();
                assert_eq!(actual, profile);
                assert_eq!(options.mode, ArgmaxMode::WaveV11);
                assert_eq!(options.expected_packets().unwrap(), packets);
                assert!(options.runtime.cache_admission);
                assert!(options.runtime.operational);
                assert!(options.runtime.rollover);
                assert!(!options.runtime.profile);
                assert!(!options.runtime.ordered_batches);
                assert!(!options.runtime.sequences);
            }
        }
    }

    #[test]
    fn attention_cli_requires_all_flags_and_rejects_duplicates() {
        let args = arguments("wave", "wave-v11", "8");
        for index in 0..args.len() {
            if !args[index].starts_with("--") {
                continue;
            }
            let mut missing = args.clone();
            missing.remove(index);
            assert!(parse(missing.into_iter()).is_err());
            let mut duplicate = args.clone();
            duplicate.push(args[index].clone());
            assert!(parse(duplicate.into_iter()).is_err());
        }
        let mut duplicate = args;
        duplicate.extend(["--attention".into(), "baseline".into()]);
        assert!(parse(duplicate.into_iter()).is_err());
    }

    #[test]
    fn attention_cli_rejects_serial_argmax_and_unreviewed_policies() {
        for (attention, mode, outputs) in [
            ("auto", "wave-v11", "8"),
            ("wave", "serial", "8"),
            ("baseline", "serial", "128"),
            ("wave", "wave-v11", "9"),
        ] {
            assert!(parse(arguments(attention, mode, outputs).into_iter()).is_err());
        }
        for extra in [
            vec!["--runtime-ordered-batches"],
            vec!["--runtime-sequences"],
            vec!["--runtime-profile"],
            vec!["--projection", "wave"],
            vec!["--context", "8192"],
            vec!["--pages", "512"],
            vec!["--prefix-cache"],
            vec!["--unknown", "value"],
        ] {
            let mut args = arguments("wave", "wave-v11", "8");
            args.extend(extra.into_iter().map(str::to_owned));
            assert!(parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn legacy_parser_still_rejects_attention_and_keeps_both_argmax_modes() {
        for mode in ["serial", "wave-v11"] {
            let args = arguments("baseline", mode, "8");
            assert!(Options::parse(args.clone().into_iter()).is_err());
            let index = args.iter().position(|flag| flag == "--attention").unwrap();
            let mut legacy = args;
            legacy.drain(index..index + 2);
            assert_eq!(Options::parse(legacy.into_iter()).unwrap().mode.label(), mode);
        }
    }

    #[test]
    fn attention_option_is_not_interpreted_inside_a_path_value() {
        let mut args = arguments("wave", "wave-v11", "8");
        args[1] = "--attention".into();
        let (options, profile) = parse(args.into_iter()).unwrap();
        assert_eq!(options.source, std::path::PathBuf::from("--attention"));
        assert_eq!(profile, CanaryProfile::AttentionWave);
    }
}
