# Retained Four-Forward Diagnostics

Unfrozen draft. Twelve tests are authored, not run. This is a data-only CPU CLI:
it launches no native process, imports no framework, and supplies no acceptance
tolerance. Root must invoke it only after the selected GPU controller and native
process tree are terminal/reaped, under a separately bounded CPU runner.

```sh
python3 -B run.py --observation "$ACTUAL_COMPLETE" \
  --observation-sha "$ROOT_REVIEWED_SHA256" --output "$FRESH_DIRECTORY"
```

The observation FilePin's extent is discovered only after its supplied digest
authenticates the actual bytes. The fixed observer package is the actual
84-test generation, manifest SHA256
`10125c91f9c83c55379c5769b2d1bfa099835744b10f292c9ebd5f152ae7e9c3`.
Its frozen intake loads the unchanged V2 comparison source and exact helper
closure. Module aliases are temporary and restored; neither `I.context` nor any
launch/owned-process method is invoked.

## Scope

The CLI checks the completion's explicit one-attempt/no-retry scope and plan,
mode, policy, selected-image, and source-generation identities. It rehashes all
35 retained files from seven owned leaves and the six topology records, checks
recorded natural exits/reaping and the original audit command/resource envelope,
and re-runs the native/host-sidecar structural validator and new observation
gate on all fourteen native files. Recomputed observation bytes must agree with
the pinned stored observation and completion result.

This replays recorded evidence; it does not perform new topology/process audits,
reconstruct deployment/source authority, or prove the outer controller has
exited. Those launch prerequisites are bound by the authenticated completion and
remain root-reviewed prerequisites. All new provenance/runtime/outer-reaping
authority fields stay false. Nothing in this CLI can authorize a GPU launch.

The unchanged `compare.py::reference` revalidates the exact mode-specific
independent framework reference, both retained passes, all tensor/cache hashes,
and its original input trajectory. The unchanged `compare_rows` reports up to
152 tensors: 36 layer hidden states, final normalization, and logits at four
positions. It reports exact BF16 word counts, maximum absolute error, RMSE,
relative L2, and maximum BF16-step distance. It never chooses a threshold.
After autoregressive input-history divergence, later `tensors` remain null,
even if a later individual input token happens to coincide again.

`passed=true` means the retained diagnostic completed, not that the candidate
was mathematically accepted. Numerical/model/production/performance flags remain
false, and `acceptance_threshold` remains null. TF4 and AR4 references cannot be
interchanged. This is not the 2,048-prompt/256-decode workload or a throughput run.

## Evidence And Tests

The fresh output directory contains actual `sources-before.json`,
`sources-after.json`, and `complete.json`. The source maps include the comparator,
all authenticated package sources, and this CLI's three files. All consumed
pins are rehashed, and source maps must match before completion is written.
Failures produce no successful completion; an already-written initial source
map may remain as failure evidence. No existing evidence is overwritten.

The authored tests cover routing, fatal reference-pin failure, exact audit/leaf
joins and limits, false authority boundaries, and history incomparability. One
test loads the actual pinned comparison source but uses tiny synthetic tensor
adapters to test its history logic; it also rejects the TF reference in AR mode.
These are not real-model numerical results. Root must freeze/run the tests and
then run the actual retained-data diagnostic separately.
