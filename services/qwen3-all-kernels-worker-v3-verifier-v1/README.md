# Ferric aggregate Worker V3 protected-verifier service foundation

This standalone package owns Ferric's fail-closed V2 service orchestration for
the 12-entry Qwen3 aggregate roster. It is deliberately outside the legacy
Ferric workspace and uses fe2o3's multi-phase Worker V3 transport at commit
`43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb`.

The foundation provides:

- caller credential plus pinned policy/measurement admission;
- hard-bounded, checksummed replay and reservation ledger epochs with retained
  deterministic indexes;
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
- stage-specific rejection and terminal-send custody.

The ledger's SHA chain detects corruption; it does not prevent rollback. That
property is an explicit unsafe deployment contract on the protected head store
and supervisor. Ledger capacity is terminal within an epoch. Rotation requires
the supervisor to quiesce admission, provision a new protected object under a
strictly increasing non-reused epoch, synchronize it, and atomically advance the
external antirollback head. There is no silent compaction.

This is **service foundation, not deployment closure**. Production still needs
reviewed measured processes implementing the current-record authenticator,
theorem checker, signer, and protected antirollback head store, plus a supervisor
that supplies only preopened descriptors and pins their identities. Every
synchronous provider IPC must impose deadline-aware transport cancellation; if
a provider returns after the outer deadline the service rejects, while a hung
in-process call cannot be cancelled by this foundation. The service never
accepts an ambient path, environment variable, default endpoint, or raw private
key.
