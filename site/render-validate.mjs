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
  "28b925a",
  "5a6d775",
  "c1b9590",
  "7521cdc",
  "1ddcd36",
  "553334d",
  "6df1f2f",
  "240bb3d",
  "f709a5d",
  "23f326a",
  "23 unadmitted bootstrap runtime bodies",
  "no current whole-tree",
  "strict Verus 81 verified / 0 errors",
  "proof tests 26/26",
  "source gate passes 28/28",
  "21/21",
  "4d2bcd1",
  "Exact v76",
  "Status 0",
  "415,660-byte Kernel IR V9 handoff",
  "26 GuardedStore operations",
  "103,616-byte",
  "all 12 kernels",
  "exact replay",
  "31a15c036261f0d7d2ab7027e709ea6c01c16f9654bc3d6e248dca3994a12282",
  "c270a528439df199cdae59b4cffda5566ad8a1d24d7029b2b58a519a4aef5b94",
  "6af8bd8292c5c108ea77e2416fb1325f11507d471254f2cc9607b6ae8be5035a",
  "fc8469775585a165c9674ff25683ee0676fd0e6c11a1885e1dde26cdcea392d5",
  "engine 689 passed / 7 ignored",
  "adapter 67 passed / 2 ignored",
  "zero failures",
  "ac5c60dfde5bdb4eae1d5ce1f1f8cc6b096afae2a7c71895bdc0c8ceddfc669f",
  "d79f354d37d65fa47ff7cc1deeff43de6f30070f7d0c487fe15d60994ad8731a",
  "non-hardware",
  "exactly one 128-output window",
  "CLOCK_MONOTONIC_RAW",
  "checked-token causality",
  "required 20-window qualification",
  "Authority: none",
  "publication, load, and launch grants are false",
  "CurrentFerricDescriptorRoster",
  "before KFD",
  "GPU and VRAM state are unchanged",
  "Docker is inaccessible to this account",
  "no baseline launch was attempted",
  "608 verified and 0 errors",
  "S1/T128",
  "NoMatch",
  "live-source",
  "audited seven-file exact kernel/host ABI delta",
  "not final or public product integration",
  "smoke binary",
  "verified Qwen snapshot",
  "protected receipt/verifier service is undeployed",
  "symmetric memory",
  "MTP",
  "No hardware execution",
  "Progressing, no team blocker",
  "public fe2o3",
  "All 33 M1 exit gates remain open",
  "No Qwen token",
  "TTFT",
  "TPOT",
  "vLLM and SGLang baselines are absent",
  "Ferric owns",
  "fe2o3 owns",
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
    requiredClaims.forEach((claim) => {
      assert(result.currentText.includes(claim), `${name}: rendered current view is missing ${claim}`);
    });
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
