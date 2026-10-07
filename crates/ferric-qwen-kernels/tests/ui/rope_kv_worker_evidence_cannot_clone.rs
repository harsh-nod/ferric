use ferric_qwen_kernels::rope_kv::InertQwen3RopeKvWorkerEvidenceV1;

fn duplicate(evidence: InertQwen3RopeKvWorkerEvidenceV1) {
    let _duplicate = evidence.clone();
}

fn main() {}
