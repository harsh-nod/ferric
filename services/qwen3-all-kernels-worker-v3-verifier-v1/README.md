# Ferric aggregate Worker V3 protected-verifier service foundation

This standalone package owns Ferric's fail-closed V2 service orchestration for
the 12-entry Qwen3 aggregate roster. It is deliberately outside the legacy
Ferric workspace and uses fe2o3's multi-phase Worker V3 transport at commit
`6492c8fa85a00d93aa6ca2a4a77675fefad2fee6`.

The foundation provides:

- caller credential plus pinned policy/measurement admission;
- hard-bounded, policy-bound replay and reservation ledgers with retained
  deterministic indexes and policy/kind-domain-separated namespaces;
- a move-only protected-storage capability minted only at a documented unsafe
  supervisor boundary, backed by an external antirollback head store;
- service challenges read from a supervisor-provided entropy descriptor and
  durably burned before release;
- a single absolute session deadline;
- a second bounded read and digest check of the retained V2 envelope and HSACO;
- canonical V2 envelope and compiler-current-record association;
- explicit protected current-record, independent checker, and external signer
  provider contracts;
- exact 12-entry request/result joins and a terminal Ferric V1 response payload;
- a connected-path entrypoint consuming only fe2o3's separately admitted
  accepted-endpoint capability;
- a bounded one-shot listener that binds one caller-supplied canonical absolute
  pathname in an effective-UID-owned exact-`0700` directory, retains the
  no-symlink parent identity, sets exact `0600` mode, prepares credential
  stamping before backlog-one listen, admits one exact PID/UID/GID, and removes
  only the captured socket device/inode through the retained parent descriptor;
- stage-specific rejection and terminal-send custody.

The ledger's SHA chain detects corruption; it does not prevent rollback. That
property is an explicit unsafe deployment contract on the protected head store
and supervisor. Each ledger capability, file header, external head, replay
guard, and reservation provider is bound to one exact Worker V3 trust-policy
identity. Capacity exhaustion is permanent and terminal for that policy. The
safe API provides no rotation, reset, deletion, reinitialization, or compaction.
A supported replacement requires a newly identified trust policy, durable
global revocation of the old policy before provisioning, separate
policy/kind-domain-separated ledgers, and permanent retention of the old heads.
The unsafe global head-store contract explicitly forbids resetting or reusing a
same-policy namespace.

The unnamed and accepted-path entrypoints share one post-Begin application
core. The accepted-path entrypoint does not create, bind, listen on, discover,
or accept a socket; for that lower-level entrypoint, the supervisor remains responsible for those
operations and for constructing fe2o3's ownership-bearing accepted-endpoint
capability. The separate one-shot listener
performs those operations without discovering a default endpoint. It computes
one deadline before path admission, rejects stale or noncanonical paths and
symlinked parents, enables `SO_PASSCRED` before `listen`, accepts only one
connection, checks the exact configured PID/UID/GID, and then delegates to the
same accepted-session core without extending the deadline. Its device/inode
cleanup refuses to unlink a missing or substituted node. All three paths use
the same caller policy, replay and reservation state, provider checks, response
construction, and terminal custody.

This is **service foundation, not deployment closure**. Production still needs
reviewed measured processes implementing the current-record authenticator,
theorem checker, signer, and protected antirollback head store, plus a process
launcher that supplies only preopened descriptors and pins their identities.
Every synchronous provider IPC must impose deadline-aware transport
cancellation; if a provider returns after the outer deadline the service
rejects, while a hung in-process call cannot be cancelled by this foundation.
The one-shot listener accepts only its explicit caller-owned path; the service
does not discover an environment variable or default endpoint and never accepts
a raw private key. Credential or endpoint-admission rejection retains the exact
accepted descriptor. After successful endpoint admission, failures provide
exactly the existing fe2o3/Ferric terminal custody and do not claim general
post-Begin descriptor recovery.

The private `0700` directory excludes different-UID path mutation. Like any
pathname API, it cannot exclude a concurrent rename by another thread or
process already running as the same effective UID. Parent and socket
device/inode checks fail closed around bind and cleanup; an ambiguous node is
left untouched rather than unlinked. Deployment must not share the service UID
or its private directory with an untrusted process.
