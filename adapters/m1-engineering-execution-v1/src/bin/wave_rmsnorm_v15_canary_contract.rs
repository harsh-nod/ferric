//! Same-image pure norm comparison over the unchanged ordered C1 workload contract.

use super::argmax_canary_contract::Options;
use super::argmax_canary_runtime::CanaryProfile;
use super::layer_c1_wave_canary_contract;
use std::path::PathBuf;

pub fn parse(
    arguments: impl Iterator<Item = String>,
) -> Result<(Options, PathBuf, CanaryProfile), String> {
    let mut arguments = arguments;
    let mut forwarded = Vec::new();
    let mut profile = None;
    let mut artifact = None;
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--rmsnorm-mode" => {
                if profile.is_some() {
                    return Err("duplicate option --rmsnorm-mode".into());
                }
                profile = Some(match arguments.next().as_deref() {
                    Some("baseline") => CanaryProfile::WaveRmsNormBaseline,
                    Some("wave-v15") => CanaryProfile::WaveRmsNormV15,
                    _ => return Err("RMSNorm mode must be baseline or wave-v15".into()),
                });
            }
            "--rmsnorm-artifact" => {
                if artifact.is_some() {
                    return Err("duplicate option --rmsnorm-artifact".into());
                }
                let path = arguments.next().ok_or("missing RMSNorm artifact path")?;
                if path.is_empty() {
                    return Err("RMSNorm artifact path must be nonempty".into());
                }
                artifact = Some(PathBuf::from(path));
            }
            "--allow-unauthenticated-machine-code"
            | "--runtime-cache-admission"
            | "--runtime-operational"
            | "--runtime-rollover" => forwarded.push(flag),
            _ => {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("missing value for {flag}"))?;
                forwarded.extend([flag, value]);
            }
        }
    }
    let profile = profile.ok_or("required option --rmsnorm-mode")?;
    let artifact = artifact.ok_or("required option --rmsnorm-artifact")?;
    let (options, layer) = layer_c1_wave_canary_contract::parse(forwarded.into_iter())?;
    if layer != CanaryProfile::LayerC1Wave {
        return Err("RMSNorm comparison requires fixed C1 Wave layers".into());
    }
    Ok((options, artifact, profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::argmax_canary_contract::{ArgmaxMode, CHUNK, CONTEXT, PAGES, PROMPT};

    fn arguments(mode: &str, outputs: &str) -> Vec<String> {
        [
            "--source",
            "source",
            "--target-artifact",
            "target",
            "--target-head-artifact",
            "head",
            "--argmax-artifact",
            "argmax",
            "--rmsnorm-artifact",
            "v15",
            "--worker",
            "worker",
            "--worker-sha256",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--device-unique-id",
            "1",
            "--reference",
            "reference",
            "--prefix-reference",
            "prefix",
            "--host-timing-output",
            "timing",
            "--argmax-mode",
            "wave-v11",
            "--max-new-tokens",
            outputs,
            "--attention",
            "wave",
            "--submission",
            "ordered",
            "--layer-projection",
            "c1-wave",
            "--rmsnorm-mode",
            mode,
            "--allow-unauthenticated-machine-code",
            "--runtime-cache-admission",
            "--runtime-operational",
            "--runtime-rollover",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    #[test]
    fn v15_canary_modes_share_images_workload_references_and_runtime() {
        assert_eq!((PROMPT, CHUNK, CONTEXT, PAGES), (128, 16, 256, 16));
        for (outputs, packets) in [("8", 9_219), ("128", 83_139)] {
            let mut inputs = Vec::new();
            for (mode, expected) in [
                ("baseline", CanaryProfile::WaveRmsNormBaseline),
                ("wave-v15", CanaryProfile::WaveRmsNormV15),
            ] {
                let (options, artifact, profile) =
                    parse(arguments(mode, outputs).into_iter()).unwrap();
                assert_eq!(profile, expected);
                assert_eq!(artifact, PathBuf::from("v15"));
                assert_eq!(options.mode, ArgmaxMode::WaveV11);
                assert_eq!(options.expected_packets().unwrap(), packets);
                assert!(
                    options.runtime.cache_admission
                        && options.runtime.operational
                        && options.runtime.rollover
                        && options.runtime.ordered_batches
                );
                assert!(
                    !options.runtime.sequences
                        && !options.runtime.profile
                        && !options.runtime.shared_full_currentness
                );
                inputs.push((
                    options.source,
                    options.target_artifact,
                    options.target_head_artifact,
                    options.argmax_artifact,
                    artifact,
                    options.worker,
                    options.worker_sha256,
                    options.device,
                    options.reference,
                    options.prefix_reference,
                    options.host_timing_output,
                    options.outputs,
                ));
            }
            assert_eq!(inputs[0], inputs[1]);
        }
    }

    #[test]
    fn v15_canary_requires_explicit_nonempty_image_and_exact_mode_once() {
        for flag in ["--rmsnorm-mode", "--rmsnorm-artifact"] {
            let args = arguments("baseline", "8");
            let index = args.iter().position(|arg| arg == flag).unwrap();
            let mut missing = args.clone();
            missing.drain(index..index + 2);
            assert!(parse(missing.into_iter()).is_err());
            let mut duplicate = args.clone();
            duplicate.extend_from_slice(&args[index..index + 2]);
            assert!(parse(duplicate.into_iter()).is_err());
            let mut empty = args.clone();
            empty[index + 1].clear();
            assert!(parse(empty.into_iter()).is_err());
            let mut no_value = args;
            no_value.drain(index..index + 2);
            no_value.push(flag.into());
            assert!(parse(no_value.into_iter()).is_err());
        }
        for mode in [
            "wave",
            "resident-wave",
            "v15",
            "auto",
            "WAVE-V15",
            "query-hoist-v14",
        ] {
            assert!(parse(arguments(mode, "8").into_iter()).is_err());
        }
    }

    #[test]
    fn v15_canary_rejects_attention_head_layer_submission_and_runtime_broadening() {
        for (flag, value) in [
            ("--layer-projection", "mfma"),
            ("--submission", "synchronous"),
            ("--attention", "baseline"),
            ("--argmax-mode", "serial"),
            ("--max-new-tokens", "9"),
        ] {
            let mut args = arguments("wave-v15", "8");
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args[index + 1] = value.into();
            assert!(parse(args.into_iter()).is_err());
        }
        for extra in [
            vec!["--runtime-sequences"],
            vec!["--runtime-profile"],
            vec!["--runtime-shared-full-currentness"],
            vec!["--runtime-ordered-batches"],
            vec!["--context", "8192"],
            vec!["--pages", "512"],
            vec!["--projection", "auto"],
            vec!["--attention-mode", "query-hoist-v14"],
            vec!["--query-hoist-artifact", "v14"],
            vec!["--unknown", "value"],
        ] {
            let mut args = arguments("baseline", "128");
            args.extend(extra.into_iter().map(str::to_owned));
            assert!(parse(args.into_iter()).is_err());
        }
    }

    #[test]
    fn v15_canary_preserves_option_shaped_path_values() {
        for flag in [
            "--source",
            "--target-artifact",
            "--target-head-artifact",
            "--argmax-artifact",
            "--rmsnorm-artifact",
            "--worker",
            "--reference",
            "--prefix-reference",
            "--host-timing-output",
        ] {
            for value in [
                "--rmsnorm-mode",
                "--rmsnorm-artifact",
                "--layer-projection",
                "--runtime-operational",
            ] {
                let mut args = arguments("wave-v15", "8");
                let index = args.iter().position(|arg| arg == flag).unwrap();
                args[index + 1] = value.into();
                let (options, artifact, profile) = parse(args.into_iter()).unwrap();
                assert_eq!(profile, CanaryProfile::WaveRmsNormV15);
                let path = match flag {
                    "--source" => options.source,
                    "--target-artifact" => options.target_artifact,
                    "--target-head-artifact" => options.target_head_artifact,
                    "--argmax-artifact" => options.argmax_artifact,
                    "--rmsnorm-artifact" => artifact,
                    "--worker" => options.worker,
                    "--reference" => options.reference,
                    "--prefix-reference" => options.prefix_reference,
                    "--host-timing-output" => options.host_timing_output,
                    _ => unreachable!(),
                };
                assert_eq!(path, PathBuf::from(value));
            }
        }
    }

    #[test]
    fn v15_canary_leaves_all_prior_parsers_and_layer_modes_closed() {
        let args = arguments("wave-v15", "8");
        assert!(layer_c1_wave_canary_contract::parse(args.clone().into_iter()).is_err());
        assert!(
            crate::wave_argmax_submission_canary_contract::parse(args.clone().into_iter()).is_err()
        );
        assert!(crate::attention_argmax_canary_contract::parse(args.clone().into_iter()).is_err());
        assert!(Options::parse(args.clone().into_iter()).is_err());
        for (mode, profile) in [
            ("mfma", CanaryProfile::LayerMfma),
            ("c1-wave", CanaryProfile::LayerC1Wave),
        ] {
            let mut old = args.clone();
            for flag in ["--rmsnorm-mode", "--rmsnorm-artifact"] {
                let index = old.iter().position(|arg| arg == flag).unwrap();
                old.drain(index..index + 2);
            }
            let index = old
                .iter()
                .position(|arg| arg == "--layer-projection")
                .unwrap();
            old[index + 1] = mode.into();
            assert_eq!(
                layer_c1_wave_canary_contract::parse(old.into_iter())
                    .unwrap()
                    .1,
                profile
            );
        }
    }
}
