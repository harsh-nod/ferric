//! Separate nine-image packed-down ablation; no default, performance or serving grant.

#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod packed_down_live_contract;
mod prefill_kv_copy_v28_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn main() -> std::process::ExitCode {
    let result = packed_down_live_contract::Options::parse(std::env::args().skip(1)).and_then(
        |options| {
            wave_target_v17_runner::run_packed_down(
                &options.base.base,
                options.variant(),
                &options.selection,
            )
        },
    );
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Packed down live profile rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
