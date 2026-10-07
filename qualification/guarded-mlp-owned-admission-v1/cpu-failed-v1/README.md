# Original CPU Attempt Failure

The first MI350 attempt stopped after five clean, naturally retired phases:
formatting, formatting check, compiler version, lock generation and Cargo
metadata. No library tests or timing samples ran.

The controller incorrectly required `.cargo-checksum.json` in Cargo's
registry extraction. The actual cache uses `.cargo-ok`; the fresh V2
controller instead authenticates the exact extracted roster and bytes against
the original pinned `.crate` archives, admitting only that exact marker.
The original failure was not modified or rerun in the same directory.

The [original receipt](evidence/failed.json) is 1,052,453 bytes, SHA-256
`dd9b6b534ae61a927e468be3a37fbe9281adc696b6b066dc6820915805ac252a`.
It records no postcheck errors, no tests, no report and no admitted dependency
inventory. The empty inventory does not mean dependencies were absent.

The original archive is 2,564,241 bytes, SHA-256
`9f00ce707f38601af205a65ad258f9da68c7b2d8204da20afa11114ccd32a514`:
877 members, 876 manifest pins, 15,081,194 expanded bytes. All 843 actual
source bodies, 30 raw evidence files, terminal, original stage ledger and
exporter are retained. This README and retention.json are added commentary.
Local retention did not run project code or rehash unavailable remote tools.
