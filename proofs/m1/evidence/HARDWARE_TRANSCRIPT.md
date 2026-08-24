# M1 MI300X Hardware Transcript Evidence

`proofs/m1/evidence/validate-hardware-transcript.py` implements protocol
`ferric.m1-validator.hardware-transcript.v1`. Its reviewed source SHA-256 is
`bfc3a952a0ebac4eee479faf7d7306d2a8a3889ffb22ad9ec3422fcd8b1eace0`.
The production evidence-index checker owns the path, protocol, and source pin;
an evidence index cannot select or substitute the validator.

## Canonical layout

For a `hardware-test` artifact named `<artifact-id>`, the validator admits
exactly these canonical, pretty-printed ASCII JSON files below the nonsymlink
evidence root:

```text
artifacts/<artifact-id>.hardware-transcript.json
hardware-rosters/<artifact-id>.json
hardware-transcripts/<artifact-id>.json
```

The report authenticates the exact byte length and SHA-256 of both immutable
companions. Each file must be a stable, single-link regular file with no
symlink in its path. Duplicate keys, unknown or missing fields, noncanonical
serialization, traversal, substitution, size drift, digest drift, and changes
during a read are rejected.

## Bound run

All three files bind the checker-provided evidence binding and the exact Open
roadmap requirement or assurance property, statement, evidence profile, path,
requirements manifest, and selected source identity. They also bind the
ordered Ferric and fe2o3 commits, trees, and measured source-closure SHA-256
values, plus the complete ordered compiler, hardware, and runtime TCB roster.
The profile is accepted only when the requirements manifest assigns it the
`hardware-test` evidence kind.

The case roster uses format `FERRIC-M1-HARDWARE-CASE-ROSTER-V1`. It contains
exactly one binding-local K7 case, bound to the same obligation, property
roster, profile, path, and checked-in procedure identity. The case explicitly
requires GPU work.

The run transcript uses format `FERRIC-M1-MI300X-HARDWARE-RUN-V1` and test
protocol `ferric.m1.mi300x-hardware-test.v1`. It requires:

- the fixed target `gfx942:xnack-` and exactly one physical device identified
  as `AMD Instinct MI300X`, PCI vendor `1002`, processor `gfx942`, with XNACK
  disabled and a canonical non-placeholder device UUID;
- operator-declared identity records for the ROCm installation, `amdgpu` driver
  module, and firmware bundle, plus the fixed Ferric hardware harness/protocol;
- the held harness binary hash and byte length exactly matching the reviewed
  pin in the checked-in procedure, harness-emitted package version, and exact
  hashes of the five named Ferric harness/runtime source files;
- the authenticated semantic kernel-manifest and program-catalog identities;
- one full K7 result joined to the roster's binding, case, and procedure, with
  program `k7-speculative-token-assembly-s1k4`, grid and workgroup `[64,1,1]`,
  positive generation, exact output tokens `[10,11,12,13,14]`, one launch and
  completion, verified output, released queue, and result `pass`; and
- explicit true submitted/completed GPU-work observations and explicit false
  `no_gpu_work`. Empty, CPU-only, skipped, partial, failed, reordered, injected,
  or self-described no-GPU runs are rejected even when their files are
  internally rehashed.

The report uses format `FERRIC-M1-HARDWARE-TRANSCRIPT-REPORT-V1`, repeats all
binding identities, independently derives the device and environment
identities, and requires the exact case and aggregate launch/completion counts.
It repeats both kernel identities. The validator independently recomputes the
domain-separated K7 observation digest and every named Ferric tool-source hash,
and enforces the procedure's reviewed harness byte-length/SHA-256 pin.
The source hashes provide source association only; they are not reproducible-
build proof or proof that the recorded binary was built from those sources.

## Ferric producer

`proofs/m1-qualification/produce-hardware-transcript.py` implements the
planner's exact 58-binding producer family. Every public invocation ends in one
`binding.NNNNN`, authenticates the complete plan/queue/source/TCB/repository
custody plus the exact harness, kernel artifact tree, checked-in K7 procedure,
and hardware environment input, then calls `ferric-m1-hardware-harness`
exactly once.
The canonical singleton request and result schemas are fixed in
`proofs/m1-qualification/hardware-k7-procedure.json`.

The hardware environment input names the KFD GPU unique ID and exact
AMD SMI device UUID derived from that unique ID and the PCI BDF. ROCm,
amdgpu-module, and firmware values are operator declarations, not independent
attestation. Python independently derives the UUID, cross-checks the returned
device and environment, and recomputes the five held Ferric source hashes. It
projects one distinct K7 case with exactly one submitted, completed,
read-back-verified launch; the K7 observation is not evidence of the selected
source path's semantics.

The producer creates the case roster, then the run transcript, and publishes
the report last. A failed transaction attempts reverse-order cleanup only for
the exact files created by that invocation; rebound entries are preserved and
reported as rollback failures. It neither imports nor invokes the trusted
validator.

## Authority boundary

Acceptance grants only `hardware-observation-only` authority. It authenticates
bounded observations recorded by the exact named harness for the exact named
run. Observations are not proofs, do not establish machine refinement, and do
not establish performance or M1 qualification. This source-only validator does
not run a GPU, create hardware evidence, create an evidence index or receipt,
or close any roadmap requirement, assurance property, or path obligation.

The hostile policy test covers Roadmap and Assurance bindings across
composition, kernel, runtime, and qualification profiles, Ferric and fe2o3
paths, identity replay, no-GPU claims, partial work, source/TCB/environment
drift, malformed and duplicate JSON, symlinks, hard links, and a simulated
in-read TOCTOU change:

```text
python3 -I proofs/m1/evidence/test-hardware-transcript-policy.py FERRIC_REPO

python3 -I proofs/m1-qualification/test-hardware-producer-policy.py \
  FERRIC_REPO FE2O3_OBJECT_REPO
```

The producer policy's 58 binding-local executions use a deterministic synthetic
harness. They validate producer topology and custody but do not claim 58
physical MI300X launches.
