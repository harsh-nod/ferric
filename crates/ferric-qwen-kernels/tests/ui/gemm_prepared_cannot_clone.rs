use ferric_qwen_kernels::gemm::PreparedQwen3GemmKernelV1;

fn clone_prepared(value: PreparedQwen3GemmKernelV1) {
    let _duplicate = value.clone();
}

fn main() {}
