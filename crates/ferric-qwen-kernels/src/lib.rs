#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

#[allow(unused_imports)]
use vstd::prelude::*;

use fe2o3_compiler_ffi::CodeObjectVersion;
use fe2o3_hsaco_finalize::{
    WorkerDeviceLibraryProviderEvidenceV1, WorkerOptimizationLevelV1, WorkerOptionsV1,
};

const COV6_NO_RUNTIME_SERVICE_ATTRIBUTES_V1: [&str; 6] = [
    "\"amdgpu-no-completion-action\"",
    "\"amdgpu-no-default-queue\"",
    "\"amdgpu-no-heap-ptr\"",
    "\"amdgpu-no-hostcall-ptr\"",
    "\"amdgpu-no-multigrid-sync-arg\"",
    "\"amdgpu-no-queue-ptr\"",
];

/// Exact LLVM/LLD build identity in Ferric's reviewed M1 Worker measurement.
pub const QWEN3_LLVM_BUILD_IDENTITY_V1: &str =
    "upstream-llvmorg-22.1.8-ca7933e47d3a3451d81e72ac174dcb5aa28b59d1";

/// Exact direct-LLVM Worker options required by every Ferric Qwen kernel lane.
pub const QWEN3_WORKER_OPTIONS_V1: WorkerOptionsV1 =
    WorkerOptionsV1::new(WorkerOptimizationLevelV1::O2, true, true);

/// Exact provider identity for Ferric's reviewed `ROCm` 7.2.4 gfx942 OCML closure.
pub const QWEN3_GFX942_OCML_PROVIDER_IDENTITY_V1: &str = "gfx942-ocml-v1";

/// Exact target for Ferric's reviewed gfx942 OCML closure.
pub const QWEN3_GFX942_OCML_PROVIDER_TARGET_V1: &str = "gfx942:xnack-";

/// Sole device-library import admitted by Ferric's reviewed gfx942 OCML closure.
pub const QWEN3_GFX942_OCML_IMPORT_V1: &str = "__ocml_exp_f32";

/// Ordered, content-addressed files in Ferric's reviewed `ROCm` 7.2.4 gfx942 OCML closure.
pub const QWEN3_GFX942_OCML_PROVIDER_FILES_V1: [(&str, [u8; 32]); 4] = [
    (
        "ocml.bc",
        [
            0xcf, 0xe9, 0x7f, 0xe9, 0xee, 0x29, 0x37, 0x9f, 0x52, 0x2e, 0x5f, 0x20, 0xae, 0x55,
            0xaa, 0xe1, 0xcd, 0xb9, 0x6e, 0xb4, 0x1d, 0x6a, 0xa2, 0x50, 0xea, 0x11, 0xc4, 0x94,
            0x1c, 0x54, 0xe0, 0x19,
        ],
    ),
    (
        "oclc_isa_version_942.bc",
        [
            0x58, 0x0d, 0x54, 0x0c, 0xc7, 0x38, 0xc0, 0xf9, 0x55, 0x4c, 0x87, 0x10, 0x57, 0x5b,
            0xbc, 0x9b, 0x51, 0xeb, 0xac, 0xdc, 0xbc, 0x29, 0xaa, 0x00, 0x74, 0xed, 0x05, 0xd3,
            0x69, 0x1d, 0xea, 0x1d,
        ],
    ),
    (
        "oclc_unsafe_math_off.bc",
        [
            0x22, 0xc7, 0x99, 0xb9, 0x15, 0x43, 0x89, 0xf0, 0x50, 0xf8, 0xf3, 0x36, 0x87, 0x62,
            0x63, 0x6b, 0x99, 0x54, 0xa2, 0xea, 0x25, 0x62, 0x21, 0x99, 0xc3, 0x59, 0x36, 0x6b,
            0xbd, 0x84, 0x65, 0x7f,
        ],
    ),
    (
        "oclc_finite_only_off.bc",
        [
            0xf3, 0x13, 0x8e, 0xee, 0xe6, 0x5c, 0x1d, 0x83, 0x23, 0x42, 0x60, 0x72, 0x8d, 0x12,
            0x4f, 0x63, 0x5f, 0x02, 0x1a, 0xbb, 0x37, 0xc4, 0x95, 0xf4, 0xed, 0x02, 0x7d, 0xfe,
            0x92, 0xbc, 0xb1, 0xdd,
        ],
    ),
];

/// SHA-256 identity of the exact canonical provider-evidence manifest above.
pub const QWEN3_GFX942_OCML_PROVIDER_MANIFEST_SHA256_V1: [u8; 32] = [
    0xe7, 0xa3, 0x92, 0x4a, 0x5b, 0xda, 0x6e, 0xb5, 0xb6, 0x2a, 0xca, 0x82, 0x6d, 0x61, 0x33, 0x76,
    0x69, 0x62, 0xfc, 0x9f, 0x6d, 0x75, 0x8f, 0xa9, 0x61, 0xdf, 0xee, 0x67, 0x4e, 0x31, 0xd7, 0xf9,
];

pub(crate) fn exact_qwen3_gfx942_ocml_provider_v1(
    provider: &WorkerDeviceLibraryProviderEvidenceV1,
) -> bool {
    provider.provider_identity() == QWEN3_GFX942_OCML_PROVIDER_IDENTITY_V1
        && provider.target().to_string() == QWEN3_GFX942_OCML_PROVIDER_TARGET_V1
        && provider.code_object_version() == CodeObjectVersion::V6
        && provider.import_symbols() == [QWEN3_GFX942_OCML_IMPORT_V1]
        && provider.manifest_identity() == &QWEN3_GFX942_OCML_PROVIDER_MANIFEST_SHA256_V1
        && provider.files().len() == QWEN3_GFX942_OCML_PROVIDER_FILES_V1.len()
        && provider.files().iter().enumerate().all(|(index, file)| {
            exact_qwen3_gfx942_ocml_provider_file_v1(index, file.basename(), file.sha256())
        })
}

fn exact_qwen3_gfx942_ocml_provider_file_v1(
    index: usize,
    basename: &str,
    sha256: &[u8; 32],
) -> bool {
    QWEN3_GFX942_OCML_PROVIDER_FILES_V1
        .get(index)
        .is_some_and(|expected| basename == expected.0 && sha256 == &expected.1)
}

/// Exact finite Qwen3 dense GEMM/GEMV compiler profiles.
pub mod gemm;
/// Exact Qwen3 lowest-ID argmax and compact-completion compiler profiles.
pub mod logits;
/// Exact Qwen3 paged-GQA decode and speculative-attention compiler profiles.
pub mod paged_decode;
/// Exact Qwen3 causal-prefill compiler profiles.
pub mod prefill;
/// Exact Qwen3 RMSNorm and explicitly residual-fused compiler profiles.
pub mod rmsnorm;
/// Exact Qwen3 split-half RoPE and global-pool P16 paged-KV compiler profiles.
pub mod rope_kv;
/// Exact Qwen3 SwiGLU compiler profiles.
pub mod swiglu;

#[cfg(test)]
mod tests {
    use sha2::{Digest as _, Sha256};

    use super::*;

    #[test]
    fn reviewed_gfx942_ocml_provider_manifest_identity_is_exact() {
        let mut preimage = Vec::new();
        for value in [
            QWEN3_GFX942_OCML_PROVIDER_IDENTITY_V1,
            QWEN3_GFX942_OCML_PROVIDER_TARGET_V1,
        ] {
            preimage.extend_from_slice(&u32::try_from(value.len()).unwrap().to_le_bytes());
            preimage.extend_from_slice(value.as_bytes());
        }
        preimage.push(6);
        preimage.extend_from_slice(&1_u32.to_le_bytes());
        preimage.extend_from_slice(
            &u32::try_from(QWEN3_GFX942_OCML_IMPORT_V1.len())
                .unwrap()
                .to_le_bytes(),
        );
        preimage.extend_from_slice(QWEN3_GFX942_OCML_IMPORT_V1.as_bytes());
        preimage.extend_from_slice(
            &u32::try_from(QWEN3_GFX942_OCML_PROVIDER_FILES_V1.len())
                .unwrap()
                .to_le_bytes(),
        );
        for (basename, sha256) in QWEN3_GFX942_OCML_PROVIDER_FILES_V1 {
            preimage.extend_from_slice(&u32::try_from(basename.len()).unwrap().to_le_bytes());
            preimage.extend_from_slice(basename.as_bytes());
            preimage.extend_from_slice(&sha256);
        }

        assert_eq!(preimage.len(), 282);
        let mut hasher = Sha256::new();
        hasher.update(b"FE2O3/DEVICE-LIBRARY-PROVIDER-MANIFEST/V1\0");
        hasher.update((preimage.len() as u64).to_le_bytes());
        hasher.update(&preimage);
        assert_eq!(
            <[u8; 32]>::from(hasher.finalize()),
            QWEN3_GFX942_OCML_PROVIDER_MANIFEST_SHA256_V1
        );
    }

    #[test]
    fn reviewed_gfx942_ocml_provider_files_reject_each_substitution_axis() {
        for (index, (basename, sha256)) in
            QWEN3_GFX942_OCML_PROVIDER_FILES_V1.into_iter().enumerate()
        {
            assert!(exact_qwen3_gfx942_ocml_provider_file_v1(
                index, basename, &sha256
            ));
            assert!(!exact_qwen3_gfx942_ocml_provider_file_v1(
                index,
                "substituted.bc",
                &sha256
            ));
            let mut substituted_sha256 = sha256;
            substituted_sha256[0] ^= 1;
            assert!(!exact_qwen3_gfx942_ocml_provider_file_v1(
                index,
                basename,
                &substituted_sha256
            ));
        }
        assert!(!exact_qwen3_gfx942_ocml_provider_file_v1(
            QWEN3_GFX942_OCML_PROVIDER_FILES_V1.len(),
            QWEN3_GFX942_OCML_PROVIDER_FILES_V1[0].0,
            &QWEN3_GFX942_OCML_PROVIDER_FILES_V1[0].1,
        ));
    }
}
