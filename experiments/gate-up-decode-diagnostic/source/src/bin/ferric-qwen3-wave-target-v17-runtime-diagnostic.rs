//! Combined V17 host/runtime attribution only; never a benchmark or serving grant.

#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;
mod wave_target_v17_runtime_diagnostic_contract;

fn main() -> std::process::ExitCode {
    let result =
        wave_target_v17_runtime_diagnostic_contract::Options::parse(std::env::args().skip(1))
            .and_then(|options| {
                options.validate()?;
                wave_target_v17_runner::run_variant(
                    &options.base,
                    wave_target_v17_runner::Variant::RuntimeDiagnostic,
                )
            });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Wave target V17 runtime diagnostic rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
