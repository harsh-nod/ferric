#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel attribute emits an internal helper module.

//! Engineering-only, fixed-shape complete decoder-layer numerical baseline.
//!
//! This is ordinary attributed Rust source for production fe2o3 compilation,
//! not a production-qualified model backend. Each workitem independently owns
//! one complete tiny decoder request and forty disjoint checkpoint outputs.
//! Two 128-workitem workgroups exercise multiple workgroups and two wave64
//! waves per workgroup without requiring atomics or cross-workgroup barriers.
//! No MFMA, cooperative tile, paged-KV runtime, or dynamic scheduler is claimed.

use fe2o3_device::{Blocked, Index1D, Math, WriteOnlyDisjointSlice, kernel, thread};

pub const KERNEL_SYMBOL: &str = "ferric_gfx950_decoder_layer_f32_v1";
pub const REQUESTS: usize = 256;
pub const HIDDEN: usize = 4;
pub const QUERY_HEADS: usize = 2;
pub const KV_HEADS: usize = 1;
pub const HEAD_DIM: usize = 2;
pub const INTERMEDIATE: usize = 4;
pub const PREFIX_TOKENS: usize = 2;
pub const INPUTS_PER_REQUEST: usize = 12;
pub const CHECKPOINTS_PER_REQUEST: usize = 40;
pub const INPUT_FLOATS: usize = REQUESTS * INPUTS_PER_REQUEST;
pub const WEIGHT_FLOATS: usize = 110;
pub const OUTPUT_FLOATS: usize = REQUESTS * CHECKPOINTS_PER_REQUEST;
pub const EXPLICIT_KERNARG_BYTES: usize = 48;
/// AQL dimensions are workitems, not workgroup counts.
pub const AQL_GRID: [u32; 3] = [256, 1, 1];
pub const WORKGROUP: [u32; 3] = [128, 1, 1];
pub const RMS_EPSILON: f32 = 1e-6;
pub const ATTENTION_SCALE: f32 = core::f32::consts::FRAC_1_SQRT_2;

/// Packed weight starts: input norm, Q/K/V, Q/K norm, O, post norm,
/// gate/up/down, cosine, sine, and final end offset.
pub const WEIGHT_OFFSETS: [usize; 14] = [0, 4, 20, 28, 36, 38, 40, 56, 60, 76, 92, 108, 109, 110];

/// Checkpoint starts: input norm, rotated Q/K, V, attention, attention residual,
/// post norm, gate/up, activated MLP, final residual, and final end offset.
pub const CHECKPOINT_OFFSETS: [usize; 12] = [0, 4, 8, 10, 12, 16, 20, 24, 28, 32, 36, 40];

// Expansion keeps fixed scalar arithmetic in the kernel body. There is no
// external helper call or hidden host numerical fallback.
macro_rules! dot4 {
    ($weights:ident, $offset:expr, $x0:expr, $x1:expr, $x2:expr, $x3:expr) => {
        (($x0 * $weights[$offset] + $x1 * $weights[$offset + 1]) + $x2 * $weights[$offset + 2])
            + $x3 * $weights[$offset + 3]
    };
}

macro_rules! stable_silu {
    ($math:ident, $value:expr) => {{
        let nonnegative = $value >= 0.0;
        let argument = if nonnegative { -$value } else { $value };
        let exponential = $math.exp_f32(argument);
        let numerator = if nonnegative { 1.0 } else { exponential };
        $value * (numerator / (1.0 + exponential))
    }};
}

macro_rules! checkpoint {
    ($output:ident, $block:ident, $offset:expr, $value:expr) => {
        if !$output.write_block(&$block, $offset, $value) {
            fe2o3_device::trap();
        }
    };
}

/// Execute one complete fixed-shape decoder layer per workitem.
///
/// Inputs: 256 records of [hidden4, rotated-prefix-keys4, prefix-values4].
/// Weights: the 110-float row-major layout documented by WEIGHT_OFFSETS.
/// Outputs: 256 records of the 40 checkpoint floats in CHECKPOINT_OFFSETS.
///
/// All inputs, weights, accumulations, intermediates and outputs are F32.
/// RMSNorm uses epsilon 1e-6. There are two query heads sharing one KV head.
/// Current Q/K receive per-head RMSNorm and split-half D2 RoPE. Prefix keys
/// are already position-rotated. The supplied cosine/sine are for position 2.
/// Attention uses all two prefix tokens plus current K/V, stable max-subtracted
/// softmax, and scale 1/sqrt(2). Projections have no bias. MLP is
/// down(SiLU(gate(postnorm)) * up(postnorm)) plus the attention residual.
///
/// The compiler's admitted F32 contraction/transcendental policy determines
/// device rounding; reference comparisons must use a declared error tolerance.
/// This diagnostic contract is not the BF16 Qwen production numerical policy.
#[kernel(
    typed,
    launch(required = [128, 1, 1], max = [128, 1, 1], max_grid = [2, 1, 1])
)]
pub fn ferric_gfx950_decoder_layer_f32_v1(
    inputs: &[f32],
    weights: &[f32],
    mut output: WriteOnlyDisjointSlice<f32, Blocked<Index1D, 1, 40>>,
) {
    // Validate all extents before any read or write. The harness additionally
    // validates the exact AQL grid, finite test inputs, and metadata identity.
    if inputs.len() != INPUT_FLOATS
        || weights.len() != WEIGHT_FLOATS
        || output.len() != OUTPUT_FLOATS
    {
        fe2o3_device::trap();
    }
    let index = thread::index_1d();
    let request = index.get();
    if request >= REQUESTS {
        return;
    }
    let Some(block) = index.checked_block::<1, 40>() else {
        fe2o3_device::trap();
    };
    let base = request * INPUTS_PER_REQUEST;
    let math = Math::current();

    // Input RMSNorm, followed by independent Q/K/V row-major projections.
    let h0 = inputs[base];
    let h1 = inputs[base + 1];
    let h2 = inputs[base + 2];
    let h3 = inputs[base + 3];
    let input_mean_square = ((h0 * h0 + h1 * h1) + h2 * h2) + h3 * h3;
    let input_inverse = 1.0 / math.sqrt_f32(input_mean_square * 0.25 + RMS_EPSILON);
    let n0 = (h0 * input_inverse) * weights[0];
    let n1 = (h1 * input_inverse) * weights[1];
    let n2 = (h2 * input_inverse) * weights[2];
    let n3 = (h3 * input_inverse) * weights[3];
    let q0 = dot4!(weights, 4, n0, n1, n2, n3);
    let q1 = dot4!(weights, 8, n0, n1, n2, n3);
    let q2 = dot4!(weights, 12, n0, n1, n2, n3);
    let q3 = dot4!(weights, 16, n0, n1, n2, n3);
    let k0 = dot4!(weights, 20, n0, n1, n2, n3);
    let k1 = dot4!(weights, 24, n0, n1, n2, n3);
    let v0 = dot4!(weights, 28, n0, n1, n2, n3);
    let v1 = dot4!(weights, 32, n0, n1, n2, n3);

    // Qwen-style per-head Q/K RMSNorm and D2 split-half position rotation.
    let q_inverse0 = 1.0 / math.sqrt_f32((q0 * q0 + q1 * q1) * 0.5 + RMS_EPSILON);
    let q_inverse1 = 1.0 / math.sqrt_f32((q2 * q2 + q3 * q3) * 0.5 + RMS_EPSILON);
    let k_inverse = 1.0 / math.sqrt_f32((k0 * k0 + k1 * k1) * 0.5 + RMS_EPSILON);
    let qn0 = (q0 * q_inverse0) * weights[36];
    let qn1 = (q1 * q_inverse0) * weights[37];
    let qn2 = (q2 * q_inverse1) * weights[36];
    let qn3 = (q3 * q_inverse1) * weights[37];
    let kn0 = (k0 * k_inverse) * weights[38];
    let kn1 = (k1 * k_inverse) * weights[39];
    let cosine = weights[108];
    let sine = weights[109];
    let qr0 = qn0 * cosine - qn1 * sine;
    let qr1 = qn1 * cosine + qn0 * sine;
    let qr2 = qn2 * cosine - qn3 * sine;
    let qr3 = qn3 * cosine + qn2 * sine;
    let kr0 = kn0 * cosine - kn1 * sine;
    let kr1 = kn1 * cosine + kn0 * sine;

    // Both query heads attend to the same initialized two-token KV prefix,
    // then to the current rotated K and current V. All three tokens are causal.
    let pk00 = inputs[base + 4];
    let pk01 = inputs[base + 5];
    let pk10 = inputs[base + 6];
    let pk11 = inputs[base + 7];
    let pv00 = inputs[base + 8];
    let pv01 = inputs[base + 9];
    let pv10 = inputs[base + 10];
    let pv11 = inputs[base + 11];
    let score00 = (qr0 * pk00 + qr1 * pk01) * ATTENTION_SCALE;
    let score01 = (qr0 * pk10 + qr1 * pk11) * ATTENTION_SCALE;
    let score02 = (qr0 * kr0 + qr1 * kr1) * ATTENTION_SCALE;
    let max01 = if score00 > score01 { score00 } else { score01 };
    let max0 = if max01 > score02 { max01 } else { score02 };
    let exponential00 = math.exp_f32(score00 - max0);
    let exponential01 = math.exp_f32(score01 - max0);
    let exponential02 = math.exp_f32(score02 - max0);
    let denominator0 = (exponential00 + exponential01) + exponential02;
    let probability00 = exponential00 / denominator0;
    let probability01 = exponential01 / denominator0;
    let probability02 = exponential02 / denominator0;
    let a0 = (probability00 * pv00 + probability01 * pv10) + probability02 * v0;
    let a1 = (probability00 * pv01 + probability01 * pv11) + probability02 * v1;

    let score10 = (qr2 * pk00 + qr3 * pk01) * ATTENTION_SCALE;
    let score11 = (qr2 * pk10 + qr3 * pk11) * ATTENTION_SCALE;
    let score12 = (qr2 * kr0 + qr3 * kr1) * ATTENTION_SCALE;
    let max11 = if score10 > score11 { score10 } else { score11 };
    let max1 = if max11 > score12 { max11 } else { score12 };
    let exponential10 = math.exp_f32(score10 - max1);
    let exponential11 = math.exp_f32(score11 - max1);
    let exponential12 = math.exp_f32(score12 - max1);
    let denominator1 = (exponential10 + exponential11) + exponential12;
    let probability10 = exponential10 / denominator1;
    let probability11 = exponential11 / denominator1;
    let probability12 = exponential12 / denominator1;
    let a2 = (probability10 * pv00 + probability11 * pv10) + probability12 * v0;
    let a3 = (probability10 * pv01 + probability11 * pv11) + probability12 * v1;

    // Attention output projection and residual, followed by the pre-MLP norm.
    let r0 = dot4!(weights, 40, a0, a1, a2, a3) + h0;
    let r1 = dot4!(weights, 44, a0, a1, a2, a3) + h1;
    let r2 = dot4!(weights, 48, a0, a1, a2, a3) + h2;
    let r3 = dot4!(weights, 52, a0, a1, a2, a3) + h3;
    let post_mean_square = ((r0 * r0 + r1 * r1) + r2 * r2) + r3 * r3;
    let post_inverse = 1.0 / math.sqrt_f32(post_mean_square * 0.25 + RMS_EPSILON);
    let p0 = (r0 * post_inverse) * weights[56];
    let p1 = (r1 * post_inverse) * weights[57];
    let p2 = (r2 * post_inverse) * weights[58];
    let p3 = (r3 * post_inverse) * weights[59];

    // SwiGLU's stable sigmoid avoids exp overflow for finite negative gates.
    let gate0 = dot4!(weights, 60, p0, p1, p2, p3);
    let gate1 = dot4!(weights, 64, p0, p1, p2, p3);
    let gate2 = dot4!(weights, 68, p0, p1, p2, p3);
    let gate3 = dot4!(weights, 72, p0, p1, p2, p3);
    let up0 = dot4!(weights, 76, p0, p1, p2, p3);
    let up1 = dot4!(weights, 80, p0, p1, p2, p3);
    let up2 = dot4!(weights, 84, p0, p1, p2, p3);
    let up3 = dot4!(weights, 88, p0, p1, p2, p3);
    let activated0 = stable_silu!(math, gate0) * up0;
    let activated1 = stable_silu!(math, gate1) * up1;
    let activated2 = stable_silu!(math, gate2) * up2;
    let activated3 = stable_silu!(math, gate3) * up3;
    let result0 = dot4!(weights, 92, activated0, activated1, activated2, activated3) + r0;
    let result1 = dot4!(weights, 96, activated0, activated1, activated2, activated3) + r1;
    let result2 = dot4!(weights, 100, activated0, activated1, activated2, activated3) + r2;
    let result3 = dot4!(weights, 104, activated0, activated1, activated2, activated3) + r3;

    // Retain each stage to make numerical diagnosis independent of final-output
    // cancellation. One typed blocked witness owns every write for this request.
    checkpoint!(output, block, 0, n0);
    checkpoint!(output, block, 1, n1);
    checkpoint!(output, block, 2, n2);
    checkpoint!(output, block, 3, n3);
    checkpoint!(output, block, 4, qr0);
    checkpoint!(output, block, 5, qr1);
    checkpoint!(output, block, 6, qr2);
    checkpoint!(output, block, 7, qr3);
    checkpoint!(output, block, 8, kr0);
    checkpoint!(output, block, 9, kr1);
    checkpoint!(output, block, 10, v0);
    checkpoint!(output, block, 11, v1);
    checkpoint!(output, block, 12, a0);
    checkpoint!(output, block, 13, a1);
    checkpoint!(output, block, 14, a2);
    checkpoint!(output, block, 15, a3);
    checkpoint!(output, block, 16, r0);
    checkpoint!(output, block, 17, r1);
    checkpoint!(output, block, 18, r2);
    checkpoint!(output, block, 19, r3);
    checkpoint!(output, block, 20, p0);
    checkpoint!(output, block, 21, p1);
    checkpoint!(output, block, 22, p2);
    checkpoint!(output, block, 23, p3);
    checkpoint!(output, block, 24, gate0);
    checkpoint!(output, block, 25, gate1);
    checkpoint!(output, block, 26, gate2);
    checkpoint!(output, block, 27, gate3);
    checkpoint!(output, block, 28, up0);
    checkpoint!(output, block, 29, up1);
    checkpoint!(output, block, 30, up2);
    checkpoint!(output, block, 31, up3);
    checkpoint!(output, block, 32, activated0);
    checkpoint!(output, block, 33, activated1);
    checkpoint!(output, block, 34, activated2);
    checkpoint!(output, block, 35, activated3);
    checkpoint!(output, block, 36, result0);
    checkpoint!(output, block, 37, result1);
    checkpoint!(output, block, 38, result2);
    checkpoint!(output, block, 39, result3);
}
