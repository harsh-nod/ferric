#![forbid(unsafe_code)]

//! Serving comparison run-plan and real measurement ingestion boundary.

use ferric_m1_benchmarks::{main_for, Metric, Suite};
use std::env;
use std::process::ExitCode;
#[allow(unused_imports)]
use vstd::prelude::*;

mod m1_r33_partial;

const METRICS: &[Metric] = &[
    Metric {
        id: "ferric-p99-latency-ns",
        zero_allowed: false,
    },
    Metric {
        id: "ferric-total-tokens-per-second",
        zero_allowed: false,
    },
    Metric {
        id: "sglang-p99-latency-ns",
        zero_allowed: false,
    },
    Metric {
        id: "sglang-total-tokens-per-second",
        zero_allowed: false,
    },
    Metric {
        id: "vllm-p99-latency-ns",
        zero_allowed: false,
    },
    Metric {
        id: "vllm-total-tokens-per-second",
        zero_allowed: false,
    },
];

const SUITE: Suite = Suite {
    name: "serving",
    obligation_id: "m1.r33",
    path_id: "serving-bench",
    source_path: "benches/m1/serving.rs",
    case_kinds: &["burst", "closed-loop", "overload-sweep", "poisson"],
    extra_identities: &[
        "sglang-baseline",
        "sglang-config",
        "tuning-budget",
        "vllm-baseline",
        "vllm-config",
    ],
    metrics: METRICS,
    extra_record_attributes: &[
        "arrival-trace-sha256",
        "server-start-roster-sha256",
    ],
    minimum_warmups: 10,
    minimum_recorded_samples: 30,
    nonclaim: "Structural acceptance authenticates externally collected Ferric, vLLM, and SGLang serving records only. It does not establish equal tuning, SLO compliance, a confidence bound, baseline superiority, hardware correctness, qualification, or close m1.r33.",
};

fn main() -> ExitCode {
    if env::args_os().nth(1).as_deref() == Some(m1_r33_partial::COMMAND.as_ref()) {
        return m1_r33_partial::main_for_arguments(env::args_os().skip(2).collect());
    }
    main_for(&SUITE)
}
