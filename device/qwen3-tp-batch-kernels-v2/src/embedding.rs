use fe2o3_device::{WriteOnlyDisjointSlice, kernel, memory, thread};

/// Independent token IDs select rows of the unchanged full BF16 embedding.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1024, 1, 1]))]
pub fn ferric_qwen3_tp_batch_embedding_bf16_v2(
    tokens: &[u32],
    weight: &[u16],
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
    if tokens.len() < rows
        || tokens.len() > 16
        || weight.len() != 151936 * 4096
        || output.len() < rows * 4096
        || output.len() > 16 * 4096
        || thread::launch_extent_1d() != rows * 4096
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let row = raw / 4096;
    let column = raw % 4096;
    if row < rows {
    } else {
        fe2o3_device::trap();
    }
    let token = memory::volatile_load(tokens, row) as usize;
    if token < 151936 {
    } else {
        fe2o3_device::trap();
    }
    let value = memory::volatile_load(weight, token * 4096 + column);
    if !output.write(invocation, value) {
        fe2o3_device::trap();
    }
}
