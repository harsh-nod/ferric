# MI350 Coupled Census CPU Result

The fresh attempt on ssh host mi350 passed all 26 phases in 158.984 seconds.
Each supervised phase exited naturally with code zero, was reaped, and left
its owned process group absent. Source and dependency postchecks were clean.
No model or GPU execution occurred.

| Check | Result |
| --- | ---: |
| Full runtime suite | 1,165 passed; 8 existing ignored |
| Full worker suite | 779 passed; 4 existing ignored |
| Focused runtime groups | 9 passed |
| Selected facade doctests | 10 passed |
| Data-only doc-parser regressions | 8 passed |
| Qualified Cargo products | 11 |
| Source-map entries | 1,049 |
| Original raw evidence files | 137 |

All inherited named outcomes are preserved. The additions are 13 runtime and
19 worker tests, including the actual worker executable's new selector/EOF
integration test. Its executable bytes were identical before and after tests
and at the final build.

The runtime tests cover rank-ordered fences, full snapshot and ID drift,
owner assertions, malformed checkpoint counts, failure and unwind at every
boundary, and legacy behavior. Worker tests cover real-ledger derivation,
first-use and Full refusal, original policy framing, checked counters,
post-runtime rejection, terminal behavior and no silent fallback.

## Original Identities

- Terminal: [complete.json](evidence/complete.json),
  2,442,525 bytes, SHA256
  `4d68df6862da654f9044cb08ad5ecedf008fb96f1fa72345d5382d6c96b89e41`.
- Final source map: [sources-after.json](evidence/sources-after.json),
  431,540 bytes, SHA256
  `145b177805d0eefdac0e868729d28d26f1646259116a9ff10e316250e1668d69`.
- Worker ELF: 6,476,504 bytes, SHA256
  `00beb857fd00914f1f305f9646c2087e20901b81f332d774b8211240497e6f95`.
- Original archive: 1,800,040 bytes, SHA256
  `a6a74ca07f55b9dbb32db2b6ed5ae2a9e72673d54b6e3cbd2d7a74a6f4f31b02`.

The [manifest](manifest.json) pins 190 original bodies; it is the 191st archive
member. This README was added after retention and is not an archive original.
The archive includes the original terminal, all raw records, ten lineage bodies,
the 25 actual formatted Rust postimages, executed helpers and source/cache
receipts. ELF and dependency bodies remain on MI350; local retention does not
claim to rehash them on this machine.

The build used an authenticated private offline Cargo cache, two jobs,
CPUs 8/9, nice 10 and hidden GPU variables. The unchanged limits were
3,600 seconds overall, 1,800 seconds per leaf, 50 seconds for cleanup,
12 GiB address space, and 40/38 GiB starting/live storage floors.

This result qualifies the runtime/worker source, not the parent, native GPU
execution, numerical acceptance, performance or Full2303 launch.
