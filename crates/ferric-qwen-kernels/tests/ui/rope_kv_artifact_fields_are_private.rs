use ferric_qwen_kernels::rope_kv::InspectedQwen3RopeKvKernelV1;

fn expose(artifact: InspectedQwen3RopeKvKernelV1) {
    let _ = artifact.worker;
}

fn main() {}
