use ferric_qwen_kernels::logits::PreparedQwen3LogitsKernelV1;

fn duplicate(value: PreparedQwen3LogitsKernelV1) {
    let _duplicate = value.clone();
}

fn main() {}
