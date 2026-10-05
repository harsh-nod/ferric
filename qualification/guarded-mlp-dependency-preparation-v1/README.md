# Guarded MLP: Locked Dependency Preparation

Engineering prerequisite for [issue #42](https://github.com/harsh-nod/ferric/issues/42),
performed on MI350 on 2026-10-05. Offline vendoring passed for the CPU-qualified
kernel crate and pinned nightly Rust sources. This checkpoint is dependency
preparation, not checked compilation, GPU execution or model acceptance.

## Results

| Check | Result |
| --- | --- |
| Initial locked offline vendor | Failed: missing `rand_xorshift` index entry |
| Separate locked online fetch command | Exit 0; 14 crate archives downloaded |
| Fetch controller preservation check | Failed: older extracted cache packages disappeared |
| Cache-preservation fixtures | 4 passed |
| Restoration policy fixtures | 8 passed |
| Missing-only cache restoration | 133 packages, 9,095 files restored |
| Second locked offline vendor | Passed; 145 packages, 6,352 files |

The final vendor tree contains 173,356,107 bytes. Its recorded configuration pins
the actual Pliron Git revision `161c385576d45d4e634ba179fa93a545b91124e6`.
Kernel sources, nightly sources, lockfiles, selected tools and CPU products
passed integrity postchecks. No generated Cargo configuration was installed.
The vendor command exited naturally and its process group was absent afterward.

See the [successful vendor receipt](retained/guarded-mlp-vendor-v228-v2/evidence/complete.json),
[summary](result.json), and retained raw commands, logs and byte inventories.
Dependency bodies and compiler binaries are not copied into this checkpoint.

## Shared-Cache Incident And Repair

The [fetch receipt](retained/guarded-mlp-fetch-v228-v1/evidence/failed.json)
remains failed despite the download command's zero exit status. Before/after
inventories showed 9,095 source files disappearing from 133 unrelated cached
packages. All original crate archives remained unchanged. The qualified
kernel dependencies and source lockfiles passed their separate checks.

This is consistent with Cargo's automatic cache cleanup, which can run during
`cargo fetch`; the inventory alone does not identify the deleting process.
Future online preparation must explicitly set
`CARGO_CACHE_AUTO_CLEAN_FREQUENCY=never`. Cargo documents both this control and
the suppression of automatic cleanup in offline mode in its
[configuration reference](https://doc.rust-lang.org/cargo/reference/config.html#cacheauto-clean-frequency).

The [repair receipt](restoration/guarded-mlp-registry-restore-v228-v1/complete.json)
records restoration of all 229,085,242 missing bytes. The repair acquired
nonblocking Cargo cache locks, verified every archived body against the original
inventory, staged all packages, then installed only wholly missing directories
with no-replace renames. Cargo's small completion markers came from an unchanged,
hash-matched existing marker. A full final census checked restored and unaffected
bodies, including archives and index entries. Existing files were not overwritten.
The repair establishes original contents, not original file modes, which were
not recorded. The failed fetch receipt was not changed or relabeled successful.

## Next Gates

Run the checked compiler with these exact dependencies and the separately
[audited tools](../guarded-mlp-compiler-tools-v1/README.md), then inspect the emitted
ABI/ISA and qualify GPU guard behavior. Reusable arena, full-worker correctness,
independent numerical acceptance, sustained 2,048/256 and 700 tokens/s remain
open, as do all M0-M7 milestone gates.
