//! Independent C1 split-attention candidate; no default, serving or numerical grant.

#![recursion_limit = "256"]

mod c1_split_attention_v25_live_contract;
#[allow(dead_code)]
mod layer_c1_wave_live_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;
mod wave_target_v17_live_contract;
mod wave_target_v17_runner;

fn main() -> std::process::ExitCode {
    let result = c1_split_attention_v25_live_contract::Options::parse(std::env::args().skip(1))
        .and_then(|options| {
            options.validate()?;
            wave_target_v17_runner::run_variant(
                &options.base,
                wave_target_v17_runner::Variant::SplitAttentionV25 {
                    artifact: &options.artifact,
                    enabled: options.enabled,
                },
            )
        });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("C1 split attention V25 live profile rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
