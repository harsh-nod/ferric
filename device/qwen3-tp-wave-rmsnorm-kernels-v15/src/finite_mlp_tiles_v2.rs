#![no_std]
#![cfg_attr(target_arch = "amdgpu", feature(rustc_attrs, stmt_expr_attributes))]
#![cfg_attr(target_arch = "amdgpu", allow(internal_features))]

//! Engineering row-tiled MLP: RMSNorm -> {Gate, Up} -> SwiGLU -> Down.
//! The eleven-root profile ends at rank-local FP32 partials. TP sum/residual and
//! the attention prefix are separate. Fixed rounds do not prove task progress.

include!("wave_numerics_v1.rs");
include!("mlp_numerics_v1.rs");
include!("mlp_tile_numerics_v2.rs");

use fe2o3_device::finite_join::{
    FiniteJoinWorkerResult,
    wave_mlp_tiles_v2::{
        MlpDownTileV2, MlpGateTileV2, MlpNormTileV2, MlpSwiGluTileV2, MlpUpTileV2,
        WaveMlpTileStorageV2, WaveMlpTileTaskV2, WaveMlpTileWorkerV2,
    },
};
use fe2o3_device::{Bf16, Gfx950Subgroup, Math, WorkgroupLdsScope};

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn norm(task: &mut MlpNormTileV2<'_, '_>, subgroup: &Gfx950Subgroup) {
    qwen_claimed_mlp_norm_v1!(
        task,
        subgroup,
        stabilized,
        Math::current().sqrt_f32(stabilized)
    );
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn gate(task: &mut MlpGateTileV2<'_, '_>, subgroup: &Gfx950Subgroup) {
    qwen_claimed_mlp_projection_tile_v2!(task, subgroup);
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn up(task: &mut MlpUpTileV2<'_, '_>, subgroup: &Gfx950Subgroup) {
    qwen_claimed_mlp_projection_tile_v2!(task, subgroup);
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn swiglu(task: &mut MlpSwiGluTileV2<'_, '_>) {
    let math = Math::current();
    qwen_claimed_mlp_swiglu_v1!(task, math);
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn down(task: &mut MlpDownTileV2<'_, '_>, subgroup: &Gfx950Subgroup) {
    qwen_claimed_mlp_down_tile_v2!(task, subgroup);
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn execute_task(task: WaveMlpTileTaskV2<'_, '_>) {
    let subgroup = Gfx950Subgroup::current();
    match task {
        WaveMlpTileTaskV2::Norm(mut task) => norm(&mut task, &subgroup),
        WaveMlpTileTaskV2::Gate(mut task) => gate(&mut task, &subgroup),
        WaveMlpTileTaskV2::Up(mut task) => up(&mut task, &subgroup),
        WaveMlpTileTaskV2::SwiGlu(mut task) => swiglu(&mut task),
        WaveMlpTileTaskV2::Down(mut task) => down(&mut task, &subgroup),
    }
}

/// Consumes unsafe engineering bindings, not a production launch capability.
#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
pub fn engineering_mlp_tiles_v2(
    storage: WaveMlpTileStorageV2<'_>,
) -> Result<FiniteJoinWorkerResult, u32> {
    let mut scope = WorkgroupLdsScope::current();
    WaveMlpTileWorkerV2::run_fixed_rounds(
        storage,
        &mut scope,
        #[cfg_attr(target_arch = "amdgpu", inline(always))]
        |task| execute_task(task),
    )
}

#[cfg_attr(target_arch = "amdgpu", rustc_force_inline)]
fn checked_mlp_tiles_v2(storage: WaveMlpTileStorageV2<'_>) {
    match engineering_mlp_tiles_v2(storage) {
        Ok(observation) => {
            if observation.error != 0 {
                fe2o3_device::trap();
            }
        }
        Err(_) => fe2o3_device::trap(),
    }
}

// Two 64-u32 exchange buffers; this entry is compiled separately, like V1-V5.
#[fe2o3_device::kernel(
    typed,
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [64, 1, 1],
           static_shared_memory_bytes = 512)
)]
pub fn ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2(storage: WaveMlpTileStorageV2<'_>) {
    checked_mlp_tiles_v2(storage);
}
