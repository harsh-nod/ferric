use fe2o3_device::{WriteOnlyDisjointSlice, kernel, memory, thread};

/// Fills one host-validated exclusive TP1 KV page from sixteen complete rows.
/// Page/slot agreement, row permutation and immutable-prefix COW are host contracts.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [256, 1, 1]))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_prefill16_kv_copy_bf16_v27(
    key: &[u16],
    value: &[u16],
    mut key_page: WriteOnlyDisjointSlice<u16>,
    mut value_page: WriteOnlyDisjointSlice<u16>,
    first_position: u32,
    physical_page: u32,
    physical_pages: u32,
    last_row_first: u32,
) {
    if first_position > 8176
        || first_position & 15 != 0
        || physical_pages == 0
        || physical_pages > 512
        || physical_page >= physical_pages
        || last_row_first > 1
        || key.len() != 16384
        || value.len() != 16384
        || key_page.len() != 16384
        || value_page.len() != 16384
        || thread::launch_extent_1d() != 16384
    {
        fe2o3_device::trap();
    }
    let key_invocation = thread::index_1d();
    let index = key_invocation.get();
    if index < 16384 {
    } else {
        fe2o3_device::trap();
    }
    let row = index / 1024;
    let source_row = if last_row_first == 0 {
        row
    } else if row == 15 {
        0
    } else {
        row + 1
    };
    if source_row < 16 {
    } else {
        fe2o3_device::trap();
    }
    let source_index = source_row * 1024 + index % 1024;
    if source_index < 16384 {
    } else {
        fe2o3_device::trap();
    }
    let key_bits = memory::volatile_load(key, source_index);
    let value_bits = memory::volatile_load(value, source_index);
    if !key_page.write(key_invocation, key_bits) {
        fe2o3_device::trap();
    }
    if !value_page.write(thread::index_1d(), value_bits) {
        fe2o3_device::trap();
    }
}
