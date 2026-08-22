//! Ferric-owned measurement policy for the M1 K1-K7 LLVM worker.
//!
//! These values are copied from the reviewed, ready release manifest retained
//! by Ferric's pinned fe2o3 revision. A caller supplies only the executable
//! pathname; it cannot substitute the executable, Worker, or LLVM identities.

use std::path::Path;

use fe2o3_hsaco_finalize::{
    ContentIdentityV1, PinnedWorkerV1, WorkerExecutionError, WorkerMeasurementV1,
};

/// SHA-256 of the exact reviewed native Worker executable.
pub const M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1: [u8; 32] = [
    0x78, 0x9d, 0xec, 0xbc, 0xe7, 0x9f, 0xbc, 0x97, 0x55, 0xfe, 0xb8, 0xc2, 0x7a, 0x94, 0x57, 0xb1,
    0xcf, 0x61, 0x98, 0xba, 0x81, 0xd9, 0xef, 0x12, 0x66, 0x7a, 0x90, 0x54, 0x34, 0x8e, 0x84, 0xf8,
];

/// Exact byte length of the reviewed native Worker executable.
pub const M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1: u64 = 86_023_808;

/// Reviewed build claim embedded by the exact Worker executable.
pub const M1_KERNEL_WORKER_BUILD_IDENTITY_V1: &str =
    "fe2o3-worker-v1-sha256-8a45ff88db4c886f6b4b789fc6572ea9dcec7391daacd994a3f6b85f138fd18d";

/// Exact upstream LLVM/LLD build identity required by every K1-K7 request.
pub const M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1: &str =
    "upstream-llvmorg-22.1.8-ca7933e47d3a3451d81e72ac174dcb5aa28b59d1";

pub(crate) fn open_m1_kernel_worker_v1(
    path: &Path,
) -> Result<PinnedWorkerV1, WorkerExecutionError> {
    let measurement = WorkerMeasurementV1::new(
        ContentIdentityV1::from_parts(
            M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1,
            M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
        ),
        M1_KERNEL_WORKER_BUILD_IDENTITY_V1,
        M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
    )?;
    PinnedWorkerV1::open(path, measurement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewed_worker_measurement_is_exact_and_nonzero() {
        assert_ne!(M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1, [0; 32]);
        assert_eq!(M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1, 86_023_808);
        assert!(M1_KERNEL_WORKER_BUILD_IDENTITY_V1.starts_with("fe2o3-worker-v1-sha256-"));
        assert_eq!(
            M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
            fe2o3_llvm_worker_handoff::EXACT_LLVM_BUILD_IDENTITY_V1
        );
    }
}
