use ferric_qwen_kernels::logits::InspectedQwen3LogitsKernelV1;

fn inspect(value: InspectedQwen3LogitsKernelV1) {
    let InspectedQwen3LogitsKernelV1 { catalog, .. } = value;
    let _ = catalog;
}

fn main() {}
