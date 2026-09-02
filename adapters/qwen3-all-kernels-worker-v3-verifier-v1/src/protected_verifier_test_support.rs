//! Test-only construction of signed aggregate service fixtures.

#![cfg(test)]

use ed25519_dalek::{Signer, SigningKey};
use fe2o3_host::WorkerV3SafetyPropertiesV1;

use crate::M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1;
use crate::protected_receipt::{
    M1AllKernelsProtectedReceiptCompilerClaimsV1, M1AllKernelsProtectedReceiptEntryV1,
    M1AllKernelsProtectedReceiptRequestClaimsV1, M1AllKernelsProtectedReceiptSourcePinV1,
    M1AllKernelsProtectedVerifierReceiptV1, M1AllKernelsProtectedVerifierTrustPolicyV1,
    M1AllKernelsUnsignedProtectedVerifierReceiptV1,
};
use crate::protected_verifier_service::{
    M1AllKernelsProtectedVerifierServiceEntryV1, M1AllKernelsProtectedVerifierServiceRequestV1,
};

pub(crate) const fn identity(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn source_pin() -> M1AllKernelsProtectedReceiptSourcePinV1 {
    M1AllKernelsProtectedReceiptSourcePinV1::new(
        identity(5),
        5_001,
        identity(6),
        6_001,
        identity(7),
        7_001,
    )
    .unwrap()
}

fn request_claims() -> M1AllKernelsProtectedReceiptRequestClaimsV1 {
    M1AllKernelsProtectedReceiptRequestClaimsV1::new(
        identity(1),
        identity(2),
        identity(3),
        identity(4),
        source_pin(),
        identity(8),
        identity(9),
        identity(10),
        identity(11),
        11_001,
    )
    .unwrap()
}

fn compiler_claims() -> M1AllKernelsProtectedReceiptCompilerClaimsV1 {
    M1AllKernelsProtectedReceiptCompilerClaimsV1::new(
        identity(20),
        identity(21),
        identity(22),
        identity(23),
        identity(24),
        identity(25),
        identity(26),
        identity(27),
        identity(28),
        29,
        [0; 32],
        identity(30),
        identity(31),
        identity(32),
        identity(33),
        identity(34),
        identity(35),
    )
    .unwrap()
}

fn receipt_entries() -> [M1AllKernelsProtectedReceiptEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1]
{
    core::array::from_fn(|index| {
        let seed = u8::try_from(index).unwrap();
        M1AllKernelsProtectedReceiptEntryV1::new(
            u16::from(seed),
            identity(50 + seed),
            identity(70 + seed),
            identity(90 + (seed % 3)),
            identity(110 + seed),
            identity(130 + seed),
            identity(150 + seed),
            WorkerV3SafetyPropertiesV1::required(),
        )
        .unwrap()
    })
}

pub(crate) fn signed_fixture_with_seed(
    signing_seed: u8,
) -> (
    M1AllKernelsProtectedVerifierTrustPolicyV1,
    M1AllKernelsProtectedVerifierReceiptV1,
) {
    let signing = SigningKey::from_bytes(&[signing_seed; 32]);
    let policy = M1AllKernelsProtectedVerifierTrustPolicyV1::new(
        signing.verifying_key().to_bytes(),
        identity(0xa1),
        identity(0xa2),
    )
    .unwrap();
    let unsigned = M1AllKernelsUnsignedProtectedVerifierReceiptV1::new(
        policy.identity(),
        request_claims(),
        compiler_claims(),
        identity(0xa1),
        identity(0xa2),
        identity(0xa3),
        receipt_entries(),
    )
    .unwrap();
    let signature = signing.sign(&unsigned.signing_bytes()).to_bytes();
    (policy, unsigned.attach_signature(signature))
}

pub(crate) fn signed_fixture() -> (
    M1AllKernelsProtectedVerifierTrustPolicyV1,
    M1AllKernelsProtectedVerifierReceiptV1,
) {
    signed_fixture_with_seed(0x91)
}

pub(crate) fn fixture_request(
    policy: &M1AllKernelsProtectedVerifierTrustPolicyV1,
    receipt: &M1AllKernelsProtectedVerifierReceiptV1,
) -> M1AllKernelsProtectedVerifierServiceRequestV1 {
    let entries = core::array::from_fn(|index| {
        M1AllKernelsProtectedVerifierServiceEntryV1::from_receipt_entry(&receipt.entries()[index])
    });
    M1AllKernelsProtectedVerifierServiceRequestV1::new(
        policy.identity(),
        *receipt.request_claims(),
        *receipt.compiler_claims(),
        entries,
    )
    .unwrap()
}
