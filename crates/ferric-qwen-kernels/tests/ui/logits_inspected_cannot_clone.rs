use ferric_qwen_kernels::logits::InspectedQwen3LogitsKernelV1;

fn duplicate(value: InspectedQwen3LogitsKernelV1) {
    let _duplicate = value.clone();
}

fn main() {}
