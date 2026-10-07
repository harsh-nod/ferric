//! Ferric-owned measurement policy for the M1 K1-K7 LLVM worker.
//!
//! This module is Ferric's reviewed M1 Worker release policy record. The pins
//! are independent of self-reported Worker evidence and of the pinned fe2o3
//! source revision. Every compiler-produced Worker V3 owner must retain this
//! exact executable, Worker, and LLVM measurement.

use fe2o3_hsaco_finalize::{
    ContentIdentityV1, LinkOptionV1, WorkerExecutionLimitsV1, WorkerMeasurementV1,
};

const M1_KERNEL_WORKER_LINK_OPTIONS_V1: [(&str, &str); 4] = [
    ("code-object-version", "6"),
    ("opt-level", "2"),
    ("strip-debug", "true"),
    ("verify-each", "true"),
];

/// SHA-256 of the exact reviewed native Worker executable.
pub const M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1: [u8; 32] = [
    0x8a, 0x69, 0x85, 0x0e, 0xa8, 0x9f, 0xe4, 0x45, 0xb5, 0xdd, 0x5a, 0x3e, 0x03, 0x16, 0x7f, 0x84,
    0x98, 0xa4, 0xd9, 0x11, 0xec, 0x58, 0x11, 0x01, 0xbf, 0x32, 0xaa, 0xa5, 0x0a, 0x08, 0x0f, 0x1a,
];

/// Exact byte length of the reviewed native Worker executable.
pub const M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1: u64 = 86_133_472;

/// Reviewed build claim embedded by the exact Worker executable.
pub const M1_KERNEL_WORKER_BUILD_IDENTITY_V1: &str =
    "fe2o3-worker-v1-sha256-d2d998b3f7f228a3b7449d21f6e908d66931948f4072aeaf90460908a400e87f";

/// Exact upstream LLVM/LLD build identity required by every K1-K7 request.
pub const M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1: &str =
    "upstream-llvmorg-22.1.8-ca7933e47d3a3451d81e72ac174dcb5aa28b59d1";

pub(crate) fn worker_measurement_matches_m1_kernel_policy_v1(
    measurement: &WorkerMeasurementV1,
) -> bool {
    measurement.executable()
        == ContentIdentityV1::from_parts(
            M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1,
            M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
        )
        && measurement.worker_build_identity() == M1_KERNEL_WORKER_BUILD_IDENTITY_V1
        && measurement.llvm_build_identity() == M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1
}

pub(crate) fn worker_execution_policy_matches_m1_kernel_policy_v1(
    limits: WorkerExecutionLimitsV1,
    options: &[LinkOptionV1],
) -> bool {
    limits == WorkerExecutionLimitsV1::default()
        && options.len() == M1_KERNEL_WORKER_LINK_OPTIONS_V1.len()
        && options
            .iter()
            .zip(M1_KERNEL_WORKER_LINK_OPTIONS_V1)
            .all(|(actual, expected)| actual.name() == expected.0 && actual.value() == expected.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewed_worker_measurement_is_exact_and_nonzero() {
        assert_ne!(M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1, [0; 32]);
        assert_eq!(M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1, 86_133_472);
        assert!(M1_KERNEL_WORKER_BUILD_IDENTITY_V1.starts_with("fe2o3-worker-v1-sha256-"));
        assert_eq!(
            M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
            ferric_qwen_kernels::QWEN3_LLVM_BUILD_IDENTITY_V1
        );
    }

    #[test]
    fn worker_measurement_policy_rejects_every_substitution_axis() {
        fn measurement(
            sha256: [u8; 32],
            byte_len: u64,
            worker_build: &str,
            llvm_build: &str,
        ) -> WorkerMeasurementV1 {
            WorkerMeasurementV1::new(
                ContentIdentityV1::from_parts(sha256, byte_len),
                worker_build,
                llvm_build,
            )
            .expect("valid test measurement")
        }

        let exact = measurement(
            M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1,
            M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
            M1_KERNEL_WORKER_BUILD_IDENTITY_V1,
            M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
        );
        assert!(worker_measurement_matches_m1_kernel_policy_v1(&exact));

        let mut changed_sha256 = M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1;
        changed_sha256[0] ^= 1;
        for substituted in [
            measurement(
                changed_sha256,
                M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
                M1_KERNEL_WORKER_BUILD_IDENTITY_V1,
                M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
            ),
            measurement(
                M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1,
                M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1 + 1,
                M1_KERNEL_WORKER_BUILD_IDENTITY_V1,
                M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
            ),
            measurement(
                M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1,
                M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
                "substituted-worker-build-v1",
                M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
            ),
            measurement(
                M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1,
                M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
                M1_KERNEL_WORKER_BUILD_IDENTITY_V1,
                "substituted-llvm-build-v1",
            ),
        ] {
            assert!(!worker_measurement_matches_m1_kernel_policy_v1(
                &substituted
            ));
        }
    }

    #[test]
    fn worker_execution_policy_rejects_every_substitution_axis() {
        use std::time::Duration;

        fn options(values: &[(&str, &str)]) -> Vec<LinkOptionV1> {
            values
                .iter()
                .map(|(name, value)| LinkOptionV1::new(*name, *value).expect("valid test option"))
                .collect()
        }

        let exact_options = options(&M1_KERNEL_WORKER_LINK_OPTIONS_V1);
        let exact_limits = WorkerExecutionLimitsV1::default();
        assert!(worker_execution_policy_matches_m1_kernel_policy_v1(
            exact_limits,
            &exact_options
        ));

        for substituted in [
            options(&M1_KERNEL_WORKER_LINK_OPTIONS_V1[..3]),
            options(&[
                ("code-object-version", "5"),
                ("opt-level", "2"),
                ("strip-debug", "true"),
                ("verify-each", "true"),
            ]),
            options(&[
                ("code-object-version", "6"),
                ("opt-level", "3"),
                ("strip-debug", "true"),
                ("verify-each", "true"),
            ]),
            options(&[
                ("code-object-version", "6"),
                ("opt-level", "2"),
                ("strip-debug", "false"),
                ("verify-each", "true"),
            ]),
            options(&[
                ("code-object-version", "6"),
                ("opt-level", "2"),
                ("strip-debug", "true"),
                ("verify-each", "false"),
            ]),
            options(&[
                ("opt-level", "2"),
                ("code-object-version", "6"),
                ("strip-debug", "true"),
                ("verify-each", "true"),
            ]),
        ] {
            assert!(!worker_execution_policy_matches_m1_kernel_policy_v1(
                exact_limits,
                &substituted
            ));
        }

        for substituted in [
            WorkerExecutionLimitsV1::new(
                exact_limits
                    .timeout()
                    .checked_sub(Duration::from_millis(1))
                    .expect("default timeout exceeds one millisecond"),
                exact_limits.stdout_bytes(),
                exact_limits.stderr_bytes(),
            )
            .expect("valid timeout substitution"),
            WorkerExecutionLimitsV1::new(
                exact_limits.timeout(),
                exact_limits.stdout_bytes() - 1,
                exact_limits.stderr_bytes(),
            )
            .expect("valid stdout substitution"),
            WorkerExecutionLimitsV1::new(
                exact_limits.timeout(),
                exact_limits.stdout_bytes(),
                exact_limits.stderr_bytes() - 1,
            )
            .expect("valid stderr substitution"),
        ] {
            assert!(!worker_execution_policy_matches_m1_kernel_policy_v1(
                substituted,
                &exact_options
            ));
        }
    }
}
