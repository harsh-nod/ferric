use ferric_qwen_kernels::swiglu::{
    CheckedQwen3SwiGluLaunchV1, InertQwen3SwiGluWorkerEvidenceV1, InertQwen3SwiGluWorkerRequestV1,
    InspectedQwen3SwiGluKernelV1, PreparedQwen3SwiGluKernelV1,
};

fn prepared(value: PreparedQwen3SwiGluKernelV1) {
    let PreparedQwen3SwiGluKernelV1 { catalog, .. } = value;
    let _ = catalog;
}

fn request(value: InertQwen3SwiGluWorkerRequestV1) {
    let InertQwen3SwiGluWorkerRequestV1 { prepared } = value;
    let _ = prepared;
}

fn evidence(value: InertQwen3SwiGluWorkerEvidenceV1) {
    let InertQwen3SwiGluWorkerEvidenceV1 { worker, .. } = value;
    let _ = worker;
}

fn inspected(value: InspectedQwen3SwiGluKernelV1) {
    let InspectedQwen3SwiGluKernelV1 { loader_plan, .. } = value;
    let _ = loader_plan;
}

fn checked(value: CheckedQwen3SwiGluLaunchV1) {
    let CheckedQwen3SwiGluLaunchV1 { profile, .. } = value;
    let _ = profile;
}

fn main() {}
