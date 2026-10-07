use ferric_qwen_kernels::gemm::{Qwen3ExpectedEmbeddingWeightIdentityV1, Qwen3GemmModelRoleV1};

fn main() {
    let _ = Qwen3ExpectedEmbeddingWeightIdentityV1 {
        role: Qwen3GemmModelRoleV1::Target8B,
        bytes: [7; 32],
    };
}
