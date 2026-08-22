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
    0x2e, 0x07, 0x58, 0x2d, 0x59, 0x34, 0x37, 0xed, 0x13, 0x2a, 0x8e, 0x1b, 0x0b, 0x3f, 0x57, 0xbc,
    0xb9, 0xa8, 0x4e, 0xde, 0x9a, 0x64, 0x97, 0x14, 0xa8, 0x96, 0x39, 0x0c, 0x6c, 0xdf, 0x83, 0xef,
];

/// Exact byte length of the reviewed native Worker executable.
pub const M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1: u64 = 86_099_920;

/// Reviewed build claim embedded by the exact Worker executable.
pub const M1_KERNEL_WORKER_BUILD_IDENTITY_V1: &str =
    "fe2o3-worker-v1-sha256-407aa6e85ffde03a20d38e359af387184cabc187dd478ac404f183b4043db497";

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
        assert_eq!(M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1, 86_099_920);
        assert!(M1_KERNEL_WORKER_BUILD_IDENTITY_V1.starts_with("fe2o3-worker-v1-sha256-"));
        assert_eq!(
            M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
            fe2o3_llvm_worker_handoff::EXACT_LLVM_BUILD_IDENTITY_V1
        );
    }
}
