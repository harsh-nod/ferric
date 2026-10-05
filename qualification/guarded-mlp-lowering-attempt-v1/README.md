# Guarded MLP: First Checked Lowering Attempt

This MI350 attempt on 2026-10-05 failed during device extraction. No HSACO was
emitted, no kernel was launched, and no correctness or performance result follows.

## Observed Failure

The managed compiler built the pinned dependencies and reached the guarded MLP
device crate. It rejected `fe2o3_device_trap_v1` because the candidate's complete
`fe2o3-device` source closure did not match the closure trusted by the transferred
extractor. The diagnostic names `src/diagnostics.rs`, but closure authentication
covers the package manifest, optional build script and recursively included source
files, not just the named function's file.

The [raw compiler diagnostic](retained/guarded-mlp-lowering-v228-v1/compile.stderr),
[command](retained/guarded-mlp-lowering-v228-v1/compile.command.json),
[failed receipt](retained/guarded-mlp-lowering-v228-v1/failed.json), source inventories
and exact controller are retained. This is a real compiler refusal after a
successful loader audit and offline dependency preparation, not a GPU failure.

| Check | Observation |
| --- | --- |
| Compiler process | Natural exit 1; reaped; process group absent |
| Compiler interval | 116.347 seconds, including dependency builds |
| Timeout or forced cleanup | Neither |
| Peak sampled private scratch | 954,756,239 bytes |
| Source, lock, tool, vendor and provider postchecks | Passed |
| Emitted image | None |

The controller retained its 600-second compiler limit, 12 GiB address-space
limit, two-core affinity and shared-host storage floors. No synthetic host-only
crate binding was passed to extraction. The trust check was not bypassed and
the controller did not retry.

## Next Step

Use a coherent compiler and device-source generation, with a published dependency
revision and fresh CPU qualification, vendoring and checked lowering. Do not
change the trusted digest merely to accept a mismatched package or remove the
trap to evade provider authentication. Actual ABI inspection, GPU guard controls,
reusable arena and full-worker validation remain subsequent gates.

All issue #42 M0-M7, independent numerical acceptance, sustained 2,048/256 and
700 tokens/s gates remain open.
