use std::collections::BTreeSet;

use ferric_qwen3_rmsnorm_device_v1::{
    QWEN3_RMSNORM_BEHAVIOR_PURE_V1, QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1,
    QWEN3_RMSNORM_BUCKET_KINDS_V1, QWEN3_RMSNORM_CODE_OBJECT_VERSION_V1,
    QWEN3_RMSNORM_EXPLICIT_KERNARG_BYTES_V1, QWEN3_RMSNORM_GLOBAL_BUFFER_ABI_V1,
    QWEN3_RMSNORM_HIDDEN_KERNARG_OFFSET_V1, QWEN3_RMSNORM_KERNARG_ALIGNMENT_V1,
    QWEN3_RMSNORM_KERNEL_DESCRIPTOR_SYMBOL_V1, QWEN3_RMSNORM_KERNEL_SYMBOL_V1,
    QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1, QWEN3_RMSNORM_MAX_GRID_WORKITEMS_V1,
    QWEN3_RMSNORM_MODEL_ROLES_V1, QWEN3_RMSNORM_OPERATIONS_V1, QWEN3_RMSNORM_PROFILE_COUNT_V1,
    QWEN3_RMSNORM_TARGET_V1, QWEN3_RMSNORM_TOTAL_KERNARG_BYTES_V1, QWEN3_RMSNORM_WORKGROUP_V1,
    Qwen3RmsNormBucketKindV1, Qwen3RmsNormBufferAccessV1, Qwen3RmsNormModelRoleV1,
    Qwen3RmsNormOperationV1, qwen3_rmsnorm_lengths_are_admitted_v1, qwen3_rmsnorm_profile_v1,
    qwen3_rmsnorm_shape_is_admitted_v1,
};

fn expected_base_rows(role: Qwen3RmsNormModelRoleV1, bucket: Qwen3RmsNormBucketKindV1) -> u32 {
    let target = role == Qwen3RmsNormModelRoleV1::Target8B;
    match bucket {
        Qwen3RmsNormBucketKindV1::PrefillS1T128 => 128,
        Qwen3RmsNormBucketKindV1::PrefillS8T128 => 1_024,
        Qwen3RmsNormBucketKindV1::PrefillS1T512 => 512,
        Qwen3RmsNormBucketKindV1::PrefillS1T2048 => 2_048,
        Qwen3RmsNormBucketKindV1::DecodeS1C8192 => 1,
        Qwen3RmsNormBucketKindV1::DecodeS8C8192 => 8,
        Qwen3RmsNormBucketKindV1::DecodeS32C8192 => 32,
        Qwen3RmsNormBucketKindV1::SpeculativeS1K4C8192 => {
            if target {
                5
            } else {
                4
            }
        }
        Qwen3RmsNormBucketKindV1::SpeculativeS8K4C8192 => {
            if target {
                40
            } else {
                32
            }
        }
        Qwen3RmsNormBucketKindV1::SpeculativeS1K8C8192 => {
            if target {
                9
            } else {
                8
            }
        }
        Qwen3RmsNormBucketKindV1::SpeculativeS1K16C8192 => {
            if target {
                17
            } else {
                16
            }
        }
    }
}

fn expected_role_geometry(role: Qwen3RmsNormModelRoleV1) -> (u32, u32, u32) {
    match role {
        Qwen3RmsNormModelRoleV1::Target8B => (4_096, 32, 8),
        Qwen3RmsNormModelRoleV1::Draft06B => (1_024, 16, 8),
    }
}

#[test]
fn exact_abi_order_offsets_access_and_kernarg_sizes_are_frozen() {
    assert_eq!(QWEN3_RMSNORM_KERNEL_SYMBOL_V1, "qwen3_rmsnorm_v1");
    assert_eq!(
        QWEN3_RMSNORM_KERNEL_DESCRIPTOR_SYMBOL_V1,
        "qwen3_rmsnorm_v1.kd"
    );
    assert_eq!(QWEN3_RMSNORM_TARGET_V1, "gfx942:xnack-");
    assert_eq!(QWEN3_RMSNORM_CODE_OBJECT_VERSION_V1, 6);
    let observed = QWEN3_RMSNORM_GLOBAL_BUFFER_ABI_V1.map(|record| {
        (
            record.ordinal,
            record.name,
            record.offset,
            record.alignment,
            record.access,
        )
    });
    assert_eq!(
        observed,
        [
            (0, "input_bf16", 0, 2, Qwen3RmsNormBufferAccessV1::ReadOnly),
            (
                2,
                "residual_bf16",
                16,
                2,
                Qwen3RmsNormBufferAccessV1::ReadOnly
            ),
            (
                4,
                "weight_bf16",
                32,
                2,
                Qwen3RmsNormBufferAccessV1::ReadOnly
            ),
            (
                6,
                "fused_residual_bf16",
                48,
                2,
                Qwen3RmsNormBufferAccessV1::WriteOnly
            ),
            (
                8,
                "normalized_bf16",
                64,
                2,
                Qwen3RmsNormBufferAccessV1::WriteOnly
            ),
        ]
    );
    assert_eq!(QWEN3_RMSNORM_EXPLICIT_KERNARG_BYTES_V1, 96);
    assert_eq!(QWEN3_RMSNORM_HIDDEN_KERNARG_OFFSET_V1, 96);
    assert_eq!(QWEN3_RMSNORM_TOTAL_KERNARG_BYTES_V1, 352);
    assert_eq!(QWEN3_RMSNORM_KERNARG_ALIGNMENT_V1, 8);
}

#[test]
fn all_132_profiles_match_independent_role_bucket_operation_geometry() {
    let mut count = 0;
    let mut identities = BTreeSet::new();
    let mut max_rows = 0;
    let mut max_workitems = 0;
    for role in QWEN3_RMSNORM_MODEL_ROLES_V1 {
        for bucket in QWEN3_RMSNORM_BUCKET_KINDS_V1 {
            let base_rows = expected_base_rows(role, bucket);
            let (hidden_size, query_heads, key_value_heads) = expected_role_geometry(role);
            for operation in QWEN3_RMSNORM_OPERATIONS_V1 {
                let profile = qwen3_rmsnorm_profile_v1(role, bucket, operation);
                let (rows, width, behavior) = match operation {
                    Qwen3RmsNormOperationV1::QueryRmsNorm => {
                        (base_rows * query_heads, 128, QWEN3_RMSNORM_BEHAVIOR_PURE_V1)
                    }
                    Qwen3RmsNormOperationV1::KeyRmsNorm => (
                        base_rows * key_value_heads,
                        128,
                        QWEN3_RMSNORM_BEHAVIOR_PURE_V1,
                    ),
                    Qwen3RmsNormOperationV1::ResidualFusedHidden => (
                        base_rows,
                        hidden_size,
                        QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1,
                    ),
                    Qwen3RmsNormOperationV1::InputRmsNorm
                    | Qwen3RmsNormOperationV1::PostAttentionRmsNorm
                    | Qwen3RmsNormOperationV1::FinalRmsNorm => {
                        (base_rows, hidden_size, QWEN3_RMSNORM_BEHAVIOR_PURE_V1)
                    }
                };
                assert_eq!(profile.role(), role);
                assert_eq!(profile.bucket(), bucket);
                assert_eq!(profile.operation(), operation);
                assert_eq!(profile.rows(), rows);
                assert_eq!(profile.width(), width);
                assert_eq!(profile.behavior(), behavior);
                assert_eq!(profile.row_elements(), u64::from(rows) * u64::from(width));
                assert_eq!(profile.weight_elements(), u64::from(width));
                assert_eq!(profile.hsa_adapter_block_counts(), [rows, 1, 1]);
                assert_eq!(profile.aql_grid_work_items(), [rows * 64, 1, 1]);
                assert!(qwen3_rmsnorm_shape_is_admitted_v1(rows, width, behavior));
                assert!(identities.insert((role, bucket, operation)));
                max_rows = max_rows.max(rows);
                max_workitems = max_workitems.max(rows * 64);
                count += 1;
            }
        }
    }
    assert_eq!(count, QWEN3_RMSNORM_PROFILE_COUNT_V1);
    assert_eq!(identities.len(), QWEN3_RMSNORM_PROFILE_COUNT_V1);
    assert_eq!(max_rows, QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1);
    assert_eq!(max_workitems, QWEN3_RMSNORM_MAX_GRID_WORKITEMS_V1);
    assert_eq!(QWEN3_RMSNORM_WORKGROUP_V1, [64, 1, 1]);
}

#[test]
fn generic_machine_shape_is_closed_over_behavior_and_width() {
    for width in [128, 1_024, 4_096] {
        assert!(qwen3_rmsnorm_shape_is_admitted_v1(
            1,
            width,
            QWEN3_RMSNORM_BEHAVIOR_PURE_V1
        ));
    }
    for width in [1_024, 4_096] {
        assert!(qwen3_rmsnorm_shape_is_admitted_v1(
            QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1,
            width,
            QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1
        ));
    }
    for (rows, width, behavior) in [
        (0, 128, 0),
        (1, 0, 0),
        (1, 64, 0),
        (1, 128, 1),
        (1, 1_024, 2),
        (1, 4_097, 0),
        (QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1 + 1, 128, 0),
        (u32::MAX, 128, u32::MAX),
    ] {
        assert!(!qwen3_rmsnorm_shape_is_admitted_v1(rows, width, behavior));
    }
}

#[test]
fn pure_requires_empty_auxiliaries_and_fused_requires_full_auxiliaries() {
    assert!(qwen3_rmsnorm_lengths_are_admitted_v1(
        128, 0, 128, 0, 128, 1, 128, 0
    ));
    assert!(qwen3_rmsnorm_lengths_are_admitted_v1(
        2_048, 2_048, 1_024, 2_048, 2_048, 2, 1_024, 1,
    ));
    for hostile in [
        (127, 0, 128, 0, 128, 1, 128, 0),
        (128, 1, 128, 0, 128, 1, 128, 0),
        (128, 0, 128, 1, 128, 1, 128, 0),
        (1_024, 0, 1_024, 1_024, 1_024, 1, 1_024, 1),
        (1_024, 1_024, 1_023, 1_024, 1_024, 1, 1_024, 1),
        (1_024, 1_024, 1_024, 1_024, 1_023, 1, 1_024, 1),
    ] {
        assert!(!qwen3_rmsnorm_lengths_are_admitted_v1(
            hostile.0, hostile.1, hostile.2, hostile.3, hostile.4, hostile.5, hostile.6, hostile.7,
        ));
    }
}
