# Guarded MLP Lowering Retry

The refreshed gfx950 compilation reached a new source-collection rejection
on MI350. It did not emit an HSACO or execute on the GPU. The earlier
[provider-source mismatch](../guarded-mlp-lowering-attempt-v1/README.md)
is no longer the observed failure, but this retry is retained as failed.

## Actual Result

The compiler exited naturally with status 1 after 117.259 seconds. Its process
was reaped and the process group was absent; there was no timeout or forced
cleanup. Whole-run time was 120.821 seconds. Candidate source/lock, vendor,
nightly Rust sources, consumed tools/providers and configuration postchecks
passed. The [failed receipt](attempt-v1/evidence/failed.json) records no artifact.

The [raw compiler diagnostic](attempt-v1/evidence/compile.stderr) is:

```text
unsupported rustc compiler intrinsic: atomic intrinsic without a concrete ordering argument
```

The reachable call chain enters `core::sync::atomic::Atomic::<u32>::load`
and then `core::sync::atomic::atomic_load`. Source collection could not
establish the ordering required by the checked atomic intrinsic rule.
This is the actual current compiler gap. The anticipated shared-state/guard
alias-analysis issue was not reached and must not be presented as this result.

## Qualified Inputs

The retry used the [27-test refreshed candidate](../guarded-mlp-dependency-refresh-v1/README.md),
its fresh locked offline vendor tree, and the
[fourteen-check loader-audited tools](../guarded-mlp-s-rpo-tool-audit-v1/README.md).
Extractor and backend are the final products of the
[full runtime compiler qualification](../guarded-mlp-s-rpo-qualification-v1/README.md).
The other five tools retain their earlier identities. No kernel arithmetic,
atomic guard protocol or compiler safety check was changed for this retry.

The [seven synthetic controller tests](controller-tests-v1/README.md) passed
separately. The executable controller differs from that tested draft only
in two reviewed actual-input bindings. Its single compiler invocation uses
the unchanged managed environment, gfx950 target, code-object version 6,
two explicit kernel roots, 600-second leaf/720-second whole limits and a
50-second cleanup reserve. No automatic retry or fallback occurred.

## Evidence And Next Step

The [retention manifest](attempt-v1/retention-manifest.json) pins twelve
original files: the controller, failed receipt and ten raw records, including
the full diagnostic, command/environment and input/source maps. Receipt
SHA-256: `f6c30e7435d46536e1983328361a933de0de86495cc78f65d9abf10827a11593`.
No LLVM handoff or executable artifact is claimed retained.

Next is a source-grounded atomic-wrapper ordering fix or diagnostic, with
controls matching the actual managed extraction path. Acquire/Release
semantics and rejection of unknown ordering must remain intact. Successful
lowering still requires independent ABI/ISA checks before GPU validation.
All issue #42 milestones, model numerics and the 700 tokens/s target remain open.
