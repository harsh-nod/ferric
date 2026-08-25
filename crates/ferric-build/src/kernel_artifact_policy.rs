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
        assert_eq!(M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1, 86_133_472);
        assert!(M1_KERNEL_WORKER_BUILD_IDENTITY_V1.starts_with("fe2o3-worker-v1-sha256-"));
        assert_eq!(
            M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
            fe2o3_llvm_worker_handoff::EXACT_LLVM_BUILD_IDENTITY_V1
        );
    }
}
