# Projection-Residual Autoregressive CPU Qualification

The finite engineering route now accepts explicit four-forward autoregressive
decoding (AR4), alongside the existing teacher-forced mode (TF4). This checkpoint
was built and tested on ASROCK through `ssh mi350-2` on 2026-10-04. It is not a
new GPU observation or production admission.

## Changes

- Bind the requested mode, authentic seed and separate projection-residual
  image into the existing bootstrap and profile validation.
- Start AR4 with token `9112`. Every subsequent input must be the previous
  checked output, not a substituted teacher-forced token.
- Reject CLI/bootstrap mode mismatches before reading Begin or opening the
  native backend. Preserve terminal poisoning after invalid trajectories,
  malformed payloads, backend failures, EOF and premature Close.
- Preserve TF4 serialization, its profile formula and command-line spelling;
  retain the original bootstrap residual image and the distinct corrected
  projection image. No fallback route or new GPU arithmetic is introduced.

The change comprises four implementation files and five test files. Thirteen
new test methods yield fifteen executions because the wire tests also compile
into the parent. Tests use synthetic transports/backends; they are not model
outputs.

## Actual Results

| Selected test group | Passed | Ignored |
| --- | ---: | ---: |
| Runtime | 208 | 0 |
| Worker | 505 | 4 |
| Parent library and binaries | 324 | 0 |
| Total | 1,037 | 4 |

All 87 build/test phases exited successfully. The four ignored worker tests are
unchanged from the CPU1022 predecessor. All seventeen selected executables were
rebuilt from a fresh target; the default-feature check also passed. Source,
configuration, dependency, input, executable and protected-target postchecks
passed without drift. This selected qualification suite is not a claim that
every test in the entire repository was run.

The actual [completion receipt](complete.json) has SHA256
`7a3c170ca6ffe6000517588280fcdb99cb58c0cbb22cc3f4be910232f7d51c54`.
The [publication result](result.json) records the exact added and renamed tests,
formatted source hashes and all built executable identities. The
[ledger](ledger.json) binds the published files; [raw](raw/) retains all 435
command, start, result, stdout and stderr records. Seven larger source and
configuration maps and all ELF bodies are retained outside Git.

The data-only [publisher](publish.py) rehashed all 442 raw members, replayed the
named outcomes, reconstructed all phase commands, checked both source archives
and rehashed all seventeen ELF bodies. It generated [integration.patch](integration.patch)
without installing source. The primary agent then applied that patch and
verified all nine live files against their compiled hashes. The publisher's
`source_integration_performed: false` describes its own action, not this later
integration step.

## Remaining Gates

The matching [native AR4 run subsequently completed](../projection-ar4-native-v1/README.md).
Independent history-aware framework comparison remains pending.
Successful CPU tests alone do not demonstrate GPU execution, numerical
acceptance, model quality, overlap or sustained throughput. The target remains
single-request Qwen3-8B BF16 target-only decoding with a 2,048-token prompt and
256 generated tokens. No 700 tokens/s result is claimed; all issue #42 M0-M7
milestones remain open.
