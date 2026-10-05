# Matched Provider Baseline

The 2026-10-05 MI350 baseline reproduces the same matrix-extraction refusal on
the previous fe2o3 revision. This specific failure **predates provider alignment**;
neither run passes the pipeline control or qualifies the new provider generation.

## Comparison

| Source revision | Exact matrix control | Source-safety result |
| --- | --- | --- |
| Published `097b4f796a283f554339cc0c0ef5c2c8c3858d2a` | Failed | Cannot authenticate core `Result::branch` |
| Proposed `a057bf3b0e9175f481a31181b19d587d342bf8f3` | Failed | Same helper and check |

The test is `dynamic_matrix_kernel_reaches_gfx942_llvm`, invoked explicitly
with `--ignored --exact --test-threads=1`. Both failures concern the genuine
`Result<Bf16MfmaMatrix<MfmaOperandA>, Bf16MatrixViewError>` error-propagation
helper reached by `?`, before pipeline construction. This is an authentication
gap, not evidence that the helper contains unsafe code.

The [structured comparison](comparison.json) joins both receipts and confirms:

- Equal pinned compiler tools and matching command arguments, working
  directories and environments after replacing only the private run root.
- The same device, compiler-product and compiler-test build selections.
- Byte-identical collector, matrix test driver and matrix/attention examples.
- Exactly 24 added device source files and three changed source files in the
  aligned generation, with no other source changes.

The baseline uses a fresh private target directory, locked offline dependencies,
two CPU cores, two Cargo jobs, `MALLOC_ARENA_MAX=2`, the same 12 GiB address-space
limit, stream/deadline bounds and shared-host free-space floors. It builds and
inventories the same targets but executes only the matrix test. Listed device,
provider and atomic tests are not claimed executed in this baseline.

## Actual Outcome

The [failed receipt](evidence/failed.json) records 18 natural, reaped phases
with no remaining process groups: the first 17 returned zero; the matrix test
returned 101. Source, dependency, configuration, tool and final-product
postchecks passed. The [raw diagnostic](evidence/matrix-extraction-0.stdout)
is retained unchanged. The retention manifest pins 97 evidence/input files.

This failed control is not converted into an expected-failure pass. The aligned
generation's [398 preceding passing tests](../guarded-mlp-provider-generation-v1/README.md)
remain partial evidence; attention extraction is still unexecuted. Ferric's
dependency is unchanged, and the proposed revision remains unpublished.

Next, retain bounded MIR/signature/source-scope observations of the rejected
helper without changing admission, implement a narrowly scoped authenticator
with negative controls, and rerun both unchanged pipeline tests and provider
qualification. No HSACO, GPU execution, model correctness or performance result
is claimed here. All issue #42 M0-M7 and the 700 tokens/s target remain open.
