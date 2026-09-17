# Asrock Engineering Bring-Up

The `mi350-2` SSH alias now reaches the Asrock gfx950 host. These observations
are an engineering bring-up, not protected runtime qualification or a full
Qwen inference result.

## Exact Platform

fe2o3 revision `cb8f51ec0b52894a1fb510bc3f523005dda8bfaf` adds a separate,
feature-gated profile for kernel `5.15.160+`, amdgpu `6.16.15`, and module
srcversion `9462451703604FCD7EC2365`. The profile keeps strict KFD/PCI device UID
equality, SPX/NPS1, XNACK-off, firmware, geometry and currentness checks. It does
not inherit the previous host's XCP UID exception. Driver source review and
CPU C-layout oracles retain the trusted-driver assumption; they do not
authenticate the loaded module or firmware. Default/protected gates remain
unchanged. See the profile's source and engineering notes in fe2o3.

Both default and engineering KFD test suites passed (423 and 504 tests,
respectively, each with one hardware-dependent test ignored), as did strict
engineering Clippy. The final profile-only checks were rerun after adding the
last provenance hash. The read-only identity example then passed device
binding, two currentness checks and descriptor-count cleanup on Asrock.

## Qwen Numerical Baseline

The existing Rust-derived Qwen3-0.6B K-projection and K-normalization artifacts
were reused unchanged. Fresh references and bindings were prepared for the
new worker before any GPU execution. All five cases passed: zero, basis,
mixed, cancellation and epsilon. Each case executes two completion-ordered
dispatches and checks 1,024 projection, quantization and normalized values,
allocation guards, input immutability and cleanup. This is not a megakernel.

Projection maximum absolute error was zero except for the mixed case
(`1.1920928955078125e-7`, or `0.005085048481565353` of its derived error bound).
All cases passed the existing strict staged sqrt/div normalization policy;
this does not assert bitwise parity with a framework's rsqrt implementation.

Worker SHA-256: `6f6e13ef2e15aedbb4e5d2cc966421c69d99784049ba00334e8334d4c97c7a4b`.
Raw records are retained under
`/home/harmenon/ferric-asrock-42/evidence/knorm-asrock-v1/` on Asrock. Private
device selectors and worker stderr are not published. Post-run GPU/UMC
activity was zero and used VRAM remained 283 MB. No clocks, driver settings,
firmware, or other users' processes were changed.

## Finite Megakernel Scheduler

The unchanged seven-task integer scheduler was rebuilt from Rust on Asrock,
using clean Ferric revision `5842ea2854713ae558a06271235ee41490b2a17c`, the
compiler revision above, and ROCm 7.3.0 / AMD LLVM 22. The fixed numerical suite
passed all 20 dispatches: 16 valid epochs and four stale-epoch rejections.
There were 112 exact independent payload comparisons within 260 checked state
words. Both workgroups participated and four dependency edges crossed between
them in every valid epoch; this is measured ownership, not an inference from
launch geometry. All input, guard, queue, allocation and process checks passed.

The launch has two workgroups, 128 lanes each, with two wave64 waves per
workgroup. Native resources are 77 SGPRs, 18 VGPRs, 1,024 LDS bytes, zero private
memory and no SGPR/VGPR spills. The host fixes grid256; ELF does not independently
encode the maximum workgroup count. Post-run GPU/UMC activity was zero and used
VRAM remained 283 MB. No GPU-only timing or performance claim is made.

[Sanitized results](../../../qualification/gfx950-task-graph-v1/evidence-asrock-v1.json)
retain per-epoch ownership and hashes of the underlying reports and states.
Native build records are under `evidence/task-graph-asrock-native-v1/`; GPU
records are under `evidence/taskgraph-asrock-gpu-v1/`, both within the Asrock
work root above. HSACO SHA-256:
`fe55cc7167fd9e6859ba3ea4cf5f66ccfdb40c91e30be4bf05d8139d46298187`.
This establishes the finite atomic scheduler core, not ordinary tensor
visibility, a full Qwen model, an unbounded runtime, or production admission.

## Ordinary Payload Publication

The separate V31 one-shot publication fixture was also built from Rust using
the same clean sources and compiler. The checked V2 handoff preserved its
LLVM bytes; ROCm 7.3.0 produced the gfx950 HSACO. Offline LLVM/ISA review
confirmed the guarded ordinary payload load, producer payload store before
READY release, and consumer REQUEST release followed by acquire. Existing
proof, descriptor and protected-runtime gates were not relaxed.

The predeclared 44-attempt GPU suite passed without retry or replacement:

| Check | Observed result |
| --- | --- |
| Valid launches / invalid-length controls | 40 / 4 |
| Exact producer payload comparisons | 5,120 |
| Ready consumers, exact current producer bits | 1,392 |
| NotReady consumers, +0 results | 3,728 |
| Ready coverage per initial pattern | Both wave64 ranges, all 128 cells |
| NotReady coverage across valid launches | Both wave64 ranges, all 128 cells |
| Invalid/+0 results, payload and flags unchanged | 1,024 |
| Guard, immutable-input and cleanup checks | Passed |

[Full sanitized results](../../../qualification/gfx950-static-publication-v1/evidence-asrock-v1.json)
retain all per-attempt Ready/NotReady cell sets. The experiment has no polling
or progress guarantee; several valid launches observed no Ready consumers.
PUBLIC memory is not COHERENT, and this numerical observation does not
authenticate general System atomic or ordinary-memory eligibility. The
engineering worker still requires explicit unauthenticated-code admission.

Native resources are 34 SGPRs, 8 VGPRs, zero LDS/private memory and no spills.
The launch is two WG128 workgroups, each with two wave64 waves; the 80-byte
kernarg has no implicit arguments. [Artifact metadata](../../../qualification/gfx950-static-publication-v1/artifact-asrock-v1.json)
binds source and native object. Raw build and GPU records remain under
`evidence/publication-asrock-native-v1/` and `evidence/publication-asrock-gpu-v1/`
in the Asrock work root. No performance or full-model result is asserted.
