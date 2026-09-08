# Ferric M1 R33 production owner V1

This standalone service is the non-test owner for Ferric's authenticated R33
resident backend. It consumes only externally supervised capabilities and
strict, canonical deployment inputs. It does not create a verifier, checker,
signer, compiler-current authority, Worker V3 publication, or model bundle.

The supervisor must transfer these exclusive descriptors:

- FD 195: fe2o3's one-use compiler-current application endpoint;
- FD 196: an already-connected protected-verifier `SOCK_SEQPACKET` endpoint;
- FD 197: an exact 32-byte sealed memfd containing an unpredictable Begin
  challenge that the supervisor has already durably reserved and globally
  replay-excluded.

Run the service with one canonical owner plan:

```text
ferric-m1-r33-production-owner-v1 serve OWNER-PLAN.json
```

`validate-plan` performs only structural and held-file validation. It consumes
no inherited capability and grants no execution authority.

The owner authenticates the exact aggregate V2 Worker publication through the
production verifier, binds the authenticated program set to the declared
runner closure, authenticates the canonical Qwen3-8B/Qwen3-0.6B model bundle,
acquires the exact checked gfx942 device through KFD, initializes model and KV
memory, constructs the complete 20-window resident input roster, and then
serves the bounded R33 lifecycle. Every service failure consumes the backend
through its explicit close path and retains any quarantined native custody
until process exit.

This executable is not an HTTP server and is not serving, performance,
correctness, qualification, or M1 evidence. Real execution still requires all
external protected services, a receipt-bearing aggregate publication, the
canonical model snapshot, and an R33 collector plan.
