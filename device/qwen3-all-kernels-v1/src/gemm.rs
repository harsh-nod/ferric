#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits undocumented helper modules.

//! Attributed Rust source for Ferric's exact Qwen3 K1 device roots.

use fe2o3_device::{
    Bf16, DisjointSlice, Index1D, Tiled2D, WriteOnlyDisjointSlice, kernel, memory, thread,
};

pub const QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1: &str =
    "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1";
pub const QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1: &str =
    "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1";
pub const QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1: &str =
    "ferric_qwen3_token_embedding_bf16_copy_v1";
pub const QWEN3_GEMM_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
pub const QWEN3_GEMM_MAX_GRID_WORKGROUPS_V1: u32 = 1_215_488;
pub const QWEN3_TOKEN_EMBEDDING_MAX_GRID_WORKGROUPS_V1: u32 = 131_072;
pub const QWEN3_VOCABULARY_SIZE_V1: u32 = 151_936;

macro_rules! target_shape_is_admitted_v1 {
    ($n:expr, $k:expr, $beta_bits:expr) => {
        (($n == 4_096 && $k == 4_096 && ($beta_bits == 0 || $beta_bits == 1_065_353_216))
            || ($n == 1_024 && $k == 4_096 && $beta_bits == 0)
            || ($n == 12_288 && $k == 4_096 && $beta_bits == 0)
            || ($n == 4_096 && $k == 12_288 && $beta_bits == 1_065_353_216)
            || ($n == 151_936 && $k == 4_096 && $beta_bits == 0))
    };
}

macro_rules! draft_shape_is_admitted_v1 {
    ($n:expr, $k:expr, $beta_bits:expr) => {
        (($n == 2_048 && $k == 1_024 && $beta_bits == 0)
            || ($n == 1_024 && $k == 1_024 && $beta_bits == 0)
            || ($n == 1_024 && $k == 2_048 && $beta_bits == 1_065_353_216)
            || ($n == 3_072 && $k == 1_024 && $beta_bits == 0)
            || ($n == 1_024 && $k == 3_072 && $beta_bits == 1_065_353_216)
            || ($n == 151_936 && $k == 1_024 && $beta_bits == 0))
    };
}

macro_rules! reference_profile_is_admitted_v1 {
    ($m:expr, $n:expr, $k:expr, $beta_bits:expr) => {
        ((($m == 1 || $m == 5 || $m == 8 || $m == 9)
            && target_shape_is_admitted_v1!($n, $k, $beta_bits))
            || (($m == 1 || $m == 4 || $m == 8) && draft_shape_is_admitted_v1!($n, $k, $beta_bits)))
    };
}

macro_rules! vectorized_profile_is_admitted_v1 {
    ($m:expr, $n:expr, $k:expr, $beta_bits:expr) => {
        ((($m == 17
            || $m == 32
            || $m == 40
            || $m == 128
            || $m == 512
            || $m == 1_024
            || $m == 2_048)
            && target_shape_is_admitted_v1!($n, $k, $beta_bits))
            || (($m == 16 || $m == 32 || $m == 128 || $m == 512 || $m == 1_024 || $m == 2_048)
                && draft_shape_is_admitted_v1!($n, $k, $beta_bits)))
    };
}

macro_rules! embedding_profile_is_admitted_v1 {
    ($rows:expr, $hidden:expr, $vocabulary:expr) => {
        ($vocabulary == QWEN3_VOCABULARY_SIZE_V1
            && ((($rows == 1
                || $rows == 5
                || $rows == 8
                || $rows == 9
                || $rows == 17
                || $rows == 32
                || $rows == 40
                || $rows == 128
                || $rows == 512
                || $rows == 1_024
                || $rows == 2_048)
                && $hidden == 4_096)
                || (($rows == 1
                    || $rows == 4
                    || $rows == 8
                    || $rows == 16
                    || $rows == 32
                    || $rows == 128
                    || $rows == 512
                    || $rows == 1_024
                    || $rows == 2_048)
                    && $hidden == 1_024)))
    };
}

#[must_use]
pub const fn qwen3_gemm_reference_profile_is_admitted_v1(
    m: u32,
    n: u32,
    k: u32,
    beta_bits: u32,
) -> bool {
    reference_profile_is_admitted_v1!(m, n, k, beta_bits)
}

#[must_use]
pub const fn qwen3_gemm_vectorized_profile_is_admitted_v1(
    m: u32,
    n: u32,
    k: u32,
    beta_bits: u32,
) -> bool {
    vectorized_profile_is_admitted_v1!(m, n, k, beta_bits)
}

#[must_use]
pub const fn qwen3_token_embedding_profile_is_admitted_v1(
    rows: u32,
    hidden: u32,
    vocabulary: u32,
) -> bool {
    embedding_profile_is_admitted_v1!(rows, hidden, vocabulary)
}

#[kernel(
    typed,
    launch(
        required = [64, 1, 1],
        max = [64, 1, 1],
        max_grid = [1215488, 1, 1]
    ),
    control_flow(loop_bounds(12288))
)]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_gemm_reference_bf16_f32_bf16_v1(
    a: &[u16],
    b: &[u16],
    mut c: DisjointSlice<u16, Tiled2D<Index1D, 64, 16, 16, 4>>,
    m: u32,
    n: u32,
    k: u32,
    beta_bits: u32,
) {
    let target_shape_is_admitted =
        (n == 4_096 && k == 4_096 && (beta_bits == 0 || beta_bits == 1_065_353_216))
            || (n == 1_024 && k == 4_096 && beta_bits == 0)
            || (n == 12_288 && k == 4_096 && beta_bits == 0)
            || (n == 4_096 && k == 12_288 && beta_bits == 1_065_353_216)
            || (n == 151_936 && k == 4_096 && beta_bits == 0);
    let draft_shape_is_admitted = (n == 2_048 && k == 1_024 && beta_bits == 0)
        || (n == 1_024 && k == 1_024 && beta_bits == 0)
        || (n == 1_024 && k == 2_048 && beta_bits == 1_065_353_216)
        || (n == 3_072 && k == 1_024 && beta_bits == 0)
        || (n == 1_024 && k == 3_072 && beta_bits == 1_065_353_216)
        || (n == 151_936 && k == 1_024 && beta_bits == 0);
    let profile_is_admitted = ((m == 1 || m == 5 || m == 8 || m == 9) && target_shape_is_admitted)
        || ((m == 1 || m == 4 || m == 8) && draft_shape_is_admitted);
    if !profile_is_admitted {
        fe2o3_device::trap();
    }
    let m = m as usize;
    let n = n as usize;
    let k = k as usize;
    if m == 0 || n == 0 || k == 0 {
        fe2o3_device::trap();
    }
    if m < 2_049 {
    } else {
        fe2o3_device::trap();
    }
    if n < 151_937 {
    } else {
        fe2o3_device::trap();
    }
    if k < 12_289 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() != m * k || b.len() != k * n || c.len() != m * n {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let tiles_per_row = (n + 15) / 16;
    let tile_rows = (m + 15) / 16;
    let expected_extent = tiles_per_row * tile_rows * 64;
    let launch_extent = (thread::grid_dim_x() as usize) * (thread::block_dim_x() as usize);
    if launch_extent != expected_extent {
        fe2o3_device::trap();
    }
    let tile_index = raw / 64;
    if tile_index >= tiles_per_row * tile_rows {
        fe2o3_device::trap();
    }
    let (tile_row, tile_column) = if n == 1_024 {
        (tile_index / 64, tile_index % 64)
    } else if n == 2_048 {
        (tile_index / 128, tile_index % 128)
    } else if n == 3_072 {
        (tile_index / 192, tile_index % 192)
    } else if n == 4_096 {
        (tile_index / 256, tile_index % 256)
    } else if n == 12_288 {
        (tile_index / 768, tile_index % 768)
    } else if n == 151_936 {
        (tile_index / 9_496, tile_index % 9_496)
    } else {
        fe2o3_device::trap();
    };
    if tile_column < tiles_per_row {
    } else {
        fe2o3_device::trap();
    }
    if tile_row < 128 {
    } else {
        fe2o3_device::trap();
    }
    if tile_column < 9_496 {
    } else {
        fe2o3_device::trap();
    }
    let lane = raw % 64;
    let row_base = tile_row * 16 + (lane / 16) * 4;
    if row_base < 2_045 {
    } else {
        fe2o3_device::trap();
    }
    let column = tile_column * 16 + lane % 16;
    if column < n {
    } else {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    let beta_one = beta_bits == 1_065_353_216;
    let row_0 = row_base;
    let row_1 = row_0 + 1;
    let row_2 = row_1 + 1;
    let row_3 = row_2 + 1;
    let active_0 = row_0 < m && column < n;
    let active_1 = row_1 < m && column < n;
    let active_2 = row_2 < m && column < n;
    let active_3 = row_3 < m && column < n;
    let mut accumulator_0 = 0.0_f32;
    let mut accumulator_1 = 0.0_f32;
    let mut accumulator_2 = 0.0_f32;
    let mut accumulator_3 = 0.0_f32;
    let mut reduction = 0_usize;
    while reduction < k {
        let right_index = reduction * n + column;
        let right = Bf16::from_bits(memory::volatile_load(b, right_index)).to_f32();
        if active_0 {
            let left_index = row_0 * k + reduction;
            let left = Bf16::from_bits(memory::volatile_load(a, left_index)).to_f32();
            accumulator_0 = accumulator_0 + left * right;
        }
        if active_1 {
            let left_index = row_1 * k + reduction;
            let left = Bf16::from_bits(memory::volatile_load(a, left_index)).to_f32();
            accumulator_1 = accumulator_1 + left * right;
        }
        if active_2 {
            let left_index = row_2 * k + reduction;
            let left = Bf16::from_bits(memory::volatile_load(a, left_index)).to_f32();
            accumulator_2 = accumulator_2 + left * right;
        }
        if active_3 {
            let left_index = row_3 * k + reduction;
            let left = Bf16::from_bits(memory::volatile_load(a, left_index)).to_f32();
            accumulator_3 = accumulator_3 + left * right;
        }
        reduction += 1;
    }
    if active_0 {
        let Some(output) = c.get_tiled_2d_mut(&tile, 0, m, n, n) else {
            fe2o3_device::trap();
        };
        if beta_one {
            accumulator_0 = accumulator_0 + Bf16::from_bits(*output).to_f32();
        }
        *output = Bf16::from_f32(accumulator_0).to_bits();
    }
    if active_1 {
        let Some(output) = c.get_tiled_2d_mut(&tile, 1, m, n, n) else {
            fe2o3_device::trap();
        };
        if beta_one {
            accumulator_1 = accumulator_1 + Bf16::from_bits(*output).to_f32();
        }
        *output = Bf16::from_f32(accumulator_1).to_bits();
    }
    if active_2 {
        let Some(output) = c.get_tiled_2d_mut(&tile, 2, m, n, n) else {
            fe2o3_device::trap();
        };
        if beta_one {
            accumulator_2 = accumulator_2 + Bf16::from_bits(*output).to_f32();
        }
        *output = Bf16::from_f32(accumulator_2).to_bits();
    }
    if active_3 {
        let Some(output) = c.get_tiled_2d_mut(&tile, 3, m, n, n) else {
            fe2o3_device::trap();
        };
        if beta_one {
            accumulator_3 = accumulator_3 + Bf16::from_bits(*output).to_f32();
        }
        *output = Bf16::from_f32(accumulator_3).to_bits();
    }
}

#[kernel(
    typed,
    launch(
        required = [64, 1, 1],
        max = [64, 1, 1],
        max_grid = [1215488, 1, 1]
    ),
    control_flow(loop_bounds(12288))
)]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1(
    a: &[u16],
    b: &[u16],
    mut c: DisjointSlice<u16, Tiled2D<Index1D, 64, 16, 16, 4>>,
    m: u32,
    n: u32,
    k: u32,
    beta_bits: u32,
) {
    let target_shape_is_admitted =
        (n == 4_096 && k == 4_096 && (beta_bits == 0 || beta_bits == 1_065_353_216))
            || (n == 1_024 && k == 4_096 && beta_bits == 0)
            || (n == 12_288 && k == 4_096 && beta_bits == 0)
            || (n == 4_096 && k == 12_288 && beta_bits == 1_065_353_216)
            || (n == 151_936 && k == 4_096 && beta_bits == 0);
    let draft_shape_is_admitted = (n == 2_048 && k == 1_024 && beta_bits == 0)
        || (n == 1_024 && k == 1_024 && beta_bits == 0)
        || (n == 1_024 && k == 2_048 && beta_bits == 1_065_353_216)
        || (n == 3_072 && k == 1_024 && beta_bits == 0)
        || (n == 1_024 && k == 3_072 && beta_bits == 1_065_353_216)
        || (n == 151_936 && k == 1_024 && beta_bits == 0);
    let profile_is_admitted =
        ((m == 17 || m == 32 || m == 40 || m == 128 || m == 512 || m == 1_024 || m == 2_048)
            && target_shape_is_admitted)
            || ((m == 16 || m == 32 || m == 128 || m == 512 || m == 1_024 || m == 2_048)
                && draft_shape_is_admitted);
    if !profile_is_admitted {
        fe2o3_device::trap();
    }
    let reduction_bound = k as u64;
    let m = m as usize;
    let n = n as usize;
    let k = k as usize;
    if m == 0 || n == 0 || k == 0 {
        fe2o3_device::trap();
    }
    if m < 2_049 {
    } else {
        fe2o3_device::trap();
    }
    if n < 151_937 {
    } else {
        fe2o3_device::trap();
    }
    if k < 12_289 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() != m * k || b.len() != k * n || c.len() != m * n {
        fe2o3_device::trap();
    }
    if k % 4 != 0 {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let tiles_per_row = (n + 15) / 16;
    let tile_rows = (m + 15) / 16;
    let expected_extent = tiles_per_row * tile_rows * 64;
    let launch_extent = (thread::grid_dim_x() as usize) * (thread::block_dim_x() as usize);
    if launch_extent != expected_extent {
        fe2o3_device::trap();
    }
    let tile_index = raw / 64;
    if tile_index >= tiles_per_row * tile_rows {
        fe2o3_device::trap();
    }
    let (tile_row, tile_column) = if n == 1_024 {
        (tile_index / 64, tile_index % 64)
    } else if n == 2_048 {
        (tile_index / 128, tile_index % 128)
    } else if n == 3_072 {
        (tile_index / 192, tile_index % 192)
    } else if n == 4_096 {
        (tile_index / 256, tile_index % 256)
    } else if n == 12_288 {
        (tile_index / 768, tile_index % 768)
    } else if n == 151_936 {
        (tile_index / 9_496, tile_index % 9_496)
    } else {
        fe2o3_device::trap();
    };
    if tile_column < tiles_per_row {
    } else {
        fe2o3_device::trap();
    }
    if tile_row < 128 {
    } else {
        fe2o3_device::trap();
    }
    if tile_column < 9_496 {
    } else {
        fe2o3_device::trap();
    }
    let lane = raw % 64;
    let row_base = tile_row * 16 + (lane / 16) * 4;
    if row_base < 2_045 {
    } else {
        fe2o3_device::trap();
    }
    let column = tile_column * 16 + lane % 16;
    if column < n {
    } else {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    let beta_one = beta_bits == 1_065_353_216;
    let row_0 = row_base;
    let row_1 = row_0 + 1;
    let row_2 = row_1 + 1;
    let row_3 = row_2 + 1;
    let active_0 = row_0 < m && column < n;
    let active_1 = row_1 < m && column < n;
    let active_2 = row_2 < m && column < n;
    let active_3 = row_3 < m && column < n;
    let mut accumulator_0 = 0.0_f32;
    let mut accumulator_1 = 0.0_f32;
    let mut accumulator_2 = 0.0_f32;
    let mut accumulator_3 = 0.0_f32;
    let mut reduction_wide = 0_u64;
    while reduction_wide < reduction_bound {
        let reduction = reduction_wide as usize;
        if reduction + 3 >= k {
            fe2o3_device::trap();
        }
        let right_index_0 = reduction * n + column;
        let right_0 = Bf16::from_bits(memory::volatile_load(b, right_index_0)).to_f32();
        let right_index_1 = right_index_0 + n;
        let right_1 = Bf16::from_bits(memory::volatile_load(b, right_index_1)).to_f32();
        let right_index_2 = right_index_1 + n;
        let right_2 = Bf16::from_bits(memory::volatile_load(b, right_index_2)).to_f32();
        let right_index_3 = right_index_2 + n;
        let right_3 = Bf16::from_bits(memory::volatile_load(b, right_index_3)).to_f32();
        if active_0 {
            let left_index_0 = row_0 * k + reduction;
            let left_0 = Bf16::from_bits(memory::volatile_load(a, left_index_0)).to_f32();
            let left_index_1 = left_index_0 + 1;
            let left_1 = Bf16::from_bits(memory::volatile_load(a, left_index_1)).to_f32();
            let left_index_2 = left_index_1 + 1;
            let left_2 = Bf16::from_bits(memory::volatile_load(a, left_index_2)).to_f32();
            let left_index_3 = left_index_2 + 1;
            let left_3 = Bf16::from_bits(memory::volatile_load(a, left_index_3)).to_f32();
            accumulator_0 = accumulator_0 + left_0 * right_0;
            accumulator_0 = accumulator_0 + left_1 * right_1;
            accumulator_0 = accumulator_0 + left_2 * right_2;
            accumulator_0 = accumulator_0 + left_3 * right_3;
        }
        if active_1 {
            let left_index_0 = row_1 * k + reduction;
            let left_0 = Bf16::from_bits(memory::volatile_load(a, left_index_0)).to_f32();
            let left_index_1 = left_index_0 + 1;
            let left_1 = Bf16::from_bits(memory::volatile_load(a, left_index_1)).to_f32();
            let left_index_2 = left_index_1 + 1;
            let left_2 = Bf16::from_bits(memory::volatile_load(a, left_index_2)).to_f32();
            let left_index_3 = left_index_2 + 1;
            let left_3 = Bf16::from_bits(memory::volatile_load(a, left_index_3)).to_f32();
            accumulator_1 = accumulator_1 + left_0 * right_0;
            accumulator_1 = accumulator_1 + left_1 * right_1;
            accumulator_1 = accumulator_1 + left_2 * right_2;
            accumulator_1 = accumulator_1 + left_3 * right_3;
        }
        if active_2 {
            let left_index_0 = row_2 * k + reduction;
            let left_0 = Bf16::from_bits(memory::volatile_load(a, left_index_0)).to_f32();
            let left_index_1 = left_index_0 + 1;
            let left_1 = Bf16::from_bits(memory::volatile_load(a, left_index_1)).to_f32();
            let left_index_2 = left_index_1 + 1;
            let left_2 = Bf16::from_bits(memory::volatile_load(a, left_index_2)).to_f32();
            let left_index_3 = left_index_2 + 1;
            let left_3 = Bf16::from_bits(memory::volatile_load(a, left_index_3)).to_f32();
            accumulator_2 = accumulator_2 + left_0 * right_0;
            accumulator_2 = accumulator_2 + left_1 * right_1;
            accumulator_2 = accumulator_2 + left_2 * right_2;
            accumulator_2 = accumulator_2 + left_3 * right_3;
        }
        if active_3 {
            let left_index_0 = row_3 * k + reduction;
            let left_0 = Bf16::from_bits(memory::volatile_load(a, left_index_0)).to_f32();
            let left_index_1 = left_index_0 + 1;
            let left_1 = Bf16::from_bits(memory::volatile_load(a, left_index_1)).to_f32();
            let left_index_2 = left_index_1 + 1;
            let left_2 = Bf16::from_bits(memory::volatile_load(a, left_index_2)).to_f32();
            let left_index_3 = left_index_2 + 1;
            let left_3 = Bf16::from_bits(memory::volatile_load(a, left_index_3)).to_f32();
            accumulator_3 = accumulator_3 + left_0 * right_0;
            accumulator_3 = accumulator_3 + left_1 * right_1;
            accumulator_3 = accumulator_3 + left_2 * right_2;
            accumulator_3 = accumulator_3 + left_3 * right_3;
        }
        reduction_wide += 4;
    }
    if active_0 {
        let Some(output) = c.get_tiled_2d_mut(&tile, 0, m, n, n) else {
            fe2o3_device::trap();
        };
        if beta_one {
            accumulator_0 = accumulator_0 + Bf16::from_bits(*output).to_f32();
        }
        *output = Bf16::from_f32(accumulator_0).to_bits();
    }
    if active_1 {
        let Some(output) = c.get_tiled_2d_mut(&tile, 1, m, n, n) else {
            fe2o3_device::trap();
        };
        if beta_one {
            accumulator_1 = accumulator_1 + Bf16::from_bits(*output).to_f32();
        }
        *output = Bf16::from_f32(accumulator_1).to_bits();
    }
    if active_2 {
        let Some(output) = c.get_tiled_2d_mut(&tile, 2, m, n, n) else {
            fe2o3_device::trap();
        };
        if beta_one {
            accumulator_2 = accumulator_2 + Bf16::from_bits(*output).to_f32();
        }
        *output = Bf16::from_f32(accumulator_2).to_bits();
    }
    if active_3 {
        let Some(output) = c.get_tiled_2d_mut(&tile, 3, m, n, n) else {
            fe2o3_device::trap();
        };
        if beta_one {
            accumulator_3 = accumulator_3 + Bf16::from_bits(*output).to_f32();
        }
        *output = Bf16::from_f32(accumulator_3).to_bits();
    }
}

#[kernel(
    typed,
    launch(
        required = [64, 1, 1],
        max = [64, 1, 1],
        max_grid = [131072, 1, 1]
    )
)]
pub fn ferric_qwen3_token_embedding_bf16_copy_v1(
    tokens: &[u32],
    weight: &[u16],
    mut output: WriteOnlyDisjointSlice<u16>,
    rows: u32,
    hidden: u32,
    vocabulary: u32,
) {
    let profile_is_admitted = vocabulary == QWEN3_VOCABULARY_SIZE_V1
        && (((rows == 1
            || rows == 5
            || rows == 8
            || rows == 9
            || rows == 17
            || rows == 32
            || rows == 40
            || rows == 128
            || rows == 512
            || rows == 1_024
            || rows == 2_048)
            && hidden == 4_096)
            || ((rows == 1
                || rows == 4
                || rows == 8
                || rows == 16
                || rows == 32
                || rows == 128
                || rows == 512
                || rows == 1_024
                || rows == 2_048)
                && hidden == 1_024));
    if !profile_is_admitted {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    let hidden = hidden as usize;
    let vocabulary = vocabulary as usize;
    if rows == 0 || hidden == 0 || vocabulary == 0 {
        fe2o3_device::trap();
    }
    if tokens.len() != rows || weight.len() != vocabulary * hidden || output.len() != rows * hidden
    {
        fe2o3_device::trap();
    }
    let launch_extent = (thread::grid_dim_x() as usize) * (thread::block_dim_x() as usize);
    if launch_extent != output.len() {
        fe2o3_device::trap();
    }

    let index = thread::index_1d();
    let output_index = index.get();
    if output_index >= output.len() {
        return;
    }
    let (row, column) = if hidden == 4_096 {
        (output_index / 4_096, output_index % 4_096)
    } else if hidden == 1_024 {
        (output_index / 1_024, output_index % 1_024)
    } else {
        fe2o3_device::trap();
    };
    let token = memory::volatile_load(tokens, row);
    if token >= QWEN3_VOCABULARY_SIZE_V1 {
        fe2o3_device::trap();
    }
    let weight_index_u64 = if hidden == 4_096 {
        if column >= 4_096 {
            fe2o3_device::trap();
        }
        ((token as u64) * 4_096_u64) | column as u64
    } else if hidden == 1_024 {
        if column >= 1_024 {
            fe2o3_device::trap();
        }
        ((token as u64) * 1_024_u64) | column as u64
    } else {
        fe2o3_device::trap();
    };
    let weight_index = weight_index_u64 as usize;
    if weight_index >= weight.len() {
        fe2o3_device::trap();
    }
    if !output.write(index, memory::volatile_load(weight, weight_index)) {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_profile_classifiers_accept_boundary_members() {
        assert!(qwen3_gemm_reference_profile_is_admitted_v1(
            9, 151_936, 4_096, 0
        ));
        assert!(qwen3_gemm_reference_profile_is_admitted_v1(
            8,
            1_024,
            2_048,
            1_065_353_216
        ));
        assert!(qwen3_gemm_vectorized_profile_is_admitted_v1(
            2_048,
            4_096,
            12_288,
            1_065_353_216
        ));
        assert!(qwen3_token_embedding_profile_is_admitted_v1(
            2_048, 4_096, 151_936
        ));
    }

    #[test]
    fn finite_profile_classifiers_reject_cross_role_and_unknown_shapes() {
        assert!(!qwen3_gemm_reference_profile_is_admitted_v1(
            5, 2_048, 1_024, 0
        ));
        assert!(!qwen3_gemm_vectorized_profile_is_admitted_v1(
            40,
            1_024,
            2_048,
            1_065_353_216
        ));
        assert!(!qwen3_gemm_vectorized_profile_is_admitted_v1(
            2_048, 4_096, 4_096, 7
        ));
        assert!(!qwen3_token_embedding_profile_is_admitted_v1(
            40, 1_024, 151_936
        ));
    }

    #[test]
    fn flattened_grid_maxima_cover_the_largest_profiles_exactly() {
        assert_eq!((151_936_usize / 16) * (2_048_usize / 16), 1_215_488);
        assert_eq!((2_048_usize * 4_096).div_ceil(64), 131_072);
        assert_eq!(QWEN3_GEMM_MAX_GRID_WORKGROUPS_V1, 1_215_488);
        assert_eq!(QWEN3_TOKEN_EMBEDDING_MAX_GRID_WORKGROUPS_V1, 131_072);
    }

    #[test]
    fn sequential_matrix_indices_preserve_axis_specific_offsets() {
        for row in [0_usize, 3, 127] {
            for stride in [1_usize, 16, 4_096] {
                for column in [0_usize, 5, 31] {
                    let mut row_offset_index = row * stride + column;
                    let mut column_offset_index = row * stride + column;
                    for offset in 0..4 {
                        assert_eq!(row_offset_index, (row + offset) * stride + column);
                        assert_eq!(column_offset_index, row * stride + column + offset);
                        if offset < 3 {
                            row_offset_index += stride;
                            column_offset_index += 1;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sequential_matrix_indices_equal_checked_arithmetic_for_every_admitted_profile_maximum() {
        const TARGET_SHAPES: &[(usize, usize)] = &[
            (4_096, 4_096),
            (1_024, 4_096),
            (12_288, 4_096),
            (4_096, 12_288),
            (151_936, 4_096),
        ];
        const DRAFT_SHAPES: &[(usize, usize)] = &[
            (2_048, 1_024),
            (1_024, 1_024),
            (1_024, 2_048),
            (3_072, 1_024),
            (1_024, 3_072),
            (151_936, 1_024),
        ];
        const TARGET_ROWS: &[usize] = &[1, 5, 8, 9, 17, 32, 40, 128, 512, 1_024, 2_048];
        const DRAFT_ROWS: &[usize] = &[1, 4, 8, 16, 32, 128, 512, 1_024, 2_048];

        for (shapes, row_counts) in [
            (TARGET_SHAPES, TARGET_ROWS),
            (DRAFT_SHAPES, DRAFT_ROWS),
        ] {
            for &(n, k) in shapes {
                for &m in row_counts {
                    let row = m - 1;
                    let column = n - 1;
                    let reduction = k - 4;
                    let reference_reduction = k - 1;
                    assert!(row <= 2_047);
                    assert!(n <= 151_936);
                    assert!(k <= 12_288);
                    assert!(column <= 151_935);
                    assert!(reference_reduction <= 12_287);

                    let checked_reference_b = reference_reduction
                        .checked_mul(n)
                        .and_then(|value| value.checked_add(column))
                        .expect("admitted reference B index must fit usize");
                    let checked_reference_a = row
                        .checked_mul(k)
                        .and_then(|value| value.checked_add(reference_reduction))
                        .expect("admitted reference A index must fit usize");
                    assert_eq!(reference_reduction * n + column, checked_reference_b);
                    assert_eq!(row * k + reference_reduction, checked_reference_a);
                    assert!(checked_reference_b < k * n);
                    assert!(checked_reference_a < m * k);

                    let mut sequential_b = reduction * n + column;
                    let mut sequential_a = row * k + reduction;
                    for offset in 0..4 {
                        let checked_b = reduction
                            .checked_add(offset)
                            .and_then(|value| value.checked_mul(n))
                            .and_then(|value| value.checked_add(column))
                            .expect("admitted B index must fit usize");
                        let checked_a = row
                            .checked_mul(k)
                            .and_then(|value| value.checked_add(reduction))
                            .and_then(|value| value.checked_add(offset))
                            .expect("admitted A index must fit usize");
                        assert_eq!(sequential_b, checked_b);
                        assert_eq!(sequential_a, checked_a);
                        assert!(checked_b < k * n);
                        assert!(checked_a < m * k);
                        if offset < 3 {
                            sequential_b += n;
                            sequential_a += 1;
                        }
                    }
                }
            }
        }
    }
}
