use ferric_qwen_kernels::rope_kv::PreparedQwen3RopeKvKernelV1;

fn expose(prepared: PreparedQwen3RopeKvKernelV1) {
    let _ = prepared.compiler_handoff;
}

fn main() {}
