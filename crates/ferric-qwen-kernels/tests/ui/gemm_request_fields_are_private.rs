use ferric_qwen_kernels::gemm::InertQwen3GemmWorkerRequestV1;

fn inspect(value: InertQwen3GemmWorkerRequestV1) {
    let InertQwen3GemmWorkerRequestV1 { prepared } = value;
    let _ = prepared;
}

fn main() {}
