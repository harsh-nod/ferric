# Guarded Layer-Zero Capture

This opt-in engineering diagnostic retains the current guarded model's
layer-zero intermediate tensors for comparison with an independent reference.
The normal decode route, wire format, kernel images and runtime policy are
unchanged. It is not a numerical acceptance or performance result.

## Worker Qualification

The fresh MI350 worker qualification passes all nine phases: **591 passed,
zero failed, four unchanged ignores**. All five new capture tests pass. The
full 595-name inventory preserves the previous 590 outcomes. Source,
dependency, private-cache and process postchecks are clean; all children exit
naturally and are reaped. No GPU execution occurs in this CPU qualification.

- [Actual result](worker-cpu-v1/evidence/complete.json): 1,607,204 bytes, SHA256
  `64855967eb15f5b5b5fadb98cbc30e8e382968099ec439090562cacc174eec47`.
- [Raw tests](worker-cpu-v1/evidence/worker-tests.stdout) and
  [tested source map](worker-cpu-v1/evidence/sources-after.json).
- Final worker ELF: 5,785,096 bytes, SHA256
  `c747212d88ef54532194792bdcc2d2dbe35610abdd54d39ce2affe6ec57653e7`.
  The binary remains on MI350; its body is not committed.
- [Retained manifest](worker-cpu-v1/manifest.json): 255 pinned bodies plus the
  manifest, including all 181 worker sources and all 50 raw evidence files.
  The transport archive is 1,752,873 bytes, SHA256
  `0a9aef046d90dc7c4160379aae3b1484fc71f3efae86b3396f146a45be329fc2`.

The exact eight formatted worker source changes are integrated. The runtime
is the previously qualified 807-file interface source generation, unchanged.
Use fe2o3 engineering revision `e9ccc629cf1d445bda4d792276a1defb2e386d88`
for the canonical relative worker dependency. [CPU recipes](CPU.md) retain
the source lineage, bounds, private caches and selected toolchains.

## Capture Boundaries

The explicit worker flag is
`--engineering-native-guarded-mlp-stage-capture-v1`. It captures only the first
forward, position zero, layer zero: 34 typed parts totaling 256,136 bytes.
These include the input, metadata/RoPE data, normalization, QKV/attention,
O-projection partials, first residual, MLP intermediates, dedicated Down
partials and final hidden values, on both ranks.

Prefix outputs are sampled after Prefix completes. R1, MLP, Down and final
hidden are sampled after the entire paired segment completes, from retained
buffers. Capture does not split that segment or insert waits between packets.
The guarded Down buffers are distinct from the old shared projection scratch.

The report is emitted only after four complete forwards and healthy native
Close. It contains original bytes with explicit stage/role/type/offset/hash
metadata, not substituted reference values. The matching parent mode validates
the report after child retirement against the actual request and layer output.
Parent CPU qualification and a fresh GPU capture are still pending at this
checkpoint. Authored parent sources in the source manifest are not promoted
by the worker test result.

## Remaining Gates

1. Qualify the capture-enabled parent, including its new validation tests.
2. Execute a fresh bounded GPU capture and verify shutdown and raw evidence.
3. Compare the current stages with the genuine independent framework capture.
4. Establish full-model numerical acceptance before sustained benchmarking.

Capture overhead excludes this mode from timing comparisons. No GPU overlap,
model speedup, sustained 2,048/256 result or 700 tokens/s claim follows. All
issue #42 M0-M7 milestones remain open.
