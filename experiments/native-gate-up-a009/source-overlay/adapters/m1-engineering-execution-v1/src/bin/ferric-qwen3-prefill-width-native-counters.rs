//! Separate profiled native prefill16/32 mechanism diagnostic, never a latency sample.
#![recursion_limit = "256"]
#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod native_prefill_width_contract;
mod ordered64_kv_copy_live_contract;
mod prefill_kv_copy_v28_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn main() -> std::process::ExitCode {
    let result = wave_target_v17_runner::parse_native_gate_up(std::env::args().skip(1)).and_then(
        |(gate_up, arguments)| {
            let (rows, arguments) = native_prefill_width_contract::parse_width(arguments)?;
            let options = ordered64_kv_copy_live_contract::Options::parse(arguments.into_iter())?;
            if let Some(selection) = gate_up {
                if rows != 32 {
                    return Err("native gate/up requires explicit prefill32".into());
                }
                return wave_target_v17_runner::run_native_gate_up_program(
                    &options.base.base,
                    options.variant(),
                    options.selection(),
                    &selection,
                    true,
                );
            }
            wave_target_v17_runner::run_native_prefill_width_program(
                &options.base.base,
                options.variant(),
                options.selection(),
                rows,
                true,
            )
        },
    );
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Native prefill width counters rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
