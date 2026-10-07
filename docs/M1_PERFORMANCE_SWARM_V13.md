# Performance Swarm V13

Updated October 5, 2026 (PDT). Both same-day HTTP cohorts are complete.
This is a scoped engineering comparison, not a production qualification or
performance win. All 33 M1 gates remain open.

## Scope

The user requested a new comparison with the latest changes. There is no
supported all-optimizations-enabled profile: the current experimental modes
have explicit composition restrictions. Today's candidate is the newest
CPU-qualified packed-down profile, not a claim that all local changes are active.

| Profile | Admitted Composition | Important Exclusions |
| --- | --- | --- |
| Packed down R1, today's candidate | V27 prefill16, split8 attention, ordered64, wave kernels, packed down | V19 KV copy, packed gate/up, token programs, active polling, grouped prefill and prefill32 |
| Packed gate/up R2 | V19 KV copy, split8, ordered64, packed gate/up | Packed down, token programs, grouped prefill and prefill32 |
| Token program | V19 KV copy, split8, baseline FFN | Both packed projections, active polling and prefill32 |

Historical timings from these different profiles cannot be added together or
used to rank today's candidates. Whole-token single-publication submission is
still a source-only proposal and is not part of this comparison.

## Frozen Inputs

The candidate is the retained V12 controller, with no inference-source change:

- Controller SHA-256: `b774ce96949e29926ba39a15c941deeb8996c02ce2f79f637e776a8d2c638087`.
- Worker SHA-256: `c0dd08086f3e954f76ac97c5b08438d887d0421db1baff0716b32f7e5607ab9b`.
- Worker source: `fe0b3520811536a82cf14af8973ba992454bfd1b`, not relabeled as current fe2o3 main.
- Native plan: `e77362ff09e39378161b58c628ad0539a888f027599a61a0a193e4216301b0f1`.
- Native stage: `/tmp/ferric-opt-v5-v25.down12d005a1` on mi350.
- Cached vLLM image: `vllm/vllm-openai-rocm@sha256:e0a3b2bd3fe7ec563916c3a5d949898d133458c18d6b2f460c906885cfb32032`.

The remote stage revalidates the complete V12 CPU/build/harness binding before
launch. Nine images and the model are reused; there is no local build, image
pull or new model download. The initial all-UID KFD check is empty, and native
admission passes the unchanged resource and device-identity checks.

## Measurement Plan

First complete the previously interrupted native ABBA: four fresh starts, one
excluded warmup and three measured 128/128 requests per start, all 2,048 IDs and
streamed/final bytes exact, with clean inner and outer completion. Preserve any
failure as a separate attempt. No timing from October 3 is pooled into this run.

Then compare the candidate and vLLM through the same HTTP client on the same
physical MI350 GPU, sequentially: Qwen3-8B, TP1/C1, context 8,192, greedy fixed
128-input/128-output requests, prefix caching and speculation off. Use the
explicit FP32 output-head policy in both engines, not a stock BF16-head claim.
Each engine has ten excluded warmups, thirty measured requests and two untimed
output diagnostics. Setup is excluded from request timing and reported separately.

The HTTP harness successor preserves the existing client and serving adapter.
It binds the exact packed-down CLI and successful native evidence and requires
all 42 Ferric outputs, exact dispatch totals, runtime baseline dtype evidence,
bounded lifecycle/interference monitoring and clean owned teardown. Its new
source must pass independent review and remote CPU tests before launch.

## Teams

| Lane | Status |
| --- | --- |
| Integration | Native ABBA and both successful HTTP cohorts complete; original failed launches preserved |
| Composition audit | Complete; no supported all-enabled profile |
| HTTP harness | 83-, 86- and 103-test successors pass remotely; cache-only R3 completes vLLM |
| Independent review | Native and both HTTP raw replays pass; exact R2/R3 plan, launch, runtime, output and cleanup evidence audited |
| Custody | Original and final five-root archives fully verified; unchanged guarded retirement removes all five stages |

## Native Results

The complete fresh ABBA passes independent replay of all 2,048 token IDs,
streamed/final bytes, raw timestamps, dispatch schedules, and clean unsignaled
inner/outer completion. The all-UID KFD endpoint check is empty.

| Arm | TTFT (ms) | TPOT (ms) | Finite Output Rate (tokens/s) |
| --- | ---: | ---: | ---: |
| Baseline AB | 866.822 | 74.948 | 12.325 |
| Packed-down AB | 894.724 | 64.813 | 14.025 |
| Packed-down BA | 940.835 | 74.640 | 12.283 |
| Baseline BA | 875.447 | 71.922 | 12.788 |

**The gain does not repeat across order.** Packed-down TPOT improves 13.52% in
AB and regresses 3.78% in BA. TTFT regresses 3.22% and 7.47%, respectively.
No slow request is discarded. This does not support default promotion or a
repeatable full-model gain. The profile remains an explicit experiment.

The complete native archive is retained locally at
`products/perf-v13-native-20261005/results.tar.gz`, SHA-256
`4ae04598c10fe10ee68cc86a4fbdd9bd45d3873aaf8749909028eed223eff8bc`.
Full verification covers 476 files and 91,525,500 bytes. The independent audit
is `products/perf-v13-native-20261005/INDEPENDENT_RAW_AUDIT.md`.

## HTTP Qualification

The first source successor passes all 83 selected tests remotely under the
unchanged G28 CPU limits, with status 0, clean teardown and no signals. Source
archive SHA-256 is `20094667ea1f7ae7955cd648e29c0dbbaa816da6ef481dc74eea4ebff7cf4856`.
Its HTTP attempt at `/tmp/ferric-packed-down-http.13d005a1/ferric-a001` stops
before readiness or measurement because the monitor rejects the host's ordinary
`fuser` mapped-access marker, `m`. The controller exits with status -15 after
requested TERM/KILL; no closed receipt exists. Owned group, workers and threads
are gone, and final KFD is empty. This is a failed attempt, not clean model
qualification or a latency sample.

A separately retained correction accepts only the observed bounded marker
grammar, continues to reject warning/error output, and sets umask 077 before
creating run artifacts. Inference binaries, images, workload, HTTP client,
adapter and baseline precision observer are unchanged. Independent review and
all 86 remote CPU tests pass with clean unsignaled completion. Corrected source
archive SHA-256 is `ab041d4dd0d8fb9bc93e1d3714c013da061517a1726e91fefa2a7a5988313b6a`;
runner SHA-256 is `7cfd9dc5a7ad3bc13120f94497495d372b3f667cda78f125660f934ce1f57c52`.
The corrected HTTP plan is
`2a8c8338145ae47faf6b94b2b008f4e7f2996d83e1a8c2e3be3820a9c9560070`,
staged separately at `/tmp/ferric-packed-down-http.13d005a2`.

## HTTP Comparison

The corrected Ferric run completes with status 0, no errors, all 42 final
outputs exact (5,376 token IDs and bytes), 157 accepted lifecycle samples and
clean unsignaled controller/worker completion. An independent raw replay agrees:

| Engine | Mean TTFT (ms) | Mean TPOT (ms) | Finite Output Rate (tokens/s) |
| --- | ---: | ---: | ---: |
| Ferric packed-down R1 | 827.434 | 59.780 | 15.201 |
| vLLM 0.28.0, fresh same-day run | 19.671 | 4.245 | 228.823 |

The measured cohort contains thirty requests and 3,840 output tokens over
252.610278 seconds; ten warmups are excluded. TTFT p50/p99 are 809.128/999.239 ms
and TPOT p50/p99 are 58.873/72.300 ms. vLLM has the same thirty measured
requests and 3,840 output tokens, with its own ten excluded warmups. All measured
requests succeed. Ferric has **42.06x higher TTFT**, **14.08x higher TPOT** and
**15.05x lower finite output rate** in these cohorts. We are not faster than vLLM.
These are streaming HTTP measurements, not calibrated token ITL or sustained
loaded throughput. Independent raw replay reproduces both engines' arithmetic,
all forty vendor timing streams, both vendor ID diagnostics, all 42 Ferric
finals and the exact configuration differences. The audit is
`products/perf-v13-http-live-audit-20261005/INDEPENDENT_MATCHED_HTTP_AUDIT.md`.

The first vLLM attempt, in the corrected root's `vllm-a001`, stops during startup
when root free space crosses the unchanged 64 GiB floor. No request timing is
available. Its outer status is 1, shutdown is not clean, and timing admission
is false. The owned container and cache are removed and final KFD is idle;
this later cleanup does not repair the failed attempt. The failed receipts are
retained. A separate successor moves only the bounded disposable cache to
existing shared memory, preserving both the root and output-filesystem 64 GiB
floors, host available-memory 128 GiB floor, 8 GiB/20,000-entry cache cap,
64 GiB container memory cap and 8 GiB writable-layer cap. It adds a 32 GiB
shared-memory floor and exact owned-cache lifecycle checks without remounting
the shared filesystem. Independent source review and all 103 remote CPU tests
pass in 3.433 seconds with clean unsignaled G28 completion.

R3 source archive: `9713f49a6e2975b2a223d6571c6a8728b209a37ec0f713ee0988c22049a5c1cd`.
R3 driver: `c30df25017fb9070db02851d1c07e4e5e05c17bde640ff04d6fc45574c335f1a`.
Vendor plan: `5844566a55552ea0f997ca581c4f8b33b47ed5c01aa4a796f13adef0f0565823`.
The fresh run at `/tmp/ferric-packed-down-http.13d005a3/vllm-a001` completes
with status 0, both ID diagnostics exact, no errors, clean container shutdown
without an internal force kill, accepted lifecycle monitoring and final idle
GPU/KFD checks. The exact container and private shared-memory cache are removed.

Independent plan comparison accepts exactly four changed leaves: three
source/input paths relocate from a2 to a3 and the reviewed driver hash changes.
All other values, types, array lengths and object keys match. Both launch
commands have 109 arguments; only the unique name/label, disposable cache bind
source, observer mount and evidence mount differ. No arbitrary field is ignored.
Model, precision, images, inference flags, environment and GPU settings agree.

The vendor's 33 retained `fuser` snapshots are all empty. They do not positively
attribute vendor GPU users or establish continuous isolation across container
namespaces. The native/Ferric observations and idle endpoints also do not prove
absence of interference between samples. This is one sequential finite cohort
per engine, not repeated-start statistical qualification or a TP8 comparison.

## Custody And Publication

Original five-root custody is fully verified locally: 1,500 files and
101,692,681 bytes, with identical before/after inventories. Archive
`products/perf-v13-comparison-pre-20261005/results.tar.gz` has SHA-256
`de1d826c1d70ae70277ad4fa3c1e9a596836d77e82f3c48dfd57be0cd2f4e1ce`;
custody SHA-256 is `60e4036d9f1d734aaed75a72e35a2868ca796cf99778cf134a555d2cd7c65f37`.
Failed launches remain failed, byte-identical evidence. The first restrictive
mode operation stops on whole-process-census churn before changing any file.
Its status 125 and empty change list are retained. One literal-output-only retry
passes the unchanged all-five-root census and tightens 137 actual result files
from 0664 to 0600 without changing bytes. The exact a2 fixture directory is
separately tightened from 0755 to 0700 under the native lock, with pinned inode,
metadata and unchanged 22-byte child. Original metadata and transition evidence
remain retained; no failed benchmark is reclassified.

Final complete custody also verifies 1,500 files and 101,692,681 bytes:
`products/perf-v13-comparison-final-20261005/results.tar.gz`, SHA-256
`36b61b23b5e59bc0a4c2095432041890aa618d2aeafe86e50908a84fdc53b60a`;
custody SHA-256 `4dac16340d9d23a4cd41f8914675e890f94748953de8c138714933c78008becc`.
Actual generated cleanup controls reverse-hash to the unchanged original strict
guard and wrapper. Their bound control roster is
`da69b9309c31a4a16e2da377c463c0e7bc69e0498c6bcf65fcc1162e3e675638`.
Guarded retirement completes with status 0 and `refused: false`, removing all
five exact native/input/HTTP roots. A separate endpoint check confirms all five
are absent, the private cache is absent and KFD census is empty. No owned
benchmark process remains. Cleanup results are retained under
`products/perf-v13-retirement-result-20261005`.
The three now-redundant remote archive copies are separately removed after
full local verification, exact remote hash/ownership checks and an empty
privileged open-file census, releasing 125,024,273 logical bytes. Small remote
audit sidecars remain; the complete archives are retained locally.

No inference-source change, project build, commit or push is performed by this comparison.
All CPU tests run remotely on mi300x and all native/HTTP GPU runs on mi350.
The retained worker is not relabeled as latest fe2o3 main. No new worktree is
created, and unrelated campaigns and shared model files are untouched.

The September 28 result remains a separate historical profile. No old sample
is pooled into today's comparison, and the mixed packed-down ABBA does not
support default promotion or adding historical optimization gains together.
