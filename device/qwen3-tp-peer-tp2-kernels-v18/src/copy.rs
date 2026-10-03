use fe2o3_device::{WriteOnlyDisjointSlice, kernel, memory, thread};

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1024, 1, 1]))]
pub fn ferric_qwen3_tp_peer_copy_bf16_v4(
    source: &[u16],
    mut output: WriteOnlyDisjointSlice<u16>,
    rows: u32,
) {
    if rows == 0 || rows > 16 {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    let elements = rows * 4096;
    if source.len() != elements
        || output.len() != elements
        || thread::launch_extent_1d() != elements
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let index = invocation.get();
    if index < elements {
    } else {
        fe2o3_device::trap();
    }
    let bits = memory::volatile_load(source, index);
    if !output.write(invocation, bits) {
        fe2o3_device::trap();
    }
}
