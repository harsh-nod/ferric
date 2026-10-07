use ferric_qwen_kernels::prefill::InspectedQwen3PrefillKernelV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<InspectedQwen3PrefillKernelV1>();
}
