# Guarded MLP Compiler Handoff Capture

Status: **actual checked replay and 29 capture-controller fixtures pass on
MI350**. The earlier failed fixture attempt is retained separately. This is
compiler and diagnostic evidence, not GPU, numerical or performance acceptance.

The [checked gfx950 image](../guarded-mlp-atomic-load-alias-lowering-v1/README.md)
contains a [wave-wide guard branch](../guarded-mlp-atomic-load-alias-lowering-v1/IMAGE-REVIEW.md).
To investigate it, the diagnostic controller passively captures the exact
descriptor-bound compiler handoff during an unchanged checked compile.
LLVM-only extraction is not substituted for the checked compiler transaction.

## Capture Boundary

The controller reads only its owned compiler child's immediate scratch
directory. It uses read-only, no-follow file descriptors, private-directory
and inode checks, bounded scans, a 1 MiB body cap, and two identical bounded
reads. It does not alter the producer's scratch, compiler invocation, sealed
file descriptors, tools, source, resource limits or cleanup behavior.

Two matching reads are a consistency check, not a general proof of concurrent
immutability. Actual replay must independently join the captured handoff to
both the current compiler observation and the prior handoff identity, and
join the new HSACO and canonical descriptor to their prior identities.
Capture and compile success are reported separately. No runtime authority is
granted by the diagnostic.

## Actual Checked Replay

The [actual receipt](attempt-v1/evidence/complete.json) reports both compile
and capture success. The compiler exits naturally with status zero after
392.792619 seconds; the whole controller takes 397.944645 seconds. The child
is reaped, its process group is absent, and no timeout or forced cleanup
occurs. All 5,308 source rows and 308 consumed input pins remain unchanged;
postchecks are clean.

The [capture record](attempt-v1/evidence/handoff-capture.json) reports 3,909
polls and one completed double-read snapshot. All four identity joins pass:

| Join | Actual identity |
| --- | --- |
| Captured handoff to prior observation | 288,742 bytes; `5f52c141f577162cbc3eda8704b173df7035e8c336b8b436c23558f101fe844c` |
| Captured handoff to current observation | Same complete handoff identity |
| Current HSACO to prior HSACO | 28,440 bytes; `de880dcebf79425ccb555c1a2bdca3ff78b926f7b2062d06b918763da4de0f66` |
| Current canonical descriptor to prior descriptor | `8cce86c5641366ab51cd4eabb466f91683d791d2e36a9c258fdd2b549a08ff07` |

The entire observation is also byte-identical to the previous compile. The
retained [handoff](attempt-v1/evidence/compiler-handoff-v2) is descriptor-bound
LLVM input to the checked worker, not an independently admitted executable.
The [retention manifest](attempt-v1/retention-manifest.json) pins 16 bodies in
a 17-member capsule, including 12 raw files, totaling 4,776,335 expanded bytes.
The receipt is 26,129 bytes, SHA-256
`6795b6383e4434ad190af753b757a6f74edfeb3e0b3cf8ae0a8f8404887a03d6`.
The archive is 753,641 bytes, SHA-256
`3f6108d9c7633436e8e18589c8e776cf03c220f3daeb4946c46e06d48d3ac2ec`.

## Actual Controller Tests

| Attempt | Result | Process | Meaning |
| --- | --- | --- | --- |
| [v1](controller-tests-v1/attempt-v1/evidence/failed.json) | 28/29 pass | Natural exit 1; reaped, process group absent | Same-size mutation was not detected by the metadata check in this run. |
| [v2](controller-tests-v1/attempt-v2/evidence/complete.json) | 29/29 pass | Natural exit 0; reaped, process group absent | A second bounded content read detects the deterministic mutation fixture. |

V1 preserves its original raw failure. Its receipt has no successful child
observation; the raw stdout separately records the 29-test census. The logs
do not establish why the metadata check missed the rewrite.

V2 adds the second bounded read and strengthens the same named mutation test
to hold the same-inode metadata fixed while changing the body after the first
read. It retains all 29 test names: 26 capture cases and three AST checks of
the unchanged controller behavior. Both runs have clean source, input, tool
and scratch postchecks, without timeouts or forced cleanup. V2 takes
0.122312 seconds for the whole controller.

Coverage includes partial and disappearing files, same-size mutation,
symlinks, hardlinks, FIFO and directory rejection, inode and parent
substitution, wrong-child and duplicate-directory rejection, size and scan
bounds, descriptor cleanup, and independent identity-join failures. The AST
checks keep inherited helper functions, resource limits and process cleanup
unchanged apart from passive polling.

Each capsule contains 15 members with 14 pinned bodies and seven raw files.
The v1 receipt is 6,524 bytes, SHA-256
`2fa03979bbec047211440b4d70432b7d2574f68611a48bcc5d553905ec01c5a6`;
the v2 receipt is 10,973 bytes, SHA-256
`73a50e83a9188ed203e2c4b0b9f24db94547c2877f3b181ebf6ab982f9992ac1`.
The respective archives are 69,347 bytes /
`74b9f659a5847e4d912a0492f36851f103fd86b8a0fc71746c394f93628fbf60`
and 69,731 bytes /
`d800b856e855c4dfb5177080bd922c140f88fd96169e537793bcb219f4d42db9`.
The [v2 retention manifest](controller-tests-v1/attempt-v2/retention-manifest.json)
binds the reviewed controller and test source to that actual run.

Inspection of the captured LLVM is the next diagnostic gate. Private runtime
admission, paired GPU lifecycle, independent numerics, all issue #42 milestones
and the 700 tokens/s target remain open.
