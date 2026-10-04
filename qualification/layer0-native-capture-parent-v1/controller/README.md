# Candidate-Only Layer0 Parent CPU Draft

This narrow successor reuses the exact clock-parent CPU controller/helper APIs
and actual parent275 baseline. Twenty pure policy tests are authored, not run.
The author has not imported this package or executed Cargo, tests, or native code.

## Retained Source Inputs

The paired source manifest is authenticated at
`layer0-native-capture-source-inputs-v228-v2.json` (844 bytes,
`c52178efe8973549afa55c09d4c7615a88dee407f537c2bb95496455300664f9`).
It contains fresh Ferric `1a9a2551ee7af44d5482cc37f46cf008158b8d10` and
the unchanged runtime `27b53d2b74c1f239988a891a4aed39e089b05663` archives.
All seven after-pins match the retained remote formatter output under
`p228-layer0-native-capture-source-v1/draft`. Its 8256-byte completion is
`d718a2638fbcc6135130c98dd64178ccbc3ec0d340ca7b15acc0b10f0c4e3421`;
both format/check leaves exited naturally with zero status. The source manifest
is 3468 bytes, `dc320fc30ee93a2568aedea5148b462a7a0b48f81621800ec381737bfd4dd279`.
These are source-formatting results, not Rust compilation or test results.
Root freezes the four package files and executes qualification separately.

The seven declared destinations contain three replacements and four additions.
Only one new opt-in binary registration is allowed in the structured Cargo
manifest; dependencies, locks and features cannot change. No worker or runtime
source overlay is authorized.

## Existing Coverage And Proposed Extension

The pinned actual `gfx950-clock-parent-cpu-v228-v1/complete.json` is
316284 bytes, SHA
`d2a118dd2a3bfac749b18de26883a661a1078a22ebf374853a11b081f43b1484`.
The controller replays its 44 exact commands/environments, 223 raw records,
765-name full library inventory, 262 selected library tests and 13 binary
tests: 275 passed, zero ignored. This is a historical result, not a result of
the new source proposal.

The new full inventory must equal the historical names plus the exact thirteen
declared capture/evidence names. Both nested modules run once through the
existing `parent-client` selector; all wire/data selectors stay unchanged.
All thirteen existing binaries retain their original test inventories.
The new binary has one declared CLI test. Only its list/test phases are added;
the existing build phase also emits it. This yields 46 phases and 233 raw
records, including three source maps. Expected 289 passes are accepted only
after actual inventories and named results agree. The default-feature library
check and all fourteen actual binary artifact selections remain mandatory.

## Source, Toolchain And Resource Custody

Fresh compiled local Ferric subtrees, the complete shared-worker subtree, and
root Cargo/toolchain/config files must match the CPU275 source map before the
parent overlay. Only the explicitly named parent and worker README bodies are
excluded from this historical comparison; full new maps retain them.
The parent builds 28 local Ferric packages against 181 locked external packages.
The sibling runtime archive and native worker are not rebuilt; the latter's
wire/data/test sources are imported by path. The result keeps both rebuild
claims false and preserves all external dependency identities.

Root runs on ASROCK through its explicit mi350-2 SSH override, with the unchanged
`/home/harmenon/ferric-asrock-42` paths, toolchain and offline Cargo cache.
The target is fresh and empty. The unchanged bounded helper retains jobs2,
CPU8/9, nice10, 12GiB address-space limit, 6GiB target limit, 40GiB setup and
38GiB active free-space floors. Stable 1.97.1, bootstrap, host opt2, zero debug
info, incremental-off and empty GPU visibility are unchanged. Metadata has
120 seconds; each other phase has 1200 seconds. No automatic cleanup is added.

Inputs, copied sources, tools, dependency manifests and selected actual binary
artifacts are rehashed after execution. Failures remain failures. Root owns
formatting, pure/Rust qualification, retention and eventual publication.

```text
python3 p228-layer0-native-capture-cpu-v2/run.py MANIFEST_SHA layer0-native-capture-cpu-v228-v2
```

Run only after other Cargo jobs are terminal and fresh shared-host capacity
checks pass. This qualification does not launch a native process, execute on
GPU, accept numerical results, reproduce HSACO, calibrate clocks, or establish
performance or production authority.

## Initial Packaging Rejection

The V1 attempt exited before compilation because the root-created archive
lacked the extractor's required `ferric/` prefix. Its input files and empty
output directories remain preserved. This V2 package changes only input
locations to a separately corrected archive and uses a fresh output directory;
the extractor and all validation requirements stay unchanged. No Rust or GPU
failure was observed in that attempt. The new source code and test roster are
unchanged; policy tests must be rerun for this exact package.
