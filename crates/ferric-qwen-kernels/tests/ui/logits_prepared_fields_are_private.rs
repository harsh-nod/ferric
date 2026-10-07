use ferric_qwen_kernels::logits::PreparedQwen3LogitsKernelV1;

fn inspect(value: PreparedQwen3LogitsKernelV1) {
    let PreparedQwen3LogitsKernelV1 { catalog, .. } = value;
    let _ = catalog;
}

fn main() {}
