#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits an undocumented helper module.

//! Attributed Rust source for Ferric's exact Qwen3 RMSNorm device kernel.
//!
//! This crate carries source, ABI, and finite-profile facts only. It grants no
//! artifact, launch, KFD dispatch, numerical-qualification, or M1 authority.

use fe2o3_device::{
    Bf16, Gfx942Collectives, Index1D, Math, RowStriped2D, Wave64, WaveLane, WriteOnlyDisjointSlice,
    kernel, memory, thread,
};

/// Exact exported kernel symbol retained from the direct-LLVM implementation.
pub const QWEN3_RMSNORM_KERNEL_SYMBOL_V1: &str = "qwen3_rmsnorm_v1";
/// Exact AMDHSA descriptor symbol retained by the authoritative artifact contract.
pub const QWEN3_RMSNORM_KERNEL_DESCRIPTOR_SYMBOL_V1: &str = "qwen3_rmsnorm_v1.kd";
/// Exact AMD GPU target retained by the authoritative compiler lane.
pub const QWEN3_RMSNORM_TARGET_V1: &str = "gfx942:xnack-";
/// Exact code-object version retained by the authoritative compiler lane.
pub const QWEN3_RMSNORM_CODE_OBJECT_VERSION_V1: u8 = 6;
/// Exact workgroup size in workitems.
pub const QWEN3_RMSNORM_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
/// Largest admitted one-dimensional grid in workgroups.
pub const QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1: u32 = 65_536;
/// Largest admitted one-dimensional AQL grid in total workitems.
pub const QWEN3_RMSNORM_MAX_GRID_WORKITEMS_V1: u32 = 4_194_304;
/// Exact Qwen3 epsilon bit pattern.
pub const QWEN3_RMSNORM_EPSILON_BITS_V1: u32 = 1e-6_f32.to_bits();
/// Exact pure RMSNorm behavior tag.
pub const QWEN3_RMSNORM_BEHAVIOR_PURE_V1: u32 = 0;
/// Exact residual-fused RMSNorm behavior tag.
pub const QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1: u32 = 1;
/// Maximum row elements owned by one wave lane.
pub const QWEN3_RMSNORM_ELEMENTS_PER_LANE_V1: usize = 64;
/// Number of exact role/bucket/operation profiles.
pub const QWEN3_RMSNORM_PROFILE_COUNT_V1: usize = 132;
/// Explicit kernarg bytes: five slice records followed by four scalars.
pub const QWEN3_RMSNORM_EXPLICIT_KERNARG_BYTES_V1: u64 = 96;
/// Hidden AMDHSA kernarg tail offset.
pub const QWEN3_RMSNORM_HIDDEN_KERNARG_OFFSET_V1: u64 = 96;
/// Total COV6 kernarg bytes including the hidden tail.
pub const QWEN3_RMSNORM_TOTAL_KERNARG_BYTES_V1: u64 = 352;
/// Required kernarg segment alignment.
pub const QWEN3_RMSNORM_KERNARG_ALIGNMENT_V1: u64 = 8;

/// Access class for one canonical global-buffer ABI record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3RmsNormBufferAccessV1 {
    ReadOnly,
    WriteOnly,
}

/// One canonical global-buffer component of the RMSNorm ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3RmsNormGlobalBufferAbiV1 {
    pub ordinal: usize,
    pub name: &'static str,
    pub offset: u64,
    pub alignment: u64,
    pub access: Qwen3RmsNormBufferAccessV1,
}

/// Exact five-buffer order and physical offsets.
pub const QWEN3_RMSNORM_GLOBAL_BUFFER_ABI_V1: [Qwen3RmsNormGlobalBufferAbiV1; 5] = [
    Qwen3RmsNormGlobalBufferAbiV1 {
        ordinal: 0,
        name: "input_bf16",
        offset: 0,
        alignment: 2,
        access: Qwen3RmsNormBufferAccessV1::ReadOnly,
    },
    Qwen3RmsNormGlobalBufferAbiV1 {
        ordinal: 2,
        name: "residual_bf16",
        offset: 16,
        alignment: 2,
        access: Qwen3RmsNormBufferAccessV1::ReadOnly,
    },
    Qwen3RmsNormGlobalBufferAbiV1 {
        ordinal: 4,
        name: "weight_bf16",
        offset: 32,
        alignment: 2,
        access: Qwen3RmsNormBufferAccessV1::ReadOnly,
    },
    Qwen3RmsNormGlobalBufferAbiV1 {
        ordinal: 6,
        name: "fused_residual_bf16",
        offset: 48,
        alignment: 2,
        access: Qwen3RmsNormBufferAccessV1::WriteOnly,
    },
    Qwen3RmsNormGlobalBufferAbiV1 {
        ordinal: 8,
        name: "normalized_bf16",
        offset: 64,
        alignment: 2,
        access: Qwen3RmsNormBufferAccessV1::WriteOnly,
    },
];

/// Target or speculative-draft Qwen3 model role.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3RmsNormModelRoleV1 {
    Target8B = 1,
    Draft06B = 2,
}

impl Qwen3RmsNormModelRoleV1 {
    #[must_use]
    pub const fn hidden_size(self) -> u32 {
        match self {
            Self::Target8B => 4_096,
            Self::Draft06B => 1_024,
        }
    }

    #[must_use]
    pub const fn query_heads(self) -> u32 {
        match self {
            Self::Target8B => 32,
            Self::Draft06B => 16,
        }
    }

    #[must_use]
    pub const fn key_value_heads(self) -> u32 {
        8
    }
}

/// One exact Ferric M1 mode bucket.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3RmsNormBucketKindV1 {
    PrefillS1T128 = 1,
    PrefillS8T128 = 2,
    PrefillS1T512 = 3,
    PrefillS1T2048 = 4,
    DecodeS1C8192 = 5,
    DecodeS8C8192 = 6,
    DecodeS32C8192 = 7,
    SpeculativeS1K4C8192 = 8,
    SpeculativeS8K4C8192 = 9,
    SpeculativeS1K8C8192 = 10,
    SpeculativeS1K16C8192 = 11,
}

impl Qwen3RmsNormBucketKindV1 {
    #[must_use]
    pub const fn sequence_and_active_tokens(self, role: Qwen3RmsNormModelRoleV1) -> [u32; 2] {
        match self {
            Self::PrefillS1T128 => [1, 128],
            Self::PrefillS8T128 => [8, 128],
            Self::PrefillS1T512 => [1, 512],
            Self::PrefillS1T2048 => [1, 2_048],
            Self::DecodeS1C8192 => [1, 1],
            Self::DecodeS8C8192 => [8, 1],
            Self::DecodeS32C8192 => [32, 1],
            Self::SpeculativeS1K4C8192 => match role {
                Qwen3RmsNormModelRoleV1::Target8B => [1, 5],
                Qwen3RmsNormModelRoleV1::Draft06B => [1, 4],
            },
            Self::SpeculativeS8K4C8192 => match role {
                Qwen3RmsNormModelRoleV1::Target8B => [8, 5],
                Qwen3RmsNormModelRoleV1::Draft06B => [8, 4],
            },
            Self::SpeculativeS1K8C8192 => match role {
                Qwen3RmsNormModelRoleV1::Target8B => [1, 9],
                Qwen3RmsNormModelRoleV1::Draft06B => [1, 8],
            },
            Self::SpeculativeS1K16C8192 => match role {
                Qwen3RmsNormModelRoleV1::Target8B => [1, 17],
                Qwen3RmsNormModelRoleV1::Draft06B => [1, 16],
            },
        }
    }
}

/// Exact graph operation or separate residual-fused auxiliary operation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3RmsNormOperationV1 {
    InputRmsNorm = 1,
    QueryRmsNorm = 2,
    KeyRmsNorm = 3,
    PostAttentionRmsNorm = 4,
    FinalRmsNorm = 5,
    ResidualFusedHidden = 6,
}

impl Qwen3RmsNormOperationV1 {
    #[must_use]
    pub const fn behavior(self) -> u32 {
        match self {
            Self::ResidualFusedHidden => QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1,
            Self::InputRmsNorm
            | Self::QueryRmsNorm
            | Self::KeyRmsNorm
            | Self::PostAttentionRmsNorm
            | Self::FinalRmsNorm => QWEN3_RMSNORM_BEHAVIOR_PURE_V1,
        }
    }
}

pub const QWEN3_RMSNORM_MODEL_ROLES_V1: [Qwen3RmsNormModelRoleV1; 2] = [
    Qwen3RmsNormModelRoleV1::Target8B,
    Qwen3RmsNormModelRoleV1::Draft06B,
];

pub const QWEN3_RMSNORM_BUCKET_KINDS_V1: [Qwen3RmsNormBucketKindV1; 11] = [
    Qwen3RmsNormBucketKindV1::PrefillS1T128,
    Qwen3RmsNormBucketKindV1::PrefillS8T128,
    Qwen3RmsNormBucketKindV1::PrefillS1T512,
    Qwen3RmsNormBucketKindV1::PrefillS1T2048,
    Qwen3RmsNormBucketKindV1::DecodeS1C8192,
    Qwen3RmsNormBucketKindV1::DecodeS8C8192,
    Qwen3RmsNormBucketKindV1::DecodeS32C8192,
    Qwen3RmsNormBucketKindV1::SpeculativeS1K4C8192,
    Qwen3RmsNormBucketKindV1::SpeculativeS8K4C8192,
    Qwen3RmsNormBucketKindV1::SpeculativeS1K8C8192,
    Qwen3RmsNormBucketKindV1::SpeculativeS1K16C8192,
];

pub const QWEN3_RMSNORM_OPERATIONS_V1: [Qwen3RmsNormOperationV1; 6] = [
    Qwen3RmsNormOperationV1::InputRmsNorm,
    Qwen3RmsNormOperationV1::QueryRmsNorm,
    Qwen3RmsNormOperationV1::KeyRmsNorm,
    Qwen3RmsNormOperationV1::PostAttentionRmsNorm,
    Qwen3RmsNormOperationV1::FinalRmsNorm,
    Qwen3RmsNormOperationV1::ResidualFusedHidden,
];

/// Exact checked launch and buffer dimensions for one catalog profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3RmsNormProfileV1 {
    role: Qwen3RmsNormModelRoleV1,
    bucket: Qwen3RmsNormBucketKindV1,
    operation: Qwen3RmsNormOperationV1,
    rows: u32,
    width: u32,
}

impl Qwen3RmsNormProfileV1 {
    #[must_use]
    pub const fn role(self) -> Qwen3RmsNormModelRoleV1 {
        self.role
    }

    #[must_use]
    pub const fn bucket(self) -> Qwen3RmsNormBucketKindV1 {
        self.bucket
    }

    #[must_use]
    pub const fn operation(self) -> Qwen3RmsNormOperationV1 {
        self.operation
    }

    #[must_use]
    pub const fn behavior(self) -> u32 {
        self.operation.behavior()
    }

    #[must_use]
    pub const fn rows(self) -> u32 {
        self.rows
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn row_elements(self) -> u64 {
        self.rows as u64 * self.width as u64
    }

    #[must_use]
    pub const fn weight_elements(self) -> u64 {
        self.width as u64
    }

    #[must_use]
    pub const fn hsa_adapter_block_counts(self) -> [u32; 3] {
        [self.rows, 1, 1]
    }

    #[must_use]
    pub const fn aql_grid_work_items(self) -> [u32; 3] {
        [self.rows * QWEN3_RMSNORM_WORKGROUP_V1[0], 1, 1]
    }
}

/// Derives one of the exact 132 role/bucket/operation profiles.
#[must_use]
pub const fn qwen3_rmsnorm_profile_v1(
    role: Qwen3RmsNormModelRoleV1,
    bucket: Qwen3RmsNormBucketKindV1,
    operation: Qwen3RmsNormOperationV1,
) -> Qwen3RmsNormProfileV1 {
    let dimensions = bucket.sequence_and_active_tokens(role);
    let base_rows = dimensions[0] * dimensions[1];
    let (rows, width) = match operation {
        Qwen3RmsNormOperationV1::QueryRmsNorm => (base_rows * role.query_heads(), 128),
        Qwen3RmsNormOperationV1::KeyRmsNorm => (base_rows * role.key_value_heads(), 128),
        Qwen3RmsNormOperationV1::InputRmsNorm
        | Qwen3RmsNormOperationV1::PostAttentionRmsNorm
        | Qwen3RmsNormOperationV1::FinalRmsNorm
        | Qwen3RmsNormOperationV1::ResidualFusedHidden => (base_rows, role.hidden_size()),
    };
    Qwen3RmsNormProfileV1 {
        role,
        bucket,
        operation,
        rows,
        width,
    }
}

/// Returns whether the generic machine shape is admitted by the exact behavior tag.
#[must_use]
pub const fn qwen3_rmsnorm_shape_is_admitted_v1(rows: u32, width: u32, behavior: u32) -> bool {
    rows != 0
        && rows <= QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1
        && ((behavior == QWEN3_RMSNORM_BEHAVIOR_PURE_V1
            && (width == 128 || width == 1_024 || width == 4_096))
            || (behavior == QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1
                && (width == 1_024 || width == 4_096)))
}

/// Returns whether all five element lengths match one exact machine invocation.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub const fn qwen3_rmsnorm_lengths_are_admitted_v1(
    input: usize,
    residual: usize,
    weight: usize,
    fused_output: usize,
    normalized_output: usize,
    rows: u32,
    width: u32,
    behavior: u32,
) -> bool {
    if !qwen3_rmsnorm_shape_is_admitted_v1(rows, width, behavior) {
        return false;
    }
    let elements = rows as usize * width as usize;
    input == elements
        && weight == width as usize
        && normalized_output == elements
        && ((behavior == QWEN3_RMSNORM_BEHAVIOR_PURE_V1 && residual == 0 && fused_output == 0)
            || (behavior == QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1
                && residual == elements
                && fused_output == elements))
}

/// Computes RMSNorm with one wave64 workgroup per row.
///
/// Inputs and outputs retain physical `u16` BF16 carriers. Pure mode requires
/// genuinely empty residual and fused-output slices. Fused mode adds the
/// residual in FP32, writes that sum narrowed to BF16, and normalizes the full
/// FP32 sum. Every lane owns columns `lane + component * 64` in its row.
#[allow(clippy::too_many_arguments, clippy::len_zero)]
#[kernel(
    typed,
    launch(
        required = [64, 1, 1],
        max = [64, 1, 1],
        max_grid = [65536, 1, 1]
    ),
    control_flow(loop_bounds(4096, 64))
)]
pub fn qwen3_rmsnorm_v1(
    input_bf16: &[u16],
    residual_bf16: &[u16],
    weight_bf16: &[u16],
    mut fused_residual_bf16: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    mut normalized_bf16: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    rows: u32,
    width: u32,
    epsilon: f32,
    behavior: u32,
) {
    let pure_mode = behavior == QWEN3_RMSNORM_BEHAVIOR_PURE_V1;
    let fused_mode = behavior == QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1;
    let pure_width = width == 128 || width == 1_024 || width == 4_096;
    let fused_width = width == 1_024 || width == 4_096;
    let shape_valid = rows != 0
        && rows <= QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1
        && ((pure_mode && pure_width) || (fused_mode && fused_width));
    let elements = rows as usize * width as usize;
    let required_lengths = input_bf16.len() == elements
        && weight_bf16.len() == width as usize
        && normalized_bf16.len() == elements;
    let auxiliary_lengths = (pure_mode
        && residual_bf16.len() == 0
        && fused_residual_bf16.len() == 0)
        || (fused_mode && residual_bf16.len() == elements && fused_residual_bf16.len() == elements);
    let exact_grid =
        thread::grid_dim_x() == rows && thread::grid_dim_y() == 1 && thread::grid_dim_z() == 1;
    if !shape_valid
        || !required_lengths
        || !auxiliary_lengths
        || epsilon.to_bits() != QWEN3_RMSNORM_EPSILON_BITS_V1
        || !exact_grid
    {
        fe2o3_device::trap();
    }

    let row = thread::block_idx_x() as usize;
    let lane = WaveLane::<Wave64>::current();
    let lane_index = lane.get() as usize;
    let collectives = Gfx942Collectives::current();
    let row_base = row * width as usize;
    let mut local_sum = 0.0_f32;
    if lane_index == 0 {
        let mut column = 0_usize;
        while column < width as usize {
            let index = row_base + column;
            let input = Bf16::from_bits(memory::volatile_load(input_bf16, index));
            if !input.is_finite() {
                fe2o3_device::trap();
            }
            let input_value = input.to_f32();
            let normalized_input = if fused_mode {
                let residual = Bf16::from_bits(memory::volatile_load(residual_bf16, index));
                if !residual.is_finite() {
                    fe2o3_device::trap();
                }
                let fused = input_value + residual.to_f32();
                if !fused.is_finite() {
                    fe2o3_device::trap();
                }
                fused
            } else {
                input_value
            };
            let square = normalized_input * normalized_input;
            let next_sum = local_sum + square;
            if !square.is_finite() || !next_sum.is_finite() {
                fe2o3_device::trap();
            }
            local_sum = next_sum;
            column += 1;
        }
    }
    let sum = collectives.subgroup_reduce_sum_f32::<64>(local_sum);
    if !sum.is_finite() {
        fe2o3_device::trap();
    }
    let mean_square = sum / width as f32;
    let stabilized = mean_square + epsilon;
    if !mean_square.is_finite() || !stabilized.is_finite() || stabilized <= 0.0 {
        fe2o3_device::trap();
    }
    let denominator = Math::current().sqrt_f32(stabilized);
    if !denominator.is_finite() || denominator <= 0.0 {
        fe2o3_device::trap();
    }
    let inverse_rms = 1.0_f32 / denominator;
    if !inverse_rms.is_finite() {
        fe2o3_device::trap();
    }
    let Some(output_row) = thread::index_1d().checked_row_striped_2d::<64, 64>() else {
        fe2o3_device::trap();
    };

    let mut component = 0_usize;
    while component < 64 {
        let column = lane_index + component * 64;
        if column < width as usize {
            let index = row_base + column;
            let input = Bf16::from_bits(memory::volatile_load(input_bf16, index));
            if !input.is_finite() {
                fe2o3_device::trap();
            }
            let input_value = input.to_f32();
            let normalized_input = if fused_mode {
                let residual = Bf16::from_bits(memory::volatile_load(residual_bf16, index));
                if !residual.is_finite() {
                    fe2o3_device::trap();
                }
                let fused = input_value + residual.to_f32();
                if !fused.is_finite() {
                    fe2o3_device::trap();
                }
                let narrowed_fused = Bf16::from_f32(fused);
                if !narrowed_fused.is_finite() {
                    fe2o3_device::trap();
                }
                if !fused_residual_bf16.write_row_striped_2d(
                    &output_row,
                    component,
                    rows as usize,
                    width as usize,
                    width as usize,
                    narrowed_fused.to_bits(),
                ) {
                    fe2o3_device::trap();
                }
                fused
            } else {
                input_value
            };
            let normalized = normalized_input * inverse_rms;
            let weight = Bf16::from_bits(memory::volatile_load(weight_bf16, column));
            if !weight.is_finite() {
                fe2o3_device::trap();
            }
            let weighted = normalized * weight.to_f32();
            if !normalized.is_finite() || !weighted.is_finite() {
                fe2o3_device::trap();
            }
            let narrowed_weighted = Bf16::from_f32(weighted);
            if !narrowed_weighted.is_finite() {
                fe2o3_device::trap();
            }
            if !normalized_bf16.write_row_striped_2d(
                &output_row,
                component,
                rows as usize,
                width as usize,
                width as usize,
                narrowed_weighted.to_bits(),
            ) {
                fe2o3_device::trap();
            }
        }
        component += 1;
    }
}
