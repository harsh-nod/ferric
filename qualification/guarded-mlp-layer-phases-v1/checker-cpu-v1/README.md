# Layer-Phase Checker CPU Qualification V1

This packet qualifies strict, read-only admission of the feature-enabled
Readiness40 Tail layer-duration record. It does not contain a native model run
or measured layer timings.

## Observed Result

| Leaf | Passing tests | Failed / errors / skipped |
| --- | ---: | ---: |
| Preserved legacy checker suite | 126 | 0 / 0 / 0 |
| Preserved forward-phase V1 suite | 20 | 0 / 0 / 0 |
| New layer-phase V2 suite | 32 | 0 / 0 / 0 |
| Total in this qualification | 178 | 0 / 0 / 0 |

The original controller completed in **78.00758577603847 seconds**.
All three leaves exited naturally with code 0, were reaped, and had absent
process groups; no forced cleanup was required. These are synthetic data
admission tests, not model correctness tests or performance measurements.

The closed source map contains 30 bodies: 28 checker/test modules plus the
controller and pinned supervisor. Source identities were unchanged across the
run. The archive contains 52 members: those 30 bodies, the input manifest,
17 original raw evidence bodies, the original terminal, the authoring source
seal and stager, and the archive manifest. The archive manifest pins the other
51 members.

## Admission Scope

The active mode is `tail_layer`. The new third-record schema is
`FerricReadiness40ForwardLayerDurationsV2`, under the existing explicit
Readiness40 Tail route and default-off
`engineering-currentness-duration-diagnostics` feature.

Admission authenticates the entire original three-line stderr file. The first
canonical policy line and second currentness-duration line retain their
original bytes; the third record binds both LF-inclusive hashes, session,
worker identity and own transcript. No V2 record is relabeled, normalized or
discarded to obtain V1 acceptance. Both the 32,768-byte third-record bound and
the unchanged 69,632-byte whole-stderr bound are enforced.

There are exactly 40 ordered forward rows. Positions 0 and 1 have no measured
layer metrics. Each of the 38 warm rows declares exactly 36 layers, for 1,368
warm layer returns. The wire carries one aggregate per forward, not a log of
individual layer returns.

The nine forward phases retain their exact checked disjoint sum. Within the
forward `layers` phase, six ordered closed-layer stages sum exactly to the
closed-layer body:

`enter_pre_census, prefix, mlp_retired_seal, hidden_post_census, full_exit, commit_prepare`

The seven paired-MLP stages sum exactly to a body contained in the closed
`mlp_retired_seal` interval:

`preflight, consume, reserve, publish, poll, retire, terminal`

The aggregate layer callback subtotal must fit within the closed-layer body,
which must fit within the enclosing forward layer phase. These nested totals
are not additional disjoint forward time. Checked integer widths, overflow,
fixed orders, cold/warm presence, authority flags and duration bounds remain
fail-closed.

Individual-return validation, including callback containment before either
worker accumulator changes, belongs to the separately qualified Rust
runtime/worker path. The aggregate wire cannot independently reconstruct or
prove each individual return.

The full-case and same-side tests retain the original Healthy Close,
transcript, four capture bodies and 124 parent-host-span custody checks.
Negative cases include malformed, duplicate and noncanonical records;
identity/hash drift; wrong stage counts or order; missing warm metrics;
overflow and containment failures; repinned semantic changes; and altered
whole-file evidence. Old validators and their 146 tests remain unchanged.

## Bounds

The controller uses a 420-second whole deadline, 120-second leaf bounds and
a 50-second cleanup reserve. The CPU-only leaves run with GPUs hidden,
affinity CPUs 8 and 9, nice 10, a 512 MiB address-space limit and a 4 MiB
stream cap. The inherited supervisor preserves the 40 GiB initial and
38 GiB live free-space floors, owned-process retirement, source posthashes
and tool identity checks.

## Original Evidence

- [Original archive](../checker-cpu-v1.tar.gz): 94,069 bytes;
  SHA-256 `6086ed8b865b064861854dfc7cc7ae8aaac66fae061cba997fd8987772af2667`.
- [Original complete receipt](complete.json): 68,280 bytes;
  SHA-256 `bff2af1ddf5bfe3247e69f48595c51bf062c9dbc09af1bddd94a9384ba6026aa`.

The archive preserves the exact commands, started/result records, stdout,
stderr, source maps and deployed Python bodies. This README is commentary,
not an original test receipt.

No GPU timing, overlap, speedup, generated-token correctness, Full2303
acceptance or native launch authority follows from this CPU packet.
