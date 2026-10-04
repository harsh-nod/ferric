#![no_std]
#![cfg_attr(target_arch = "amdgpu", feature(rustc_attrs))]
#![cfg_attr(target_arch = "amdgpu", allow(internal_features))]

//! Source-only engineering entry for a real RMSNorm4096 -> K512 checkpoint.
//! No typed grouped import, object, launch, or numerical qualification yet.
//! The registered ordinary Rust root below is intentionally still unsupported
//! by the existing three-root classifier. Extend that same production route;
//! do not supply substitute IR or turn this declaration into launch permission.
//! This standalone source input is not included by the existing RMSNorm or
//! projection libraries; materialize this file and its local include together
//! through the existing ordinary source extraction driver.
//!
//! AMD-only force-inlining asks the pinned nightly to retain the real numerical
//! and scheduler bodies in kernel MIR, without changing compiler admission.

include!("wave_numerics_v1.rs");

use fe2o3_device::finite_join::{
    FiniteJoinWorkerResult,
    wave_tasks::{KeyTaskV1, NormTaskV1, WaveTaskStorageV1, WaveTaskV1, WaveWorkerV1},
};
use fe2o3_device::{Bf16, Gfx950Subgroup, Math, WorkgroupLdsScope};

const EPSILON: f32 = 1e-6_f32;

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn norm(task: &mut NormTaskV1<'_, '_>, subgroup: &Gfx950Subgroup) {
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
fn key(task: &mut KeyTaskV1<'_, '_>, subgroup: &Gfx950Subgroup) {
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
fn execute_task(task: WaveTaskV1<'_, '_>) {
    let subgroup = Gfx950Subgroup::current();
    match task {
        WaveTaskV1::Norm(mut task) => norm(&mut task, &subgroup),
        WaveTaskV1::Key(mut task) => key(&mut task, &subgroup),
    }
}

/// Consumes a separately unsafe engineering storage binding. This is real Rust
/// source, not a public checked kernel or protected publication entry point.
/// Storage construction retains all convergence/atomic/lifetime obligations.
#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
pub fn engineering_rmsnorm_key_tasks_v1(
    storage: WaveTaskStorageV1<'_>,
) -> Result<FiniteJoinWorkerResult, u32> {
    let mut scope = WorkgroupLdsScope::current();
    WaveWorkerV1::run(storage, &mut scope, |task| execute_task(task))
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn checked_rmsnorm_key_tasks_v1(storage: WaveTaskStorageV1<'_>) {
    match engineering_rmsnorm_key_tasks_v1(storage) {
        Ok(observation) => {
            if observation.error != 0 {
                fe2o3_device::trap();
            }
        }
        Err(_) => fe2o3_device::trap(),
    }
}

/// The actual six-root source extraction target, not an accepted kernel image.
/// Its grouped type/claim operations require a distinct compiler recipe; the
/// old finite-join run terminal and its15-event witness cannot admit this body.
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
pub fn ferric_qwen3_claimed_rmsnorm_key_bf16_v1(storage: WaveTaskStorageV1<'_>) {
    checked_rmsnorm_key_tasks_v1(storage);
}
