use fe2o3_device::{WriteOnlyDisjointSlice, kernel, memory, thread};

/// Copies one TP1 row to two host-validated, independently owned KV slot views.
/// Page ownership, table agreement and immutable-prefix COW remain host contracts.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [16, 1, 1]))]
pub fn ferric_qwen3_tp_c1_kv_copy_bf16_v19(
    key: &[u16],
    value: &[u16],
    mut key_slot: WriteOnlyDisjointSlice<u16>,
    mut value_slot: WriteOnlyDisjointSlice<u16>,
    position: u32,
    physical_page: u32,
    physical_pages: u32,
) {
    if position >= 8192
        || physical_pages == 0
        || physical_pages > 512
        || physical_page >= physical_pages
        || key.len() != 1024
        || value.len() != 1024
        || key_slot.len() != 1024
        || value_slot.len() != 1024
        || thread::launch_extent_1d() != 1024
    {
        fe2o3_device::trap();
    }
    let key_invocation = thread::index_1d();
    let index = key_invocation.get();
    if index < 1024 {
    } else {
        fe2o3_device::trap();
    }
    let key_bits = memory::volatile_load(key, index);
    let value_bits = memory::volatile_load(value, index);
    if !key_slot.write(key_invocation, key_bits) {
        fe2o3_device::trap();
    }
    if !value_slot.write(thread::index_1d(), value_bits) {
        fe2o3_device::trap();
    }
}
