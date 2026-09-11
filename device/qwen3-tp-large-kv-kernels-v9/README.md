# TP1 Large Physical KV Profile V9

This additive gfx950 engineering image contains exactly two roots. It keeps
the frozen v5 profile and all existing arithmetic unchanged. It is not a new
default, protected proof, native/model result, or performance qualification.

- `ferric_qwen3_tp_batch32_large_kv_append_v9`: unchanged 112-byte explicit
  append ABI, Wave64, exactly one workgroup, grid-leader exclusive writes.
- `ferric_qwen3_tp_batch32_large_kv_paged_gqa_bf16_f32_v9`: unchanged 116-byte
  baseline attention ABI, Wave64, `rows * 32` workgroups, at most 1024.
- Both require TP1, rows 1..32, physical pages 1..16384. Logical positions
  remain below 8192 and per-row page-table stride remains at most 512.
- A full K or V cache has 268435456 BF16 elements (512 MiB); their largest
  element index is 268435455. All 36 layer pairs total 36 GiB. Weights,
  workspaces, queues, host payloads and availability headroom are additional.
- Append retains complete duplicate-slot validation before any write.
  Attention retains causal bounds, serial QK reduction, online softmax order,
  finite checks and BF16 rounding. No wave attention or RoPE root is added.

The host must explicitly bind the separate v9 image before allocation and
reject unsupported wave/peer/sequence/capture/replica combinations. An actual
pool size at or below 512 pages does not make v9 a legacy profile. The host
integration owns profile identity, logical/physical allocation separation and
page ownership; this crate must not silently weaken them.

The kernel bodies and every inline helper are checked against the existing
v5 source after only the declared symbol, TP1-admission and physical-bound
substitutions. Fresh typed emission and root-owned native/model gates remain
required. All builds, tests and emission run on mi300x; the integration lead
alone schedules GPU probes on mi350.

## Native Plan

`tools/probe.py` uses the unchanged SHA-pinned engineering worker helper. Its
nine-case roster covers append and attention at rows 1/16/17/32, page counts
513/8192/16384, physical pages 511/512 and the last page, the final physical
slot, reverse page placement, all active outputs and untouched tails/guards.
Append uses a final logical position 8191; only the one-row attention case
traverses the full logical 8192 context. Other attention cases use at most 17
causal tokens, with future table entries and cache data intentionally invalid
but unreachable. Invalid required accesses and nonfinite traps are host-only.

The largest live device cache pair is 1 GiB; inputs/output/guards are additional.
Full-byte readback and reference custody can transiently use several GiB of
host memory, so the harness requires at least 12 GiB available host headroom.
Root must independently check actual GPU memory headroom, global idleness and
owned-process teardown. No deliberate GPU trap or performance claim is made.

Run host-only checks on mi300x:

```sh
python3 tools/probe.py --self-test --helper /path/to/proofs/tensor-parallel-kernels-v1/probe.py
python3 -m unittest discover -s tools -p test_probe.py -v
```

Root-owned native invocation after artifact admission:

```sh
python3 tools/probe.py --run --operational --helper /absolute/pinned/probe.py \
  --worker /absolute/worker --worker-sha256 WORKER_SHA256 \
  --artifact /absolute/observation.hsaco --artifact-sha256 HSACO_SHA256 \
  --device-unique-id EXPECTED_PHYSICAL_ID --output /absolute/fresh-private-result
```

The report records the actual helper/source/worker/image identities, individual
dispatch receipts and full-array hashes; it confers no M1 authority. Dispatch
times are diagnostic only, not model TTFT/TPOT or serving throughput.
