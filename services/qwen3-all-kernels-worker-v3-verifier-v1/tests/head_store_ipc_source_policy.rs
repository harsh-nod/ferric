//! Source policy for the descriptor-only protected antirollback head-store client.

const HEAD_STORE_SOURCE: &str = include_str!("../src/head_store_ipc.rs");
const DURABLE_SOURCE: &str = include_str!("../src/durable.rs");
const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const README: &str = include_str!("../README.md");

fn head_store_ipc_policy(head_store: &str, durable: &str, library: &str, readme: &str) -> bool {
    let Some(test_boundary) = head_store.find("#[cfg(test)]") else {
        return false;
    };
    let production = &head_store[..test_boundary];
    let tests = &head_store[test_boundary..];

    for required in [
        "pub unsafe fn admit_from_supervisor(",
        "Self::admit_inner::<true>(",
        "unsafe impl ProtectedLedgerHeadStoreV1 for PreopenedProtectedHeadStoreClientV1",
        "peer: OwnedFd",
        "peer: Option<OwnedFd>",
        "ProtectedHeadStoreClientAdmissionFailureV1 { error, peer }",
        "let Some(peer) = self.peer.take()",
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
        "request_identity(",
        "response_identity_fields(",
        "&protocol_identity",
        "&provider_identity",
        "&namespace_identity",
        "&policy_identity",
        "&admission_session_identity",
        "&kind.tag().to_le_bytes()",
        "&request_sequence.to_le_bytes()",
        "if !response.matches_request(request)",
        "ProtectedHeadStoreResponseStatusV1::Advanced",
        "ProtectedHeadStoreResponseStatusV1::Conflict",
        "ProtectedHeadStoreResponseStatusV1::Absent if !self.observed_existing",
        "ProtectedHeadStoreClientErrorV1::RollbackEvidence",
        "next.record_count() == current.record_count().checked_add(1)",
        "head.record_count() < prior.record_count()",
        "self.peer = None",
        "ProtectedHeadStoreClientCustodyV1::Poisoned",
        "operation_timeout < Duration::from_millis(1)",
        "operation_timeout > MAX_OPERATION_TIMEOUT",
    ] {
        if !production.contains(required) {
            return false;
        }
    }

    for forbidden in [
        "admit_inner::<false>",
        "load_head_inner::<false>",
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
    ] {
        if production.contains(forbidden) {
            return false;
        }
    }

    let normalized_readme = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    production.matches("&protocol_identity").count() == 4
        && production.matches("&provider_identity").count() == 4
        && production.matches("&namespace_identity").count() == 4
        && production.matches("&policy_identity").count() == 4
        && production.matches("&admission_session_identity").count() == 4
        && production
            .matches("&request_sequence.to_le_bytes()")
            .count()
            == 4
        && tests.contains("Self::admit_inner::<false>(")
        && tests.contains("libc::SCM_RIGHTS")
        && tests.contains("cross_admission_prequeued_sequence_one_responses_poison")
        && tests.contains("valid_recomputed_response_coordinate_substitutions_poison")
        && durable.contains("pub(crate) const fn from_tag(tag: u16) -> Option<Self>")
        && library.contains("PreopenedProtectedHeadStoreClientV1")
        && library.contains("ProtectedHeadStoreClientCustodyV1")
        && normalized_readme.contains(
            "The head-store wire protocol and descriptor checks do not prove external durability.",
        )
        && normalized_readme.contains(
            "A correlated `Absent` after synchronized existence evidence is treated as rollback",
        )
        && normalized_readme
            .contains("The admission identity must never be reused for that provider and namespace")
        && normalized_readme
            .contains("exclusive custody of a fresh connection with no prequeued packets")
        && normalized_readme
            .contains("dropping a boxed `Retained` failure does not close the endpoint")
}

#[test]
fn head_store_ipc_is_namespace_bound_monotonic_and_fail_closed() {
    assert!(head_store_ipc_policy(
        HEAD_STORE_SOURCE,
        DURABLE_SOURCE,
        LIB_SOURCE,
        README,
    ));
}

#[test]
fn head_store_source_policy_rejects_security_boundary_substitution() {
    let safe_admission = HEAD_STORE_SOURCE.replacen(
        "pub unsafe fn admit_from_supervisor(",
        "pub fn admit_from_supervisor(",
        1,
    );
    let same_uid = HEAD_STORE_SOURCE.replacen(
        "Self::admit_inner::<true>(",
        "Self::admit_inner::<false>(",
        1,
    );
    let stream = HEAD_STORE_SOURCE.replacen("libc::SOCK_SEQPACKET", "libc::SOCK_STREAM", 1);
    let detached_namespace = HEAD_STORE_SOURCE.replacen("&namespace_identity", "&[0x55; 32]", 1);
    let detached_sequence =
        HEAD_STORE_SOURCE.replacen("&request_sequence.to_le_bytes()", "&0_u64.to_le_bytes()", 1);
    let detached_admission_session =
        HEAD_STORE_SOURCE.replacen("&admission_session_identity", "&[0x5a; 32]", 1);
    let missing_correlation =
        HEAD_STORE_SOURCE.replacen("if !response.matches_request(request)", "if false", 1);
    let rollback_absent = HEAD_STORE_SOURCE.replacen(
        "ProtectedHeadStoreResponseStatusV1::Absent if !self.observed_existing",
        "ProtectedHeadStoreResponseStatusV1::Absent",
        1,
    );
    let skipped_cas = HEAD_STORE_SOURCE.replacen(
        "next.record_count() == current.record_count().checked_add(1)",
        "next.record_count() >= current.record_count().checked_add(1)",
        1,
    );
    let retained_ambiguity = HEAD_STORE_SOURCE.replacen("self.peer = None", "return", 1);
    let weakened_docs = README.replacen(
        "wire protocol and descriptor checks do not prove\nexternal durability",
        "wire protocol proves external durability",
        1,
    );

    for (index, (head_store, readme)) in [
        (safe_admission.as_str(), README),
        (same_uid.as_str(), README),
        (stream.as_str(), README),
        (detached_namespace.as_str(), README),
        (detached_sequence.as_str(), README),
        (detached_admission_session.as_str(), README),
        (missing_correlation.as_str(), README),
        (rollback_absent.as_str(), README),
        (skipped_cas.as_str(), README),
        (retained_ambiguity.as_str(), README),
        (HEAD_STORE_SOURCE, weakened_docs.as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            !head_store_ipc_policy(head_store, DURABLE_SOURCE, LIB_SOURCE, readme),
            "hostile head-store source mutation {index} was accepted"
        );
    }
}
