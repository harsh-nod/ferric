//! Closed wave/v11 submission comparison; no serving or M1 authority.

#![recursion_limit = "256"]

mod argmax_canary_contract;
#[allow(dead_code)]
mod argmax_canary_runtime;
mod attention_argmax_canary_contract;
#[allow(dead_code)]
mod paired_paged_canary_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_submission_canary_contract;

fn main() -> std::process::ExitCode {
    match wave_argmax_submission_canary_contract::parse(std::env::args().skip(1)) {
        Ok((options, profile)) => argmax_canary_runtime::execute(Ok(options), profile),
        Err(error) => {
            eprintln!("Wave argmax submission canary rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
