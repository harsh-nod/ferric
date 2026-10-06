# S/RPO Qualification Helper Tests

All sixteen synthetic helper tests passed on MI350. These validate two
small harness components needed by the next full runtime compiler build;
they are not compiler, kernel, GPU or model tests and do not increase the
[445-test scoped compiler result](../guarded-mlp-core-kernel-error-identity-qualification-v1/README.md).

## Coverage

| Helper | Tests | Checks |
| --- | ---: | --- |
| Full-suite result parser | 8 | Exact named outcomes and ignored identities; rejects duplicate, missing, unexpected, malformed or contradictory records |
| Final backend archive selection | 8 | Exact Cargo target, package, profile and source; one ordinary backend rlib; rejects aliases, ambiguous products, ELF and thin archives |

The parser preserves historical ignored tests explicitly instead of treating
them as passes. The archive selector is specific to the backend rlib; it does
not relax ELF checks for executables or shared libraries. Both helpers still
need to be exercised against the actual full build products and test output.

## Actual Run

The [complete receipt](attempt-v1/evidence/complete.json) records one natural
zero exit, a reaped leader and absent process group. All sixteen exact test
IDs passed with no skips, expected failures or errors. The controller took
0.119 seconds. All five source files, the input manifest and Python/prlimit
pins were unchanged, and integrity postchecks were clean.

The [named transcript](attempt-v1/evidence/s-producer-helper-tests.stderr)
and [child result](attempt-v1/evidence/s-producer-helper-tests.stdout) are
retained byte-for-byte. The [manifest](attempt-v1/retention-manifest.json)
pins fourteen original files, including all seven raw evidence records.
Complete receipt SHA-256:
`7be7a582c407c8e61a32107db633036bac8b0d450466ad6deee3a0c3cf707a9a`.

The isolated Python child used the unchanged owned-process supervisor,
CPU cores 8/9, nice 10 and hidden GPUs. Its limits were 120 seconds per leaf,
180 seconds overall with 50 seconds reserved for cleanup, 12 GiB address
space, 1 GiB per file, 6 GiB scratch and 64 MiB output streams. No Cargo build
or GPU operation ran in this checkpoint.

Next: qualify the assembled S/RPO compiler with its full historical test
rosters, the new wrapper tests, final product pins and extraction controls.
Loader checks, guarded gfx950 HSACO/GPU execution, independent model numerics
and sustained Qwen3-8B BF16 2,048/256 performance remain separate open gates.
