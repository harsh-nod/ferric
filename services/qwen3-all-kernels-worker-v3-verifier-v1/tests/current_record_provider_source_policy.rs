//! Source policy for the descriptor-only compiler-current provider server.

const PROVIDER_SOURCE: &str = include_str!("../src/current_record_provider.rs");
const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const README: &str = include_str!("../README.md");

fn provider_source_policy(provider: &str, library: &str, readme: &str) -> bool {
    for required in [
        "pub unsafe fn admit_from_supervisor(",
        "Self::admit_inner::<true>(",
        "peer: Option<OwnedFd>",
        "authority: A",
        "ProtectedCompilerCurrentServerAdmissionFailureV1 {",
        "validate_endpoint::<true>",
        "require_empty_connection(&peer)",
        "FdFlags::CLOEXEC",
        "OFlags::NONBLOCK",
        "libc::SO_DOMAIN",
        "libc::SO_TYPE",
        "libc::SOCK_SEQPACKET",
        "libc::SO_ACCEPTCONN",
        "libc::SO_ERROR",
        "libc::SO_PEERCRED",
        "rustix::process::geteuid().as_raw()",
        "libc::MSG_PEEK",
        "libc::recvmsg(",
        "libc::MSG_CTRUNC",
        "header.msg_controllen != 0",
        "libc::MSG_TRUNC",
        "libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL",
        "request.request_sequence() != self.next_request_sequence",
        "request.protocol_identity() == self.protocol_identity",
        "request.provider_measurement() == self.provider_measurement",
        "request.compiler_policy_identity() == self.compiler_policy_identity",
        "request.admission_session_identity() == self.admission_session_identity",
        "WorkerV3VerificationRequestV1::decode_canonical(",
        "CompilerExecutionReceiptCarriageV1::decode(",
        "WorkerV3VerificationCurrentRecordFrameV2::decode_canonical(",
        ".verify(",
        "PROVIDER_TRANSCRIPT_DOMAIN_V1",
        "ProtectedCompilerCurrentResponseV1::authenticated(",
        "ProtectedCompilerCurrentResponseV1::rejected(&request)",
        "ProtectedCompilerCurrentServerCustodyV1::Poisoned",
        "ProtectedCompilerCurrentServerCustodyV1::Retained",
    ] {
        if !provider.contains(required) {
            return false;
        }
    }
    for forbidden in [
        "admit_inner::<false>",
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
        "private_key",
        "secret_key",
    ] {
        if provider.contains(forbidden) {
            return false;
        }
    }
    let normalized_readme = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    library.contains("PreopenedProtectedCompilerCurrentServerV1")
        && library.contains("ProtectedCompilerCurrentAuthorityV1")
        && normalized_readme.contains(
            "A production authority must use those coordinates to reacquire and verify the retained full envelope",
        )
        && normalized_readme.contains(
            "a durable admission-session/replay store that survives every process restart",
        )
        && normalized_readme.contains(
            "do not implement or provision the external signer, protected head-store process, or compiler-current authenticator daemon",
        )
}

#[test]
fn provider_is_descriptor_only_correlated_and_fail_closed() {
    assert!(provider_source_policy(PROVIDER_SOURCE, LIB_SOURCE, README));
}

#[test]
fn provider_source_policy_rejects_boundary_substitution() {
    let safe_admission = PROVIDER_SOURCE.replacen(
        "pub unsafe fn admit_from_supervisor(",
        "pub fn admit_from_supervisor(",
        1,
    );
    let same_uid = PROVIDER_SOURCE.replacen(
        "Self::admit_inner::<true>(",
        "Self::admit_inner::<false>(",
        1,
    );
    let stream = PROVIDER_SOURCE.replacen("libc::SOCK_SEQPACKET", "libc::SOCK_STREAM", 1);
    let no_signature = PROVIDER_SOURCE.replacen(".verify(", ".skip_verify(", 1);
    let no_sequence = PROVIDER_SOURCE.replacen(
        "request.request_sequence() != self.next_request_sequence",
        "false",
        1,
    );
    let no_poison = PROVIDER_SOURCE.replacen(
        "ProtectedCompilerCurrentServerCustodyV1::Poisoned",
        "ProtectedCompilerCurrentServerCustodyV1::Retained",
        1,
    );
    let weakened_docs = README.replacen(
        "a durable admission-session/replay store that survives every process\nrestart",
        "an in-memory session counter",
        1,
    );
    for (index, (provider, readme)) in [
        (safe_admission.as_str(), README),
        (same_uid.as_str(), README),
        (stream.as_str(), README),
        (no_signature.as_str(), README),
        (no_sequence.as_str(), README),
        (no_poison.as_str(), README),
        (PROVIDER_SOURCE, weakened_docs.as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            !provider_source_policy(provider, LIB_SOURCE, readme),
            "hostile compiler-current provider mutation {index} was accepted"
        );
    }
}
