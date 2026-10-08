import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { chromium } from "playwright";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const pageUrl = pathToFileURL(join(siteRoot, "index.html")).href;
const screenshotRoot = process.env.FERRIC_SCREENSHOT_DIR;
const compactScreenshots = process.env.FERRIC_COMPACT_SCREENSHOTS === "1";
const clientTokenCaptures = [];
const viewports = [
  ["desktop", 1440, 1100],
  ["validation-edge-1027", 1027, 900],
  ["validation-edge-981", 981, 900],
  ["sweep-proof-800", 800, 900],
  ["header-edge-710", 710, 844],
  ["header-edge-701", 701, 844],
  ["mobile", 390, 844],
  ["narrow", 320, 720],
];
const dynamicRoots = [
  "[data-readiness]",
  "[data-resident-progress]",
  "[data-performance]",
  "[data-attention-progress]",
  "[data-submission-progress]",
  "[data-live-http-progress]",
  "[data-live-http-teams]",
  "[data-residual-diagnostic]",
  "[data-c1-checkpoint]",
  "[data-live-c1-checkpoint]",
  "[data-envelope]",
  "[data-capabilities]",
  "[data-validation]",
  "[data-transitions]",
  "[data-teams]",
  "[data-boundaries]",
  "[data-observation]",
  "[data-tp-observations]",
  "[data-progress]",
  "[data-gates]",
];
const requiredClaims = [
  "Updated 2026-10-06",
  "Width55c same-plan matched HTTP comparison is complete",
  "median TTFT 431.033 ms and TPOT 53.119 ms, versus vLLM 0.28.0 at 18.969 ms and 4.352 ms",
  "Finite output rates are 17.902 and 223.768 tokens/s",
  "22.72 times slower on median TTFT and 12.20 times slower on median TPOT",
  "5,376 output token IDs",
  "Historical R6 results are not pooled",
  "does not isolate a width speedup or establish a reliable decode gain",
  "dd84f4503d4fbef3a0fd0eee2c7025a1bb910eac4fe81fd7f09bbb13a6d7b134",
  "R6 same-plan matched HTTP comparison is complete",
  "median TTFT 783.767 ms and TPOT 53.400 ms, versus vLLM 0.28.0 at 19.322 ms and 4.397 ms",
  "Finite output rates are 16.932 and 221.380 tokens/s",
  "30 measured requests after 10 excluded warmups and two untimed diagnostics",
  "Ferric then vLLM, not cross-engine ABBA",
  "vendor token IDs are checked only in two untimed diagnostics",
  "finite observations, not continuous isolation",
  "clock, power, temperature and throttle equivalence is not established",
  "The observer repair is not a speedup",
  "Ferric is not faster than vLLM",
  "completed V17 grouped-prefill a004 lowers median native TTFT by 10.9687%",
  "from 868.156454 to 772.931099 ms",
  "Median TPOT is essentially unchanged: 53.210963 to 53.192764 ms, a 0.0342% improvement",
  "All six adjacent pairs improve TTFT",
  "within-native prefill scheduling gain only, not a vendor win, ordered64 comparison, default promotion or formal proof",
  "All 14 V17 cells and guarded raw replay pass",
  "24 measured requests per arm after two excluded warmups per latency cell",
  "all 9,472 output token IDs and streamed bytes match the independent reference",
  "613-command program with 216 dynamic slots",
  "no cached ABI or fence composition",
  "The separate R6 matched HTTP result above does not change this within-native attribution",
  "completed V16 native fence comparison is 1.0693% lower median TPOT",
  "below its fixed 5% promotion gate",
  "median TPOT 53.734052 / 53.159463 ms and TTFT 877.497994 / 871.949030 ms",
  "no aggregate TTFT gain is admitted",
  "native TTFT/TPOT remain unmeasured",
  "host completion wait averages 49.443990 ms inside a 50.040968 ms Execute round trip",
  "Completion wait is not GPU-only time",
  "residual is not a between-token engine-overhead budget",
  "mean TTFT 827.434 ms and TPOT 59.780 ms",
  "vLLM 0.28.0 at 19.671 ms and 4.245 ms",
  "historical V13 matched HTTP comparison remains separate",
  "same frozen eded9474b worker and kernel images",
  "median TTFT 820.787174 / 872.554286 ms and median TPOT 50.134290 / 53.245459 ms",
  "All 14 cells complete cleanly with independent raw replay",
  "Only two of six adjacent pairs improve",
  "V14 remains default-off",
  "Earlier failed attempts remain retained; none is relabeled as a pass",
  "Retained historical V4 checkpoint (September 29, 2026)",
  "September 29 engineering checkpoint",
  "Fixed-safe gate/up: no useful measured gain",
  "September 26 engineering checkpoint",
  "September 23 native prefill screen",
  "September 22 engineering checkpoint",
  "Authenticated resident serving",
  "Ferric dc75d928 adopts fe2o3 a167 through the exact 80-file source transition",
  "50 phases, 48 source-gate tests and 32 locked graphs pass",
  "all 1,172 source files match the audited census",
  "fresh b700 host-residual release controller",
  "479 passed, 12 ignored and zero failed",
  "selected mi350 physical GPU2",
  "Each five-input-token arm emits [12095, 13] and text ' Paris.'",
  "pair checker exits zero and both independent final custody audits pass",
  "a167 adoption produced no new a167 GPU product",
  "no full numerical or tensor parity, performance comparability, TP8, gfx942, serving or protected M1 qualification",
  "Latest Completed Same-Source Pair: Width55c",
  "not a completed same-source-plan three-arm comparison",
  "No competitive improvement is declared",
  "Ferric 7544c3e4 implements fresh 1..8-member S8/T128 prefill",
  "781 tests with nine ignored",
  "52 pending-Verus bodies, not proofs",
  "affb1d48 passes 363 library, 24 CLI, 63 batch, 36 comparator and 27 reference-parser CPU tests",
  "e336ac3a passes 11 CPU tests and admission of the actual historical evidence",
  "without a fresh reference execution",
  "compiler 46d189e5; adoption of upstream 930342d9 is pending",
  "15:45:35 UTC observation after reboot reports 13 minutes uptime",
  "only /dev/dri/card0, and no amdgpu, /dev/kfd or KFD topology",
  "neither observation is a reservation or launch admission",
  "no new native tensor comparison, TP8, performance, serving or protected qualification result",
  "Historical a167 source and GPU2 host-residual pair",
  "Ferric dc75d928, tree e93d046c, adopts fe2o3 a167 through the exact 80-file source transition",
  "Both five-input-token arms use the same controller and retained 6f-attributed debug worker, TP14 artifact and canonical model",
  "both emit token IDs [12095, 13] and text ' Paris.'",
  "Both workers close, owned process groups are absent and the selected-device idle baseline is restored",
  "4b42f42066739cfb1cbcd5dc11c5179890afbea4c6ddb17ecc6c1f8be09b4abb",
  "135291cbba170244136530d675554e4cc8a31150f0f5ecc099d82dfbba173119",
  "f60e3b1f7ba0c41ecdede2ea99459e79d019325312c52db3265c65030e27b694",
  "The retained checkpoints below preserve their original dates, source identities, evidence and then-current limitations.",
  "mi350: eight checked identities, no kernel dispatch",
  "actual mi300x-built fe406 identity executable passes --all on eight distinct physical gfx950 UIDs",
  "new native identity event; the earlier e22 Asrock event is not relabeled",
  "original MI350 profile 6f859b0a67f8ee2497393206930ff35bf1a9ae69d33f172106a790a0c9226667",
  "eight process apertures and contracted-clear currentness",
  "All eight GPUs remain at zero utilization and 297,766,912 bytes VRAM each",
  "visible-user observations, not a reservation or general teardown proof",
  "does not acquire explicit VM authority, allocate GPU memory, create queues, dispatch kernels or set XNACK",
  "38 payloads and the exact 40-member archive, including the result directory and ledger",
  "8c5c2d0300801d122ecd6879bdca301d752517ae9dc18b6ab0fb1ee8602bc6cb, 633,615 bytes",
  "sole completed probe owner is removed with exit zero, reclaiming 2,960 KiB",
  "No shared job, model, other stage or worktree is changed",
  "glibc 2.39 removes the known Asrock controller-link mismatch but does not replace full loader validation",
  "old /tmp/ferric-qwen8.TRNKht runtime/model bundle is absent",
  "eight-device identity is not TP8 inference or gfx942 evidence",
  "mi350: model transport passed, authentication pending",
  "13 regular files totaling 17,907,716,208 bytes",
  "single-linked and read-only mode 0400",
  "33 source, 35 destination and 36 coordinator payloads",
  "transport success, NOT CONTENT AUTHENTICATED",
  "matching names and sizes are not content proof",
  "STATE remains unchanged: incomplete; never use as a model source",
  "No model intake, GPU open, Qwen inference, TP8 dispatch or timing result is claimed",
  "Fresh fourteen-root gfx950 emission and model content authentication remain prerequisites",
  "mi350: passive loader checks passed",
  "controller-readelf, controller-verify, controller-list, worker-readelf, worker-verify and worker-list, with outer exit zero",
  "original 59822dd4 controller a8e7961e and fe406 worker 47426e4f",
  "15,272-byte archive at SHA-256 536d7892001441c0ea6c56109becf9594aab3e81b78d08afe0e6b5ef5af42c77 retains 97 hash-checked records",
  "neither ELF entrypoint is invoked, and no GPU or model is opened",
  "not the Asrock-private relink, Qwen execution, kernel dispatch, TP8 validation or a timing result",
  "Compiler fe406: four-tool custody complete",
  "fresh fe406 four-tool compiler campaign on mi300x finishes all six phases with zero status",
  "82,422,000-byte archive at SHA-256 9853c1dc1822b4e57a3245e2a17f75c9cb8c122a1405a1d0c4c2a811190177ef",
  "155 payloads, 156 members and the separate 37 external terminal records",
  "not six tools, fresh kernel emission, GPU dispatch, Qwen execution or protected qualification",
  "fe406-produced binaries use production source and build inputs unchanged at e364",
  "They were not built from e364, no hypothetical rebuild byte equality is claimed",
  "whole-repository or tutorial-test equivalence is not established",
  "Ferric still pins fe406; this source-only comparison is not a new dependency adoption",
  "tested Ferric controller source remains 59822dd4",
  "TP14 prerequisites: source-gate and metadata passed",
  "Ferric 59822dd4 / compiler fe406 passes all 17 mi300x phases and 48 gate tests, with two actual fresh producers",
  "163 packages: 53 fe406 and two Pliron packages",
  "All 1,157 source files are unchanged",
  "four generated TCB inventories and normal coverage remain byte-identical, with no proof promotion",
  "5,311,957-byte archive at SHA-256 a57181029ad0a9c7ac672311ffb6c07580832094c0bc377ab881598b2191e184",
  "300 payloads, 301 members and 20 external terminal records",
  "Retention R1 remains exit 1 from a deployment directory-mode mismatch before archive creation, not a failed source-gate campaign",
  "separate fresh R2 retention passes with exit zero; the original failure is preserved",
  "not a current fourteen-kernel artifact, compiler emission, native execution or qualification",
  "Fresh fourteen-root gfx950 emission remains pending",
  "earlier Asrock identity and TP1 portability checkpoints retain their original host and source attribution",
  "mi350 eight-device identity source",
  "mi350 identity ELF SHA-256",
  "mi350 original profile SHA-256",
  "mi350 eight-device identity evidence SHA-256",
  "mi350 identity payload ledger SHA-256",
  "mi350 model source transfer evidence SHA-256",
  "mi350 model destination transfer evidence SHA-256",
  "mi350 model coordinator evidence SHA-256",
  "mi350 passive loader evidence SHA-256",
  "Compiler fe406 tool evidence SHA-256",
  "TP14 source-gate evidence SHA-256",
  "TP1 controller: CPU checks and static portability passed",
  "Ferric 59822dd4, tree 9020cf3c, aligns TP compiler markers, the contract and exact adapter admission with all fourteen emitted roots at compiler fe406b0c",
  "imported MFMA root is admission metadata only, not a new dispatch selection; kernel arithmetic, default targets and protected routes are unchanged",
  "71 focused tests pass: 34 gfx950 and 34 gfx942 device tests, one default adapter and two TP-enabled adapter admission regressions",
  "not the complete adapter suite",
  "17 phase records preserve one R2 adapter-format failure and its successful R3 correction; the other 16 phases exit zero",
  "Actual Cargo records identify 14 retained test executables",
  "Three controller producers remain distinct: original source 289288fa",
  "fixed source 59822dd4 at a8e7961e825a3b3a393f00041814c0e6f7ad401e38d2cf719fe866fe9a6febe3",
  "same fixed source with private mi350-2 link inputs at cbcceea01f035515b886754ebedbc7c832450afc22fc9b791876ec182932d264",
  "ordinary link requires GLIBC_2.39, unavailable on the destination's glibc 2.35",
  "20 captured installed files totaling 7,631,367 bytes lowers the maximum required GLIBC version to 2.34",
  "No shared library, Rust dependency or repository source is changed, and no ELF is patched",
  "648 linker trace entries: 10 private sysroot, 609 owned target and 29 pinned Rust-standard-library inputs",
  "static compatibility only; neither the controller nor a GPU kernel is executed",
  "97,572,588-byte archive at SHA-256 13da490f7aef70c7ce1d495d682d239f932b372f9926cade57033e53e1f569b5, with 362 payloads, 367 unique regular members and five matching embedded metadata files",
  "Timestamp precision is not independently validated",
  "cleanup R2 exits zero and removes only two completed owned test stages totaling 3,099,044 KiB",
  "active build/cache owners, canonical models and shared jobs remain untouched",
  "R1 performed syntax preflight only, with no deletion",
  "All 99 external retention/cleanup records match fresh remote hashes",
  "Limited process visibility remains explicit; no global no-use claim is made",
  "Neither inspected local worktree registry had an eligible finished tree, so none was deleted",
  "Current fourteen-root gfx950 emission, model content authentication and the TP1 Qwen smoke remain pending",
  "prior e22d22 identity bind keeps its original attribution",
  "One gfx950 does not replace gfx942 capture or eight-GPU evidence",
  "No new Qwen output, TTFT/TPOT, protected proof or M1 gate closure is claimed; all 33 gates remain open",
  "Compiler fe406: dependency adoption validated",
  "Ferric 289288fa, tree 41028757, integrates published fe2o3 fe406b0c through the exact 80-file transition after all 50 main mi300x phases and four separate preflights pass",
  "32 locked graphs, 35 metadata records, 48 fresh source-gate tests and four TCB inventories",
  "Two upstream lowerer DEV declarations are added in the DEV and combined inventories; no other non-revision inventory delta is admitted",
  "Coverage, the runtime TCB, property binder and Pliron remain unchanged",
  "All 1,157 committed source files byte-match the validated final census at SHA-256 5c9c6a80ef544b2cafcbb0a19a50fe38d55201638309c01989e82d7bb1a2aee2",
  "24,639,559-byte archive at SHA-256 fc371ab77b306505f30c9008ad77ed534d5439102ce5b978c736b47d4e6f5deb, with 1,981 payloads and 1,982 members",
  "dependency, graph and source-policy adoption, not fresh engine validation, compiler emission, native execution, Verus or performance qualification",
  "runtime CPU/ABI validation remains at fe406; the checked mi350-2 native bind remains at e22d22 with an explicit identical-ELF bridge",
  "BF16 wrapper proof remains at cdc60712 / compiler 9b64, and the gfx942 emission plus three capture attempts remain at 7f1121f1 / 9b64",
  "No older result is relabeled as new Ferric native or numerical evidence",
  "Asrock runtime: published with fresh CPU and ABI checks",
  "Published fe2o3 fe406b0c, tree 0797c61b, rebases the six-file Asrock runtime port onto main 8cbd2a94",
  "Exact feature-gated platform admission and strict KFD/PCI UID equality remain required; no XCP exception is added",
  "422 default KFD tests and 490 engineering KFD tests, each with one existing ignored test",
  "64 UAPI integration tests, 27 default and 31 engineering compile-fail documentation tests, both strict Clippy selections and formatting",
  "Three fresh C probes built against 18 captured Asrock LP64 UAPI headers reproduce all 16 KFD, 51 event and four DRM output lines",
  "ABI layout/request checks, not live driver semantics or protected attestation; C compiler shared-library closure is not claimed",
  "25,208,962-byte archive at SHA-256 15a5dca4042f5d29d2347bed68361485140e3c97a69b1eb68b2c62e32d12d387, with 340 payloads and 341 members",
  "retained 18 phases include an initial pre-child supervisor-limit failure and successful successors",
  "original failure is not relabeled and no phase cap was relaxed",
  "earlier native event remains attributed to e22d22; byte identity is not a new GPU run",
  "fe406b0c fast-forwards main from 8cbd2a94, the remote tip is confirmed and the Actions query reports zero runs",
  "not unrelated upstream simulation/lowerer work, Qwen inference, gfx942, protected qualification or performance",
  "mi350-2: checked identity bind, no kernel dispatch",
  "At 17:32:55Z on September 17, the mi300x-built e22d22 identity executable completes a bounded native device bind on mi350-2",
  "explicitly selecting gfx950 UID 1956390207832604050",
  "Command, wrapper and cleanup exit zero",
  "Currentness checks pass against exact profile cf65a0d24a127c3a7fb12b8aefb1668fe75cb779036562a0dde2d00f52ce2978",
  "one process aperture and descriptor counts of five before and five after",
  "VRAM remains 297,750,528 bytes and final utilization is zero; this is not an exclusive reservation or a general teardown proof",
  "opens KFD/render devices and subscribes to reset events",
  "does not explicitly acquire a VM, allocate GPU memory, create a queue, dispatch kernels or set XNACK mode",
  "All 30 payloads in the 631,238-byte archive are verified at SHA-256 7ec8ef6ee8a02626f90b926189a8d85358611c97d0278567be6b7f4d67034adb",
  "later fe406 rebuild's identical ELF is an explicit byte-identity bridge, not a rerun or a changed source attribution",
  "one gfx950 identity observation, not gfx942 capture, TP8 validation, Qwen inference, numerical parity or a timing result",
  "Earlier compiler 998: dependency adoption validated",
  "Ferric c54ac0b0, tree cb435261, adopts fe2o3 99897145 through the exact 80-file pin transition",
  "All 50 bounded mi300x dependency/source-policy phases pass, including 32 locked graphs, 48 fresh source-gate tests and four TCB derivations",
  "Coverage and the runtime TCB remain unchanged",
  "24,607,654-byte archive at SHA-256 5212acb34f16f448d367903a8bb5a963fe32d399b73ccad728168f32ddc57570, with 1,964 payloads and 1,965 members",
  "validates pins, dependency graphs and source policy only",
  "does not validate new matching compiler tools or emission, native execution, Verus proofs, numerical parity, serving or performance",
  "BF16 wrapper proof remains attributed to cdc60712 with compiler 9b64",
  "retained emission and all three native attempts remain at 7f1121f1 / 9b64",
  "Neither is relabeled as 998 evidence",
  "Protected qualification remains unavailable and all 33 M1 gates remain open",
  "Final-RMS R3: timeout after KFD binding",
  "Native R3 runs on mi300x GPU4 from 16:00:40Z to 16:20:43Z on September 17",
  "Campaign and native process both exit 124 after the unchanged 1200-second command limit",
  "timeout, not the shared-GPU guard abort observed in R1 and R2",
  "Startup reaches kfd-bind, with no later marker; the last marker does not identify the cause",
  "no capture directory, capture-pass record or stdout output, and no independent reference is launched",
  "4,524,837-byte failure archive at SHA-256 e9dba42ec89a7397e63d057ba4fc55f44b40462beca2a3f9871088499cf58db6, with 52 payloads and 53 members",
  "Retention success does not change the failed capture result",
  "source-only initialization audit identifies 36 GiB target plus 28 GiB draft logical zero-KV hashing, duplicate weight-image SHA-256",
  "static logical work counts, not measured traffic, time attribution or a demonstrated timeout cause",
  "No measured speedup, numerical comparison, serving or TTFT/TPOT result is claimed",
  "earlier two guarded aborts remain retained separately, and no automatic retry is launched",
  "BF16 observer wrapper: verified and integrated",
  "Candidate cdc60712, tree dce6a22d, changes only the shared spec export and production qualification-logits wrapper",
  "Ordinary pinned Verus with --no-cheating verifies the actual lowest_id_finite_bf16_argmax body: one verified condition, zero errors, not an entire-crate proof",
  "lowest-ID maximum when the entire BF16 row is finite, exact lane and byte extents for a malformed row, and the first nonfinite token with its original lane",
  "All 15 qualification-logits host tests pass, including the two new diagnostic cases",
  "Seven actual-body mutations produce genuine proof failures under unchanged contracts",
  "Six reject at the default solver limit; the Ok-token mutation rejects in a separate R2 at rlimit 50",
  "original R1 remains inconclusive, not a passing negative",
  "first positive invocation's missing-rustup failure is also preserved separately from the successful invocation",
  "Both strict Clippy selections, 10 focused spec tests and 48 source-gate tests pass",
  "Normal coverage validation passes with exactly one candidate body classification changed",
  "Independent review accepts the retained 48,258,102-byte validation archive, with 647 payloads and 648 members",
  "2199bd5fa87b87286802282bb86168ece6229878a5893d1eec65c49992a92048",
  "Ferric a6e0be41, tree f5b89fcc, integrates the two validated Rust files, the admission removal and exactly one generated coverage-row change",
  "All four integrated files are byte-identical to coverage candidate 4f2d071e",
  "Proof and tests remain attributed to cdc60712; no integrated-source rerun is claimed",
  "selected wrapper proof does not prove surrounding allocation, readback, GPU arithmetic, protected runtime or numerical parity",
  "Earlier compiler 9b64: 13-kernel emission passed",
  "Ferric 7f1121f1 adopts published compiler 9b64a2da after all 50 dependency/source-policy phases pass",
  "24+3-phase host campaign with 70 tests, and fresh 198-package vendor formation are separately validated",
  "matching CPU-only emission R2 passes all 13 phases on mi300x",
  "13 gfx942:xnack- kernel entries and descriptors, code-object version 6, exact output replay and actual MFMA instruction evidence",
  "HSACO is 113,192 bytes",
  "b73a25170b87a61710c04f0d0416822112e0525cdc1d6e71b632366ec9a956d7",
  "1,924,709-byte emission archive's 503 payloads, 505 members and 22 external records, with exact signed-nanosecond metadata",
  "earlier R1 remains failed: a stage-size observation raced with scratch cleanup, leaving controller125/wrapper143/command0 and no inspection",
  "LLVM worker retains its original producer216822/tree613ef attribution",
  "New upstream main 7581df51 has a source-only audit of one compiler commit; runtime, KFD and platform code are unchanged",
  "has not replaced adopted 9b64 or been validated by these matching-tool, host, emission or guarded-native records",
  "No GPU is launched by emission; publication, load and launch grants remain false",
  "Earlier final-RMS R1/R2: two guarded aborts",
  "Both stop during CPU model bootstrap when foreign shared-GPU tests appear",
  "Both campaign exits are 126 and owned command exits are 143",
  "Only owned groups are terminated and strictly confirmed absent; no foreign job or shared library is modified",
  "Neither attempt reaches Ferric's KFD-bind phase or produces a five-file capture, capture-pass record or numerical comparison",
  "No independent reference is launched",
  "R1 has 56 payloads/57 members and R2 has 57 payloads/58 members",
  "Custody success does not change either failed native result",
  "another attempt requires an uninterrupted shared-GPU window, not relaxed guards",
  "Earlier mi350-2 survey: platform compatibility required",
  "kernel 5.15.160+ differs from the 6.8.0-124-generic profile checked at fe2o3 9b64",
  "amdgpu module is 6.16.15 / 9462451703604FCD7EC2365 instead of admitted 6.16.13 / 703B1127E578BC5D4BD6615",
  "engineering worker requires this platform check too; machine-code opt-in does not bypass it",
  "separately identified and validated fe2o3 platform profile, not a relaxed check or host modification",
  "surveyed caches also lack the canonical Qwen3 target/draft pair",
  "cannot substitute for the pending gfx942 final-RMS capture or close its hardware-specific M1 gate",
  "No workload is launched on mi350-2",
  "TP1 roster and controller source",
  "TP1 roster and controller tree",
  "TP1 original controller SHA-256",
  "TP1 fixed-source controller SHA-256",
  "TP1 portable controller SHA-256",
  "TP1 private link-input archive SHA-256",
  "TP1 build and portability evidence SHA-256",
  "BF16 wrapper proof/test source",
  "BF16 wrapper proof/test tree",
  "BF16 wrapper integration source",
  "BF16 wrapper integration tree",
  "BF16 wrapper validation archive SHA-256",
  "BF16 wrapper positive transcript SHA-256",
  "Published Asrock runtime source",
  "Published Asrock runtime tree",
  "Asrock CPU and ABI evidence SHA-256",
  "mi350-2 identity observation source",
  "mi350-2 identity ELF SHA-256",
  "mi350-2 selected profile SHA-256",
  "mi350-2 identity evidence SHA-256",
  "Current integrated compiler",
  "Compiler fe406 adoption source",
  "Compiler fe406 adoption tree",
  "Compiler fe406 final source census SHA-256",
  "Compiler fe406 adoption evidence SHA-256",
  "Compiler 998 adoption source",
  "Compiler 998 adoption tree",
  "Compiler 998 adoption evidence SHA-256",
  "Native R3 timeout source",
  "Native R3 timeout evidence SHA-256",
  "Matching 9b64 emission source",
  "Matching 9b64 HSACO SHA-256",
  "Matching 9b64 emission evidence SHA-256",
  "Guarded native R1 failure evidence SHA-256",
  "Guarded native R2 failure evidence SHA-256",
  "The wrapper's integration does not relabel earlier host, native or performance results",
  "BF16 host scanner: both selected bodies verified",
  "At exact Ferric 039018f0 and adopted compiler f7f, host R1 terminates 0 after all 30 phases pass",
  "48 source-gate, 136 spec and 760 engine tests with nine ignored",
  "formatting, metadata, three strict Clippy selections, normal source coverage and source/input preservation",
  "All three Rust source payloads match 039018f0",
  "generated coverage adds exactly one scanner row",
  "retained 31,481,979-byte host archive at SHA-256 138fb2084bbf3702f01cc40a89691089c27031b9f91e2360a68b815bbb0dbd70 has independently verified custody over 281 payloads, 283 members and 14 external records from four genuine fresh producers",
  "Ferric aec2fa54, tree 69666667, integrates the exact three validated Rust source files plus the actual generated coverage row",
  "Before coverage promotion, all 1,157 source hashes match 039018f0",
  "coverage record is SHA-256 a633fd15b14c05a843bd9a3836992796d57d75b37d37a9a866adacaf2628a392",
  "host suite ran at 039018f0 and is not relabeled as a rerun at aec2fa54",
  "earlier 7116b056 / compiler bd1 host artifact retains its own identity and is not relabeled",
  "At the same exact 039018f0 source, ordinary Verus R4 passes all 21 phases with --no-cheating",
  "One integer-key body and one executable full-vocabulary scanner body verify",
  "scanner accounts for two positive verification conditions",
  "All six scanner body mutations fail without resource exhaustion under unchanged signatures, specifications, postconditions, invariants and decreases",
  "selected-body evidence, not proof of the engine wrapper, device numerics or protected runtime",
  "R4 proof archive's 249 payloads, 251 members and 12 external records",
  "Proof R1's checker-count failure, R2's skip-last exhaustion and R3's nonfinite exhaustion remain failed campaigns with retained custody",
  "historical GPU logit mismatch remains unresolved, with no numerical acceptance, performance claim or M1 gate closure",
  "Completed local copies removed; evidence retained",
  "completed duplicate local extractions are removed, reclaiming 206,932 KiB",
  "earlier integrated helper worktree is removed, reclaiming 45,184 KiB",
  "clean 0390 scanner worktree is removed through normal Git worktree removal, reclaiming another 45,196 KiB",
  "post-removal outside-path survey is empty",
  "branch and the proof and host archives remain retained",
  "Original archives, verification records and earlier failure evidence remain retained",
  "Earlier compiler f7f checkpoint",
  "Compiler f7f passes all 50 bounded mi300x dependency/source-gate phases from Ferric source base 7116b056",
  "32 locked graphs and 48 fresh source-gate tests",
  "exact 80-file revision transition preserves the runtime TCB and selected-helper coverage",
  "All 1,651 payload hashes, sizes and modes, the exact 1,652-member roster of the 20,968,339-byte archive and 14 external records pass independent custody checks",
  "Independent final review finds no issue; Ferric 3836cc1f integrates the exact transition",
  "all 1,157 source identities match the validated census",
  "Newer upstream 1d0294bf, tree 1b932be1, is separately observed with a source-only archive and census",
  "delta from 6ae is confined to four lower-MIR source/test files, with no manifest, lock, toolchain, package-source or target change",
  "has not been built, vendor-formed, validated or adopted",
  "not relabeled as the validated f7f input, which was the adopted compiler at that checkpoint",
  "dependency/source-policy validation, not whole-engine or native validation",
  "BF16 host and proof results retain their original attribution",
  "User-authorized mi350-2 is reachable and exposes one gfx950 GPU, unique ID 1956390207832604050",
  "availability observation, not native admission",
  "checked model cache lacks the required canonical target/draft bundle",
  "current-source TP1 needs matching rebuilt tools, worker and genuine gfx950 TP emission",
  "frozen 6e/abd8 gfx942 final-RMS capture cannot run there unchanged",
  "no gfx942 HSACO or M1 qualification is relabeled",
  "Existing native, numerical and performance results remain unchanged",
  "BF16 exact validated host source",
  "BF16 integrated host source",
  "BF16 retained host R1 evidence SHA-256",
  "BF16 exact selected-proof source",
  "BF16 prior host R4 evidence SHA-256",
  "BF16 selected proof R4 evidence SHA-256",
  "Compiler f7f adoption source",
  "Compiler f7f validation archive SHA-256",
  "Earlier integrated compiler (f7f)",
  "Earlier source-only compiler observation",
  "Retained bd1 dependency pin",
  "Matching abd8 kernels: all 13 emission phases passed",
  "Matching abd8 compiler tools and frozen Ferric 6e2ad936 pass all 13 bounded CPU-only emission phases on mi300x",
  "Fresh canonical and private empty-home metadata agree",
  "112,936-byte gfx942:xnack- code-object-v6 ELF has exactly 13 kernel entries and descriptors",
  "includes the expected BF16 MFMA instruction and passes exact output replay",
  "All 230 payload hashes, sizes and modes, the exact 231-member archive roster and 12 external hashes are independently checked",
  "emitter retains its original 512 MiB initial reserve",
  "Original vendor formation exit 1 remains explicit",
  "Publication, load and launch grants remain false",
  "not GPU execution, numerical acceptance, protected proof qualification, performance evidence or an M1 gate closure",
  "Vendor contract unit-checked; original formation remains failed",
  "all 198 package checksums, 143 unchanged registry packages and 55 canonical Git payloads",
  "all 163 resolved packages, features, workspace and package IDs agree",
  "all 55 entire Git-package objects match after only package-relative path relocation",
  "All 29 semantic records verify, with an independent comparison of the raw metadata",
  "future manifest-contract validator separately passes 12 bounded unit cases",
  "legitimate absent Git Cargo.toml.orig, unexpected orig, post-vendor rewrites, source/package drift and locked semantic drift",
  "not a vendor formation: there is no fresh stage, producer snapshot or accepted vendor tree",
  "original abd8 formation stays failed; no normalization retry or synthetic success receipt is used",
  "eight aliases/four completed tool inodes, reclaiming 208,668 KiB (203.8 MiB)",
  "Earlier failed assessments and cleanup inventories remain retained; no visibility exception is added",
  "Compiler bd1 adopted; GPU validation held",
  "Compiler bd1 is adopted at Ferric 819ebcdb after all 50 bounded mi300x dependency/source-gate phases pass, including 32 locked graphs and 47 source-gate tests",
  "Source coverage and the runtime TCB are exactly unchanged",
  "exact 80-file transition and all 1,157 source identities match the validated census",
  "All 1,652 payload hashes, sizes and modes and the exact 1,653-member roster of the 20,958,731-byte archive are independently checked",
  "dependency/source-policy adoption, not whole-engine validation",
  "matching emission and optimized release remain frozen at 6e/abd8",
  "not relabeled as 9f8 or bd1",
  "native final-RMS candidate now binds the actual artifact, tools and release but remains UNREADY",
  "At 2026-09-17T02:22:21Z, selected GPU4 is 97% busy with 105,477,615,616 bytes of VRAM used",
  "No GPU workload is launched or interrupted",
  "The retained checkpoints below preserve their original source identities and then-pending states",
  "No new numerical, serving or TTFT/TPOT result",
  "Earlier abd8 emission tracker source",
  "Matching abd8 HSACO SHA-256",
  "Matching abd8 inspection SHA-256",
  "Matching abd8 emission evidence SHA-256",
  "Separate vendor source assessment receipt SHA-256",
  "Separate Cargo semantic receipt SHA-256",
  "Completed-tool cleanup receipt SHA-256",
  "Current integrated compiler",
  "Compiler bd1 adoption source",
  "Compiler bd1 validation archive SHA-256",
  "Compiler bd1 retention receipt SHA-256",
  "Compiler bd1 external ledger SHA-256",
  "Prefill EOS: authenticated terminal ownership, host checks passed",
  "authenticated retirement and healthy queue shutdown before any successor",
  "Owners remain retained until close; the single-token terminal owner cannot resume or clone",
  "R33 fixed-length measurements still reject short EOS completions, and continuation remains allocation-free",
  "On compiler 9f8, R2 passes 758 engine tests with nine ignored plus the isolated allocation test",
  "R3 passes seven compile-fail ownership doctests and strict engine Clippy",
  "R4 passes all ten phases, including 112 adapter tests with one ignored, 38 source-policy tests and strict adapter Clippy",
  "Source coverage adds exactly 14 pending-Verus helpers with no verified promotion",
  "Earlier R1 stale derived-Debug admissions, R2 relative-TMPDIR fixture failures and R3 test-counter Clippy naming failures remain retained separately",
  "The fix is integrated at Ferric eed0ad07",
  "R4's 8,623,993-byte archive retains 123 independently checked payloads in 124 members",
  "These CPU-only checks add no GPU result, protected proof qualification, performance measurement or M1 gate closure",
  "Prefill EOS integration source",
  "Prefill EOS R4 host evidence SHA-256",
  "Canonical prepack publication: atomic no-replace fix",
  "Ferric fee352ed fixes a canonical prepack publication race",
  "Atomic no-replace publication now preserves the competing destination",
  "test-only old-code run reproduces the overwrite",
  "corrected executable passes all 12 unit tests on mi300x with compiler abd8",
  "Strict binary Clippy, formatting and normal source-coverage inventory, generation and validation pass",
  "all 169 payloads and 19 external records are independently checked",
  "Earlier test-import, socket-path and metadata-profile failures remain separate, not counted as passes",
  "not native GPU, protected-proof or performance qualification",
  "Compiler 9f8 adopted: dependency and source checks passed",
  "Compiler 9f8ffda4 is adopted at Ferric d48ae0d5",
  "All 50 bounded mi300x dependency/source-gate phases pass",
  "32 locked graphs, 47 source-gate tests, four exact dependency inventories and normal source coverage",
  "One new upstream test target is an explicitly reviewed dependency-inventory addition",
  "compiler production code changes",
  "native runtime/KFD, device code, compiler manifests and locks, the toolchain and LLVM-worker subtree are unchanged from abd8",
  "all 1,655 payloads and 27 external records are independently checked",
  "Matching 9f8 compiler tools and device emission have not run",
  "Existing 6e2ad936 / abd8 tools and optimized diagnostic release, 2d3 host diagnostics and 7f90 native observations keep their original identities",
  "does not establish a new GPU result, numerical acceptance, protected proof qualification, performance measurement or M1 gate closure",
  "they are not 9f8 tool builds or native validation",
  "Retained abd8 diagnostic: vendor formation stopped",
  "frozen 6e2ad936 / abd8 diagnostic path attempted vendor formation once",
  "approved 384 MiB reserve and unchanged 10 GiB stage cap",
  "normalizer exited 0 and the exact 198-package roster and checksum checks passed",
  "formation exited 1 at the required Cargo.toml.orig check for dialect-amdgcn",
  "All other admission checks remained unchanged",
  "No vendor was accepted and no retry ran",
  "not a 9f8 tool, device-emission, native or performance result",
  "Final-RMS diagnostics: host and protocol checks passed",
  "169 selected Rust tests, both strict Clippy checks and normal source inventory, generation and validation",
  "19 pending-Verus functions with no removals, verified promotions, module changes or TCB changes",
  "25 Python tests: 13 final-RMS protocol, four unchanged M5 and eight ranking cases",
  "All 1,494 host payloads and 43 protocol payloads are retained and independently hash-checked",
  "Neither a native final-RMS capture nor the hooked full 133-token model reference has run",
  "earlier row-131 mismatch and published token 4710 remain unchanged",
  "no administrator-managed isolated runtime is known to be available",
  "shared host libraries are untouched",
  "Compiler abd8 adopted: 50 dependency and source phases passed",
  "All 50 dependency/source-gate phases pass: 32 locked graphs, 47 fresh source-gate tests and four exact dependency inventories",
  "Fresh normal and test source-gate executables are retained; these are not the matching compiler tools",
  "All 1,157 integrated source identities match the validated census",
  "all 1,646 payload hashes, sizes and modes and 16 external records are independently verified",
  "Matching abd8 tool-build success is recorded separately below; MFMA emission and native validation remain pending",
  "Matching abd8 compiler tools: six build phases passed",
  "Matching compiler abd8be76 tools pass all six build phases on mi300x with four fresh, non-test products",
  "All 5,123 compiler source files and 34 protected inputs remain unchanged",
  "90-member compact archive retains 89 payloads",
  "every hash, size, mode and header and all 15 external records are independently verified",
  "Docs-only tracker b1adf9bb records this result; Ferric implementation source remains 6e2ad936",
  "engineering tool builds, not full compiler qualification",
  "matching optimized release is recorded separately below; MFMA emission, native final-RMS capture and hooked reference comparison remain pending",
  "no GPU run, numerical acceptance, performance measurement or M1 gate closure is claimed",
  "Optimized diagnostic release and device metadata passed",
  "optimized diagnostic executable passes all six release phases at Ferric 6e2ad936 / compiler abd8be76",
  "fresh normal producer has opt-level 3, no root features, debug information or RPATH/RUNPATH",
  "All 302 archive members, 301 payloads and 12 external records have independently verified custody",
  "Historical R6 tests remain at compiler 2d3, not rerun on abd8",
  "separate locked, offline, all-feature device metadata phase passes",
  "53 compiler Git packages at abd8 and two Pliron packages at cc902cc8",
  "all nine raw records verified",
  "Metadata is not vendor formation or emission",
  "separate cleanup inventory remains held with no additional deletion; vendor capacity is not admitted",
  "Docs-only tracker d1124352 records this checkpoint",
  "no numerical acceptance, protected proof qualification, performance measurement or M1 gate closure is claimed",
  "Final-RMS host evidence ledger SHA-256",
  "Final-RMS protocol evidence ledger SHA-256",
  "The following model-bundle and compiler records describe earlier checkpoints",
  "Model-bundle component proof: scoped result, separate assessment",
  "Host checks and source inventory passed",
  "Registered proof timed out; protected qualification unavailable",
  "Retained M5 logits: CPU-only ranking replay",
  "reference tokens 16, 220 and 576 tie at 18.75",
  "Ferric selects 576 at 18.75, while tokens 16 and 220 are 18.625",
  "All eight new ranking tests pass, and the actual replay preserves the original comparison metrics",
  "neither a cause nor an FP32 fix or RMSNorm explanation",
  "CPU-only M5 ranking replay SHA-256",
  "119 verified queries, zero errors and 114 successful function details",
  "original campaign remains exit 1",
  "without rerunning Verus or rewriting that failure",
  "not observed file-open telemetry",
  "47 tests and strict Clippy on its original R5 source",
  "18 selected regressions, zero failures or ignores, strict Clippy and preservation checks",
  "R7 then passes all eight source-inventory phases",
  "Checked-body classification is not a new proof",
  "1.32 GiB while preserving source, copied executables and evidence",
  "unchanged 600-second cargo-verus limit during fresh dependency compilation",
  "no selected-theorem result is produced",
  "three negative mutations and two evidence-policy checks do not launch",
  "Dependency verification counts are not the selected theorem's result",
  "timeout archive has complete independently verified custody",
  "Protected functional-refinement qualification remains unavailable",
  "Newer upstream 2d3ffbed is observed but unadopted and unvalidated",
  "No new GPU observation, TTFT, TPOT or throughput measurement",
  "Prepack publication fix source",
  "Prepack publication evidence SHA-256",
  "Earlier adopted compiler (9f8)",
  "Compiler 9f8 adoption evidence SHA-256",
  "Retained abd8 release tracker source",
  "Model-bundle integration source",
  "Component Verus evidence SHA-256; original campaign failed",
  "Separate component assessment SHA-256",
  "Source-gate R5 evidence SHA-256; generation then failed",
  "Scoped engine-host evidence SHA-256",
  "R7 inventory evidence SHA-256",
  "Integrated executable inventory SHA-256",
  "Registered theorem timeout evidence SHA-256",
  "Earlier compiler 9f8 adoption source",
  "Earlier abd8 compiler adoption source",
  "Compiler abd8 adoption evidence SHA-256",
  "Compiler abd8 normal source-gate SHA-256",
  "Compiler abd8 test source-gate SHA-256",
  "Compiler abd8 matching-tool evidence SHA-256",
  "Retained abd8 diagnostic source",
  "Optimized abd8 diagnostic release evidence SHA-256",
  "Retained abd8 device metadata SHA-256",
  "Earlier compiler 7f90 adoption source",
  "Earlier R18 compiler and M5 summary",
  "Finite singleton windows: K4, K8 and K16",
  "Compiler 7f90 adopted: 47 dependency and source phases passed",
  "Matching 7f90 compiler tools: all six tool phases passed",
  "Matching 7f90 compiler tools now pass all six tool phases with four fresh products",
  "CLI, linker proxy, backend and extractor are four fresh Cargo products",
  "all 5,060 compiler source files match the canonical archive",
  "Historical compiler-111 and compiler-1fad tools and artifacts are preserved",
  "61,196,208-byte archive has 91 members and 90 payloads",
  "not full compiler qualification or a Ferric optimized-release, device-emission, native, numerical or performance result",
  "Compiler 7f90 matching-tool evidence SHA-256",
  "RMSNorm: 167 tests pass; strict Clippy stops the campaign",
  "R4: tests and build-script parity pass; Clippy stops",
  "R5: 24 tests and all six strict Clippy selections pass",
  "Compiler 7f90d187 is adopted at Ferric f77236cb",
  "all 47 dependency/source-gate phases pass, including 42 fresh source-gate tests",
  "167 tests, zero failures or ignores, and 21 fresh Ferric test executables",
  "overall R3 campaign exits 101",
  "first strict Clippy selection rejects print_literal",
  "other five Clippy selections are unlaunched",
  "complete scoped test pass, not a complete host campaign or Clippy pass",
  "28 locks add the fe2o3-macros to fe2o3-rustc-front dependency",
  "kernel_context_contract_wire test target",
  "20,554,306-byte archive has 1,608 members and 1,607 independently verified payload hashes and sizes",
  "94,103,785-byte archive has 191 members and 190 independently verified payload hashes",
  "R4 passes 24 standalone tests with four fresh executables and ten actual build-script parity commands",
  "old and new output bytes match in all four fallback/managed modes",
  "overall R4 campaign exits 101",
  "first strict Clippy needless_borrows_for_generic_args diagnostic in the host test",
  "27,083,593-byte archive has 123 members and 122 independently verified payloads",
  "other 143 tests retain their d70491b2 attribution",
  "not claimed as rerun on dfd951ce",
  "Successor 011e0aca removes only the needless borrow in one host test",
  "All 17 focused R5 phases pass on mi300x",
  "24 standalone RMSNorm tests, zero failures or ignores, four fresh test executables and all six strict Clippy selections",
  "25,049,968-byte archive has 186 members and 185 independently verified payloads with complete local custody",
  "ten-command build-script parity remains at dfd951ce and is not rerun or relabeled as 011e evidence",
  "This host campaign alone establishes no native result, numerical improvement, accepted tolerance or performance claim",
  "They do not validate compiler 7f90 or establish an RMSNorm numerical improvement",
  "Compiler 7f90 adoption evidence SHA-256",
  "RMSNorm R3 tested source; campaign 101",
  "RMSNorm R3 retained evidence SHA-256",
  "RMSNorm R4 source; campaign 101, parity passed",
  "RMSNorm R4 retained evidence SHA-256",
  "RMSNorm R5 source; focused campaign passed",
  "RMSNorm R5 retained evidence SHA-256",
  "Compiler 7f90: optimized release and vendor formation passed",
  "matching optimized executable is rebuilt and its 198-package vendor view is formed",
  "Matching MFMA13 emission passes all 13 phases at 011e0aca / compiler 7f90",
  "On Ferric 011e0aca / compiler 7f90d187",
  "all six optimized release phases pass on mi300x",
  "fresh 14,254,640-byte executable is the normal no-feature opt3/debug0 Cargo product",
  "all 1,148 source files are unchanged",
  "release archive has 210 members and 209 independently verified payloads",
  "Vendor formation also passes: 198 packages copied, 53 compiler packages refreshed",
  "143 registry and two Pliron packages preserved",
  "both older vendor views and source inputs unchanged",
  "All 62 raw formation records have independently verified hashes, sizes and exact roster",
  "Matching emission and later GPU observations are recorded separately",
  "No numerical improvement, accepted tolerance or performance result is claimed",
  "Newer upstream e7aff682 remains observed but unadopted",
  "Compiler 7f90 optimized-release evidence SHA-256",
  "Compiler 7f90 release executable SHA-256",
  "Compiler 7f90 vendor-formation receipt SHA-256",
  "Compiler 7f90 vendor raw-custody manifest SHA-256",
  "Matching MFMA13 emission: all 13 phases passed",
  "Matching MFMA13 emission passes all 13 phases on Ferric 011e0aca / compiler 7f90d187",
  "fresh same-source metadata and empty-home vendor parity",
  "exact sets of 13 kernel entries and 13 descriptors, actual MFMA instructions",
  "113,192-byte HSACO",
  "2,046,894-byte archive with 175 members and 174 locally verified payloads",
  "Entry/descriptor set checks do not attest Ferric descriptor-table order",
  "Publication, load and launch grants remain false; no GPU execution occurred",
  "This emission campaign establishes no numerical acceptance or performance claim",
  "Newer upstream e7aff682 remains unadopted",
  "Compiler 7f90 MFMA13 inspection SHA-256",
  "Compiler 7f90 MFMA13 image SHA-256",
  "Compiler 7f90 emission evidence SHA-256",
  "Native capture stopped by the shared-GPU guard",
  "native capture exits 126 when the unchanged shared-GPU guard observes 349,642,752 bytes",
  "Only the owned group is terminated and confirmed absent",
  "No logits or capture-pass receipt is produced, and process exit is unrecorded",
  "49,152 bytes above baseline, so GPU restoration is not claimed",
  "failure archive has 46 members and 45 independently verified payloads",
  "No reference comparison is launched from this failed capture",
  "M5 comparison tracker source",
  "Compiler 7f90 native-attempt failure evidence SHA-256",
  "Current five-position comparison: one greedy-token mismatch",
  "The only published token, 4710, is unchanged; this is not numerical qualification",
  "second 011e0aca / compiler 7f90 capture completes with process and campaign exit 0",
  "1,519,360 bytes of logits and the exact 298,647,552-byte GPU memory baseline restored",
  "50-member archive has 49 independently verified payloads",
  "All 20 pre-comparison input checks pass, including all 133 token IDs",
  "Two fresh independent 133-token reference forwards are byte-identical and match the historical reference bytes",
  "Four target argmax tokens match, but position 131 selects 576 instead of reference token 16",
  "uncommitted tail row: zero draft tokens are accepted",
  "correction token 4710 is the only published token, unchanged from the baseline",
  "Maximum absolute error is 0.25; per-row RMSE ranges approximately 0.0386 to 0.0795",
  "mixed result, not a general numerical improvement",
  "Rows 128, 129 and 132 improve in both maximum absolute error and RMSE",
  "row 130 has unchanged maximum error but higher RMSE, and row 131 is worse on both measures",
  "reference archive has 59 members and 58 independently verified payloads",
  "All 22 post-comparison identity checks pass, permitting matched-input two-checkpoint error deltas",
  "Both compiler and source changed, so this is not an isolated RMSNorm ablation",
  "No numerical pass, accepted tolerance, performance result or M1 gate closure is claimed",
  "Compiler 7f90 second-capture evidence SHA-256",
  "Compiler 7f90 five-position comparison SHA-256",
  "Compiler 7f90 pre-comparison identity SHA-256",
  "Compiler 7f90 reference evidence SHA-256",
  "Compiler 7f90 post-comparison identity SHA-256",
  "On frozen Ferric a0cc9929 / compiler 1fadb7e0",
  "Retained M5: matching tools, executable and MFMA13 image",
  "all six matching compiler-tool phases pass on mi300x",
  "CLI, linker proxy, backend and extractor are fresh Cargo products",
  "all 5,046 compiler source files and protected historical tools remain unchanged",
  "85 members and 84 verified payloads",
  "All six optimized one-bin release phases also pass",
  "14,254,656-byte speculative smoke executable",
  "normal no-feature opt3/debug0 Cargo product",
  "88 members and 87 verified payloads",
  "Matching MFMA13 emission passes all 12 phases",
  "exact 13-entry and 13-descriptor inspection",
  "gfx942:xnack-, code-object version 6 and exact output replay",
  "112,680-byte HSACO",
  "169 archive members / 168 payloads have independent local custody checks",
  "original 21682228 producer attribution",
  "Engineering publication, load and launch grants remain false",
  "not adopted or validated by this frozen campaign",
  "Five-position native capture: guarded run completed",
  "one actual S1/K4 round on mi300x GPU 4",
  "all five target-logit positions 128 through 132",
  "1,519,360 BF16 bytes",
  "process and outer wrapper exit 0",
  "source, input and control checks pass",
  "owned process group is absent",
  "GPU allocation baseline is restored",
  "5,580,101 bytes with 50 members and 49 payloads",
  "all 50 members and 49 payload hashes and sizes pass independent local verification",
  "Five matching tokens; logit differences remain unqualified",
  "Two full 133-token reference forwards repeat byte-identically without reusing Ferric KV",
  "historical 128-active-token filled prefix plus the anchor and four actual draft proposals",
  "not normal real-prompt serving",
  "All five greedy argmax tokens match [4710, 16, 15, 16, 198]",
  "zero token mismatches and all logits finite",
  "maximum absolute error is 0.2421875",
  "RMSE ranges approximately from 0.0313 to 0.0755",
  "maximum BF16 ULP distance is 31,640",
  "2,345,460-byte archive containing 59 members and 58 payloads",
  "all 59 members and 58 payload hashes, sizes, modes and timestamps pass independent local verification",
  "Token agreement does not establish logit conformance",
  "no reviewed tolerance, numerical pass, serving qualification or TTFT/TPOT/throughput result is claimed",
  "not reviewed numerical parity, production admission, serving qualification or a matched performance result",
  "later compiler-1fad native capture does not relabel compiler-349 host/proof or compiler-111 numerical results",
  "Retained M5 engineering source",
  "Compiler 1fad matching-tool evidence SHA-256",
  "Compiler 1fad optimized-release evidence SHA-256",
  "Compiler 1fad MFMA13 emission evidence SHA-256",
  "Retained five-position capture archive SHA-256",
  "Retained five-position comparison SHA-256",
  "Retained independent reference logits SHA-256",
  "Retained five-position reference archive SHA-256",
  "Upstream compiler observed at the 1fad checkpoint",
  "Compiler 1fadb7e0 adopted: dependency and source checks passed",
  "Private Ferric a0cc9929 adopts published fe2o3 1fadb7e0 across exactly 80 active pin files",
  "separate fresh R2 campaign passes all 47 mi300x dependency/source-gate phases",
  "native_v12_text_descriptor_replay_v1 test target",
  "dialect-amdgcn development dependency on fe2o3-amd-target",
  "not a revision-only inventory change",
  "All 1,148 local source hashes and the complete tracked-file roster match",
  "first attempt stops with exit 1 on its revision-only assumption",
  "not engine, Verus, native, numerical or serving validation on 1fadb7e0",
  "compiler-349 host and proof results and compiler-111 native and prefill results below retain their original identities",
  "Combined Ferric source 568882e2 on compiler 349b2cd0",
  "Combined M5 capture: host checks passed on compiler 349",
  "750 engine tests with nine explicit ignores, 22 smoke-bin tests, and Python suites of 4, 23 and 15 tests",
  "all five real S1/K4 target-logit rows",
  "Strict Clippy passes for the engine library and exact smoke binary, not all adapter or all-target coverage",
  "Private integration 6b5322dc contains the exact tested implementation",
  "initial missing-import host failure remains retained",
  "No M5 native capture, five-position numerical comparison, serving qualification or new performance result is claimed",
  "Compiler 349: focused production catch-up proof",
  "Seven genuine dependency exports are rebuilt and verified",
  "executable mode: one verified, zero errors, solver rlimit 116,015, with --no-cheating",
  "nine actual-body negative mutations belong to the earlier a9fad31b / compiler 111 campaign and have not been rerun on compiler 349",
  "focused single-method proof, not whole-engine, queue/owner or physical-KV composition",
  "does not validate later compiler heads",
  "Core compiler boundary: 52 tests pass, strict Clippy remains open",
  "52 selected tests on mi300x in R5, on tested tree a29459fd over parent 4c298382",
  "existing unavailable functional-refinement runtime boundary",
  "not production proof admission or a full compiler/proof pass",
  "Strict Clippy exits 101 with 17 diagnostics at unchanged parent sites",
  "overall campaign remains exit 101",
  "published as 1fadb7e0 on main after a fresh rebase onto 958a80a3",
  "52 tests are not rerun or relabeled as a test campaign on that rebased tree",
  "155 normal/build dependency nodes",
  "push uses [skip ci]",
  "this R5 campaign does not validate the adopted Ferric engine",
  "They are not new M5 or compiler-349 GPU results",
  "Compiler 111: 32 tokens and completed draft catch-up",
  "one completed draft catch-up after full K4 acceptance and a subsequent round",
  "32 outputs exclude the prefill anchor",
  "durations are not TTFT or TPOT",
  "These results do not validate compiler 349b2cd0 or 45d0bf2e",
  "Prefill: matching token, logit differences still unqualified",
  "151,936 finite BF16 logit pairs and one token, with zero token mismatches",
  "Maximum absolute error is 0.125, RMSE is 0.03982024072957679",
  "Maximum BF16 ULP distance is 31,373",
  "No reviewed tolerance or numerical pass is claimed",
  "first wrapper exits 1",
  "corrected wrapper exits 0 in a separate directory",
  "Both attempts are retained",
  "Earlier compiler 111 proof: nine negative mutations rejected",
  "combined-source host and compiler-349 proof results above supersede those pending states, not their original evidence identities",
  "actual production commit_draft_catchup_transition method: one verified, zero errors",
  "does not prove the whole engine, sealed owner mapping, queue completion or the physical KV join",
  "All nine deliberate implementation mutations fail the selected method's postconditions",
  "original source is restored and final source/dependency checks pass",
  "not yet integrated or validated on compiler 349b2cd0",
  "Earlier compiler 349 adoption and 45d0 observation",
  "No engine, Verus or GPU result is attributed to this adoption",
  "read-only upstream check observes 45d0bf2e",
  "it is not adopted or validated here",
  "Earlier compiler 111 adoption: dependency checks passed",
  "They do not supersede the native, numerical, proof or compiler checkpoints above",
  "All 47 mi300x dependency/source-gate phases pass",
  "32 locked/offline graphs across 35 metadata checks",
  "42 tests from a freshly built source gate",
  "four actual CLI inventory derivations",
  "Three compiler-bearing inventories change only their revision",
  "runtime inventory is unchanged",
  "not full compiler-tool qualification, an engine host suite, Verus, kernel emission or GPU qualification",
  "earlier smoke R3/R4 results remain attributed to compiler 8af54567",
  "not rerun or relabeled as 111 results",
  "Earlier 8af smoke checkpoint: scoped host checks passed",
  "Its 8af test attribution and then-pending adoption status are preserved",
  "exact 12/13 program count from the admitted artifact",
  "147 tests with two existing hardware ignores",
  "38 source-policy tests, four engine regressions, three GEMM regressions",
  "strict all-target adapter Clippy",
  "separately attributed host cohorts, not one green campaign",
  "Ferric remains pinned to 8af54567",
  "published fe2o3 111722028 is rebased onto d6471109",
  "152 tests with ten existing ignores",
  "zero GitHub workflow runs and zero check runs",
  "not complete compiler qualification",
  "Current kernel emission and the rebuilt native GPU retry remain pending",
  "No new GPU result, TTFT, TPOT or throughput measurement",
  "Earlier selected-proof and native checkpoints",
  "They do not supersede the current smoke or compiler results above",
  "The MFMA path stays opt-in with LegacyScalar12 as default",
  "separate thirteen-program roster and explicit engineering opt-in",
  "Earlier retained checkpoints, including the R4 snapshot",
  "They do not supersede the current proof or maintenance results above",
  "All 33 M1 gates remain open",
  "No new GPU or performance result",
  "Seven-case tooling; GPU comparisons remain 1 of 7",
  "seven canonical cases and 52 output rows",
  "All 24 comparison tests, 15 engineering and 23 legacy reference tests",
  "remaining six GPU cases have not run",
  "Repeated resident rounds: host validation",
  "Exact dfb passes 707 engine library tests (nine existing ignores)",
  "exact 942 results: 109 library tests (one ignore)",
  "12 speculative CLI tests, 37 source-policy tests",
  "Earlier import, resource-stop and source-inventory failures remain retained separately",
  "Host checks alone do not establish native execution or qualification",
  "Five native K4 rounds: zero and partial acceptance",
  "actual structural registry, bridge and coordinator",
  "Accepted draft counts are [0, 0, 1, 2, 0]",
  "full-acceptance maintenance/restore remains unobserved",
  "actively EOT-filled to 128 tokens, not attention-mask padded",
  "prefill anchor 12 is excluded",
  "not ordinary raw-prompt serving or a numerical comparison",
  "Earlier debug attempt: stopped before inference",
  "unchanged 8113/4f6 image",
  "not a deadline or resource-cap stop",
  "ownership remains unresolved",
  "Stdout is empty: no prefill, speculative-round, token or catch-up/restore result is claimed",
  "failed engineering attempt, not a native continuation or performance result",
  "Optimized resident executable: build only",
  "retained 14,133,320-byte executable",
  "GPU access was disabled during the build phase",
  "does not replace or relabel the debug guard-stop record",
  "Retained c5 source coverage: no proof upgrades",
  "177 modules and 8,557 bodies",
  "92 new structural-runtime/comparator rows remain proof-pending",
  "No proof label or M1 gate is upgraded",
  "Matrix-kernel strategy: separate host validation",
  "complete unpublished b86 compiler overlay",
  "709 engine library tests (nine ignores)",
  "110 adapter library tests (one ignore)",
  "87 capture tests (two ignores) and 38 policies",
  "Exact successor 1fc with the same overlay passes scoped engine/adapter strict Clippy and eight focused catalog tests",
  "These older b86 host records did not emit an MFMA device image",
  "No MFMA GPU execution or performance improvement has been demonstrated",
  "Earlier compiler checkpoint: paired exports before emission",
  "14 selected proofs; maintenance fix host-tested",
  "R9 on cde4faaa passes all 14 selected executable page-return bodies",
  "does not prove outer pool/queue composition, negative mutations or the changed maintenance checker",
  "Native R6 uses frozen 00ac6a22/4f6 and exits 134",
  "before submitting the 425 maintenance packets",
  "26 distinct tests across 27 executions",
  "not a full-engine, Clippy, Verus or GPU pass",
  "still needs rebuilding and a GPU retry",
  "fetched 8af54567 is not the validated pin",
  "No new TTFT, TPOT or throughput measurement",
  "Earlier 55-pinned MFMA host checkpoint; consumer proof then open",
  "1,195 passes / 14 ignored across two sources",
  "R3 at 538aa053 adds 11 scoped",
  "R4 at 873f4958 passes strict core engine/spec/Qwen Clippy",
  "103 aggregate kernel style diagnostics per GPU target remain open",
  "Adapter-only reorder 2108e736 passes formatting, with its lint retry pending",
  "R4 proof retry reaches SMT but fails final-prefix and next-prefix obligations",
  "actual failed retry, not a pending run or a proved consumer",
  "upstream 179340e3 is not adopted or validated",
  "Earlier e3 dependency refresh; caller then unvalidated",
  "Private Ferric integration 9e10cc95 updates 80 dependency, lock, policy and inventory files to published compiler e3c359fb",
  "dependency-only overlay on 58833759 passes 42 mi300x phases",
  "all 32 locked/offline graphs",
  "38 source-gate tests and reviewed dependency inventories",
  "M0 property binder retains its separate e527 pin",
  "These receipts validate the 588 dependency overlay, not the newer Rust caller or a full 9e engine run",
  "8af1e52b's contracted batch ledger consumer loop; its host, Verus and native validation have not run",
  "Earlier 1fad status tracker source",
  "Retained abd8 dependency pin",
  "MFMA13 smoke validation compiler",
  "Earlier published correction; adoption then pending",
  "Earlier validated compiler pin",
  "Earlier fetched compiler; adoption then pending",
  "Earlier private integration source",
  "Dependency-only validation base",
  "Dependency refresh evidence SHA-256",
  "Original contracted caller; then unvalidated",
  "Frozen native runtime: tested diagnostics, maintenance still blocked",
  "32-token attempts: separate failures, no inferred success",
  "R5 at 00ac stops after about eight seconds with guard exit126",
  "No completed rounds, tokens, full acceptance or maintenance success are inferred",
  "Ferric MFMA image emitted; GPU validation pending",
  "Retirement proof: scoped positives pass, composition still open",
  "Unpublished compiler candidate 5602",
  "actual paired row-major/column-major storage exports",
  "At this earlier checkpoint, HSACO emission and the real 13-root aggregate were pending",
  "Retained twelve-kernel image: emitted, not GPU-tested",
  "actual R12 commit proof at 339c subsequently fails",
  "Trigger-only successor bd8e22f7",
  "pinned Verus b677dd5: two solver queries and zero errors",
  "six scoped metadata proof bodies",
  "checked query counts 2/2/2/1/1/1, nine total",
  "Actual-runtime mutation campaign R16 exits 0 with all 24 negative receipts accepted",
  "complete positive/negative raw archive is retained and independently SHA-256 checked",
  "whole-crate correctness or hardware behavior",
  "Test-only routing candidate da3b3a1d",
  "exact one routing test pass",
  "strict Clippy fails on two test-only lints",
  "Successor 45a211e5 passes all five remote host phases",
  "exact 4f6 metadata",
  "exact one routing test and strict all-target Clippy",
  "Before/after source receipts match and the frozen native binary is unchanged",
  "25 actual remote mock fixtures",
  "not GPU or native validation",
  "Native R6 has not run",
  "Earlier helper integration 8fa40b3a passes all eight remote host phases",
  "723 engine tests with nine ignored",
  "Six actual engine helper bodies also pass pinned cargo-verus",
  "helper negative mutations and caller/whole-roster/native composition remain pending",
  "79f74a3e fails R5 before SMT with exit101",
  "Parentheses-only successor 74c52a73 then passes R6 for the selected global_page_index_core exec body",
  "one verified query, zero errors and rlimit154038",
  "2aae5839, whose R1 host attempt exits 1 at formatting only",
  "All five source after-checks pass, but 2aae has no completed host validation",
  "Test-formatting-only successor 21818e2a preserves production bytes equal to proof source 74c52a73",
  "its separate R2 host campaign passes all eight phases, including 724 engine tests with nine ignored and strict all-target engine/spec Clippy",
  "The 8fa 723-test host cohort is not relabeled",
  "All six 218 source before/after checks match",
  "R7 completes all seven actual index-body negatives",
  "Full R7 evidence is retained and independently hash-checked",
  "ledger candidate efa069bd",
  "726 engine tests with nine ignored",
  "fails strict engine Clippy on manual_map; spec Clippy is not reached",
  "Successor 47d07cee",
  "original R2 host run is recovered without a rerun and exits 0",
  "All 16 source checks pass, source-after equality matches, and owned process groups are absent",
  "SSH transport exit 255 is separate from the actual host run's exit 0",
  "Full host evidence is retained and independently SHA-256 checked",
  "The R9 strict nine-helper positive proof campaign completes all nine selected actual bodies",
  "each reports one verified exec query, zero errors and nonzero SMT work",
  "Full R9 proof evidence is retained and independently SHA-256 checked",
  "R10 completes all six actual ledger-body negative checks and exact restorations",
  "Full R10 negative evidence is downloaded and SHA-256 checked",
  "Independent retained-evidence review is complete with no blockers",
  "The verified ledger helpers are integrated at 66e32993",
  "device_cache.rs is byte-identical to host/proof source 47d07cee",
  "Host and proof receipts remain attributed to 47d07cee; no integrated-source rerun is claimed",
  "Earlier compiler R10 host evidence SHA-256",
  "Selected global-index core R7 negative evidence SHA-256",
  "Integrated index host evidence SHA-256",
  "Ledger candidate failed host evidence SHA-256",
  "Ledger successor host evidence SHA-256",
  "Ledger successor R9 proof evidence SHA-256",
  "Ledger successor R10 negative evidence SHA-256",
  "Verified ledger helper source",
  "Ledger helper integration source",
  "Tested private compiler 86ccc2a5",
  "all twelve unchanged patches on observed base bd4d5f42",
  "R9 remote delta host campaign completes all 23 phases with exit 0",
  "the paired BF16 fixture",
  "Published compiler e3c359fb is rebased onto observed upstream 2585ce64 with all twelve patches unchanged",
  "R12 completes all 19 mi300x phases with 1,030 Rust tests passed, zero failed and one existing gfx1151 ignore",
  "nine Python dependency tests and the CI dispatch harness also pass",
  "The initial inherited-environment CI harness failure after six passing phases is retained separately",
  "The R2 retainer rejected paired --nocapture framing; the strict R3 retainer passes without rerunning tests",
  "The complete R12 archive is retained locally and independently SHA-256 checked",
  "A normal non-force main push completed: observed GitHub main equals e3c359fb at 2026-09-15T23:48:06Z",
  "At that observation, no Actions runs or check runs exist for the exact commit",
  "no workflow or protection settings were changed",
  "No current release tools, emission, strict compiler Clippy or native GPU execution is claimed",
  "Earlier published compiler source",
  "Earlier compiler R12 host evidence SHA-256",
  "Initial R12 pre-build failure evidence SHA-256",
  "The tip contains [skip ci] under explicit user approval, and all builds remain on mi300x",
  "Previous private compiler 12845295 is rebased onto observed upstream 48323569 with all twelve patches unchanged; its R10 delta host campaign passes all 14 phases, with source-bound freshness checks and the actual LLVM 18.1.3 assembler identity unchanged afterward",
  "New 86 source-bound tool builds have not run",
  "Frozen compiler e0d108b2 separately passes 42 host phases and four actual tool build/copy phases",
  "113,192-byte gfx942 image exactly replays",
  "13 entry/descriptor pairs",
  "These e0 tools and image are not relabeled as 86, 128 or e3c",
  "Full source and raw R10 results are retained and independently SHA-256 checked",
  "Unchanged R9 suites were not rerun",
  "no 128 release tools, emission, strict compiler Clippy, GPU result, compiler push or Ferric repin is claimed",
  "no GPU ran, and publication/load/launch grants remain false",
  "not new emission, GPU execution or qualification",
  "Remote validation: fresh resource admission required",
  "not a current admission reading",
  "Additional cleanup restored resources",
  "root free space fluctuates on the shared host",
  "all eight GPUs at 100% busy, not idle admission",
  "Separate read-only mi350-2 inventory finds one physical MI350X/gfx950 GPU",
  "Foreign KFD queues prevent idle admission despite zero utilization at that observation",
  "platform/UAPI review is required, not just an accepted-string change",
  "No artifact was retargeted or launched there",
  "42 lineage, 84 MIR, 1,029 Pliron and 543 backend tests",
  "37 exact f9 aggregate host tests",
  "Baseline strict Clippy retains an unchanged dependency failure",
  "emit and exactly replay the 103,616-byte gfx942 image",
  "12 kernel entries and 12 descriptors",
  "differs from 4f6 and has no GPU validation",
  "earlier pre-emission resource stop remains separate",
  "No accepted matched SGLang result is available",
  "One-case numerical comparison",
  "one generated 128-token input, not the earlier five-token English prompt or speculative padded prompt",
  "151,936 compared logits are finite",
  "both select token 198 and have identical top-10 token ordering",
  "logit rows are not byte-identical",
  "maximum BF16 ULP error is 31,121",
  "maximum absolute error 0.0703125",
  "RMSE 0.01524191977257529",
  "cosine similarity 0.9999202347605426",
  "20,968 exact BF16 matches and 619 opposite nonzero signs",
  "within one model load, not two fresh launches",
  "one of seven planned R29 cases, not a tolerance acceptance or full R29 comparison",
  "does not GPU-validate the newer 4f6 image",
  "Retained 4f6 image: separate structural GPU observation",
  "HSACO is 103,616 bytes and differs from ccfd despite unchanged aggregate kernel bodies",
  "separate c5 resident observation now exercises this image on a GPU",
  "without numerical qualification or relabeling its 8113 producer",
  "actual older producer identities, not 4f6 labels",
  "Retained four-token native target observation",
  "four IDs [12095, 13, 576, 6722], text \" Paris. The capital\"",
  "one-prompt token-prefix spot check, not an R29 logit comparison",
  "Controller offsets are not comparable TTFT, TPOT or throughput",
  "immediate archived after-state still reports 56,838,090,752 VRAM bytes",
  "later independent direct sysfs check observes return to the exact 298,647,552-byte baseline",
  "delayed check is separate from the immutable archive; the original after-state is not rewritten",
  "No device reset or foreign-process termination was performed",
  "Retained c6 native target smoke",
  "five-token prompt \"The capital of France is\" produces one token: ID 12095, \" Paris\"",
  "selected-device memory returns to its baseline",
  "earlier f17/c6 image, not the newer bb2/ccfd image",
  "do not establish general numerical parity, speculative catch-up, serving readiness, performance or M1 qualification",
  "Raw controller offsets are not serving TTFT/TPOT",
  "Retained ccfd compiler and catch-up checkpoint",
  "543 backend tests passing",
  "Exact private Ferric bb2b012 passes 37 aggregate host tests",
  "1,021 Pliron library tests, 58 integration tests and 13 doctests",
  "all 12 kernel entries and 12 matching descriptors",
  "HSACO is 103,872 bytes",
  "Publication, load and launch grants all remain false; engineering authority is none",
  "newer image differs from c6 and has its own separate four-token native observation",
  "earlier native smoke is not relabeled as ccfd",
  "Retained bb2 host validation and engineering binaries",
  "all 71 host and metadata phases with exit 0",
  "normal fe2o3_host artifact has features=[]",
  "building them is not GPU execution or qualification",
  "Retained d094 scoped source coverage",
  "173 modules, 8,435 executable bodies, 723 verified labels and 7,712 unverified identities",
  "helper and six executed callees verified individually",
  "seven sensitive executable mutations rejected",
  "engine preflight and complete physical catch-up chain remain unverified",
  "Resolved assertion differential",
  "49 assertions, one false-to-true proof gain and zero true-to-false losses",
  "already unproved, not a regression introduced by 852",
  "all producer assertions and limits remain mandatory",
  "Retained earlier host cohort",
  "Latest combined host validation is separate from the retained bb2/ccfd cohorts",
  "maintenance advances draft KV by one and emits zero served tokens",
  "697 library tests with nine explicit ignores",
  "171 doctests",
  "Strict engine, adapter and production-owner Clippy passes",
  "current adapter passes 109 tests with one ignore and 37 policies",
  "production owner passes 16 tests and three policies",
  "generated runner source equality; this is not a native runner or image",
  "Canonical target and draft preparation",
  "exact frozen bed11/829 CLI, not a relabeled 852 binary",
  "Retained source coverage and dependency checks",
  "173 modules and 8,433 executable identities",
  "722 existing verified labels remain unchanged",
  "38 source-gate tests, 31 verifier policies and six source-pin policies",
  "Earlier diagnostic and getter checkpoints",
  "their pending states describe those earlier checkpoints",
  "paged GQA coordinate calculation at line 322, rather than GEMM",
  "actual AST equivalence across 14 profiles",
  "bb81, source fingerprint 09ab2715abb3, line 378",
  "diagnostic attribution, not a validated compiler repair",
  "New combined Ferric engine validation against this pin is pending",
  "4 verified, 0 errors, with the exact 190-file verifier distribution closure",
  "not whole-crate verification or a proof of physical catch-up",
  "No warmed allocation-free catch-up or end-to-end serving result is claimed",
  "Earlier host validation and qualification boundary",
  "The 722 existing verified bodies are unchanged; 7,530 remain unverified, including 18 new pending-Verus rows",
  "Earlier compiler and canonical gfx942 extraction",
  "All 1,006 Pliron tests pass with zero ignores, including the 13 focused regressions",
  "Kernel attribution is unknown; this is not identified as a GEMM failure",
  "Exit 1 produces no image, replay or GPU result",
  "The eaa results do not validate e6cfa668",
  "77,791,232-invocation launch",
  "Historical measurements below are unchanged",
  "C1 live route: context8192 HTTP",
  "199.719",
  "154.310",
  "4.532745",
  "5.707624",
  "22.737% lower",
  "25.920% higher",
  "2873.632 to 2827.804 ms",
  "One fresh start per arm",
  "all forty timed SSE/request IDs",
  "38.643 times this C1 output rate",
  "pre-worker socket-bind failure with zero requests",
  "Retained C1 native diagnostic and integer proof",
  "Sharded argmax v13: static image checks",
  "52 explicit bytes, hidden start 56 and 312 total kernarg bytes",
  "only the typed handoff digest was retained",
  "detailed typed-node review and native parity remain unadmitted",
  "actually built at 216822",
  "Query-hoist v14: finite native parity",
  "8 cases + CPU replay passed",
  "not a same-compiler ablation or full-model/context8192 parity",
  "original ten synthetic control tests",
  "all nine phases, including 11 synthetic methods",
  "116 explicit bytes, hidden start 120 and 376 total kernarg bytes",
  "the typed handoff is digest-only; detailed typed review is still unadmitted",
  "eight finite native cases at TP1/context32",
  "Wave RMSNorm v15: model correctness",
  "23 kernel tests, 31 verifier tests",
  "40 complete buffer records and 80 guards",
  "all 357 protocol rows",
  "FP32 association differs from the old serial fold",
  "not general FP32 proof, invalid-case qualification or Qwen model parity",
  "Wave64, 70 SGPR / 22 VGPR and zero spills/private/LDS/AGPR",
  "c9c7036cf98034b638886f54a745701e70ac3571; V15 emission and scoped model checks passed",
  "87fdcead7418e8f64db29fdb2703f5a4c8c9b742; twenty host phases and four fixed-prompt cases passed; descriptive ABBA replay passed; no HTTP qualification",
  "Independent CPU replay reproduces all four complete checked traces and wrappers",
  "Correctness timings are excluded from the separate ABBA cohort",
  "mean TPOT 166.854 to 113.231 ms",
  "within-mode TPOT range/mean is 27.96% for baseline and 40.92% for V15",
  "not HTTP latency, GPU duration, a stable gain or competitive result",
  "V14 canary and current runtime",
  "4 full-Qwen cases passed",
  "Scoped current controller host checks pass",
  "full 1,025-test suite belongs to the earlier 61e source and was not rerun at ae267",
  "484 library tests with one unchanged ignore, 31 doctests",
  "These are not model runs",
  "four real Qwen3-8B correctness cases pass",
  "independent CPU replay exactly reproduces all four wrapper objects",
  "initial ContentDirectoryIdentity failure occurred before setup and remains archived",
  "do not establish generalized or context8192 parity",
  "Raw host timings are retained without reduction or a new performance claim",
  "offline-dependency and typed-convergence failures before R3",
  "Qualification requests in both arms",
  "These counts do not describe the separate 42-request matched cohorts",
  "Wave attention / v11: context8192 HTTP",
  "3482.380",
  "276.255",
  "3.318800",
  "2862.079",
  "192.127",
  "4.694997",
  "not a context256 proxy",
  "first sample-window start through the last sample-window end",
  "all forty timed HTTP request IDs and final token arrays",
  "not repeated independent starts, stable tails, confidence intervals or sustained-load qualification",
  "vLLM was not rerun alongside this new profile",
  "The v5 and FP32-v8 images remain frozen artifacts with their original provenance",
  "Current parallel checkpoint",
  "C1 layer projection: variable native diagnostic",
  "180.651",
  "162.999",
  "4.976281",
  "5.523671",
  "2787.321 to 2772.372 ms",
  "9.772% lower",
  "11.000% higher",
  "0.038% lower and 19.852% lower",
  "does not establish a stable gain",
  "142.252 to 183.746 ms",
  "All 33 M1 gates remain open",
  "v13 ownership: standalone integer proof",
  "18 queries covering all 16 required functions with zero errors",
  "not actual-kernel refinement",
  "not a refinement of this kernel",
  "strict Clippy actually exits zero",
  "Typed emission, native parity and performance gain remain unadmitted",
  "C1 live route",
  "all 40 host commands",
  "892 test invocations, 48 ignored across 21 result rows",
  "host-only admission, not a new native or HTTP measurement",
  "Retained HTTP and kernel checkpoints",
  "Visible-token attention",
  "Stopped for the current C1 workload",
  "single-row decode has zero masked tail iterations",
  "Emission then failed the unique-header-exit check",
  "Clippy commands exited 101 and matched known debt, not a strict Clippy pass",
  "278 SGPR spills versus zero",
  "18 tests where 19 were expected after stale binary reuse",
  "Guarded source 8b57646 failed FE2O3-PROGRESS-002",
  "Failure custody is accepted, not compiler or image admission",
  "Ordered residual tail",
  "780 adapter test invocations, 41 ignored across 19 result rows",
  "74 HTTP fixtures, 38 source-gate and 31 protected-policy tests",
  "Private integration 212ec85 preserves the tested code",
  "Four native TP1/context256 cases now pass: synchronous and ordered modes with 8 or 128 outputs",
  "These are fixed-reference correctness checks, not HTTP or performance qualification; their timings are excluded",
  "Residual-tail diagnostic: decode regression",
  "194.521",
  "218.333",
  "4.648212",
  "4.200225",
  "Decode regressed",
  "increased 12.24%",
  "fell 9.64%",
  "Mean TTFT improved 3.86%",
  "Both new TPOT samples are slower than both old samples",
  "n=2 per exact binary",
  "not HTTP latency or GPU durations",
  "build environments are not proved identical",
  "Published serving measurements remain unchanged",
  "Earlier team records",
  "Ordered submission: host and correctness gates passed",
  "51 host commands without retry",
  "745 passed adapter test invocations, 38 ignored",
  "26 and 18 synthetic CPU methods",
  "Ordered submission: full128 ABBA diagnostic",
  "15.45% lower TPOT and 17.56% higher mean per-run output rate",
  "Paired rate gains are 21.03% and 14.03%",
  "Synchronous / ordered / ordered / synchronous; n=2 per mode",
  "Operation-group timings are not comparable across submission modes",
  "Attention attribution and validation",
  "Seven attention operation groups",
  "41.068",
  "72.59",
  "40 unchanged inputs and 96 guard sides",
  "not GPU durations",
  "parent and IPC spans overlap",
  "Combined attention / argmax: host and native gates passed",
  "42 required commands across 43 actual attempts",
  "685 passed test invocations, 34 ignored",
  "failed nested-TMP invocation and earlier Clippy failure",
  "Combined attention / argmax: full128 ABBA diagnostic",
  "521.458 / 258.562 ms",
  "50.42% lower TPOT",
  "97.33% higher mean rate",
  "224.460-292.664 ms",
  "56.99% and 43.84%",
  "not stable estimates, confidence intervals, HTTP serving or GPU durations",
  "not 256 divided by pooled duration",
  "First matched Ferric / vLLM cell",
  "Ferric is substantially slower than vLLM on this cell",
  "3705.967",
  "506.969",
  "19.243",
  "4.414",
  "220.558111",
  "not assigned zero throughput",
  "Independent paged-draft FP32 reference",
  "Paged draft: all eight native cases pass",
  "Paired K4: two fresh native passes",
  "Parallel FP32 argmax: 14 native fixtures",
  "Opt-in argmax route: host gate passed",
  "Retirement harness: rejected attempt preserved",
  "Ordered runtime: diagnostic counters only",
  "Earlier native checkpoints",
  "Native argmax: full 128-output pair",
  "Native argmax: short ABBA diagnostics",
  "553.419547 / 528.962031 ms",
  "16.79% to 7.72%",
  "not isolated GPU-kernel durations",
  "not isolated GPU execution",
  "Wave plus ordered: short-canary ABBA",
  "SGLang r7: numerical rejection retained",
  "two genuine last-proposal catch-ups",
  "Native rejection/rollback branches still need other workloads",
  "The adapter still uses v8",
  "+12.27%, n=2 per mode",
  "Do not substitute this short canary into the unprofiled matched 128/128 table",
  "10 of 30 measured texts",
  "No SGLang timing is admitted",
  "fe2o3 frozen measurement cutoff",
  "Active development tracks 8efd4fd416d1ffae7a718144e4d299fe3c8f7590",
  "tree 93a51b4af0287ccb51dec07fe431dae71fc55d0f, with separate latest-source gates",
  "These historical GPU receipts are not rebuilt or relabeled; no latest-source performance result is claimed here.",
  "566 all-target test invocations",
  "568 all-target test invocations",
  "Ferric's MFMA head and vLLM use BF16 operands with FP32 accumulation/output",
  "Sustained ingress and loopback HTTP",
  "32-row FP32 head: bounded model passes",
  "Shared peer currentness: native only",
  "Admission cache: frozen TP1 canary",
  "Large KV pool: bounded model pass",
  "Wave attention with FP32 head: tiny canary",
  "Earlier ordered-batch native checkpoint",
  "Polling experiments: regression retained",
  "Earlier authenticated draft-intake checkpoint",
  "Ordered batches: recovery with a fresh serial control",
  "Standalone draft reference and fast-kernel fixtures",
  "Exact 656 CPU integration checkpoint",
  "Continuous V3 measurement, not a serving result",
  "21682228486f7186cc3c37ddf165fffc438d8b6a",
  "lowered mean canary rate 35.40%",
  "4.531460 to 5.537212 output tokens/s",
  "reuse-prefix TTFT -21.81% and TPOT -24.62%",
  "Serial controls vary substantially",
  "not paged draft model qualification",
  "passes 36 native full-buffer/guard fixtures",
  "526 host test invocations, 19 explicit ignores",
  "no automatic proposal generation, paged native model or speculative serving",
  "two repeated 128-input/128-output runs",
  "It is not a stock-BF16-head reference",
  "516 adapter test invocations, 18 explicit ignores",
  "There is no drain between windows",
  "failed windows are retained",
  "mean workload rate rises 8.70%",
  "reuse-prefix TTFT falls 9.08% and TPOT falls 10.51%",
  "lowers mean fixed-canary rate 18.35%",
  "raises mean rate 16.23% against its own matched control",
  "These core experiments remain unpublished and are not adopted",
  "38,654,705,664 bytes (36 GiB) of KV payload",
  "Long-context and 32-request concurrency remain unqualified",
  "at most 16 observed rows",
  "184 packets over 26 exact producer/consumer chains",
  "Legacy serial DispatchSequence behavior is unchanged",
  "no model-backed success fixture was run",
  "3e3a77284a61654134211f8145dd0ddeebb2ff91",
  "All 15 native fixtures pass at rows 1/16/17/31/32",
  "actual maximum rows are 16 and 17, not 32",
  "rates 2.678148 and 2.665643 output tokens/s are effectively flat",
  "four sequential requests and nine frozen-reference outputs",
  "No endpoint remains active",
  "No matched vLLM/SGLang result exists",
  "Sequential baseline launches are now approved",
  "kernel-admission caching, not radix prefix caching",
  "Mean workload rate rises 27.72%",
  "mean whole-process time is 1.14% worse",
  "The primary 32-request workload needs 8,704 pages",
  "Checked concurrent-rank rounds",
  "Host timing and numerical diagnosis",
  "79706b43a177a2fd3fa43ec328221fa3e5041af5",
  "5110577a6d8c45390dfb353386cde748efd5d76c",
  "6f6a67bb2f6de70a1c5533bbcde1b09449c37359",
  "FP32 head on TP1: faster requests, slower startup",
  "slower fresh full process",
  "not broad numerical qualification",
  "passes nine native scalar/MFMA-head and argmax fixtures",
  "Physical GPU overlap is not measured",
  "the immediate BF16 tie mechanism is established",
  "Concurrent peer rounds: matched TP2 and TP8",
  "Host-staged execution is still faster",
  "n=1 per mode",
  "host intervals, not GPU timestamps",
  "0.471982 to 0.504046",
  "Not steady-state serving throughput",
  "Request identities are never pooled",
  "correctness failure under investigation",
  "902fef6e1478b3ac677e5456b2a2d1f917456fba",
  "1b262ac3dd23ee63067e40587d62a124f40b9fc9",
  "7528e7345cef0158d7034cdbae23011e3c6fb5d2",
  "MFMA R1 checkpoint: faster requests, slower startup",
  "MFMA repeated: request gains, startup cost",
  "MFMA plus pruning: no additive gain observed",
  "TP1 residual repeated: small decode change",
  "Current-controller combinations: mixed correctness",
  "at most 17 rows, not 32",
  "no rejected-run timing is published",
  "Startup regresses",
  "0.575054",
  "209.070",
  "negative performance evidence",
  "1557.722",
  "CPU transpose helper only",
  "TP1 device residual pair",
  "Eight-GPU allocation cohorts: 64 outputs",
  "True 32-row execution: a latency tradeoff",
  "Source-matched peer controls: a regression",
  "Setup transpose: full-model observation",
  "not a causal decode-speed claim",
  "admission TTFT worsens",
  "not a sum of instance rates",
  "240 full-array equality checks",
  "a689418",
  "ce9e0bc",
  "e3dc8d6",
  "e5d23e1",
  "6809185",
  "886a0d3",
  "c94c69f",
  "de490d6",
  "b30e7a6",
  "57bb4f6",
  "c492c31",
  "9cc283b",
  "61a40ca",
  "2abcc47",
  "57973da",
  "7747409",
  "ea6ef07",
  "fc5f869",
  "4d04864",
  "d05b2da",
  "c46263c",
  "2958b69",
  "48de4c1",
  "5df8c5e",
  "7c0c4a2",
  "be88974e99748fccb5fd4e6f0c3122b3e9a1822db038fcbd3da8707b92d9d830",
  "3d9533b2482e3f8424059333495fe1384f9cd66287471755c5fce0e1528eaee0",
  "100ecefebf2496006c1c0603c274bdff399d63eae23a35ef44f72eede25688c1",
  "1e44a64",
  "d9e9a38",
  "61fca5d4441acea3a4f5ca548b5fde142294153027b0b052b8ac32f27bd7af9c",
  "d7d68e4b4a9c9ec2951a1f849d65573f16c00a893979eb2e00d79f732a10aa27",
  "21bf7ae6a2df53c7e5c18985d1352274b224d6655d7ccc17bba98d51582fac84",
  "dcc7c07",
  "b524a9c",
  "b10a6e0ef1b40c1e05d5320e5391f1c946bea1f4b8badcffef8cbe58124233ee",
  "ea6ef07",
  "136a6d2",
  "c9684e2",
  "254b89a",
  "cf6faec",
  "6d115af",
  "d211c9a",
  "7f59964",
  "76.512s",
  "6.150s",
  "12.44x",
  "1206.846s",
  "93.074s",
  "12.97x",
  "18m33.8s",
  "c1b9590",
  "7521cdc",
  "23f326a",
  "7,874 bodies",
  "168 modules",
  "strict Verus 81 verified / 0 errors",
  "proof tests 26/26",
  "source gate passes 28/28",
  "21/21",
  "Exact v77",
  "Process status 0",
  "415,541-byte Kernel IR V9 handoff",
  "26 GuardedStore operations",
  "103,616-byte",
  "all 12 kernels",
  "exact replay",
  "3f1a68ed30f243e640f38e90fedc483bfaf8d07f3a11578e21a8569369781f84",
  "3ce9820a870379d8e0d9e76bb98178776617a1ab132c7154d38929b1243a6d4d",
  "1dc443f1c4a22570e5a997c24518c0c9dd7cc3ef11312229bb24bad6031df120",
  "f3522e568e787ea808e47ce56e82553c3f3214b7a7b9d1a20f82889cc67fa8c2",
  "711 engine target",
  "171 doctests",
  "93 adapter",
  "28/28 source-gate",
  "7 hardware ignores",
  "exactly one 128-output window",
  "CLOCK_MONOTONIC_RAW",
  "checked-token causality",
  "page-less successor KV leasing",
  "exact K draft growth",
  "fail-closed custody",
  "independent no-blocker review",
  "resident daemon",
  "independently reviewed",
  "all 10 strict proof packages",
  "full negative/property policy",
  "no qualification receipt",
  "Qualification #7",
  "Qualification #8",
  "Clippy",
  "all 37",
  "600-second timeout",
  "SUN_LEN",
  "exact dependency TCB",
  "33/33 source-gate",
  "7,229",
  "7,922 bodies",
  "170 modules",
  "6a78aca1ed6c7fc5822cec74f636591631f8d129f0c123e760634cc3a1b510b4",
  "7200d867b34491373922bfe5399bccb66d1e6b2ee9bb99d00e1ad29175688fa4",
  "92886d7b80a20fa3d7d451fc9f212cc313f5852213d6e3328eaa2923a2cc5027",
  "on HOLD",
  "independently accepted",
  "full combined qualification has not run",
  "no daemon",
  "durability",
  "session generator store",
  "launcher",
  "distinct-UID",
  "no formal acceptance is claimed",
  "58 service tests",
  "3 ignores",
  "88 adapter tests",
  "f18ffe2",
  "04d09e0",
  "94a3d9396f3ba83e909e41e691939c35ed38d6697f43bc1dfc0a34dc812dbeef",
  "262edc61c9f1d4c9a474aac56f8749c34a74c557ab131eb06b0fbd263673362a",
  "maximum of 16",
  "11-allocation",
  "S1/K4",
  "accepted zero draft tokens",
  "correction token 3681",
  "4.509687305 seconds",
  "9.557095343 seconds",
  "14.066782648 seconds",
  "1478.66 seconds",
  "authenticated_AB=false",
  "The new private TP pool and batched driver implement persistent resident physical-page prefix reuse separately",
  "Authority: none",
  "hardware completion",
  "process status 0",
  "4.227489904 seconds",
  "15.429140005 seconds",
  "3.733883367 seconds/token",
  "17.076646692 seconds",
  "1445.15 seconds",
  "10.15 seconds above",
  "different output length",
  "uncontrolled cold variance",
  "neither speedup nor regression",
  "target-only",
  "not speculative serving",
  "benchmark_comparable=false",
  "r33_tpot_eligible=false",
  "9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa",
  "608 verified and 0 errors",
  "S1/T128",
  "NoMatch",
  "live-source",
  "audited seven-file exact kernel/host ABI delta",
  "not final or public product integration",
  "Protected infrastructure is absent",
  "symmetric memory",
  "MTP",
  "independent-review HOLD",
  "actual full warmed route",
  "settlement",
  "release",
  "explicit faulted Active+Resident close",
  "128-token acceptance",
  "129-token rejection",
  "26m48s",
  "QualificationFinalLogits(NonFinite { lane: 0, token: 0 })",
  "status 134",
  "stale 2026-08-26",
  "cannot represent current aggregate kernels",
  "engineering aggregate artifact exists",
  "target smoke can bind it",
  "obsolete seven-object artifacts",
  "no honest identity conversion",
  "aggregate technical-capture wiring",
  "protected paths stay fail-closed",
  "No current Qwen accuracy",
  "20 inputs",
  "seven numerical buckets",
  "technical prequalification only",
  "not request latency",
  "full-envelope lookup",
  "live ledger",
  "external rollback",
  "durable replay",
  "pre-existing e535/f405 DCO ancestry concern",
  "Progressing, no team blocker",
  "public fe2o3",
  "All 33 M1 exit gates remain open",
  "four target-only Qwen tokens",
  "TTFT",
  "TPOT",
  "vLLM and SGLang baselines are absent",
  "Docker permission",
  "no native",
  "Ferric owns",
  "fe2o3 owns",
];

const retiredCurrentClaims = new Set([
  "No current Qwen accuracy",
  "d211c9a",
  "7f59964",
  "415,541-byte Kernel IR V9 handoff",
  "103,616-byte",
  "3ce9820a870379d8e0d9e76bb98178776617a1ab132c7154d38929b1243a6d4d",
  "f3522e568e787ea808e47ce56e82553c3f3214b7a7b9d1a20f82889cc67fa8c2",
  "no formal acceptance is claimed",
  "r33_tpot_eligible=false",
  "pre-existing e535/f405 DCO ancestry concern",
  "1e44a64",
  "d9e9a38",
  "Process status 0",
  "3f1a68ed30f243e640f38e90fedc483bfaf8d07f3a11578e21a8569369781f84",
  "1dc443f1c4a22570e5a997c24518c0c9dd7cc3ef11312229bb24bad6031df120",
  "exactly one 128-output window",
  "checked-token causality",
  "exact dependency TCB",
  "262edc61c9f1d4c9a474aac56f8749c34a74c557ab131eb06b0fbd263673362a",
  "4.509687305 seconds",
  "9.557095343 seconds",
  "14.066782648 seconds",
  "hardware completion",
  "process status 0",
  "4.227489904 seconds",
  "15.429140005 seconds",
  "3.733883367 seconds/token",
  "17.076646692 seconds",
  "1445.15 seconds",
  "10.15 seconds above",
  "different output length",
  "uncontrolled cold variance",
  "neither speedup nor regression",
  "not speculative serving",
  "actual full warmed route",
  "settlement",
  "explicit faulted Active+Resident close",
  "128-token acceptance",
  "129-token rejection",
  "26m48s",
  "QualificationFinalLogits(NonFinite { lane: 0, token: 0 })",
  "status 134",
  "stale 2026-08-26",
  "cannot represent current aggregate kernels",
  "engineering aggregate artifact exists",
  "target smoke can bind it",
  "obsolete seven-object artifacts",
  "no honest identity conversion",
  "aggregate technical-capture wiring",
  "protected paths stay fail-closed",
  "four target-only Qwen tokens",
  "e5d23e1",
  "b10a6e0ef1b40c1e05d5320e5391f1c946bea1f4b8badcffef8cbe58124233ee",
  "7,874 bodies",
  "168 modules",
  "711 engine target",
  "93 adapter",
  "on HOLD",
  "independent-review HOLD",
  "ce9e0bc",
  "all 12 kernels",
  "all 10 strict proof packages",
  "full negative/property policy",
  "Qualification #8",
  "all 37",
  "600-second timeout",
  "SUN_LEN",
  "92886d7b80a20fa3d7d451fc9f212cc313f5852213d6e3328eaa2923a2cc5027",
  "full combined qualification has not run",
  "6a78aca1ed6c7fc5822cec74f636591631f8d129f0c123e760634cc3a1b510b4",
  "7200d867b34491373922bfe5399bccb66d1e6b2ee9bb99d00e1ad29175688fa4",
  "Protected infrastructure is absent",
  "no qualification receipt",
  "33/33 source-gate",
]);
const currentRequiredClaims = [
  "private Ferric 1fc45a52",
  "fe2o3 main at 5ed3840a",
  "494 CPU tests with one hardware test ignored",
  "367 release library tests with six ignored",
  "All builds and CPU tests run on mi300x-2",
  "All 16 requests across four modes match the independent token IDs and decoded text",
  "one excluded warmup and three measured requests in fixed arm order",
  "Baseline: TTFT 2662.66 ms, TPOT 135.85 ms, output rate 6.43 tokens/s",
  "V14 only: TPOT 134.74 ms",
  "V15 only: TPOT 86.42 ms",
  "Combined: TTFT 2280.81 ms, TPOT 84.55 ms, output rate 9.83 tokens/s",
  "not HTTP latency, sustained throughput, GPU durations, stable gains or a vendor comparison",
  "Runtime and packing changes are shared by all arms, so their separate gains are not measured",
  "Retained earlier CPU checkpoint, with its original source pins",
  "Continuous batching and paged TP attention",
  "Persistent resident radix prefixes",
  "Batched Qwen with resident prefix reuse",
  "no isolated cache-speedup claim follows",
  "The CLI admits at most 32 requests, 1-256 requested output tokens each, and 240 batches without ring rollover",
  "Prefix reuse reduces work from 50 to 34 physical token rows and six to five batched forwards",
  "113.271 s",
  "47.326 s",
  "47.802 s",
  "86.271 s",
  "37.697 s",
  "466.414s cached versus 461.166s uncached",
  "340b2bd61a8b05bbdca45f4ed9151b28acd8a2f1ee4351b7310e76ccfc456460",
  "139 library tests with one preexisting ignore",
  "Earlier planner/cursor Verus receipts do not cover the new pool",
  "Persistence means across requests, not disk or process restart",
  "All eight MI350X gfx950 devices",
  "descriptor cleanup",
  "420 unit tests, 20 integration tests, and 27 doctests",
  "TP1/2/8",
  "36 selected Verus queries with 0 errors",
  "8 rejected actual-body mutations",
  "Public fe2o3 a8b016e",
  "All 12 passed host loader closure/materialization",
  "Artifact authority remains none",
  "final Ferric a8 repin checks passed",
  "strict Verus 8 verified / 0 errors",
  "8 rejected cursor-body mutations",
  "One unwarmed TP8 sequence matches all 32 output IDs",
  "one post-first interval each",
  "not a controlled speed comparison",
  "3546d54d2c4a913f5d079701aed557d0a378bba8",
  "433 runtime tests",
  "All six synthetic GPU probes pass",
  "31 host/source tests per target",
  "648 tests with 9 ignored",
  "754-file source closure",
  "These fixtures are not full Qwen validation",
  "a real single RMSNorm dispatch",
  "no radix prefix cache in this execution profile",
  "not MI350 Qwen execution",
  "two-token TP1/2/8 Qwen smokes",
  "TP8: 32-token single-sequence result",
  "163.648 s",
  "40.260 s (mean of 31 intervals)",
  "One unwarmed sequence, no repeated-run statistics or controlled speed comparison.",
  "same twelve kernel bodies",
  "Existing MI300X Qwen32 observations remain separate and unchanged",
  ...requiredClaims.filter((claim) => !retiredCurrentClaims.has(claim)),
  "d9f6bbcd",
  "a3c941c",
  "exact Qwen input bundle",
  "14.36s TTFT",
  "3.05s TPOT",
  "invalid and noncomparable",
  "wrong GEMM layout",
  "nonsense output",
  "068e15991b97a829af6e77f2d105262e3ffc5fc69f1fa41a5ed5efdcae0da5e3",
  "6dfba0ac",
  " Paris. The capital of Italy is Rome",
  "8/8",
  "13.649661699s TTFT",
  "2.7656050044 seconds per post-first token",
  "34.094s",
  "hardware completion",
  "benchmark_comparable=false",
  "eb219f0",
  "046029d",
  "6ef opt-in",
  "63e",
  "bb50d0e",
  "7f3f235",
  "c09f212e82eace9326a6d0a0e47ff7a898a8901e5e531fa01ea52213b064b54b",
  "a6a3f2e",
  "9392755",
  "d33933adf7f5dfe0a9aa4aba0cc4cb3909b5aa35f9bfb65db7af9af4a5c5bb40",
  "33d754aaa10292fa37e974eb004b6c52067a141dcd5b58ed080023c6bf315c2d",
  "5d40961ec2fd365835867251330072674bdf16dc48a18c4c2bc446bcd60f28b8",
  "d894caf042156abf21436c98fa3de7d40af124ba7374baa0b879bf7df582af44",
  "13.442289487s",
  "2.769104887645161s",
  "31 gaps",
  "246.065540159s",
  "346.46s",
  "TTFT excludes setup",
  "std::Instant",
  "MONOTONIC_RAW",
  "cross-clock sums are approximate",
  "354 deterministic slots",
  "712-record source closure",
  "8,207 executable bodies",
  "arithmetic cardinality only",
  "not R33 authority",
  "0ea54ed",
  "15f49dd",
  "42882993",
  "528fa128e398b9aac5f5fa672388b44ff7b7e67932332abbb61d7e9704715d7a",
  "cf786f800b818a1771c32bd9aa3eb2fe8daf56c625177aa193d6406eab033804",
  "382afa968efda2b761919746e20a2e032b9bdef01ed57ce56dcc757af2ac69a5",
  "26 GuardedStore",
  "exact replay",
  "32/32",
  "296 Verus queries",
  "143 release tests",
  "seven byte-exact",
  "1.115x",
  "1.119x",
  "0.49%",
  "1.53%",
  "off by default",
  "stderr-only",
  "98.23 seconds",
  "97.313 GiB",
  "58.133 seconds",
  "three TCB",
  "source-pinned ELF",
  "matching engineering aggregate",
  "negative actual-body semantic mutations",
  "complete developer qualifier",
  "197/197",
  "711-record source-closure",
  "protected promotion",
  "both frozen",
  "32-token",
  "one prompt",
  "production-used same-shape core",
  "repeated 16-round",
  "positive allocator control",
  "636 engine tests",
  "9 hardware ignores",
  "171 doctests",
  "Integrated engine check",
  "strict Clippy",
  "K3 work-proportional KV-write",
  "no speedup",
  "kernels are authored through fe2o3",
  "The new private TP pool and batched driver implement persistent resident physical-page prefix reuse separately",
  "symmetric memory",
  "MTP",
  "all 33 M1 exit gates remain open",
  "Required before authenticated R33 serving and M1",
  "not authenticated R33",
  "No authenticated 20-window run",
  "external authority bundle",
  "authority-free aggregate 528",
  "target smoke",
  "S1/T128 capture",
  "vLLM/SGLang comparison",
];

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

if (screenshotRoot) {
  await mkdir(screenshotRoot, { recursive: true });
}

const browser = await chromium.launch({ headless: true, args: ["--disable-gpu"] });
try {
  for (const [name, width, height] of viewports) {
    const page = await browser.newPage({ viewport: { width, height } });
    const browserErrors = [];
    page.on("console", (message) => {
      if (message.type() === "error") {
        browserErrors.push(`console: ${message.text()}`);
      }
    });
    page.on("pageerror", (error) => browserErrors.push(`page: ${error.message}`));

    await page.goto(pageUrl, { waitUntil: "load" });
    await page.waitForFunction(
      (selectors) => selectors.every((selector) => document.querySelector(selector)?.children.length),
      dynamicRoots,
    );
    for (const privateCommit of [
      "2048c10",
      "c29cfed",
      "7224c33",
      "65cb435",
      "8cdde149643446b20730bc60206669c5bab1ca8d",
      "f83efafad755ae69abadb2304806415b28d44057",
      "3924efcbc93ae3c25ba649e7dc7ee8fc4dba543d",
      "59b0a9a42d3c29bf5fef29024913db1029d4c133",
    ]) {
      assert(await page.locator(`a[href*="${privateCommit}"]`).count() === 0,
        `${name}: unpublished source ${privateCommit} must not be a public commit link`);
    }

    const result = await page.evaluate((selectors) => {
      function visibleBox(element) {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return {
          display: style.display,
          visibility: style.visibility,
          opacity: Number.parseFloat(style.opacity),
          width: rect.width,
          height: rect.height,
        };
      }

      const body = visibleBox(document.body);
      const main = visibleBox(document.querySelector("main"));
      const roots = selectors.map((selector) => ({
        selector,
        children: document.querySelector(selector).children.length,
        ...visibleBox(document.querySelector(selector)),
      }));
      const currentView = document.body.cloneNode(true);
      currentView.querySelector("[data-progress]")?.remove();
      const currentText = currentView.textContent.replace(/\s+/g, " ").trim();
      const sections = [...document.querySelectorAll("main > section")].map((section) => {
        const rect = section.getBoundingClientRect();
        return { id: section.id || section.className, top: rect.top, bottom: rect.bottom };
      });
      const authorityChildOverlaps = [...document.querySelectorAll(".authority-item")]
        .map((item, index) => {
          const [tag, detail] = item.children;
          if (!tag || !detail) return null;
          const tagRect = tag.getBoundingClientRect();
          const detailRect = detail.getBoundingClientRect();
          const overlaps =
            tagRect.left < detailRect.right - 0.5 &&
            tagRect.right > detailRect.left + 0.5 &&
            tagRect.top < detailRect.bottom - 0.5 &&
            tagRect.bottom > detailRect.top + 0.5;
          return overlaps
            ? {
                index,
                tag: tag.textContent.trim(),
                tagRight: tagRect.right,
                detailLeft: detailRect.left,
              }
            : null;
        })
        .filter(Boolean);
      return {
        body,
        main,
        roots,
        currentText,
        bodyTextLength: document.body.innerText.trim().length,
        horizontalOverflow:
          Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) -
          window.innerWidth,
        sections,
        authorityChildOverlaps,
      };
    }, dynamicRoots);

    for (const [label, box] of [["body", result.body], ["main", result.main]]) {
      assert(box.display !== "none", `${name}: ${label} must not use display:none`);
      assert(box.visibility !== "hidden" && box.visibility !== "collapse", `${name}: ${label} is hidden`);
      assert(box.opacity > 0, `${name}: ${label} is transparent`);
      assert(box.width > 0 && box.height > 0, `${name}: ${label} has no rendered area`);
    }
    assert(result.bodyTextLength > 1000, `${name}: rendered body is blank or incomplete`);
    assert(result.horizontalOverflow <= 1, `${name}: page has horizontal overflow`);
    result.roots.forEach((root) => {
      assert(root.children > 0, `${name}: ${root.selector} rendered no children`);
      assert(root.display !== "none", `${name}: ${root.selector} uses display:none`);
      assert(root.visibility !== "hidden" && root.visibility !== "collapse", `${name}: ${root.selector} is hidden`);
      assert(root.opacity > 0, `${name}: ${root.selector} is transparent`);
      assert(root.width > 0 && root.height > 0, `${name}: ${root.selector} has no rendered area`);
    });
    result.sections.slice(1).forEach((section, index) => {
      const previous = result.sections[index];
      assert(
        section.top >= previous.bottom - 1,
        `${name}: section ${section.id} overlaps ${previous.id}`,
      );
    });
    assert(
      result.authorityChildOverlaps.length === 0,
      `${name}: authority legend children overlap: ${JSON.stringify(result.authorityChildOverlaps)}`,
    );
    const missingClaims = currentRequiredClaims.filter(
      (claim) => !result.currentText.includes(claim),
    );
    assert(
      missingClaims.length === 0,
      `${name}: rendered current view is missing ${missingClaims.join(", ")}`,
    );
    for (const claim of [
      /Signed (?:public fe2o3 main|fe2o3 public main|current public main).*0ea/i,
      /matching (?:0ea )?engineering aggregate (?:is underway|and combined host checks are underway)/i,
      /0ea (?:artifact|aggregate) has not (?:been )?emitted/i,
      /Ferric has not run (?:the )?32(?:-token| tokens)/i,
      /hardware execution is waiting for (?:a shared GPU|GPU availability)/i,
      /stale (?:authenticated )?S1\/K4 (?:source-)?policy/i,
      /fe2o3-production-build-config-v1/i,
      /6,854 admitted/i,
      /97 unadmitted/i,
      /7,632 executable bodies/i,
      /681 directly verified/i,
      /6,951 explicitly unverified/i,
      /0 unadmitted/i,
      /0 stale/i,
      /focused v20/i,
      /focused v26/i,
      /v21's single-SSA-local fix/i,
      /production R33 backend remain absent/i,
      /Blocked on RMSNorm barrier convergence/i,
      /current kernel obligation is RMSNorm barrier convergence/i,
      /Current exact blocker: qwen3_rmsnorm_v1/i,
      /Blocked on native LLVM worker abort/i,
      /Current exact blocker: native LLVM worker abort/i,
      /No aggregate HSACO was produced/i,
      /no thirteen-root Ferric MFMA image or GPU numerical result exists/i,
      /Ferric MFMA image still pending/i,
      /WorkgroupCount X lowering fails/i,
      /generic fe2 WorkgroupSize X lowering fails/i,
      /exact aggregate qualification (?:is )?(?:complete|qualified|green)/i,
      /(?:current|qualified|available) aggregate HSACO (?:is )?(?:ready|accepted|published|available)/i,
      /production paired-prefill executor (?:is )?(?:complete|ready|running)/i,
      /Qwen serving (?:is )?(?:ready|complete|running)/i,
      /vLLM\/SGLang comparison (?:is )?(?:complete|passed|green)/i,
      /CurrentFerricDescriptorRoster/i,
      /before KFD/i,
      /GPU and VRAM state are unchanged/i,
      /No Qwen token/i,
      /No hardware execution/i,
      /unintegrated engineering harness/i,
      /(?:combined exact|exact combined) qualification #5 (?:is )?(?:complete|qualified|green)/i,
      /(?:combined exact|exact combined) qualification #6 (?:is )?(?:complete|qualified|green)/i,
      /qualification receipt (?:was )?(?:emitted|available)/i,
    ]) {
      assert(!claim.test(result.currentText), `${name}: rendered current view overclaims open work: ${claim}`);
    }
    assert(!result.currentText.includes("57d2d9c"), `${name}: historical pin leaked into current rows`);
    assert(!/\bselected fe2o3 pin\b/i.test(result.currentText), `${name}: rendered a selected-pin claim`);
    assert(!/\bcurrent (?:fe2o3 )?(?:pin|dependency)\b/i.test(result.currentText), `${name}: rendered a current-dependency claim`);
    assert(browserErrors.length === 0, `${name}: ${browserErrors.join("; ")}`);

    for (const [id, expected] of [
      ["fixed-safe-v10-progress", "On one MI350X on mi350-2, all 36 cells pass: ten finite cases in each of two activation epochs and 16 timing cells in A/B/B/A order. All seven full-buffer and guard records pass, including final timing checks, with clean unsignaled worker and supervisor teardown. R3 shows no useful measured gain and is not promoted."],
      ["fixed-safe-v10-scope", "This is a cache-hot synthetic 12288x4096 gate/up pair, not Qwen inference. Both arms use compiler/SDK 1a5999f6 plus the same capture patch and the historical 5e2/dd6 runtime. Each timing cell has a fresh worker, two excluded warmup groups and 32 measured groups, for 512 retained samples. One-pair and five-pair groups, activation epochs and AB/BA orders remain separate; inner groups are not independent worker repetitions."],
      ["fixed-safe-v10-measurement", "The metric is instrumented controller wall per cache-hot gate/up pair, in microseconds, with prepacking and uploads excluded. Worker and controller scopes overlap and cannot be added or subtracted to infer GPU time. Positive improvement means lower candidate latency. The final 206.882 us baseline is retained, not discarded or pooled away."],
      ["fixed-safe-v10-runtime", "Every 32-group timing cell records 321 operational-currentness checks. Their overlapping host scope records about 384-387 us per pair with one pair per group and 77-79 us with five; measured transfer and kernel-admission deltas are zero. Token programs are already integrated and measured on historical 807f0: ordinary/token TPOT is 60.702/55.296 ms in AB and 54.161/55.297 ms in BA, with no repeatable gain. Latest source review indicates the same minimum ten currentness checks per backend group for token and matched ordered64 paths, plus five outer token checks; these are source counts, not measured token counters. Next priorities are latest-client qualification and measured group utilization/batching analysis, not a demonstrated GPU bottleneck or model gain."],
      ["fixed-safe-v10-next", "Separate R4 finite-check-hoist CPU validation passes 36 tests per feature mode and strict Clippy, plus 16 native-harness and 11 launcher fixtures. Emission a002 succeeds and actual ISA/ELF metadata are captured; independent static review accepts the finite-only diagnostic, with no completed R3-versus-R4 native result recorded here. The separately built 4fb8ae50 core worker passes 106 library and six CLI tests. Latest client source and wire migration remain CPU-unqualified, with the lock not regenerated and full-model qualification pending."],
      ["fixed-safe-v10-limits", "Finite parity is not invalid-input or numerical-trap qualification. This screen establishes no GPU duration, TTFT/TPOT, serving throughput, stable gain, TP8 or vendor comparison. No default or model promotion follows. Historical HTTP results remain separate and unchanged, and all 33 M1 gates remain open."],
    ]) {
      const paragraph = page.locator("#" + id);
      assert(await paragraph.count() === 1 && await paragraph.isVisible(), `${name}: fixed-safe claim missing: ${id}`);
      assert((await paragraph.innerText()).replace(/\s+/g, " ").trim() === expected,
        `${name}: fixed-safe scope or qualification changed: ${id}`);
    }
    const fixedSafeV10Details = page.locator("#fixed-safe-v10-evidence");
    assert(await fixedSafeV10Details.count() === 1, `${name}: fixed-safe evidence must be unique`);
    assert((await fixedSafeV10Details.locator("summary").innerText()).trim()
      === "Fixed-safe R3: all eight comparisons and evidence", `${name}: fixed-safe disclosure label changed`);
    await fixedSafeV10Details.locator("summary").click();
    const fixedSafeV10Table = fixedSafeV10Details.locator("#fixed-safe-v10-results");
    const fixedSafeV10Rows = [
      ["1-0-AB", ["1", "0", "AB", "624.215", "623.537", "+0.1086%"]],
      ["1-0-BA", ["1", "0", "BA", "624.504", "624.165", "+0.0543%"]],
      ["1-1-AB", ["1", "1", "AB", "626.242", "625.710", "+0.0851%"]],
      ["1-1-BA", ["1", "1", "BA", "623.300", "625.341", "-0.3275%"]],
      ["5-0-AB", ["5", "0", "AB", "217.030", "217.230", "-0.0920%"]],
      ["5-0-BA", ["5", "0", "BA", "218.166", "218.418", "-0.1153%"]],
      ["5-1-AB", ["5", "1", "AB", "218.988", "219.872", "-0.4037%"]],
      ["5-1-BA", ["5", "1", "BA", "206.882", "217.991", "-5.3698%"]],
    ];
    const fixedSafeV10ActualRows = await fixedSafeV10Table.locator("tbody tr").evaluateAll((rows) => rows.map((row) => [
      row.dataset.fixedSafeV10Row, Array.from(row.querySelectorAll("th, td"), (cell) => cell.textContent.trim()),
    ]));
    const fixedSafeV10Header = await fixedSafeV10Table.locator("thead th").allTextContents();
    assert(JSON.stringify(fixedSafeV10Header.map((text) => text.trim()))
      === JSON.stringify(["Pairs/group", "Epoch", "Order", "Baseline (us/pair)", "R3 (us/pair)", "Improvement"]),
    `${name}: fixed-safe table units changed`);
    assert(JSON.stringify(fixedSafeV10ActualRows) === JSON.stringify(fixedSafeV10Rows),
      `${name}: fixed-safe eight unpooled comparisons changed`);
    assert((await fixedSafeV10Table.locator("caption").innerText()).trim()
      === "Instrumented controller wall per cache-hot gate/up pair; positive improvement means lower candidate latency.",
    `${name}: fixed-safe measurement caption changed`);
    const fixedSafeV10Identities = [
      ["R3 native results archive SHA-256", "ceed68468e5b30e30dc64fa89b9cdf79cc3cc421b9703fe5141a8a8bf352a5c0"],
      ["Compiler and SDK source", "1a5999f6e1c5f2363bc2d525af65e84c46502ce6"],
      ["Baseline image SHA-256", "fdffa040ad94723460a0891cfb5be22be25cd90c440008067f81bde37fed453e"],
      ["R3 image SHA-256", "4f6ebd3d2294f20ece54a40b20a040129363886fb05b6b7e512bdd9bce447f89"],
      ["Historical runtime worker SHA-256", "dd6bd3b4a910478e85d2f3530be25819153fad1f08bf33bb14dd534514a601a2"],
    ];
    assert(await fixedSafeV10Details.locator("dt").count() === fixedSafeV10Identities.length,
      `${name}: fixed-safe evidence roster changed`);
    for (const [label, identity] of fixedSafeV10Identities) {
      const term = fixedSafeV10Details.locator("dt").filter({ hasText: new RegExp("^" + label + "$") });
      assert(await term.count() === 1 && (await term.locator("xpath=following-sibling::dd[1]").innerText()).trim() === identity,
        `${name}: fixed-safe evidence identity changed: ${label}`);
    }
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1),
      `${name}: expanded fixed-safe evidence overflows the page`);
    if (screenshotRoot && (name === "desktop" || name === "mobile")) {
      await fixedSafeV10Details.screenshot({ path: join(screenshotRoot, `${name}-fixed-safe-v10-expanded.png`) });
    }
    await fixedSafeV10Details.locator("summary").click();

    for (const [id, expected] of [
      ["september23-progress", "Four native starts match all 2,048 output IDs and decoded bytes. Candidate TTFT is 72.306% lower in AB and 69.447% lower in BA; TPOT is 7.575% higher in AB and 8.130% higher in BA. Finite-window output rate is 8.857% and 7.908% higher, respectively."],
      ["prefill32-v7-scope", "Qwen3-8B on mi350, TP1/C1, 128/128 tokens, context 8192, BF16 with an FP32 head; prefix caching and speculation off. Each start excludes one warmup and measures three requests. Both use chunk32, the same controller, dd6 worker built from 5e2 and nine images; only V5 versus two V27 prefill copies changes. V19 decode stays fixed."],
      ["prefill32-v7-limits", "AB and BA remain separate. This native screen is not HTTP, sustained throughput, a stable gain or a vendor win. Defaults remain unchanged."],
      ["prefill32-v7-parity", "The separate isolated prefill-copy prerequisite passes all 20 native byte-and-guard parity cells on mi350, with clean unsignaled worker and supervisor exits. That parity check is not a timing result."],
      ["packet-ticks-v7-progress", "The opt-in ordered64 diagnostic has accepted CPU qualification with 2,585 passing Rust executions. Native replay matches all 128 output IDs and decoded bytes, retaining 87,711 packet records. Independent raw-interval and custody reviews pass. Raw ticks are uncalibrated, not nanoseconds, shader-only time, additive time shares or a speedup. No new vendor comparison is established."],
      ["packed-gate-up-v7-progress", "The fresh 33-cell campaign passes all 65,536 packing patterns, 16 finite-parity cells and 16 timing cells. Four separate ABBA cohorts observe 40.0% to 42.6% lower controller wall time, a 1.67 to 1.74x baseline/candidate ratio, including activation packing."],
      ["packed-gate-up-v7-currentness", "This is a cache-hot synthetic 12288x4096 gate/up pair, using historical compiler/SDK 413ba987 and the 5e2/dd6 worker. One-pair and five-pair groups, each in two queue epochs, stay separate. Earlier interrupted attempts remain failures; the historical binaries are not latest-qualified."],
      ["packed-gate-up-v7-limits", "This is not GPU time, model TTFT/TPOT or serving throughput. Numerical-trap and model qualification remain open. No vLLM win is claimed; prior vendor results, defaults and all 33 open M1 gates remain unchanged."],
      ["runtime-wait-v8-progress", "Default and active-poll wait arms each pass 210 dependent packets and 528 guarded readbacks. The active arm completes all six groups without fallback. These are synthetic correctness and accounting checks, not a measured latency gain."],
      ["runtime-token-v8-progress", "The separate token-program smoke passes 195 dependent packets across reuse, release and queue rollover, plus exact default-route and late-pointer rejections. All owned workers are reaped without signals; expected fatal exits are not clean exits. Zero GPU publication after the late error is not directly observed."],
      ["client-token-v8-progress", "An independently audited V4 CPU receipt covers all seven feature families and 32 test targets: five fresh R13 token-binary targets plus 27 explicitly inherited R12 targets. It records 5,168 passing test executions, not unique tests; 278 hardware/authored ignores and seven instances of the existing library exclusion remain disclosed. Compiler dependency and generated-input checks justify inheritance; R13 changes only one test file, with 1,320 other source files unchanged. R13's 14 production-check/strict-Clippy commands, diagnostic aggregate, both 52-test policy gates, all 10 actual-image checks and actual ABI producer/consumer pass. The stale test assertion omitted 290 required zero-extent RMSNorm fixups: 2,569 total equals 2,279 nonempty plus 290 empty fixups. R13 compiled both controllers successfully but failed retention; R14 independently copied the same binaries and completed final collection without a Rust source change or rebuild. The qualified 21-file client was integrated behind the default-off 652-packet token-program opt-in. R7 native-harness qualification passes all 24 CPU fixtures; both controllers pass cold parity and native ABBA. The fresh matched HTTP pair passes; no repeatable token-program gain, sustained-serving qualification or default promotion is established."],
      ["runtime-publication-v8-progress", "Runtime source 807f0bef70da75c81e56de6eb4fd6e9c1f78e5e2 was published to fe2o3 main with [skip ci]. Historical main 89c8c899 retained all nine runtime trees and 762 closure paths. Latest observed main 1f4d83c4 (September 26 UTC) retains the same 896 mapped inputs and complete runtime closure as fd1b32e8. Compared with qualified 807, eight of nine runtime trees and 759 prior paths remain unchanged; three paths are modified and two added, for 764 current paths. Its conditional gfx942 direct-dispatch path leaves existing gfx950 worker, token-program, ordered-batch and wait source files unchanged. Only 891 of 896 broadly mapped files match 807. Runtime source equivalence no longer holds. The measured worker/client qualification is exact 807, not a rebuild or native qualification of 1f4d83c4 and not latest-compiler qualification. Historical binaries retain their original identities; adopting newer runtime code requires fresh qualification."],
      ["runtime-token-v8-limits", "Synthetic runtime checks alone establish no model gain. The fresh single HTTP pair leaves Ferric 11.752 times slower than vLLM by mean TPOT; the historical three-pair series retains its original 15.100 to 18.157 range. Defaults and all 33 M1 gates remain unchanged."],
      ["matched-http-v8-progress", "One fresh Ferric-to-vLLM 0.28.0 HTTP pair passes on September 26 UTC. Ferric mean TTFT is 890.522 ms versus 19.567 ms; mean TPOT is 51.443 ms versus 4.377 ms; finite output rate is 17.240 versus 222.219 tokens/s. Ferric remains 11.752 times slower by mean TPOT. All 60 measured requests succeed."],
      ["matched-http-v8-scope", "Each engine uses ten excluded warmups, thirty measured requests and two untimed exact-output diagnostics: Qwen3-8B on mi350 GPU0, TP1/C1, 128/128 tokens, context 8192, BF16 decoder with explicit FP32 output head, greedy fixed length, prefix caching and speculation off. Ferric retains the qualified 807 worker/client and opt-in token program, not latest-SDK qualification."],
      ["matched-http-v8-limits", "HTTP TTFT is client-send to first nonempty text; TPOT is first-to-last text divided by 127, not true per-token interarrival latency. Output rate is 3,840 tokens over the finite measured cohort including gaps and drain, not sustained loaded throughput. This is one fixed-order pair, not repeated-start confidence, a stable tail, a stock BF16-head ranking, TP8 or a speculative comparison. Historical three-pair and native-ingress measurements are not pooled with it."],
      ["token-model-v8-progress", "Both cold paths match all 256 token IDs and decoded bytes. Four native ABBA starts match all 2,048 outputs including warmups, with clean unsignaled teardown. Ordinary/token TPOT is 60.702/55.296 ms in AB and 54.161/55.297 ms in BA: the candidate is 8.9% lower in one order but 2.1% higher in reverse. Baseline drift prevents a repeatable token-program speedup claim."],
      ["token-model-v8-limits", "Each native start excludes one warmup and measures three requests. These are controller-ingress timings, not HTTP or GPU-only duration; the fixed prompt reaches context 255 and does not qualify the 256-to-257 fallback transition, loaded serving or TP8. Both controllers and the worker keep their actual 807-qualified provenance."],
      ["packed-model-v8-progress", "The default-off standalone packed gate/up route is integrated across exactly 20 files. R5 fast qualification passes all fourteen diagnostic commands, 44 focused tests and 53 policy tests after four test-only cfg guards repair the retained R4 failure. All 26 draft native-harness fixtures pass with fake controllers and clean temporary-directory cleanup. The all-fresh 36-target full CPU matrix, real packed model parity, cold/ABBA timing and HTTP comparison remain open. The ordinary 688-dispatch packed route is separate from the fixed 652-dispatch token program; no packed model speedup or default promotion is claimed."],
    ]) {
      const paragraph = page.locator("#" + id);
      assert(await paragraph.count() === 1 && await paragraph.isVisible(), `${name}: current progress claim missing: ${id}`);
      assert((await paragraph.innerText()).replace(/\s+/g, " ").trim() === expected,
        `${name}: current progress scope, regression or qualification state changed: ${id}`);
    }
    const prefill32V7Disclosure = page.getByText("Prefill32 native screen and isolated parity: evidence", { exact: true });
    const prefill32V7Details = page.locator("#prefill32-v7-identities");
    assert(await prefill32V7Details.count() === 1, `${name}: September 23 evidence must be unique`);
    await prefill32V7Disclosure.click();
    assert(await prefill32V7Details.locator("dt").count() === 4, `${name}: September 23 evidence roster changed`);
    for (const [label, digest] of [
      ["Prefill32 native report SHA-256", "5ef8c2f2e5a313e16c69401a72f633b9539afcee285d407fe94b65843693bfdc"],
      ["Prefill32 native supervisor SHA-256", "3443cba3e4030e19900e50d4e0875c14eb1a53a6cd3ade10c33503787a56853f"],
      ["Isolated prefill-copy parity report SHA-256", "d8a5df743cd9d30ca714d02e8aa7a24774b36067bda80afbbe3c633deb825508"],
      ["Isolated prefill-copy parity supervisor SHA-256", "3a381a2e184f9f9a1295984821da2f7882b235969732ee453bcf7d3c3397c277"],
    ]) {
      const term = prefill32V7Details.locator("dt").filter({ hasText: new RegExp("^" + label + "$") });
      assert(await term.count() === 1 && (await term.locator("xpath=following-sibling::dd[1]").innerText()).trim() === digest,
        `${name}: September 23 evidence changed: ${label}`);
    }
    await prefill32V7Disclosure.click();
    const runtimeV8Details = page.locator("#runtime-v8-identities");
    assert(await runtimeV8Details.count() === 1, `${name}: runtime evidence must be unique`);
    await runtimeV8Details.locator("summary").click();
    const runtimeV8Identities = [
  [
    "Packed synthetic report SHA-256",
    "7e3e71e09202d32e6192614af0fce90a3f43dd46c307c3c8788d1c6c2d0079ec"
  ],
  [
    "Wait native report SHA-256",
    "592d5b1b21c24b3a97984dcc673f1981c14a8feb11974fe88d50fefaf380b1a2"
  ],
  [
    "Token native report SHA-256",
    "77b77a3065af3a52ad15bc66933c7bfabf814d481e4f1e81a2151db10b102484"
  ],
  [
    "Initial client CPU custody SHA-256",
    "b59a5339b1318d948d97a1070c3d3998115f736ce9d3faf7952094e042e8c958"
  ],
  [
    "Fresh HTTP pair summary SHA-256",
    "8ea42b053b11716d805b7865fed98279163da959b8d43903d0b75d394d5a37f3"
  ],
  [
    "Token model ABBA report SHA-256",
    "34ef4b4e3eff4e55c023c2b656b90f102b17964bac78a4d1d2e6b183ec4d4ab7"
  ],
  [
    "Packed draft fixture custody SHA-256",
    "7522d19a9c1a84940512d4810de6c15be0056cf30e2ff9ba6ffcd35cb9892d82"
  ]
];
    assert(await runtimeV8Details.locator("dt").count() === runtimeV8Identities.length,
      `${name}: runtime evidence roster changed`);
    for (const [label, digest] of runtimeV8Identities) {
      assert(await runtimeV8Details.getByText(label, { exact: true }).count() === 1
        && await runtimeV8Details.getByText(digest, { exact: true }).count() === 1,
        `${name}: runtime evidence changed: ${label}`);
    }
    await runtimeV8Details.locator("summary").click();
    const kvCopyV7Campaigns = [["j1","Campaign J1: first screen","kv-copy-v7-results","J1 KV-copy native AB and BA screen",[["baseline-AB","Baseline AB","807.243","64.455","14.232567"],["candidate-AB","KV copy AB","809.845","52.820","17.024785"],["candidate-BA","KV copy BA","827.309","52.755","17.003771"],["baseline-BA","Baseline BA","807.647","63.878","14.348660"]]],["j2","Campaign J2: independent repeat","kv-copy-v7-repeat-results","J2 KV-copy native AB and BA screen",[["baseline-AB","Baseline AB","944.284","77.743","11.831300"],["candidate-AB","KV copy AB","858.399","59.668","15.171775"],["candidate-BA","KV copy BA","852.170","59.939","15.121163"],["baseline-BA","Baseline BA","892.749","71.798","12.785176"]]]];
    const kvCopyV7Tables = [];
    for (const [campaign, title, id, region, rows] of kvCopyV7Campaigns) {
      const table = page.getByRole("region", { name: region, exact: true });
      kvCopyV7Tables.push(table);
      assert(await page.getByRole("heading", { name: title, exact: true }).count() === 1,
        `${name}: KV-copy campaign heading changed`);
      assert(await table.count() === 1 && await table.isVisible(), `${name}: KV-copy table must be unique and visible`);
      assert(await table.locator("tbody tr").count() === 4, `${name}: each KV-copy campaign requires four native starts`);
      assert(await table.locator("#" + id).getAttribute("data-kv-copy-v7-campaign") === campaign,
        `${name}: KV-copy campaign binding changed`);
      assert(JSON.stringify(await table.locator("thead th").allTextContents())
        === JSON.stringify(["Native start", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]),
      `${name}: KV-copy columns changed`);
      assert(JSON.stringify(await table.locator("tbody tr").evaluateAll((values) => values.map((row) => row.dataset.kvCopyV7Arm)))
        === JSON.stringify(rows.map((row) => campaign + "-" + row[0])), `${name}: KV-copy AB/BA order changed`);
      for (const [arm, ...expected] of rows) {
        const row = table.locator('[data-kv-copy-v7-arm="' + campaign + "-" + arm + '"]');
        assert(await row.count() === 1 && JSON.stringify((await row.locator("th, td").allTextContents()).map((value) => value.trim()))
          === JSON.stringify(expected), `${name}: KV-copy values changed: ${campaign}/${arm}`);
      }
      const geometry = await table.evaluate((region) => {
        const table = region.querySelector("table");
        const box = region.getBoundingClientRect();
        const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
          const range = document.createRange();
          range.selectNodeContents(cell);
          const text = range.getBoundingClientRect();
          const rect = cell.getBoundingClientRect();
          return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
        }).map((cell) => cell.textContent.trim());
        return { left: box.left, right: box.right, width: box.width, height: box.height, clipped,
          tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
      });
      assert(geometry.width > 0 && geometry.height > 0 && geometry.left >= -1
        && geometry.right <= width + 1 && geometry.clipped.length === 0, `${name}: KV-copy table clips text or viewport`);
      if (geometry.tableWidth > geometry.clientWidth + 1) {
        assert(["auto", "scroll"].includes(geometry.overflowX), `${name}: KV-copy table must scroll inside its region`);
      }
    }
    for (const [id, expected] of [["september22-progress","Two independent ordered64 KV-copy native screens match all 4,096 output IDs and decoded bytes. J1 TPOT is 18.051% lower in AB and 17.413% lower in BA; J2 is 23.250% and 16.516% lower. J1 TTFT is 0.322% and 2.435% higher; J2 TTFT is lower, with absolute timing drift between campaigns. These small native screens are not HTTP, a vendor comparison or a stable win. Defaults are unchanged. All 33 M1 gates remain open."],["kv-copy-v7-scope","September 22: Qwen3-8B on mi350, TP1/C1, 128 input and 128 output tokens, context 8192, BF16 checkpoint with an explicit FP32 head. Prefix caching and speculation are off. Two independent campaigns each use four fresh starts: AB runs baseline then candidate, BA reverses the order. Each start excludes one warmup and measures three requests: eight starts, 24 measured requests and eight excluded warmups in total. Both campaigns use the same controller, dd6 worker built from 5e2, and nine images. Only baseline versus parallel-c1-v19 KV append changes. Ordered64, parallel-prefill16-v27, split8-v21 and baseline partial GEMV remain fixed."],["kv-copy-v7-outcome","J1 candidate TPOT is 18.051% lower in AB and 17.413% lower in BA. Finite-window ingress output rate is 19.619% and 18.504% higher; TTFT is 0.322% and 2.435% higher, respectively. The J1 BA TTFT regression is retained."],["kv-copy-v7-repeat-outcome","J2 candidate TPOT is 23.250% lower in AB and 16.516% lower in BA. Finite-window ingress output rate is 28.234% and 18.271% higher; TTFT is 9.095% and 4.545% lower, respectively."],["kv-copy-v7-drift","Absolute timings drift between campaigns: baseline mean TPOT is 63.878 to 64.455 ms in J1 and 71.798 to 77.743 ms in J2; candidate mean TPOT is 52.755 to 52.820 ms in J1 and 59.668 to 59.939 ms in J2. Campaigns and orders remain separate; no samples are pooled or dropped. These descriptive native measurements are not HTTP latency, sustained throughput, GPU time, confidence bounds or a stable gain."],["kv-copy-v7-correctness","All 32 requests, including eight excluded warmups, match all 4,096 reference output IDs and decoded bytes. Each request retains 135 model batches and 87,711 packets. All eight controller starts exit 0; worker closure is separately confirmed by all_workers_exited metadata. Reserved inner groups are absent after cleanup; their receipts retain TERM and KILL flags as sent, not signal-free teardown. Both outer supervisors exit 0 without TERM or KILL."],["kv-copy-v7-limits","Both campaigns are retained separately. No new HTTP or vendor comparison is established; the earlier vLLM numbers below are unchanged. This is not a vLLM or SGLang win, a TP8 or speculative-decoding result, serving qualification or a default change. All 33 M1 gates remain open."]]) {
      const paragraph = page.locator("#" + id);
      assert(await paragraph.count() === 1 && await paragraph.isVisible(), `${name}: KV-copy paragraph missing: ${id}`);
      assert((await paragraph.innerText()).replace(/\s+/g, " ").trim() === expected, `${name}: KV-copy claim changed: ${id}`);
    }
    const kvCopyV7Disclosure = page.getByText("Both KV-copy native campaigns: evidence", { exact: true });
    const kvCopyV7Details = page.locator("#kv-copy-v7-identities");
    assert(await kvCopyV7Details.count() === 1, `${name}: KV-copy evidence must be unique`);
    await kvCopyV7Disclosure.click();
    for (const [label, digest] of [["J1 KV-copy native report SHA-256","ce74369df5b5ed52721f2ad043d1e0828060494bd7bfebf6da7fa5de9359bb68"],["J1 KV-copy supervisor SHA-256","9ed41c54a927adc53426342f3b3563703138a4507c152dc33772903f675caeee"],["J2 KV-copy native report SHA-256","55c1455bea17de55ca52aaaaa1ceb5b8bbe8f2d230f3c79ad423c4a0c22d6634"],["J2 KV-copy supervisor SHA-256","7a02e34c290c21cd932debb61bd37baf3e3c93bad8e34e69ab14a58e38b05479"]]) {
      const term = kvCopyV7Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: KV-copy evidence changed: ${label}`);
    }
    await kvCopyV7Disclosure.click();

    const september22Table = page.getByRole("region", { name: "Ordered64 HTTP three accepted pairs", exact: true });
    assert(await september22Table.count() === 1 && await september22Table.isVisible(), `${name}: September 22 table must be unique and visible`);
    assert(await september22Table.locator("tbody tr").count() === 3, `${name}: ordered64 HTTP requires three accepted pairs`);
    assert(JSON.stringify(await september22Table.locator("thead th").allTextContents())
      === JSON.stringify(["Pair", "Ferric TPOT (ms)", "vLLM TPOT (ms)", "Ferric / vLLM"]), `${name}: ordered64 HTTP columns changed`);
    const september22Rows = [
      ["pair1", ["Pair 1", "63.833", "4.228", "15.100"]],
      ["pair2", ["Pair 2", "79.535", "4.380", "18.157"]],
      ["pair3", ["Pair 3", "67.801", "4.394", "15.432"]],
    ];
    assert(JSON.stringify(await september22Table.locator("tbody tr").evaluateAll((rows) => rows.map((row) => row.dataset.ordered64HttpPair)))
      === JSON.stringify(september22Rows.map((row) => row[0])), `${name}: ordered64 HTTP pair order changed`);
    for (const [pair, expected] of september22Rows) {
      const row = september22Table.locator(`[data-ordered64-http-pair="${pair}"]`);
      assert(await row.count() === 1 && JSON.stringify((await row.locator("th, td").allTextContents()).map((s) => s.trim()))
        === JSON.stringify(expected), `${name}: ordered64 HTTP ${pair} values changed`);
    }
    for (const [id, claims] of [
      ["september22-http-progress", ["all three alternating pairs", "15.100 to 18.157 times slower than vLLM", "single-cold V7 diagnostic", "TTFT 818.53 ms and TPOT 64.64 ms", "not GPU time", "No speedup is claimed", "All 33 M1 gates remain open"]],
      ["matched-ordered64-http-scope", ["September 22", "Qwen3-8B on mi350, TP1/C1", "128 input and 128 output tokens", "context 8192", "BF16 checkpoint with an explicit FP32 head", "Prefix caching and speculation are off", "All six starts and all three alternating paired replays pass", "exact-output diagnostics, timing replay and owned cleanup"]],
      ["matched-ordered64-http-outcome", ["does not beat vLLM in any pair", "16.229 times, not a pooled latency or confidence bound", "first-to-last text span divided by 127", "not true token interarrival latency", "not competitiveness, stable-tail, sustained-throughput or framework-win qualification"]],
      ["v7-host-diagnostic-result", ["One cold instrumented request", "128 output token IDs and decoded bytes match", "135 model batches, 87,711 dispatches and clean exit", "TTFT is 818.53 ms and TPOT is 64.64 ms", "not a new vendor comparison or a matched HTTP result"]],
      ["v7-host-diagnostic-intervals", ["Mean decode batch wall is 64.626 ms", "58.473 ms (90.48%)", "0.171 ms (0.265%)", "includes GPU work, polling and fences", "excludes preparation/staging", "is not GPU time", "Phase scopes overlap and cannot be added as kernel costs"]],
      ["v7-host-diagnostic-limits", ["All 127 decode batches are retained", "seven slower final batches", "Instrumentation can perturb execution", "No speedup is claimed", "defaults are unchanged and all 33 M1 gates remain open", "No SGLang, TP8 or speculative-decoding performance result"]],
      ["september22-history", ["earlier HTTP R3 remains a separate rejected, incomplete series", "not repaired, replaced or relabeled", "retain their original scope"]],
    ]) {
      const paragraph = page.locator(`#${id}`);
      assert(await paragraph.count() === 1 && await paragraph.isVisible(), `${name}: September 22 paragraph missing: ${id}`);
      const text = (await paragraph.innerText()).replace(/\s+/g, " ");
      for (const claim of claims) assert(text.includes(claim), `${name}: September 22 rendered claim changed: ${claim}`);
    }
    const september22Geometry = await september22Table.evaluate((region) => {
      const table = region.querySelector("table");
      const box = region.getBoundingClientRect();
      const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const rect = cell.getBoundingClientRect();
        return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return { left: box.left, right: box.right, width: box.width, height: box.height, clipped,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
    });
    assert(september22Geometry.width > 0 && september22Geometry.height > 0 && september22Geometry.left >= -1
      && september22Geometry.right <= width + 1 && september22Geometry.clipped.length === 0, `${name}: September 22 table clips text or viewport`);
    if (september22Geometry.tableWidth > september22Geometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(september22Geometry.overflowX), `${name}: September 22 table must scroll inside its region`);
    }
    const september22Disclosure = page.getByText("September 22 comparison and diagnostic evidence", { exact: true });
    const september22Details = page.locator("#september22-identities");
    assert(await september22Details.count() === 1, `${name}: September 22 evidence must be unique`);
    await september22Disclosure.click();
    for (const [label, digest] of [
      ["Ordered64 three-pair series SHA-256", "579c4abeda3f56ca2a77ec1e1f63d37c0961cec92381f90da6a368d79ab3a3ec"],
      ["V7 host diagnostic report SHA-256", "3ddeaf566cffd14d4503a5078de7f87d6a99c0da4227ffd9cf25606f622998fe"],
    ]) {
      const term = september22Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: September 22 evidence changed: ${label}`);
    }
    await september22Disclosure.click();

    const httpR3 = page.getByRole("region", { name: "HTTP R3 accepted individual pairs", exact: true });
    assert(await httpR3.count() === 1 && await httpR3.isVisible(), `${name}: HTTP R3 table must be unique and visible`);
    assert(await httpR3.locator("tbody tr").count() === 4, `${name}: HTTP R3 requires four accepted engine rows only`);
    assert(JSON.stringify(await httpR3.locator("thead th").allTextContents())
      === JSON.stringify(["Pair and engine", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]), `${name}: HTTP R3 columns changed`);
    const httpR3Rows = [
      ["pair1-ferric", ["Pair1: Ferric composed", "810.928", "69.587", "13.265065"]],
      ["pair1-vllm", ["Pair1: vLLM 0.28.0", "20.148", "4.367", "222.524242"]],
      ["pair2-ferric", ["Pair2: Ferric composed", "811.404", "69.186", "13.335104"]],
      ["pair2-vllm", ["Pair2: vLLM 0.28.0", "18.835", "4.350", "223.905213"]],
    ];
    assert(JSON.stringify(await httpR3.locator("tbody tr").evaluateAll((rows) => rows.map((row) => row.dataset.httpR3Arm)))
      === JSON.stringify(httpR3Rows.map((row) => row[0])), `${name}: HTTP R3 row order changed`);
    for (const [arm, expected] of httpR3Rows) {
      const row = httpR3.locator(`[data-http-r3-arm="${arm}"]`);
      assert(await row.count() === 1 && JSON.stringify((await row.locator("th, td").allTextContents()).map((s) => s.trim()))
        === JSON.stringify(expected), `${name}: HTTP R3 ${arm} values changed`);
    }
    const httpR3Text = await page.locator("#matched-http-r3-title").evaluate((heading) => {
      const parts = [];
      for (let node = heading; node && node.id !== "matched-v22-http-title"; node = node.nextElementSibling) {
        if (!node.matches("details")) parts.push(node.innerText);
      }
      return parts.join(" ").replace(/\s+/g, " ").trim();
    });
    for (const claim of [
      "two pairs accepted, series rejected", "TP1/C1, 128 input and 128 output tokens",
      "context 8192, BF16 decoder and FP32 output head", "speculation off and prefix caching off",
      "10 warmups, measures 30 requests and checks two untimed output diagnostics",
      "Pair1 runs Ferric then vLLM; Pair2 runs vLLM then Ferric",
      "15.93 times vLLM in Pair1 and 15.91 times in Pair2", "40.25 and 43.08 times vLLM",
      "not a pooled result or confidence estimate", "Ferric does not beat vLLM in either accepted pair",
      "unchanged manifest declares three pairs", "Pair3 Ferric was rejected",
      "foreign process appeared", "timing is not admitted", "An owned command required forced cleanup; cleanup completed",
      "Pair3 vLLM was never started",
      "No accepted Pair3 summary exists", "aggregator returned rejected-series",
      "competitiveness and framework-win flags false", "no replacement, splicing, aggregate spread or completed-series claim",
      "client send to first nonempty text", "divided by 127, not true token interarrival latency",
      "3,840 tokens", "including request gaps and drain but excluding warmups and diagnostics",
      "not sustained throughput", "No confidence interval, stable-tail, equal-p99-SLO or release qualification",
      "all 42 requests against full token IDs and decoded bytes", "exact token IDs only in two untimed diagnostics",
      "40 timed requests match decoded text and usage", "Accepted pairs pass owned cleanup and GPU-idle postflight",
      "sampled every 500 ms, not continuously proven", "rejected third attempt remains preserved",
      "Composition R6 controller, rebuilt 5e2 worker and seven fixed images",
      "packed16-v22, split8-v21 and parallel-prefill16-v27 enabled", "worker is uninstrumented",
      "not an independent model-file rehash or source-to-binary authentication", "not a stock BF16-head comparison",
      "differences do not isolate individual optimization gains", "No SGLang, TP8, speculative-decoding",
    ]) assert(httpR3Text.includes(claim), `${name}: HTTP R3 scope missing: ${claim}`);
    const httpR3Geometry = await httpR3.evaluate((region) => {
      const table = region.querySelector("table");
      const box = region.getBoundingClientRect();
      const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const rect = cell.getBoundingClientRect();
        return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return { left: box.left, right: box.right, width: box.width, height: box.height, clipped,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
    });
    assert(httpR3Geometry.width > 0 && httpR3Geometry.height > 0 && httpR3Geometry.left >= -1
      && httpR3Geometry.right <= width + 1 && httpR3Geometry.clipped.length === 0, `${name}: HTTP R3 text/viewport geometry`);
    if (httpR3Geometry.tableWidth > httpR3Geometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(httpR3Geometry.overflowX), `${name}: HTTP R3 table must scroll inside its region`);
    }
    const httpR3Disclosure = page.getByText("HTTP R3 accepted pairs and rejected-series identities", { exact: true });
    const httpR3Details = page.locator("details.performance-identities").filter({ has: httpR3Disclosure });
    assert(await httpR3Details.count() === 1, `${name}: HTTP R3 disclosure must be unique`);
    await httpR3Disclosure.click();
    for (const [label, digest] of [
      ["HTTP R3 Pair1 summary SHA-256", "d3fa11cce07e0181e155498b72191a47e88a3240abf8cd840dbd5c075c5e788e"],
      ["HTTP R3 Pair2 summary SHA-256", "e74214f1923eac3bbda26ae13acdaae0d0a2d1011ec3aa3cfce52338d5da4c97"],
      ["HTTP R3 rejected series SHA-256", "962bce221f76fee67bb40a71bd9e0302095eba742f60885ef0b6cd1fc2e03159"],
      ["HTTP R3 Pair3 Ferric rejection SHA-256", "c8a69e6f21cc9fe08d161617ef4a100630bc32674270d6634bb49bf60e4bbe11"],
      ["HTTP R3 original series manifest SHA-256", "1753e5cb0fd40b7f4b6dcedef54d8dfa506915de26e317515e93c3354a5d17ee"],
      ["HTTP R3 shared plan SHA-256", "981c0991d88692b36d53df47866b0292c16f843d281ffbbe74fdfc7c701b5e04"],
    ]) {
      const term = httpR3Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: HTTP R3 identity changed: ${label}`);
    }
    await httpR3Disclosure.click();

    const matchedWidth = page.getByRole("region", { name: "Width55c same-plan matched HTTP: 30 measured requests per engine", exact: true });
    assert(await matchedWidth.count() === 1 && await matchedWidth.isVisible(), `${name}: Width55c HTTP table must be unique and visible`);
    assert(await matchedWidth.locator("tbody tr").count() === 2, `${name}: Width55c HTTP engine count`);
    assert(JSON.stringify(await matchedWidth.locator("thead th").allTextContents())
      === JSON.stringify(["Engine", "Median TTFT (ms)", "p95 TTFT (ms)", "Median TPOT (ms)", "p95 TPOT (ms)", "Finite output tokens/s"]),
    `${name}: Width55c HTTP metric columns`);
    for (const [index, values] of [
      [0, ["Ferric 55c prefill32", "431.033", "435.865", "53.119", "53.390", "17.902"]],
      [1, ["vLLM 0.28.0", "18.969", "19.742", "4.352", "4.358", "223.768"]],
    ]) {
      assert(JSON.stringify(await matchedWidth.locator("tbody tr").nth(index).locator("td").allTextContents()) === JSON.stringify(values),
        `${name}: Width55c HTTP raw-derived metric row ${index}`);
    }
    const matchedWidthGeometry = await matchedWidth.evaluate((region) => {
      const box = region.getBoundingClientRect();
      const table = region.querySelector("table");
      const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const rect = cell.getBoundingClientRect();
        return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return { left: box.left, right: box.right, width: box.width, height: box.height, clipped,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
    });
    assert(matchedWidthGeometry.width > 0 && matchedWidthGeometry.height > 0 && matchedWidthGeometry.left >= -1
      && matchedWidthGeometry.right <= width + 1 && matchedWidthGeometry.clipped.length === 0, `${name}: Width55c HTTP text/viewport geometry`);
    if (matchedWidthGeometry.tableWidth > matchedWidthGeometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(matchedWidthGeometry.overflowX), `${name}: Width55c HTTP table must scroll inside its region`);
    }

    const latestHttp = page.getByRole("region", { name: "Earlier V22 matched HTTP cohort", exact: true });
    assert(await latestHttp.count() === 1 && await latestHttp.isVisible(), `${name}: latest HTTP table must be unique and visible`);
    assert(await latestHttp.locator("tbody tr").count() === 2, `${name}: latest HTTP has exactly two admitted engines`);
    assert(JSON.stringify(await latestHttp.locator("thead th").allTextContents())
      === JSON.stringify(["Engine", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]), `${name}: latest HTTP columns changed`);
    for (const [engine, expected] of [
      ["ferric", ["Ferric V22 packed", "2329.922", "83.598", "9.885898"]],
      ["vllm", ["vLLM 0.28.0", "18.636", "4.252", "228.904359"]],
    ]) {
      const row = latestHttp.locator(`[data-v22-http-engine="${engine}"]`);
      assert(await row.count() === 1 && JSON.stringify((await row.locator("th, td").allTextContents()).map((s) => s.trim()))
        === JSON.stringify(expected), `${name}: latest HTTP ${engine} accepted values changed`);
    }
    const latestHttpText = await page.locator("#matched-v22-http-title").evaluate((heading) => {
      const parts = [];
      for (let node = heading; node && node.id !== "native-partial-gemv-r4-title"; node = node.nextElementSibling) {
        if (!node.matches("details")) parts.push(node.innerText);
      }
      return parts.join(" ").replace(/\s+/g, " ").trim();
    });
    for (const claim of [
      "Earlier V22 / vLLM HTTP pair: no win",
      "19.66 times the mean TPOT and 125.02 times the mean TTFT",
      "10 excluded warmups, 30 measured requests and two untimed output diagnostics.",
      "Cohorts run in fixed order, vLLM then Ferric, with one server start each.",
      "There is no confidence interval or stable-tail claim.",
      "This is not a stock BF16-head comparison.",
      "not true token interarrival latency.", "It is not sustained throughput.",
      "Ferric token IDs and decoded bytes match for all 42 requests.",
      "vLLM token IDs match in two untimed diagnostics; its 40 timed requests match decoded text and usage.",
      "same 500 ms process-ownership sampling policy", "Sampling is not continuous isolation proof.",
      "Earlier contention-rejected attempts remain excluded.",
      "not an independent model-file rehash or source-to-binary authentication.",
      "Ordered64, V19, V23 and V25 are not part of this HTTP result.",
      "cross-campaign differences do not attribute a gain to packet packing alone.",
      "No TP8, speculative-decoding, SGLang, general framework ranking or serving qualification is established.",
    ]) assert(latestHttpText.includes(claim), `${name}: latest HTTP limitation missing: ${claim}`);
    const latestGeometry = await latestHttp.evaluate((region) => {
      const table = region.querySelector("table");
      const box = region.getBoundingClientRect();
      const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const rect = cell.getBoundingClientRect();
        return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return { left: box.left, right: box.right, width: box.width, height: box.height, clipped,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
    });
    assert(latestGeometry.width > 0 && latestGeometry.height > 0 && latestGeometry.left >= -1
      && latestGeometry.right <= width + 1 && latestGeometry.clipped.length === 0, `${name}: latest HTTP table text/viewport geometry`);
    if (latestGeometry.tableWidth > latestGeometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(latestGeometry.overflowX), `${name}: latest HTTP table must scroll within its region`);
    }
    const latestDisclosure = page.getByText("Earlier V22 HTTP replay and receipt identities", { exact: true });
    const latestDetails = page.locator("details.performance-identities").filter({ has: latestDisclosure });
    assert(await latestDetails.count() === 1, `${name}: one latest HTTP receipt disclosure`);
    await latestDisclosure.click();
    for (const [label, digest] of [
      ["Paired replay summary SHA-256", "0283449b779015bffe0ba72954069ffcca7b809ae6aadf91aa24d6b3fb9d1c07"],
      ["Shared plan SHA-256", "71855e76d38c3d3ff11d21b7ff33d7c67303a5fc28dff07c8deebb0ee14cf7fd"],
      ["Ferric receipt SHA-256", "91cc42648156fbfedd44a03b704ad9c80a125efe8809b0fc4cff5441b9d2fe92"],
      ["vLLM receipt SHA-256", "e428d623aaece123ef80d926b93e9be409bfbeb3bef9dcd1023503bf132b35c2"],
    ]) {
      const term = latestDetails.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: latest HTTP receipt missing or changed: ${label}`);
    }
    await latestDisclosure.click();

    const partialGemvR4 = page.getByRole("region", { name: "Partial GEMV R4 native AB/BA screening", exact: true });
    assert(await partialGemvR4.count() === 1 && await partialGemvR4.isVisible(), `${name}: Partial GEMV R4 table must be unique and visible`);
    assert(await partialGemvR4.locator("tbody tr").count() === 4, `${name}: Partial GEMV R4 requires four separate starts`);
    assert(JSON.stringify(await partialGemvR4.locator("thead th").allTextContents())
      === JSON.stringify(["Native start", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]), `${name}: Partial GEMV R4 columns changed`);
    for (const [arm, expected] of [
      ["baseline-ab", ["Baseline AB", "877.878138", "79.613330", "11.647774"]],
      ["candidate-ab", ["Candidate AB", "945.068094", "82.110499", "11.253843"]],
      ["candidate-ba", ["Candidate BA", "1040.976727", "91.870497", "10.071339"]],
      ["baseline-ba", ["Baseline BA", "885.450433", "91.893385", "10.194000"]],
    ]) {
      const row = partialGemvR4.locator(`[data-partial-gemv-r4-arm="${arm}"]`);
      assert(await row.count() === 1 && JSON.stringify((await row.locator("th, td").allTextContents()).map((s) => s.trim()))
        === JSON.stringify(expected), `${name}: Partial GEMV R4 ${arm} accepted values changed`);
    }
    for (const [id, claims] of [
      ["partial-gemv-r4-progress", ["No gain is demonstrated", "not promoted to HTTP or made the default", "matched vLLM scoreboard remains unchanged"]],
      ["native-partial-gemv-r4-scope", ["TP1/C1", "AB runs baseline then candidate; BA reverses that order", "excludes one warmup and measures three requests", "not HTTP or vendor results"]],
      ["native-partial-gemv-r4-outcome", ["1.031366 in AB and 0.999751 in BA", "1.076537 and 1.175647", "No gain is demonstrated", "not pooled or cherry-picked", "not sustained throughput", "not promoted to HTTP or made the default", "16 times vLLM's TPOT", "No SGLang, TP8 or speculative-decoding gain"]],
      ["native-partial-gemv-r4-correctness", ["All 16 requests match 2,048 reference output IDs and decoded bytes", "no remaining KFD owners", "not source-to-binary authentication", "All 33 M1 gates remain open"]],
    ]) {
      const text = (await page.locator(`#${id}`).innerText()).replace(/\s+/g, " ");
      for (const claim of claims) assert(text.includes(claim), `${name}: Partial GEMV R4 scope missing: ${claim}`);
    }
    const partialGemvR4Geometry = await partialGemvR4.evaluate((region) => {
      const table = region.querySelector("table");
      const box = region.getBoundingClientRect();
      const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const rect = cell.getBoundingClientRect();
        return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return { left: box.left, right: box.right, width: box.width, height: box.height, clipped,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
    });
    assert(partialGemvR4Geometry.width > 0 && partialGemvR4Geometry.height > 0 && partialGemvR4Geometry.left >= -1
      && partialGemvR4Geometry.right <= width + 1 && partialGemvR4Geometry.clipped.length === 0,
    `${name}: Partial GEMV R4 table text/viewport geometry`);
    if (partialGemvR4Geometry.tableWidth > partialGemvR4Geometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(partialGemvR4Geometry.overflowX), `${name}: Partial GEMV R4 table must scroll within its region`);
    }
    const partialGemvR4Disclosure = page.getByText("Partial GEMV R4 native report and supervisor", { exact: true });
    const partialGemvR4Details = page.locator("details.performance-identities").filter({ has: partialGemvR4Disclosure });
    assert(await partialGemvR4Details.count() === 1, `${name}: one Partial GEMV R4 evidence disclosure`);
    await partialGemvR4Disclosure.click();
    for (const [label, digest] of [
      ["Partial GEMV R4 report SHA-256", "2d5c5846318a93fd888cf35e27cd87f9d53914976bb8371bc1a4e3f92a4d05a4"],
      ["Partial GEMV R4 supervisor SHA-256", "ff36b8b0e62cd722cfc3c8b2acb091bee8283952d7e859bf082af8af92bf2ae0"],
    ]) {
      const term = partialGemvR4Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: Partial GEMV R4 evidence missing or changed: ${label}`);
    }
    await partialGemvR4Disclosure.click();

    const ordered64Native = page.getByRole("region", { name: "Ordered64 native AB/BA screening", exact: true });
    assert(await ordered64Native.count() === 1 && await ordered64Native.isVisible(), `${name}: ordered64 native table must be unique and visible`);
    assert(await ordered64Native.locator("tbody tr").count() === 4, `${name}: ordered64 native requires four separate starts`);
    assert(JSON.stringify(await ordered64Native.locator("thead th").allTextContents())
      === JSON.stringify(["Native start", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]), `${name}: ordered64 native columns changed`);
    assert(JSON.stringify(await ordered64Native.locator("tbody tr").evaluateAll((rows) =>
      rows.map((row) => row.getAttribute("data-ordered64-four-core-arm"))))
      === JSON.stringify(["baseline-AB", "candidate-AB", "candidate-BA", "baseline-BA"]), `${name}: ordered64 native arm order changed`);
    for (const [arm, expected] of [
      ["baseline-AB",["Baseline AB","1024.554916","94.799826","9.797235"]],
      ["candidate-AB",["Candidate AB","858.870490","70.714876","13.007960"]],
      ["candidate-BA",["Candidate BA","865.604204","70.180564","13.089270"]],
      ["baseline-BA",["Baseline BA","876.261260","80.273610","11.561235"]],
    ]) {
      const row = ordered64Native.locator('[data-ordered64-four-core-arm="' + arm + '"]');
      assert(await row.count() === 1 && JSON.stringify((await row.locator("th, td").allTextContents()).map((s) => s.trim()))
        === JSON.stringify(expected), `${name}: ordered64 native ${arm} accepted values changed`);
    }
    for (const [id, expected] of [
      ["ordered64-native-progress","All 2,048 output IDs and decoded bytes match across four native starts. TPOT is 25.4061% lower in AB and 12.5733% lower in BA; the two orders remain separate. Each start excludes one warmup and measures three requests. This is native screening, not a new HTTP result or vLLM win. Defaults and the matched HTTP scoreboard remain unchanged."],
      ["native-ordered64-four-core-scope","Qwen3-8B on mi350, TP1/C1, sequential 128-input/128-output requests. AB runs baseline then candidate; BA reverses that order. Each start excludes one warmup and measures three requests. Both arms use the same controller, dd6 worker built from 5e2 and eight images. Only packed16 versus packed64 ordered submission changes; parallel-prefill16-v27 and split8-v21 stay enabled, and partial GEMV stays off."],
      ["native-ordered64-four-core-outcome","Candidate/baseline TPOT ratios are 0.745939 in AB and 0.874267 in BA: TPOT is 25.4061% and 12.5733% lower, respectively. TTFT ratios are 0.838286 and 0.987838; output-rate ratios are 1.327717 and 1.132169. Orders are not pooled or cherry-picked. These finite-window output rates are not sustained throughput, shader timings, confidence intervals or a stable gain. This supports a separate matched HTTP test, not HTTP qualification or a vLLM win. Defaults remain unchanged. The matched HTTP scoreboard still shows roughly 16 times vLLM's TPOT. No SGLang, TP8 or speculative-decoding gain is established."],
      ["native-ordered64-four-core-provenance","CPU qualification retains 17 original R3 one-core gates and a separate successful four-core controller build; it is not 18 fresh four-core gates. The controller's opt-in runtime dependency is pinned to 13e; the worker remains built from 5e2. The baseline profile is prefill16-decode-partial-gemv-v28-live-v1 with GEMV off; the candidate profile is prefill16-decode-ordered64-v29-live-v1."],
      ["native-ordered64-four-core-correctness","All 16 requests match all 2,048 reference output IDs and decoded bytes. The outer supervisor exits 0 without signals. Each arm records cleanup_ok true and its owned group absent, with TERM and KILL flags retained as sent. This is not a claim of signal-free arm teardown. Exact output parity is not independent source-to-binary authentication, protected proof or serving qualification. All 33 M1 gates remain open."],
    ]) {
      const paragraph = page.locator("#" + id);
      assert(await paragraph.count() === 1 && (await paragraph.innerText()).replace(/\s+/g, " ").trim() === expected,
        `${name}: ordered64 native scope or provenance changed: ${id}`);
    }
    const ordered64NativeGeometry = await ordered64Native.evaluate((region) => {
      const table = region.querySelector("table");
      const box = region.getBoundingClientRect();
      const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const rect = cell.getBoundingClientRect();
        return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      const originalScroll = region.scrollLeft;
      region.scrollLeft = region.scrollWidth;
      const scrolled = region.scrollLeft;
      region.scrollLeft = originalScroll;
      return { left: box.left, right: box.right, width: box.width, height: box.height, clipped, scrolled,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
    });
    assert(ordered64NativeGeometry.width > 0 && ordered64NativeGeometry.height > 0 && ordered64NativeGeometry.left >= -1
      && ordered64NativeGeometry.right <= width + 1 && ordered64NativeGeometry.clipped.length === 0,
    `${name}: ordered64 native table text/viewport geometry`);
    if (ordered64NativeGeometry.tableWidth > ordered64NativeGeometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(ordered64NativeGeometry.overflowX) && ordered64NativeGeometry.scrolled > 0,
        `${name}: ordered64 native table must scroll within its region`);
    }
    const ordered64NativeDisclosure = page.getByText("Ordered64 native report and supervisor", { exact: true });
    const ordered64NativeDetails = page.locator("details.performance-identities").filter({ has: ordered64NativeDisclosure });
    assert(await ordered64NativeDetails.count() === 1, `${name}: one ordered64 native evidence disclosure`);
    await ordered64NativeDisclosure.click();
    for (const [label, digest] of [
      ["Ordered64 native report SHA-256","cbef8172a332d0d3086889f8ac5cab4b1f913114a4c37535bcf23b062c812015"],
      ["Ordered64 native supervisor SHA-256","e2e234a94971f996b4f4bb1167ed80664ee1362c45a7d091a816b6d3ce2d7617"],
    ]) {
      const term = ordered64NativeDetails.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: ordered64 native evidence missing or changed: ${label}`);
    }
    await ordered64NativeDisclosure.click();

    const composedV28 = page.getByRole("region", { name: "Composition R6 native prefill and decode screening", exact: true });
    assert(await composedV28.count() === 1 && await composedV28.isVisible(), `${name}: Composition R6 table must be unique and visible`);
    assert(await composedV28.locator("tbody tr").count() === 4, `${name}: Composition R6 requires four arms`);
    assert(JSON.stringify(await composedV28.locator("thead th").allTextContents())
      === JSON.stringify(["Native arm", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]), `${name}: Composition R6 columns changed`);
    const composedV28Rows = [
      ["baseline", ["Baseline", "2430.855653", "93.406125", "8.954908643"]],
      ["decode-only", ["Decode only", "2365.162141", "76.228505", "10.625352362"]],
      ["prefill-only", ["Prefill only", "1021.103724", "93.120622", "9.962451944"]],
      ["composed", ["Prefill + decode", "855.944209", "85.379419", "10.940545624"]],
    ];
    assert(JSON.stringify(await composedV28.locator("tbody tr").evaluateAll((rows) => rows.map((row) => row.dataset.composedV28Arm)))
      === JSON.stringify(composedV28Rows.map(([arm]) => arm)), `${name}: Composition R6 order changed`);
    for (const [arm, expected] of composedV28Rows) {
      const row = composedV28.locator(`[data-composed-v28-arm="${arm}"]`);
      assert(await row.count() === 1 && JSON.stringify((await row.locator("th, td").allTextContents()).map((text) => text.trim()))
        === JSON.stringify(expected), `${name}: Composition R6 ${arm} values changed`);
    }
    const composedV28Text = await page.locator("#native-composed-v28-title").evaluate((heading) => {
      const parts = [];
      for (let node = heading; node && node.id !== "native-v28-title"; node = node.nextElementSibling) {
        if (!node.matches("details")) parts.push(node.innerText);
      }
      return parts.join(" ").replace(/\s+/g, " ").trim();
    });
    for (const claim of ["baseline, decode-only, prefill-only, combined", "excludes one warmup and measures three requests",
      "rebuilt 5e2 worker and seven fixed images; instrumentation is off", "V22 packing and V25 split attention",
      "V27 page copy through V28", "TTFT is 64.7884% lower", "TPOT is 8.5933% lower", "output rate is 22.1737% higher",
      "in this cohort combined TPOT was 12.0046% higher than decode-only", "85.379419 versus 76.228505 ms",
      "Repeated runs are needed to determine whether that difference persists", "does not establish a stable penalty",
      "not multiplied historical gains", "do not establish a confidence interval, stable improvement or tail behavior",
      "completion span divided by 127", "384 output tokens", "including gaps and excluding warmup",
      "not HTTP timings, GPU durations or sustained throughput", "HTTP R3 pairs above remain losses",
      "no vendor, SGLang, TP8, speculative-decoding or serving qualification", "2,048 reference output IDs and decoded bytes",
      "135 batches per request", "83,139 packets", "87,711", "expected source schedules, not measured counters",
      "inner teardown used TERM/KILL", "outer supervisor completed cleanly without signals", "OCML accuracy premise remains unverified",
      "not independent source-to-binary authentication or protected proof", "Historical images and prerequisite runs retain their original identities",
      "Defaults are unchanged; all 33 M1 gates remain open"]) {
      assert(composedV28Text.includes(claim), `${name}: Composition R6 visible limitation missing: ${claim}`);
    }
    const composedV28Geometry = await composedV28.evaluate((region) => {
      const table = region.querySelector("table");
      const box = region.getBoundingClientRect();
      const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const rect = cell.getBoundingClientRect();
        return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return { left: box.left, right: box.right, width: box.width, height: box.height, clipped,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
    });
    assert(composedV28Geometry.width > 0 && composedV28Geometry.height > 0 && composedV28Geometry.left >= -1
      && composedV28Geometry.right <= width + 1 && composedV28Geometry.clipped.length === 0, `${name}: Composition R6 text/viewport geometry`);
    if (composedV28Geometry.tableWidth > composedV28Geometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(composedV28Geometry.overflowX), `${name}: Composition R6 table must scroll within its region`);
    }
    const composedV28Disclosure = page.getByText("Composition R6 native report and retained evidence", { exact: true });
    const composedV28Details = page.locator("details.performance-identities").filter({ has: composedV28Disclosure });
    assert(await composedV28Details.count() === 1, `${name}: exactly one Composition R6 disclosure`);
    await composedV28Disclosure.click();
    for (const [label, digest] of [
      ["Composition R6 report SHA-256", "dbf5e34ea6466d84211c85b778fe668d046c5de931491c32b1f4a8118498bcd6"],
      ["Composition R6 archive SHA-256", "eb1f97196a736c2b9512a12c966a8ddb25d5aa3f09153ead95489dcf70ef564c"],
      ["Composition R6 controller SHA-256", "1ffd68bc905ed6a1cc1bfb7dfad88f64b519ed1aede5aa2c079768982960afe6"],
      ["Composition R6 worker SHA-256", "dd6bd3b4a910478e85d2f3530be25819153fad1f08bf33bb14dd534514a601a2"],
    ]) {
      const term = composedV28Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: Composition R6 evidence changed: ${label}`);
    }
    await composedV28Disclosure.click();

    const nativeV28 = page.getByRole("region", { name: "V28 native prefill-copy screening", exact: true });
    assert(await nativeV28.count() === 1 && await nativeV28.isVisible(), `${name}: V28 table must be unique and visible`);
    assert(await nativeV28.locator("tbody tr").count() === 2, `${name}: V28 requires exactly two arms`);
    assert(JSON.stringify(await nativeV28.locator("thead th").allTextContents())
      === JSON.stringify(["Native prefill arm", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]), `${name}: V28 columns changed`);
    const nativeV28Rows = [
      ["baseline", ["Baseline", "2312.563649", "85.149690", "9.750891"]],
      ["parallel-prefill16-v27", ["V28 parallel prefill copy", "808.974061", "85.154418", "11.011655"]],
    ];
    assert(JSON.stringify(await nativeV28.locator("tbody tr").evaluateAll((rows) => rows.map((row) => row.dataset.v28Arm)))
      === JSON.stringify(nativeV28Rows.map(([arm]) => arm)), `${name}: V28 fixed arm order changed`);
    for (const [arm, expected] of nativeV28Rows) {
      const row = nativeV28.locator(`[data-v28-arm="${arm}"]`);
      assert(await row.count() === 1 && JSON.stringify((await row.locator("th, td").allTextContents()).map((text) => text.trim()))
        === JSON.stringify(expected), `${name}: V28 ${arm} accepted values changed`);
    }
    const nativeV28Text = await page.locator("#native-v28-title").evaluate((heading) => {
      const parts = [];
      for (let node = heading; node && node.id !== "native-composition-title"; node = node.nextElementSibling) {
        if (!node.matches("details")) parts.push(node.innerText);
      }
      return parts.join(" ").replace(/\s+/g, " ").trim();
    });
    for (const claim of ["one excluded warmup followed by three measured requests", "fixed baseline then parallel-prefill16-v27 order",
      "same V28 controller, historical f68 worker", "V22 packing and V25 split attention are absent",
      "mean TTFT is 65.0183% lower and TPOT is effectively unchanged", "no confidence interval, stable gain or tail claim",
      "cannot be multiplied into this prefill result", "combined candidate is measured separately in Composition R6 above",
      "neither HTTP timings nor GPU durations", "completion span by 127", "384 tokens over the measured ingress window",
      "not sustained throughput", "19.66 times the TPOT and 125.02 times the TTFT",
      "No new vLLM or SGLang comparison", "all 1,024 reference token IDs and decoded bytes",
      "135 batches and 83,139 packets per request", "288 prefill append kernels without reducing the packet count",
      "inner arm teardown used owned TERM/KILL signals", "outer supervisor exited cleanly without signals",
      "earlier staging-path failure remains excluded", "not general numerical or protected proof qualification",
      "engineering custody, not independent source-to-binary authentication",
      "Defaults stay unchanged and all 33 M1 gates remain open"]) {
      assert(nativeV28Text.includes(claim), `${name}: V28 visible limitation missing: ${claim}`);
    }
    const nativeV28Geometry = await nativeV28.evaluate((region) => {
      const table = region.querySelector("table");
      const box = region.getBoundingClientRect();
      const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const rect = cell.getBoundingClientRect();
        return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return { left: box.left, right: box.right, width: box.width, height: box.height, clipped,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
    });
    assert(nativeV28Geometry.width > 0 && nativeV28Geometry.height > 0 && nativeV28Geometry.left >= -1
      && nativeV28Geometry.right <= width + 1 && nativeV28Geometry.clipped.length === 0, `${name}: V28 text/viewport geometry`);
    if (nativeV28Geometry.tableWidth > nativeV28Geometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(nativeV28Geometry.overflowX), `${name}: V28 table must scroll within its region`);
    }
    const nativeV28Disclosure = page.getByText("V28 native screening report and retained evidence", { exact: true });
    const nativeV28Details = page.locator("details.performance-identities").filter({ has: nativeV28Disclosure });
    assert(await nativeV28Details.count() === 1, `${name}: exactly one V28 evidence disclosure`);
    await nativeV28Disclosure.click();
    for (const [label, digest] of [
      ["V28 native report SHA-256", "fd58ba7aa82f7bc7a18868e9837bf0de2929910ef519d74b8302a0b2d68ead17"],
      ["V28 retained archive SHA-256", "039561bd6c792e3e57c7685a65c27e787905aa7f8579850254852f7c17e6fee5"],
      ["V28 controller SHA-256", "3ea106b88e1e77c48ff9d32aa988ecd0554d9b6d5bc1d7eb62b5e4caeff0914c"],
      ["Historical V28 worker SHA-256", "f68e42197f5f91854f7ec15c92b2fe1eee29a5620a6d313751e64f0af9598ebc"],
    ]) {
      const term = nativeV28Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: V28 receipt missing or changed: ${label}`);
    }
    await nativeV28Disclosure.click();
    const composition = page.getByRole("region", { name: "V22 and V25 native composition cohort", exact: true });
    assert(await composition.count() === 1 && await composition.isVisible(), `${name}: composition table must be unique and visible`);
    assert(await composition.locator("tbody tr").count() === 4, `${name}: composition requires exactly four native arms`);
    assert(JSON.stringify(await composition.locator("thead th").allTextContents())
      === JSON.stringify(["Native arm", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]), `${name}: composition columns changed`);
    const compositionRows = [
      ["baseline", ["Baseline", "2299.826074", "84.737454", "9.799532"]],
      ["packed-only", ["V22 packing only", "2325.464341", "83.252562", "9.923006"]],
      ["split-only", ["V25 split only", "2302.796852", "73.957084", "10.944131"]],
      ["composed", ["V22 + V25 composed", "2311.805893", "70.241527", "11.395125"]],
    ];
    assert(JSON.stringify(await composition.locator("tbody tr").evaluateAll((rows) => rows.map((row) => row.dataset.compositionArm)))
      === JSON.stringify(compositionRows.map(([arm]) => arm)), `${name}: composition fixed arm order changed`);
    for (const [arm, expected] of compositionRows) {
      const row = composition.locator(`[data-composition-arm="${arm}"]`);
      assert(await row.count() === 1 && JSON.stringify((await row.locator("th, td").allTextContents()).map((text) => text.trim()))
        === JSON.stringify(expected), `${name}: composition ${arm} accepted values changed`);
    }
    const compositionText = await page.locator("#native-composition-title").evaluate((heading) => {
      const parts = [];
      for (let node = heading; node && node.id !== "native-v22-title"; node = node.nextElementSibling) {
        if (!node.matches("details")) parts.push(node.innerText);
      }
      return parts.join(" ").replace(/\s+/g, " ").trim();
    });
    for (const claim of [
      "one excluded warmup and three measured requests per arm",
      "same controller binary, historical f68 worker and six historical images",
      "mean TPOT is 17.11% lower", "mean TTFT is 0.52% higher", "output rate is 16.28% higher",
      "not an additive prediction", "do not establish confidence intervals, a stable gain or tail behavior",
      "controller-ingress timings, not HTTP timings or GPU durations", "span divided by 127",
      "384 output tokens", "including inter-request gaps and excluding warmup", "It is not sustained throughput",
      "HTTP R3 pairs above remain losses", "no vendor comparison, SGLang measurement, TP8 result or serving qualification",
      "All 16 requests match all 2,048 reference token IDs and decoded bytes", "owned cleanup passes",
      "unverified OCML accuracy bound", "do not establish general numerical or protected proof qualification",
      "Defaults remain unchanged", "not a latest-runtime result", "All 33 M1 gates remain open",
    ]) assert(compositionText.includes(claim), `${name}: composition visible limitation missing: ${claim}`);
    const compositionGeometry = await composition.evaluate((region) => {
      const table = region.querySelector("table");
      const box = region.getBoundingClientRect();
      const clipped = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const rect = cell.getBoundingClientRect();
        return text.left < rect.left - 1 || text.right > rect.right + 1 || text.top < rect.top - 1 || text.bottom > rect.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return { left: box.left, right: box.right, width: box.width, height: box.height, clipped,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth, overflowX: getComputedStyle(region).overflowX };
    });
    assert(compositionGeometry.width > 0 && compositionGeometry.height > 0 && compositionGeometry.left >= -1
      && compositionGeometry.right <= width + 1 && compositionGeometry.clipped.length === 0, `${name}: composition text/viewport geometry`);
    if (compositionGeometry.tableWidth > compositionGeometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(compositionGeometry.overflowX), `${name}: composition table must scroll within its region`);
    }
    const compositionDisclosure = page.getByText("Native composition report and retained evidence", { exact: true });
    const compositionDetails = page.locator("details.performance-identities").filter({ has: compositionDisclosure });
    assert(await compositionDetails.count() === 1, `${name}: exactly one composition evidence disclosure`);
    await compositionDisclosure.click();
    for (const [label, digest] of [
      ["Composition report SHA-256", "86500d3357241488ac64692618c7a1bf05ab71062b79b7c75c2d5dfefdd77447"],
      ["Composition retained archive SHA-256", "bda385eba79356b7bb311eb83c44e0e064c7ec4a598bb6ab0dadff2a696aee43"],
      ["Controller SHA-256", "898b4ded4d93583c4421f3dbc87fb7beb8ce43e22e4ba18534e6425d91a8d900"],
      ["Historical worker SHA-256", "f68e42197f5f91854f7ec15c92b2fe1eee29a5620a6d313751e64f0af9598ebc"],
    ]) {
      const term = compositionDetails.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: composition receipt missing or changed: ${label}`);
    }
    await compositionDisclosure.click();
    for (const [selector, claims] of [
      ["#http-r3-progress", ["Pair1 and Pair2 pass individual replay", "three-pair series is rejected and incomplete", "69.587 and 69.186 ms", "4.367 and 4.350 ms", "15.93 and 15.91 times vLLM", "foreign GPU process appeared", "timing is not admitted", "Pair3 vLLM was never started", "No replacement or splicing repairs this series", "not a vendor win or sustained-serving result"]],
      ["#v28-progress", ["all 1,024 reference token IDs and decoded bytes", "Mean TTFT falls 65.0183%", "2312.564 to 808.974 ms", "TPOT is effectively unchanged at 85.150 versus 85.154 ms", "not HTTP or a vendor win", "newer HTTP R3 pairs above are still losses", "do not multiply this reduction", "Defaults stay unchanged"]],
      ["#composition-progress", ["all 2,048 reference token IDs and decoded bytes", "TPOT is 17.11% lower", "TTFT is 0.52% higher", "not a new vLLM or SGLang win"]],
      ["#composed-v28-progress", ["2,048 output token IDs and decoded bytes", "TTFT is 64.7884% lower", "TPOT is 8.5933% lower",
        "output rate is 22.1737% higher", "In this cohort, combined TPOT was also 12.0046% higher than decode-only",
        "repeated runs are needed to determine whether that difference persists", "Three measured requests per arm, in fixed order",
        "not HTTP, sustained throughput or a vendor win", "newer HTTP R3 pairs remain losses; their three-pair series is incomplete"]],
      ["#model-r8-progress", ["four cold ordinary/diagnostic requests", "512 output IDs and decoded bytes", "166,278 raw timestamp records",
        "20,478 groups", "same rebuilt 5e2 worker", "not nanoseconds, additive shader costs or a performance result",
        "exact parity is not proof qualification", "Composition R6 itself is uninstrumented"]],
      ["#timestamp-progress", ["3d473f9ff", "210 dependent operations and 528 guard checks", "raw ticks, not shader nanoseconds", "no performance gain or proof qualification", "no GitHub-hosted build or test", "historical f68 worker"]],
      ["#v27-progress", ["Wave64", "336-byte kernarg segment and 12 arguments", "no LDS, scratch, spills or AGPR", "28 correctness cases and retains 12 timing cells", "clean supervisor teardown", "not a full-model TTFT or TPOT result", "V28 full-model native screening is reported below; neither result is serving qualification"]],
    ]) {
      const paragraph = page.locator(selector);
      assert(await paragraph.count() === 1 && await paragraph.isVisible(), `${name}: missing visible ${selector}`);
      const text = (await paragraph.innerText()).replace(/\s+/g, " ");
      for (const claim of claims) assert(text.includes(claim), `${name}: ${selector} claim changed: ${claim}`);
    }
    const v27Disclosure = page.getByText("V27 isolated validation evidence", { exact: true });
    const v27Details = page.locator("details.performance-identities").filter({ has: v27Disclosure });
    assert(await v27Details.count() === 1, `${name}: one V27 evidence disclosure`);
    await v27Disclosure.click();
    for (const [label, digest] of [
      ["V27 native report SHA-256", "e501fb19d64e4c28168612eeaa6a5f06865da4399a78910af6070e3a116c58e8"],
      ["V27 native archive SHA-256", "32f5972eb1274bc0d3825e52e7c0cf65a09249f71f3aa36c44ce68c71223a32a"],
    ]) {
      const term = v27Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible()
        && await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
      `${name}: V27 receipt missing or changed: ${label}`);
    }
    await v27Disclosure.click();

    const nativeV22 = page.getByRole("region", { name: "V22 native controller A/B", exact: true });
    assert(await nativeV22.count() === 1 && await nativeV22.isVisible(), `${name}: V22 native table must be unique and visible`);
    assert(await nativeV22.locator("tbody tr").count() === 2, `${name}: V22 has exactly two native arms`);
    assert(JSON.stringify(await nativeV22.locator("thead th").allTextContents())
      === JSON.stringify(["Packet-grouping arm", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]),
    `${name}: V22 metric columns changed`);
    for (const [arm, expected] of [
      ["baseline", ["Control: baseline", "2600.412", "111.871", "7.614984"]],
      ["packed16-v22", ["Candidate: packed16-v22", "2524.218", "98.857", "8.488262"]],
    ]) {
      const row = nativeV22.locator(`[data-v22-arm="${arm}"]`);
      assert(await row.count() === 1, `${name}: V22 ${arm} row must be unique`);
      const cells = (await row.locator("th, td").allTextContents()).map((text) => text.trim());
      assert(JSON.stringify(cells) === JSON.stringify(expected), `${name}: V22 ${arm} accepted values changed`);
    }
    const nativeV22Text = await page.locator("#native-v22-title").evaluate((heading) => {
      const parts = [];
      for (let node = heading; node && !node.matches("#native-v19-title"); node = node.nextElementSibling) {
        parts.push(node.textContent);
      }
      return parts.join(" ").replace(/\s+/g, " ");
    });
    for (const claim of [
      "one excluded warmup and three measured requests, in fixed baseline then candidate order",
      "same controller, f68 runtime, five unchanged images",
      "mean TPOT is 11.63% lower and finite-window output rate is 11.47% higher",
      "98.199, 109.135 and 89.237 ms",
      "do not establish a stable effect or confidence interval",
      "controller-ingress timings, not HTTP timings",
      "span divided by 127", "Output rate divides 384 tokens",
      "not sustained throughput, a GPU duration, a vendor comparison or serving qualification",
      "all 1,024 reference token IDs and decoded bytes", "540 batches and 332,556 dispatches",
      "ordered64 worker is not used in this pair",
      "does not replace or improve the frozen HTTP result below",
    ]) {
      assert(nativeV22Text.includes(claim), `${name}: V22 visible scope missing: ${claim}`);
    }
    const ordered64Text = (await page.locator("#ordered64-progress").textContent()).replace(/\s+/g, " ");
    assert(ordered64Text.includes("Historical synthetic-only check.")
      && ordered64Text.includes("251 synthetic dependent packets and 594 full-buffer/guard checks")
      && ordered64Text.includes("no model inference, timing result or model-performance gain is established"),
    `${name}: ordered64 correctness-only boundary changed`);
    const nativeV22Geometry = await nativeV22.evaluate((region) => {
      const box = region.getBoundingClientRect();
      const table = region.querySelector("table");
      return {
        left: box.left, right: box.right, width: box.width, height: box.height,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth,
        overflowX: getComputedStyle(region).overflowX,
        clippedCells: [...table.querySelectorAll("th, td")].filter((cell) => cell.scrollWidth > cell.clientWidth + 1)
          .map((cell) => cell.textContent.trim()),
      };
    });
    assert(nativeV22Geometry.width > 0 && nativeV22Geometry.height > 0, `${name}: V22 table has no rendered area`);
    assert(nativeV22Geometry.left >= -1 && nativeV22Geometry.right <= width + 1, `${name}: V22 region exceeds viewport`);
    assert(nativeV22Geometry.clippedCells.length === 0, `${name}: V22 cell text is clipped: ${nativeV22Geometry.clippedCells.join(", ")}`);
    if (nativeV22Geometry.tableWidth > nativeV22Geometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(nativeV22Geometry.overflowX), `${name}: wide V22 table must scroll in its region`);
    }
    const nativeV22Disclosure = page.getByText("V22 native report and retained evidence", { exact: true });
    const nativeV22Details = page.locator("details.performance-identities").filter({ has: nativeV22Disclosure });
    assert(await nativeV22Details.count() === 1, `${name}: exactly one V22 receipt disclosure`);
    await nativeV22Disclosure.click();
    for (const [label, digest] of [
      ["V22 native report SHA-256", "634c1634aac5a1a01fd7e436fe155a8d3300e14e5dca76bb0920dd0af54aa697"],
      ["V22 retained archive SHA-256", "a16078f0541f06857cad27a6e549ed83759535ba71db1b967b61cced223e7287"],
      ["Ordered64 correctness report SHA-256", "77e225baf226ef220234834207f6be8568b941ee35410da092bc7f014d5d0b33"],
    ]) {
      const term = nativeV22Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible(), `${name}: V22 receipt label missing: ${label}`);
      assert((await term.evaluate((element) => element.nextElementSibling.textContent)).trim() === digest,
        `${name}: V22 receipt changed: ${label}`);
    }
    await nativeV22Disclosure.click();

    const nativeV19 = page.getByRole("region", { name: "V19 native controller A/B", exact: true });
    assert(await nativeV19.count() === 1 && await nativeV19.isVisible(), `${name}: V19 native table must be unique and visible`);
    assert(await nativeV19.locator("tbody tr").count() === 2, `${name}: V19 has exactly two native arms`);
    assert(JSON.stringify(await nativeV19.locator("thead th").allTextContents())
      === JSON.stringify(["KV-copy arm", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]),
    `${name}: V19 metric columns changed`);
    for (const [arm, expected] of [
      ["baseline", ["Control: baseline", "2300.203", "84.754", "9.797679"]],
      ["parallel-c1-v19", ["Candidate: parallel-c1-v19", "2458.523", "90.230", "9.196630"]],
    ]) {
      const row = nativeV19.locator(`[data-v19-arm="${arm}"]`);
      assert(await row.count() === 1, `${name}: V19 ${arm} row must be unique`);
      const cells = (await row.locator("th, td").allTextContents()).map((text) => text.trim());
      assert(JSON.stringify(cells) === JSON.stringify(expected), `${name}: V19 ${arm} accepted values changed`);
    }
    const nativeV19Text = await page.locator("#native-v19-title").evaluate((heading) => {
      const parts = [];
      for (let node = heading; node && !node.matches("#matched-v5-title"); node = node.nextElementSibling) {
        if (!node.matches("details")) parts.push(node.innerText);
      }
      return parts.join(" ").replace(/\s+/g, " ").trim();
    });
    for (const claim of [
      "V19 native KV-copy A/B: no gain",
      "Qwen3-8B on mi350, TP1/C1, with 128 input tokens and 128 output tokens, context 8192, BF16 decoder / FP32 head, speculation off and prefix caching off.",
      "Each arm has one excluded warmup and three measured requests, in fixed control then candidate order.",
      "Both arms load the same six images and use the same controller, runtime and sequential arrivals; only the KV-copy selector changes.",
      "The candidate is slower on average in this run, with measured TPOT samples of 81.875, 101.385 and 87.431 ms.",
      "Three samples in fixed order do not establish a stable effect or a confidence interval.",
      "Defaults remain unchanged.",
      "These are controller-ingress timings, not HTTP timings.",
      "TTFT spans request arrival to first token completion; TPOT is the first-to-last token completion span divided by 127.",
      "Output rate divides 384 tokens by the measured ingress window, including inter-request gaps and excluding warmup.",
      "This is not sustained throughput, a GPU duration, a vendor comparison or serving qualification.",
      "All eight requests match all 1,024 reference token IDs and decoded bytes.",
      "Each arm completes 540 batches and 332,556 dispatches with successful owned cleanup.",
      "Both arms retain inherited CPU affinity and nice 0.",
      "Build receipts bind engineering provenance, not independent source-to-binary authentication.",
      "This native A/B does not replace or improve the frozen HTTP result below.",
    ]) {
      assert(nativeV19Text.includes(claim), `${name}: V19 visible scope or limitation missing: ${claim}`);
    }
    for (const claim of [
      /\b(?:V19|the candidate|Ferric) (?:is |runs )?(?:faster|beats|outperforms|wins)\b/i,
      /\b(?:V19|the candidate) (?:speedup|performance gain)\b/i,
    ]) {
      assert(!claim.test(nativeV19Text), `${name}: V19 contains a gain claim: ${claim}`);
    }
    const nativeV19Geometry = await nativeV19.evaluate((region) => {
      const table = region.querySelector("table");
      const rect = region.getBoundingClientRect();
      const clippedCells = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const box = cell.getBoundingClientRect();
        return text.left < box.left - 1 || text.right > box.right + 1
          || text.top < box.top - 1 || text.bottom > box.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return {
        left: rect.left, right: rect.right, width: rect.width, height: rect.height,
        tableWidth: table.getBoundingClientRect().width, clientWidth: region.clientWidth,
        overflowX: getComputedStyle(region).overflowX, clippedCells,
      };
    });
    assert(nativeV19Geometry.width > 0 && nativeV19Geometry.height > 0,
      `${name}: V19 table has no rendered area`);
    assert(nativeV19Geometry.left >= -1 && nativeV19Geometry.right <= width + 1,
      `${name}: V19 table region exceeds the viewport`);
    assert(nativeV19Geometry.clippedCells.length === 0,
      `${name}: V19 table text exceeds cells: ${nativeV19Geometry.clippedCells.join(", ")}`);
    if (nativeV19Geometry.tableWidth > nativeV19Geometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(nativeV19Geometry.overflowX), `${name}: wide V19 table must scroll within its region`);
    }
    const nativeV19Disclosure = page.getByText("V19 native report and retained evidence", { exact: true });
    const nativeV19Details = page.locator("details.performance-identities").filter({ has: nativeV19Disclosure });
    assert(await nativeV19Details.count() === 1, `${name}: exactly one V19 receipt disclosure`);
    await nativeV19Disclosure.click();
    for (const [label, digest] of [
      ["V19 native report SHA-256", "93817dcd9bb5d66ced2f03d639a89e152626097dc979783a82c6d76ea95600b3"],
      ["V19 retained archive SHA-256", "4f8e658d2c14bdb8ce1669aeab8ef382444d93fb4db258ed9cfd16e812a37a61"],
    ]) {
      const term = nativeV19Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible(), `${name}: V19 receipt label missing: ${label}`);
      assert(await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
        `${name}: V19 receipt digest changed: ${label}`);
    }
    await nativeV19Disclosure.click();

    const matchedV5 = page.getByRole("region", { name: "V5 first matched HTTP cohort", exact: true });
    assert(await matchedV5.count() === 1 && await matchedV5.isVisible(), `${name}: V5 matched table must be unique and visible`);
    assert(await matchedV5.locator("tbody tr").count() === 2, `${name}: V5 has exactly two admitted engines`);
    assert(JSON.stringify(await matchedV5.locator("thead th").allTextContents())
      === JSON.stringify(["Engine", "Mean TTFT (ms)", "Mean TPOT (ms)", "Output tokens/s"]),
    `${name}: V5 metric columns changed`);
    for (const [engine, expected] of [
      ["ferric", ["Ferric V17 combined", "2555.823", "114.619", "7.479295"]],
      ["vllm", ["vLLM 0.28.0", "19.909", "4.232", "229.358430"]],
    ]) {
      const row = matchedV5.locator(`[data-v5-engine="${engine}"]`);
      assert(await row.count() === 1, `${name}: V5 ${engine} row must be unique`);
      const cells = (await row.locator("th, td").allTextContents()).map((text) => text.trim());
      assert(JSON.stringify(cells) === JSON.stringify(expected), `${name}: V5 ${engine} replay-confirmed values changed`);
    }
    const matchedV5Text = await page.locator("#matched-v5-title").evaluate((heading) => {
      const parts = [];
      for (let node = heading; node && !node.matches("[data-live-http-progress]"); node = node.nextElementSibling) {
        if (!node.matches("details")) parts.push(node.innerText);
      }
      return parts.join(" ").replace(/\s+/g, " ").trim();
    });
    for (const claim of [
      "V5 matched Ferric / vLLM HTTP pair",
      "Qwen3-8B on mi350 physical GPU 0, TP1, concurrency 1, 128 input and 128 output tokens, context 8192, BF16 decoder and FP32 output-head profile.",
      "Both engines use the same SSE client, with speculation and prefix caching disabled.",
      "10 excluded warmups, 30 measured requests and two untimed output diagnostics.",
      "Ferric has 27.08 times the mean TPOT and 128.37 times the mean TTFT of vLLM in this cell.",
      "All 30 measured requests succeed for each engine.",
      "Cohorts run in fixed order, vLLM then Ferric, with one server start each; no confidence interval or stable tail claim follows.",
      "This is not a stock BF16-output-head comparison.",
      "TTFT is client send to first nonempty text.",
      "TPOT is the first-to-last text span divided by 127, not true per-token inter-token latency.",
      "Output rate divides 3,840 tokens by the complete measured cohort window, including request gaps and drain, excluding warmups and diagnostics.",
      "It is not sustained throughput.",
      "Independent replay passes the same plan, client, model/image hash receipts and reference checks.",
      "Ferric token IDs match for all 42 requests; vLLM token IDs match in the two untimed diagnostics, while its 40 timed requests match decoded text and usage.",
      "Owned processes and the vendor container are cleaned up, and GPU postflight checks are idle.",
      "The replay checks retained hash receipts; it does not independently rehash model files.",
      "Ferric baseline keeps its 1fc45a52 implementation and 5ed3840a runtime attribution",
      "existing kernel images retain their original producers.",
      "A finite matched cohort does not establish sustained throughput, serving qualification, a general engine ranking or SGLang performance.",
    ]) {
      assert(matchedV5Text.includes(claim), `${name}: V5 visible scope or limitation missing: ${claim}`);
    }
    for (const claim of [
      /\bFerric (?:is |runs )?(?:faster than|beats|outperforms)\b/i,
      /\bFerric (?:wins|is competitive|has achieved parity)\b/i,
      /\b(?:general engine|framework) win\b/i,
    ]) {
      assert(!claim.test(matchedV5Text), `${name}: V5 contains a win claim: ${claim}`);
    }
    const matchedV5Geometry = await matchedV5.evaluate((region) => {
      const table = region.querySelector("table");
      const rect = region.getBoundingClientRect();
      const tableRect = table.getBoundingClientRect();
      const clippedCells = [...table.querySelectorAll("th, td")].filter((cell) => {
        const range = document.createRange();
        range.selectNodeContents(cell);
        const text = range.getBoundingClientRect();
        const box = cell.getBoundingClientRect();
        return text.left < box.left - 1 || text.right > box.right + 1
          || text.top < box.top - 1 || text.bottom > box.bottom + 1;
      }).map((cell) => cell.textContent.trim());
      return {
        left: rect.left, right: rect.right, width: rect.width, height: rect.height,
        tableWidth: tableRect.width, clientWidth: region.clientWidth,
        overflowX: getComputedStyle(region).overflowX, clippedCells,
      };
    });
    assert(matchedV5Geometry.width > 0 && matchedV5Geometry.height > 0,
      `${name}: V5 table has no rendered area`);
    assert(matchedV5Geometry.left >= -1 && matchedV5Geometry.right <= width + 1,
      `${name}: V5 table region exceeds the viewport`);
    assert(matchedV5Geometry.clippedCells.length === 0,
      `${name}: V5 table text exceeds cells: ${matchedV5Geometry.clippedCells.join(", ")}`);
    if (matchedV5Geometry.tableWidth > matchedV5Geometry.clientWidth + 1) {
      assert(["auto", "scroll"].includes(matchedV5Geometry.overflowX), `${name}: wide V5 table must scroll within its region`);
    }
    const matchedV5Disclosure = page.getByText("V5 replay and receipt identities", { exact: true });
    const matchedV5Details = page.locator("details.performance-identities").filter({ has: matchedV5Disclosure });
    assert(await matchedV5Details.count() === 1, `${name}: exactly one V5 receipt disclosure`);
    await matchedV5Disclosure.click();
    for (const [label, digest] of [
      ["Paired replay summary SHA-256", "11c761a2e4408fc29a822a29f87ed10995c5799e407fc3233068f197836974a9"],
      ["Shared plan SHA-256", "b25fa9824afeac3b5531aa4748b708abfd8edf87e839a7e862d68f1d41f18175"],
      ["Ferric receipt SHA-256", "f6e4837cf1a613e41e28ec311476fbe77a9b1fab9c133a2c7140a8e6cc1be37a"],
      ["vLLM receipt SHA-256", "dc98f64b11228002526fe47184b3fe14de419347f45e467692d6ebe9991c94f2"],
    ]) {
      const term = matchedV5Details.getByText(label, { exact: true });
      assert(await term.count() === 1 && await term.isVisible(), `${name}: V5 receipt label missing: ${label}`);
      assert(await term.evaluate((node) => node.nextElementSibling.textContent.trim()) === digest,
        `${name}: V5 receipt digest changed: ${label}`);
    }
    await matchedV5Disclosure.click();

    const matchedTable = page.getByRole("region", { name: "Matched 128/128: client latency and output rate", exact: true });
    assert(await matchedTable.locator("tbody tr").count() === 2, `${name}: exactly two admitted matched engines`);
    assert((await matchedTable.textContent()).includes("1.879805"), `${name}: Ferric measured rate`);
    assert(!(await matchedTable.textContent()).includes("SGLang"), `${name}: failed SGLang must not enter numeric table`);
    await page.getByText("Matched-cell percentiles and evidence", { exact: true }).click();
    assert(await page.getByRole("region", { name: "Matched 128/128: descriptive single-cohort percentiles", exact: true })
      .locator("tbody tr").count() === 2, `${name}: both matched percentile rows`);
    const provenanceDisclosure = page.getByText("Exact identities and archived evidence", { exact: true });
    await provenanceDisclosure.click();
    const generatorLabel = page.getByText("Matched MFMA ledger generator source SHA-256", { exact: true });
    assert(await generatorLabel.count() === 1 && await generatorLabel.isVisible(), `${name}: missing generator-source label`);
    assert(await page.getByText("Matched MFMA ledgerCanonicalId", { exact: true }).count() === 0,
      `${name}: misleading canonical-ledger label remains`);
    assert(await generatorLabel.evaluate((node) => node.nextElementSibling.textContent)
      === "8719cb980590c41c7c8751b95117a858b04c42912964c811bebf7e2c4d69bbea",
    `${name}: generator digest changed`);
    const roundGenerator = page.getByText("Concurrent matrix performance-ledger generator source SHA-256", { exact: true });
    const headGenerator = page.getByText("FP32 head summary generator source SHA-256", { exact: true });
    assert(await headGenerator.isVisible(), `${name}: missing FP32 summary-source label`);
    assert(await headGenerator.evaluate((node) => node.nextElementSibling.textContent)
      === "1d0cecc87021798aa7aba7dca2fa4085a8c92fed66da5bb230f06e6e017ae6d0",
    `${name}: FP32 generator digest changed`);
    const headReport = page.getByText("FP32 head generated three-case summary SHA-256", { exact: true });
    assert(await headReport.isVisible(), `${name}: missing FP32 generated-summary label`);
    assert(await headReport.evaluate((node) => node.nextElementSibling.textContent)
      === "38ba909a53c3b6d4ae491b22ffbe56927ff50b484dd445fc048eb8184f55b97a",
    `${name}: FP32 summary digest changed`);
    assert(await roundGenerator.isVisible(), `${name}: missing concurrent generator-source label`);
    assert(await roundGenerator.evaluate((node) => node.nextElementSibling.textContent)
      === "6da3b95acda3332bfa27a1c51685dda316d5f188615b9b939554d5eef8c9085e",
    `${name}: concurrent generator digest changed`);
    const roundReport = page.getByText("Concurrent matrix TP8 generated three-mode report SHA-256", { exact: true });
    assert(await roundReport.isVisible(), `${name}: missing concurrent report label`);
    assert(await roundReport.evaluate((node) => node.nextElementSibling.textContent)
      === "d2e86b755c7b76915888f26ec4a7049e71981cb11b7e0ef43352130539575b64",
    `${name}: concurrent report digest changed`);
    assert(await page.getByText("1xTP8 canonical expectation SHA-256", { exact: true }).isVisible(),
      `${name}: canonical replica-expectation label changed`);
    await provenanceDisclosure.click();

    const latencyDisclosure = page.getByText("All single-run request latencies", { exact: true });
    await latencyDisclosure.click();

    const replicaDisclosure = page.getByText("All replica-cohort request latencies", { exact: true });
    await replicaDisclosure.click();
    const currentDisclosure = page.getByText("All accepted current-controller request latencies", { exact: true });
    await currentDisclosure.click();
    const roundDisclosure = page.getByText("All concurrent-matrix request latencies", { exact: true });
    const headDisclosure = page.getByText("All FP32-head request latencies", { exact: true });
    await headDisclosure.click();
    for (const [caption, count] of [["FP32 head: three separate process-window observations", 3],
      ["FP32 head: only the two approved candidate-to-baseline pairs", 2],
      ["FP32 head: all 12 named request latencies", 12]]) {
      assert(await page.getByRole("region", { name: caption, exact: true }).locator("tbody tr").count() === count,
        `${name}: FP32 table cardinality: ${caption}`);
    }
    await headDisclosure.click();
    await roundDisclosure.click();
    assert(await page.getByRole("region", { name: "Concurrent peer matrix: all 24 named request latencies", exact: true })
      .locator("tbody tr").count() === 24, `${name}: missing concurrent matrix request identities`);
    await roundDisclosure.click();
    assert(await page.getByRole("region", { name: "Current-controller compatibility: accepted named request latencies", exact: true })
      .locator("tbody tr").count() === 12, `${name}: missing accepted current-controller request identities`);
    await currentDisclosure.click();
    const repeatedDisclosure = page.getByText("All repeated MFMA request latencies", { exact: true });
    await repeatedDisclosure.click();
    const tp1RepeatedDisclosure = page.getByText("All repeated TP1 residual request latencies", { exact: true });
    await tp1RepeatedDisclosure.click();
    assert(await page.getByRole("region", { name: "Repeated TP1 residual pair: named request means and ranges", exact: true })
      .locator("tbody tr").count() === 8, `${name}: missing repeated TP1 residual request identities`);
    await tp1RepeatedDisclosure.click();
    assert(await page.getByRole("region", { name: "Repeated MFMA pair: named request means and ranges", exact: true })
      .locator("tbody tr").count() === 8, `${name}: missing repeated MFMA request identities`);
    await repeatedDisclosure.click();
    const wideDisclosure = page.getByText("All wide-policy request latencies", { exact: true });
    await wideDisclosure.click();
    const peerDisclosure = page.getByText("All source-matched peer request latencies", { exact: true });
    await peerDisclosure.click();
    assert(await page.getByRole("region", { name: "Source-matched peer controls: per-request latency", exact: true })
      .locator("tbody tr").count() === 16, `${name}: missing source-matched peer request identities`);
    await peerDisclosure.click();
    assert(await page.getByRole("region", { name: "Wide row-policy pair: eight named requests per policy", exact: true })
      .locator("tbody tr").count() === 16, `${name}: missing wide-policy request identities`);
    await wideDisclosure.click();
    const cohortCount = await page.evaluate(() => window.FERRIC_PERFORMANCE.replicaCohorts.profiles.length);
    assert(await page.getByRole("region", { name: "Replica cohorts: eight named requests per layout", exact: true })
      .locator("tbody tr").count() === 8 * cohortCount, `${name}: missing replica request identities`);
    await replicaDisclosure.click();

    for (const [caption, rows] of [["Single-run attention operation groups: host intervals, not GPU durations", 7],
      ["Live wave/v11 HTTP: separate admitted context8192 cohorts", 2],
      ["Residual-tail native diagnostic: n=2 per exact binary", 2],
      ["Ordered-submission correctness: fixed native packet and ownership bounds", 4],
      ["Submission full128 ABBA: native host means and run ranges", 2],
      ["Combined attention full128 ABBA: native host means and run ranges", 2],
      ["Matched MFMA pair: process windows", 2],
      ["Concurrent peer matrix: six process-window observations", 6],
      ["Concurrent peer matrix: separate workload-rate baselines", 2],
      ["Repeated MFMA pair: mean and observed range", 2],
      ["Repeated TP1 residual pair: mean and observed range", 2],
      ["Current-controller compatibility: rejected profiles", 2],
      ["Current-controller compatibility: accepted process windows", 3],
      ["Current-controller compatibility: observed rows and profiles", 3],
      ["Cumulative MFMA and pruning: process windows", 1], ["Cumulative MFMA and pruning: request latencies", 4],
      ["Matched TP1 residual pair: process windows", 2], ["Matched TP1 residual pair: request latencies", 8],
      ["Setup transpose model pair: process windows", 2], ["Setup transpose model pair: request latencies", 8],
      ["Matched MFMA pair: request latencies", 8], ["Original peer checkpoint: process windows", 2],
      ["Original peer checkpoint: request latencies", 8], ["CPU-only transpose helper, sums of per-case medians", 3]]) {
      assert(await page.getByRole("region", { name: caption, exact: true }).locator("tbody tr").count() === rows,
        `${name}: missing measurement rows: ${caption}`);
    }
    assert(await page.getByRole("region", { name: "Single-run ablation latencies by request identity", exact: true })
      .locator("tbody tr").count() === 20, `${name}: missing single-run per-request latencies`);
    assert(await page.locator("[data-live-http-teams] .team-item").count() === 4,
      `${name}: missing current live HTTP/kernel team checkpoints`);
    assert(await page.locator("[data-c1-checkpoint] .team-item").count() === 2,
      `${name}: missing separate C1/v13 checkpoints`);
    assert(await page.locator("[data-live-c1-checkpoint] .team-item").count() === 4,
      `${name}: missing separate V13/V14/V15 and runtime checkpoints`);
    assert(await page.getByRole("region", { name: "C1 live route: same-binary finite HTTP cohorts", exact: true })
      .locator("tbody tr").count() === 2, `${name}: missing admitted C1 live HTTP pair`);
    const liveC1Disclosure = page.getByText("C1 live HTTP percentiles and evidence", { exact: true });
    await liveC1Disclosure.click();
    assert(await page.getByRole("region", { name: "C1 live route: retained HTTP percentiles", exact: true })
      .locator("tbody tr").count() === 2, `${name}: missing C1 live HTTP percentiles`);
    await liveC1Disclosure.click();
    assert(await page.getByRole("region", { name: "C1 layer projection: same-binary n=2 diagnostic", exact: true })
      .locator("tbody tr").count() === 2, `${name}: missing same-binary C1 diagnostic rows`);
    for (const label of ["Stopped", "Compiler rejected", "Native correctness accepted"]) {
      assert(await page.locator("[data-live-http-teams] .state-tag").filter({ hasText: new RegExp(`^${label}$`) }).count() === 1,
        `${name}: missing exact current checkpoint label: ${label}`);
    }
    const liveHttpDisclosure = page.getByText("Live HTTP percentiles and evidence", { exact: true });
    await liveHttpDisclosure.click();
    assert(await page.getByRole("region", { name: "Live wave/v11 HTTP: descriptive single-cohort percentiles", exact: true })
      .locator("tbody tr").count() === 2, `${name}: missing admitted live HTTP percentiles`);
    assert(await page.getByText("Ordered HTTP measurement is awaiting completed independent replay; no ordered timing is published yet.",
      { exact: true }).count() === 0, `${name}: obsolete pending ordered result`);
    await liveHttpDisclosure.click();
    await latencyDisclosure.click();

    if (screenshotRoot) {
      if (name === "desktop" || name === "mobile") {
        await page.evaluate(() => { document.documentElement.style.scrollBehavior = "auto"; });
        for (const [heading, suffix] of [["Authenticated resident serving", "resident-checkpoint"],
          ["Fixed-safe gate/up: no useful measured gain", "fixed-safe-v10-progress"],
          ["Prefill32: lower TTFT, higher TPOT", "prefill32-v7-progress"],
          ["Packet-tick attribution accepted", "packet-ticks-v7-progress"],
          ["Packed gate/up: accepted synthetic screen", "packed-gate-up-v7-progress"],
          ["Runtime wait and token programs: native correctness", "runtime-token-v8-progress"],
          ["Token client: full-model parity", "client-token-v8-progress"],
          ["Fresh HTTP: vLLM remains faster", "matched-http-v8-title"],
          ["Token model ABBA: no repeatable gain", "token-model-v8-title"],
          ["Packed model: CPU qualification in progress", "packed-model-v8-title"],
          ["Ordered64 HTTP: three accepted pairs, still slower", "ordered64-http-three-pairs"],
          ["V7 host diagnostic: attribution, not a speedup", "v7-host-diagnostic"],
          ["Attention attribution and validation", "attention-attribution"],
          ["Wave attention / v11: context8192 HTTP", "live-http"],
          ["Current parallel checkpoint", "live-teams"],
          ["C1 live route: context8192 HTTP", "live-c1-http"],
          ["Sharded argmax v13: static image checks", "v13-emission"],
          ["Query-hoist v14: finite native parity", "v14-native"],
          ["Wave RMSNorm v15: model correctness", "v15-native"],
          ["V14 canary and current runtime", "v14-runtime"],
          ["C1 layer projection: variable native diagnostic", "c1-diagnostic"],
          ["v13 ownership: standalone integer proof", "v13-proof"],
          ["Sharded argmax v13", "v13-host"],
          ["C1 live route", "c1-live-host"],
          ["Visible-token attention", "checkpoint-visible"],
          ["Cooperative KV append", "checkpoint-append"],
          ["Ordered residual tail", "checkpoint-residual"],
          ["Residual-tail diagnostic: decode regression", "residual-regression"],
          ["Ordered submission: host and correctness gates passed", "submission-correctness"],
          ["Ordered submission: full128 ABBA diagnostic", "submission-abba"],
          ["Combined attention / argmax: full128 ABBA", "attention-abba-table"]]) {
          await page.getByRole("heading", { name: heading, exact: true }).evaluate((node) => {
            const headerHeight = document.querySelector("header").getBoundingClientRect().height;
            window.scrollTo(0, window.scrollY + node.getBoundingClientRect().top - headerHeight - 20);
          });
          if (suffix === "client-token-v8-progress") {
            const span = await page.evaluate(() => ({
              top: window.scrollY + document.querySelector("#client-token-v8-progress-title").getBoundingClientRect().top,
              bottom: window.scrollY + document.querySelector("#runtime-token-v8-limits").getBoundingClientRect().bottom,
              width: window.innerWidth, height: window.innerHeight,
            }));
            assert(Number.isFinite(span.top) && Number.isFinite(span.bottom) && span.bottom > span.top,
              `${name}: bounded complete client capture span`);
            const frames = [];
            let covered = span.top;
            for (let frame = 0; covered < span.bottom - 0.5 && frame < 8; frame += 1) {
              const start = frame === 0 ? span.top : covered - 48;
              await page.evaluate((top) => {
                const header = Math.max(0, Math.min(window.innerHeight,
                  document.querySelector("header").getBoundingClientRect().bottom));
                window.scrollTo(0, top - header - 20);
              }, start);
              const view = await page.evaluate(() => ({
                top: window.scrollY + Math.max(0, Math.min(window.innerHeight,
                  document.querySelector("header").getBoundingClientRect().bottom)) + 20,
                bottom: window.scrollY + window.innerHeight - 8,
                width: window.innerWidth, height: window.innerHeight,
              }));
              assert(view.width === span.width && view.height === span.height
                && Number.isFinite(view.top) && Number.isFinite(view.bottom)
                && view.bottom > view.top && view.bottom - view.top <= view.height
                && view.top <= start + 1 && view.bottom > covered + 1,
              `${name}: client frames must progress with no hidden content gap`);
              const file = frame === 0 ? `${name}-${suffix}.png`
                : `${name}-${suffix}-part-${String(frame + 1).padStart(2, "0")}.png`;
              await page.screenshot({ path: join(screenshotRoot, file) });
              frames.push({ file, top: view.top, bottom: view.bottom });
              covered = Math.min(span.bottom, view.bottom);
            }
            assert(covered >= span.bottom - 0.5 && frames.length > 0 && frames.length <= 8,
              `${name}: every client claim must appear in bounded overlapping frames`);
            clientTokenCaptures.push({ name, ...span, frames });
            continue;
          }
          if (["prefill32-v7-progress", "packet-ticks-v7-progress", "packed-gate-up-v7-progress",
            "runtime-token-v8-progress"].includes(suffix)) {
            const last = {
              "prefill32-v7-progress": "#prefill32-v7-limits",
              "packet-ticks-v7-progress": "#packet-ticks-v7-progress",
              "packed-gate-up-v7-progress": "#packed-gate-up-v7-limits",
              "runtime-token-v8-progress": "#runtime-token-v8-progress",
            }[suffix];
            assert(await page.locator(last).evaluate((node) => node.getBoundingClientRect().bottom <= window.innerHeight - 8),
              `${name}: complete current progress claims must fit in their focused capture`);
          }
          await page.screenshot({ path: join(screenshotRoot, `${name}-${suffix}.png`) });
        }
        const captureKvTables = async () => {
          await page.locator("#kv-copy-v7-j1-title").evaluate((node) => {
            const headerHeight = document.querySelector("header").getBoundingClientRect().height;
            window.scrollTo(0, window.scrollY + node.getBoundingClientRect().top - headerHeight - 20);
          });
          const frame = await page.evaluate(() => ({
            top: document.querySelector("#kv-copy-v7-j1-title").getBoundingClientRect().top,
            bottom: document.querySelector("#kv-copy-v7-repeat-results").getBoundingClientRect().bottom,
            header: document.querySelector("header").getBoundingClientRect().bottom,
            height: window.innerHeight,
          }));
          assert(frame.top >= frame.header && frame.bottom <= frame.height - 8,
            `${name}: both complete KV-copy campaign tables must fit in the retained capture`);
        };
        await captureKvTables();
        await page.screenshot({ path: join(screenshotRoot, `${name}-kv-copy-v7-first-screen.png`) });
        await page.locator('nav a[href="#performance"]').click();
        await page.screenshot({ path: join(screenshotRoot, `${name}-performance.png`) });
        if (name === "mobile") {
          for (const table of kvCopyV7Tables) {
            await table.evaluate((region) => { region.scrollLeft = region.scrollWidth; });
            assert(await table.evaluate((region) => region.scrollWidth <= region.clientWidth
              || region.scrollLeft > 0), `${name}: both KV-copy right-edge views must scroll when required`);
          }
          await captureKvTables();
          await page.screenshot({ path: join(screenshotRoot, `${name}-kv-copy-v7-first-screen-right.png`) });
          for (const table of kvCopyV7Tables) {
            await table.evaluate((region) => { region.scrollLeft = 0; });
          }
          await september22Table.evaluate((region) => {
            region.scrollLeft = region.scrollWidth;
            const headerHeight = document.querySelector("header").getBoundingClientRect().height;
            window.scrollTo(0, window.scrollY + region.getBoundingClientRect().top - headerHeight - 20);
          });
          await page.screenshot({ path: join(screenshotRoot, `${name}-ordered64-http-three-pairs-right.png`) });
          await september22Table.evaluate((region) => { region.scrollLeft = 0; });
        }
        await prefill32V7Disclosure.click();
        await prefill32V7Details.evaluate((node) => {
          const headerHeight = document.querySelector("header").getBoundingClientRect().height;
          window.scrollTo(0, window.scrollY + node.getBoundingClientRect().top - headerHeight - 20);
        });
        assert(await prefill32V7Details.evaluate((node) => {
          const box = node.getBoundingClientRect();
          const header = document.querySelector("header").getBoundingClientRect().bottom;
          return box.top >= header && box.bottom <= window.innerHeight - 8;
        }), `${name}: all September 23 evidence identities must fit in the retained capture`);
        await page.screenshot({ path: join(screenshotRoot, `${name}-prefill32-v7-evidence.png`) });
        await prefill32V7Disclosure.click();
        await kvCopyV7Disclosure.click();
        await kvCopyV7Details.evaluate((node) => {
          const headerHeight = document.querySelector("header").getBoundingClientRect().height;
          window.scrollTo(0, window.scrollY + node.getBoundingClientRect().top - headerHeight - 20);
        });
        assert(await kvCopyV7Details.evaluate((node) => {
          const box = node.getBoundingClientRect();
          const header = document.querySelector("header").getBoundingClientRect().bottom;
          return box.top >= header && box.bottom <= window.innerHeight - 8;
        }), `${name}: both KV-copy campaign evidence identities must fit in the retained capture`);
        await page.screenshot({ path: join(screenshotRoot, `${name}-kv-copy-v7-evidence.png`) });
        await kvCopyV7Disclosure.click();
        await september22Disclosure.click();
        await september22Details.evaluate((node) => {
          const headerHeight = document.querySelector("header").getBoundingClientRect().height;
          window.scrollTo(0, window.scrollY + node.getBoundingClientRect().top - headerHeight - 20);
        });
        await page.screenshot({ path: join(screenshotRoot, `${name}-september22-evidence.png`) });
        await september22Disclosure.click();
        for (const [label, suffix] of [
          ["Combined attention / argmax: host and native gates passed", "attention-composition-host"],
          ["Combined attention / argmax: full128 ABBA diagnostic", "attention-composition-abba"],
          ["TP1 attention: eight exact native fixtures", "attention-fixtures"],
          ["Paired K4: two fresh native passes", "paired-native"],
          ["Wave plus ordered: short-canary ABBA", "wave-ordered"],
          ["Opt-in argmax route: host gate passed", "argmax-route"],
          ["Native argmax: full 128-output pair", "argmax-full128"],
          ["Native argmax: short ABBA diagnostics", "argmax-short8"],
          ["Retirement harness: rejected attempt preserved", "argmax-rejected"],
          ["Ordered runtime: diagnostic counters only", "runtime-counters"],
        ]) {
          await page.getByText(label, { exact: true }).evaluate((node) => {
            const headerHeight = document.querySelector("header").getBoundingClientRect().height;
            window.scrollTo(0, window.scrollY + node.getBoundingClientRect().top - headerHeight - 20);
          });
          await page.screenshot({ path: join(screenshotRoot, `${name}-${suffix}.png`) });
        }
        if (!compactScreenshots) {
          for (const [heading, suffix] of [["MFMA R1 checkpoint: faster requests, slower startup", "mfma"],
            ["FP32 head on TP1: faster requests, slower startup", "fp32-head"],
            ["Concurrent peer rounds: matched TP2 and TP8", "concurrent-matrix"],
            ["Eight-GPU allocation cohorts: 64 outputs", "replicas"],
            ["True 32-row execution: a latency tradeoff", "wide"],
            ["Current-controller combinations: mixed correctness", "current"],
            ["MFMA repeated: request gains, startup cost", "mfma-repeated"],
            ["Source-matched peer controls: a regression", "peer-controls"],
            ["Setup transpose: full-model observation", "transpose-model"],
            ["Original peer checkpoint: correctness, not a speedup", "peer"], ["CPU transpose helper only", "transpose"]]) {
            await page.getByRole("heading", { name: heading, exact: true }).evaluate((node) => {
              const headerHeight = document.querySelector("header").getBoundingClientRect().height;
              window.scrollTo(0, window.scrollY + node.getBoundingClientRect().top - headerHeight - 20);
            });
            await page.screenshot({ path: join(screenshotRoot, `${name}-${suffix}.png`) });
          }
          await page.getByRole("heading", { name: "Single-repetition ablations", exact: true }).scrollIntoViewIfNeeded();
          await page.screenshot({ path: join(screenshotRoot, `${name}-ablations.png`) });
        }
        await page.evaluate(() => window.scrollTo(0, 0));
        await page.screenshot({ path: join(screenshotRoot, `${name}-overview.png`) });
      }
      if (!compactScreenshots) await page.screenshot({ path: join(screenshotRoot, `${name}.png`), fullPage: true });
    }
    await page.close();
  }
} finally {
  await browser.close();
}

if (screenshotRoot) {
  assert(clientTokenCaptures.length === 2
    && clientTokenCaptures.map((row) => row.name).join(",") === "desktop,mobile",
  "Both current client capture spans must be retained");
  await writeFile(join(screenshotRoot, "client-token-v8-capture.json"), JSON.stringify({
    schema: "FerricClientTokenPagesCaptureV1", views: clientTokenCaptures,
    all_claims_covered: true, layout_modified: false,
  }, null, 2) + "\n", { flag: "wx", mode: 0o600 });
}

if (process.env.FERRIC_EXHAUSTIVE_WIDTHS === "1") {
  const sweepBrowser = await chromium.launch({ headless: true, args: ["--disable-gpu"] });
  try {
    const page = await sweepBrowser.newPage({ viewport: { width: 320, height: 900 } });
    await page.goto(pageUrl, { waitUntil: "load" });
    await page.waitForFunction(
      (selectors) => selectors.every((selector) => document.querySelector(selector)?.children.length),
      dynamicRoots,
    );
    await page.locator("#fixed-safe-v10-evidence").evaluate((details) => { details.open = true; });
    for (let width = 320; width <= 1440; width += 1) {
      await page.setViewportSize({ width, height: 900 });
      const result = await page.evaluate(() => {
        const viewportClipping = [...document.querySelectorAll(".repo-link, .state-tag")]
          .filter((element) => !element.closest(".transition-table-wrap"))
          .map((element) => {
            const rect = element.getBoundingClientRect();
            return { label: element.textContent.trim(), left: rect.left, right: rect.right };
          })
          .filter(({ left, right }) => left < -1 || right > window.innerWidth + 1);
        const authorityChildOverlaps = [...document.querySelectorAll(".authority-item")]
          .map((item, index) => {
            const [tag, detail] = item.children;
            if (!tag || !detail) return null;
            const tagRect = tag.getBoundingClientRect();
            const detailRect = detail.getBoundingClientRect();
            const overlaps =
              tagRect.left < detailRect.right - 0.5 &&
              tagRect.right > detailRect.left + 0.5 &&
              tagRect.top < detailRect.bottom - 0.5 &&
              tagRect.bottom > detailRect.top + 0.5;
            return overlaps
              ? {
                  index,
                  tag: tag.textContent.trim(),
                  tagRight: tagRect.right,
                  detailLeft: detailRect.left,
                }
              : null;
          })
          .filter(Boolean);
        return {
          overflow:
            Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) -
            window.innerWidth,
          viewportClipping,
          sections: [...document.querySelectorAll("main > section")].map((section) => {
            const rect = section.getBoundingClientRect();
            return { id: section.id || section.className, top: rect.top, bottom: rect.bottom };
          }),
          authorityChildOverlaps,
        };
      });
      assert(result.overflow <= 1, `${width}px sweep: page has horizontal overflow`);
      assert(
        result.viewportClipping.length === 0,
        `${width}px sweep: clipped status or repository control: ${JSON.stringify(result.viewportClipping)}`,
      );
      assert(
        result.authorityChildOverlaps.length === 0,
        `${width}px sweep: authority legend children overlap: ${JSON.stringify(result.authorityChildOverlaps)}`,
      );
      result.sections.slice(1).forEach((section, index) => {
        const previous = result.sections[index];
        assert(
          section.top >= previous.bottom - 1,
          `${width}px sweep: section ${section.id} overlaps ${previous.id}`,
        );
      });
    }
    await page.close();
  } finally {
    await sweepBrowser.close();
  }
  console.log("Validated every Ferric Pages width from 320px through 1440px.");
}

console.log("Validated rendered Ferric Pages at 1440px, 1027px, 981px, 800px, 710px, 701px, 390px, and 320px.");
