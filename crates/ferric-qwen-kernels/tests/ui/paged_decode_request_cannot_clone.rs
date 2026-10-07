use ferric_qwen_kernels::paged_decode::InertQwen3PagedDecodeWorkerRequestV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<InertQwen3PagedDecodeWorkerRequestV1>();
}
