//! Explicit model raw-tick diagnostic. Never an uninstrumented serving benchmark.

#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod prefill_kv_copy_v28_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn parse_output(
    arguments: impl IntoIterator<Item = String>,
) -> Result<(std::path::PathBuf, Vec<String>), String> {
    let mut arguments = arguments.into_iter();
    let mut output = None;
    let mut retained = Vec::new();
    while let Some(argument) = arguments.next() {
        if argument == "--model-timestamps-output" {
            if output.is_some() {
                return Err("duplicate model timestamp output".into());
            }
            let path = arguments.next().ok_or("missing model timestamp output")?;
            if path.is_empty() || path.starts_with('-') {
                return Err("invalid model timestamp output".into());
            }
            output = Some(std::path::PathBuf::from(path));
        } else if matches!(argument.as_str(), "--split-attention-artifact" | "--split-attention-mode" | "--c1-packet-mode") {
            return Err("model timestamps exclude V28 decode composition".into());
        } else if matches!(
            argument.as_str(),
            "--live-stdin"
                | "--allow-unauthenticated-machine-code"
                | "--runtime-cache-admission"
                | "--runtime-operational"
                | "--queue-rollover"
                | "--disable-prefix-cache"
                | "--prune-output-head"
        ) {
            retained.push(argument);
        } else {
            let value = arguments
                .next()
                .ok_or_else(|| format!("missing value for {argument}"))?;
            retained.extend([argument, value]);
        }
    }
    Ok((
        output.ok_or("--model-timestamps-output is required")?,
        retained,
    ))
}

fn main() -> std::process::ExitCode {
    let result = parse_output(std::env::args().skip(1)).and_then(|(output, arguments)| {
        let options = prefill_kv_copy_v28_live_contract::Options::parse(arguments.into_iter())?;
        options.validate()?;
        if options.decode.is_some() {
            return Err("model timestamps exclude V28 decode composition".into());
        }
        wave_target_v17_runner::run_model_timestamps(
            &options.base,
            wave_target_v17_runner::Variant::PrefillKvCopyV28 {
                artifact: &options.artifact,
                enabled: options.enabled,
                decode: None,
            },
            &output,
        )
    });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Model raw-timestamp diagnostic rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_output;

    #[test]
    fn model_timestamp_parser_rejects_composition_flags_but_not_path_values() {
        for flag in ["--split-attention-artifact", "--split-attention-mode", "--c1-packet-mode"] {
            assert!(parse_output(["--model-timestamps-output", "raw.json", flag, "baseline"].map(str::to_owned)).is_err());
            let parsed = parse_output(["--model-timestamps-output", "raw.json", "--source", flag].map(str::to_owned)).unwrap();
            assert_eq!(parsed.1, ["--source", flag]);
        }
    }

    #[test]
    fn output_is_required_unique_and_does_not_consume_v28_options() {
        let parsed = parse_output(
            [
                "--model-timestamps-output",
                "/owned/raw.json",
                "--copy-mode",
                "baseline",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(parsed.0, std::path::Path::new("/owned/raw.json"));
        assert_eq!(parsed.1, ["--copy-mode", "baseline"]);
        for arguments in [
            vec![],
            vec!["--model-timestamps-output"],
            vec!["--model-timestamps-output", "--copy-mode"],
            vec![
                "--model-timestamps-output",
                "a",
                "--model-timestamps-output",
                "b",
            ],
        ] {
            assert!(parse_output(arguments.into_iter().map(str::to_owned)).is_err());
        }
    }

    #[test]
    fn output_parser_preserves_option_shaped_values_and_existing_boolean_flags() {
        let parsed = parse_output(
            [
                "--source",
                "--model-timestamps-output",
                "--live-stdin",
                "--model-timestamps-output",
                "/owned/raw.json",
                "--prune-output-head",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(
            parsed.1,
            [
                "--source",
                "--model-timestamps-output",
                "--live-stdin",
                "--prune-output-head"
            ]
        );
        assert!(
            parse_output(["--source", "--model-timestamps-output"].map(str::to_owned)).is_err()
        );
    }
}
