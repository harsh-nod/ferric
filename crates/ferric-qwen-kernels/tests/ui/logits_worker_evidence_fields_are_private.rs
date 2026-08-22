use ferric_qwen_kernels::logits::InertQwen3LogitsWorkerEvidenceV1;

fn inspect(value: InertQwen3LogitsWorkerEvidenceV1) {
    let InertQwen3LogitsWorkerEvidenceV1 { worker, .. } = value;
    let _ = worker;
}

fn main() {}
