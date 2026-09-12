# Layer-Only C1 Wave Projection Experiment

Status: source candidate only. No formatter, host test, build, native model run,
numerical qualification or performance claim has been executed for this revision.
Base: `4014178032c6d56e21a088934d61b5983988ae34`.

## Driver Boundary

The new terminal `configure_ordered_c1_wave_layers_fp32_argmax_v11` selector uses
the existing ordered-wave/v11 admission before enabling a separate layer flag.
The exact target/TP1/capacity32, MFMA projection, pruned FP32-v8 head, v11 binding,
wave attention, device-TP1 reduction, ordered-capable transport and freshness
requirements are retained. Large KV, draft, peers, sequences, numerical capture,
repeated selection and late selection remain rejected. There is no fallback.

The projection policy remains MFMA. Four layer command sites represent Q/K/V,
attention output, gate/up and feed-forward down: seven operations per layer.
Only an active row count of one selects the existing Wave GEMV roots and original
NxK weights. Rows 2 through 32 retain exact MFMA commands and transposed KxN
weights. The output head calls the original command method, uses the exact
MFMA-v8 kernel/weight/grid, and keeps v11 argmax. No allocation, kernel, ABI,
dispatch count, ordered residual tail, collective or default changes are made.
`layer_projection_mode()` describes layers only, not the head.

## Existing Artifact

The actual measured full v5 image already contains both needed Wave GEMV roots:

- HSACO `98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502`.
- Manifest `c559d0533907323aff7dda03215adab6326423c3f151b296e22394c9baf37d00`.
- Handoff `94807eb1b5f78ce1b9e217b464eb5a98d6a270c6aeb932d8847b5ff4315c7756`.
- Actual source `eff229bdd8339a47e84d392d3e34b83beea87383`, compiler
  `3e74a9324a5acd7107e96a4a9b5319d3dd5ecde8`; these are historical identities,
  not relabeled as current compiler output.

Full `open_batch32(..., true)` retains the exact 15-root admission and the worker
loads every inspected root. No image re-emission is required. Wave reassociates
FP32 accumulation; Q/K/V/gate/up still narrow to BF16, output/down remain FP32
partials before the unchanged residual reduction. Exact output parity is not
inferred from an unchanged head, finite fixtures or historical BF16-head runs.

## Focused Host Coverage

Seven new test methods use the `layer_c1_wave_` filter:

- Effective-mode helper equivalence to existing commands for every row 1..32,
  all seven layer shapes, and unchanged disabled behavior for all old modes.
- Full recording execution for all row counts; only the 252 C1 layer commands
  change, normalized command comparison holds, and head/multirow commands,
  allocations, metadata, selected reads, packet reservations and output ownership
  remain identical. These synthetic outputs are not kernel emulation.
- Zero/multiple selected head-row pruning and synchronous head barriers.
- Twenty-seven atomic rejection mutations and unchanged legacy selectors.
- Terminal policy freeze, prepublication invalid-selection rejection, and twelve
  failures including Wave submit/wait, ordered submit/wait, attention, residual,
  head, bad choice and packet preparation. Failures cannot mint completion and
  submitted pool pages remain quarantined.

Required future gates: remote-only format review, strict Clippy, these focused
tests, all adapter tests/old profile regressions, source policies and existing
artifact admission. No execution is authorized by this document.

## Separate Canary Composition

The additive `ferric-qwen3-layer-c1-wave-canary` reuses the existing argmax canary
runtime/reference/lifecycle with two closed profiles selected by
`--layer-projection mfma|c1-wave`. Its parser delegates to the unchanged submission
parser and additionally requires explicit `--submission ordered`; synchronous
submission is rejected. Both require the same existing wave attention, MFMA
setup, v8/v11 images and runtime flags before any effect. The baseline selects
the existing terminal API; only the candidate selects the new API. All legacy
entrypoints, parsers, four-record schemas and frozen Python checkers remain
unchanged. This is source implementation only, not an executed qualification.

New `FerricLayerC1WaveCanary{Setup,Prefill,Observation,Closed}V1` names and an
explicit `layer_projection` setup field distinguish these runs. `projection:
mfma` describes the resident/head policy; `head_precision: fp32-v8` and full
target/prefix reference validation stay unchanged. Profile/runtime disagreement
is rejected before timing files, model intake, worker creation or allocation.
Workload identity remains independent of the compared policy and uses the same
exact fixed prompt/length pins. The setup annotation adds no key or byte change
for any legacy profile. Old record emission code is otherwise retained.

Qualify both new-binary profiles at 8 then 128 outputs against the unchanged fixed
token IDs and independently decoded UTF8, with exact dispatch/cursor/lifecycle and
full unforced cleanup checks. The 128-output workload retains 83,139 packets,
135 batches and cursor255. Head commands remain synchronous; ordered groups use
the inherited 11/6 residual-tail composition in both arms. No TP8, HTTP, serving,
speculation or numerical-acceptance broadening is implied. Only after both modes
pass may a newly frozen matched ABBA measure host-wall TTFT/TPOT/throughput; old
cohorts are immutable and no gain is promised.

## Combined Gate Expectations

Against the exact4014178 baseline's780 passes/41 ignores/19 all-target result
rows, the combined source expects845 passes/45 ignores/20 result rows. This is
a pre-execution expectation, not a result:

- Seven new library tests from the driver/helper revision.
- Four new shared runtime profile tests in each of the three existing canary
  binaries:12 additional invocations, no new ignores there.
- The new binary:45 passes/4 ignores. Breakdown:3 contract passes/1 frozen-golden
  ignore,11 shared runtime passes,5 attention-parser passes,5 submission-parser
  passes,5 new layer-parser passes,2 host-timing passes,14 worker passes/3 worker
  fixture ignores. No ignored fixture is claimed executed by ordinary all-targets.
- One new source-policy test, bringing that integration target to29 methods.
- Eight doctests remain unchanged. Compiler/source protected policies and five
  inventories are unchanged; no kernel/SDK/compiler/runtime/lockfile edits.

Focused filters are `layer_c1_wave_` on the library (7),
`layer_c1_wave_canary_contract::tests` on the new binary (5), and
`argmax_canary_runtime::profile_tests::layer_` on each of four canary binaries (4
each). Old parser/profile/ordered-failure tests remain in ordinary coverage.
Explicit existing v11 artifact and two frozen serde workload goldens retain
their separate fixture prerequisites. Remote formatting and the complete combined
host gate still require root source/control review before execution.
