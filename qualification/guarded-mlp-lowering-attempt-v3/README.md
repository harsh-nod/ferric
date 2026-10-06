# Guarded MLP Optimized-Inlining Lowering Attempt

On 2026-10-06 UTC, a fresh guarded gfx950 lowering attempt on MI350 passed the
previous atomic-ordering collection failure and reached a different rejection:

```text
production compilation general kernel verification failed:
semantic-to-ranked projection rejected semantic CFG exceeds the ranked block limit before loop analysis
```

The compiler leaf exited naturally with status 1 after 117.370709306 seconds
and was reaped with an absent process group. Whole-run time was 120.971775939
seconds. No timeout or forced cleanup occurred. Source and consumed-input
postchecks passed without errors. **No HSACO was emitted or GPU kernel run.**

## What Changed

The [qualified refreshed driver](../guarded-mlp-driver-cpu-v1/README.md) and
[fresh fourteen-command loader audit](../guarded-mlp-s-rpo-tool-audit-v2/README.md)
replace only `cargo-fe2o3`; the other six tools, candidate Rust bodies and
offline vendor tree remain unchanged. The command explicitly selects
`--mir-normalization optimized-inline-v1`. The raw compiler command confirms
`-Zinline-mir=yes -Copt-level=3` with the fixed gfx950 Wave64 features and
JumpThreading disabled. The LLVM worker policy remains O2 with verification.

Atomic-ordering, unsafe-source and source-origin checks were not relaxed.
The previous atomic-load diagnostic is absent from this run. The new failure
is in semantic-to-ranked projection before loop analysis; it is not evidence
that the possible shared-state/guard alias issue has passed or failed.

## Interpretation And Next Work

The source guard checks an empty or over-limit semantic CFG before loop
analysis. Its current maximum is 1,024 blocks. This diagnostic does **not**
identify the rejected function or report its actual block count, so neither
is asserted here. The candidate's 548 literal atomic state checks are a
plausible source of inlined CFG growth, not a measured attribution.

Next is a bounded diagnostic identifying the function and block count, followed
by evaluation of a smaller equivalent kernel representation. Any change must
preserve all 548 state checks, atomic ordering, generation ownership and the
consumer guard protocol. Merely raising a verifier limit is not part of this
checkpoint. Native valid/invalid guard behavior, independent model numerics,
sustained 2,048/256 decode and 700 tokens/s remain unqualified.

## Evidence

[attempt-v1](attempt-v1) retains the exact executed controller, failed receipt,
ten raw records and retention manifest. Receipt: 6,210 bytes, SHA-256
`ae4a8a3d60f3c65a53cb8e56f1409b0e1ee8287740bd2bcb7102d59cf08a5f83`.
The 667,614-byte archive SHA-256 is
`95af51bb1f2c490e04c781e19599d46730e47b2c31d4f0bf0e49842f7f32f471`;
all 13 members and 4,181,644 body bytes were verified after transfer.

[controller-tests-v1/attempt-v1](controller-tests-v1/attempt-v1) retains a
separate MI350 run of eleven admission and four normalization fixtures.
All fifteen passed in one natural zero/reaped leaf, with unchanged inputs and
clean postchecks. Whole-run time was 0.820038 seconds. Receipt SHA-256:
`9ac4101234e79340353fb85648f671d80fe1255a7cb5c7f132e7e41d64f59498`.
These fixtures are synthetic; their small ELF-magic payload is not an HSACO.

The tested controller SHA-256 was
`904fbb6ed4331e68eede9a40c7fac3f5b9ecc97a6066188fe390774a20f36ba4`.
Only its pending loader-receipt constant was subsequently bound to the actual
passed audit `d79c1b84...`; the executed controller SHA-256 is
`f51fbf0c68bd76f89fca4ab145b5060dea1ff67f7a148062f9fecef7f5e9d308`.
Reversing that one binding reproduces the tested source exactly. The fifteen
fixtures are not claimed to have executed against the later bound hash.

All issue #42 milestones remain open. This checkpoint changes no production
execution path and claims no end-to-end model or performance result.
