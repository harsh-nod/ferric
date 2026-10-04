# Projection-Residual Capture Supervisor

On 2026-10-04, all **34 synthetic policy tests passed on `mi350`** in
14.439 seconds. No tests were skipped. Sources were identical before and after
the run. These tests did not execute either model binary or launch a GPU kernel.

The [retained receipt](evidence/complete.json) has SHA256
`763050083ca48d606e3e76e94978ecb792a944fd568247eab454176b5926f955`.
The [transcript](evidence/tests.log) records every actual test. The
[bounded runner](run_pure.py) used two CPU cores, nice10, hidden GPUs, a 2 GiB
address-space limit, a 120-second CPU limit and a 180-second outer deadline.

## Coverage

| Suite | Passed | Scope |
| --- | ---: | --- |
| Capture validation | 13 | New image/profile binding, all 28 typed arrays, logical KV indexing, untouched cache bytes and Close-only capture |
| Input validation | 10 | Joint-qualified parent/worker, checked image provenance, unchanged workload and separate request schema |
| Supervisor | 11 | One native attempt, six audits, owned cleanup, failure retention and source checks |

The [exact tested package](source/manifest.json) preserves the original captured
source bytes, including the author's pre-execution notes. This qualification
README records the subsequent actual test result. It is not an installed
standalone package: the retained controller uses the original evidence layout and
historical checked helpers named in [intake](source/INTAKE.md).

## Next Hardware Gate

The supervisor selects the [CPU-qualified runtime](../projection-residual-runtime-v1/README.md)
and [checked gfx950 residual image](../projection-residual-lowering-v1/README.md).
It retains one genuine layer-zero capture, requires fourteen unchanged
pre-residual arrays and validates both complete KV buffers. The downstream
values may change because the candidate materializes the projection in BF16
before residual addition. It does not require equality with the old final hidden
state or reinterpret an old observation as candidate evidence.

Fresh runtime compatibility checks and the actual GPU capture remain pending
at this checkpoint. The [separate numerical comparator](../projection-residual-comparison-v1/README.md)
must then check both residual formulas and report framework differences.
These tests establish no full-model numerical acceptance, performance result,
production admission or progress against the sustained 700 tokens/s target.
