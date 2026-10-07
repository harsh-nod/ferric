//! Explicit same-image chunk32 page-copy experiment; no serving qualification.

#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod ordered64_kv_copy_live_contract;
mod prefill32_pages_live_contract;
mod prefill_kv_copy_v28_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn main() -> std::process::ExitCode {
    let result = prefill32_pages_live_contract::Options::parse(std::env::args().skip(1)).and_then(
        |options| {
            wave_target_v17_runner::run_ordered64_kv_copy(
                &options.base.base.base,
                options.base.variant(),
                options.selection(),
            )
        },
    );
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Prefill32 pages live profile rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
