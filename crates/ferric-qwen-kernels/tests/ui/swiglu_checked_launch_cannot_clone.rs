use ferric_qwen_kernels::swiglu::CheckedQwen3SwiGluLaunchV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<CheckedQwen3SwiGluLaunchV1>();
}
