# MI350 Shared-Full Runtime Preparation

The two CPU883 observer parents and common worker passed fresh runtime-library
audits on MI350 on 2026-10-05 UTC. Both reviewed input assemblies completed.
This is preparation for the [default/shared comparison](../projection-ar4-shared-host-gpu-preparation-v1/README.md),
not evidence that either new GPU arm has completed.

| Executable | Resolved libraries | Audit result |
| --- | ---: | --- |
| Default observer parent | 4 | readelf and ldd passed |
| Shared-full observer parent | 4 | readelf and ldd passed |
| Common worker | 3 | readelf and ldd passed |

All six bounded commands exited naturally with empty stderr, reaped processes
and no forced cleanup. Recorded topology was unchanged before and after each
audit. Both parents resolve libgcc, libm, libc and the ELF loader; the worker
resolves libgcc, libc and the loader. No missing dependency or RPATH/RUNPATH
override was present. These are actual executable compatibility observations,
not a fresh compiler qualification or proof of GPU behavior.

## Matched Inputs

Each assembly binds the actual CPU883 completion, passing 122-test supervisor
receipt, newly deployed executable identities and fresh runtime reviews.
Both preserve the exact GPU u64 identifiers, model, prompt, seed, BF16 arithmetic,
images and deadlines. Relative to the prior successful AR4 request, only the
worker, session and evidence directory change. Between the two new requests,
only session and evidence directory differ; route selection is a parent/CLI
distinction. Each assembly records 1,811 consumed identities and successful
integrity postchecks.

The two review records explicitly retain the policy distinction: the default
route stays false-only; the shared route configures `(false, false, true)` before
observer enable. Both preserve fresh checks and one-attempt owned execution.
The assembly copies root decisions and does not generate approvals itself.

## Transport Correction

The first data-only assembly stopped before output creation or GPU launch because
the transfer archive hard-linked the packaged and external test-wrapper copies.
The assembler's single-link check correctly refused them. Root replaced only the
external copy with identical bytes, preserving the packaged file. Both then had
link count one and the original `c3f3c069...` SHA-256; no source byte or admission
rule changed. The successful default assembly followed this correction.

A separate local check caught incorrectly split session strings before remote
assembly; the deployed configurations contain distinct 32-byte sessions. These
preparation errors and corrections are retained in the
[primary observation](primary/observation-1.json), not hidden as GPU retries.

## Evidence

- [Default-parent audit](audits/default-parent/complete.json): `69e5b2c47724cc8dbe87693a708baca560f8a4b8bdc1f40a99df49afefd4e61c`.
- [Shared-parent audit](audits/shared-parent/complete.json): `f542d7d4d2ee86a82b22c6e5e90a4ee08d60cb7775b09c461dd5816190a1af1d`.
- [Worker audit](audits/worker/complete.json): `0c8c15d9a197909a730192cd2d1e3e4dbafe9c8f53decb8753a312646fd5c330`.
- [Default assembly](inputs/default/assembly.json): `719887bb6b1cbf20071d70f9962d8ddf1e126723102a3e0409c4dea1537b488c`.
- [Shared assembly](inputs/shared/assembly.json): `c75cc37855c8d1348e8e4d7960edda91fbbfbcccc32886685ba80ae058e11b7f`.
- [Publication inventory](result.json): byte-exact copies of all 51 audit files,
  eight assembly files, seven root inputs, the primary observation and publisher.

The local publisher verifies retained metadata and body hashes. It does not
reopen remote executables/libraries, replay the full assembly input graph, rerun
commands or infer GPU success. The paired native run must independently pass its
fresh device/process audits, lifecycle checks and output validation.
All issue #42 milestones, numerical acceptance, sustained 2,048/256 decode and
700 tokens/s remain open.
