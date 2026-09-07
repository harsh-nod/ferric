//! Source policy for the descriptor-only protected receipt-signer client.

const SIGNER_SOURCE: &str = include_str!("../src/signer_ipc.rs");
const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const LOCKFILE: &str = include_str!("../Cargo.lock");
const README: &str = include_str!("../README.md");

fn signer_ipc_policy(signer: &str, library: &str, readme: &str) -> bool {
    let Some(test_boundary) = signer.find("#[cfg(test)]") else {
        return false;
    };
    let production = &signer[..test_boundary];
    let tests = &signer[test_boundary..];

    for required in [
        "pub fn admit(",
        "Self::admit_inner::<true>(",
        "self.sign_inner::<true>(input)",
        "peer: OwnedFd",
        "peer: Option<OwnedFd>",
        "ProtectedReceiptSignerClientAdmissionFailureV1 { error, peer }",
        "let Some(peer) = self.peer.take()",
        "if input.deadline().remaining().is_none()",
        "self.peer = Some(peer)",
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
        "signing_input_sha256",
        "signing_input,",
        "&policy_identity",
        "&provider_identity",
        "&verifying_key",
        "response_identity(",
        "&status.code().to_le_bytes()",
        "if !response.matches_request(request)",
        "ProtectedReceiptSignerResponseStatusV1::Rejected",
        "ProtectedReceiptSignerClientErrorV1::SignerRejected",
    ] {
        if !production.contains(required) {
            return false;
        }
    }

    for forbidden in [
        "admit_inner::<false>",
        "sign_inner::<false>",
        "std::env",
        "env::",
        "std::path",
        "Path::",
        "UnixStream",
        "UnixDatagram",
        "TcpStream",
        "File::open",
        "SigningKey",
        "SCM_RIGHTS",
        "/dev/",
        "AbsoluteSessionDeadlineV1::after(",
    ] {
        if production.contains(forbidden) {
            return false;
        }
    }

    let Some(deadline_at) = production.find("if input.deadline().remaining().is_none()") else {
        return false;
    };
    let Some(request_at) = production.find("let request = ProtectedReceiptSignerRequestV1::new(")
    else {
        return false;
    };
    let Some(take_at) = production.find("let Some(peer) = self.peer.take()") else {
        return false;
    };
    let normalized_readme = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    deadline_at < request_at
        && request_at < take_at
        && production.matches("&status.code().to_le_bytes()").count() == 2
        && tests.contains("Self::admit_inner::<false>(")
        && tests.contains("self.sign_inner::<false>(input)")
        && tests.contains("SigningKey")
        && tests.contains("libc::SCM_RIGHTS")
        && library.contains("PreopenedProtectedReceiptSignerClientV1")
        && library.contains("ProtectedReceiptSignerClientAdmissionFailureV1")
        && normalized_readme
            .contains("Its descriptor checks and wire protocol grant no production authority.")
        && normalized_readme.contains(
            "the same-UID client and development signing key exist only below `cfg(test)`.",
        )
}

fn signer_dependency_policy(manifest: &str, lockfile: &str) -> bool {
    let dependency = "libc = \"=0.2.189\"";
    if manifest.matches(dependency).count() != 1
        || ["socket2 =", "nix =", "tokio =", "ring ="]
            .into_iter()
            .any(|forbidden| manifest.contains(forbidden))
    {
        return false;
    }
    let package_marker =
        "[[package]]\nname = \"ferric-qwen3-all-kernels-worker-v3-verifier-service-v1\"";
    let Some(package) = lockfile.split_once(package_marker).map(|(_, tail)| tail) else {
        return false;
    };
    let package = package.split("\n[[package]]").next().unwrap_or(package);
    package.matches("\n \"libc\",\n").count() == 1
}

#[test]
fn signer_ipc_is_descriptor_only_correlated_and_fail_closed() {
    assert!(signer_ipc_policy(SIGNER_SOURCE, LIB_SOURCE, README));
}

#[test]
fn signer_ipc_source_policy_rejects_security_boundary_substitution() {
    let production_same_uid = SIGNER_SOURCE.replacen(
        "Self::admit_inner::<true>(",
        "Self::admit_inner::<false>(",
        1,
    );
    let stream_fallback = SIGNER_SOURCE.replacen("libc::SOCK_SEQPACKET", "libc::SOCK_STREAM", 1);
    let detached_status =
        SIGNER_SOURCE.replacen("&status.code().to_le_bytes()", "&0_u16.to_le_bytes()", 1);
    let lost_endpoint = SIGNER_SOURCE.replacen(
        "ProtectedReceiptSignerClientAdmissionFailureV1 { error, peer }",
        "panic!(\"drop rejected endpoint: {error:?} {peer:?}\")",
        1,
    );
    let missing_correlation =
        SIGNER_SOURCE.replacen("if !response.matches_request(request)", "if false", 1);
    let weakened_docs = README.replacen(
        "descriptor checks and wire protocol grant no production authority",
        "descriptor checks establish production authority",
        1,
    );
    for (index, (signer, library, readme)) in [
        (production_same_uid.as_str(), LIB_SOURCE, README),
        (stream_fallback.as_str(), LIB_SOURCE, README),
        (detached_status.as_str(), LIB_SOURCE, README),
        (lost_endpoint.as_str(), LIB_SOURCE, README),
        (missing_correlation.as_str(), LIB_SOURCE, README),
        (SIGNER_SOURCE, LIB_SOURCE, weakened_docs.as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            !signer_ipc_policy(signer, library, readme),
            "hostile signer IPC policy mutation {index} was accepted"
        );
    }
}

#[test]
fn signer_transport_dependency_is_exact_and_locked() {
    assert!(signer_dependency_policy(MANIFEST, LOCKFILE));
    for (index, (manifest, lockfile)) in [
        (MANIFEST.replacen("=0.2.189", "0.2", 1), LOCKFILE.to_owned()),
        (
            MANIFEST.replacen("[dependencies]", "[dependencies]\nsocket2 = \"0.6\"", 1),
            LOCKFILE.to_owned(),
        ),
        (
            MANIFEST.to_owned(),
            LOCKFILE.replacen(
                "\n \"ferric-qwen3-all-kernels-worker-v3-verifier-v1\",\n \"libc\",\n \"rustix\",",
                "\n \"ferric-qwen3-all-kernels-worker-v3-verifier-v1\",\n \"rustix\",",
                1,
            ),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            !signer_dependency_policy(&manifest, &lockfile),
            "hostile signer dependency mutation {index} was accepted"
        );
    }
}
