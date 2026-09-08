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
  "Physical device-KV prefix reuse is M2",
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
]);
const currentRequiredClaims = [
  ...requiredClaims.filter((claim) => !retiredCurrentClaims.has(claim)),
  "d7d2",
  "2ba02a8",
  "724 engine targets",
  "171 engine doctests",
  "104 adapter targets",
  "33 source-gate tests",
  "Four narrow warmed",
  "zero allocations and reallocations",
  "public next-window path",
  "still allocates",
  "allocation-free claim is unaccepted",
  "20-window run was not attempted",
  "authenticated artifacts",
  "non-test production owner",
  "rustc-literal-escaper",
  "No current aggregate HSACO",
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
