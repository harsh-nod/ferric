# M1 MI300X Hardware Transcript Evidence

`proofs/m1/evidence/validate-hardware-transcript.py` implements protocol
`ferric.m1-validator.hardware-transcript.v1`. Its reviewed source SHA-256 is
`1c84dbe9f4bfea8d4e3a1859522320b56848c39f61a949c7244745cd995a070b`.
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

The case roster uses format `FERRIC-M1-HARDWARE-CASE-ROSTER-V1`. Cases are
nonempty, unique, canonically ordered, bound to the same obligation, property
roster, profile, and path, and name a non-placeholder procedure identity. Each
case explicitly requires GPU work.

The run transcript uses format `FERRIC-M1-MI300X-HARDWARE-RUN-V1` and test
protocol `ferric.m1.mi300x-hardware-test.v1`. It requires:

- the fixed target `gfx942:xnack-` and exactly one physical device identified
  as `AMD Instinct MI300X`, PCI vendor `1002`, processor `gfx942`, with XNACK
  disabled and a canonical non-placeholder device UUID;
- identity records for the ROCm installation, `amdgpu` driver module, firmware
  bundle, and the fixed Ferric hardware harness/protocol;
- ordered results for every rostered case, with a non-placeholder observation
  identity, a positive launch count, the same positive completion count, and
  result `pass`; and
- explicit true submitted/completed GPU-work observations and explicit false
  `no_gpu_work`. Empty, CPU-only, skipped, partial, failed, reordered, injected,
  or self-described no-GPU runs are rejected even when their files are
  internally rehashed.

The report uses format `FERRIC-M1-HARDWARE-TRANSCRIPT-REPORT-V1`, repeats all
binding identities, independently derives the device and environment
identities, and requires the exact case and aggregate launch/completion counts.

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
```
