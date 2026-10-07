# Uniform Endpoints With Checked Loads

Source-only, unformatted and untested. No emission or native launch is
authorized by this proposal. It is separate from both the emitted original
prefetch4 image and the grouped direct-slice successor that failed convergence.
The unchanged-body V5 component control and matched split-K emission remain
ahead of this experiment in the serial CPU queue.

## Narrow Delta

The complete before crate is the qualified original prefetch4 SDK84 source,
with resolved lock `1e25ff7b362da386e17e14efcfc2fb2f7b6ee4274920bcae7418f7520a63b7c1`.
Only the partial root changes:

- A pre-loop check explicitly states `row < rows`, `column < 4096`, and the
  two physical row endpoints. Every operand is subgroup-uniform: geometry and
  lengths are uniform ABI inputs, row/column derive from block identity.
- One `inner_3 < k` predicate surrounds all eight existing safe `load_or`
  operations and the original four ordered product/add/finite updates.
- All admitted K values are multiples of 256, so the four lane-stride offsets
  are jointly active. No inactive group reads or computes a product.
- No direct indexing, pointer access, new SDK operation, dummy-index access,
  bounds-check removal, trap inside the loop, or early loop exit is introduced.
  The original views, entry/length/launch guards, narrowing, 48-step loop,
  reduction and output-store code remain. The BF16 root is unchanged.

The added endpoint checks are redundant for admitted input geometry. Given
`element < rows*4096`, `row=element/4096`, and `column=element%4096`, the A row
end is at most `rows*k`, and the weight row end at most `4096*k`. Existing
constraints bound the latter by 50,331,648 elements, so endpoint arithmetic
does not overflow the selected 64-bit ABI. Invalid original inputs still trap
in the unchanged original guards before these checks.

Production source SHA256:
`4961736a0bd51fc1d5c07c5fe7a96324d602dc58eaccb28228202aef38ba8ff7`.

## Concrete Optimization Hypothesis

This assessment uses immutable Git object
`84ab85a424c821c9c2eca155baa5e33ea064349a`, not the unrelated dirty core checkout.
The relevant production files match published core
`510ec3b62ce245225235e1195a7982ff06fcc713`.

`production_semantic_kir_v1.rs:19264` builds six predicates per checked read:
logical row, logical column, row multiplication overflow, offset addition
overflow, column addition overflow, and physical storage index. The original
view offset is zero. The new row guards repeat the exact logical row tests;
the physical endpoint facts and common maximum-column predicate expose the
needed inequalities directly in dominating source control.

`fe2o3-amdgcn-model/src/lowering.rs:6897` translates each GuardedLoad into a
branch/load/phi, with no failure exit. The common active branch rejoins within
the same loop; it does not require the varying-exit exception that the failed
direct-index experiment could not prove. This makes convergence plausible,
not established until actual compilation.

The worker uses its target-aware default O2 pipeline. Simplifying the smaller
column bounds and physical-index comparisons still requires relational
inequality and no-overflow propagation. No retained evidence proves that LLVM
will remove these branches or keep several raw loads outstanding. The first
emission decision is therefore actual guard/VMEM-wait inspection, not a speed
claim. Refusal or unchanged pairwise waits must be retained as a failed
mechanism. Do not alter the verifier to force admission.

## Authored Tests

Six new `endpoint_loador` fixtures retain the complete predecessor source and
derive the exact expected AST, copying all four arithmetic blocks from it.
They reject endpoint, read, fallback, ordering, math, finite-policy, loop and
output mutations; enumerate all admitted K/lane/group predicates; validate
row/column endpoint and narrowing bounds; compare every scalar accumulator and
finite flag across ordinary/exceptional BF16 patterns; and poison inactive
tails. The three original contract fixtures are adjusted only for the one
pre-loop check and hand partial-loop coverage to the complete AST fixture.
Five original host fixtures are unchanged. All 14 are authored, not run.

Host scalar checks do not emulate native reduction or prove LLVM FP behavior.
Remote format/source custody, 14 tests, strict Clippy, exact compiler/SDK/provider
emission, ISA scheduling/FP-mode/resource inspection and native guarded parity
are separate future gates. Component timing must use the same current control
and include both O (K4096) and down (K12288); a component result is not full-model
TTFT/TPOT. No source is integrated or promoted by this handoff.
