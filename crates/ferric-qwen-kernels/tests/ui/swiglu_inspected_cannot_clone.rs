use ferric_qwen_kernels::swiglu::InspectedQwen3SwiGluKernelV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<InspectedQwen3SwiGluKernelV1>();
}
