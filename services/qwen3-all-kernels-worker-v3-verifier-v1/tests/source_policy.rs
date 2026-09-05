//! Source policy for the connected-path Ferric service entrypoint.

const SERVICE_SOURCE: &str = include_str!("../src/service.rs");
const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const README: &str = include_str!("../README.md");

fn connected_path_service_policy(service: &str, library: &str, readme: &str) -> bool {
    let production = service.split("#[cfg(test)]").next().unwrap_or(service);
    let unnamed_marker = "pub fn run_ferric_protected_verifier_session_v2";
    let accepted_marker = "pub fn run_ferric_protected_verifier_accepted_session_v2";
    let accepted_until_marker =
        "pub(crate) fn run_ferric_protected_verifier_accepted_session_until_v2";
    let parameters_marker = "fn service_admission_parameters_v2";
    let core_marker = "fn run_ferric_protected_verifier_post_begin_v2";
    if production.matches(unnamed_marker).count() != 1
        || production.matches(accepted_marker).count() != 1
        || production.matches(accepted_until_marker).count() != 1
        || production.matches(parameters_marker).count() != 1
        || production.matches(core_marker).count() != 1
        || production
            .matches("service_admission_parameters_v2(config)?")
            .count()
            != 2
        || production
            .matches("run_ferric_protected_verifier_post_begin_v2(begin, deadline, config)")
            .count()
            != 3
    {
        return false;
    }
    let Some(unnamed_at) = production.find(unnamed_marker) else {
        return false;
    };
    let Some(accepted_at) = production.find(accepted_marker) else {
        return false;
    };
    let Some(accepted_until_at) = production.find(accepted_until_marker) else {
        return false;
    };
    let Some(parameters_at) = production.find(parameters_marker) else {
        return false;
    };
    let Some(core_at) = production.find(core_marker) else {
        return false;
    };
    if !(unnamed_at < accepted_at
        && accepted_at < accepted_until_at
        && accepted_until_at < parameters_at
        && parameters_at < core_at)
    {
        return false;
    }
    let unnamed = &production[unnamed_at..accepted_at];
    let accepted = &production[accepted_at..accepted_until_at];
    let accepted_until = &production[accepted_until_at..parameters_at];
    let core = &production[core_at..];
    if !unnamed.contains("begin_worker_v3_verification_session_until_v2(")
        || unnamed.contains("begin_worker_v3_verification_accepted_session_until_v2(")
        || !accepted.contains("endpoint: WorkerV3VerificationAcceptedServiceEndpointV2")
        || !accepted.contains("begin_worker_v3_verification_accepted_session_until_v2(")
        || accepted.contains("control: OwnedFd")
        || accepted.contains("begin_worker_v3_verification_session_until_v2(")
        || !accepted_until.contains("deadline: AbsoluteSessionDeadlineV1")
        || !accepted_until.contains("begin_worker_v3_verification_accepted_session_until_v2(")
        || accepted_until.contains("service_admission_parameters_v2(config)?")
        || accepted_until.contains("AbsoluteSessionDeadlineV1::after(")
        || core.contains("begin_worker_v3_verification_session_until_v2(")
        || core.contains("begin_worker_v3_verification_accepted_session_until_v2(")
    {
        return false;
    }
    let mut prior = 0;
    for operation in [
        "validate_roster(",
        "read_and_decode_payloads(",
        ".receive_current_record()",
        ".authenticate_current_record(",
        ".verify_all_kernels(",
        "request_receipt_signature(",
        ".send_application_response(",
    ] {
        let Some(position) = core.find(operation) else {
            return false;
        };
        if position < prior {
            return false;
        }
        prior = position;
    }
    let normalized_readme = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    library
        .matches("run_ferric_protected_verifier_accepted_session_v2")
        .count()
        == 1
        && normalized_readme.contains(
            "The unnamed and accepted-path entrypoints share one post-Begin application core.",
        )
        && normalized_readme
            .contains("does not create, bind, listen on, discover, or accept a socket")
        && normalized_readme.contains("supervisor remains responsible")
}

#[test]
fn connected_path_entrypoint_is_bound_to_the_common_post_begin_core() {
    assert!(connected_path_service_policy(
        SERVICE_SOURCE,
        LIB_SOURCE,
        README
    ));
}

#[test]
fn connected_path_source_policy_rejects_entrypoint_and_core_substitution() {
    let wrong_begin = SERVICE_SOURCE.replacen(
        "begin_worker_v3_verification_accepted_session_until_v2(",
        "begin_worker_v3_verification_session_until_v2(",
        1,
    );
    let detached_core = SERVICE_SOURCE.replacen(
        "run_ferric_protected_verifier_post_begin_v2(begin, deadline, config)",
        "Err(FerricProtectedVerifierServiceFailureV1::UnsupportedTransportDisposition)",
        1,
    );
    let raw_control = SERVICE_SOURCE.replacen(
        "endpoint: WorkerV3VerificationAcceptedServiceEndpointV2",
        "control: OwnedFd",
        1,
    );
    let missing_export = LIB_SOURCE.replacen(
        "run_ferric_protected_verifier_accepted_session_v2",
        "run_detached_protected_verifier_session_v2",
        1,
    );
    let weakened_docs = README.replacen("supervisor remains responsible", "caller may discover", 1);
    for (index, (service, library, readme)) in [
        (wrong_begin.as_str(), LIB_SOURCE, README),
        (detached_core.as_str(), LIB_SOURCE, README),
        (raw_control.as_str(), LIB_SOURCE, README),
        (SERVICE_SOURCE, missing_export.as_str(), README),
        (SERVICE_SOURCE, LIB_SOURCE, weakened_docs.as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            !connected_path_service_policy(service, library, readme),
            "hostile connected-path policy mutation {index} was accepted"
        );
    }
}
