use ferric_qwen_kernels::prefill::PreparedQwen3PrefillKernelV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<PreparedQwen3PrefillKernelV1>();
}
