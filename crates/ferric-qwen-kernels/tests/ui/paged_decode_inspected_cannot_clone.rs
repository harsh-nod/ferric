use ferric_qwen_kernels::paged_decode::InspectedQwen3PagedDecodeKernelV1;

fn require_clone<T: Clone>() {}

fn main() {
    require_clone::<InspectedQwen3PagedDecodeKernelV1>();
}
