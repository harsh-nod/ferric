//! Fixed-request decode attribution, never a latency qualification sample.
#![recursion_limit = "256"]
#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod native_prefill_width_contract;
mod native_decode_request_contract;
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
            let gate_up = gate_up.ok_or("explicit gate/up selection required")?;
            let (rows, arguments) = native_prefill_width_contract::parse_width(arguments)?;
            if rows != 32 { return Err("decode diagnostic requires prefill32".into()); }
            let options = ordered64_kv_copy_live_contract::Options::parse(arguments.into_iter())?;
            wave_target_v17_runner::run_native_gate_up_decode_diagnostic(
                &options.base.base, options.variant(), options.selection(), &gate_up,
            )
        },
    );
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Native gate/up decode diagnostic rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
