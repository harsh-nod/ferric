# Ferric aggregate Worker V3 protected-verifier service foundation

This standalone package owns Ferric's fail-closed V2 service orchestration for
the 12-entry Qwen3 aggregate roster. It is deliberately outside the legacy
Ferric workspace and uses fe2o3's multi-phase Worker V3 transport at commit
`cf6faec0ee3c026d3a1fc5090ab606a3b425225c`.

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
- a descriptor-only protected-signer client for one supervisor-preopened,
  connected `SOCK_SEQPACKET` endpoint with pinned object identity, exact peer
  PID/UID/GID, provider identity, and public key;
- a descriptor-only protected head-store client for one supervisor-preopened,
  connected `SOCK_SEQPACKET` endpoint pinned to one protocol, measured provider,
  policy, ledger kind, and durable namespace;
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
The concrete signer client uses only the service's supplied absolute deadline
and fixed-size canonical packets. It binds the complete receipt signing input,
policy, provider, public key, request identity, response status, and signature;
rejects ancillary data and packet truncation; and permanently drops its
endpoint after every post-send ambiguous outcome. A correlated explicit signer
rejection leaves the synchronized channel available. Production admission
requires a separately provisioned signer UID distinct from the verifier UID;
the same-UID client and development signing key exist only below `cfg(test)`.
The one-shot listener accepts only its explicit caller-owned path; the service
does not discover an environment variable or default endpoint and never accepts
a raw private key. Credential or endpoint-admission rejection retains the exact
accepted descriptor. After successful endpoint admission, failures provide
exactly the existing fe2o3/Ferric terminal custody and do not claim general
post-Begin descriptor recovery.

The protected head-store client uses fixed 428-byte request and 360-byte
response packets and one supervisor-bounded absolute timeout per operation.
Every exchange binds a fresh nonzero supervisor-provisioned admission-session
identity, a nonzero per-client request sequence, the protocol and provider
identities, and the exact `(namespace, policy, ledger kind)` context. The
admission identity must never be reused for that provider and namespace,
including across verifier and provider restarts, so a sequence-one response
from an earlier admission cannot correlate to a new client.
Initialization admits only an empty head. Compare-and-advance admits only the
same policy/header and an exact one-record successor with a nonzero record
identity. A protected peer may return `Advanced` only after that successor is
externally durable. Correlated conflicts and explicit rejections leave the
channel synchronized. A correlated `Absent` after synchronized existence
evidence is treated as rollback, as is a loaded head below or inconsistent
with the client's last observed head. Endpoint change, ancillary data,
truncation, replayed/uncorrelated responses, deadline expiry after send, and
every other ambiguous post-send outcome permanently poison and close the
endpoint. The head-store wire protocol and descriptor checks do not prove
external durability. Constructing the trait implementation is therefore an
unsafe supervisor boundary requiring a separately measured protected provider,
exclusive durable serialization, and permanent namespace retention. Admission
also requires exclusive custody of a fresh connection with no prequeued packets
and no descriptor duplicate retained by the supervisor or an earlier client.
The typed bounded methods preserve whether the client retains the endpoint.
The legacy trait boxes that typed failure; dropping a boxed `Retained` failure
does not close the endpoint because custody remains in the client, while callers
that need to branch on custody must downcast it or call the typed methods.

These clients do not implement or provision the external signer or protected
head-store processes, store a private key, authenticate a compiler current
record, run an independent checker, or launch a protected deployment. Signer
authority is unchanged: Its descriptor checks and wire protocol grant no production authority.
Head-store authority exists only through its explicit unsafe supervisor admission.

The private `0700` directory excludes different-UID path mutation. Like any
pathname API, it cannot exclude a concurrent rename by another thread or
process already running as the same effective UID. Parent and socket
device/inode checks fail closed around bind and cleanup; an ambiguous node is
left untouched rather than unlinked. Deployment must not share the service UID
or its private directory with an untrusted process.
