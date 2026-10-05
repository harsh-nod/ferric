# Projection AR4 Observer Runtime Preparation

The [qualified observer parent and worker](../projection-ar4-host-observation-v1/README.md)
were deployed to MI350 at their original recorded paths. Fresh executable audits
and request assembly completed on 2026-10-05 UTC. This checkpoint is preparation
for a native observation, not evidence that the GPU run has passed.

## Completed Checks

| Check | Actual Result |
| --- | --- |
| Runtime selector tests | 14 passed; no failures or skips |
| Input assembler tests | 8 passed; no failures or skips |
| Parent runtime audit | `readelf` and `ldd` passed; four resolved libraries |
| Worker runtime audit | `readelf` and `ldd` passed; three resolved libraries |
| Owned audit processes | All four exited naturally, were reaped, no forced cleanup |
| Input assembly | Four fresh files produced; source and platform postchecks passed |

The selected parent is 13,938,936-byte SHA-256 `1fad2d8aafe9ce4e799322d42d1015756ead228bf2f237c302ee260fecb656fd`.
The worker is 5,159,512-byte SHA-256 `14135b08c276ba9d38fcbae635fe14c3db8bd11b934ccf5ac8d51e7c2876bd75`.
Both deployed copies were executable, single-link regular files owned by UID9661.
The parent resolves libgcc, libm, libc and the loader; the worker does not require
libm. Neither audit found a missing dependency or an RPATH/RUNPATH override.
Host, boot and selected topology identities were stable across the audit records.

The selector reuses the frozen runtime audit engine unchanged. The new adapter
selects actual observer CPU products, not the preceding CPU1037 binaries. Its
tests use the real frozen CPU contract with synthetic fixtures; they are not
GPU tests. The assembler tests cover lossless u64 IDs, exact request changes,
freshness, review requirements, alias restoration and retained input bytes.
All six adapter source/test/document bodies were unchanged before and after tests.

## Controlled Request

The original four-forward autoregressive request changes only `decode.worker`,
`decode.session` and `decode.evidence_directory`. The new parent is selected in
the outer plan. Prefix `29fd58e7`, SiLU `b0d1766f`, projection `25338bea`, all
original Begin images, model, prompt, devices and deadlines stay unchanged.
No historical output token is forced. A prompt file containing 2,048 tokens
does not make this four-forward route a 2,048-token prefill.

Fresh runtime reviews bind the new executables and actual audit records.
The existing full-currentness policy, one-attempt lifecycle and six surrounding
GPU audits remain required. Host counters are nested and inclusive, not disjoint
GPU durations. No speedup, overlap, numerical acceptance or throughput is claimed.

## Evidence

- Parent audit completion: `e62da84ebbe4c62a12cefb75fdae205c350ba4db5425f80863bf68da4d78f7a8`.
- Worker audit completion: `53df676155305a92cf9951e1e3094fae6be99f1977d5e6926d0d901f81870f04`.
- Assembled plan: `6928c127cf5a85d7e41e4a67bf851a299e8723db85a24bdb74bf32abcee016e7`.
- Assembled request: `850d0d7bd521a2a416161e6444c8d865ae53d186828ff6070f1336962b6c522c`.

`audits/` preserves both complete seventeen-file audit trees. `tests/` preserves
primary SSH test observations and before/after source hashes. `source/` contains
the executed adapters and tests; authored-status wording in those copied
documents is retained unchanged. `inputs/` contains the actual assembly,
request, plan, engineering notes and fresh runtime reviews. No executable,
weight or shared-library body is published.

The GPU observation is the next independent execution step. All issue #42 M0-M7
acceptance milestones, full-model correctness, sustained BF16 2,048/256 decoding
and the 700 tokens/s target remain open.
