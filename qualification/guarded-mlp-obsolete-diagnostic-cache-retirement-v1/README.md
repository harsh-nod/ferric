# Obsolete Diagnostic Cache Retirement

Three failed diagnostic generations' build caches were removed from `mi350`
to provide space for the next isolated compiler qualification. The exact
[controller](controller.py) and [terminal receipt](receipt.json) are retained.

| Failed Generation | Removed Build Files | Logical Bytes |
| --- | ---: | ---: |
| Core Result diagnostic V1 | 1,939 | 2,120,281,521 |
| Checked attention diagnostic V1 | 1,939 | 2,122,561,681 |
| Checked attention diagnostic V2 | 1,939 | 2,122,498,291 |

Each generation's empty `tmp` was also removed. The controller prevalidated
all six directories, exact failed receipts and archives, 57 natural/reaped
child outcomes, ownership and ordinary-file inventories before deleting
anything. It checked owned-process references with elevated metadata access,
then irreversibly dropped to UID 9661 before deletion. All 309 preserved
evidence pins were rechecked afterward. Source, inputs, raw records and
archives remain; no successful generation was touched.

Free space changed from 43,613,700,096 to 49,381,679,104 bytes during cleanup.
Other users share this filesystem, so this difference is not an isolated
physical-byte measurement. The process scan is a point-in-time check, not a
lock against future processes. The source manifests are preserved; this
maintenance run did not independently rehash every source-tree body.

Controller SHA-256:
`7cbe91e01dd0468c45337fc12352d7ce45699baf1d1c8084adf0411d7075f7d6`.
Receipt SHA-256:
`ef57b584655f25dc5b0b506bdc0bd5c0b3390d6f528bd6776a74e88b0c92e092`.

Original evidence: [Core Result](../guarded-mlp-core-result-diagnostic-v1/README.md)
and [checked attention](../guarded-mlp-core-checked-attention-diagnostic-v1/README.md).
This storage cleanup adds no compiler, GPU or performance qualification.
