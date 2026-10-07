//! Bounded token-program counters only; timings are not latency samples.
#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod ordered64_kv_copy_live_contract;
mod prefill_kv_copy_v28_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn backend(
    arguments: &mut impl Iterator<Item = String>,
) -> Result<tp_worker::TokenProgramBackend, String> {
    if arguments.next().as_deref() != Some("--token-program-backend") {
        return Err("counters require leading --token-program-backend".into());
    }
    match arguments.next().as_deref() {
        Some("ordered64-groups-v1") => Ok(tp_worker::TokenProgramBackend::Ordered64GroupsV1),
        Some("native-whole-program-v1") => Ok(tp_worker::TokenProgramBackend::NativeWholeProgramV1),
        _ => Err("token backend must be ordered64-groups-v1 or native-whole-program-v1".into()),
    }
}

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args().skip(1);
    let result = backend(&mut arguments).and_then(|backend| {
        let options = ordered64_kv_copy_live_contract::Options::parse(arguments)?;
        wave_target_v17_runner::run_token_program_counters(
            &options.base.base,
            options.variant(),
            options.selection(),
            backend,
        )
    });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Token program counters rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_backend_requires_explicit_closed_leading_selector() {
        for (name, expected) in [
            (
                "ordered64-groups-v1",
                tp_worker::TokenProgramBackend::Ordered64GroupsV1,
            ),
            (
                "native-whole-program-v1",
                tp_worker::TokenProgramBackend::NativeWholeProgramV1,
            ),
        ] {
            let mut args = ["--token-program-backend", name, "--source", "model"]
                .map(str::to_owned)
                .into_iter();
            assert_eq!(backend(&mut args).unwrap(), expected);
            assert_eq!(args.collect::<Vec<_>>(), ["--source", "model"]);
        }
        for args in [
            vec![],
            vec!["--source", "model"],
            vec!["--token-program-backend"],
            vec!["--token-program-backend", "auto"],
            vec!["--token-program-backend", ""],
        ] {
            assert!(backend(&mut args.into_iter().map(str::to_owned)).is_err());
        }
    }
}
