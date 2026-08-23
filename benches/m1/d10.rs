#![forbid(unsafe_code)]

//! D10 core-kernel run-plan and real measurement ingestion boundary.

mod d10_policy;

use ferric_m1_benchmarks::{main_for, Metric, Suite};
use std::env;
use std::process::ExitCode;
#[allow(unused_imports)]
use vstd::prelude::*;

const METRICS: &[Metric] = &[
    Metric {
        id: "ferric-reference-throughput-flops-per-second",
        zero_allowed: false,
    },
    Metric {
        id: "ferric-throughput-flops-per-second",
        zero_allowed: false,
    },
    Metric {
        id: "vendor-throughput-flops-per-second",
        zero_allowed: false,
    },
];

const SUITE: Suite = Suite {
    name: "d10",
    obligation_id: "m1.r31",
    path_id: "d10-bench",
    source_path: "benches/m1/d10.rs",
    case_kinds: &[
        "flash-attention-prefill",
        "gemm-gemv",
        "gqa-paged-decode",
        "logits-argmax",
        "rmsnorm-residual",
        "rope-paged-kv",
        "swiglu-projection",
    ],
    extra_identities: &["tuning-budget", "vendor-baseline", "vendor-config"],
    metrics: METRICS,
    extra_record_attributes: &["resource-inspection-sha256"],
    minimum_warmups: 10,
    minimum_recorded_samples: 30,
    nonclaim: "Structural acceptance authenticates externally collected D10 kernel records only. It does not recompute the D10 gates, establish artifact or target conformance, prove kernel correctness, qualify performance, or close m1.r31.",
};

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|command| command == "admit-experiment-policy")
    {
        return match d10_policy::admit_experiment_policy(&arguments) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("FAIL: {error}");
                ExitCode::FAILURE
            }
        };
    }
    main_for(&SUITE)
}
