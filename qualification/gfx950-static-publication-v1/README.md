# Static Publication Engineering Qualification

Status: Rust-to-gfx950 native build and the fixed 44-attempt engineering GPU
suite passed on Asrock (`mi350-2`). This is an observed one-shot publication
experiment, not protected admission or general native-memory eligibility.
The device and host dependencies pin compiler revision
`cb8f51ec0b52894a1fb510bc3f523005dda8bfaf`; `build.sh` requires its clean snapshot.
See the [Asrock results](#asrock-results), [complete sanitized suite](evidence-asrock-v1.json),
[native artifact metadata](artifact-asrock-v1.json), and
[bring-up notes](../../docs/evidence/gfx950-megakernel-42/asrock.md).

## Fixed Contract

Grid256/WG128/wave64. WG0 writes one ordinary f32 per cell then System-releases
READY=2. WG1 System-releases REQUEST=1, System-acquires once, and reads the
ordinary cell only when the acquired value is READY and the actual bound holds.
There is no polling, progress guarantee, cross-dispatch reuse, or host reset
during execution. Consumer-clear excludes initially READY values; zero
initialization is not a proof premise. Initialized u32 storage, exclusive
custody, valid lifetime, and the native atomic/memory contract remain required.

Five distinct guarded allocations contain payload f32[128], flags u32[128],
immutable-use input f32[128], statuses u32[256], and values f32[256]. Physical
kernarg is five pointer/length pairs (80 bytes), plus observed implicit bytes.
Flags retain the shared-atomic ABI, never ordinary readonly/noalias packing.
Input is an original exclusive/RW argument consumed into the V30 readonly view.
All buffers have 64-byte guards on both sides and four-byte element alignment.

## Frozen Matrix

`reference.py prepare` defines 44 attempts before any GPU observation:
40 valid attempts (eight each with initial flags all0, allREQUEST, allREADY,
all0xffffffff, and a mixed pattern including READY/nonprotocol values), then
four no-effect controls (payload127, flags127, payload0, flags0). Each uses a
new disposable worker and five fresh allocations. No failed or inconclusive
attempt is replaced; timeouts, malformed results, guard failures, and cleanup
failures terminate the suite. Valid input words are finite, nonzero, distinct
within each cell set, and distinct between attempts; comparisons are exact bits.

Every valid attempt requires 128 Published/+0 producers, all128 final payload
words equal to independent inputs, consumers only Ready or NotReady, Ready
values equal to the corresponding current producer, and NotReady values +0.
Final flags are REQUEST or READY; a Ready consumer requires finalREADY. A
NotReady consumer may have either final flag. Invalid shapes require 256
Invalid/+0 results and byte-identical payload/flags, including unused storage.

Suite success additionally requires at least one Ready cell in each of the two
consumer wave64 ranges **for every initial-flag pattern**, aggregated over the
eight predeclared attempts, and at least one NotReady in each consumer wave
across the same40 valid attempts. Every Ready bitmap and every NotReady result
is retained. Missing coverage, including allNotReady or allReady, is
`inconclusive` (exit3),
not a pass. No execution-order assumption is used to predict Ready counts.
The GPU cannot establish the absence of an unused ordinary read merely from a
zero result; no-read semantics remain source/KIR replay and simulator checks.

## Execution Boundary

`FE2O3_COMPILER_BIN=FROZEN_COMPILER bash build.sh WORK_ROOT NEW_OUTPUT_DIR`
requires a clean-source snapshot with matching commit, binary, loader and
provenance manifests. Parsed Cargo metadata must pin both device and host
dependencies to that same revision. The script retains checked handoff,
unchanged LLVM, providers/tools, native object and hashes, then rechecks
the compiler snapshot and loader files after the build.
`verify-suite.sh PROBE WORKER HSACO SOURCE PRIVATE_DEVICE_ID NEW_EVIDENCE_DIR`
freezes input/reference/artifact/executable hashes before dispatch. Only an
explicitly authorized operator may run it after native review. It uses the
existing unauthenticated engineering worker, not a generated protected launch.

The worker requests DEVICE_LOCAL_PUBLIC (VRAM|WRITABLE|PUBLIC), **not COHERENT**.
PUBLIC, matching atomic numerical results, and AQL fences do not authenticate
System atomic eligibility or ordinary-store visibility. The expert experiment
has an explicit operator-reviewed native-memory assumption; there is no safe
or protected dispatch through this harness. Existing shared-atomic and gfx950
protected gates remain closed. No CPU/GPU concurrent access, other-GPU use,
HBM residency/bandwidth, GPU event timing, general scheduling, or model claim.

## Asrock Results

The fixed matrix completed exactly once: 40 valid attempts and four invalid
length controls, with no retry, replacement, or matrix extension. The suite
passed its predeclared coverage policy, with 1,392 Ready consumers returning
the exact current input bits and 3,728 NotReady consumers returning +0.
Each initial pattern observed Ready in both consumer waves; across its eight
attempts, every cell in each wave was covered. NotReady also covered every
cell across the 40 valid attempts. These aggregate observations do not imply
a progress guarantee for any individual launch. The retained results include
valid launches with no Ready observation.

All 5,120 producer payload words matched their independent inputs. The four
invalid controls returned 1,024 Invalid/+0 results with unchanged payloads
and flags. All guards, input immutability and worker cleanup checks passed.
The native artifact has 80 explicit kernarg bytes, no implicit arguments,
34 SGPRs, 8 VGPRs, no LDS/private memory, and no spills. Grid256 is enforced
by the fixed harness/source contract, not an ELF maximum-workgroup field.

Ferric source revision was `5842ea2854713ae558a06271235ee41490b2a17c`;
ROCm was 7.3.0 / AMD LLVM 22. HSACO SHA-256:
`d2df70547618e16485774a64016148c479980f14b27aabe3652c330b55ea30f1`.
