use ferric_qwen_kernels::rope_kv::InertQwen3RopeKvWorkerEvidenceV1;

fn expose(evidence: InertQwen3RopeKvWorkerEvidenceV1) {
    let _ = evidence.worker;
}

fn main() {}
