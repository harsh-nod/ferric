use fe2o3_device::{WriteOnlyDisjointSlice, kernel, memory, thread};

/// Independent token IDs select rows of the unchanged full BF16 embedding.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [512, 1, 1]))]
pub fn ferric_qwen3_draft_batch32_embedding_bf16_v10(
    tokens: &[u32],
    weight: &[u16],
    mut output: WriteOnlyDisjointSlice<u16>,
    rows: u32,
) {
    if rows == 0 || rows > 32 {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    if tokens.len() < rows
        || tokens.len() > 32
        || weight.len() != 151936 * 1024
        || output.len() < rows * 1024
        || output.len() > 32 * 1024
        || thread::launch_extent_1d() != rows * 1024
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let row = raw / 1024;
    let column = raw % 1024;
    if row < rows {
    } else {
        fe2o3_device::trap();
    }
    let token = memory::volatile_load(tokens, row) as usize;
    if token < 151936 {
    } else {
        fe2o3_device::trap();
    }
    let value = memory::volatile_load(weight, token * 1024 + column);
    if !output.write(invocation, value) {
        fe2o3_device::trap();
    }
}
