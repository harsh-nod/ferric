use ferric_qwen_kernels::paged_decode::CheckedQwen3PagedDecodeLaunchV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<CheckedQwen3PagedDecodeLaunchV1>();
}
