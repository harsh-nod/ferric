use ferric_qwen_kernels::swiglu::PreparedQwen3SwiGluKernelV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<PreparedQwen3SwiGluKernelV1>();
}
