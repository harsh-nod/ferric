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
