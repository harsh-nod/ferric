import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { chromium } from "playwright";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const pageUrl = pathToFileURL(join(siteRoot, "index.html")).href;
const screenshotRoot = process.env.FERRIC_SCREENSHOT_DIR;
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
  "Continuous batching and paged TP attention",
  "Persistent resident radix prefixes",
  "Batched Qwen with resident prefix reuse",
  "Single logical-tick observations only",
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

const browser = await chromium.launch({ headless: true });
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

    if (screenshotRoot) {
      await page.screenshot({ path: join(screenshotRoot, `${name}.png`), fullPage: true });
    }
    await page.close();
  }
} finally {
  await browser.close();
}

if (process.env.FERRIC_EXHAUSTIVE_WIDTHS === "1") {
  const sweepBrowser = await chromium.launch({ headless: true });
  try {
    const page = await sweepBrowser.newPage({ viewport: { width: 320, height: 900 } });
    await page.goto(pageUrl, { waitUntil: "load" });
    await page.waitForFunction(
      (selectors) => selectors.every((selector) => document.querySelector(selector)?.children.length),
      dynamicRoots,
    );
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
