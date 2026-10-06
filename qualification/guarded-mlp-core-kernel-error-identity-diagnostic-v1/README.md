# KernelError Identity Conversion Diagnostic

This is a bounded compiler observation after the
[438-test checked-wrapper checkpoint](../guarded-mlp-core-checked-attention-qualification-v1/README.md),
not a production recognition rule or successful attention compilation.
The diagnostic uses base revision
`3016e979b0d52663067f7a878d47ca775a58df36` plus a pinned three-file overlay.
Ferric's production dependency and execution path are unchanged.

## Actual MI350 Run

Both compiler builds passed in 40.496 and 47.077 seconds. All
[six renderer tests passed](attempt-v1/evidence/core-kernel-error-identity-diagnostic-tests.stdout).
These tests check output limits and completion reporting; they do not add to
the earlier 438 production controls, which this diagnostic did not rerun.

The unchanged attention control returned 101 at the same independent
`From<KernelError>` source-origin check after 15.021 seconds. The
[actual failed receipt](attempt-v1/evidence/failed.json) records 19 natural,
reaped exits with no remaining process groups: the first 18 returned zero,
then attention failed. Sources and dependencies were unchanged, and integrity
postchecks were clean. No timeout or forced cleanup occurred.

## Observed Body

The [raw output](attempt-v1/evidence/attention-extraction-0.stdout) contains one
complete capture of genuine core `<T as From<T>>::from`, instantiated with
the authenticated provider's `KernelError`:

| Component | Observed Representation |
| --- | --- |
| Signature | Safe Rust function, `KernelError -> KernelError` |
| Locals | Argument and return local, both normalized to `KernelError` |
| Scope | One root scope, cleared cross-crate data, no parent or inline origin |
| Control flow | One non-cleanup block |
| Statement | `_0 = move _1` |
| Terminator | Return |

The [derived marker](attempt-v1/derived-core-kernel-error-identity-diagnostic.json)
joins byte-for-byte to raw output at offset 5,381, extent 1,724 bytes,
including a 1,635-byte body. Its SHA-256 is
`16c283e8c917dc6f88d0208c493ad5db3b770352fda65c765ec3c671deac295b`.
This is evidence for a future whole-body check, not permission to accept
arbitrary identity conversions or printed type names.

## Scope And Limits

The hook only observes the instance already selected for rejection. It checks
genuine core `From`/`from_fn` identity, the concrete safe signature and the
existing provider classification. It changes no accepted predicate or error
path. Output is opt-in only for the attention control and limited to one
64 KiB record per compiler process, with 128 locals/blocks/scopes and 1,024
statements as upper bounds. Partial or truncated output cannot become a
complete record.

The [retention manifest](attempt-v1/retention-manifest.json) pins 105 original
files and one derived observation, including the isolated source overlay,
controller, input manifest and 99 raw evidence records. Builds use two CPU
cores/jobs, offline locked dependencies, hidden GPUs and bounded scratch.

Next: implement and test an exact identity-conversion check, then rerun the
full compiler controls including attention. The current control targets
gfx942 LLVM extraction on the MI350 host; it does not produce qualified
gfx950 HSACO or execute a GPU kernel. Independent model numerics and the
sustained BF16 2,048/256 performance target remain open.
