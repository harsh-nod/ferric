//! Source policy for the one-shot protected-verifier listener.

const LISTENER_SOURCE: &str = include_str!("../src/listener.rs");
const SERVICE_SOURCE: &str = include_str!("../src/service.rs");
const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const README: &str = include_str!("../README.md");

fn listener_policy(listener: &str, service: &str, library: &str, readme: &str) -> bool {
    let production = listener.split("#[cfg(test)]").next().unwrap_or(listener);
    let entry = "pub fn run_ferric_protected_verifier_listener_session_v2";
    if production.matches(entry).count() != 1
        || service
            .matches("pub(crate) fn run_ferric_protected_verifier_accepted_session_until_v2")
            .count()
            != 1
        || library
            .matches("run_ferric_protected_verifier_listener_session_v2")
            .count()
            != 1
    {
        return false;
    }

    let Some(entry_at) = production.find(entry) else {
        return false;
    };
    let entry = &production[entry_at..];
    let mut prior = 0;
    for operation in [
        "AbsoluteSessionDeadlineV1::after(config.timeout())",
        "canonical_service_path(expected_service_path)",
        "rustix::fs::openat2(",
        "rustix::fs::statat(&parent, basename, AtFlags::SYMLINK_NOFOLLOW)",
        "rustix::net::socket_with(",
        "rustix::net::bind(&listener, &address)",
        "rustix::fs::chmodat(",
        "verify_socket_path(&bound_path, SOCKET_MODE_V2)",
        "prepare_worker_v3_verification_receiver_v1(&listener)",
        "rustix::net::listen(&listener, 1)",
        "accept_until(&listener, deadline.instant())",
        "bound_path.cleanup()",
        "accepted_peer_credentials(&accepted)",
        "credentials_match(config.caller_policy(), observed)",
        "WorkerV3VerificationAcceptedServiceEndpointV2::admit(",
        "run_ferric_protected_verifier_accepted_session_until_v2(endpoint, deadline, config)",
    ] {
        let Some(position) = entry.find(operation) else {
            return false;
        };
        if position < prior {
            return false;
        }
        prior = position;
    }

    let cleanup = production.find("impl BoundSocketPathV2").and_then(|start| {
        production[start..]
            .find("impl Drop for BoundSocketPathV2")
            .map(|end| &production[start..start + end])
    });
    let Some(cleanup) = cleanup else {
        return false;
    };
    let identity = cleanup.find("self.socket.matches(&current)");
    let file_type = cleanup.find("FileType::from_raw_mode(current.st_mode) != FileType::Socket");
    let unlink =
        cleanup.find("rustix::fs::unlinkat(&self.parent, &self.basename, AtFlags::empty())");
    if !matches!((identity, file_type, unlink), (Some(a), Some(b), Some(c)) if a < c && b < c)
        || !production.contains("ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS")
        || !production.contains("const SOCKET_MODE_V2: Mode = Mode::RUSR.union(Mode::WUSR)")
        || !production.contains("retained.st_mode & 0o7777 != Mode::RWXU.bits()")
        || !production.contains("retained.st_uid != rustix::process::geteuid().as_raw()")
        || !production.contains("SocketFlags::CLOEXEC | SocketFlags::NONBLOCK")
        || !production.contains("source: ListenerFailureSourceV2::Endpoint(failure)")
        || !production.contains("Some(failure.into_control())")
    {
        return false;
    }

    let docs = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    docs.contains("one deadline before path admission")
        && docs.contains("checks the exact configured PID/UID/GID")
        && docs.contains("refuses to unlink a missing or substituted node")
        && docs.contains("do not claim general post-Begin descriptor recovery")
        && docs.contains("cannot exclude a concurrent rename")
}

#[test]
fn listener_binds_the_exact_prepared_path_and_shared_deadline_before_handoff() {
    assert!(listener_policy(
        LISTENER_SOURCE,
        SERVICE_SOURCE,
        LIB_SOURCE,
        README
    ));
}

#[test]
fn listener_policy_rejects_deadline_mode_identity_credential_and_custody_weakening() {
    let mutations = [
        LISTENER_SOURCE.replacen(
            "run_ferric_protected_verifier_accepted_session_until_v2(endpoint, deadline, config)",
            "run_ferric_protected_verifier_accepted_session_v2(endpoint, config)",
            1,
        ),
        LISTENER_SOURCE.replacen(
            "rustix::net::listen(&listener, 1)",
            "rustix::net::listen(&listener, 128)",
            1,
        ),
        LISTENER_SOURCE.replacen("Mode::RUSR.union(Mode::WUSR)", "Mode::RWXU", 1),
        LISTENER_SOURCE.replacen("retained.st_mode & 0o7777 != Mode::RWXU.bits()", "false", 1),
        LISTENER_SOURCE.replacen("self.socket.matches(&current)", "true", 1),
        LISTENER_SOURCE.replacen(
            "credentials_match(config.caller_policy(), observed)",
            "true",
            1,
        ),
        LISTENER_SOURCE.replacen("Some(failure.into_control())", "None", 1),
    ];
    for (index, mutation) in mutations.iter().enumerate() {
        assert!(
            !listener_policy(mutation, SERVICE_SOURCE, LIB_SOURCE, README),
            "hostile listener mutation {index} was accepted"
        );
    }
}
