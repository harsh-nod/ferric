# Captured R2 Boundary Replay V2

This is a narrow source-only correction to the retained failed V1 synthetic
gate. No V2 test or replay has been executed by its author. All arithmetic,
capture admission, original operand pins, limits, twenty named tests and
success/failure rules remain unchanged. Only one malformed-test mutation and
its diagnostic guard change; the runner has a fresh output namespace and test
source hash.

## Preserved Failure

V1 ran twenty synthetic tests on MI350: nineteen passed and one failed. The
original failed receipt is 4478 bytes /
`acbc87f6912e47f93f28490d7c79c72bdb4229ad0c9c94377aec4955ce0912a7`;
its raw log is 3360 bytes /
`602e8b1d36b222e49b6aadd4eff2b9d716ed8fb625696734fcb997848db7b3c0`.
Source posthashes were clean. No actual captured-data admission or R2 replay
ran. Those two bodies and the V1 sources remain immutable under the separate
failed attempt.

The failing negative fixture changed part 29's scalar to `bf16`, but part 29
is rank-1 Up and was already `bf16`. It therefore expected rejection of an
unchanged valid input. V2 uses invalid `f32`, labels each mutation with a
subtest, and asserts the serialized input actually changes before checking
rejection. Serialization is intentional: Python dictionary equality alone
would consider the separate Boolean-rank mutation `True == 1` unchanged.

## Unchanged Scope

The [V1 source description](../model-current-r2-replay-v1/README.md) explains
the exact native source contract and fourteen-data-body custody. V2 retains
the same seven Python files, input manifest and two Rust contract bodies.
The unchanged integer replay performs ordered FP32 +0/rank0/rank1 addition,
BF16 materialization, then each rank's actual BF16 residual addition and final
BF16 narrowing. No rank partial is individually narrowed.

Success requires all twenty tests, clean posthashes, all 8192 native final
encodings matching and all 4096 separate genuine-framework residual-control
encodings matching. Finite mismatches preserve all row results and derived
outputs before a failed terminal. No failed result is promoted or overwritten.
Derived materialized Down bytes are not claimed to have been captured.

Root stages the ten runtime inputs with `contract/` preserved and invokes
`python3 -I -B run.py`. Output is the fresh directory
`E/guarded-mlp-model-current-r2-replay-v228-v2`. Optional `--self-test` uses
the separate `...-v228-v2-tests` directory. The unchanged bound is 120 seconds,
256 MiB address space, 40 inputs/32 MiB total, and at most 8 MiB output.
No compiler, native worker, child, framework or GPU execution occurs.

An exact result is a captured-operand source-boundary diagnostic, not proof
of machine instruction order, Down dot products, numerical acceptance,
full-model acceptance, performance, or production authority. No thresholds
or unchanged-receipt revalidation are introduced. All future V2 outcome pins
remain unknown until root executes the new attempt.
