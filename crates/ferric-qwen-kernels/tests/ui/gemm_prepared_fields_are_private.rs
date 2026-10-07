use ferric_qwen_kernels::gemm::PreparedQwen3GemmKernelV1;

fn inspect(value: PreparedQwen3GemmKernelV1) {
    let PreparedQwen3GemmKernelV1 { catalog, .. } = value;
    let _ = catalog;
}

fn main() {}
