use ferric_qwen_kernels::paged_decode::{
    CheckedQwen3PagedDecodeLaunchV1, InertQwen3PagedDecodeWorkerEvidenceV1,
    InertQwen3PagedDecodeWorkerRequestV1, InspectedQwen3PagedDecodeKernelV1,
    PreparedQwen3PagedDecodeKernelV1,
};

fn prepared(value: PreparedQwen3PagedDecodeKernelV1) {
    let PreparedQwen3PagedDecodeKernelV1 { catalog, .. } = value;
    let _ = catalog;
}

fn request(value: InertQwen3PagedDecodeWorkerRequestV1) {
    let InertQwen3PagedDecodeWorkerRequestV1 { prepared } = value;
    let _ = prepared;
}

fn evidence(value: InertQwen3PagedDecodeWorkerEvidenceV1) {
    let InertQwen3PagedDecodeWorkerEvidenceV1 { worker, .. } = value;
    let _ = worker;
}

fn inspected(value: InspectedQwen3PagedDecodeKernelV1) {
    let InspectedQwen3PagedDecodeKernelV1 { loader_plan, .. } = value;
    let _ = loader_plan;
}

fn checked(value: CheckedQwen3PagedDecodeLaunchV1) {
    let CheckedQwen3PagedDecodeLaunchV1 { profile, .. } = value;
    let _ = profile;
}

fn main() {}
