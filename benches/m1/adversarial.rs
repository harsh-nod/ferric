#![forbid(unsafe_code)]

//! Adversarial run-plan and real fault-observation ingestion boundary.

use ferric_m1_benchmarks::{main_for, Metric, Suite};
use std::process::ExitCode;
#[allow(unused_imports)]
use vstd::prelude::*;

const METRICS: &[Metric] = &[
    Metric {
        id: "canary-corruptions",
        zero_allowed: true,
    },
    Metric {
        id: "faults-observed",
        zero_allowed: true,
    },
    Metric {
        id: "leaked-resources",
        zero_allowed: true,
    },
    Metric {
        id: "rollback-mismatches",
        zero_allowed: true,
    },
    Metric {
        id: "unexpected-errors",
        zero_allowed: true,
    },
];

const SUITE: Suite = Suite {
    name: "adversarial",
    obligation_id: "m1.r30",
    path_id: "adversarial-bench",
    source_path: "benches/m1/adversarial.rs",
    case_kinds: &[
        "canary",
        "cancellation",
        "exhaustion",
        "fault-injection",
        "rollback",
    ],
    extra_identities: &["canary-layout", "fault-plan"],
    metrics: METRICS,
    extra_record_attributes: &["fault-roster-sha256"],
    minimum_warmups: 0,
    minimum_recorded_samples: 1,
    nonclaim: "Structural acceptance authenticates externally collected adversarial records only. It does not establish canary integrity, cancellation safety, exhaustion handling, rollback refinement, fault coverage, hardware correctness, or close m1.r30.",
};

fn main() -> ExitCode {
    main_for(&SUITE)
}
