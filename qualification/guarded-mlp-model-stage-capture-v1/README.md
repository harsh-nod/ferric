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

## Parent Qualification

The fresh MI350 parent qualification passes all 54 phases and 46 selected test
scopes: **380 passed, zero failed or ignored**. It preserves every prior selected
pass and adds exactly four capture-validation library tests and one guarded
binary test. The complete 862-name library inventory is retained; the full
parent library suite is not executed. All five required parent products are
built, and source, dependency, private-cache and process postchecks are clean.

- [Actual result](parent-cpu-v1/evidence/complete.json): 3,787,634 bytes, SHA256
  `f1dd4cf50c704bc1f39472df3a8299872de5490fcf073160077888629192ee91`.
- [Capture and inherited client tests](parent-cpu-v1/evidence/parent-client.stdout),
  [guarded binary tests](parent-cpu-v1/evidence/guarded-bin-tests.stdout), and
  [tested source map](parent-cpu-v1/evidence/sources-after.json).
- Final guarded parent ELF: 13,673,168 bytes, SHA256
  `f77b78a963d20fac64d4ce0c7ffd6f0d028d2d147aa4957dc47160abb4cfe25d`.
  Its body remains on MI350 and is not committed.
- [Retained manifest](parent-cpu-v1/manifest.json): 399 pinned bodies plus the
  manifest, including 274 raw evidence files, 102 lineage bodies and 12 selected
  source bodies. The transport archive is 2,851,054 bytes, SHA256
  `4b07d6b424624fb8926cca56df242d6844d072a130e5b2ed0f7b16d8fff10be1`.

Only the four formatted parent source rows from this capsule are integrated.
Its eight worker overlay bodies are the authored parent-build inputs, not the
separately formatted worker bodies qualified above, and are not copied over
those canonical worker sources. This CPU result does not establish GPU capture
or numerical acceptance.

## Capture Checker Qualification

The independent capture/parser gate passes all **12 tests** on MI350: eight
capture cases and four device-identity cases. The owned test process exits
naturally, is reaped, and leaves no process group; all input and tool postchecks
are clean. [The original result](checker-cpu-v1/evidence/complete.json) is
10,882 bytes, SHA256
`642d4ee30d0ad42ae849e17c42b9cc10b69ce61f7cbcb581026595b8d83e324a`.
The retained directory includes all six inputs and seven raw test files plus
the terminal report. This gate tests synthetic data, not model numerics.

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
The parent CPU qualification above independently covers this validation path.
Actual GPU capture evidence is a separate publication gate; the CPU capsules
do not promote a native outcome or independent numerical acceptance.

## Remaining Gates

1. Publish the separately audited GPU capture, including shutdown and raw evidence.
2. Compare the current stages with the genuine independent framework capture.
3. Establish full-model numerical acceptance before sustained benchmarking.

Capture overhead excludes this mode from timing comparisons. No GPU overlap,
model speedup, sustained 2,048/256 result or 700 tokens/s claim follows. All
issue #42 M0-M7 milestones remain open.
