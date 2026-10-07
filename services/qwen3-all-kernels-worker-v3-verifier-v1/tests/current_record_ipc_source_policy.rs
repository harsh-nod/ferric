//! Source policy for the descriptor-only protected compiler-current client.

const CURRENT_SOURCE: &str = include_str!("../src/current_record_ipc.rs");
const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const README: &str = include_str!("../README.md");

fn current_record_ipc_policy(current: &str, library: &str, readme: &str) -> bool {
    let Some(test_boundary) = current.find("#[cfg(test)]") else {
        return false;
    };
    let production = &current[..test_boundary];
    let tests = &current[test_boundary..];
    for required in [
        "pub unsafe fn admit_from_supervisor(",
        "Self::admit_inner::<true>(",
        "unsafe impl ProtectedCompilerCurrentRecordProviderV1",
        "peer: OwnedFd",
        "peer: Option<OwnedFd>",
        "ProtectedCompilerCurrentClientAdmissionFailureV1 { error, peer }",
        "let Some(peer) = self.peer.take()",
        "if deadline.remaining().is_none()",
        "self.peer = Some(peer)",
        "self.peer = None",
        "FdFlags::CLOEXEC",
        "OFlags::NONBLOCK",
        "libc::SO_DOMAIN",
        "libc::SO_TYPE",
        "libc::SOCK_SEQPACKET",
        "libc::SO_ACCEPTCONN",
        "libc::SO_ERROR",
        "libc::SO_PEERCRED",
        "stat.st_dev != endpoint.device || stat.st_ino != endpoint.inode",
        "observed.uid == rustix::process::geteuid().as_raw()",
        "libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL",
        "libc::recvmsg(",
        "libc::MSG_CTRUNC",
        "header.msg_controllen != 0",
        "libc::MSG_TRUNC",
        "REQUEST_IDENTITY_DOMAIN_V1",
        "RESPONSE_IDENTITY_DOMAIN_V1",
        "&protocol_identity",
        "&provider_measurement",
        "&compiler_policy_identity",
        "&admission_session_identity",
        "&request_sequence.to_le_bytes()",
        "begin_request.encode_canonical()",
        "let envelope_bytes = envelope",
        "envelope.compiler_execution_receipt()",
        "current_record.encode_canonical()",
        "if !response.matches_request(request)",
        "ProtectedCompilerCurrentResponseStatusV1::Authenticated",
        "ProtectedCompilerCurrentResponseStatusV1::Rejected",
        "AuthenticatedCompilerCurrentRecordV1::from_independent_authentication(",
        "ProtectedCompilerCurrentClientCustodyV1::Poisoned",
    ] {
        if !production.contains(required) {
            return false;
        }
    }
    for forbidden in [
        "admit_inner::<false>",
        "authenticate_parts::<false>",
        "std::env",
        "env::",
        "std::path",
        "Path::",
        "UnixStream",
        "UnixDatagram",
        "TcpStream",
        "File::open",
        "SCM_RIGHTS",
        "/dev/",
        "SigningKey",
        "AbsoluteSessionDeadlineV1::after(",
    ] {
        if production.contains(forbidden) {
            return false;
        }
    }
    let normalized_readme = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    production.matches("&admission_session_identity").count() == 2
        && production.matches("&request_sequence.to_le_bytes()").count() == 4
        && tests.contains("admit_inner::<false>(")
        && tests.contains("authenticate_parts::<false>(")
        && tests.contains("cross_admission_old_session_sequence_one_prequeue_poisons")
        && tests.contains("recomputed_response_coordinate_substitutions_poison")
        && tests.contains("duration_to_poll_millis(Duration::MAX)")
        && tests.contains("libc::SCM_RIGHTS")
        && library.contains("PreopenedProtectedCompilerCurrentClientV1")
        && library.contains("ProtectedCompilerCurrentClientCustodyV1")
        && normalized_readme.contains(
            "The admission identity must never be reused for that provider, including across",
        )
        && normalized_readme.contains(
            "Admission requires exclusive custody of a fresh connection with no prequeued packets",
        )
        && normalized_readme.contains(
            "do not implement or provision the external signer, protected head-store process, or compiler-current authenticator daemon",
        )
}

#[test]
fn compiler_current_ipc_is_descriptor_only_session_bound_and_fail_closed() {
    assert!(current_record_ipc_policy(
        CURRENT_SOURCE,
        LIB_SOURCE,
        README
    ));
}

#[test]
fn compiler_current_source_policy_rejects_boundary_substitution() {
    let safe_admission = CURRENT_SOURCE.replacen(
        "pub unsafe fn admit_from_supervisor(",
        "pub fn admit_from_supervisor(",
        1,
    );
    let same_uid = CURRENT_SOURCE.replacen(
        "Self::admit_inner::<true>(",
        "Self::admit_inner::<false>(",
        1,
    );
    let stream = CURRENT_SOURCE.replacen("libc::SOCK_SEQPACKET", "libc::SOCK_STREAM", 1);
    let detached_session = CURRENT_SOURCE.replacen("&admission_session_identity", "&[0x55; 32]", 1);
    let detached_sequence =
        CURRENT_SOURCE.replacen("&request_sequence.to_le_bytes()", "&0_u64.to_le_bytes()", 1);
    let missing_correlation =
        CURRENT_SOURCE.replacen("if !response.matches_request(request)", "if false", 1);
    let retained_ambiguity = CURRENT_SOURCE.replacen("self.peer = None", "return", 1);
    let weakened_docs = README.replacen(
        "exclusive custody of a fresh connection with no\nprequeued packets",
        "shared custody of an existing connection",
        1,
    );
    for (index, (current, readme)) in [
        (safe_admission.as_str(), README),
        (same_uid.as_str(), README),
        (stream.as_str(), README),
        (detached_session.as_str(), README),
        (detached_sequence.as_str(), README),
        (missing_correlation.as_str(), README),
        (retained_ambiguity.as_str(), README),
        (CURRENT_SOURCE, weakened_docs.as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            !current_record_ipc_policy(current, LIB_SOURCE, readme),
            "hostile compiler-current source mutation {index} was accepted"
        );
    }
}
