//! Distinct TP1/16-row prefill copy candidate; no default, numerical, benchmark or serving grant.

#![recursion_limit = "256"]

#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod prefill_kv_copy_v28_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn main() -> std::process::ExitCode {
    let result = prefill_kv_copy_v28_live_contract::Options::parse(std::env::args().skip(1))
        .and_then(|options| {
            options.validate()?;
            wave_target_v17_runner::run_variant(
                &options.base,
                wave_target_v17_runner::Variant::PrefillKvCopyV28 {
                    artifact: &options.artifact,
                    enabled: options.enabled,
                    decode: options.decode.as_ref().map(|decode| {
                        wave_target_v17_runner::DecodeComposition {
                            artifact: &decode.artifact,
                            split: decode.split,
                            packed: decode.packed,
                            ordered64: decode.ordered64,
                            gemv: decode
                                .gemv
                                .as_ref()
                                .map(|(path, enabled)| (path.as_path(), *enabled)),
                        }
                    }),
                },
            )
        });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Prefill KV copy V28 live profile rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
