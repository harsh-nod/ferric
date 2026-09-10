# Ferric aggregate Worker V3 protected-verifier service foundation

This standalone package owns Ferric's fail-closed V2 service orchestration for
the 12-entry Qwen3 aggregate roster. It is deliberately outside the legacy
Ferric workspace and uses fe2o3's multi-phase Worker V3 transport at commit
`3546d54d2c4a913f5d079701aed557d0a378bba8`.

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
- a descriptor-only protected compiler-current client for one
  supervisor-preopened connected `SOCK_SEQPACKET` endpoint, pinned to one
  protocol, measured provider, compiler policy, and fresh admission session;
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

The protected compiler-current client uses fixed canonical request and response
packets and only the absolute deadline supplied by the outer verification
session. Its request contains the complete canonical Begin frame, the complete
canonical compiler receipt carriage selected from the decoded envelope, and the
complete canonical current-record frame. It also binds the exact decoded
envelope length and digest, pinned compiler policy, measured provider, protocol,
fresh admission-session identity, and nonzero monotonic request sequence into
both domain-separated transcript identities. The admission identity must never
be reused for that provider, including across supervisor, verifier, and provider
restarts. Admission requires exclusive custody of a fresh connection with no
prequeued packets and no descriptor duplicate retained by a supervisor or an
earlier client. A correlated explicit provider rejection retains the synchronized
endpoint. Deadline expiry before a send also retains it; every ambiguous
post-send outcome, response substitution, replay, ancillary transfer, or
truncation permanently closes and poisons it. A success token is minted only
for an exactly correlated `Authenticated` response with a nonzero protected
transcript. The typed failure exposes retained/poisoned custody; dropping that
failure does not itself close a retained endpoint because the client still owns
it.

The compiler-current server complement owns one supervisor-preopened connected
`SOCK_SEQPACKET` endpoint and one measured authority implementation. Admission
requires a fresh empty connection, exact descriptor flags/device/inode and
peer credentials, distinct provider/verifier UIDs, the same pinned protocol,
provider measurement, compiler policy, and never-reused admission-session
identity as the client. For each packet it requires the next exact sequence,
strictly decodes the canonical Begin, carriage, and current-record frames again,
rechecks every available digest/identity association, and verifies the nested
current-record signature against the exact carried policy before calling its
narrow unsafe authority interface. It emits only an exactly correlated canonical
`Authenticated` or generic `Rejected` response under the caller-supplied absolute
deadline. Authority rejection retains the synchronized endpoint; malformed,
substituted, replayed, ancillary-bearing, late, or otherwise ambiguous traffic
permanently poisons and closes it. The returned transcript binds the authority's
nonzero transcript to the complete request, measurement, policy, admission
session, and sequence.

The request carries the complete Begin, carriage, and current-record frames but
only the complete envelope's length and digest. A production authority must use
those coordinates to reacquire and verify the retained full envelope; it must
reject when those bytes are unavailable. The unsafe authority implementation
also remains responsible for independently protected policy and signing-key
configuration, exact live Worker-ledger lookup, external monotonic rollback
state, and a durable admission-session/replay store that survives every process
restart. The in-memory server sequence check supplements but does not replace
that durable state.

These IPC primitives do not implement or provision the external signer,
protected head-store process, or compiler-current authenticator daemon; implement the
compiler policy or live session store; store a private key; run an independent
checker; or launch a protected deployment. Signer
authority is unchanged: Its descriptor checks and wire protocol grant no production authority.
Head-store authority exists only through its explicit unsafe supervisor admission.

The private `0700` directory excludes different-UID path mutation. Like any
pathname API, it cannot exclude a concurrent rename by another thread or
process already running as the same effective UID. Parent and socket
device/inode checks fail closed around bind and cleanup; an ambiguous node is
left untouched rather than unlinked. Deployment must not share the service UID
or its private directory with an untrusted process.
