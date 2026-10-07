use ferric_qwen_kernels::gemm::CheckedQwen3TokenEmbeddingLaunchV1;

fn inspect(binding: CheckedQwen3TokenEmbeddingLaunchV1) {
    let _ = binding.profile;
    let _ = binding.buffers;
}

fn main() {}
