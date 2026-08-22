# Ferric finite kernel catalog foundation

This crate maps every operation in the 22 exact target/draft B3 sequential
plans to one of seven finite kernel families. Each family has one Ferric-owned
source declaration under `crates/ferric-qwen-kernels/src/`. The record also
binds the future Ferric source closure, reusable fe2o3 compiler/runtime
dependency closure, compiler, target, kernel proof/ABI, runtime contract/ABI,
and TCB identities required by the offline build.

The K1-K7 paths are ownership declarations, not claims that each file exists,
has been reviewed, or implements its family. The verified operation handoff
does not consume this roster as authority. Some graph operations require an
explicit `RequiredExtension`. No binding is proof evidence or an executable
candidate. This crate contains no kernel source, compiler integration, object,
HSACO, device allocation, queue, load, dispatch, launch, completion, hardware
observation, performance result, or qualification receipt. It does not close
any M1 roadmap or property obligation.
