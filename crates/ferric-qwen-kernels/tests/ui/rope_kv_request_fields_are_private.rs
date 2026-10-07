use ferric_qwen_kernels::rope_kv::InertQwen3RopeKvWorkerRequestV1;

fn expose(request: InertQwen3RopeKvWorkerRequestV1) {
    let _ = request.prepared;
}

fn main() {}
