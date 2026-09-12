//! Adds only a closed layer selector to the unchanged ordered submission contract.

use super::argmax_canary_contract::Options;
use super::argmax_canary_runtime::CanaryProfile;
use super::wave_argmax_submission_canary_contract;

pub fn parse(arguments: impl Iterator<Item = String>) -> Result<(Options, CanaryProfile), String> {
    let mut arguments = arguments;
    let mut forwarded = Vec::new();
    let mut profile = None;
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--layer-projection" => {
                if profile.is_some() {
                    return Err("duplicate option --layer-projection".into());
                }
                profile = Some(match arguments.next().as_deref() {
                    Some("mfma") => CanaryProfile::LayerMfma,
                    Some("c1-wave") => CanaryProfile::LayerC1Wave,
                    _ => return Err("layer projection must be mfma or c1-wave".into()),
                });
            }
            "--allow-unauthenticated-machine-code"
            | "--runtime-cache-admission"
            | "--runtime-operational"
            | "--runtime-rollover" => forwarded.push(flag),
            _ => {
                // Preserve option-shaped path values for the existing parser.
                let value = arguments.next().ok_or_else(|| format!("missing value for {flag}"))?;
                forwarded.extend([flag, value]);
            }
        }
    }
    let profile = profile.ok_or("required option --layer-projection")?;
    let (options, submission) = wave_argmax_submission_canary_contract::parse(forwarded.into_iter())?;
    if submission != CanaryProfile::SubmissionOrdered {
        return Err("layer comparison requires fixed submission ordered".into());
    }
    Ok((options, profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::argmax_canary_contract::{ArgmaxMode, CHUNK, CONTEXT, PAGES, PROMPT};

    fn arguments(layer: &str, outputs: &str) -> Vec<String> {
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
            "--argmax-mode", "wave-v11",
            "--max-new-tokens", outputs,
            "--attention", "wave",
            "--submission", "ordered",
            "--layer-projection", layer,
            "--allow-unauthenticated-machine-code",
            "--runtime-cache-admission",
            "--runtime-operational",
            "--runtime-rollover",
        ].map(str::to_owned).to_vec()
    }

    #[test]
    fn layer_canary_modes_keep_identical_runtime_references_and_packet_counts() {
        assert_eq!((PROMPT, CHUNK, CONTEXT, PAGES), (128, 16, 256, 16));
        for (outputs, packets) in [("8", 9_219), ("128", 83_139)] {
            let mut snapshots = Vec::new();
            for (layer, expected) in [("mfma", CanaryProfile::LayerMfma), ("c1-wave", CanaryProfile::LayerC1Wave)] {
                let (options, profile) = parse(arguments(layer, outputs).into_iter()).unwrap();
                assert_eq!(profile, expected);
                assert_eq!(options.mode, ArgmaxMode::WaveV11);
                assert_eq!(options.expected_packets().unwrap(), packets);
                assert!(options.runtime.cache_admission && options.runtime.operational && options.runtime.rollover && options.runtime.ordered_batches);
                assert!(!options.runtime.sequences && !options.runtime.profile && !options.runtime.shared_full_currentness);
                snapshots.push((options.source, options.target_artifact, options.target_head_artifact, options.argmax_artifact, options.worker, options.worker_sha256, options.device, options.reference, options.prefix_reference, options.host_timing_output, options.outputs));
            }
            assert_eq!(snapshots[0], snapshots[1]);
        }
    }

    #[test]
    fn layer_canary_requires_every_flag_and_rejects_duplicate_selectors() {
        let args = arguments("mfma", "8");
        for index in 0..args.len() {
            if !args[index].starts_with("--") { continue; }
            let mut missing = args.clone();
            missing.remove(index);
            assert!(parse(missing.into_iter()).is_err());
            let mut duplicate = args.clone();
            duplicate.push(args[index].clone());
            assert!(parse(duplicate.into_iter()).is_err());
        }
        for (flag, value) in [("--layer-projection", "c1-wave"), ("--submission", "ordered"), ("--attention", "wave")] {
            let mut args = args.clone();
            args.extend([flag.into(), value.into()]);
            assert!(parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn layer_canary_rejects_unreviewed_arithmetic_lengths_and_runtime_overrides() {
        for layer in ["auto", "wave", "baseline", "MFMA", ""] {
            assert!(parse(arguments(layer, "8").into_iter()).is_err());
        }
        for (flag, value) in [("--submission", "synchronous"), ("--attention", "baseline"), ("--argmax-mode", "serial"), ("--max-new-tokens", "9"), ("--max-new-tokens", "0")] {
            let mut args = arguments("c1-wave", "8");
            let index = args.iter().position(|argument| argument == flag).unwrap();
            args[index + 1] = value.into();
            assert!(parse(args.into_iter()).is_err());
        }
        for extra in [vec!["--runtime-ordered-batches"], vec!["--runtime-sequences"], vec!["--runtime-profile"], vec!["--runtime-shared-full-currentness"], vec!["--projection", "auto"], vec!["--context", "8192"], vec!["--pages", "512"], vec!["--prefix-cache"], vec!["--unknown", "value"]] {
            let mut args = arguments("c1-wave", "8");
            args.extend(extra.into_iter().map(str::to_owned));
            assert!(parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn layer_canary_preserves_option_shaped_path_values_at_every_boundary() {
        for flag in ["--source", "--target-artifact", "--target-head-artifact", "--argmax-artifact", "--worker", "--reference", "--prefix-reference", "--host-timing-output"] {
            for value in ["--layer-projection", "--submission", "--attention", "--runtime-operational"] {
                let mut args = arguments("c1-wave", "8");
                let index = args.iter().position(|argument| argument == flag).unwrap();
                args[index + 1] = value.into();
                let (options, profile) = parse(args.into_iter()).unwrap();
                assert_eq!(profile, CanaryProfile::LayerC1Wave);
                let actual = match flag {
                    "--source" => options.source,
                    "--target-artifact" => options.target_artifact,
                    "--target-head-artifact" => options.target_head_artifact,
                    "--argmax-artifact" => options.argmax_artifact,
                    "--worker" => options.worker,
                    "--reference" => options.reference,
                    "--prefix-reference" => options.prefix_reference,
                    "--host-timing-output" => options.host_timing_output,
                    _ => unreachable!(),
                };
                assert_eq!(actual, std::path::PathBuf::from(value));
            }
        }
    }

    #[test]
    fn layer_canary_leaves_all_prior_parsers_closed_and_prior_modes_unchanged() {
        let args = arguments("c1-wave", "8");
        assert!(wave_argmax_submission_canary_contract::parse(args.clone().into_iter()).is_err());
        assert!(crate::attention_argmax_canary_contract::parse(args.clone().into_iter()).is_err());
        assert!(Options::parse(args.clone().into_iter()).is_err());
        for submission in ["synchronous", "ordered"] {
            let mut args = args.clone();
            let index = args.iter().position(|flag| flag == "--layer-projection").unwrap();
            args.drain(index..index + 2);
            let index = args.iter().position(|flag| flag == "--submission").unwrap();
            args[index + 1] = submission.into();
            let (options, profile) = wave_argmax_submission_canary_contract::parse(args.into_iter()).unwrap();
            assert_eq!(options.runtime.ordered_batches, submission == "ordered");
            assert_eq!(profile, if submission == "ordered" { CanaryProfile::SubmissionOrdered } else { CanaryProfile::SubmissionSynchronous });
        }
    }
}
