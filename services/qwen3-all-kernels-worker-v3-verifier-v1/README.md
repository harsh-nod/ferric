# Ferric aggregate Worker V3 protected-verifier service foundation

This standalone package owns Ferric's fail-closed V2 service orchestration for
the 12-entry Qwen3 aggregate roster. It is deliberately outside the legacy
Ferric workspace and uses fe2o3's multi-phase Worker V3 transport at commit
`43ada6c5029d2daf62908fd1cfa86ee56cc4d9eb`.

The foundation provides:

- caller credential plus pinned policy/measurement admission;
- append-only, checksummed, descriptor-backed replay and reservation ledgers;
- service challenges read from a supervisor-provided entropy descriptor and
  durably burned before release;
- a single absolute session deadline;
- a second bounded read and digest check of the retained V2 envelope and HSACO;
- canonical V2 envelope and compiler-current-record association;
- explicit protected current-record, independent checker, and external signer
  provider contracts;
- exact 12-entry request/result joins and a terminal Ferric V1 response payload;
- stage-specific rejection and terminal-send custody.

This is **service foundation, not deployment closure**. Production still needs
reviewed measured processes implementing the current-record authenticator,
theorem checker, and signing provider, plus a supervisor that supplies only
preopened descriptors and pins their identities. The service never accepts an
ambient path, environment variable, default endpoint, or raw private key.
