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

## Qualified Parent And Checker

The [parent CPU result](parent-cpu-v1/evidence/complete.json) records 55 clean
phases and 387 selected passing tests across 47 scopes. All five selected host
binaries built. The 868-name library inventory was listed, not fully executed.
The original SSH connection dropped after the run completed; the original
receipt was recovered and authenticated, with no qualification rerun.

The parent receipt is 3,815,416 bytes, SHA-256
`ab72e2316315ec6b0766284e0b4c26e834b2502cd4a6fc29c304a875950e3c8f`.
Its guarded parent ELF is 13,836,368 bytes, SHA-256
`abf358b7fdcce0e2c432aa906d06c8790a3727d5471993ee01879d74a851f5c9`.
The 2,873,734-byte export archive has SHA-256
`66c0279008c04f16256539cd123019f01cbdf0b2a064898a411265442ee0f4a2`.
The retained capsule contains 407 files, 406 pins and 279 original raw files,
with 23,408,088 expanded bytes. Parent integration uses only the three tested
parent postimages, not the authored worker overlay stored in this capsule.

The [synthetic checker](checker-cpu-v1/evidence/complete.json) passed all 14
tests: ten host-report cases and four topology cases. Its six unchanged inputs,
seven original raw records and terminal are retained. The terminal is 11,136
bytes, SHA-256
`d31177e489652ced325a473577ca1114af44e0211e76c1db1203527f5abb59f1`.
The sole child exited naturally and was reaped, with a clean source/tool check.

## Reading The Counters

The intended report has 587 snapshots and 586 checked intervals. Rank/group
identity, queue epochs, actual worker and completion history are joined before
publication after healthy close. Counter scopes can nest and overlap; adding
them does not yield elapsed GPU time. The paired route does not populate every
generic dispatch timer. No GPU overlap graph can be inferred from these counters.

## Actual MI350 Execution

The [native host-mode run](gpu-attempt-v1/ar4/complete.json) passed in one
attempt with 11 clean process phases, six idle snapshots, healthy Close, and
all 587 snapshots / 586 intervals verified. It completed four forwards through
all 36 layers on both ranks, producing `67, 25, 576, 2701` from the own-output
history starting at token `9112`.

The [instrumentation comparison](gpu-attempt-v1/ar4/instrumentation-comparison.json)
finds all four 606,976-byte payloads and input histories equal to the earlier
ordinary guarded AR4 run. The capsule retains both sets of payloads so this
comparison can be checked offline. This establishes observed byte equality,
not an independent numerical-accuracy claim.

The paired segments account for 7.924-8.004 seconds of host wall time per
forward. They record 2,016 full-currentness checks per rank per forward,
with 3.403-3.441 seconds per rank in those inclusive counters. Publication
checks are recorded separately and must not be added indiscriminately to
nested counters. This motivates a separately qualified shared-full-currentness
experiment; it does not measure GPU kernel duration or an optimization gain.

The [integer host-accounting analysis](analysis-v1/README.md) also passes on
MI350: ten synthetic tests, eight clean input posthashes, and exact disjoint
wall reconciliation. [Measured tables](analysis-v1/summary.md) keep nested
rank/shared counters separate from the prefix/paired/hidden/other wall totals.

The original terminal is 614,572 bytes, SHA-256
`91a20d628644eb784d651e4d4d87571cb55cdb367971092fcea40d259ee38ec2`.
Its controller elapsed time is 410.540 seconds, including setup and audits,
not a throughput denominator. The raw host report is 510,613 bytes, SHA-256
`ed930946ce1402171cdfb3a830f883dfda7fc6f7bd76763af238de54872c90c4`.
The 3,929,492-byte archive has SHA-256
`1bebcaec4eb2dd74600193981173d25b2da39ac0df861c48d36b04e3ca001d31`.
The retained capsule contains 97 files, 96 pins and 79 current raw records,
with 7,907,742 expanded bytes. Five files are the original ordinary-AR4
reference terminal and four payloads. External executable/library/readset
bodies are not bundled. The exporter and retainer perform data checks only.

Numerical acceptance, sustained throughput, and production qualification remain
open. This four-forward diagnostic is not the requested 2,048-token prompt /
256-token decode benchmark. Ferric #42 M0-M7 remain open.
