//! Source policy for the default rejection backend and configured binder.

use fe2o3_host::CompilerGeneratedKernelExpectationRosterV1;
use ferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;
use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprMethodCall, Fields, File, ImplItem, ImplItemFn, Item, ItemFn,
    ItemImpl, ItemMod, ItemUse, Meta, Path, ReturnType, Type, UseTree, Visibility,
};

const SOURCE: &str = include_str!("../src/lib.rs");
const RECEIPT_SOURCE: &str = include_str!("../src/protected_receipt.rs");
const CLIENT_SOURCE: &str = include_str!("../src/protected_verifier_client.rs");
const SERVICE_SOURCE: &str = include_str!("../src/protected_verifier_service.rs");
const TEST_SUPPORT_SOURCE: &str = include_str!("../src/protected_verifier_test_support.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const LOCKFILE: &str = include_str!("../Cargo.lock");
const README: &str = include_str!("../README.md");
const FE2O3_REVISION: &str = "43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb";
const TEST_MODULE_BOUNDARY: &str = "#[cfg(test)]\nmod tests {";
const BACKEND_TRAIT: &str =
    "WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1>";
const ZERO_TYPE: &str = "M1AllKernelsProtectedVerifierV1";
const CONFIGURED_TYPE_NAME: &str = "M1AllKernelsProductionProtectedVerifierV1";
const EVIDENCE_TYPE: &str = "WorkerV3ProtectedRosterVerificationEvidenceV1";
const EVIDENCE_CONSTRUCTOR: &str = "WorkerV3ProtectedRosterVerificationEvidenceV1::new";
// Co-located review tripwires. They provide no external provenance or authority.
const REVIEWED_FILE_SHA256: [[u8; 32]; 7] = [
    [
        167, 250, 228, 102, 146, 244, 60, 112, 211, 29, 113, 127, 101, 126, 17, 222, 245, 85, 240,
        223, 174, 133, 21, 22, 36, 176, 69, 95, 201, 115, 227, 47,
    ],
    [
        195, 119, 11, 143, 50, 4, 217, 48, 95, 184, 90, 183, 252, 155, 177, 118, 152, 33, 3, 19,
        20, 99, 98, 151, 180, 221, 112, 70, 109, 36, 46, 208,
    ],
    [
        149, 7, 11, 108, 45, 206, 232, 124, 186, 215, 118, 157, 86, 127, 149, 124, 24, 136, 69, 89,
        28, 152, 109, 119, 37, 91, 63, 137, 165, 176, 192, 40,
    ],
    [
        119, 0, 112, 223, 146, 53, 127, 254, 153, 252, 82, 174, 50, 184, 138, 184, 213, 153, 59,
        53, 233, 97, 103, 86, 93, 131, 62, 32, 183, 156, 25, 216,
    ],
    [
        25, 93, 20, 131, 237, 148, 170, 105, 134, 49, 139, 125, 151, 2, 246, 2, 243, 77, 238, 95,
        4, 53, 9, 149, 204, 198, 78, 167, 240, 54, 180, 86,
    ],
    [
        185, 49, 80, 79, 192, 88, 79, 24, 149, 3, 32, 43, 160, 67, 57, 217, 42, 227, 252, 204, 86,
        59, 147, 76, 126, 0, 245, 225, 29, 220, 75, 43,
    ],
    [
        247, 161, 224, 195, 78, 210, 75, 123, 141, 104, 233, 216, 134, 253, 160, 198, 187, 40, 38,
        14, 190, 34, 138, 16, 149, 165, 35, 183, 208, 189, 80, 198,
    ],
];
const REVIEWED_LIB_NODE_FINGERPRINTS: [[u8; 32]; 23] = [
    [
        50, 238, 77, 41, 217, 199, 224, 23, 244, 21, 119, 56, 197, 141, 198, 227, 9, 253, 142, 86,
        105, 130, 43, 38, 186, 66, 185, 88, 156, 137, 112, 178,
    ],
    [
        150, 149, 86, 54, 20, 230, 199, 123, 95, 31, 49, 205, 239, 139, 160, 98, 69, 102, 132, 75,
        21, 147, 137, 185, 212, 53, 179, 204, 144, 170, 145, 27,
    ],
    [
        41, 121, 49, 173, 169, 173, 232, 189, 229, 39, 74, 109, 189, 22, 211, 20, 168, 20, 74, 76,
        146, 145, 192, 116, 24, 222, 126, 147, 224, 126, 184, 135,
    ],
    [
        185, 149, 174, 242, 86, 103, 187, 71, 85, 247, 206, 244, 15, 6, 55, 114, 193, 189, 107,
        220, 99, 188, 236, 246, 132, 118, 53, 141, 74, 156, 203, 242,
    ],
    [
        100, 52, 229, 209, 200, 138, 199, 150, 190, 126, 5, 137, 129, 151, 173, 180, 19, 61, 204,
        44, 76, 220, 71, 14, 23, 69, 5, 166, 198, 191, 169, 2,
    ],
    [
        182, 112, 114, 135, 231, 199, 79, 124, 108, 14, 197, 150, 219, 97, 13, 145, 167, 159, 138,
        227, 219, 151, 225, 127, 242, 231, 225, 218, 82, 14, 163, 206,
    ],
    [
        236, 41, 154, 219, 92, 3, 84, 178, 73, 96, 62, 38, 237, 56, 164, 121, 57, 115, 133, 27, 57,
        189, 221, 144, 146, 21, 66, 50, 176, 64, 194, 85,
    ],
    [
        193, 231, 93, 62, 111, 43, 190, 67, 15, 193, 126, 47, 125, 215, 249, 227, 247, 176, 133,
        102, 183, 241, 180, 99, 69, 86, 42, 188, 208, 190, 188, 252,
    ],
    [
        77, 160, 73, 31, 254, 138, 240, 206, 17, 49, 226, 178, 255, 123, 21, 87, 229, 220, 2, 156,
        217, 23, 147, 8, 239, 11, 85, 1, 112, 241, 106, 118,
    ],
    [
        235, 183, 116, 68, 78, 5, 168, 237, 63, 241, 205, 252, 57, 194, 203, 24, 233, 69, 58, 75,
        178, 113, 68, 119, 12, 143, 83, 75, 61, 142, 0, 55,
    ],
    [
        54, 229, 87, 129, 69, 60, 128, 39, 35, 245, 141, 52, 58, 41, 21, 154, 72, 29, 217, 102, 23,
        176, 91, 240, 65, 220, 239, 171, 103, 110, 226, 136,
    ],
    [
        220, 165, 238, 33, 222, 39, 144, 99, 110, 40, 142, 138, 35, 43, 96, 15, 138, 68, 194, 85,
        213, 33, 120, 111, 101, 116, 237, 241, 210, 254, 211, 13,
    ],
    [
        123, 31, 228, 183, 181, 105, 73, 157, 69, 191, 146, 254, 179, 109, 31, 249, 204, 149, 177,
        245, 186, 168, 169, 69, 126, 136, 165, 36, 203, 162, 139, 167,
    ],
    [
        32, 183, 172, 125, 174, 72, 242, 122, 44, 23, 122, 14, 198, 212, 196, 62, 138, 204, 193,
        88, 220, 50, 1, 30, 57, 223, 0, 46, 136, 50, 169, 26,
    ],
    [
        61, 14, 171, 223, 118, 173, 225, 189, 63, 154, 117, 128, 217, 191, 10, 6, 11, 231, 248, 93,
        228, 218, 156, 160, 86, 223, 214, 108, 60, 40, 164, 39,
    ],
    [
        215, 131, 156, 62, 162, 146, 193, 4, 10, 161, 251, 145, 246, 212, 130, 231, 9, 132, 43,
        211, 0, 233, 210, 99, 5, 26, 231, 195, 150, 35, 253, 59,
    ],
    [
        182, 100, 212, 133, 159, 236, 249, 32, 234, 24, 108, 98, 77, 88, 36, 168, 174, 144, 137,
        195, 74, 64, 131, 146, 166, 83, 238, 207, 29, 114, 224, 29,
    ],
    [
        181, 212, 84, 130, 178, 252, 119, 216, 185, 69, 214, 62, 226, 36, 255, 48, 35, 191, 208,
        217, 27, 13, 86, 148, 219, 63, 31, 52, 83, 89, 119, 193,
    ],
    [
        216, 45, 131, 254, 0, 238, 222, 248, 40, 26, 125, 252, 111, 20, 164, 229, 224, 237, 143,
        207, 21, 49, 99, 53, 143, 133, 112, 32, 224, 159, 7, 104,
    ],
    [
        177, 197, 249, 209, 239, 162, 220, 142, 240, 163, 110, 252, 20, 213, 48, 84, 253, 197, 227,
        160, 17, 212, 112, 198, 17, 1, 113, 157, 158, 212, 226, 96,
    ],
    [
        49, 169, 182, 106, 228, 12, 111, 80, 31, 81, 14, 160, 23, 33, 41, 206, 133, 165, 211, 186,
        183, 254, 162, 102, 134, 147, 122, 214, 107, 63, 83, 105,
    ],
    [
        101, 49, 229, 201, 105, 218, 125, 245, 78, 156, 233, 191, 80, 16, 247, 241, 123, 14, 201,
        102, 198, 127, 41, 38, 216, 145, 131, 20, 6, 188, 49, 234,
    ],
    [
        77, 247, 145, 90, 139, 253, 7, 182, 145, 235, 58, 245, 66, 45, 161, 176, 176, 80, 124, 242,
        124, 174, 251, 154, 146, 74, 160, 83, 52, 117, 111, 168,
    ],
];
const REVIEWED_SIBLING_AST_FINGERPRINTS: [[u8; 32]; 4] = [
    [
        132, 181, 173, 71, 182, 250, 154, 170, 178, 236, 34, 188, 5, 43, 199, 29, 94, 45, 143, 138,
        213, 42, 98, 16, 254, 251, 110, 231, 51, 232, 170, 12,
    ],
    [
        215, 224, 181, 159, 127, 161, 118, 207, 141, 89, 85, 150, 192, 79, 25, 210, 83, 25, 189,
        178, 47, 202, 87, 239, 6, 158, 138, 1, 11, 66, 70, 109,
    ],
    [
        14, 13, 82, 110, 6, 11, 196, 61, 238, 133, 37, 37, 200, 246, 123, 56, 56, 218, 103, 73,
        188, 74, 30, 93, 57, 118, 209, 38, 115, 217, 95, 159,
    ],
    [
        53, 52, 116, 17, 55, 128, 29, 219, 98, 76, 106, 120, 53, 8, 202, 20, 120, 90, 196, 230, 1,
        224, 102, 17, 7, 238, 132, 3, 33, 74, 29, 70,
    ],
];
const EXPECTED_MANIFEST_SEMANTICS: &str = r#"
[package]
name = "ferric-qwen3-all-kernels-worker-v3-verifier-v1"
version = "0.1.0"
edition = "2024"
rust-version = "1.97.1"
publish = false

[workspace]

[dependencies]
ed25519-dalek = { version = "=2.2.0", default-features = false, features = ["fast", "zeroize"] }
fe2o3-host = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb", version = "=0.1.0" }
fe2o3-hsaco-finalize = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb", version = "=0.1.0" }
fe2o3-runtime-protocol = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb", version = "=0.1.0" }
fe2o3-verifier = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb", version = "=0.1.0" }
fe2o3-worker-v3-verification-client = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb", version = "=0.1.0" }
fe2o3-worker-v3-verification-protocol = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb", version = "=0.1.0" }
ferric-qwen3-all-kernels-device-v1 = { path = "../../device/qwen3-all-kernels-v1" }
ferric-qwen3-all-kernels-worker-v3-source-pin-v1 = { path = "../qwen3-all-kernels-worker-v3-source-pin-v1" }
libc = "=0.2.189"
rustix = { version = "=1.1.4", features = ["fs"] }
sha2 = { version = "=0.11.0", default-features = false }

[dev-dependencies]
fe2o3-artifact-transaction = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb", version = "=0.1.0" }
fe2o3-external-anchor-protocol = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb", version = "=0.1.0" }
proc-macro2 = "=1.0.107"
quote = "=1.0.47"
syn = { version = "=2.0.119", features = ["full", "visit"] }
toml = "=1.1.4"

[lints.rust]
missing_docs = "deny"
unsafe_op_in_unsafe_fn = "deny"

[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
"#;
const REVIEWED_FREE_FUNCTIONS: [&str; 13] = [
    "authenticated_entry_coordinates_associate_v1",
    "authenticated_entry_evidence_v1",
    "authenticated_receipt_associates_v1",
    "generic_verification_request_v1",
    "locally_revalidate_request_v1",
    "missing_protected_verification_receipt_v1",
    "payload_snapshot_error_v2",
    "protected_payload_snapshots_v2",
    "protected_service_request_from_current_audit_v1",
    "protected_service_request_v1",
    "protected_service_request_with_compiler_claims_v1",
    "sealed_payload_snapshot_v2",
    "validate_local_request_associations_v1",
];
#[derive(Clone)]
struct AstVerifierPolicySource {
    compact: String,
    zero_rejection: ImplItemFn,
    zero_verify: ImplItemFn,
    configured_constructor: ImplItemFn,
    configured_authority_query: ImplItemFn,
    configured_verify: ImplItemFn,
    local_revalidation: ItemFn,
    local_associations: ItemFn,
    service_request_builder: ItemFn,
    current_audit_service_request_builder: ItemFn,
    shared_service_request_builder: ItemFn,
    generic_request_builder: ItemFn,
    payload_snapshots_builder: ItemFn,
    sealed_snapshot_builder: ItemFn,
    entry_coordinate_join: ItemFn,
    entry_evidence_mapper: ItemFn,
    receipt_association: ItemFn,
}

impl AstVerifierPolicySource {
    #[allow(clippy::too_many_lines)]
    fn parse(source: &str) -> Option<Self> {
        let production = production_file(source)?;
        if !top_level_surface_is_exact(&production) {
            return None;
        }
        if reviewed_lib_node_fingerprints(&production)? != REVIEWED_LIB_NODE_FINGERPRINTS {
            return None;
        }

        let zero_impls = impls_for_type(&production, ZERO_TYPE);
        if zero_impls.len() != 2 {
            return None;
        }
        let zero_inherent = unique_impl(&zero_impls, None, false)?;
        let zero_backend = unique_impl(&zero_impls, Some(BACKEND_TRAIT), true)?;
        if !impl_has_exact_surface(
            zero_inherent,
            &["new", "reject_missing_protected_receipt"],
            &[],
        ) || !impl_has_exact_surface(zero_backend, &["verify_protected_roster"], &["Error"])
        {
            return None;
        }

        let configured_impls = impls_for_type(&production, CONFIGURED_TYPE_NAME);
        if configured_impls.len() != 3 {
            return None;
        }
        let configured_debug = unique_impl(&configured_impls, Some("fmt::Debug"), false)?;
        let configured_inherent = unique_impl(&configured_impls, None, false)?;
        let configured_backend = unique_impl(&configured_impls, Some(BACKEND_TRAIT), true)?;
        if !impl_has_exact_surface(configured_debug, &["fmt"], &[])
            || !impl_has_exact_surface(configured_inherent, &["grants_authority", "new"], &[])
            || !impl_has_exact_surface(configured_backend, &["verify_protected_roster"], &["Error"])
        {
            return None;
        }

        let configured_constructor = unique_impl_function(configured_inherent, "new")?.clone();
        let configured_authority_query =
            unique_impl_function(configured_inherent, "grants_authority")?.clone();
        let configured_verify =
            unique_impl_function(configured_backend, "verify_protected_roster")?.clone();
        let zero_verify = unique_impl_function(zero_backend, "verify_protected_roster")?.clone();
        let zero_rejection =
            unique_impl_function(zero_inherent, "reject_missing_protected_receipt")?.clone();

        let mut surface = ProductionSurface::default();
        surface.visit_file(&production);
        if surface.invalid_attribute
            || surface.invalid_macro
            || surface.invalid_use
            || surface.nested_or_extra_function
            || surface.type_alias
            || surface.return_expressions != 0
            || surface.reviewed_macros != 10
            || surface.evidence_constructor_calls != 1
            || surface.evidence_constructor_references != 1
            || surface.configured_constructor_references != 0
            || surface.verify_references != 0
            || surface.explicit_configured_constructions != 0
            || surface.unsafe_blocks != 2
            || surface.helper_calls != [1, 1, 1, 1, 2, 1, 5, 1, 1, 1, 2, 2, 1]
            || surface.helper_references != surface.helper_calls
            || surface.free_functions
                != REVIEWED_FREE_FUNCTIONS
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
        {
            return None;
        }

        let configured_audit = MethodAudit::from_impl(&configured_verify);
        if configured_audit.evidence_constructor_calls != 1
            || configured_audit.evidence_constructor_references != 1
            || configured_audit.unsafe_blocks != 1
        {
            return None;
        }
        let entry_evidence_mapper =
            unique_free_function(&production, "authenticated_entry_evidence_v1")?.clone();
        let entry_audit = MethodAudit::from_item(&entry_evidence_mapper);
        if entry_audit.unsafe_blocks != 1 {
            return None;
        }

        Some(Self {
            compact: compact_tokens(&production),
            zero_rejection,
            zero_verify,
            configured_constructor,
            configured_authority_query,
            configured_verify,
            local_revalidation: unique_free_function(&production, "locally_revalidate_request_v1")?
                .clone(),
            local_associations: unique_free_function(
                &production,
                "validate_local_request_associations_v1",
            )?
            .clone(),
            service_request_builder: unique_free_function(
                &production,
                "protected_service_request_v1",
            )?
            .clone(),
            current_audit_service_request_builder: unique_free_function(
                &production,
                "protected_service_request_from_current_audit_v1",
            )?
            .clone(),
            shared_service_request_builder: unique_free_function(
                &production,
                "protected_service_request_with_compiler_claims_v1",
            )?
            .clone(),
            generic_request_builder: unique_free_function(
                &production,
                "generic_verification_request_v1",
            )?
            .clone(),
            payload_snapshots_builder: unique_free_function(
                &production,
                "protected_payload_snapshots_v2",
            )?
            .clone(),
            sealed_snapshot_builder: unique_free_function(
                &production,
                "sealed_payload_snapshot_v2",
            )?
            .clone(),
            entry_coordinate_join: unique_free_function(
                &production,
                "authenticated_entry_coordinates_associate_v1",
            )?
            .clone(),
            entry_evidence_mapper,
            receipt_association: unique_free_function(
                &production,
                "authenticated_receipt_associates_v1",
            )?
            .clone(),
        })
    }
}

fn compact_tokens(tokens: &impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn hash_length_prefixed(hasher: &mut Sha256, tag: u8, bytes: &[u8]) {
    hasher.update([tag]);
    hasher.update(
        u64::try_from(bytes.len())
            .expect("token text length fits u64")
            .to_le_bytes(),
    );
    hasher.update(bytes);
}

fn hash_token_stream(hasher: &mut Sha256, stream: TokenStream) {
    hasher.update([0xf0]);
    for token in stream {
        match token {
            TokenTree::Group(group) => {
                let delimiter = match group.delimiter() {
                    Delimiter::Parenthesis => 0,
                    Delimiter::Brace => 1,
                    Delimiter::Bracket => 2,
                    Delimiter::None => 3,
                };
                hasher.update([0x10, delimiter]);
                hash_token_stream(hasher, group.stream());
                hasher.update([0x11]);
            }
            TokenTree::Ident(ident) => {
                hash_length_prefixed(hasher, 0x20, ident.to_string().as_bytes());
            }
            TokenTree::Punct(punct) => {
                hasher.update([0x30]);
                hasher.update(u32::from(punct.as_char()).to_le_bytes());
                hasher.update([match punct.spacing() {
                    Spacing::Alone => 0,
                    Spacing::Joint => 1,
                }]);
            }
            TokenTree::Literal(literal) => {
                hash_length_prefixed(hasher, 0x40, literal.to_string().as_bytes());
            }
        }
    }
    hasher.update([0xf1]);
}

fn token_fingerprint(tokens: &impl ToTokens) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_token_stream(&mut hasher, tokens.to_token_stream());
    hasher.finalize().into()
}

fn source_sha256(source: &str) -> [u8; 32] {
    Sha256::digest(source.as_bytes()).into()
}

fn manifest_semantics_policy(source: &str) -> bool {
    let Ok(actual) = toml::from_str::<toml::Value>(source) else {
        return false;
    };
    let expected = toml::from_str::<toml::Value>(EXPECTED_MANIFEST_SEMANTICS)
        .expect("reviewed manifest semantics parse");
    actual == expected
}

fn lock_string<'a>(package: &'a toml::Table, field: &str) -> Option<&'a str> {
    package.get(field)?.as_str()
}

fn lock_coordinate_matches(
    packages: &[toml::Value],
    name: &str,
    version: &str,
    source: Option<&str>,
    checksum: Option<&str>,
) -> bool {
    let mut matches = packages
        .iter()
        .filter_map(toml::Value::as_table)
        .filter(|package| {
            lock_string(package, "name") == Some(name)
                && lock_string(package, "version") == Some(version)
        });
    let Some(package) = matches.next() else {
        return false;
    };
    matches.next().is_none()
        && lock_string(package, "source") == source
        && lock_string(package, "checksum") == checksum
}

#[allow(clippy::too_many_lines)]
fn lockfile_semantics_policy(source: &str) -> bool {
    let Ok(lockfile) = toml::from_str::<toml::Value>(source) else {
        return false;
    };
    let Some(root) = lockfile.as_table() else {
        return false;
    };
    if root.get("version").and_then(toml::Value::as_integer) != Some(4) {
        return false;
    }
    let Some(packages) = root.get("package").and_then(toml::Value::as_array) else {
        return false;
    };
    let mut verifiers = packages
        .iter()
        .filter_map(toml::Value::as_table)
        .filter(|package| {
            lock_string(package, "name") == Some("ferric-qwen3-all-kernels-worker-v3-verifier-v1")
                && lock_string(package, "version") == Some("0.1.0")
        });
    let Some(verifier) = verifiers.next() else {
        return false;
    };
    if verifiers.next().is_some() {
        return false;
    }
    let expected_dependencies = [
        "ed25519-dalek",
        "fe2o3-artifact-transaction",
        "fe2o3-external-anchor-protocol",
        "fe2o3-host",
        "fe2o3-hsaco-finalize",
        "fe2o3-runtime-protocol",
        "fe2o3-verifier",
        "fe2o3-worker-v3-verification-client",
        "fe2o3-worker-v3-verification-protocol",
        "ferric-qwen3-all-kernels-device-v1",
        "ferric-qwen3-all-kernels-worker-v3-source-pin-v1",
        "libc",
        "proc-macro2",
        "quote",
        "rustix",
        "sha2 0.11.0",
        "syn 2.0.119",
        "toml",
    ];
    if verifier
        .get("dependencies")
        .and_then(toml::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(toml::Value::as_str)
                .collect::<Vec<_>>()
        })
        != Some(expected_dependencies.to_vec())
    {
        return false;
    }

    let registry = Some("registry+https://github.com/rust-lang/crates.io-index");
    let fe2o3 = Some(
        "git+https://github.com/harsh-nod/fe2o3.git?rev=43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb#43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb",
    );
    [
        (
            "ed25519-dalek",
            "2.2.0",
            registry,
            Some("70e796c081cee67dc755e1a36a0a172b897fab85fc3f6bc48307991f64e4eca9"),
        ),
        ("fe2o3-artifact-transaction", "0.1.0", fe2o3, None),
        ("fe2o3-external-anchor-protocol", "0.1.0", fe2o3, None),
        ("fe2o3-host", "0.1.0", fe2o3, None),
        ("fe2o3-hsaco-finalize", "0.1.0", fe2o3, None),
        ("fe2o3-runtime-protocol", "0.1.0", fe2o3, None),
        ("fe2o3-verifier", "0.1.0", fe2o3, None),
        ("fe2o3-worker-v3-verification-client", "0.1.0", fe2o3, None),
        (
            "fe2o3-worker-v3-verification-protocol",
            "0.1.0",
            fe2o3,
            None,
        ),
        ("ferric-qwen3-all-kernels-device-v1", "0.1.0", None, None),
        (
            "ferric-qwen3-all-kernels-worker-v3-source-pin-v1",
            "0.1.0",
            None,
            None,
        ),
        (
            "libc",
            "0.2.189",
            registry,
            Some("3eaf3ede3fee6db1a4c2ee091bf8a8b4dccdc6d17f656fb07896ee72867612f2"),
        ),
        (
            "proc-macro2",
            "1.0.107",
            registry,
            Some("985e7ec9bb745e6ce6535b544d84d6cd6f7ad8bd711c398938ae983b91a766d9"),
        ),
        (
            "quote",
            "1.0.47",
            registry,
            Some("1fbf4db142a473a8d80c26bbf18454ed458bf8d26c8219c331daecfdbd079001"),
        ),
        (
            "rustix",
            "1.1.4",
            registry,
            Some("b6fe4565b9518b83ef4f91bb47ce29620ca828bd32cb7e408f0062e9930ba190"),
        ),
        (
            "sha2",
            "0.11.0",
            registry,
            Some("446ba717509524cb3f22f17ecc096f10f4822d76ab5c0b9822c5f9c284e825f4"),
        ),
        (
            "syn",
            "2.0.119",
            registry,
            Some("872831b642d1a07999a962a351ed35b955ea2cfc8f3862091e2a240a84f17297"),
        ),
        (
            "toml",
            "1.1.4+spec-1.1.0",
            registry,
            Some("3aace63f4bbcdfc2c965b059de67119c89c4017a70d633be6c104910f67056f5"),
        ),
    ]
    .iter()
    .all(|(name, version, package_source, checksum)| {
        lock_coordinate_matches(packages, name, version, *package_source, *checksum)
    })
}

fn item_tokens(tokens: &impl ToTokens) -> String {
    compact_tokens(tokens)
}

fn compact_text(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn path_text(path: &Path) -> String {
    compact_tokens(path)
}

fn path_is(path: &Path, expected: &str) -> bool {
    path_text(path) == expected
}

fn path_ends_with(path: &Path, expected: &[&str]) -> bool {
    path.segments.len() >= expected.len()
        && path
            .segments
            .iter()
            .rev()
            .zip(expected.iter().rev())
            .all(|(segment, expected)| segment.ident == expected)
}

fn type_ends_with(ty: &Type, expected: &str) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none() && path_ends_with(&path.path, &[expected])
}

fn cfg_test_attribute(attribute: &Attribute) -> bool {
    let Meta::List(meta) = &attribute.meta else {
        return false;
    };
    meta.path.is_ident("cfg") && compact_tokens(&meta.tokens) == "test"
}

fn production_file(source: &str) -> Option<File> {
    let mut file = syn::parse_file(source).ok()?;
    let cfg_test_modules = file
        .items
        .iter()
        .filter(
            |item| matches!(item, Item::Mod(module) if module.content.is_some() && module.attrs.iter().any(cfg_test_attribute)),
        )
        .count();
    if cfg_test_modules != 1 {
        return None;
    }
    let Item::Mod(test_module) = file.items.last()? else {
        return None;
    };
    if test_module.ident != "tests"
        || test_module.content.is_none()
        || test_module.attrs.len() != 1
        || !cfg_test_attribute(&test_module.attrs[0])
    {
        return None;
    }
    file.items.pop();
    Some(file)
}

fn impls_for_type<'a>(file: &'a File, name: &str) -> Vec<&'a ItemImpl> {
    file.items
        .iter()
        .filter_map(|item| match item {
            Item::Impl(implementation) if type_ends_with(&implementation.self_ty, name) => {
                Some(implementation)
            }
            _ => None,
        })
        .collect()
}

fn unique_impl<'a>(
    implementations: &[&'a ItemImpl],
    trait_name: Option<&str>,
    unsafe_impl: bool,
) -> Option<&'a ItemImpl> {
    let mut matches = implementations.iter().copied().filter(|implementation| {
        let actual_trait = implementation
            .trait_
            .as_ref()
            .map(|(_, path, _)| path_text(path));
        actual_trait.as_deref() == trait_name
            && implementation.unsafety.is_some() == unsafe_impl
            && implementation.defaultness.is_none()
            && implementation.generics.params.is_empty()
            && implementation.generics.where_clause.is_none()
    });
    let implementation = matches.next()?;
    matches.next().is_none().then_some(implementation)
}

fn impl_has_exact_surface(
    implementation: &ItemImpl,
    expected_functions: &[&str],
    expected_types: &[&str],
) -> bool {
    let mut functions = Vec::new();
    let mut types = Vec::new();
    for item in &implementation.items {
        match item {
            ImplItem::Fn(function) => functions.push(function.sig.ident.to_string()),
            ImplItem::Type(item_type) => types.push(item_type.ident.to_string()),
            _ => return false,
        }
    }
    functions.sort();
    types.sort();
    functions == expected_functions && types == expected_types
}

fn unique_impl_function<'a>(implementation: &'a ItemImpl, name: &str) -> Option<&'a ImplItemFn> {
    let mut functions = implementation.items.iter().filter_map(|item| match item {
        ImplItem::Fn(function) if function.sig.ident == name => Some(function),
        _ => None,
    });
    let function = functions.next()?;
    functions.next().is_none().then_some(function)
}

fn unique_free_function<'a>(file: &'a File, name: &str) -> Option<&'a ItemFn> {
    let mut functions = file.items.iter().filter_map(|item| match item {
        Item::Fn(function) if function.sig.ident == name => Some(function),
        _ => None,
    });
    let function = functions.next()?;
    functions.next().is_none().then_some(function)
}

fn attributes_fingerprint(attributes: &[Attribute]) -> [u8; 32] {
    let mut stream = TokenStream::new();
    for attribute in attributes {
        attribute.to_tokens(&mut stream);
    }
    token_fingerprint(&stream)
}

fn reviewed_lib_node_fingerprints(file: &File) -> Option<[[u8; 32]; 23]> {
    let zero_impls = impls_for_type(file, ZERO_TYPE);
    let zero_inherent = unique_impl(&zero_impls, None, false)?;
    let zero_backend = unique_impl(&zero_impls, Some(BACKEND_TRAIT), true)?;
    let configured_impls = impls_for_type(file, CONFIGURED_TYPE_NAME);
    let configured_debug = unique_impl(&configured_impls, Some("fmt::Debug"), false)?;
    let configured_inherent = unique_impl(&configured_impls, None, false)?;
    let configured_backend = unique_impl(&configured_impls, Some(BACKEND_TRAIT), true)?;
    let zero_error_impls = impls_for_type(file, "M1AllKernelsProtectedVerifierErrorV1");
    let production_error_impls =
        impls_for_type(file, "M1AllKernelsProductionProtectedVerifierErrorV1");

    let mut fingerprints = vec![
        attributes_fingerprint(&file.attrs),
        token_fingerprint(unique_impl_function(zero_inherent, "new")?),
        token_fingerprint(unique_impl_function(
            zero_inherent,
            "reject_missing_protected_receipt",
        )?),
        token_fingerprint(unique_impl_function(
            zero_backend,
            "verify_protected_roster",
        )?),
        token_fingerprint(configured_debug),
        token_fingerprint(unique_impl_function(configured_inherent, "new")?),
        token_fingerprint(unique_impl_function(
            configured_inherent,
            "grants_authority",
        )?),
        token_fingerprint(unique_impl_function(
            configured_backend,
            "verify_protected_roster",
        )?),
        token_fingerprint(unique_impl(&zero_error_impls, Some("fmt::Display"), false)?),
        token_fingerprint(unique_impl(
            &production_error_impls,
            Some("fmt::Display"),
            false,
        )?),
    ];
    for name in REVIEWED_FREE_FUNCTIONS {
        fingerprints.push(token_fingerprint(unique_free_function(file, name)?));
    }
    fingerprints.try_into().ok()
}

fn allowed_attribute(attribute: &Attribute) -> bool {
    [
        "allow",
        "cfg",
        "deny",
        "derive",
        "doc",
        "must_use",
        "non_exhaustive",
    ]
    .iter()
    .any(|allowed| attribute.path().is_ident(allowed))
}

fn use_tree_has_alias_or_glob(tree: &UseTree) -> bool {
    match tree {
        UseTree::Rename(_) | UseTree::Glob(_) => true,
        UseTree::Group(group) => group.items.iter().any(use_tree_has_alias_or_glob),
        UseTree::Path(path) => use_tree_has_alias_or_glob(&path.tree),
        UseTree::Name(_) => false,
    }
}

fn use_tree_binds_forbidden_name(tree: &UseTree) -> bool {
    match tree {
        UseTree::Name(name) => name.ident == "core" || name.ident == "write",
        UseTree::Rename(_) | UseTree::Glob(_) => true,
        UseTree::Group(group) => group.items.iter().any(use_tree_binds_forbidden_name),
        UseTree::Path(path) => use_tree_binds_forbidden_name(&path.tree),
    }
}

fn reviewed_module(module: &ItemMod) -> bool {
    let public = matches!(module.vis, Visibility::Public(_));
    module.content.is_none()
        && match module.ident.to_string().as_str() {
            "protected_receipt" | "protected_verifier_client" | "protected_verifier_service" => {
                public
                    && !module.attrs.is_empty()
                    && module
                        .attrs
                        .iter()
                        .all(|attribute| attribute.path().is_ident("doc"))
            }
            "protected_verifier_test_support" => {
                !public && module.attrs.len() == 1 && cfg_test_attribute(&module.attrs[0])
            }
            _ => false,
        }
}

fn configured_struct_attributes_are_exact(item: &syn::ItemStruct) -> bool {
    match item.ident.to_string().as_str() {
        "M1AllKernelsPendingDescriptorProjectionV1"
        | "M1AllKernelsPendingDescriptorBindingProjectionV1"
        | "M1AllKernelsPendingPhysicalKernelProjectionV1"
        | "M1AllKernelsPendingEntryProjectionV1"
        | "M1AllKernelsPendingRequestProjectionV1" => {
            item.attrs.len() == 1
                && compact_tokens(&item.attrs[0]) == "#[allow(dead_code)]"
                && matches!(item.vis, Visibility::Inherited)
        }
        "M1AllKernelsLocallyRevalidatedOwnersV1" => {
            item.attrs.is_empty() && matches!(item.vis, Visibility::Inherited)
        }
        ZERO_TYPE => {
            matches!(item.vis, Visibility::Public(_))
                && item
                    .attrs
                    .iter()
                    .filter(|attr| attr.path().is_ident("derive"))
                    .count()
                    == 1
                && item.attrs.iter().all(|attr| {
                    attr.path().is_ident("doc")
                        || compact_tokens(attr) == "#[derive(Clone,Copy,Debug,Default)]"
                })
        }
        CONFIGURED_TYPE_NAME => {
            matches!(item.vis, Visibility::Public(_))
                && !item.attrs.is_empty()
                && item
                    .attrs
                    .iter()
                    .all(|attribute| attribute.path().is_ident("doc"))
                && configured_fields_are_exact(&item.fields)
        }
        _ => false,
    }
}

fn configured_fields_are_exact(fields: &Fields) -> bool {
    let Fields::Named(fields) = fields else {
        return false;
    };
    let expected = [
        ("client", "Option<M1AllKernelsProtectedVerifierClientV2>"),
        (
            "begin_challenge",
            "Option<M1AllKernelsProtectedVerifierBeginChallengeV2>",
        ),
        ("trust_policy", "M1AllKernelsProtectedVerifierTrustPolicyV1"),
        (
            "current_auditor",
            "InheritedWorkerV3CompilerCurrentRecordAuditorV1",
        ),
    ];
    fields.named.len() == expected.len()
        && fields
            .named
            .iter()
            .zip(expected)
            .all(|(field, (name, ty))| {
                field.ident.as_ref().is_some_and(|ident| ident == name)
                    && matches!(field.vis, Visibility::Inherited)
                    && field.attrs.is_empty()
                    && compact_tokens(&field.ty) == ty
            })
}

fn reviewed_enum(item: &syn::ItemEnum) -> bool {
    let derive = match item.ident.to_string().as_str() {
        "M1AllKernelsProtectedVerifierErrorV1" => "#[derive(Clone,Copy,Debug,Eq,PartialEq)]",
        "M1AllKernelsProductionProtectedVerifierErrorV1" => "#[derive(Debug)]",
        _ => return false,
    };
    matches!(item.vis, Visibility::Public(_))
        && item
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("derive"))
            .count()
            == 1
        && item
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("non_exhaustive"))
            .count()
            == 1
        && item.attrs.iter().all(|attr| {
            attr.path().is_ident("doc")
                || attr.path().is_ident("non_exhaustive")
                || compact_tokens(attr) == derive
        })
}

fn reviewed_free_function(function: &ItemFn) -> bool {
    let name = function.sig.ident.to_string();
    REVIEWED_FREE_FUNCTIONS.contains(&name.as_str())
        && matches!(function.vis, Visibility::Inherited)
        && function.sig.unsafety.is_none()
        && function.sig.abi.is_none()
        && function.sig.asyncness.is_none()
        && function.sig.variadic.is_none()
        && function.sig.constness.is_some() == (name == "missing_protected_verification_receipt_v1")
        && if name == "validate_local_request_associations_v1" {
            function.attrs.len() == 1
                && compact_tokens(&function.attrs[0]) == "#[allow(clippy::too_many_lines)]"
        } else {
            function.attrs.is_empty()
        }
}

fn reviewed_const(item: &syn::ItemConst) -> bool {
    match item.ident.to_string().as_str() {
        "M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1" => {
            matches!(item.vis, Visibility::Public(_))
                && !item.attrs.is_empty()
                && item.attrs.iter().all(|attr| attr.path().is_ident("doc"))
        }
        "M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1" | "_" => {
            matches!(item.vis, Visibility::Inherited) && item.attrs.is_empty()
        }
        _ => false,
    }
}

fn impl_identity(implementation: &ItemImpl) -> String {
    let trait_name = implementation
        .trait_
        .as_ref()
        .map_or_else(|| "inherent".to_owned(), |(_, path, _)| path_text(path));
    format!(
        "{}:{}:{trait_name}",
        implementation.unsafety.is_some(),
        compact_tokens(&implementation.self_ty)
    )
}

fn exact_impl_roster(implementations: &mut [String]) -> bool {
    let mut expected = [
        "false:M1AllKernelsPendingRequestProjectionV1:inherent",
        "false:M1AllKernelsProductionProtectedVerifierErrorV1:Error",
        "false:M1AllKernelsProductionProtectedVerifierErrorV1:fmt::Display",
        "false:M1AllKernelsProductionProtectedVerifierV1:fmt::Debug",
        "false:M1AllKernelsProductionProtectedVerifierV1:inherent",
        "false:M1AllKernelsProtectedVerifierErrorV1:Error",
        "false:M1AllKernelsProtectedVerifierErrorV1:fmt::Display",
        "false:M1AllKernelsProtectedVerifierV1:inherent",
        "true:M1AllKernelsProductionProtectedVerifierV1:WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1>",
        "true:M1AllKernelsProtectedVerifierV1:WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1>",
    ];
    implementations.sort();
    expected.sort_unstable();
    implementations == expected
}

fn reviewed_use(item_use: &ItemUse) -> bool {
    let expected = [
        "usecrate::protected_receipt::{M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,M1AllKernelsProtectedReceiptCompilerClaimsV1,M1AllKernelsProtectedReceiptEntryV1,M1AllKernelsProtectedReceiptErrorV1,M1AllKernelsProtectedReceiptRequestClaimsV1,M1AllKernelsProtectedReceiptSourcePinV1,M1AllKernelsProtectedVerifierTrustPolicyV1,};",
        "usecrate::protected_verifier_client::{M1AllKernelsProtectedVerifierBeginChallengeV2,M1AllKernelsProtectedVerifierClientErrorV2,M1AllKernelsProtectedVerifierClientV2,};",
        "usecrate::protected_verifier_service::{M1AllKernelsProtectedVerifierServiceEntryV1,M1AllKernelsProtectedVerifierServiceProtocolErrorV1,M1AllKernelsProtectedVerifierServiceRequestV1,};",
        "usefe2o3_host::{BlockSizeV1,CompilerGeneratedKernelExpectationRosterEntryV1,CompilerGeneratedKernelExpectationRosterV1,InheritedWorkerV3CompilerCurrentRecordAuditorV1,WorkerV3CompilerCurrentRecordAuditErrorV1,WorkerV3CompilerCurrentRecordAuditV1,WorkerV3CompilerExecutionEvidenceErrorV1,WorkerV3CompilerExecutionVerificationV1,WorkerV3ProtectedRosterEntryEvidenceV1,WorkerV3ProtectedRosterVerificationEvidenceV1,WorkerV3ProtectedRosterVerifierBackendV1,WorkerV3RosterVerificationRequestV1,};",
        "usefe2o3_hsaco_finalize::{FinalizedDescriptorInspection,RevalidatedProtectedWorkerV3FinalizerDerivationV1,};",
        "usefe2o3_verifier::{ValidatedCompilerMultiRootProofInputsV1,ValidatedCompilerMultiRootTargetLineageV1,};",
        "usefe2o3_worker_v3_verification_protocol::{WorkerV3VerificationEntryCoordinateV1,WorkerV3VerificationFdPayloadDescriptorV1,WorkerV3VerificationMeasurementIdentityV1,WorkerV3VerificationPolicyIdentityV1,WorkerV3VerificationProtocolErrorV1,WorkerV3VerificationRequestV1,WorkerV3VerificationRosterIdentityV1,};",
        "useferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;",
        "useferric_qwen3_all_kernels_worker_v3_source_pin_v1::{M1AggregateSourcePinErrorV1,project_m1_aggregate_module_handoff_v1,};",
        "userustix::fs::{MemfdFlags,Mode,SealFlags};",
        "usesha2::{Digest,Sha256};",
        "usestd::error::Error;",
        "usestd::fmt;",
        "usestd::fs::File;",
        "usestd::io::{self,Write};",
        "usestd::os::fd::OwnedFd;",
    ];
    item_use.attrs.is_empty()
        && matches!(item_use.vis, Visibility::Inherited)
        && expected.contains(&compact_tokens(item_use).as_str())
}

fn top_level_surface_is_exact(file: &File) -> bool {
    if file.attrs.len() < 3
        || !file.attrs[..file.attrs.len() - 2]
            .iter()
            .all(|attribute| attribute.path().is_ident("doc"))
        || compact_tokens(&file.attrs[file.attrs.len() - 2]) != "#![deny(missing_docs)]"
        || compact_tokens(&file.attrs[file.attrs.len() - 1]) != "#![deny(unsafe_op_in_unsafe_fn)]"
    {
        return false;
    }
    let mut implementations = Vec::new();
    let mut counts = [0_usize; 7];
    for item in &file.items {
        let index = match item {
            Item::Const(item_const) if reviewed_const(item_const) => 0,
            Item::Enum(item_enum) if reviewed_enum(item_enum) => 1,
            Item::Fn(function) if reviewed_free_function(function) => 2,
            Item::Impl(implementation) if implementation.attrs.is_empty() => {
                implementations.push(impl_identity(implementation));
                3
            }
            Item::Mod(module) if reviewed_module(module) => 4,
            Item::Struct(item_struct) if configured_struct_attributes_are_exact(item_struct) => 5,
            Item::Use(item_use) if reviewed_use(item_use) => 6,
            _ => return false,
        };
        counts[index] += 1;
    }
    counts == [3, 2, 13, 10, 4, 8, 16] && exact_impl_roster(&mut implementations)
}

fn expression_path(expression: &Expr) -> Option<&Path> {
    match expression {
        Expr::Path(path) if path.qself.is_none() => Some(&path.path),
        _ => None,
    }
}

fn call_path_is(call: &ExprCall, expected: &str) -> bool {
    expression_path(&call.func).is_some_and(|path| path_is(path, expected))
}

fn expression_is_bare_path(expression: &Expr, expected: &str) -> bool {
    expression_path(expression).is_some_and(|path| {
        path.leading_colon.is_none() && path.segments.len() == 1 && path.is_ident(expected)
    })
}

fn configured_tail_is_exact(function: &ImplItemFn) -> bool {
    let Some(syn::Stmt::Expr(Expr::Call(ok_call), None)) = function.block.stmts.last() else {
        return false;
    };
    if !call_path_is(ok_call, "Ok") || ok_call.args.len() != 1 {
        return false;
    }
    let Some(Expr::Unsafe(unsafe_expression)) = ok_call.args.first() else {
        return false;
    };
    let [syn::Stmt::Expr(Expr::Call(evidence_call), None)] =
        unsafe_expression.block.stmts.as_slice()
    else {
        return false;
    };
    call_path_is(evidence_call, EVIDENCE_CONSTRUCTOR)
        && evidence_call.args.len() == 7
        && evidence_call
            .args
            .iter()
            .zip([
                "finalizer",
                "compiler_execution",
                "proof_inputs",
                "target_lineage",
                "verifier_measurement",
                "verification_transcript",
                "entries",
            ])
            .all(|(argument, expected)| expression_is_bare_path(argument, expected))
}

fn helper_index(path: &Path) -> Option<usize> {
    let terminal = path.segments.last()?.ident.to_string();
    REVIEWED_FREE_FUNCTIONS
        .iter()
        .position(|expected| terminal == *expected)
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
struct ProductionSurface {
    invalid_attribute: bool,
    invalid_macro: bool,
    invalid_use: bool,
    nested_or_extra_function: bool,
    type_alias: bool,
    evidence_constructor_calls: usize,
    evidence_constructor_references: usize,
    configured_constructor_references: usize,
    verify_references: usize,
    explicit_configured_constructions: usize,
    unsafe_blocks: usize,
    return_expressions: usize,
    reviewed_macros: usize,
    helper_calls: [usize; REVIEWED_FREE_FUNCTIONS.len()],
    helper_references: [usize; REVIEWED_FREE_FUNCTIONS.len()],
    free_functions: BTreeSet<String>,
    item_depth: usize,
}

impl<'ast> Visit<'ast> for ProductionSurface {
    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        self.invalid_attribute |= !allowed_attribute(attribute);
        visit::visit_attribute(self, attribute);
    }

    fn visit_item_fn(&mut self, function: &'ast ItemFn) {
        if self.item_depth != 0
            || !REVIEWED_FREE_FUNCTIONS
                .iter()
                .any(|allowed| function.sig.ident == allowed)
            || !self.free_functions.insert(function.sig.ident.to_string())
        {
            self.nested_or_extra_function = true;
        }
        self.item_depth += 1;
        visit::visit_item_fn(self, function);
        self.item_depth -= 1;
    }

    fn visit_item_mod(&mut self, module: &'ast ItemMod) {
        if !reviewed_module(module) {
            self.nested_or_extra_function = true;
        }
        visit::visit_item_mod(self, module);
    }

    fn visit_item_type(&mut self, item_type: &'ast syn::ItemType) {
        self.type_alias = true;
        visit::visit_item_type(self, item_type);
    }

    fn visit_item_use(&mut self, item_use: &'ast ItemUse) {
        self.invalid_use |= use_tree_has_alias_or_glob(&item_use.tree)
            || use_tree_binds_forbidden_name(&item_use.tree);
        visit::visit_item_use(self, item_use);
    }

    fn visit_macro(&mut self, item_macro: &'ast syn::Macro) {
        self.invalid_macro |= !path_is(&item_macro.path, "::core::write");
        self.reviewed_macros += 1;
        visit::visit_macro(self, item_macro);
    }

    fn visit_expr_return(&mut self, expression: &'ast syn::ExprReturn) {
        self.return_expressions += 1;
        visit::visit_expr_return(self, expression);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(expression) = &*call.func {
            let path = &expression.path;
            if path_ends_with(path, &[EVIDENCE_TYPE, "new"])
                || expression_associated_path_is(expression, EVIDENCE_TYPE, "new")
            {
                self.evidence_constructor_calls += 1;
            }
            if let Some(index) = helper_index(path) {
                self.helper_calls[index] += 1;
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if call.method == "verify_protected_roster" {
            self.verify_references += 1;
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
        let path = &expression.path;
        if path_ends_with(path, &[EVIDENCE_TYPE, "new"])
            || expression_associated_path_is(expression, EVIDENCE_TYPE, "new")
        {
            self.evidence_constructor_references += 1;
        }
        if path_ends_with(path, &[CONFIGURED_TYPE_NAME, "new"])
            || expression_associated_path_is(expression, CONFIGURED_TYPE_NAME, "new")
        {
            self.configured_constructor_references += 1;
        }
        if path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "verify_protected_roster")
        {
            self.verify_references += 1;
        }
        if let Some(index) = helper_index(path) {
            self.helper_references[index] += 1;
        }
        visit::visit_expr_path(self, expression);
    }

    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        if path_ends_with(&expression.path, &[CONFIGURED_TYPE_NAME]) {
            self.explicit_configured_constructions += 1;
        }
        visit::visit_expr_struct(self, expression);
    }

    fn visit_expr_unsafe(&mut self, expression: &'ast syn::ExprUnsafe) {
        self.unsafe_blocks += 1;
        visit::visit_expr_unsafe(self, expression);
    }
}

fn expression_associated_path_is(expression: &syn::ExprPath, ty: &str, member: &str) -> bool {
    expression
        .qself
        .as_ref()
        .is_some_and(|qself| type_ends_with(&qself.ty, ty))
        && expression
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == member)
}

#[derive(Default)]
struct TypeNameVisitor {
    configured: bool,
}

impl<'ast> Visit<'ast> for TypeNameVisitor {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        self.configured |= path
            .path
            .segments
            .iter()
            .any(|segment| segment.ident == CONFIGURED_TYPE_NAME);
        visit::visit_type_path(self, path);
    }
}

fn type_mentions_configured(ty: &Type) -> bool {
    let mut visitor = TypeNameVisitor::default();
    visitor.visit_type(ty);
    visitor.configured
}

#[derive(Default)]
struct SiblingEscapeSurface {
    configured_impls: usize,
    configured_constructions: usize,
    configured_constructor_references: usize,
    configured_returns: usize,
    configured_type_references: usize,
    evidence_type_references: usize,
    evidence_promotions: usize,
    protected_verify_references: usize,
    nested_modules: usize,
    item_macros: usize,
    type_aliases: usize,
    use_aliases_or_globs: usize,
}

impl SiblingEscapeSurface {
    fn is_clear(&self) -> bool {
        self.configured_impls == 0
            && self.configured_constructions == 0
            && self.configured_constructor_references == 0
            && self.configured_returns == 0
            && self.configured_type_references == 0
            && self.evidence_type_references == 0
            && self.evidence_promotions == 0
            && self.protected_verify_references == 0
            && self.nested_modules == 0
            && self.item_macros == 0
            && self.type_aliases == 0
            && self.use_aliases_or_globs == 0
    }
}

impl<'ast> Visit<'ast> for SiblingEscapeSurface {
    fn visit_item_impl(&mut self, implementation: &'ast ItemImpl) {
        self.configured_impls += usize::from(type_ends_with(
            &implementation.self_ty,
            CONFIGURED_TYPE_NAME,
        ));
        visit::visit_item_impl(self, implementation);
    }

    fn visit_signature(&mut self, signature: &'ast syn::Signature) {
        if let ReturnType::Type(_, ty) = &signature.output {
            self.configured_returns += usize::from(type_mentions_configured(ty));
        }
        visit::visit_signature(self, signature);
    }

    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        self.configured_constructions +=
            usize::from(path_ends_with(&expression.path, &[CONFIGURED_TYPE_NAME]));
        self.evidence_promotions += usize::from(path_ends_with(&expression.path, &[EVIDENCE_TYPE]));
        visit::visit_expr_struct(self, expression);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = &*call.func {
            self.evidence_promotions += usize::from(
                path_ends_with(&path.path, &[EVIDENCE_TYPE, "new"])
                    || expression_associated_path_is(path, EVIDENCE_TYPE, "new"),
            );
            self.configured_constructor_references += usize::from(
                path_ends_with(&path.path, &[CONFIGURED_TYPE_NAME, "new"])
                    || expression_associated_path_is(path, CONFIGURED_TYPE_NAME, "new"),
            );
            self.protected_verify_references += usize::from(
                path.path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "verify_protected_roster"),
            );
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        self.evidence_promotions += usize::from(
            path_ends_with(&path.path, &[EVIDENCE_TYPE, "new"])
                || expression_associated_path_is(path, EVIDENCE_TYPE, "new"),
        );
        self.configured_constructor_references += usize::from(
            path_ends_with(&path.path, &[CONFIGURED_TYPE_NAME, "new"])
                || expression_associated_path_is(path, CONFIGURED_TYPE_NAME, "new"),
        );
        self.protected_verify_references += usize::from(
            path.path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "verify_protected_roster"),
        );
        visit::visit_expr_path(self, path);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        self.protected_verify_references += usize::from(call.method == "verify_protected_roster");
        visit::visit_expr_method_call(self, call);
    }

    fn visit_item_mod(&mut self, module: &'ast ItemMod) {
        self.nested_modules += 1;
        visit::visit_item_mod(self, module);
    }

    fn visit_item_macro(&mut self, item_macro: &'ast syn::ItemMacro) {
        self.item_macros += 1;
        visit::visit_item_macro(self, item_macro);
    }

    fn visit_item_type(&mut self, item_type: &'ast syn::ItemType) {
        self.type_aliases += 1;
        visit::visit_item_type(self, item_type);
    }

    fn visit_item_use(&mut self, item_use: &'ast ItemUse) {
        self.use_aliases_or_globs += usize::from(use_tree_has_alias_or_glob(&item_use.tree));
        visit::visit_item_use(self, item_use);
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        self.configured_type_references += usize::from(
            path.path
                .segments
                .iter()
                .any(|segment| segment.ident == CONFIGURED_TYPE_NAME),
        );
        self.evidence_type_references += usize::from(
            path.path
                .segments
                .iter()
                .any(|segment| segment.ident == EVIDENCE_TYPE),
        );
        visit::visit_type_path(self, path);
    }
}

fn sibling_production_policy(source: &str, expected_fingerprint: [u8; 32]) -> bool {
    let Some(production) = production_file(source) else {
        return false;
    };
    sibling_file_has_no_escape(&production)
        && token_fingerprint(&production) == expected_fingerprint
}

fn sibling_file_has_no_escape(file: &File) -> bool {
    let mut escape = SiblingEscapeSurface::default();
    escape.visit_file(file);
    escape.is_clear()
}

fn sibling_structural_policy(source: &str) -> bool {
    production_file(source).is_some_and(|production| sibling_file_has_no_escape(&production))
}

fn test_support_policy(source: &str, expected_fingerprint: [u8; 32]) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    if file.attrs.len() < 2
        || !file.attrs[..file.attrs.len() - 1]
            .iter()
            .all(|attribute| attribute.path().is_ident("doc"))
        || !cfg_test_attribute(&file.attrs[file.attrs.len() - 1])
    {
        return false;
    }
    sibling_file_has_no_escape(&file) && token_fingerprint(&file) == expected_fingerprint
}

#[derive(Default)]
struct MethodAudit {
    critical_calls: Vec<&'static str>,
    invalid_control: bool,
    invalid_macro: bool,
    evidence_constructor_calls: usize,
    evidence_constructor_references: usize,
    configured_constructor_references: usize,
    verify_references: usize,
    self_constructions: usize,
    unsafe_blocks: usize,
}

impl MethodAudit {
    fn from_impl(function: &ImplItemFn) -> Self {
        let mut audit = Self::default();
        audit.visit_block(&function.block);
        audit
    }

    fn from_item(function: &ItemFn) -> Self {
        let mut audit = Self::default();
        audit.visit_block(&function.block);
        audit
    }
}

impl<'ast> Visit<'ast> for MethodAudit {
    fn visit_expr(&mut self, expression: &'ast Expr) {
        self.invalid_control |= matches!(
            expression,
            Expr::Async(_)
                | Expr::Block(_)
                | Expr::Break(_)
                | Expr::Closure(_)
                | Expr::Const(_)
                | Expr::Continue(_)
                | Expr::ForLoop(_)
                | Expr::If(_)
                | Expr::Loop(_)
                | Expr::Match(_)
                | Expr::Return(_)
                | Expr::TryBlock(_)
                | Expr::While(_)
                | Expr::Yield(_)
        );
        visit::visit_expr(self, expression);
    }

    fn visit_macro(&mut self, item_macro: &'ast syn::Macro) {
        self.invalid_macro = true;
        visit::visit_macro(self, item_macro);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        let labels = [
            (
                "M1AllKernelsPendingRequestProjectionV1::from_request",
                "projection",
            ),
            ("locally_revalidate_request_v1", "local_revalidation"),
            (
                "Self::reject_missing_protected_receipt",
                "terminal_rejection",
            ),
            (
                "missing_protected_verification_receipt_v1",
                "missing_receipt",
            ),
            ("protected_service_request_v1", "service_request"),
            (
                "protected_service_request_from_current_audit_v1",
                "prebind_service_request",
            ),
            ("generic_verification_request_v1", "generic_request"),
            ("protected_payload_snapshots_v2", "payload_snapshots"),
            ("authenticated_receipt_associates_v1", "receipt_association"),
            ("authenticated_entry_evidence_v1", "entry_evidence"),
        ];
        for (path, label) in labels {
            if call_path_is(call, path) {
                self.critical_calls.push(label);
            }
        }
        if let Some(path) = expression_path(&call.func)
            && path_is(path, EVIDENCE_CONSTRUCTOR)
        {
            self.critical_calls.push("evidence_promotion");
        }
        if matches!(&*call.func, Expr::Path(path) if path_ends_with(&path.path, &[EVIDENCE_TYPE, "new"]) || expression_associated_path_is(path, EVIDENCE_TYPE, "new"))
        {
            self.evidence_constructor_calls += 1;
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let label = match call.method.to_string().as_str() {
            "audit_roster_with_challenge" => Some("current_audit"),
            "bind_exact_compiler_execution_v1" => Some("compiler_binding"),
            "take" => Some("owner_take"),
            "begin" => Some("begin"),
            "into_parts" => Some("service_challenge"),
            "into_compiler_execution_challenge" => Some("compiler_challenge"),
            "canonical_evidence_view" => Some("current_record_view"),
            "submit_current_record" => Some("submit_current_record"),
            "verify_protected_roster" => {
                self.verify_references += 1;
                None
            }
            _ => None,
        };
        if let Some(label) = label {
            self.critical_calls.push(label);
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
        let path = &expression.path;
        if path_ends_with(path, &[EVIDENCE_TYPE, "new"])
            || expression_associated_path_is(expression, EVIDENCE_TYPE, "new")
        {
            self.evidence_constructor_references += 1;
        }
        if path_ends_with(path, &[CONFIGURED_TYPE_NAME, "new"])
            || expression_associated_path_is(expression, CONFIGURED_TYPE_NAME, "new")
            || path_is(path, "Self::new")
        {
            self.configured_constructor_references += 1;
        }
        if path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "verify_protected_roster")
        {
            self.verify_references += 1;
        }
        visit::visit_expr_path(self, expression);
    }

    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        if expression.path.is_ident("Self") {
            self.self_constructions += 1;
        }
        visit::visit_expr_struct(self, expression);
    }

    fn visit_expr_unsafe(&mut self, expression: &'ast syn::ExprUnsafe) {
        self.unsafe_blocks += 1;
        visit::visit_expr_unsafe(self, expression);
    }
}

fn zero_state_policy(source: &str) -> bool {
    let Some(policy) = AstVerifierPolicySource::parse(source) else {
        return false;
    };
    let verify = item_tokens(&policy.zero_verify);
    let rejection = item_tokens(&policy.zero_rejection);
    let verify_audit = MethodAudit::from_impl(&policy.zero_verify);
    let rejection_audit = MethodAudit::from_impl(&policy.zero_rejection);
    compact_tokens(&policy.zero_verify.sig)
        == "unsafefnverify_protected_roster(&mutself,request:&WorkerV3RosterVerificationRequestV1<'_,M1AllKernelsWorkerV3RosterV1>,)->Result<WorkerV3ProtectedRosterVerificationEvidenceV1,Self::Error>"
        && matches!(policy.zero_verify.vis, Visibility::Inherited)
        && policy.zero_verify.block.stmts.len() == 3
        && verify_audit.critical_calls
            == ["projection", "local_revalidation", "terminal_rejection"]
        && !verify_audit.invalid_control
        && !verify_audit.invalid_macro
        && verify_audit.evidence_constructor_calls == 0
        && verify_audit.evidence_constructor_references == 0
        && verify_audit.configured_constructor_references == 0
        && verify_audit.verify_references == 0
        && verify_audit.unsafe_blocks == 0
        && verify.ends_with(
        "Self::reject_missing_protected_receipt(&pending_request,owners.finalizer,owners.proof_inputs,owners.target_lineage,owners.hsaco_reinspection,)}",
    )
        && compact_tokens(&policy.zero_rejection.sig).starts_with(
            "fnreject_missing_protected_receipt<FinalizerOwner,ProofInputsOwner,TargetLineageOwner,HsacoReinspectionOwner,>",
        )
        && matches!(policy.zero_rejection.vis, Visibility::Inherited)
        && policy.zero_rejection.block.stmts.len() == 1
        && rejection_audit.critical_calls == ["missing_receipt"]
        && !rejection_audit.invalid_control
        && !rejection_audit.invalid_macro
        && rejection_audit.unsafe_blocks == 0
        && rejection.ends_with("Err(missing_protected_verification_receipt_v1())}")
}

fn configured_constructor_policy(source: &str) -> bool {
    let Some(policy) = AstVerifierPolicySource::parse(source) else {
        return false;
    };
    let constructor = item_tokens(&policy.configured_constructor);
    let constructor_audit = MethodAudit::from_impl(&policy.configured_constructor);
    let authority = item_tokens(&policy.configured_authority_query);
    compact_tokens(&policy.configured_constructor.sig)
        == "unsafefnnew(client:M1AllKernelsProtectedVerifierClientV2,begin_challenge:M1AllKernelsProtectedVerifierBeginChallengeV2,trust_policy:M1AllKernelsProtectedVerifierTrustPolicyV1,current_auditor:InheritedWorkerV3CompilerCurrentRecordAuditorV1,)->Self"
        && matches!(policy.configured_constructor.vis, Visibility::Public(_))
        && policy.configured_constructor.block.stmts.len() == 1
        && constructor_audit.self_constructions == 1
        && constructor_audit.configured_constructor_references == 0
        && constructor_audit.evidence_constructor_references == 0
        && constructor_audit.verify_references == 0
        && constructor_audit.unsafe_blocks == 0
        && !constructor_audit.invalid_control
        && !constructor_audit.invalid_macro
        && constructor.ends_with("{Self{client:Some(client),begin_challenge:Some(begin_challenge),trust_policy,current_auditor,}}")
        && compact_tokens(&policy.configured_authority_query.sig)
            == "constfngrants_authority(&self)->bool"
        && matches!(policy.configured_authority_query.vis, Visibility::Public(_))
        && policy.configured_authority_query.attrs.len() == 2
        && policy
            .configured_authority_query
            .attrs
            .iter()
            .all(|attribute| {
                attribute.path().is_ident("doc") || attribute.path().is_ident("must_use")
            })
        && policy.configured_authority_query.block.stmts.len() == 1
        && authority.ends_with("{false}")
}

fn configured_binder_policy(source: &str) -> bool {
    let Some(policy) = AstVerifierPolicySource::parse(source) else {
        return false;
    };
    let audit = MethodAudit::from_impl(&policy.configured_verify);
    compact_tokens(&policy.configured_verify.sig)
        == "unsafefnverify_protected_roster(&mutself,request:&WorkerV3RosterVerificationRequestV1<'_,M1AllKernelsWorkerV3RosterV1>,)->Result<WorkerV3ProtectedRosterVerificationEvidenceV1,Self::Error>"
        && matches!(policy.configured_verify.vis, Visibility::Inherited)
        && policy.configured_verify.block.stmts.len() == 25
        && audit.critical_calls
            == [
                "projection",
                "local_revalidation",
                "owner_take",
                "owner_take",
                "generic_request",
                "payload_snapshots",
                "begin",
                "service_challenge",
                "compiler_challenge",
                "current_audit",
                "prebind_service_request",
                "current_record_view",
                "submit_current_record",
                "receipt_association",
                "compiler_binding",
                "service_request",
                "entry_evidence",
                "evidence_promotion",
            ]
        && !audit.invalid_control
        && !audit.invalid_macro
        && audit.evidence_constructor_calls == 1
        && audit.evidence_constructor_references == 1
        && audit.configured_constructor_references == 0
        && audit.verify_references == 0
        && audit.self_constructions == 0
        && audit.unsafe_blocks == 1
        && configured_tail_is_exact(&policy.configured_verify)
}

#[test]
fn roster_is_the_exact_twelve_entry_aggregate() {
    assert_eq!(M1AllKernelsWorkerV3RosterV1::ENTRIES.len(), 12);
    assert_eq!(
        M1AllKernelsWorkerV3RosterV1::ENTRIES
            .iter()
            .map(fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1::export_name)
            .collect::<Vec<_>>(),
        [
            "qwen3_swiglu_bf16_f32_v1",
            "qwen3_gqa_prefill_causal_bf16_f32_v1",
            "ferric_qwen3_lowest_id_argmax_bf16_v1",
            "qwen3_paged_kv_write_v1",
            "qwen3_paged_gqa_decode_bf16_f32_v1",
            "ferric_qwen3_speculative_token_assembly_v1",
            "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1",
            "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1",
            "ferric_qwen3_token_embedding_bf16_copy_v1",
            "ferric_qwen3_compact_completion_v1",
            "qwen3_rope_v1",
            "qwen3_rmsnorm_v1",
        ]
    );
}

#[test]
fn zero_state_backend_remains_unconditionally_fail_closed() {
    assert!(zero_state_policy(SOURCE));
    assert!(!zero_state_policy(&SOURCE.replacen(
        "Self::reject_missing_protected_receipt(",
        "unreachable_success(",
        1,
    )));
    assert!(!zero_state_policy(&SOURCE.replacen(
        "locally_revalidate_request_v1(request, &pending_request)?",
        "Default::default()",
        1,
    )));
}

#[test]
fn both_backends_share_one_local_owner_revalidation_and_association_path() {
    let policy =
        AstVerifierPolicySource::parse(SOURCE).expect("canonical source policy must parse");
    let local_revalidation = item_tokens(&policy.local_revalidation);
    let local_associations = item_tokens(&policy.local_associations);
    assert_eq!(
        policy
            .compact
            .matches("fnlocally_revalidate_request_v1(")
            .count(),
        1
    );
    assert_eq!(
        policy
            .compact
            .matches("locally_revalidate_request_v1(")
            .count(),
        3
    );
    assert_eq!(
        policy
            .compact
            .matches("fnvalidate_local_request_associations_v1(")
            .count(),
        1
    );
    for required in [
        "::fe2o3_hsaco_finalize::verify_finalized(request.finalized_hsaco_bytes(),)",
        ".independently_revalidate_finalizer_derivation()",
        ".validate_compiler_multi_root_proof_inputs_v1()",
        ".validate_compiler_multi_root_target_lineage_v1(&proof_inputs)",
        "validate_local_request_associations_v1(request,pending,&owners)?",
    ] {
        assert_eq!(
            local_revalidation.matches(required).count(),
            1,
            "missing or repeated local owner derivation `{required}`"
        );
    }
    for required in [
        "finalizer_identity.as_bytes() == &pending.finalizer_derivation_sha256",
        "finalized_hsaco.sha256() == &pending.finalized_hsaco_sha256",
        "final_llvm.sha256() == *finalizer_module.sha256()",
        "final_llvm.sha256() == *semantic_module.sha256()",
        "pending.target == \"gfx942:xnack-\"",
        "pending.code_object_version == 6",
        "proof_roots.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1",
        "matched_reinspected_kernels",
        "descriptor_workgroup == root.workgroup()",
    ] {
        let required = compact_text(required);
        assert!(
            local_associations.contains(&required),
            "missing local join `{required}`"
        );
    }
}

#[test]
fn configured_binder_orders_all_one_shot_transitions_before_promotion() {
    assert!(configured_constructor_policy(SOURCE));
    assert!(configured_binder_policy(SOURCE));
}

#[test]
fn sibling_modules_are_exact_and_cannot_expose_authority() {
    assert!(sibling_production_policy(
        RECEIPT_SOURCE,
        REVIEWED_SIBLING_AST_FINGERPRINTS[0]
    ));
    assert!(sibling_production_policy(
        CLIENT_SOURCE,
        REVIEWED_SIBLING_AST_FINGERPRINTS[1]
    ));
    assert!(sibling_production_policy(
        SERVICE_SOURCE,
        REVIEWED_SIBLING_AST_FINGERPRINTS[2]
    ));
    assert!(test_support_policy(
        TEST_SUPPORT_SOURCE,
        REVIEWED_SIBLING_AST_FINGERPRINTS[3]
    ));
}

#[test]
fn sibling_structural_gate_rejects_every_authority_escape_surface() {
    for snippet in [
        "mod hidden_authority_surface {}",
        r"impl Default for crate::M1AllKernelsProductionProtectedVerifierV1 {
    fn default() -> Self { loop {} }
}",
        r"impl From<()> for crate::M1AllKernelsProductionProtectedVerifierV1 {
    fn from(_: ()) -> Self { loop {} }
}",
        r"fn direct_factory(
    trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
    current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,
) -> crate::M1AllKernelsProductionProtectedVerifierV1 {
    crate::M1AllKernelsProductionProtectedVerifierV1 {
        client: None, trust_policy, current_auditor,
    }
}",
        r"fn safe_new_wrapper(
    client: M1AllKernelsProtectedVerifierClientV2,
    trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
    current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,
) -> impl core::fmt::Debug {
    unsafe { crate::M1AllKernelsProductionProtectedVerifierV1::new(
        client, trust_policy, current_auditor,
    ) }
}",
        r"fn safe_verify_wrapper(verifier: &mut AnyVerifier, request: &AnyRequest) {
    let _ = unsafe { verifier.verify_protected_roster(request) };
}",
        r"fn aggregate_promoter() {
    let _ = unsafe { <WorkerV3ProtectedRosterVerificationEvidenceV1>::new(
        finalizer, compiler_execution, proof_inputs, target_lineage,
        verifier_measurement, verification_transcript, entries,
    ) };
}",
        "type AuthorityAlias = crate::M1AllKernelsProductionProtectedVerifierV1;",
        r"fn opaque_factory() -> impl core::fmt::Debug {
    unsafe { core::mem::transmute::<(), crate::M1AllKernelsProductionProtectedVerifierV1>(()) }
}",
        r"fn protected_verify_reference() {
    let _call = WorkerV3ProtectedRosterVerifierBackendV1::verify_protected_roster;
}",
        r"use crate::WorkerV3ProtectedRosterVerificationEvidenceV1 as AuthorityEvidence;
fn aliased_evidence_promoter() {
    let _ = AuthorityEvidence::new;
}",
        r"fn fabricated_evidence() -> crate::WorkerV3ProtectedRosterVerificationEvidenceV1 {
    unsafe { core::mem::zeroed() }
}",
    ] {
        let candidate = inject_before_tests(CLIENT_SOURCE, snippet);
        assert_ne!(candidate, CLIENT_SOURCE);
        assert!(!sibling_structural_policy(&candidate));
    }

    let missing_test_only_gate = TEST_SUPPORT_SOURCE.replacen("#![cfg(test)]\n", "", 1);
    assert!(!test_support_policy(
        &missing_test_only_gate,
        REVIEWED_SIBLING_AST_FINGERPRINTS[3]
    ));
}

#[test]
fn manifest_lock_and_raw_file_tripwires_match_reviewed_inputs() {
    assert!(manifest_semantics_policy(MANIFEST));
    assert!(lockfile_semantics_policy(LOCKFILE));
    assert_eq!(
        [
            source_sha256(SOURCE),
            source_sha256(RECEIPT_SOURCE),
            source_sha256(CLIENT_SOURCE),
            source_sha256(SERVICE_SOURCE),
            source_sha256(TEST_SUPPORT_SOURCE),
            source_sha256(MANIFEST),
            source_sha256(LOCKFILE),
        ],
        REVIEWED_FILE_SHA256
    );
}

#[test]
fn parsed_manifest_and_lock_reject_dependency_or_lint_drift() {
    for candidate in [
        MANIFEST.replacen("[dependencies]", "[dependencies]\nanyhow = \"1\"", 1),
        MANIFEST.replacen("[workspace]", "[workspace]\nmembers = [\"escape\"]", 1),
        MANIFEST.replacen(
            "unsafe_op_in_unsafe_fn = \"deny\"",
            "unsafe_op_in_unsafe_fn = \"allow\"",
            1,
        ),
        MANIFEST.replacen("proc-macro2 = \"=1.0.107\"", "proc-macro2 = \"1\"", 1),
    ] {
        assert_ne!(candidate, MANIFEST);
        assert!(!manifest_semantics_policy(&candidate));
    }

    let dependency_substitution = LOCKFILE.replacen(
        " \"proc-macro2\",\n \"quote\",\n \"rustix\",\n \"sha2 0.11.0\",",
        " \"proc-macro2\",\n \"quote\",\n \"rustix\",\n \"sha2 0.10.9\",",
        1,
    );
    let checksum_substitution = LOCKFILE.replacen(
        "985e7ec9bb745e6ce6535b544d84d6cd6f7ad8bd711c398938ae983b91a766d9",
        "085e7ec9bb745e6ce6535b544d84d6cd6f7ad8bd711c398938ae983b91a766d9",
        1,
    );
    for candidate in [dependency_substitution, checksum_substitution] {
        assert_ne!(candidate, LOCKFILE);
        assert!(!lockfile_semantics_policy(&candidate));
    }

    let verifier_marker = "[[package]]\nname = \"ferric-qwen3-all-kernels-worker-v3-verifier-v1\"";
    let verifier_start = LOCKFILE
        .find(verifier_marker)
        .expect("reviewed verifier package entry exists");
    let verifier_tail = &LOCKFILE[verifier_start..];
    let verifier_end = verifier_tail[verifier_marker.len()..]
        .find("[[package]]")
        .map_or(LOCKFILE.len(), |offset| {
            verifier_start + verifier_marker.len() + offset
        });
    let duplicate_verifier = format!("{LOCKFILE}\n{}", &LOCKFILE[verifier_start..verifier_end]);
    assert!(!lockfile_semantics_policy(&duplicate_verifier));
}

#[test]
fn token_fingerprint_preserves_literals_delimiters_punctuation_and_boundaries() {
    let fingerprint = |source: &str| {
        token_fingerprint(
            &source
                .parse::<TokenStream>()
                .expect("hostile token stream parses"),
        )
    };
    for (left, right) in [
        (r#""a b""#, r#""ab""#),
        ("a+=b", "a+ =b"),
        ("(a)", "[a]"),
        ("foo bar", "foobar"),
    ] {
        assert_ne!(fingerprint(left), fingerprint(right));
    }
}

#[test]
fn service_claims_come_only_from_typed_source_and_bound_current_owners() {
    let policy =
        AstVerifierPolicySource::parse(SOURCE).expect("canonical source policy must parse");
    let builder = format!(
        "{}{}{}",
        item_tokens(&policy.service_request_builder),
        item_tokens(&policy.current_audit_service_request_builder),
        item_tokens(&policy.shared_service_request_builder),
    );
    for required in [
        "project_m1_aggregate_module_handoff_v1(request.semantic_compiler_handoff().module_handoff(),)",
        "M1AllKernelsProtectedReceiptSourcePinV1::new(",
        "M1AllKernelsProtectedReceiptRequestClaimsV1::new(",
        "compiler.subject_sha256()==pending.compiler_execution_subject_sha256",
        "compiler.authenticates_signed_currentness_evidence()",
        "M1AllKernelsProtectedReceiptCompilerClaimsV1::new(",
        "compiler.current_record_verification_sha256()",
        "compiler.current_record_attestation_sha256()",
        "compiler.protected_policy_verification_sha256()",
        "compiler.protected_worker_ledger_verification_sha256()",
        "compiler.external_rollback_verification_sha256()",
        "current_audit.authenticates_expected_fresh_challenge()",
        "*verification.identity().as_bytes()",
        "*current_audit.attestation_identity().as_bytes()",
        "M1AllKernelsProtectedVerifierServiceEntryV1::new(",
        "M1AllKernelsProtectedVerifierServiceRequestV1::new(",
    ] {
        assert!(
            builder.contains(required),
            "missing request derivation `{required}`"
        );
    }
    for forbidden in [
        "[1;32]",
        "[0;32]",
        "Sha256::digest",
        "from_json",
        "std::env",
    ] {
        assert!(
            !builder.contains(forbidden),
            "fabricated request input `{forbidden}`"
        );
    }
}

#[test]
fn generic_v2_request_and_payload_snapshots_are_exact_and_sealed() {
    let policy =
        AstVerifierPolicySource::parse(SOURCE).expect("canonical source policy must parse");
    let generic = item_tokens(&policy.generic_request_builder);
    for required in [
        "challenge.into_protocol_challenge()",
        "WorkerV3VerificationRosterIdentityV1::new(pending.roster_identity)",
        "WorkerV3VerificationPolicyIdentityV1::new(*trust_policy.identity().as_bytes())",
        "WorkerV3VerificationMeasurementIdentityV1::new(trust_policy.verifier_measurement_sha256())",
        "WorkerV3VerificationFdPayloadDescriptorV1::load_envelope_v2(",
        "WorkerV3VerificationFdPayloadDescriptorV1::finalized_hsaco(",
        "WorkerV3VerificationEntryCoordinateV1::new(",
        "entry.logical_name",
        "entry.export_name",
        "entry.marker_binding_identity",
        "entry.generated_host_contract_identity",
    ] {
        assert!(
            generic.contains(required),
            "missing generic V2 input `{required}`"
        );
    }
    for forbidden in ["Default::default", "from_rng", "getrandom", "rand::"] {
        assert!(
            !generic.contains(forbidden),
            "implicit challenge authority `{forbidden}`"
        );
    }

    let snapshots = item_tokens(&policy.payload_snapshots_builder);
    let envelope = snapshots
        .find("\"ferric-worker-v3-load-envelope-v2\"")
        .expect("envelope snapshot exists");
    let hsaco = snapshots
        .find("\"ferric-worker-v3-finalized-hsaco\"")
        .expect("HSACO snapshot exists");
    assert!(
        envelope < hsaco,
        "snapshot FD order must be envelope then HSACO"
    );
    assert!(snapshots.contains("envelope_view.exact_canonical_bytes()"));
    assert!(snapshots.contains("request.finalized_hsaco_bytes()"));

    let sealed = item_tokens(&policy.sealed_snapshot_builder);
    for required in [
        "MemfdFlags::CLOEXEC|MemfdFlags::ALLOW_SEALING",
        "rustix::fs::fchmod(&writer,Mode::RUSR)",
        ".write_all(bytes)",
        ".flush()",
        "SealFlags::WRITE|SealFlags::GROW|SealFlags::SHRINK|SealFlags::SEAL",
        "rustix::fs::fcntl_add_seals(&writer,seals)",
    ] {
        assert!(
            sealed.contains(required),
            "missing sealed snapshot step `{required}`"
        );
    }

    let client = item_tokens(&production_file(CLIENT_SOURCE).expect("client source parses"));
    assert!(!client.contains("implDefaultforM1AllKernelsProtectedVerifierBeginChallengeV2"));
    assert!(!client.contains("implCloneforM1AllKernelsProtectedVerifierBeginChallengeV2"));
    assert!(!client.contains("getrandom"));
    assert!(!client.contains("rand::"));
}

#[test]
fn final_join_rechecks_and_maps_every_signed_entry_result() {
    let policy =
        AstVerifierPolicySource::parse(SOURCE).expect("canonical source policy must parse");
    let coordinates = item_tokens(&policy.entry_coordinate_join);
    let mapper = item_tokens(&policy.entry_evidence_mapper);
    let receipt = item_tokens(&policy.receipt_association);
    for required in [
        ".zip(authenticated.receipt().entries())",
        "authenticated_entry_coordinates_associate_v1(",
        "signed.generated_host_contract_identity()",
        "signed.proof_executable_binding_sha256()",
        "signed.rust_type_layout_contract_sha256()",
        "signed.rust_effect_contract_sha256()",
        "signed.safety_properties()",
        "evidence.len()==M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1",
    ] {
        assert!(
            mapper.contains(required),
            "missing signed-entry join `{required}`"
        );
    }
    for required in [
        "usize::from(signed.ordinal())==ordinal",
        "expected_ordinal==ordinal",
        "expected_lineage==Some(typed_lineage)",
        "signed.lineage_identity()==typed_lineage",
        "signed.marker_binding_identity()==expected_marker",
        "signed.generated_host_contract_identity()==expected_generated_host",
    ] {
        assert!(
            coordinates.contains(required),
            "missing coordinate join `{required}`"
        );
    }
    for required in [
        "authenticated.policy_identity()==trust_policy.identity()",
        "service_request.matches_receipt(authenticated.receipt())",
        "authenticated.receipt().verifier_measurement_sha256()==trust_policy.verifier_measurement_sha256()",
        "authenticated.receipt().checker_measurement_sha256()==trust_policy.checker_measurement_sha256()",
    ] {
        assert!(
            receipt.contains(required),
            "missing authenticated receipt join `{required}`"
        );
    }
}

fn inject_before_tests(source: &str, snippet: &str) -> String {
    source.replacen(
        TEST_MODULE_BOUNDARY,
        &format!("{snippet}\n\n{TEST_MODULE_BOUNDARY}"),
        1,
    )
}

#[test]
fn comments_and_test_tail_literals_cannot_supply_production_policy_nodes() {
    let comment_decoy = format!(
        "/* nested /* unsafe impl FakeBackend */ fn verify_protected_roster() {{}} */\n{SOURCE}"
    );
    assert!(zero_state_policy(&comment_decoy));
    assert!(configured_constructor_policy(&comment_decoy));
    assert!(configured_binder_policy(&comment_decoy));

    let production_literal_decoy = SOURCE.replacen(
        "    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {\n        match self {",
        r###"    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _raw_decoy = r##"impl M1AllKernelsProductionProtectedVerifierV1 { pub fn from_parts() {} }"##;
        let _string_decoy = "WorkerV3ProtectedRosterVerificationEvidenceV1::new";
        let _char_decoy = '}';
        match self {"###,
        1,
    );
    assert!(AstVerifierPolicySource::parse(&production_literal_decoy).is_none());

    let test_tail_decoy = SOURCE.replacen(
        TEST_MODULE_BOUNDARY,
        r###"#[cfg(test)]
mod tests {
    const RAW_DECOY: &str = r##"impl M1AllKernelsProductionProtectedVerifierV1 { pub fn from_parts() {} }"##;
    const STRING_DECOY: &str = "WorkerV3ProtectedRosterVerificationEvidenceV1::new";
    const CHAR_DECOY: char = '}';"###,
        1,
    );
    assert!(zero_state_policy(&test_tail_decoy));
    assert!(configured_constructor_policy(&test_tail_decoy));
    assert!(configured_binder_policy(&test_tail_decoy));
}

#[test]
fn comments_and_strings_cannot_replace_real_backend_obligations() {
    let changed = SOURCE.replacen(
        "Self::reject_missing_protected_receipt(",
        "unsafe_success_without_receipt(",
        1,
    );
    assert_ne!(changed, SOURCE);
    let comment_decoy =
        format!("/* unsafe impl Fake {{ Self::reject_missing_protected_receipt( */\n{changed}");
    let string_decoy = format!(
        "const ZERO_POLICY_DECOY: &str = r#\"Self::reject_missing_protected_receipt(\"#;\n{changed}"
    );
    assert!(!zero_state_policy(&changed));
    assert!(!zero_state_policy(&comment_decoy));
    assert!(!zero_state_policy(&string_decoy));

    let changed_configured = SOURCE.replacen(
        ".submit_current_record(",
        ".accept_unbound_current_record(",
        1,
    );
    let configured_comment =
        format!("/* unsafe impl Fake {{ .submit_current_record( */\n{changed_configured}");
    let configured_string = format!(
        "const CONFIGURED_POLICY_DECOY: &str = r#\".submit_current_record(\"#;\n{changed_configured}"
    );
    assert!(!configured_binder_policy(&configured_comment));
    assert!(!configured_binder_policy(&configured_string));
}

#[test]
fn exact_impl_and_direct_child_extraction_reject_wrong_and_nested_decoys() {
    let wrong_zero = SOURCE.replacen(
        "    unsafe fn verify_protected_roster(",
        "    unsafe fn bypass_protected_roster(",
        1,
    );
    let wrong_impl_decoy = inject_before_tests(
        &wrong_zero,
        "struct WrongVerifier;\nimpl WrongVerifier {\n    unsafe fn verify_protected_roster() { Self::reject_missing_protected_receipt() }\n}",
    );
    assert!(!zero_state_policy(&wrong_impl_decoy));

    let nested_decoy = SOURCE.replacen(
        "    unsafe fn verify_protected_roster(",
        "    fn nested_decoy() { unsafe fn verify_protected_roster() {} }\n\n    unsafe fn bypass_protected_roster(",
        1,
    );
    assert!(!zero_state_policy(&nested_decoy));

    let wrong_configured = SOURCE.replacen(
        "    for M1AllKernelsProductionProtectedVerifierV1\n{",
        "    for WrongProductionProtectedVerifierV1\n{",
        1,
    );
    assert_ne!(wrong_configured, SOURCE);
    assert!(!configured_binder_policy(&wrong_configured));

    let configured_nested_decoy = SOURCE.replacen(
        "    type Error = M1AllKernelsProductionProtectedVerifierErrorV1;\n\n    unsafe fn verify_protected_roster(",
        "    type Error = M1AllKernelsProductionProtectedVerifierErrorV1;\n\n    fn nested_decoy() { unsafe fn verify_protected_roster() {} }\n\n    unsafe fn bypass_protected_roster(",
        1,
    );
    assert_ne!(configured_nested_decoy, SOURCE);
    assert!(!configured_binder_policy(&configured_nested_decoy));
}

#[test]
fn safe_inherent_and_trait_factories_cannot_construct_the_configured_backend() {
    let insertion =
        "    /// Configuration alone grants no verification, load, or launch authority.";
    for factory in [
        r"    pub fn from_parts(
        client: M1AllKernelsProtectedVerifierClientV2,
        trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
        current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,
    ) -> Self {
        Self { client: Some(client), trust_policy, current_auditor }
    }

",
        r"    fn private_from_parts(
        client: M1AllKernelsProtectedVerifierClientV2,
        trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
        current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,
    ) -> Self {
        Self { client: Some(client), trust_policy, current_auditor }
    }

",
        r"    pub fn safe_wrapper(
        client: M1AllKernelsProtectedVerifierClientV2,
        trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
        current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,
    ) -> Self {
        unsafe { Self::new(client, trust_policy, current_auditor) }
    }

",
    ] {
        let candidate = SOURCE.replacen(insertion, &format!("{factory}{insertion}"), 1);
        assert_ne!(candidate, SOURCE);
        assert!(!configured_constructor_policy(&candidate));
    }

    let qualified_default = inject_before_tests(
        SOURCE,
        r"impl Default for crate::M1AllKernelsProductionProtectedVerifierV1 {
    fn default() -> Self { loop {} }
}",
    );
    assert!(!configured_constructor_policy(&qualified_default));
}

#[test]
fn configured_backend_fields_are_exact_private_and_typed() {
    for candidate in [
        SOURCE.replacen(
            "    current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,\n}",
            "    current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,\n    bypass: bool,\n}",
            1,
        ),
        SOURCE.replacen(
            "    client: Option<M1AllKernelsProtectedVerifierClientV2>,",
            "    pub client: Option<M1AllKernelsProtectedVerifierClientV2>,",
            1,
        ),
        SOURCE.replacen(
            "    trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,",
            "    trust_policy: Option<M1AllKernelsProtectedVerifierTrustPolicyV1>,",
            1,
        ),
    ] {
        assert_ne!(candidate, SOURCE);
        assert!(!configured_constructor_policy(&candidate));
    }
}

#[test]
fn zero_constructor_and_configured_debug_are_exact_sensitive_nodes() {
    let zero_constructor_drift = SOURCE.replacen(
        "    pub const fn new() -> Self {\n        Self\n    }",
        "    pub const fn new() -> Self {\n        loop {}\n    }",
        1,
    );
    assert_ne!(zero_constructor_drift, SOURCE);
    assert!(AstVerifierPolicySource::parse(&zero_constructor_drift).is_none());

    let debug_drift = SOURCE.replacen(
        ".field(\"client_available\", &self.client.is_some())",
        ".field(\"client_available\", &true)",
        1,
    );
    assert_ne!(debug_drift, SOURCE);
    assert!(AstVerifierPolicySource::parse(&debug_drift).is_none());
}

#[test]
fn free_factories_verify_wrappers_and_promoters_are_rejected_globally() {
    let free_factory = inject_before_tests(
        SOURCE,
        r"pub fn configured_factory(
    client: M1AllKernelsProtectedVerifierClientV2,
    trust_policy: M1AllKernelsProtectedVerifierTrustPolicyV1,
    current_auditor: InheritedWorkerV3CompilerCurrentRecordAuditorV1,
) -> M1AllKernelsProductionProtectedVerifierV1 {
    unsafe { crate::M1AllKernelsProductionProtectedVerifierV1::new(client, trust_policy, current_auditor) }
}",
    );
    assert!(!configured_constructor_policy(&free_factory));

    let verify_wrapper = inject_before_tests(
        SOURCE,
        r"pub fn safe_verify_wrapper(
    verifier: &mut M1AllKernelsProductionProtectedVerifierV1,
    request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
) -> Result<WorkerV3ProtectedRosterVerificationEvidenceV1, M1AllKernelsProductionProtectedVerifierErrorV1> {
    unsafe { verifier.verify_protected_roster(request) }
}",
    );
    assert!(!configured_binder_policy(&verify_wrapper));

    let second_promotion = inject_before_tests(
        SOURCE,
        r"fn extra_promotion() {
    let _ = unsafe {
        crate::WorkerV3ProtectedRosterVerificationEvidenceV1::new(
            finalizer, compiler_execution, proof_inputs, target_lineage,
            verifier_measurement, verification_transcript, entries,
        )
    };
}",
    );
    assert!(!configured_binder_policy(&second_promotion));

    let qualified_promotion = SOURCE.replacen(
        "WorkerV3ProtectedRosterVerificationEvidenceV1::new(\n                finalizer,",
        "crate::WorkerV3ProtectedRosterVerificationEvidenceV1::new(\n                finalizer,",
        1,
    );
    assert_ne!(qualified_promotion, SOURCE);
    assert!(!configured_binder_policy(&qualified_promotion));
}

#[test]
fn early_return_closure_and_prefixed_identifier_decoys_are_rejected() {
    let early_return = SOURCE.replacen(
        "        let pending = M1AllKernelsPendingRequestProjectionV1::from_request(request).ok_or(",
        "        if false { return Err(M1AllKernelsProductionProtectedVerifierErrorV1::RosterRequestAssociationFailed); }\n        let pending = M1AllKernelsPendingRequestProjectionV1::from_request(request).ok_or(",
        1,
    );
    assert_ne!(early_return, SOURCE);
    assert!(!configured_binder_policy(&early_return));

    let closure_decoy = SOURCE.replacen(
        "        let bound_service_request = protected_service_request_v1(\n            request,",
        "        let _body_decoy = || protected_service_request_v1(\n            request, &pending, &compiler_execution, &self.trust_policy,\n        );\n        let bound_service_request = unreviewed_service_request_v1(\n            request,",
        1,
    );
    assert_ne!(closure_decoy, SOURCE);
    assert!(!configured_binder_policy(&closure_decoy));

    let prefixed_call =
        SOURCE.replacen(".submit_current_record(", ".evil_submit_current_record(", 1);
    assert_ne!(prefixed_call, SOURCE);
    assert!(!configured_binder_policy(&prefixed_call));

    let prefixed_helper = SOURCE.replacen(
        "fn locally_revalidate_request_v1(",
        "fn decoyfn_locally_revalidate_request_v1(",
        1,
    );
    assert_ne!(prefixed_helper, SOURCE);
    assert!(!zero_state_policy(&prefixed_helper));
    assert!(!configured_binder_policy(&prefixed_helper));
}

#[test]
fn configured_association_and_entry_results_cannot_be_discarded() {
    let discarded_association = SOURCE.replacen(
        "        authenticated_receipt_associates_v1(\n            &authenticated,\n            &service_request,\n            &self.trust_policy,\n        )\n        .then_some(())\n        .ok_or(\n            M1AllKernelsProductionProtectedVerifierErrorV1::AuthenticatedReceiptAssociationFailed,\n        )?;",
        "        let _discarded_association = (\n            authenticated_receipt_associates_v1(\n                &authenticated, &service_request, &self.trust_policy,\n            ),\n            true,\n        ).1;",
        1,
    );
    assert_ne!(discarded_association, SOURCE);
    assert!(!configured_binder_policy(&discarded_association));

    let discarded_entries = SOURCE.replacen(
        "        let entries = authenticated_entry_evidence_v1(request, &pending, &authenticated)?;",
        "        let entries = (\n            authenticated_entry_evidence_v1(request, &pending, &authenticated)?,\n            Vec::<WorkerV3ProtectedRosterEntryEvidenceV1>::new(),\n        ).1;",
        1,
    );
    assert_ne!(discarded_entries, SOURCE);
    assert!(!configured_binder_policy(&discarded_entries));
}

#[test]
fn exact_helper_fingerprints_reject_unreachable_and_closure_decoys() {
    let local_revalidation_decoy = SOURCE.replacen(
        "    let hsaco_reinspection = ::fe2o3_hsaco_finalize::verify_finalized(",
        "    let _decoy = || request.finalized_hsaco_bytes();\n    let hsaco_reinspection = ::fe2o3_hsaco_finalize::verify_finalized(",
        1,
    );
    let missing_receipt_decoy = SOURCE.replacen(
        "        expected_roster_entries: M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1,",
        "        expected_roster_entries: 0,",
        1,
    );
    let receipt_decoy = SOURCE.replacen(
        "    authenticated.policy_identity() == trust_policy.identity()",
        "    { let _decoy = || authenticated.policy_identity() == trust_policy.identity(); true }",
        1,
    );
    let coordinate_decoy = SOURCE.replacen(
        "    usize::from(signed.ordinal()) == ordinal",
        "    { let _decoy = || usize::from(signed.ordinal()) == ordinal; true }",
        1,
    );
    let local_association_decoy = SOURCE.replacen(
        "    let finalizer_identity = owners.finalizer.identity();",
        "    if false { return Ok(()); }\n    let finalizer_identity = owners.finalizer.identity();",
        1,
    );
    let service_builder_decoy = SOURCE.replacen(
        "    let source_pin = project_m1_aggregate_module_handoff_v1(",
        "    let _decoy = || project_m1_aggregate_module_handoff_v1(\n        request.semantic_compiler_handoff().module_handoff(),\n    );\n    let source_pin = project_m1_aggregate_module_handoff_v1(",
        1,
    );
    let entry_mapper_decoy = SOURCE.replacen(
        "                signed.safety_properties(),",
        "                {\n                    let _decoy = || signed.safety_properties();\n                    fe2o3_host::WorkerV3SafetyPropertiesV1::required()\n                },",
        1,
    );
    for candidate in [
        local_revalidation_decoy,
        missing_receipt_decoy,
        receipt_decoy,
        coordinate_decoy,
        local_association_decoy,
        service_builder_decoy,
        entry_mapper_decoy,
    ] {
        assert_ne!(candidate, SOURCE);
        assert!(AstVerifierPolicySource::parse(&candidate).is_none());
    }
}

#[test]
fn macro_alias_attribute_and_derive_evasions_are_rejected() {
    let overwrite = SOURCE.replacen("::core::write!(", "overwrite!(", 1);
    assert_ne!(overwrite, SOURCE);
    assert!(AstVerifierPolicySource::parse(&overwrite).is_none());

    let aliased_write = inject_before_tests(
        &SOURCE.replacen("::core::write!(", "write!(", 1),
        "use hostile_macro as write;",
    );
    assert!(AstVerifierPolicySource::parse(&aliased_write).is_none());

    let local_write = inject_before_tests(SOURCE, "macro_rules! write { ($($token:tt)*) => {} }");
    assert!(AstVerifierPolicySource::parse(&local_write).is_none());

    for attribute in [
        "#[derive(Default)]",
        "#[cfg_attr(any(), derive(Default))]",
        "#[evil::authority]",
    ] {
        let candidate = SOURCE.replacen(
            "pub struct M1AllKernelsProductionProtectedVerifierV1 {",
            &format!("{attribute}\npub struct M1AllKernelsProductionProtectedVerifierV1 {{"),
            1,
        );
        assert_ne!(candidate, SOURCE);
        assert!(!configured_constructor_policy(&candidate));
    }

    let macro_use = SOURCE.replacen(
        "mod protected_verifier_test_support;",
        "#[macro_use]\nmod protected_verifier_test_support;",
        1,
    );
    assert!(AstVerifierPolicySource::parse(&macro_use).is_none());

    let missing_unsafe_lint = SOURCE.replacen("#![deny(unsafe_op_in_unsafe_fn)]\n", "", 1);
    assert!(AstVerifierPolicySource::parse(&missing_unsafe_lint).is_none());
    let weakened_unsafe_lint = SOURCE.replacen(
        "#![deny(unsafe_op_in_unsafe_fn)]",
        "#![allow(unsafe_op_in_unsafe_fn)]",
        1,
    );
    assert!(AstVerifierPolicySource::parse(&weakened_unsafe_lint).is_none());
}

#[test]
fn display_bodies_and_macro_owners_are_exact() {
    let message_substitution = SOURCE.replacen(
        "local aggregate request revalidation failed: {error}",
        "accepted unreviewed local request: {error}",
        1,
    );
    assert_ne!(message_substitution, SOURCE);
    assert!(AstVerifierPolicySource::parse(&message_substitution).is_none());

    let moved_macro = SOURCE.replacen(
        "            Self::LocalRevalidation(error) => {\n                ::core::write!(",
        "            Self::LocalRevalidation(error) => {\n                if false { ::core::write!(formatter, \"decoy {error}\")?; }\n                ::core::write!(",
        1,
    );
    assert_ne!(moved_macro, SOURCE);
    assert!(AstVerifierPolicySource::parse(&moved_macro).is_none());
}

#[test]
fn macro_test_module_and_post_test_decoys_cannot_supply_production_authority() {
    let macro_decoy = inject_before_tests(
        SOURCE,
        "macro_rules! emit_backend { () => { unsafe impl FakeBackend {} }; }\nemit_backend!();",
    );
    assert!(AstVerifierPolicySource::parse(&macro_decoy).is_none());

    let changed = SOURCE.replacen(
        ".submit_current_record(",
        ".accept_unbound_current_record(",
        1,
    );
    let test_decoy = changed.replacen(
        TEST_MODULE_BOUNDARY,
        "#[cfg(test)]\nmod tests {\n    fn submit_current_record_decoy() { client.submit_current_record(); }",
        1,
    );
    assert!(!configured_binder_policy(&test_decoy));

    let post_test_decoy = format!("{SOURCE}\nunsafe impl FakePostTestBackend {{}}");
    assert!(AstVerifierPolicySource::parse(&post_test_decoy).is_none());
}

#[test]
fn unsafe_constructor_contract_cannot_be_satisfied_by_detached_comment_text() {
    let safe_constructor = SOURCE.replacen("pub unsafe fn new(", "pub fn new(", 1);
    assert!(!configured_constructor_policy(&safe_constructor));

    let omitted = SOURCE.replacen(
        "protected signing key must be bound to the exact admitted verifier",
        "signing-key binding requirement omitted",
        1,
    );
    let detached_decoy = omitted.replacen(
        "    /// Takes ownership of all caller-admitted one-shot deployment inputs.",
        "    /* protected signing key must be bound to the exact admitted verifier */\n    /// Takes ownership of all caller-admitted one-shot deployment inputs.",
        1,
    );
    assert!(!configured_constructor_policy(&omitted));
    assert!(!configured_constructor_policy(&detached_decoy));
}

#[test]
fn owner_custody_and_promotion_order_evasions_are_rejected() {
    let dropped_zero_owner = SOURCE.replacen(
        "            owners.finalizer,\n            owners.proof_inputs,",
        "            (),\n            owners.proof_inputs,",
        1,
    );
    assert_ne!(dropped_zero_owner, SOURCE);
    assert!(!zero_state_policy(&dropped_zero_owner));

    let dropped_production_owner = SOURCE.replacen(
        "                finalizer,\n                compiler_execution,\n                proof_inputs,",
        "                unsafe { core::mem::zeroed() },\n                compiler_execution,\n                proof_inputs,",
        1,
    );
    assert_ne!(dropped_production_owner, SOURCE);
    assert!(!configured_binder_policy(&dropped_production_owner));

    let premature_promotion = SOURCE.replacen(
        "        let authenticated = pending_client",
        "        let _premature = unsafe {\n            WorkerV3ProtectedRosterVerificationEvidenceV1::new(\n                finalizer, compiler_execution, proof_inputs, target_lineage,\n                verifier_measurement, verification_transcript, entries,\n            )\n        };\n        let authenticated = pending_client",
        1,
    );
    assert_ne!(premature_promotion, SOURCE);
    assert!(!configured_binder_policy(&premature_promotion));

    let discarded_valid_promotion = SOURCE.replacen(
        "        Ok(unsafe {\n            WorkerV3ProtectedRosterVerificationEvidenceV1::new(\n                finalizer,\n                compiler_execution,\n                proof_inputs,\n                target_lineage,\n                verifier_measurement,\n                verification_transcript,\n                entries,\n            )\n        })",
        "        Ok(unsafe {\n            let valid = WorkerV3ProtectedRosterVerificationEvidenceV1::new(\n                finalizer,\n                compiler_execution,\n                proof_inputs,\n                target_lineage,\n                verifier_measurement,\n                verification_transcript,\n                entries,\n            );\n            core::mem::forget(valid);\n            core::mem::zeroed()\n        })",
        1,
    );
    assert_ne!(discarded_valid_promotion, SOURCE);
    assert!(!configured_binder_policy(&discarded_valid_promotion));
}

#[test]
fn request_and_entry_join_guards_ignore_wrong_scope_and_comment_decoys() {
    let changed_revalidation = SOURCE.replacen(
        "::fe2o3_hsaco_finalize::verify_finalized(",
        "unreviewed_finalized_bytes(",
        1,
    );
    let revalidation_decoy = inject_before_tests(
        &changed_revalidation,
        "fn wrong_owner_source() { ::fe2o3_hsaco_finalize::verify_finalized(request.finalized_hsaco_bytes()); }",
    );
    assert!(AstVerifierPolicySource::parse(&revalidation_decoy).is_none());

    let request_join = "compiler.subject_sha256()==pending.compiler_execution_subject_sha256";
    let changed_request = SOURCE.replacen(
        "compiler.subject_sha256() == pending.compiler_execution_subject_sha256",
        "true",
        1,
    );
    let request_decoy = inject_before_tests(
        &changed_request,
        &format!("fn wrong_request_builder() {{ let _ = {request_join}; }}"),
    );
    assert!(AstVerifierPolicySource::parse(&request_decoy).is_none());

    let changed_entry = SOURCE.replacen("signed.safety_properties(),", "incomplete_safety,", 1);
    let entry_decoy = format!("/* signed.safety_properties(), */\n{changed_entry}");
    assert!(AstVerifierPolicySource::parse(&entry_decoy).is_none());
}

#[test]
fn production_source_contains_no_deployment_values_or_runtime_surface() {
    for forbidden in [
        "std::env",
        "UnixStream::connect",
        "SigningKey",
        "CURRENT.json",
        "fe2o3_kfd",
        "fe2o3_hsa_runtime",
        "authorize_hsa_load",
        "synthetic_for_test_only",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "forbidden surface `{forbidden}`"
        );
    }
    assert!(!MANIFEST.contains("worker-v3-verifier-test-support"));
    assert!(MANIFEST.contains("[workspace]"));
    assert!(MANIFEST.contains("fe2o3-verifier ="));
    assert!(MANIFEST.contains("fe2o3-worker-v3-verification-client ="));
    assert!(MANIFEST.contains("fe2o3-worker-v3-verification-protocol ="));
    assert!(MANIFEST.contains(
        "ferric-qwen3-all-kernels-worker-v3-source-pin-v1 = { path = \"../qwen3-all-kernels-worker-v3-source-pin-v1\" }"
    ));
    let revisions = MANIFEST
        .lines()
        .filter(|line| line.starts_with("fe2o3-"))
        .map(|line| {
            line.split_once("rev = \"")
                .and_then(|(_, tail)| tail.split_once('"'))
                .map(|(revision, _)| revision)
                .expect("every direct fe2o3 dependency is pinned")
        })
        .collect::<Vec<_>>();
    assert_eq!(revisions.len(), 8);
    assert!(revisions.iter().all(|revision| *revision == FE2O3_REVISION));
}

#[test]
fn documentation_states_prerequisites_and_nonclaims() {
    let normalized = README.split_whitespace().collect::<Vec<_>>().join(" ");
    for statement in [
        "zero-state default",
        "always returns `MissingProtectedVerificationReceipt`",
        "previously admitted one-shot V2 service client",
        "Begin challenge already reserved in durable deployment replay state",
        "caller-provisioned trust policy",
        "inherited FD195 compiler-current auditor",
        "requires the resulting service request to byte-match the pre-bind request",
        "maps all 12 signed proof-to-executable, Rust type-layout, Rust effect",
        "generic V2 now transports immutable envelope and HSACO snapshots",
        "Signing caller-supplied hash echoes does not satisfy",
        "A production deployment still must provide",
        "embeds none of those deployment values",
        "does not provide a service process, signing key, real receipt, model bundle, `CURRENT` record, qualification result, or GPU result",
        "grants no publication, load, launch, or inference authority by itself",
    ] {
        assert!(
            normalized.contains(statement),
            "README missing `{statement}`"
        );
    }
}
