//! Source-level policy checks for the fail-closed aggregate verifier adapter.

use fe2o3_host::CompilerGeneratedKernelExpectationRosterV1;
use ferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;
use std::ops::Range;

const SOURCE: &str = include_str!("../src/lib.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const README: &str = include_str!("../README.md");

const FE2O3_REVISION: &str = "57d2d9ced5c113d40546ea1dee603e8ba499cf40";
const PRODUCTION_VERIFY_METHOD_ITEM: &str = "unsafe fn verify_protected_roster";
const PRODUCTION_REJECTION_HELPER_ITEM: &str = "fn reject_missing_protected_receipt";
const COMPACT_VERIFY_METHOD_ITEM: &str = "unsafefnverify_protected_roster(";
const COMPACT_REJECTION_HELPER_ITEM: &str = "fnreject_missing_protected_receipt<";
const COMPACT_REJECTION_HELPER_NAME: &str = "fnreject_missing_protected_receipt";
const COMPACT_INHERENT_IMPL_HEADER: &str = "implM1AllKernelsProtectedVerifierV1{";
const COMPACT_BACKEND_IMPL_HEADER: &str = "unsafeimplWorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1>forM1AllKernelsProtectedVerifierV1{";
const PRIVATE_REJECTION_HELPER_LEAD_IN: &str = "    pub const fn new() -> Self {\n        Self\n    }\n\n    fn reject_missing_protected_receipt";
const TEST_MODULE_BOUNDARY: &str = "#[cfg(test)]\nmod tests {";
const REQUIRED_GFX942_TARGET_LITERAL: &str = "\"gfx942:xnack-\"";
const FINALIZER_CRATE_IDENTIFIER: &str = "fe2o3_hsaco_finalize";
const FINALIZED_VERIFIER_IDENTIFIER: &str = "verify_finalized";
const LOW_LEVEL_INSPECTOR_IDENTIFIER: &str = "inspect_and_bind_kernel_descriptors";
const FULLY_QUALIFIED_FINALIZED_VERIFIER: &str =
    "::fe2o3_hsaco_finalize::verify_finalized(request.finalized_hsaco_bytes())";

struct ProtectedBoundaryRanges {
    verify_method: Range<usize>,
    rejection_helper: Range<usize>,
}

fn mask_non_newline_bytes(bytes: &mut [u8]) {
    for byte in bytes {
        if !matches!(*byte, b'\n' | b'\r') {
            *byte = b' ';
        }
    }
}

fn raw_string_opening(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    if bytes.get(start) != Some(&b'r') {
        return None;
    }
    let mut cursor = start + 1;
    let mut hash_count = 0;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
        hash_count += 1;
    }
    (bytes.get(cursor) == Some(&b'"')).then_some((cursor, hash_count))
}

fn char_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'\'') {
        return None;
    }
    let mut cursor = start + 1;
    if bytes.get(cursor) == Some(&b'\\') {
        cursor += 1;
        match *bytes.get(cursor)? {
            b'u' if bytes.get(cursor + 1) == Some(&b'{') => {
                cursor += 2;
                while !matches!(bytes.get(cursor), None | Some(&b'}')) {
                    cursor += 1;
                }
                if bytes.get(cursor) != Some(&b'}') {
                    return None;
                }
                cursor += 1;
            }
            b'x' => cursor += 3,
            _ => cursor += 1,
        }
    } else {
        let width = match *bytes.get(cursor)? {
            0x00..=0x7f => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => 4,
        };
        cursor += width;
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

fn rust_code_without_comments_or_strings(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut code = bytes.to_vec();
    let mut index = 0;
    while index < bytes.len() {
        if bytes.get(index..index + 2) == Some(b"//") {
            let start = index;
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            mask_non_newline_bytes(&mut code[start..index]);
            continue;
        }
        if bytes.get(index..index + 2) == Some(b"/*") {
            let start = index;
            let mut depth = 1_usize;
            index += 2;
            while index < bytes.len() && depth != 0 {
                if bytes.get(index..index + 2) == Some(b"/*") {
                    depth += 1;
                    index += 2;
                } else if bytes.get(index..index + 2) == Some(b"*/") {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            assert_eq!(
                depth, 0,
                "production source has an unterminated block comment"
            );
            mask_non_newline_bytes(&mut code[start..index]);
            continue;
        }
        if let Some((opening_quote, hash_count)) = raw_string_opening(bytes, index) {
            let start = index;
            index = opening_quote + 1;
            let mut closed = false;
            while index < bytes.len() {
                let closing_end = index + 1 + hash_count;
                if bytes[index] == b'"'
                    && closing_end <= bytes.len()
                    && bytes[index + 1..closing_end]
                        .iter()
                        .all(|byte| *byte == b'#')
                {
                    index = closing_end;
                    closed = true;
                    break;
                }
                index += 1;
            }
            assert!(closed, "production source has an unterminated raw string");
            mask_non_newline_bytes(&mut code[start..index]);
            continue;
        }
        if bytes[index] == b'"' {
            let start = index;
            let mut escaped = false;
            let mut closed = false;
            index += 1;
            while index < bytes.len() {
                let byte = bytes[index];
                index += 1;
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    closed = true;
                    break;
                }
            }
            assert!(closed, "production source has an unterminated string");
            if bytes.get(start..index) != Some(REQUIRED_GFX942_TARGET_LITERAL.as_bytes()) {
                mask_non_newline_bytes(&mut code[start..index]);
            }
            continue;
        }
        if let Some(end) = char_literal_end(bytes, index) {
            mask_non_newline_bytes(&mut code[index..end]);
            index = end;
            continue;
        }
        index += 1;
    }
    String::from_utf8(code).expect("masked Rust source must remain UTF-8")
}

fn compact_rust_code(code: &str) -> String {
    code.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn compact_production_code(source: &str) -> Option<String> {
    let code = rust_code_without_comments_or_strings(source);
    let (production, _tests) = split_complete_production_and_tests(&code)?;
    Some(compact_rust_code(production))
}

const fn is_identifier_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn contains_identifier(code: &str, identifier: &str) -> bool {
    code.match_indices(identifier).any(|(start, _)| {
        let before = start
            .checked_sub(1)
            .and_then(|index| code.as_bytes().get(index));
        let after = code.as_bytes().get(start + identifier.len());
        before.is_none_or(|byte| !is_identifier_byte(*byte))
            && after.is_none_or(|byte| !is_identifier_byte(*byte))
    })
}

fn unique_occurrence(code: &str, needle: &str) -> Option<usize> {
    let mut occurrences = code.match_indices(needle);
    let (start, _) = occurrences.next()?;
    occurrences.next().is_none().then_some(start)
}

fn balanced_braced_item_range(code: &str, start: usize, limit: usize) -> Option<Range<usize>> {
    let opening = code.get(start..limit)?.find('{')? + start;
    let mut depth = 0_usize;
    for (offset, byte) in code.as_bytes()[opening..limit].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(start..opening + offset + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn unique_balanced_impl_range(code: &str, header: &str) -> Option<Range<usize>> {
    let start = unique_occurrence(code, header)?;
    let range = balanced_braced_item_range(code, start, code.len())?;
    (range.start + header.len() <= range.end).then_some(range)
}

fn brace_depth_at(code: &str, implementation: &Range<usize>, position: usize) -> Option<usize> {
    if position <= implementation.start || position >= implementation.end {
        return None;
    }
    let opening = code
        .get(implementation.clone())?
        .find('{')?
        .checked_add(implementation.start)?;
    let mut depth = 0_usize;
    for byte in &code.as_bytes()[opening..position] {
        match byte {
            b'{' => depth += 1,
            b'}' => depth = depth.checked_sub(1)?,
            _ => {}
        }
    }
    Some(depth)
}

fn item_range_within(
    code: &str,
    item_name: &str,
    implementation: &Range<usize>,
) -> Option<Range<usize>> {
    let start = unique_occurrence(code, item_name)?;
    if start < implementation.start || start + item_name.len() > implementation.end {
        return None;
    }
    if brace_depth_at(code, implementation, start)? != 1 {
        return None;
    }
    let item = balanced_braced_item_range(code, start, implementation.end)?;
    (item.end <= implementation.end).then_some(item)
}

fn protected_boundary_ranges(compact: &str) -> Option<ProtectedBoundaryRanges> {
    if compact.contains("r#") {
        return None;
    }
    let inherent_impl = unique_balanced_impl_range(compact, COMPACT_INHERENT_IMPL_HEADER)?;
    let backend_impl = unique_balanced_impl_range(compact, COMPACT_BACKEND_IMPL_HEADER)?;
    let verify_method = item_range_within(compact, COMPACT_VERIFY_METHOD_ITEM, &backend_impl)?;
    let rejection_helper =
        item_range_within(compact, COMPACT_REJECTION_HELPER_ITEM, &inherent_impl)?;
    let private_lead_in = compact_rust_code(PRIVATE_REJECTION_HELPER_LEAD_IN);
    compact
        .get(inherent_impl)
        .is_some_and(|item| item.contains(&private_lead_in))
        .then_some(ProtectedBoundaryRanges {
            verify_method,
            rejection_helper,
        })
}

fn has_unique_private_boundary_items(code: &str) -> bool {
    protected_boundary_ranges(&compact_rust_code(code)).is_some()
}

fn protected_verify_method(source: &str) -> Option<String> {
    let code = rust_code_without_comments_or_strings(source);
    let (production, _tests) = split_complete_production_and_tests(&code)?;
    let compact = compact_rust_code(production);
    let ranges = protected_boundary_ranges(&compact)?;
    compact.get(ranges.verify_method).map(str::to_owned)
}

fn split_complete_production_and_tests(code: &str) -> Option<(&str, &str)> {
    if code.matches("#[cfg(test)]").count() != 1 {
        return None;
    }
    let (production, tests_after_opening) = code.split_once(TEST_MODULE_BOUNDARY)?;
    let mut depth = 1_usize;
    for (offset, byte) in tests_after_opening.bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    if tests_after_opening[offset + 1..].trim().is_empty() {
                        return Some((production, &tests_after_opening[..offset]));
                    }
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

fn has_no_production_macro_authority(code: &str) -> bool {
    let Some((production, _tests)) = split_complete_production_and_tests(code) else {
        return false;
    };
    let mut compact = production.split_whitespace().collect::<String>();
    for (attribute, expected_count) in [
        ("#![deny(missing_docs)]", 1),
        ("#![deny(unsafe_op_in_unsafe_fn)]", 1),
        ("#[allow(dead_code)]", 5),
        ("#[derive(Clone,Copy,Debug,Eq,PartialEq)]", 1),
        ("#[non_exhaustive]", 1),
        ("#[derive(Clone,Copy,Debug,Default)]", 1),
        ("#[must_use]", 1),
        ("#[allow(clippy::too_many_lines)]", 1),
    ] {
        if compact.matches(attribute).count() != expected_count {
            return false;
        }
        compact = compact.replace(attribute, "");
    }
    let allowed_checked_coverage_negation = "!*matched";
    if compact.matches(allowed_checked_coverage_negation).count() != 1 {
        return false;
    }
    compact = compact.replacen(allowed_checked_coverage_negation, "false==*matched", 1);
    if compact.contains("#[") || compact.contains("#![") {
        return false;
    }
    let bytes = compact.as_bytes();
    !bytes
        .iter()
        .enumerate()
        .any(|(index, byte)| *byte == b'!' && bytes.get(index + 1).is_none_or(|next| *next != b'='))
}

fn identifier_token_count(source: &str, identifier: &str) -> usize {
    source
        .split(|character: char| character != '_' && !character.is_alphanumeric())
        .filter(|token| *token == identifier)
        .count()
}

fn anchored_verifier_method_satisfies_policy(candidate: &str) -> bool {
    let Some(canonical) = protected_verify_method(SOURCE) else {
        return false;
    };
    candidate == canonical
        && candidate
            .matches(FULLY_QUALIFIED_FINALIZED_VERIFIER)
            .count()
            == 1
        && identifier_token_count(candidate, FINALIZED_VERIFIER_IDENTIFIER) == 1
}

fn protected_verifier_policy_accepts(source: &str) -> bool {
    let code = rust_code_without_comments_or_strings(source);
    let Some((production, _tests)) = split_complete_production_and_tests(&code) else {
        return false;
    };
    if !has_unique_private_boundary_items(production)
        || !has_no_production_macro_authority(&code)
        || !production_has_no_external_authority_surface(source)
    {
        return false;
    }
    let compact = compact_rust_code(production);
    let Some(ranges) = protected_boundary_ranges(&compact) else {
        return false;
    };
    let Some(method) = compact.get(ranges.verify_method) else {
        return false;
    };
    if !anchored_verifier_method_satisfies_policy(method)
        || identifier_token_count(production, FINALIZER_CRATE_IDENTIFIER) != 1
        || identifier_token_count(production, FINALIZED_VERIFIER_IDENTIFIER) != 1
        || identifier_token_count(production, LOW_LEVEL_INSPECTOR_IDENTIFIER) != 0
    {
        return false;
    }

    let Some(canonical_code) = compact_production_code(SOURCE) else {
        return false;
    };
    let Some(canonical_ranges) = protected_boundary_ranges(&canonical_code) else {
        return false;
    };
    compact.get(ranges.rejection_helper) == canonical_code.get(canonical_ranges.rejection_helper)
}

#[test]
fn backend_is_specific_to_the_exact_aggregate_roster() {
    assert!(SOURCE.contains(
        "unsafe impl WorkerV3ProtectedRosterVerifierBackendV1<\
M1AllKernelsWorkerV3RosterV1>"
    ));
    assert!(
        SOURCE.contains("WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>")
    );
    assert!(SOURCE.contains("M1AllKernelsWorkerV3RosterV1::ENTRIES.len()"));
}

#[test]
fn pending_projection_is_private_and_typed_request_only() {
    assert!(SOURCE.contains("struct M1AllKernelsPendingRequestProjectionV1 {"));
    assert!(SOURCE.contains("struct M1AllKernelsPendingEntryProjectionV1 {"));
    assert!(SOURCE.contains("struct M1AllKernelsPendingDescriptorProjectionV1 {"));
    assert!(SOURCE.contains("struct M1AllKernelsPendingDescriptorBindingProjectionV1 {"));
    assert!(SOURCE.contains("struct M1AllKernelsPendingPhysicalKernelProjectionV1 {"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingRequestProjectionV1"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingEntryProjectionV1"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingDescriptorProjectionV1"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingDescriptorBindingProjectionV1"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingPhysicalKernelProjectionV1"));
    assert!(SOURCE.contains(
        "fn from_request(\n        request: &WorkerV3RosterVerificationRequestV1<'_, \
M1AllKernelsWorkerV3RosterV1>,"
    ));
    assert!(SOURCE.contains(
        "fn entry_from_request(\n        request: &WorkerV3RosterVerificationRequestV1<'_, \
M1AllKernelsWorkerV3RosterV1>,"
    ));
    for forbidden in [
        "from_untrusted",
        "from_parts",
        "from_json",
        "serialize",
        "deserialize",
        ".expect(",
        ".unwrap(",
        "panic!(",
        "unreachable!(",
        "unimplemented!(",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "production source contains forbidden parallel input surface {forbidden}"
        );
    }
}

#[test]
fn pending_projection_covers_every_request_identity_axis() {
    for getter in [
        "challenge_identity()",
        "roster_identity()",
        "lineage_identity()",
        "finalizer_derivation_sha256()",
        "compiler_execution_subject_sha256()",
        "compiler_execution_carriage_sha256()",
        "compiler_execution_policy_sha256()",
        "compiler_execution_issuer_journal_sha256()",
        "compiler_occurrence_sha256()",
        "compiler_execution_receipt_sha256()",
        "compiler_execution_publication_sha256()",
        "compiler_execution_acknowledgment_sha256()",
        "compiler_execution_worker_ledger_record_sha256()",
        "compiler_execution_sequence()",
        "compiler_execution_prior_rollback_anchor()",
        "compiler_execution_current_rollback_anchor()",
        "capsule_sha256()",
        "formal_memory_receipt_sha256()",
        "proof_binding_receipt_sha256()",
        "finalized_hsaco_sha256()",
        "finalized_hsaco_length()",
        "target()",
        "code_object_version()",
    ] {
        assert!(
            SOURCE.contains(getter),
            "pending request projection omits {getter}"
        );
    }
}

#[test]
fn pending_projection_has_exactly_twelve_ordered_complete_entry_rows() {
    assert!(SOURCE.contains("const M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1: usize = 12;"));
    assert!(SOURCE.contains("const _: [(); M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1]"));
    assert!(SOURCE.contains("[(); M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1];"));
    assert!(SOURCE.contains("entries: [M1AllKernelsPendingEntryProjectionV1;"));
    assert!(SOURCE.contains("M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1],"));
    assert!(SOURCE.contains("request\n            .marker_entries()\n            .iter()"));
    assert!(SOURCE.contains(".map(|(ordinal, marker)| Self::entry_from_request("));
    assert!(SOURCE.contains(".collect::<Vec<_>>()\n            .try_into()\n            .ok()?;"));
    assert!(SOURCE.contains(") -> Option<Self> {"));
    assert!(!SOURCE.contains("marker_entries[ordinal]"));
    assert!(SOURCE.contains(".entry_lineage_identity(ordinal)"));
    for field in [
        "ordinal,",
        "logical_name: marker.logical_name()",
        "export_name: marker.export_name()",
        "marker_binding_identity: marker.kernel_binding_id()",
        "generated_host_contract_identity: marker.generated_host_contract_identity()",
        ".map(|identity| *identity.as_bytes())",
        "lineage_identity: lineage",
        "descriptor,",
        "descriptor_binding,",
        "physical_kernel,",
    ] {
        assert!(
            SOURCE.contains(field),
            "ordered entry projection omits {field}"
        );
    }
}

#[test]
fn every_entry_projects_typed_descriptor_binding_and_physical_facts() {
    for getter in [
        "request.descriptor(ordinal)",
        "request.descriptor_binding(ordinal)",
        "request.physical_kernel(ordinal)",
    ] {
        assert!(SOURCE.contains(getter), "entry projection omits {getter}");
    }
    for field in [
        "kernel_id: *descriptor.kernel_id().as_bytes()",
        "logical_name: descriptor.logical_name().as_str().to_owned()",
        "entry_name: descriptor.entry_name().as_str().to_owned()",
        "descriptor_symbol: descriptor.descriptor_symbol().as_str().to_owned()",
        "source_evidence_identity:",
        "source_evidence_digest:",
        "executable_ir_evidence_identity:",
        "executable_ir_evidence_digest:",
        "explicit_argument_size:",
        "kernarg_segment_size:",
        "kernarg_segment_alignment:",
        "launch_block_size: descriptor.launch().block_size()",
        "capability_count:",
        "logical_argument_count:",
        "kernel_index: binding.kernel_index()",
        "descriptor_address: binding.descriptor_address()",
        "descriptor_file_offset: binding.descriptor_file_offset()",
        "entry_address: binding.entry_address()",
        "entry_file_offset: binding.entry_file_offset()",
        "entry_size: binding.entry_size()",
        "kernel_code_entry_byte_offset:",
        "compute_pgm_rsrc3:",
        "compute_pgm_rsrc1:",
        "compute_pgm_rsrc2:",
        "kernel_code_properties:",
        "kernarg_preload: descriptor.kernarg_preload()",
        "name: physical.name().to_owned()",
        "symbol: physical.symbol().to_owned()",
        "group_segment_fixed_size:",
        "private_segment_fixed_size:",
        "wavefront_size:",
        "sgpr_count:",
        "vgpr_count:",
        "agpr_count:",
        "sgpr_spill_count:",
        "vgpr_spill_count:",
        "max_flat_workgroup_size:",
        "required_workgroup_size:",
        "max_workgroups:",
        "cluster_dims:",
        "uniform_work_group_size:",
        "uses_dynamic_stack:",
        "workgroup_processor_mode:",
        "implicit_argument_offset:",
        "implicit_argument_size:",
        "explicit_argument_count:",
        "hidden_argument_count:",
    ] {
        assert!(
            SOURCE.contains(field),
            "typed descriptor/physical projection omits {field}"
        );
    }
    for optional in [
        "descriptor: Option<M1AllKernelsPendingDescriptorProjectionV1>",
        "descriptor_binding: Option<M1AllKernelsPendingDescriptorBindingProjectionV1>",
        "physical_kernel: Option<M1AllKernelsPendingPhysicalKernelProjectionV1>",
    ] {
        assert!(
            SOURCE.contains(optional),
            "typed row absence is not explicit for {optional}"
        );
    }
}

#[test]
fn typed_roster_fixes_the_exact_twelve_projection_rows() {
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
    assert!(M1AllKernelsWorkerV3RosterV1::ENTRIES.iter().all(|entry| {
        entry.kernel_binding_id() != [0; 32] && entry.generated_host_contract_identity() != [0; 32]
    }));
}

#[test]
#[allow(clippy::too_many_lines)]
fn production_backend_projects_then_preflights_in_exact_custody_order() {
    assert!(
        protected_verifier_policy_accepts(SOURCE),
        "canonical production source must satisfy the reusable verifier policy"
    );
    let code = rust_code_without_comments_or_strings(SOURCE);
    let (production, _tests) = split_complete_production_and_tests(&code)
        .expect("one balanced cfg(test) module must consume the complete source suffix");
    let compact = compact_rust_code(production);
    let ranges = protected_boundary_ranges(&compact).expect(
        "one exact inherent verifier impl and one exact protected-backend impl must own the boundary items",
    );
    assert!(
        has_unique_private_boundary_items(production),
        "production verifier method and private rejection helper must each belong only to their exact impl"
    );
    assert!(
        has_no_production_macro_authority(&code),
        "production verifier source must not define or invoke macros or unknown attributes"
    );
    let method = &compact[ranges.verify_method];
    let method_body = method
        .split_once('{')
        .expect("production verifier method must have a body")
        .1
        .strip_suffix('}')
        .expect("production verifier method must terminate");
    let expected_method_body = r#"
        let pending_request = M1AllKernelsPendingRequestProjectionV1::from_request(request)
            .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;
        let finalized_verification =
            ::fe2o3_hsaco_finalize::verify_finalized(request.finalized_hsaco_bytes());
        let hsaco_reinspection = finalized_verification.map_err(|_| {
            M1AllKernelsProtectedVerifierErrorV1::ExactFinalizedHsacoVerificationFailed
        })?;
        let finalizer_owner = request
            .independently_revalidate_finalizer_derivation()
            .map_err(|_| {
                M1AllKernelsProtectedVerifierErrorV1::FinalizerDerivationRevalidationFailed
            })?;
        let proof_inputs_owner = request
            .validate_compiler_multi_root_proof_inputs_v1()
            .map_err(|_| {
                M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootProofInputsValidationFailed
            })?;
        let target_lineage_owner = request
            .validate_compiler_multi_root_target_lineage_v1(&proof_inputs_owner)
            .map_err(|_| {
                M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootTargetLineageValidationFailed
            })?;
        {
            let validate_associations = || -> Result<(), Self::Error> {
                let finalizer_identity = finalizer_owner.identity();
                let finalized_hsaco = finalizer_owner.finalized_hsaco_identity();
                (finalizer_identity.as_bytes() == &pending_request.finalizer_derivation_sha256
                    && finalized_hsaco.sha256() == &pending_request.finalized_hsaco_sha256
                    && finalized_hsaco.byte_len() == pending_request.finalized_hsaco_length)
                    .then_some(())
                    .ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::FinalizerArtifactAssociationFailed,
                    )?;

                let final_llvm = target_lineage_owner.final_llvm_identity();
                let finalizer_module = finalizer_owner.compiler_module_identity();
                let semantic_module = request
                    .semantic_compiler_handoff()
                    .module_handoff()
                    .module_identity();
                let target_binding = target_lineage_owner.target_binding();
                (final_llvm.sha256() == *finalizer_module.sha256()
                    && final_llvm.byte_len() == finalizer_module.byte_len()
                    && final_llvm.sha256() == *semantic_module.sha256()
                    && final_llvm.byte_len() == semantic_module.byte_len()
                    && target_binding.configured_target() == pending_request.target
                    && target_binding.code_object_version()
                        == u16::from(pending_request.code_object_version)
                    && pending_request.target == "gfx942:xnack-"
                    && pending_request.code_object_version == 6)
                    .then_some(())
                    .ok_or(M1AllKernelsProtectedVerifierErrorV1::CompilerTargetAssociationFailed)?;

                let markers = request.marker_entries();
                let proof_roots = proof_inputs_owner.roots();
                let reinspected = hsaco_reinspection.hsaco();
                let reinspected_kernels = reinspected.kernels();
                let reinspected_bindings = hsaco_reinspection.kernel_bindings().bindings();
                (markers.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
                    && proof_roots.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
                    && target_binding.root_count() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
                    && hsaco_reinspection.descriptor_table() == request.descriptor_table()
                    && reinspected.target() == request.target()
                    && reinspected.code_object_version().number()
                        == request.code_object_version().number()
                    && reinspected_kernels.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
                    && reinspected_bindings.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1
                    && hsaco_reinspection.kernel_bindings().load_layout().is_some())
                    .then_some(())
                    .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;

                (markers.iter().all(|marker| {
                    proof_roots
                        .iter()
                        .filter(|root| root.kernel_binding() == &marker.kernel_binding_id())
                        .count()
                        == 1
                }) && proof_roots.iter().all(|root| {
                    markers
                        .iter()
                        .filter(|marker| marker.kernel_binding_id() == *root.kernel_binding())
                        .count()
                        == 1
                }))
                .then_some(())
                .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;

                let mut matched_reinspected_kernels =
                    [false; M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1];
                pending_request.entries.iter().try_for_each(|entry| {
                    let (root_index, root) = proof_roots
                        .iter()
                        .enumerate()
                        .find(|(_, root)| root.kernel_binding() == &entry.marker_binding_identity)
                        .ok_or(
                            M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                        )?;
                    let _lineage = entry.lineage_identity.as_ref().ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                    )?;
                    let descriptor = entry.descriptor.as_ref().ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                    )?;
                    let descriptor_binding = entry.descriptor_binding.as_ref().ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                    )?;
                    let physical = entry.physical_kernel.as_ref().ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                    )?;
                    let mut matching_reinspected_kernels =
                        reinspected_kernels.iter().enumerate().filter(|(_, kernel)| {
                            kernel.name() == entry.export_name
                                && kernel.symbol() == descriptor.descriptor_symbol
                        });
                    let (reinspected_index, reinspected_physical) =
                        matching_reinspected_kernels.next().ok_or(
                            M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                        )?;
                    matching_reinspected_kernels
                        .next()
                        .is_none()
                        .then_some(())
                        .ok_or(
                            M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                        )?;
                    let matched = matched_reinspected_kernels
                        .get_mut(reinspected_index)
                        .ok_or(
                            M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                        )?;
                    (!*matched).then_some(()).ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                    )?;
                    *matched = true;
                    let reinspected_binding = reinspected_bindings.get(reinspected_index).ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                    )?;
                    let request_physical = request.physical_kernel(entry.ordinal).ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                    )?;
                    let request_binding = request.descriptor_binding(entry.ordinal).ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                    )?;
                    let target_workgroup = target_binding.workgroup(root_index).ok_or(
                        M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed,
                    )?;
                    let descriptor_workgroup = match descriptor.launch_block_size {
                        BlockSizeV1::Exact(dimensions) => {
                            Some([dimensions.x(), dimensions.y(), dimensions.z()])
                        }
                        BlockSizeV1::Any | BlockSizeV1::AtMost(_) => None,
                    }
                    .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)?;

                    (entry.logical_name == root.logical_name()
                        && entry.export_name == root.export_symbol()
                        && descriptor.kernel_id == entry.marker_binding_identity
                        && descriptor.kernel_id == *root.kernel_binding()
                        && descriptor.logical_name == root.logical_name()
                        && descriptor.entry_name == root.export_symbol()
                        && physical.name == root.export_symbol()
                        && physical.symbol == descriptor.descriptor_symbol
                        && reinspected_physical == request_physical
                        && *reinspected_binding == request_binding
                        && reinspected_binding.kernel_index() == reinspected_index
                        && request_binding.kernel_index() == reinspected_index
                        && descriptor_binding.kernel_index == reinspected_index
                        && target_workgroup.kernel() == root.kernel_id()
                        && target_workgroup.workgroup() == root.workgroup()
                        && descriptor_workgroup == root.workgroup())
                    .then_some(())
                    .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)
                })?;
                matched_reinspected_kernels
                    .iter()
                    .all(|matched| *matched)
                    .then_some(())
                    .ok_or(M1AllKernelsProtectedVerifierErrorV1::RosterEntryAssociationFailed)
            };
            validate_associations()?;
        }
        Self::reject_missing_protected_receipt(
            &pending_request,
            finalizer_owner,
            proof_inputs_owner,
            target_lineage_owner,
            hsaco_reinspection,
        )
    "#;
    let expected_method_body =
        compact_rust_code(&rust_code_without_comments_or_strings(expected_method_body));
    assert_eq!(
        method_body, expected_method_body,
        "production method must project first, run only the exact ordered preflight, and move the unshadowed owners into rejection"
    );

    let rejection_helper = &compact[ranges.rejection_helper];
    let expected_rejection_helper = r"
        <FinalizerOwner, ProofInputsOwner, TargetLineageOwner, HsacoReinspectionOwner,>(
            _request: &M1AllKernelsPendingRequestProjectionV1,
            _finalizer_owner: FinalizerOwner,
            _proof_inputs_owner: ProofInputsOwner,
            _target_lineage_owner: TargetLineageOwner,
            _hsaco_reinspection_owner: HsacoReinspectionOwner,
        ) -> Result<
            WorkerV3ProtectedRosterVerificationEvidenceV1,
            M1AllKernelsProtectedVerifierErrorV1
        > {
            Err(missing_protected_verification_receipt_v1())
    ";
    assert_eq!(
        rejection_helper
            .strip_prefix(COMPACT_REJECTION_HELPER_NAME)
            .expect("production rejection helper must have its exact private name")
            .strip_suffix('}')
            .expect("production rejection helper must terminate"),
        compact_rust_code(expected_rejection_helper),
        "private rejection helper must consume every inferred owner and remain unconditionally fail-closed"
    );
    assert_eq!(SOURCE.matches("Err(").count(), 1);
    assert_eq!(SOURCE.matches("Ok(").count(), 0);
    assert_eq!(
        method
            .matches("::fe2o3_hsaco_finalize::verify_finalized(request.finalized_hsaco_bytes())")
            .count(),
        1,
        "the exact borrowed finalized bytes must be verified exactly once"
    );
    assert_eq!(
        method
            .matches("letvalidate_associations=||->Result<(),Self::Error>{")
            .count(),
        1
    );
    assert!(method.contains("};validate_associations()?;}Self::reject_missing_protected_receipt("));
    assert!(!method.contains("proof_roots["));
    assert!(!method.contains("markers["));
    assert!(!method.contains("reinspected_kernels["));
    assert!(!method.contains("reinspected_bindings["));
    for owner_type in [
        "RevalidatedProtectedWorkerV3FinalizerDerivationV1",
        "ValidatedCompilerMultiRootProofInputsV1",
        "ValidatedCompilerMultiRootTargetLineageV1",
    ] {
        assert!(
            !method.contains(owner_type),
            "association closure must infer the move-only owner type {owner_type}"
        );
    }
}

#[test]
fn exact_target_literal_is_bound_inside_the_anchored_production_method() {
    let canonical = protected_verify_method(SOURCE).expect("canonical verifier method must parse");
    assert_eq!(canonical.matches(REQUIRED_GFX942_TARGET_LITERAL).count(), 1);

    let changed_real_target = SOURCE.replacen(
        "pending_request.target == \"gfx942:xnack-\"",
        "pending_request.target == \"gfx942:xnack+\"",
        1,
    );
    let comment_decoy = changed_real_target.replacen(
        "unsafe impl WorkerV3ProtectedRosterVerifierBackendV1",
        "// pending_request.target == \"gfx942:xnack-\"\nunsafe impl WorkerV3ProtectedRosterVerifierBackendV1",
        1,
    );
    let string_decoy = changed_real_target.replacen(
        TEST_MODULE_BOUNDARY,
        "const TARGET_LITERAL_DECOY: &str = \"gfx942:xnack-\";\n\n#[cfg(test)]\nmod tests {",
        1,
    );
    let in_method_string_decoy = changed_real_target.replacen(
        "let markers = request.marker_entries();",
        "let _target_literal_decoy = \"gfx942:xnack-\";\n                let markers = request.marker_entries();",
        1,
    );

    for hostile in [
        changed_real_target,
        comment_decoy,
        string_decoy,
        in_method_string_decoy,
    ] {
        assert!(
            !protected_verifier_policy_accepts(&hostile),
            "a target decoy must not satisfy the anchored verifier method policy"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn association_preflight_shape_rejects_each_critical_hostile_mutation() {
    let code = rust_code_without_comments_or_strings(SOURCE);
    let (production, _tests) = split_complete_production_and_tests(&code)
        .expect("one balanced cfg(test) module must consume the complete source suffix");
    let compact = compact_rust_code(production);
    let ranges = protected_boundary_ranges(&compact).expect("canonical boundary must parse");
    let canonical = &compact[ranges.verify_method];
    assert!(anchored_verifier_method_satisfies_policy(canonical));

    for (needle, replacement) in [
        (
            "finalizer_identity.as_bytes()==&pending_request.finalizer_derivation_sha256",
            "true",
        ),
        (
            "finalized_hsaco.sha256()==&pending_request.finalized_hsaco_sha256",
            "true",
        ),
        (
            "finalized_hsaco.byte_len()==pending_request.finalized_hsaco_length",
            "true",
        ),
        ("final_llvm.sha256()==*finalizer_module.sha256()", "true"),
        ("final_llvm.byte_len()==finalizer_module.byte_len()", "true"),
        ("final_llvm.sha256()==*semantic_module.sha256()", "true"),
        ("final_llvm.byte_len()==semantic_module.byte_len()", "true"),
        (
            "target_binding.configured_target()==pending_request.target",
            "true",
        ),
        (
            "target_binding.code_object_version()==u16::from(pending_request.code_object_version)",
            "true",
        ),
        (
            "hsaco_reinspection.descriptor_table()==request.descriptor_table()",
            "true",
        ),
        ("reinspected.target()==request.target()", "true"),
        (
            "reinspected.code_object_version().number()==request.code_object_version().number()",
            "true",
        ),
        (
            "reinspected_kernels.len()==M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1",
            "true",
        ),
        (
            "reinspected_bindings.len()==M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1",
            "true",
        ),
        (
            "hsaco_reinspection.kernel_bindings().load_layout().is_some()",
            "true",
        ),
        ("pending_request.target==", "true=="),
        ("pending_request.code_object_version==6", "true"),
        (
            "markers.len()==M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1",
            "true",
        ),
        (
            "proof_roots.len()==M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1",
            "true",
        ),
        (
            "target_binding.root_count()==M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1",
            "true",
        ),
        ("root.kernel_binding()==&marker.kernel_binding_id()", "true"),
        ("marker.kernel_binding_id()==*root.kernel_binding()", "true"),
        (".count()==1", ".count()>0"),
        (
            "entry.lineage_identity.as_ref().ok_or(",
            "Some(&[0_u8;32]).ok_or(",
        ),
        (
            "entry.descriptor.as_ref().ok_or(",
            "None::<&M1AllKernelsPendingDescriptorProjectionV1>.ok_or(",
        ),
        (
            "entry.descriptor_binding.as_ref().ok_or(",
            "None::<&M1AllKernelsPendingDescriptorBindingProjectionV1>.ok_or(",
        ),
        (
            "entry.physical_kernel.as_ref().ok_or(",
            "None::<&M1AllKernelsPendingPhysicalKernelProjectionV1>.ok_or(",
        ),
        ("kernel.name()==entry.export_name", "true"),
        ("kernel.symbol()==descriptor.descriptor_symbol", "true"),
        ("matching_reinspected_kernels.next().is_none()", "true"),
        (
            "matched_reinspected_kernels.get_mut(reinspected_index)",
            "matched_reinspected_kernels.get_mut(entry.ordinal)",
        ),
        ("(!*matched).then_some(())", "true.then_some(())"),
        (
            "reinspected_bindings.get(reinspected_index)",
            "reinspected_bindings.get(entry.ordinal)",
        ),
        (
            "request.physical_kernel(entry.ordinal).ok_or(",
            "request.physical_kernel(reinspected_index).ok_or(",
        ),
        (
            "request.descriptor_binding(entry.ordinal).ok_or(",
            "request.descriptor_binding(reinspected_index).ok_or(",
        ),
        (
            "target_binding.workgroup(root_index).ok_or(",
            "target_binding.workgroup(entry.ordinal).ok_or(",
        ),
        (
            "BlockSizeV1::Exact(dimensions)",
            "BlockSizeV1::AtMost(dimensions)",
        ),
        (
            "BlockSizeV1::Any|BlockSizeV1::AtMost(_)=>None",
            "BlockSizeV1::Any=>None,BlockSizeV1::AtMost(dimensions)=>Some([dimensions.x(),dimensions.y(),dimensions.z()])",
        ),
        ("entry.logical_name==root.logical_name()", "true"),
        ("entry.export_name==root.export_symbol()", "true"),
        (
            "descriptor.kernel_id==entry.marker_binding_identity",
            "true",
        ),
        ("descriptor.kernel_id==*root.kernel_binding()", "true"),
        ("descriptor.logical_name==root.logical_name()", "true"),
        ("descriptor.entry_name==root.export_symbol()", "true"),
        ("physical.name==root.export_symbol()", "true"),
        ("physical.symbol==descriptor.descriptor_symbol", "true"),
        ("reinspected_physical==request_physical", "true"),
        ("*reinspected_binding==request_binding", "true"),
        (
            "reinspected_binding.kernel_index()==reinspected_index",
            "true",
        ),
        ("request_binding.kernel_index()==reinspected_index", "true"),
        ("descriptor_binding.kernel_index==reinspected_index", "true"),
        ("target_workgroup.kernel()==root.kernel_id()", "true"),
        ("target_workgroup.workgroup()==root.workgroup()", "true"),
        ("descriptor_workgroup==root.workgroup()", "true"),
        (
            "matched_reinspected_kernels.iter().all(|matched|*matched)",
            "true",
        ),
    ] {
        assert!(
            canonical.contains(needle),
            "missing hostile fixture token {needle}"
        );
        let hostile = canonical.replacen(needle, replacement, 1);
        assert!(
            !anchored_verifier_method_satisfies_policy(&hostile),
            "anchored verifier policy accepted hostile mutation for {needle}"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn exact_finalized_verifier_policy_rejects_every_call_and_owner_evasion() {
    assert!(protected_verifier_policy_accepts(SOURCE));
    let exact_call = "::fe2o3_hsaco_finalize::verify_finalized(request.finalized_hsaco_bytes())";
    assert_eq!(SOURCE.matches(exact_call).count(), 1);
    let masked_production = rust_code_without_comments_or_strings(SOURCE);
    let (masked_production, _tests) = split_complete_production_and_tests(&masked_production)
        .expect("canonical production and tests must split");
    assert_eq!(
        identifier_token_count(masked_production, FINALIZER_CRATE_IDENTIFIER),
        1
    );
    assert_eq!(
        SOURCE
            .matches("inspect_and_bind_kernel_descriptors")
            .count(),
        0
    );

    let unqualified = SOURCE.replacen(
        exact_call,
        "verify_finalized(request.finalized_hsaco_bytes())",
        1,
    );
    let wrapper = unqualified.replacen(
        TEST_MODULE_BOUNDARY,
        r"fn verify_finalized(
    _bytes: &[u8],
) -> Result<
    ::fe2o3_hsaco_finalize::FinalizedDescriptorInspection,
    ::fe2o3_hsaco_finalize::FinalizationError,
> {
    ::fe2o3_hsaco_finalize::verify_finalized(&[])
}

#[cfg(test)]
mod tests {",
        1,
    );
    let wrong_bytes = SOURCE.replacen("request.finalized_hsaco_bytes()", "&[]", 1);
    let omitted_call = SOURCE.replacen(exact_call, "omitted_finalized_verification()", 1);
    let duplicate_call = SOURCE.replacen(
        "        let pending_request =",
        r"        let _discarded_reinspection =
            ::fe2o3_hsaco_finalize::verify_finalized(request.finalized_hsaco_bytes());
        let pending_request =",
        1,
    );
    let second_parser = SOURCE.replacen(
        "        let entries = request",
        r"        let _second_parse =
            ::fe2o3_hsaco::inspect_and_bind_kernel_descriptors(
                request.finalized_hsaco_bytes(),
            );
        let entries = request",
        1,
    );
    let sibling_finalizer_api = SOURCE.replacen(
        "        let entries = request",
        r"        let _sibling_inspection =
            ::fe2o3_hsaco_finalize::inspect_finalized(request.finalized_hsaco_bytes());
        let entries = request",
        1,
    );
    let crate_alias = SOURCE
        .replacen(
            "use std::fmt;",
            "use fe2o3_hsaco_finalize as hsaco_finalize_again;\nuse std::fmt;",
            1,
        )
        .replacen(
            "        let entries = request",
            r"        let _sibling_inspection =
            hsaco_finalize_again::inspect_finalized(request.finalized_hsaco_bytes());
        let entries = request",
            1,
        );
    let function_alias = SOURCE.replacen(
        "use std::fmt;",
        "use fe2o3_hsaco_finalize::inspect_finalized as inspect_finalized_again;\nuse std::fmt;",
        1,
    )
    .replacen(
        "        let entries = request",
        r"        let _sibling_inspection =
            inspect_finalized_again(request.finalized_hsaco_bytes());
        let entries = request",
        1,
    );
    let raw_identifier_crate_path = SOURCE.replacen(
        "        let entries = request",
        r"        let _sibling_inspection =
            ::r#fe2o3_hsaco_finalize::inspect_finalized(request.finalized_hsaco_bytes());
        let entries = request",
        1,
    );
    let parenthesized_callee = SOURCE.replacen(
        "        let entries = request",
        r"        let _discarded_reinspection =
            (::fe2o3_hsaco_finalize::verify_finalized)(
                request.finalized_hsaco_bytes(),
            );
        let entries = request",
        1,
    );
    let function_item_alias = SOURCE.replacen(
        "        let entries = request",
        r"        let verifier = ::fe2o3_hsaco_finalize::verify_finalized;
        let _discarded_reinspection = verifier(request.finalized_hsaco_bytes());
        let entries = request",
        1,
    );
    let ignored_result = SOURCE.replacen(
        "        let hsaco_reinspection =",
        "        let _ignored_hsaco_reinspection =",
        1,
    );
    let weakened_descriptor_table = SOURCE.replacen(
        "                    && hsaco_reinspection.descriptor_table() == request.descriptor_table()",
        "                    && true",
        1,
    );
    let dropped_owner = SOURCE.replacen(
        "            hsaco_reinspection,\n        )",
        "            (),\n        )",
        1,
    );

    for (case, hostile) in [
        ("unqualified call", unqualified),
        ("private wrapper", wrapper),
        ("wrong bytes", wrong_bytes),
        ("omitted call", omitted_call),
        ("duplicate direct call", duplicate_call),
        ("second low-level parser", second_parser),
        ("sibling finalizer API", sibling_finalizer_api),
        ("finalizer crate alias", crate_alias),
        ("finalizer function alias", function_alias),
        (
            "raw-identifier finalizer crate path",
            raw_identifier_crate_path,
        ),
        ("parenthesized callee", parenthesized_callee),
        ("function-item alias", function_item_alias),
        ("ignored result", ignored_result),
        (
            "weakened descriptor-table equality",
            weakened_descriptor_table,
        ),
        ("dropped finalization-verification owner", dropped_owner),
    ] {
        assert_ne!(
            hostile, SOURCE,
            "hostile fixture was not constructed: {case}"
        );
        assert!(
            !protected_verifier_policy_accepts(&hostile),
            "exact finalized-verifier policy accepted hostile substitution: {case}"
        );
    }
}

#[test]
fn association_failures_are_exactly_four_coarse_variants() {
    for variant in [
        "ExactFinalizedHsacoVerificationFailed",
        "FinalizerArtifactAssociationFailed",
        "CompilerTargetAssociationFailed",
        "RosterEntryAssociationFailed",
    ] {
        assert_eq!(
            SOURCE.matches(&format!("    {variant},")).count(),
            1,
            "association error variant must have one enum declaration"
        );
    }
    for forbidden in [
        "FinalizerIdentityMismatch",
        "FinalizedHsacoMismatch",
        "TargetMismatch",
        "CodeObjectVersionMismatch",
        "MissingDescriptor",
        "DuplicateBinding",
        "LaunchBlockMismatch",
    ] {
        assert!(!SOURCE.contains(forbidden));
    }
}

#[test]
fn boundary_item_extraction_rejects_comment_decoys_and_exposed_helpers() {
    let missing_real_method = SOURCE.replacen(
        PRODUCTION_VERIFY_METHOD_ITEM,
        "unsafe fn bypass_protected_roster",
        1,
    );
    let commented_method_decoy = format!(
        "/* {PRODUCTION_VERIFY_METHOD_ITEM}() {{ canonical decoy }} */\n{missing_real_method}"
    );
    assert!(!has_unique_private_boundary_items(
        &rust_code_without_comments_or_strings(&commented_method_decoy)
    ));

    for exposed in [
        SOURCE.replacen(
            "    fn reject_missing_protected_receipt",
            "    pub fn reject_missing_protected_receipt",
            1,
        ),
        SOURCE.replacen(
            "    fn reject_missing_protected_receipt",
            "    pub\n    fn reject_missing_protected_receipt",
            1,
        ),
        SOURCE.replacen(
            "    fn reject_missing_protected_receipt",
            "    #[inline]\n    fn reject_missing_protected_receipt",
            1,
        ),
    ] {
        assert!(!has_unique_private_boundary_items(
            &rust_code_without_comments_or_strings(&exposed)
        ));
    }
}

#[test]
fn boundary_items_cannot_be_satisfied_by_wrong_impl_decoys() {
    let masked_source = rust_code_without_comments_or_strings(SOURCE);
    let method_start = unique_occurrence(&masked_source, PRODUCTION_VERIFY_METHOD_ITEM)
        .expect("fixture source has one canonical verifier method");
    let method_range = balanced_braced_item_range(&masked_source, method_start, SOURCE.len())
        .expect("fixture verifier method is balanced");
    let canonical_method = &SOURCE[method_range];
    let bypassing_trait_method = r"unsafe
    fn verify_protected_roster(
        &mut self,
        request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    ) -> Result<WorkerV3ProtectedRosterVerificationEvidenceV1, Self::Error> {
        let pending_request = M1AllKernelsPendingRequestProjectionV1::from_request(request);
        Self::reject_missing_protected_receipt(&pending_request, (), (), ())
    }";
    let split_trait_method = SOURCE.replacen(canonical_method, bypassing_trait_method, 1);
    let inherent_method_decoy = split_trait_method.replacen(
        "\n}\n\nconst fn missing_protected_verification_receipt_v1",
        &format!(
            "\n\n{canonical_method}\n}}\n\nconst fn missing_protected_verification_receipt_v1"
        ),
        1,
    );
    let inherent_method_code = rust_code_without_comments_or_strings(&inherent_method_decoy);
    let (inherent_method_production, _tests) =
        split_complete_production_and_tests(&inherent_method_code)
            .expect("inherent-method fixture keeps the complete test boundary");
    assert!(!has_unique_private_boundary_items(
        inherent_method_production
    ));

    let helper_start = unique_occurrence(&masked_source, PRODUCTION_REJECTION_HELPER_ITEM)
        .expect("fixture source has one canonical rejection helper");
    let helper_range = balanced_braced_item_range(&masked_source, helper_start, SOURCE.len())
        .expect("fixture rejection helper is balanced");
    let canonical_helper = &SOURCE[helper_range];
    let split_real_helper = SOURCE.replacen(
        PRODUCTION_REJECTION_HELPER_ITEM,
        "pub fn\n    reject_missing_protected_receipt",
        1,
    );
    let decoy_type_helper = split_real_helper.replacen(
        "impl M1AllKernelsProtectedVerifierV1 {",
        &format!(
            "struct ProtectedVerifierHelperDecoy;\n\n\
             impl ProtectedVerifierHelperDecoy {{\n\
                 pub const fn new() -> Self {{\n\
                     Self\n\
                 }}\n\n\
             {canonical_helper}\n\
             }}\n\n\
             impl M1AllKernelsProtectedVerifierV1 {{"
        ),
        1,
    );
    let decoy_helper_code = rust_code_without_comments_or_strings(&decoy_type_helper);
    let (decoy_helper_production, _tests) = split_complete_production_and_tests(&decoy_helper_code)
        .expect("decoy-helper fixture keeps the complete test boundary");
    assert!(!has_unique_private_boundary_items(decoy_helper_production));
}

#[test]
fn boundary_method_must_be_a_direct_child_and_cannot_use_a_raw_name() {
    let masked_source = rust_code_without_comments_or_strings(SOURCE);
    let method_start = unique_occurrence(&masked_source, PRODUCTION_VERIFY_METHOD_ITEM)
        .expect("fixture source has one canonical verifier method");
    let method_range = balanced_braced_item_range(&masked_source, method_start, SOURCE.len())
        .expect("fixture verifier method is balanced");
    let canonical_method = &SOURCE[method_range];
    let nested_local_decoy = format!(
        r"unsafe fn r#verify_protected_roster(
        &mut self,
        request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    ) -> Result<WorkerV3ProtectedRosterVerificationEvidenceV1, Self::Error> {{
        type LocalVerifierDecoy = M1AllKernelsProtectedVerifierV1;
        impl LocalVerifierDecoy {{
            {canonical_method}
        }}
        let pending_request = M1AllKernelsPendingRequestProjectionV1::from_request(request);
        Self::reject_missing_protected_receipt(&pending_request, (), (), ())
    }}"
    );
    let hostile = SOURCE.replacen(canonical_method, &nested_local_decoy, 1);
    let hostile_code = rust_code_without_comments_or_strings(&hostile);
    let (hostile_production, _tests) = split_complete_production_and_tests(&hostile_code)
        .expect("nested local-decoy fixture keeps the complete test boundary");
    let compact = compact_rust_code(hostile_production);
    let backend_impl = unique_balanced_impl_range(&compact, COMPACT_BACKEND_IMPL_HEADER)
        .expect("fixture retains one exact target backend impl");
    let nested_start = unique_occurrence(&compact, COMPACT_VERIFY_METHOD_ITEM)
        .expect("fixture retains one canonical nested verifier method");
    assert!(
        brace_depth_at(&compact, &backend_impl, nested_start).is_some_and(|depth| depth > 1),
        "canonical decoy must be nested below the target impl's direct-child depth"
    );
    assert!(
        item_range_within(&compact, COMPACT_VERIFY_METHOD_ITEM, &backend_impl).is_none(),
        "a nested local decoy must not satisfy the target backend method"
    );
    assert!(compact.contains("r#"));
    assert!(protected_boundary_ranges(&compact).is_none());
}

#[test]
fn production_boundary_rejects_items_inside_tests_and_post_test_tail() {
    let relocated_items = format!(
        "{TEST_MODULE_BOUNDARY}\n{PRIVATE_REJECTION_HELPER_LEAD_IN}\n{PRODUCTION_VERIFY_METHOD_ITEM}() {{}}\n}}"
    );
    let relocated_code = rust_code_without_comments_or_strings(&relocated_items);
    let (relocated_production, _tests) = split_complete_production_and_tests(&relocated_code)
        .expect("fixture test module is balanced");
    assert!(!has_unique_private_boundary_items(relocated_production));

    let post_test_macro_backend = format!(
        "{SOURCE}\nmacro_rules! emit_backend {{ ($name:ident) => {{ unsafe fn $name() {{}} }}; }}\nemit_backend!(verify_protected_roster);"
    );
    let post_test_code = rust_code_without_comments_or_strings(&post_test_macro_backend);
    assert!(split_complete_production_and_tests(&post_test_code).is_none());
    assert!(!has_no_production_macro_authority(&post_test_code));
}

#[test]
fn production_macro_policy_rejects_token_tree_decoy_and_macro_emitted_backend() {
    let macro_fixture = r"
macro_rules! canonical_method_decoy {
    () => {
        unsafe fn verify_protected_roster() {
            canonical_owner_custody_shape!();
        }
    };
}
macro_rules! emit_bypassing_backend {
    ($name:ident) => {
        unsafe impl WorkerV3ProtectedRosterVerifierBackendV1<
            M1AllKernelsWorkerV3RosterV1,
        > for M1AllKernelsProtectedVerifierV1 {
            unsafe fn $name(&mut self, request: &Request) -> Result<Evidence, Error> {
                let pending_request = Projection::from_request(request);
                Self::reject_missing_protected_receipt(&pending_request, (), (), ())
            }
        }
    };
}
emit_bypassing_backend!(verify_protected_roster);
";
    let hostile = SOURCE.replacen(
        "const M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1: usize = 12;",
        &format!("{macro_fixture}\nconst M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1: usize = 12;"),
        1,
    );
    assert!(!has_no_production_macro_authority(
        &rust_code_without_comments_or_strings(&hostile)
    ));
}

#[test]
fn every_common_custody_failure_is_distinct_and_fail_closed() {
    for mapping in [
        "finalized_verification.map_err(|_| {\n            M1AllKernelsProtectedVerifierErrorV1::ExactFinalizedHsacoVerificationFailed",
        "independently_revalidate_finalizer_derivation()\n            .map_err(|_| {\n                M1AllKernelsProtectedVerifierErrorV1::FinalizerDerivationRevalidationFailed",
        "validate_compiler_multi_root_proof_inputs_v1()\n            .map_err(|_| {\n                M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootProofInputsValidationFailed",
        "validate_compiler_multi_root_target_lineage_v1(&proof_inputs_owner)\n            .map_err(|_| {\n                M1AllKernelsProtectedVerifierErrorV1::CompilerMultiRootTargetLineageValidationFailed",
    ] {
        assert!(
            SOURCE.contains(mapping),
            "preflight failure lacks exact fail-closed mapping {mapping}"
        );
    }
    assert_eq!(
        SOURCE
            .matches("ExactFinalizedHsacoVerificationFailed")
            .count(),
        4
    );
    assert_eq!(
        SOURCE
            .matches("FinalizerDerivationRevalidationFailed")
            .count(),
        4
    );
    assert_eq!(
        SOURCE
            .matches("CompilerMultiRootProofInputsValidationFailed")
            .count(),
        4
    );
    assert_eq!(
        SOURCE
            .matches("CompilerMultiRootTargetLineageValidationFailed")
            .count(),
        4
    );
}

#[test]
fn no_synthetic_or_projected_identity_acceptance_surface_exists() {
    for forbidden in [
        "synthetic_for_test_only",
        "worker-v3-verifier-test-support",
        "Sha256",
        "use sha2",
        "sha2::",
        "sha2 =",
        "Digest",
        "verifier_measurement_sha256",
        "verification_transcript_sha256",
        "proof_executable_binding_sha256",
        "WorkerV3ProtectedRosterVerificationEvidenceV1::new",
        "WorkerV3ProtectedRosterEntryEvidenceV1::new",
        "WorkerV3ProtectedRosterEntryEvidenceV1",
        "AuthenticatedWorkerV3RosterV1",
        "AuthenticatedWorkerV3ExecutableV1",
        "ValidatedCompilerMultiRootProofInputsV1",
        "ValidatedCompilerMultiRootTargetLineageV1",
        "RevalidatedProtectedWorkerV3FinalizerDerivationV1",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "production source contains forbidden acceptance surface {forbidden}"
        );
    }
    for forbidden in [
        "synthetic_for_test_only",
        "worker-v3-verifier-test-support",
        "WorkerV3ProtectedRosterVerificationEvidenceV1::new",
        "WorkerV3ProtectedRosterEntryEvidenceV1::new",
        "WorkerV3ProtectedRosterEntryEvidenceV1",
        "AuthenticatedWorkerV3RosterV1",
        "AuthenticatedWorkerV3ExecutableV1",
        "ValidatedCompilerMultiRootProofInputsV1",
        "ValidatedCompilerMultiRootTargetLineageV1",
        "RevalidatedProtectedWorkerV3FinalizerDerivationV1",
    ] {
        assert!(
            !MANIFEST.contains(forbidden),
            "manifest contains forbidden acceptance surface {forbidden}"
        );
    }
}

#[test]
fn adapter_has_no_external_input_or_policy_authority_surface() {
    assert!(production_has_no_external_authority_surface(SOURCE));
    for forbidden in [
        "std::env",
        "std::fs",
        "std::net",
        "std::path",
        "std::process",
        "env!",
        "option_env!",
        "File::",
        "OpenOptions",
        "Path::",
        "PathBuf",
        "Command::",
        "TcpStream",
        "UnixStream",
        "serde",
        "serde_json",
        "json!",
        "clap",
        "argh",
        "lexopt",
        "fn main(",
        "policy_key",
        "policy_root",
        "trust_root",
        "root_key",
        "secret_key",
        "public_key",
        "keyring",
        "fe2o3-kfd",
        "fe2o3_kfd",
        "fe2o3-hsa-runtime",
        "fe2o3_hsa_runtime",
        "hip_runtime",
        "hip::",
        "ferric-engine",
        "ferric_engine",
        "launch(",
        ".load(",
        "::load(",
        "authorize_hsa_load",
    ] {
        assert!(
            !MANIFEST.contains(forbidden),
            "manifest contains {forbidden}"
        );
    }
}

fn production_has_no_external_authority_surface(source: &str) -> bool {
    let Some(mut production) = compact_production_code(source) else {
        return false;
    };
    let allowed_descriptor_launch = "descriptor.launch().block_size()";
    if production.matches(allowed_descriptor_launch).count() != 1 {
        return false;
    }
    production = production.replacen(allowed_descriptor_launch, "descriptor.block_size()", 1);

    if [
        "std::env",
        "std::fs",
        "std::net",
        "std::path",
        "std::process",
        "hip::",
        "env!",
        "option_env!",
        "json!",
        "fnmain(",
    ]
    .iter()
    .any(|forbidden| production.contains(forbidden))
    {
        return false;
    }
    if [
        "File",
        "OpenOptions",
        "Path",
        "PathBuf",
        "Command",
        "TcpStream",
        "UnixStream",
        "serde",
        "serde_json",
        "clap",
        "argh",
        "lexopt",
        "policy_key",
        "policy_root",
        "trust_root",
        "root_key",
        "secret_key",
        "public_key",
        "keyring",
        "fe2o3_kfd",
        "fe2o3_hsa_runtime",
        "hip_runtime",
        "ferric_engine",
        "load",
        "launch",
        "authorize_hsa_load",
    ]
    .iter()
    .any(|identifier| contains_identifier(&production, identifier))
    {
        return false;
    }
    true
}

#[test]
fn authority_policy_rejects_whitespace_comments_ufcs_paths_and_turbofish() {
    let inject = |snippet: &str| {
        SOURCE.replacen(
            TEST_MODULE_BOUNDARY,
            &format!("{snippet}\n\n{TEST_MODULE_BOUNDARY}"),
            1,
        )
    };
    for snippet in [
        "fn hostile() { runtime . load (); }",
        "fn hostile() { runtime . /* split */ load (); }",
        "fn hostile() { Runtime :: load (bytes); }",
        "fn hostile() { Runtime :: load :: <Artifact> (bytes); }",
        "fn hostile() { runtime . launch :: <Grid> (); }",
        "fn hostile() { Runtime :: launch (grid); }",
        "fn hostile() { std :: process :: Command :: new (); }",
        "fn hostile() { std /* split */ :: process :: Command :: new (); }",
        "fn hostile() { Command :: <Mode> :: new (); }",
        "fn hostile() { descriptor /* split */ . launch ().block_size(); }",
        "fn hostile() { authorize_hsa_load :: <Adapter> (); }",
        "fn hostile() { (Type::<Artifact>::load)(owner); }",
        "fn hostile() { let f = Type::<Artifact>::load; f(owner); }",
        "fn hostile() { (Type::<Grid>::launch)(owner); }",
        "fn hostile() { let f = Type::<Grid>::launch; f(owner); }",
        "fn hostile() { (Type::<Adapter>::authorize_hsa_load)(owner); }",
        "fn hostile() { let f = Type::<Adapter>::authorize_hsa_load; f(owner); }",
    ] {
        assert!(
            !production_has_no_external_authority_surface(&inject(snippet)),
            "normalized production policy accepted hostile source: {snippet}"
        );
    }

    let inert_decoys = inject(
        "const INERT_DECOY: &str = \"std :: process :: Command :: new (); Runtime::load::<Artifact>();\";\n/* Runtime :: launch :: <Grid> (); */",
    );
    assert!(production_has_no_external_authority_surface(&inert_decoys));

    let similar_identifiers = inject(
        "fn inert(kernarg_preload: u16, launch_block_size: u32) { let _ = (kernarg_preload, launch_block_size); }",
    );
    assert!(production_has_no_external_authority_surface(
        &similar_identifiers
    ));
}

#[test]
fn standalone_manifest_pins_the_current_generic_boundary() {
    assert!(MANIFEST.contains("[workspace]"));
    assert!(MANIFEST.contains(&format!(
        "fe2o3-host = {{ git = \"https://github.com/harsh-nod/fe2o3.git\", rev = \"{FE2O3_REVISION}\", version = \"=0.1.0\" }}"
    )));
    assert!(MANIFEST.contains(&format!(
        "fe2o3-hsaco-finalize = {{ git = \"https://github.com/harsh-nod/fe2o3.git\", rev = \"{FE2O3_REVISION}\", version = \"=0.1.0\" }}"
    )));
    assert!(MANIFEST.contains(
        "ferric-qwen3-all-kernels-device-v1 = { path = \"../../device/qwen3-all-kernels-v1\" }"
    ));
    assert!(!MANIFEST.contains("Cargo.lock"));
    assert_eq!(MANIFEST.matches(FE2O3_REVISION).count(), 2);
}

#[test]
fn documentation_states_the_non_authority_boundary() {
    let normalized_readme = README.split_whitespace().collect::<Vec<_>>().join(" ");
    for statement in [
        "private reject-only projection",
        "typed `WorkerV3RosterVerificationRequestV1`",
        "exactly 12 ordered entry rows",
        "typed descriptor, ELF-binding, and physical-kernel facts",
        "kernarg-preload field",
        "not runtime pointers or load authority",
        "lineage subprojection remains",
        "`None` is faithfully projected, but the association preflight rejects it",
        "no public constructor, serializer, or JSON input",
        "no environment, file, or CLI input",
        "finalized-HSACO verifier exactly once",
        "exact bytes borrowed from the typed request",
        "Descriptor-table, binding, parse, or canonical whole-HSACO digest failure",
        "`ExactFinalizedHsacoVerificationFailed` error",
        "owned `FinalizedDescriptorInspection`",
        "same-process descriptive integrity, not an independent authority",
        "neither owns the artifact bytes nor loads the code object",
        "common-custody preflight in exact order",
        "independently revalidates the finalizer derivation",
        "validates the common multi-root compiler proof inputs",
        "validates the common multi-root target lineage by borrowing those proof inputs",
        "Each failure maps to a distinct fail-closed error",
        "three inferred move-only common owners and the owned descriptive reinspection are retained together",
        "not exposed or serialized",
        "lexically scoped, zero-argument association closure",
        "finalizer identity and finalized-HSACO digest and length",
        "final LLVM module to both the finalizer compiler module and semantic handoff module",
        "literal `gfx942:xnack-` / COV6 target",
        "exactly 12 markers, proof roots, and target workgroups",
        "reinspection target and COV must equal the typed request",
        "complete descriptor table must be fully equal to the typed request table",
        "exactly 12 metadata kernels and descriptor bindings",
        "descriptive load layout",
        "marker/proof binding bijection in both directions",
        "by binding identity rather than ordinal",
        "lineage, descriptor, ELF-binding, and physical-kernel facts",
        "metadata-kernel and marker orders are distinct domains",
        "one unique reinspected metadata index using both the export name and descriptor symbol",
        "checked 12-element coverage table",
        "requires a complete permutation",
        "never uses the canonical ordinal to index either reinspection array",
        "fully equal to the typed request values at the canonical ordinal",
        "both the reinspected and typed binding kernel indices must equal the found metadata index",
        "target workgroup is read at the matched proof-root index",
        "only `BlockSizeV1::Exact` equal to the proof-root workgroup is accepted",
        "`Any`, `AtMost`, or a missing descriptor is rejected",
        "closure ends before all four owners are moved unchanged",
        "do not establish the per-entry proof-to-executable, Rust layout, or Rust effect joins",
        "does not construct fe2o3 verification evidence",
        "no protected policy key or trust root",
        "neither panics nor invents a zero identity",
        "unconditional `Err(MissingProtectedVerificationReceipt)`",
        "does not accept hashes as a substitute",
        "grants no verification, load, launch, or inference authority",
        "no direct KFD, HSA, HIP, engine, or model import",
        "broader resolved runtime closure",
        "MissingProtectedVerificationReceipt",
    ] {
        assert!(
            normalized_readme.contains(statement),
            "README is missing `{statement}`"
        );
    }
}
