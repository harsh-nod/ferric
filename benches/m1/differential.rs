#![forbid(unsafe_code)]

//! Target-only differential run-plan and real-record ingestion boundary.

use ferric_m1_benchmarks::{main_for, Metric, Suite};
use std::process::ExitCode;
#[allow(unused_imports)]
use vstd::prelude::*;

const METRICS: &[Metric] = &[
    Metric {
        id: "compared-logits",
        zero_allowed: false,
    },
    Metric {
        id: "compared-tokens",
        zero_allowed: false,
    },
    Metric {
        id: "maximum-logit-ulp-error",
        zero_allowed: true,
    },
    Metric {
        id: "token-mismatches",
        zero_allowed: true,
    },
];

const SUITE: Suite = Suite {
    name: "differential",
    obligation_id: "m1.r29",
    path_id: "differential-bench",
    source_path: "benches/m1/differential.rs",
    case_kinds: &[
        "decode-s1-c8192",
        "decode-s32-c8192",
        "decode-s8-c8192",
        "prefill-s1-t128",
        "prefill-s1-t2048",
        "prefill-s1-t512",
        "prefill-s8-t128",
    ],
    extra_identities: &["reference-implementation", "reference-protocol"],
    metrics: METRICS,
    extra_record_attributes: &[
        "ferric-output-sha256",
        "reference-output-sha256",
    ],
    minimum_warmups: 0,
    minimum_recorded_samples: 1,
    nonclaim: "Structural acceptance authenticates externally collected target-only differential records only. It does not validate a logit tolerance, prove token equality, establish numerical or hardware correctness, qualify performance, or close m1.r29.",
};

fn main() -> ExitCode {
    main_for(&SUITE)
}
