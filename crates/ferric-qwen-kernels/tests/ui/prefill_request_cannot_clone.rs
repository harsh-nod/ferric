use ferric_qwen_kernels::prefill::InertQwen3PrefillWorkerRequestV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<InertQwen3PrefillWorkerRequestV1>();
}
