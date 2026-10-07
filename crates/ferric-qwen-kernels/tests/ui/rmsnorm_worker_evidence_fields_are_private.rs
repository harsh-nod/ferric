use ferric_qwen_kernels::rmsnorm::InertQwen3RmsNormWorkerEvidenceV1;

fn expose(evidence: InertQwen3RmsNormWorkerEvidenceV1) {
    let _ = evidence.worker;
}

fn main() {}
