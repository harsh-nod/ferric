use ferric_qwen_kernels::paged_decode::PreparedQwen3PagedDecodeKernelV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<PreparedQwen3PagedDecodeKernelV1>();
}
