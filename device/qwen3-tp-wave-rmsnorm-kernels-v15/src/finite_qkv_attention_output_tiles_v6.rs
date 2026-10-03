#![no_std]
#![cfg_attr(target_arch = "amdgpu", feature(rustc_attrs, stmt_expr_attributes))]
#![cfg_attr(target_arch = "amdgpu", allow(internal_features))]

//! P227 row-tiled engineering source: RMSNorm -> QKV -> post -> attention -> O.
//! Every row preserves V5 lane-strided FP32 accumulation and XOR reduction;
//! each claimed head preserves its original ascending causal-token recurrence.
//! This fifteen-root profile needs its own authenticated compiler/runtime path.
//! Raw projections stay immutable. Post writes rotated Q and one paged K/V row;
//! the dependent attention task reads initialized causal history and writes BF16.
//! O writes FP32 rank-local partials without narrowing before the later TP sum.
//! Earlier profiles and launch routes are not widened. This is one layer prefix,
//! not a complete model or a scheduling-progress proof.

include!("wave_numerics_v1.rs");
include!("head_rope_numerics_v3.rs");
include!("attention_online.rs");
include!("output_projection_numerics_v5.rs");
include!("prefix_tiles_numerics_v6.rs");

use fe2o3_device::finite_join::{
    FiniteJoinWorkerResult,
    wave_qkv_attention_output_tiles_v6::{
        AttentionHeadTileV6, AttentionReadyTileV6, OutputProjectionTileV6,
        QkvAttentionOutputNormTileV6, QkvAttentionOutputProjectionTileV6,
        WaveQkvAttentionOutputTileStorageV6, WaveQkvAttentionOutputTileTaskV6,
        WaveQkvAttentionOutputTileWorkerV6,
    },
};
use fe2o3_device::{Bf16, Gfx950Subgroup, Math, WorkgroupLdsScope};

const EPSILON: f32 = 1e-6_f32;

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn norm(task: &mut QkvAttentionOutputNormTileV6<'_, '_>, subgroup: &Gfx950Subgroup) {
    let lane = task.lane();
    let (sum, invalid) = qwen_wave_square_sum_v1!(
        lane,
        column,
        Bf16::from_bits(match task.input(column) {
            Some(value) => value,
            None => 0x7fc0,
        }),
        subgroup
    );
    // Collective results make this early exit uniform; every lane still enters
    // the outer round's completion pipeline, which rejects incomplete writes.
    if invalid != 0.0 || !sum.is_finite() {
        task.reject();
        return;
    }
    let mean = sum / 4096.0_f32;
    let stabilized = mean + EPSILON;
    if !mean.is_finite() || !stabilized.is_finite() || stabilized <= 0.0 {
        task.reject();
        return;
    }
    let denominator = Math::current().sqrt_f32(stabilized);
    if !denominator.is_finite() || denominator <= 0.0 {
        task.reject();
        return;
    }
    let inverse = 1.0_f32 / denominator;
    if !inverse.is_finite() {
        task.reject();
        return;
    }
    let mut component = 0;
    while component < 64 {
        let column = lane + 64 * component;
        let input = Bf16::from_bits(match task.input(column) {
            Some(value) => value,
            None => 0x7fc0,
        });
        let weight = Bf16::from_bits(match task.weight(column) {
            Some(value) => value,
            None => 0x7fc0,
        });
        let (normalized, narrowed) = qwen_norm_narrow_v1!(input.to_f32(), inverse);
        let weighted = narrowed.to_f32() * weight.to_f32();
        let output = Bf16::from_f32(weighted);
        if !input.is_finite()
            || !weight.is_finite()
            || !normalized.is_finite()
            || !narrowed.is_finite()
            || !weighted.is_finite()
            || !output.is_finite()
            || !task.write_component(component, output.to_bits())
        {
            task.reject();
        }
        component += 1;
    }
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn projection(task: &mut QkvAttentionOutputProjectionTileV6<'_, '_>, subgroup: &Gfx950Subgroup) {
    qwen_claimed_projection_tile_v6!(task, subgroup);
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn postprocess(task: &mut AttentionReadyTileV6<'_, '_>) {
    qwen_head_rope_post_v3!(task, stabilized, Math::current().sqrt_f32(stabilized));
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn attention(task: &mut AttentionHeadTileV6<'_, '_>, subgroup: &Gfx950Subgroup) {
    let math = Math::current();
    qwen_claimed_attention_head_v6!(task, subgroup, math);
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn output_projection(task: &mut OutputProjectionTileV6<'_, '_>, subgroup: &Gfx950Subgroup) {
    qwen_claimed_output_projection_tile_v6!(task, subgroup);
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn execute_task(task: WaveQkvAttentionOutputTileTaskV6<'_, '_>) {
    let subgroup = Gfx950Subgroup::current();
    match task {
        WaveQkvAttentionOutputTileTaskV6::Norm(mut task) => norm(&mut task, &subgroup),
        WaveQkvAttentionOutputTileTaskV6::Projection(mut task) => projection(&mut task, &subgroup),
        WaveQkvAttentionOutputTileTaskV6::Post(mut task) => postprocess(&mut task),
        WaveQkvAttentionOutputTileTaskV6::Attention(mut task) => attention(&mut task, &subgroup),
        WaveQkvAttentionOutputTileTaskV6::OutputProjection(mut task) => {
            output_projection(&mut task, &subgroup)
        }
    }
}

/// Consumes a separately unsafe engineering storage binding. This is real Rust
/// source, not a public checked kernel or protected publication entry point.
/// Storage construction retains all convergence/atomic/lifetime obligations.
#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
pub fn engineering_rmsnorm_qkv_attention_output_tiles_v6(
    storage: WaveQkvAttentionOutputTileStorageV6<'_>,
) -> Result<FiniteJoinWorkerResult, u32> {
    let mut scope = WorkgroupLdsScope::current();
    // Keep the callback in the checked body instead of an outlined FnMut borrow.
    WaveQkvAttentionOutputTileWorkerV6::run(
        storage,
        &mut scope,
        #[cfg_attr(target_arch = "amdgpu", inline(always))]
        |task| execute_task(task),
    )
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn checked_rmsnorm_qkv_attention_output_tiles_v6(storage: WaveQkvAttentionOutputTileStorageV6<'_>) {
    match engineering_rmsnorm_qkv_attention_output_tiles_v6(storage) {
        Ok(observation) => {
            if observation.error != 0 {
                fe2o3_device::trap();
            }
        }
        Err(_) => fe2o3_device::trap(),
    }
}

/// Fifteen-root extraction target. Compiler admission must bind this distinct
/// profile and all284 state words; a V5 22-word handoff is not interchangeable.
// 64 WG64 groups (global4096), with two64-u32 workgroup buffers/LDS512.
// 256 fixed rounds do not promise global progress or establish convergence.
#[fe2o3_device::kernel(
    typed,
    launch(
        required = [64, 1, 1],
        max = [64, 1, 1],
        max_grid = [64, 1, 1],
        static_shared_memory_bytes = 512
    )
)]
pub fn ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6(
    storage: WaveQkvAttentionOutputTileStorageV6<'_>,
) {
    checked_rmsnorm_qkv_attention_output_tiles_v6(storage);
}
