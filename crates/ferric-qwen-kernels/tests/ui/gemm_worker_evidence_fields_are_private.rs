use ferric_qwen_kernels::gemm::InertQwen3GemmWorkerEvidenceV1;

fn inspect(value: InertQwen3GemmWorkerEvidenceV1) {
    let InertQwen3GemmWorkerEvidenceV1 { worker, .. } = value;
    let _ = worker;
}

fn main() {}
