use ferric_qwen_kernels::gemm::InspectedQwen3GemmKernelV1;

fn inspect(value: InspectedQwen3GemmKernelV1) {
    let InspectedQwen3GemmKernelV1 { catalog, .. } = value;
    let _ = catalog;
}

fn main() {}
