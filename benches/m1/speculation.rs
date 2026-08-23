#![forbid(unsafe_code)]

//! Speculation holdout run-plan and real measurement ingestion boundary.

use ferric_m1_benchmarks::{main_for, Metric, Suite};
use std::env;
use std::process::ExitCode;
#[allow(unused_imports)]
use vstd::prelude::*;

mod m1_r32_paired_collector;
mod m1_r32_speculation_records;

const METRICS: &[Metric] = &[
    Metric {
        id: "accepted-tokens",
        zero_allowed: true,
    },
    Metric {
        id: "ferric-p99-latency-ns",
        zero_allowed: false,
    },
    Metric {
        id: "ferric-total-tokens-per-second",
        zero_allowed: false,
    },
    Metric {
        id: "target-invocations",
        zero_allowed: false,
    },
    Metric {
        id: "target-only-p99-latency-ns",
        zero_allowed: false,
    },
    Metric {
        id: "target-only-total-tokens-per-second",
        zero_allowed: false,
    },
];

const SUITE: Suite = Suite {
    name: "speculation",
    obligation_id: "m1.r32",
    path_id: "speculation-bench",
    source_path: "benches/m1/speculation.rs",
    case_kinds: &[
        "speculative-s1-k16-c8192",
        "speculative-s1-k4-c8192",
        "speculative-s1-k8-c8192",
        "speculative-s8-k4-c8192",
    ],
    extra_identities: &["draft-artifact", "holdout", "target-only-artifact"],
    metrics: METRICS,
    extra_record_attributes: &[
        "acceptance-trace-sha256",
        "admitted-plan-sha256",
    ],
    minimum_warmups: 10,
    minimum_recorded_samples: 30,
    nonclaim: "Structural acceptance authenticates externally collected speculation and target-only records only. It does not recompute the speedup or latency gates, establish an eligible holdout, prove rollback or sampling refinement, qualify performance, or close m1.r32.",
};

fn main() -> ExitCode {
    if env::args_os().nth(1).as_deref() == Some(m1_r32_paired_collector::COMMAND.as_ref()) {
        return m1_r32_paired_collector::main_for_arguments(env::args_os().skip(2).collect());
    }
    if env::args_os().nth(1).as_deref() == Some(m1_r32_speculation_records::COMMAND.as_ref()) {
        return m1_r32_speculation_records::main_for_arguments(env::args_os().skip(2).collect());
    }
    main_for(&SUITE)
}
