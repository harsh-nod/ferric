# M1 Speculative KV Indexing Contract

This document records the source-level indexing required before speculative
completion can be composed with target and draft paged KV. It is not a runtime,
machine, qualification, or ROADMAP closure claim.

For a round of width `K`, the last accepted token is the unprocessed round
anchor at the shared pre-round target/draft cursor. Target graph ordinal zero
consumes the anchor, and target ordinal `i + 1` consumes draft candidate `i`.
The target therefore has `K + 1` active inputs, matching the generated graph.
Draft ordinal zero consumes the anchor, and draft ordinal `i + 1 < K` consumes
candidate `i`; the final candidate was produced rather than consumed by the
draft, so its graph has `K` active inputs.

Target choice `i` is the logit after target input ordinal `i`. Choices are not
KV entries. If `A` candidates are accepted, choice `A` is the correction or
bonus emitted after the accepted prefix. That token is explicitly
`DeferredUntilNextStep`; this contract does not invent a KV write for it.

The tentative intervals are:

```text
target: [cursor, cursor + K + 1)
draft:  [cursor, cursor + K)
```

For every `0 <= A <= K`, the target commit end is `cursor + A + 1`. The draft
commit end is `cursor + min(A + 1, K)`. Positions from each commit end to that
role's tentative end are the rejected suffix. The fixed commit tables make
every accepted-count case explicit and reject noncanonical unused entries.

`SpeculativeKvRoundIndex` additionally binds the exact request generation,
completion epoch, plan identity, target/draft selections, finite bucket, input
tokens, cursor, intervals, and commit tables. It exposes deterministic ordinal
and target-choice lookup methods and a fail-closed exact-authority validator.

The contract does not yet apply `StepPublication`, commit or roll back physical
pages, publish tokens, or prove a generated runner. Queue, device, allocation,
address, kernel, HSA, machine, and performance refinement remain outside it.
