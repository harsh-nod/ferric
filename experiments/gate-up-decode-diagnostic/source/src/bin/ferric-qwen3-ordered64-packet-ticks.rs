//! Bounded raw GPU packet-processing ticks, not a serving benchmark.

#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod ordered64_packet_ticks_contract;
mod prefill_kv_copy_v28_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn main() -> std::process::ExitCode {
    let result = ordered64_packet_ticks_contract::Options::parse(std::env::args().skip(1))
        .and_then(|options| {
            wave_target_v17_runner::run_ordered64_packet_ticks(
                &options.live.base,
                options.variant(),
                options.selection(),
                &options.output,
            )
        });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Ordered64 packet tick diagnostic rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
