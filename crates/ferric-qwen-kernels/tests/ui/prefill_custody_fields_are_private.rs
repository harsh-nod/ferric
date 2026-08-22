use ferric_qwen_kernels::prefill::{
    CheckedQwen3PrefillLaunchV1, InertQwen3PrefillWorkerEvidenceV1,
    InertQwen3PrefillWorkerRequestV1, InspectedQwen3PrefillKernelV1, PreparedQwen3PrefillKernelV1,
};

fn prepared(value: PreparedQwen3PrefillKernelV1) {
    let PreparedQwen3PrefillKernelV1 { catalog, .. } = value;
    let _ = catalog;
}

fn request(value: InertQwen3PrefillWorkerRequestV1) {
    let InertQwen3PrefillWorkerRequestV1 { prepared } = value;
    let _ = prepared;
}

fn evidence(value: InertQwen3PrefillWorkerEvidenceV1) {
    let InertQwen3PrefillWorkerEvidenceV1 { worker, .. } = value;
    let _ = worker;
}

fn inspected(value: InspectedQwen3PrefillKernelV1) {
    let InspectedQwen3PrefillKernelV1 { loader_plan, .. } = value;
    let _ = loader_plan;
}

fn checked(value: CheckedQwen3PrefillLaunchV1) {
    let CheckedQwen3PrefillLaunchV1 { profile, .. } = value;
    let _ = profile;
}

fn main() {}
