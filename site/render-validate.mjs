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
  "1dc659b",
  "6df1f2f",
  "e8b9908",
  "eebdb38",
  "240bb3d",
  "f709a5d",
  "3d10825",
  "84cdb971",
  "72c78c6",
  "6b3a5ab",
  "23 unadmitted bootstrap runtime bodies",
  "no current whole-tree",
  "strict Verus 81 verified / 0 errors",
  "proof tests 26/26",
  "source gate passes 28/28",
  "597 passed",
  "164 doctests",
  "608 verified and 0 errors",
  "S1/T128",
  "NoMatch",
  "live-source",
  "exact-total loop bound is cleared",
  "27/27",
  "full ferric-qwen-kernels suite is green",
  "engine physical-recipe checks pass 7/7",
  "Exact v17 reaches genuine lowering",
  "sole current terminal blocker",
  "function 6 retained local/type (178,63,borrow for compiler intrinsic)",
  "General fe2o3 private-slot lowering is required",
  "status 1 with no output",
  "bfad31c3331c6a2b92de73a4d873b88c94a8b45dfb7eb3d835f7515f9ce69965",
  "smoke binary",
  "verified Qwen snapshot",
  "protected receipt/verifier service is undeployed",
  "symmetric memory",
  "MTP",
  "no hardware execution",
  "no HSACO",
  "Progressing, no team blocker",
  "Blocked on fe2o3 private-slot lowering",
  "All 33 M1 exit gates remain open",
  "No Qwen token",
  "TTFT",
  "TPOT",
  "vLLM/SGLang baseline",
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
