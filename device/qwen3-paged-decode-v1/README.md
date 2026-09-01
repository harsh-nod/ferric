# Ferric Qwen3 Paged Decode Device V1

This non-authoritative compatibility package exposes Ferric's exact Qwen3
paged-GQA causal-decode kernel from canonical source owned by
`qwen3-all-kernels-v1`. It is retained for focused tests and is not a
production selected-package or publication root. It retains the production symbol and
six-slice ABI from `ferric-qwen-kernels`, the closed fourteen-profile
target/draft B3 catalog, Wave64 launch geometry, committed-token causal bounds,
global P16 page-table mapping, quotient GQA, sequential D128 score reduction,
online stable-softmax recurrence, and adjacent two-BF16 output elements per
workitem.

Every immutable Q, K, V, page-table, and committed-count access uses fe2o3's
bounded `memory::volatile_load`. Output authority is the compiler-issued
`Blocked<Index1D, 1, 2>` view: invocation `g` owns exactly `2*g` and `2*g+1`.
The source traps on an unrecognized profile, a committed-plus-active context
above 8,192, a physical page outside the global 16,384-page pool, a non-finite
input or intermediate, or a failed owned store. A trap is per workitem; it does
not roll back stores completed by other workitems.

The page table maps logical token `k` through `[sequence, k/16]` directly to a
global physical page. There is no sequence-local cache base or cache stride.
Query head `h` maps to KV head `h/4` for Qwen3-8B and `h/2` for Qwen3-0.6B.
K/V for every active token must already have been initialized by K3 before this
kernel is dispatched.

This package is outside Ferric's host workspace and carries no artifact,
publication, load, dispatch, numerical-qualification, whole-Qwen, or M1
authority. The package pins exact reviewed fe2o3 revision
`9f97985ee0a4a8ef0bc8f0fa0fd33771c8180592`; that source pin grants none of
those authorities. Exact attributed-source extraction, generated ABI
reconciliation, artifact admission, KFD execution, numerical differential
qualification, and MI300X performance evidence remain required.
