# Finite TP1 Query-Hoist Parity Profile

CPU-checked candidate; no GPU run is claimed. The native wrapper
fails before loading host support while STAGE, WORKER, IMAGE_DIRECTORIES and
PROBE_SOURCE remain unbound. Root must review and freeze that small binding
with actual worker/device/path identities before a separately authorized launch.
The pinned legacy lifecycle/checker must remain compatible with the chosen
stage and worker; emission-worker source216 is not a native-worker assumption.

## Existing Bytes Only

`probe.py` reuses the exact generator
`../tp1-attention-kernels-v1/probe.py` at SHA
`6a7c7baf533a9fa7e0773126519992bb62dfdaefa3ed5f239729319fd6a7f789`,
and its exact core `../tensor-parallel-kernels-v1/probe.py` at SHA
`a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b`.
Neither old source, fixture, kernel, compiler, inventory nor route is changed.

The four existing input families are uniform rows1/17/31 and selector rows32.
Each dispatches resident Wave then V14, for exactly8 cases. TP1/context32,
capacity32, Qheads32/KVheads8/dimension128, page16/physical68, reversed remapped
pages and positions0/1/14/15/16/17/30/31 are unchanged. The one-row case uses1.
Names and symbols are the only changes from each frozen `wave` fixture.
Each case is validated by the old validator before/after the closed rebinding.

All floating inputs are finite, with nonzero dense QK, token-varying V,
Hadamard patterns spanning bit6/bit0, finite future/decoy values and inactive
output tails. Exact integer BF16 expectations and the no-selected-token
fallback come from the frozen oracle; no tolerance or new softmax solver.
See the original fixture PLAN for arithmetic and independent mutation coverage.
The V14 ISA separately retained under the emission evidence uses the expected
scale0x3db504f3 at0x293c and exp lower clamp0xc2ce8ed0 at0x266c with zero selects
0x29ec/0x2a18, supporting the same finite selector premise. This is engineering
evidence, not a general floating-point/exp proof.

## Two Closed Images

- Resident Wave v5 image98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502,
  original sourceeff229/compiler3e74.
- V14 image8f21681fe9103b670ee5666f429a45682e77fc90eb16802a02c4b6fb93a192c8,
  source1843d8f/compiler8efd. Its typed handoff is retained as a digest only.

Full source/compiler/root/image/observation/handoff pins are in `probe.IMAGES`.
The images use different compilers: no same-compiler performance ablation.
Both expose six slices plus fiveu32:116 explicit bytes, hidden120+256,
kernarg376/align8, WG64/Wave64, zero private/LDS. Every native result independently
binds its role, root, image digest and full loaded ABI/resource metadata.

## Admission And Lifecycle

One worker holds both authenticated image files, uses the old operational
configuration and core probe, and closes normally after the eight cases.
Any configuration, dispatch or close failure aborts and cannot return an
admitted report. The wrapper also rejects nonzero/boolean statuses, any forced
cleanup, a remaining worker PID, missing/reordered cases or incorrect provenance.
It reconstructs all48 complete buffer hashes and96 guards: every input byte,
active output and inactive output tail. Each output has262144 bytes.

The root wrapper is a narrow fork of the accepted TP1 lifecycle: fixed frozen
parser/support pins, exclusive nontruncating shared lease, all-eight-GPU idle
before/after, host2GiB/VRAM1GiB headroom,600s child bound,8MiB logs, held source/
image/worker pins and separate raw receipts. No launch is authorized by an
engineering observation's `exact_output_replay`, which is compiler byte replay.
Its publication/load/launch grants remain false.

## Focused CPU Gate

Source `fec0311` passes **8 profile +7 wrapper =15 synthetic test methods**
on mi300x in 52.574 seconds. They use real frozen-generator bytes and realistic complete
metadata, not miniature placeholder buffers. Cached generated cases reduce
test construction cost while retaining the real case validator. Tests cover
paired bytes/order, closed roots/geometry/tails, source-loader mutations,
normal/configuration/dispatch/close paths, all48 expected buffer checks,
complete argument/resource metadata, observation pins/grants and unbound launch.

The unchanged old10 fixture methods pass separately in 15.601 seconds, without
mixing their `probe` module with the new one: total25 methods passed, plus the
exact eight-case CPU self-test. All three groups close without forced cleanup;
the complete archive `12d834216e717dc2df1642494233b664225e3fab6b2194a1c1dd8b051b6c9fa5`
has matching independent streams and all39 file hashes/sizes pass root checks.
The source/control bytes remain unchanged. These are fixture checks, not GPU
results. Any repeat needs a separately reviewed bounded remote CPU gate.
Set `FERRIC_ATTENTION_FIXTURES`,
`FERRIC_ATTENTION_HELPER` and `FERRIC_PAIRED_CHECKER` to authenticated staged
copies; the checker must be SHA256
`2568a3dce9e6031012f548608deb6ce5c1de8914d53612ac38539ab576af5cf9`.
No local test/build/formatter, dependency installation, native execution or
control binding is part of this source change.

Even a future PASS admits only these finite TP1/context32 cases. It does not
admit TP2/8, context8192, model inference/parity, serving, general numerical
accuracy, Verus proof, transactional behavior on invalid pages, default
promotion or a performance gain. Worker dispatch timings are retained only
as raw protocol fields and excluded from every comparison.
