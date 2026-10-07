use fe2o3_device::{Index1D, StridedReadView2D, WriteOnlyDisjointSlice, kernel, thread};

/// Exact C1 activation view, packed without changing any BF16 representation.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]))]
pub fn ferric_qwen3_c1_gate_up_activation_pack_u32_v8(
    source: &[u16],
    mut output: WriteOnlyDisjointSlice<u32, Index1D>,
) {
    if source.len() != 4096 || output.len() != 2048 || thread::launch_extent_1d() != 2048 {
        fe2o3_device::trap();
    }
    let Ok(view) = StridedReadView2D::from_shared_slice(source, 0, 1, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let word = invocation.get();
    if word < 2048 {
    } else {
        fe2o3_device::trap();
    }
    let group = word / 64;
    let lane = word % 64;
    let low = group * 128 + lane;
    let low_bits = view.load_or(0, low, 0);
    let high_bits = view.load_or(0, low + 64, 0);
    let packed = (low_bits as u32) | ((high_bits as u32) << 16);
    if !output.write(invocation, packed) {
        fe2o3_device::trap();
    }
}
