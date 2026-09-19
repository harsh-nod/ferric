//! Same-image target attention/normalization ablations; no benchmark or serving grant.

#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn main() -> std::process::ExitCode {
    match wave_target_v17_live_contract::Options::parse(std::env::args().skip(1))
        .and_then(|options| wave_target_v17_runner::run(&options))
    {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Wave target V17 live profile rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
