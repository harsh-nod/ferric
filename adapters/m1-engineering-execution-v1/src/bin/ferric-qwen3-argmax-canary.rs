//! Fixed target-only argmax comparison, not a serving endpoint or M1 authority.

#![recursion_limit = "256"]

mod argmax_canary_contract;
#[allow(dead_code)]
mod argmax_canary_runtime;
// Reuse the frozen reference parser and byte helpers without changing its paired CLI.
#[allow(dead_code)]
mod paired_paged_canary_contract;
mod tp_host_timing;
mod tp_worker;

fn main() -> std::process::ExitCode {
    argmax_canary_runtime::execute(
        argmax_canary_contract::Options::parse(std::env::args().skip(1)),
        argmax_canary_runtime::CanaryProfile::LegacyArgmax,
    )
}
