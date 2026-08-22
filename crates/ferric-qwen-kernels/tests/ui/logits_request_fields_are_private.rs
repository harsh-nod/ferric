use ferric_qwen_kernels::logits::InertQwen3LogitsWorkerRequestV1;

fn inspect(value: InertQwen3LogitsWorkerRequestV1) {
    let InertQwen3LogitsWorkerRequestV1 { prepared } = value;
    let _ = prepared;
}

fn main() {}
