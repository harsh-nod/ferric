//! Short-lived R33 collector frontend for an independently supervised service.

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    ferric_m1_engineering_execution_v1::r33_service::adapter_main(&arguments)
}
