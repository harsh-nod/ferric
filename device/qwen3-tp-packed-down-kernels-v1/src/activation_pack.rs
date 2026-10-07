use fe2o3_device::{Index1D, StridedReadView2D, WriteOnlyDisjointSlice, kernel, thread};

/// C1 down activation payload packing into real, separately allocated u32 storage.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [96, 1, 1]))]
pub fn ferric_qwen3_c1_down_activation_pack_u32_r1(
    source: &[u16],
    mut output: WriteOnlyDisjointSlice<u32, Index1D>,
    rows: u32,
    k: u32,
) {
    if rows != 1 || k != 12288 {
        fe2o3_device::trap();
    }
    if source.len() < 12288
        || source.len() > 32 * 12288
        || output.len() < 6144
        || output.len() > 32 * 6144
        || thread::launch_extent_1d() != 6144
    {
        fe2o3_device::trap();
    }
    let Ok(view) = StridedReadView2D::from_shared_slice(source, 0, 1, 12288, 12288) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let word = invocation.get();
    if word < 6144 {
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
