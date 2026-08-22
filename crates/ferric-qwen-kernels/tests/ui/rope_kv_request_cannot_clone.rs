use ferric_qwen_kernels::rope_kv::{
    lower_qwen3_rope_kv_kernel_v1, prepare_qwen3_rope_kv_kernel_v1,
    Qwen3RopeKvSourceBindingsV1,
};

fn main() {
    let prepared = prepare_qwen3_rope_kv_kernel_v1(Qwen3RopeKvSourceBindingsV1::new(
        [1; 32], [2; 32], [3; 32], [4; 32],
    ))
    .unwrap();
    let request = lower_qwen3_rope_kv_kernel_v1(prepared);
    let _duplicate = request.clone();
}
