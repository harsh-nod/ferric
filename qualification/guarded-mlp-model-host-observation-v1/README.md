# Guarded MLP Host Observation

This optional engineering mode records inclusive host counters around setup,
each layer's prefix, paired MLP/residual segment, hidden-state retention, and
four autoregressive forwards. It preserves the conservative runtime policy,
wire protocol, GPU images, ownership checks, and healthy-close requirement.
It cannot be combined with tensor-stage capture.

## Qualified Worker

The MI350 worker qualification passed all 601 selected non-ignored tests, with
four unchanged ignores, across nine clean phases. The full worker inventory is
605 names. This includes ten new tests; all 595 inherited outcomes are preserved.
The ten integrated source changes are the actual formatted postimages, and all
184 canonical worker source files match the retained qualified source bodies.

- Original receipt: [worker CPU result](worker-cpu-v1/evidence/complete.json),
  1,625,743 bytes, SHA-256
  `fd4b55f53e2a61aafb49bb901796ca6c5d72a2e34d725aabc16cd4d14cef082e`.
- Worker ELF: 5,873,056 bytes, SHA-256
  `9221a902a52a07b96b6855c8b802d4e634cbc0970e3f1e252d4130cf6bc1753d`.
- Export archive: 1,764,646 bytes, SHA-256
  `b66acb1536b6668e411ee8129b9578843e1fea429505c2c5941ea541ad3fcfbb`.
- Retained capsule: 259 files, 258 pins, 50 original raw files; 11,593,547
  expanded bytes. External runtime, dependency, cache and ELF bodies were
  rehashed remotely; this is not a self-contained toolchain or executable bundle.

## Interpretation

The intended report has 587 snapshots and 586 checked intervals. Rank/group
identity, queue epochs, actual worker and completion history are joined before
publication after healthy close. Counter scopes can nest and overlap; adding
them does not yield elapsed GPU time. The paired route does not populate every
generic dispatch timer. No GPU overlap graph can be inferred from these counters.

This checkpoint qualifies worker source and CPU behavior only. Native host-mode
execution, numerical acceptance, throughput, and production qualification are
not established here. The four-forward diagnostic is not the requested
2,048-token prompt / 256-token decode benchmark. Ferric #42 M0-M7 remain open.
