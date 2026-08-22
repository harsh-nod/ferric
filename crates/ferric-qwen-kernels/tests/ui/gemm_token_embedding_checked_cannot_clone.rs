use ferric_qwen_kernels::gemm::CheckedQwen3TokenEmbeddingLaunchV1;

fn duplicate(
    binding: CheckedQwen3TokenEmbeddingLaunchV1,
) -> (
    CheckedQwen3TokenEmbeddingLaunchV1,
    CheckedQwen3TokenEmbeddingLaunchV1,
) {
    let copied = binding.clone();
    (binding, copied)
}

fn main() {}
