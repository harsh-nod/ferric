//! Explicit whole-token TP1/C1 experiment; no serving or performance grant.
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

fn main() -> std::process::ExitCode {
    let result = ordered64_kv_copy_live_contract::Options::parse(std::env::args().skip(1))
        .and_then(|options| {
            wave_target_v17_runner::run_native_token_program(
                &options.base.base,
                options.variant(),
                options.selection(),
            )
        });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Native whole-token program rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
