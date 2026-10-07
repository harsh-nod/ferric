//! Explicit same-image V15 comparison; no serving or benchmark admission.

#![recursion_limit = "256"]

mod argmax_canary_contract;
#[allow(dead_code)]
mod argmax_canary_runtime;
mod attention_argmax_canary_contract;
mod layer_c1_wave_canary_contract;
#[allow(dead_code)]
mod paired_paged_canary_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_submission_canary_contract;
mod wave_rmsnorm_v15_canary_contract;

fn main() -> std::process::ExitCode {
    match wave_rmsnorm_v15_canary_contract::parse(std::env::args().skip(1)) {
        Ok((options, artifact, profile)) => {
            argmax_canary_runtime::execute_wave_rmsnorm_v15(options, &artifact, profile)
        }
        Err(error) => {
            eprintln!("Wave RMSNorm V15 canary rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
