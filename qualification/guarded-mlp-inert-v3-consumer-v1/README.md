# Lossless Inert V3 Completion Content

This CPU checkpoint advances Ferric [issue #42](https://github.com/harsh-nod/ferric/issues/42)
after the [default-off extraction selector](../guarded-mlp-inert-v3-selector-v1/README.md).
It implements a separate inert-only completion **content consumer**, not
protected publication intake. The selector remains explicitly disabled.
No worker compilation, HSACO, GPU, numerical/performance evidence or M0-M7
completion is claimed.

## Implementation

The new [completion module](source/crates/cargo-fe2o3/src/protected_compiler_handoff_v3/inert_semantic_completion_v1.rs)
retains the original `Eligibility` and its live parent borrow together with
the complete `InertSemanticCompilerModuleHandoffV3` owner. It:

- Requires a reported successful child `ExitStatus`.
- Revalidates retained parent custody and the exact prepared invocation
  transport before and after the content join.
- Requires an independently expected **complete outer identity**, not merely
  a self-consistent replacement capsule.
- Joins the full invocation descriptor, including all six closure roles,
  every argv token, cwd and complete environment, and requires gfx950.
- Retains the complete formal-memory and proof-binding receipt preimages
  and every other capsule segment, without extracting a bare V2 handoff.
- Allows serialization of only the complete canonical outer bytes; it has no
  conversion to protected transaction intake, worker or launch authority.

The [eligibility revalidation helper](source/crates/cargo-fe2o3/src/protected_compiler_handoff_v3/inert_semantic_extraction_v1.rs)
rejoins the original sealed parent image to the selected transport. The
[existing strict FFI codec](source/crates/fe2o3-compiler-ffi/src/inert_semantic_module_handoff_v3.rs)
already checks outer/inner byte bounds, exact canonical pair identities,
target agreement and final-module commitment. Those checks are reused, not
reimplemented with a competing format or parser. Its owned decoder keeps
the outer, nested module, capsule and receipt views in the same allocation.

**Receipt preimages remain opaque inert content.** Retention is not replay
of a real conditional formal archive, proof of compiler authorship, discharge
of runtime premises, or registration of a numerical reference. Supplying an
`ExitStatus` does not establish owned-process wait/reaping or file custody.
The future caller must independently own those facts and the expected outer
identity. No successful live-parent `Completion` was executed in this campaign.

## Actual CPU Qualification

SSH mi350 CPU8/9, nice10, offline/jobs2, 12 GiB service memory maximum,
swap disabled, owned subreaper/process groups:

| Cohort | Actual Result |
| --- | --- |
| Invocation library | 103 passed |
| Preserved engineering frontend | 25 passed |
| Capture/closure upgrade | 8 passed |
| Default-off selector | 10 passed |
| Binding-wrapper lifecycle | 6 passed |
| New inert completion content | 9 passed |
| Prior qualified compiler gate tests | 2 passed |
| Total | 163 passed |
| Owned phases | 15 natural status0 exits, reaped leaders, groups absent |
| Elapsed | 76.988416 seconds |
| Source inputs | 5,814 unchanged |
| Source/tool/dependency/artifact postchecks | Passed |

See [terminal receipt](cpu/complete.json), [consumer outcomes](cpu/consumer-tests.stdout),
[selector outcomes](cpu/selector-tests.stdout), and [data audit](publication-audit.json).
The five driver ignores remain unchanged/unrun. The two compiler gate tests
reuse the prior qualified compiler ELF; the full compiler/device/lowerer
suite is not rebuilt or rerun. The previous selector harness failure and
retained-binary recovery remain separate, preserved evidence; their exact
receipt/source joins are checked before choosing this candidate sourcebase.

The nine new tests cover exact outer allocation/byte retention, opaque formal
and proof preimages, V2/truncated/trailing-data refusal, unsuccessful/signalled
status, all six foreign closure roles and cwd mismatch, a self-consistent
wrong target, recomputed capsule substitutions against the expected outer
identity, inherited final-commitment mismatch refusal, raw capsule tampering,
and source-level API/parent-retention/publication-refusal controls.

All successful content fixtures are explicitly labeled **schema-only**.
Their opaque markers are not actual conditional premises or proof archives.
The final-commitment negative control is rejected by the existing outer
constructor before a malformed outer owner can exist; the consumer uses
that same strict codec. The no-authority/publication controls include
source-level assertions, not execution of a protected publication attempt.

## Remaining Work

The full extraction-only route is still unqualified. The [existing protected
intake](source/crates/cargo-fe2o3/src/protected_compiler_handoff_v3.rs) remains
separate and unchanged apart from module registration. The default-off
request still returns `ExtractionOnlyConsumerUnqualified`; no fallback into
protected publication or ordinary V2 intake is introduced.

Next bounded implementation: a pinned output-file intake for the inert-only
completion path. Retain the original private directory/file objects, enforce
bounded ordinary-file reads, bind the exact returned outer bytes/identity,
and test symlink/path substitution, truncation, oversize and changed-file
refusals. Keep child wait/reaping custody explicit and do not advertise that
file-content checks authenticate an executing compiler. Then integrate a
separate owned child lifecycle; only qualify the complete route when actual
inputs and outer lineage survive end to end.

No fresh worker compilation until that complete matching route is qualified.
The eight actual runtime requirements, early-stop CFG issue and unresolved
GPU completion hold stay closed. No signed-Verus, compiler-refinement,
numerical, artifact, publication, load or launch authority is granted, and
the 700 tokens/s target remains unmet.

## Provenance

[Runner](run_cpu.py), [input preparer](prepare_inputs.py), [exporter](export.py),
[input manifest](input-manifest.json), [originals ledger](originals-ledger.json),
and [archive](originals.tar.gz) retain the actual tested sources and raw phases.
The data audit rehashes candidate sources, tools, parents and selected artifacts
before export. The archive embeds relevant source/log/controller originals,
not complete compiled/toolchain/source trees; full input rosters and artifact
hashes remain in the receipt.
