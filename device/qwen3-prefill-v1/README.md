# Ferric Qwen3 Prefill Device V1

This non-authoritative compatibility package exposes Ferric's exact Qwen3
paged-GQA causal-prefill kernel from canonical source owned by
`qwen3-all-kernels-v1`. It is retained for focused tests and is not a
production selected-package or publication root. It retains the production symbol and
five-slice ABI from `ferric-qwen-kernels`, the closed target/draft B3 profile
set, Wave64 launch geometry, P16 page-table mapping, sequential D128 score
reduction, online stable-softmax recurrence, and adjacent two-BF16 output
elements per workitem.

Every immutable Q, K, V, and page-table access uses fe2o3's bounded
`memory::volatile_load`. Output authority is the compiler-issued
`Blocked<Index1D, 1, 2>` view: invocation `g` owns exactly `2*g` and `2*g+1`.
The source traps on an unrecognized profile, invalid page, non-finite input or
intermediate, or failed owned store. A trap is per workitem; it does not roll
back stores completed by other workitems.

This package is outside Ferric's host workspace and carries no artifact,
publication, load, dispatch, numerical-qualification, whole-Qwen, or M1
authority. The package pins exact reviewed fe2o3 revision
`9f97985ee0a4a8ef0bc8f0fa0fd33771c8180592`; that source pin grants none of
those authorities, and the package still must be compiled into current
artifacts and qualified on MI300X.
