use ferric_qwen_kernels::prefill::CheckedQwen3PrefillLaunchV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<CheckedQwen3PrefillLaunchV1>();
}
