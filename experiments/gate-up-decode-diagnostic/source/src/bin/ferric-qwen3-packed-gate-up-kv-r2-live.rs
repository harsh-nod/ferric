//! Separate ten-image V19 plus packed gate/up ablation; no performance or serving grant.

#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod ordered64_kv_copy_live_contract;
mod packed_gate_up_live_contract;
mod prefill_kv_copy_v28_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn main() -> std::process::ExitCode {
    let result = packed_gate_up_live_contract::Options::parse(std::env::args().skip(1)).and_then(
        |options| {
            if let Some(output) = &options.host_timing {
                return wave_target_v17_runner::run_packed_host_diagnostic(
                    &options.base.base.base,
                    options.variant(),
                    options.base.selection(),
                    &options.selection,
                    output,
                );
            }
            wave_target_v17_runner::run_packed_gate_up(
                &options.base.base.base,
                options.variant(),
                options.base.selection(),
                &options.selection,
            )
        },
    );
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Packed gate/up live profile rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
