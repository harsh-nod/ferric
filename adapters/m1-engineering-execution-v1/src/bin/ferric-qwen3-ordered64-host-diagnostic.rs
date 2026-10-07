//! Bounded opt-in host diagnostic; never a matched-performance profile.

#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod ordered64_host_diagnostic_contract;
mod prefill_kv_copy_v28_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn main() -> std::process::ExitCode {
    let result = ordered64_host_diagnostic_contract::Options::parse(std::env::args().skip(1))
        .and_then(|options| {
            options.validate()?;
            if options.prefill16_ordered {
                return wave_target_v17_runner::run_prefill16_ordered_host_diagnostic(
                    &options.live.base,
                    options.variant(),
                    &options.output,
                    options.runtime_counters,
                );
            }
            wave_target_v17_runner::run_ordered64_host_diagnostic_with_wait(
                &options.live.base,
                options.variant(),
                &options.output,
                options.runtime_counters,
                options.active_poll_10ms,
            )
        });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Ordered64 host diagnostic rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
