#![no_std]
#![cfg_attr(target_arch = "amdgpu", feature(rustc_attrs, stmt_expr_attributes))]
#![cfg_attr(target_arch = "amdgpu", allow(internal_features))]

//! Distinct engineering source: RMSNorm -> QKV -> head norm/RoPE/KV -> attention.
//! This thirteen-root profile needs its own authenticated compiler/runtime path.
//! Raw projections stay immutable. Post writes rotated Q and one paged K/V row;
//! the dependent attention task reads initialized causal history and writes BF16.
//! The Post V3 profile and launch route are not widened. This is one layer prefix,
//! not a complete model or a scheduling-progress proof.

include!("wave_numerics_v1.rs");
include!("head_rope_numerics_v3.rs");
include!("attention_online.rs");
include!("attention_numerics_v4.rs");

use fe2o3_device::finite_join::{
    FiniteJoinWorkerResult,
    wave_qkv_attention_tasks_v4::{
        AttentionReadyTaskV4, AttentionTaskV4, QkvAttentionNormTaskV4,
        QkvAttentionProjectionTaskV4, WaveQkvAttentionTaskStorageV4, WaveQkvAttentionTaskV4,
        WaveQkvAttentionWorkerV4,
    },
};
use fe2o3_device::{Bf16, Gfx950Subgroup, Math, WorkgroupLdsScope};

const EPSILON: f32 = 1e-6_f32;

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn norm(task: &mut QkvAttentionNormTaskV4<'_, '_>, subgroup: &Gfx950Subgroup) {
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
fn projection(task: &mut QkvAttentionProjectionTaskV4<'_, '_>, subgroup: &Gfx950Subgroup) {
    let lane = task.lane();
    let mut column = 0;
    while column < 256 {
        // Invalid lanes continue all256 collectives; rejection is gathered
        // uniformly afterward, so no lane exits around a subgroup operation.
        let (sum, narrowed, finite) = qwen_wave_dot_v1!(
            lane,
            inner,
            Bf16::from_bits(match task.input(inner) {
                Some(value) => value,
                None => 0x7fc0,
            })
            .to_f32(),
            Bf16::from_bits(match task.weight(column, inner) {
                Some(value) => value,
                None => 0x7fc0,
            })
            .to_f32(),
            subgroup
        );
        if !finite || !sum.is_finite() || !narrowed.is_finite() {
            task.reject();
        } else if lane == 0 && !task.write_column(column, narrowed.to_bits()) {
            task.reject();
        }
        column += 1;
    }
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn postprocess(task: &mut AttentionReadyTaskV4<'_, '_>) {
    qwen_head_rope_post_v3!(task, stabilized, Math::current().sqrt_f32(stabilized));
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn attention(task: &mut AttentionTaskV4<'_, '_>, subgroup: &Gfx950Subgroup) {
    let math = Math::current();
    qwen_claimed_attention_v4!(task, subgroup, math);
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn execute_task(task: WaveQkvAttentionTaskV4<'_, '_>) {
    let subgroup = Gfx950Subgroup::current();
    match task {
        WaveQkvAttentionTaskV4::Norm(mut task) => norm(&mut task, &subgroup),
        WaveQkvAttentionTaskV4::Projection(mut task) => projection(&mut task, &subgroup),
        WaveQkvAttentionTaskV4::Post(mut task) => postprocess(&mut task),
        WaveQkvAttentionTaskV4::Attention(mut task) => attention(&mut task, &subgroup),
    }
}

/// Consumes a separately unsafe engineering storage binding. This is real Rust
/// source, not a public checked kernel or protected publication entry point.
/// Storage construction retains all convergence/atomic/lifetime obligations.
#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
pub fn engineering_rmsnorm_qkv_attention_tasks_v4(
    storage: WaveQkvAttentionTaskStorageV4<'_>,
) -> Result<FiniteJoinWorkerResult, u32> {
    let mut scope = WorkgroupLdsScope::current();
    // Keep the callback in the checked body instead of an outlined FnMut borrow.
    WaveQkvAttentionWorkerV4::run(
        storage,
        &mut scope,
        #[cfg_attr(target_arch = "amdgpu", inline(always))]
        |task| execute_task(task),
    )
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn checked_rmsnorm_qkv_attention_tasks_v4(storage: WaveQkvAttentionTaskStorageV4<'_>) {
    match engineering_rmsnorm_qkv_attention_tasks_v4(storage) {
        Ok(observation) => {
            if observation.error != 0 {
                fe2o3_device::trap();
            }
        }
        Err(_) => fe2o3_device::trap(),
    }
}

/// Thirteen-root extraction target. Compiler admission must bind this distinct
/// profile; a Post V3 handoff cannot authenticate its additional output root.
// Two workgroup buffers of 64 u32 elements require 512 static LDS bytes.
#[fe2o3_device::kernel(
    typed,
    launch(
        required = [64, 1, 1],
        max = [64, 1, 1],
        max_grid = [2, 1, 1],
        static_shared_memory_bytes = 512
    )
)]
pub fn ferric_qwen3_claimed_rmsnorm_qkv_attention_bf16_v4(
    storage: WaveQkvAttentionTaskStorageV4<'_>,
) {
    checked_rmsnorm_qkv_attention_tasks_v4(storage);
}
