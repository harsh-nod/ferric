# Paired Terminal Validation

The explicit gfx950 runtime API passed CPU qualification on `mi350` and is
integrated in fe2o3 commit `686f9edd2e`. A separately qualified
[warm-only Ferric caller](../guarded-mlp-warm-paired-terminal-v1/README.md)
is now integrated after 673 worker and 438 selected parent test passes.
Its matched native control/candidate comparison now passes on MI350 with
complete payload/history parity. The linked caller report records a 25.483%
lower warm segment host sum in one pair, not GPU timing or an end-to-end
speedup.

## Change

`Group::dispatch_guarded_mlp_pair_paired_terminal_v1` opts a single dispatch
into paired terminal validation, only for genuine `ReuseRetired` storage.
Fresh storage is refused before accessing the device contexts. Existing
dispatch methods, default policies, queue limits and reusable-bank lifecycle
checks are unchanged.

After both ranks have logically retired, a private transaction performs one
full Group check on entry and another on exit, replacing eight full checks
across terminal observation, rank completion and finish. It retains six
intermediate queue/fault probes, three acquired state snapshots per rank,
all ten actual completion signals, owner/generation/retirement checks and
absolute deadlines. Neither completion state nor a result is published before
the trailing check succeeds. Error, timeout or unwind quarantines the Group,
staging state and both owners, including failures around partial commit.

This deliberately changes the method-local sampling cadence of complete
topology/aperture/reset observations. It does not prove equivalence to checking
those facts at every old boundary. No allocation, mapping, packet publication,
callback or sleep occurs inside this terminal transaction; it is not a
general-purpose cached-currentness API.

The source-level reduction is six full Group checks per paired segment, or
216 over 36 layers. This is a call-count prediction, not measured latency,
GPU overlap or an end-to-end improvement.

## Actual CPU Results

The [original MI350 receipt](cpu-v1/evidence/complete.json) records 23 naturally
completed phases with clean source, process, tool, dependency and cache
postchecks. CPUs 8/9, two Cargo jobs, private offline caches and hidden GPU
visibility were used. Native ignored tests were not run.

| Check | Actual Result |
|---|---|
| Full ordinary KFD selection | 1,119 passed; eight unchanged ignores |
| New terminal and admission tests, included above | 11 passed |
| Unchanged Ferric worker selection | 658 passed; four unchanged ignores |
| Selected facade doctests | 10 passed |
| Rustdoc parser regression cases | Eight passed |
| Runtime/worker executable products | 11 built and pinned |

Tests cover refusal and unwind before and after effects, deadline expiry,
rank-one and snapshot drift, all ten signals, lagging queue credit, public
signature constraints, Fresh storage and missing retirement proof. The actual
worker executable was pinned before/after its EOF integration probes and
matched the final build product. No new worker route was added by this change.

The terminal is 2,250,677 bytes, SHA-256
`bf0fa20a37ff3bf748ef5984df0dceb5cd1c453ef5c05c7111779b31cda33720`.
Its 141.47 seconds is CPU qualification wall time, not inference latency.

The [retention map](cpu-v1/manifest.json) covers 158 archive members,
including 122 raw bodies, six tested runtime postimages, source maps, lineage,
manifests and helpers. The [integration map](source-integration.json) records
all four replacements, two additions and actual formatter postimages. The
reduced qualification workspace and locks are retained as evidence, not copied
over canonical workspace metadata.

## Remaining Gates

A matched native comparison of the separately qualified opt-in Ferric caller
now preserves full payload/history equality and clean retirement, using the
same reusable storage and currentness policies. Repeated controlled timing,
full-forward latency and sustained workload performance remain unmeasured
for this route. The independent Readiness40 numerical discrepancy
is a separate correctness investigation, not fixed or explained by this API.
All issue #42 milestones and the 700 tokens/s target remain open.

The [source-derived bandwidth budget](roofline-v1/README.md) estimates what
700 tokens/s would require for BF16 TP2 single-request 2,048/256 decode.
With the current full rank-0 LM head and a conditional 8 TB/s peak per device,
the mean-context streaming floor is 1.044 ms/token and the target needs about
73.1% of rank 0's peak bandwidth before nonoverlapped overhead. This is a
planning model, not measured GPU traffic, throughput or an attainability claim.
