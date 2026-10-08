# Opt-In MIR Inlining Threshold

This is a compiler experiment for Ferric [issue #42](https://github.com/harsh-nod/ferric/issues/42),
authored, built and tested on `ssh mi350`. It is not a native GPU
qualification, production admission, independent numerical acceptance or
performance result. The implementation is retained here as a frontend patch;
the fe2o3 production branches and Ferric runtime path are not changed.

## Motivation

The [preceding early-STOP experiment](../guarded-mlp-early-stop-v1/README.md)
retains eight unsuccessful compiler invocations. Its latest fixed/early pair
was rejected because collection reached a possible panic through
`core::sync::atomic::atomic_compare_exchange`.

The historical successful MLP recipe selected
`-Zinline-mir-hint-threshold=16384`. The current engineering frontend did not
offer that flag. This experiment tests the hypothesis that stronger inlining
could expose constant atomic orderings and eliminate unreachable panic arms.
The atomic orderings, admission checks and compiler backend are unchanged.
An historical flag is a hypothesis to test, not proof of compatibility with
the retained current compiler.

## Implementation

`--mir-normalization optimized-inline-hint-16384-v1` is a new explicit
engineering selector. It adds exactly one flag to `optimized-inline-v1`:

```text
-Zinline-mir-hint-threshold=16384
```

The omitted-selector default remains `minimal-v1`. Both pre-existing profiles
retain their exact flag strings for gfx942 and gfx950. No free-form Rust flags,
environment overrides, alternate atomic semantics or admission exceptions
were added.

The three-file [patch](frontend.patch) includes the enum/parser/help, fixed
flag generation and tests. A small manifest field constructor is shared by
the real manifest writer and its serialization test. It still records worker
optimization `O2`, `strip_debug=true`, `verify_each=true` and the original
limits. Worker `O2` is distinct from the frontend's `-Copt-level=3`.

## CPU Qualification

The fresh frontend passes **21 focused tests**: 18 byte-unchanged existing
test bodies and three additions covering exact/default flags on both targets,
strict selector and override rejection, and manifest fixed-options agreement.
The binary inventory contains 405 tests; 384 were filtered, including the five
unchanged ignored tests. This is not a full binary or repository test run.

All seven phases returned zero naturally, were reaped and left their owned
process groups absent. The controller completed in 97.867334807 seconds.
Source, dependency, configuration, tool and artifact postchecks passed.
All 5,297 project files match the retained frontend baseline except the three
explicit files; controller and supervisor bring the input roster to 5,299.

- Receipt: `237f3742f47464641584c65bf63526adf18ab0983d90ea2ee8526e8479c4c212`.
- Frontend: 81,290,520 bytes, `0a04eafefdee18e40c31069abc54505d845598f53dffebb69d988a10a0dc301b`.
- Baseline receipt: `a869f0f4aa0572c82492cb3d90bdb0dc9bb2625a7b441ff1f3e9a876a87079d9`.

The [original test output](cpu/driver-tests.stdout) and
[CPU receipt](cpu/complete.json) distinguish the tested artifact from the final
frontend executable. The [archive](qualification.tar.gz) retains commands,
raw streams, inventories, dependency/source ledgers, exact source preimages
and postimages, and the paired compile records. The [manifest](manifest.json)
pins the packet; archive member pins point back to their original remote files.

## Paired Compile

Both versions use the new frontend and explicit 16384 profile, with the same
retained backend, extractor, Rust toolchain, link worker and vendor tree as
the preceding attempt. V6 keeps the previous named callback
`#[inline(always)]`; V7 changes only that callback to `#[inline(never)]`.
Within each pair, the sole fixture difference is `run_fixed_rounds` versus
`run`. The original numerical includes and arithmetic remain unchanged.

| Attempt | Arm | Callback | Compiler Leaf Wall Time | Actual Result |
| --- | --- | --- | ---: | --- |
| V6 | Fixed | Always-inline | 116.872218007 s | Exit 1: inlined FnMut source origin |
| V6 | Early STOP | Always-inline | 117.413967654 s | Exit 1: same source-origin refusal |
| V7 | Fixed | No-inline | 118.720522923 s | Exit 1: mutable aggregate helper parameter |
| V7 | Early STOP | No-inline | 118.602566160 s | Exit 1: same helper-parameter refusal |

These are **four failed compile attempts**, not passing kernel tests or GPU
timings. All exited naturally, were reaped and left their owned process groups
absent, with no timeout, forced cleanup, storage failure or postcheck error.
No HSACO was produced and no candidate was promoted.

V6 rejects an inlined `core::ops::FnMut::call_mut` source origin. The retained
collector requires its named target already in the executable roster. An
always-inlined target may disappear; this is a source-consistent explanation,
not a retained-MIR proof. V7 matches the compiler's existing positive fixture
shape by retaining the named callback body. It reaches pre-ranked
semantic-to-Kernel-IR materialization and rejects function 0's parameter:
`UniqueBorrow`, mutable reference to an ordinary aggregate, source argument 0,
declaration `Rust source 94c691d17656:828:5`.

Original stderr is retained for
[V6 fixed](pair/v6/fixed-compile.stderr),
[V6 early](pair/v6/early-compile.stderr),
[V7 fixed](pair/v7/fixed-compile.stderr) and
[V7 early](pair/v7/early-compile.stderr).
Changing the first diagnostic does not establish end-to-end compatibility or
prove that every earlier failure is resolved. Next work is to identify the
exact helper contract and qualify its lowering without weakening borrow or
source-safety checks. Any successful artifact would still require ABI,
numerical and native qualification before timing.

## Resource And Claim Boundaries

Frontend builds explicitly use two Cargo jobs. CPU affinity is 8/9 and nice
level is 10; GPU visibility is disabled. The paired extraction command clears
the caller environment before starting inner Cargo, so its outer
`CARGO_BUILD_JOBS=2` is **not an explicit inner-job limit**. Previous records
must not be read as proving that limit. Both arms retain identical affinity,
process ownership, address-space/storage limits and 600-second leaf bounds
with a 50-second cleanup reserve. No deadline or free-space floor was raised.

No native GPU run is authorized by these receipts. Existing native model
evidence remains forty prompt forwards and zero generated tokens, with parity
against historical Ferric rather than an independent framework. Full2303,
independent 256-output acceptance, sustained 2,048/256 BF16 target-only decode,
700 tokens/s and all M0-M7 exits remain open.
