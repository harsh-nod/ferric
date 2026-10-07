# Native Fixture Runner

`probe.py` defines36 lazy fixtures spanning all14 roots. It uses the unchanged
`proofs/tensor-parallel-kernels-v1/probe.py` protocol helper at SHA-256
`a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b`.
It never starts a worker in `--self-test` mode. CPU tests cover complete
small-N/real-K transpose and independent scalar FP32 reference arithmetic,
full-vocabulary argmax/tail behavior, causal page tables and exact head mapping.

The root GPU coordinator must hold its lease, verify all eight physical device
identities and idle state, and provide a fresh0700 output directory. Reserve
at least12GiB host headroom and2GiB free memory on the selected GPU. The largest
individual weight buffer is311,164,928 bytes; cases are generated and freed
one at a time. The runner is diagnostic and exports no accepted model rates.
Invalid or nonfinite fault fixtures remain CPU-only.

```sh
python3 -B probe.py --self-test --helper /pinned/probe-helper.py
python3 -B probe.py --run --operational \
  --helper /pinned/probe-helper.py \
  --worker /pinned/worker --worker-sha256 WORKER_SHA256 \
  --artifact /pinned/observation.hsaco --artifact-sha256 IMAGE_SHA256 \
  --device-unique-id SELECTED_PHYSICAL_ID --output /private/fresh/result
```

Root must separately pin this runner, source commit, observation manifest and
compiler handoff. The normal successful result has schema
`FerricDraftBatch32SyntheticKernelProbeV10`,36 unique fixture records,14 unique
symbols, `clean_teardown: true`, `checks_pass: true`, and no model or performance
qualification. Check the outer exit status and before/after idle receipts as
well. An output file alone is not an execution receipt. Preserve any failed
attempt instead of replacing its result with a retry.

The two attention fixtures deliberately distinguish Q16/KV8 ratio2 from the
target ratio4. One uses17 rows at positions15/16 and poisoned inaccessible
future entries; the other reaches logical8191/physical511. Append checks32
distinct writes across15/16 at the last physical page and page0. Projection
fixtures retain exact dyadic operands, every active output and all capacity32
tail bytes. The FP32 head/argmax separation uses generic fixture IDs7/101,
not any watched real-model token IDs.
