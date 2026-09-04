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
  "[data-boundaries]",
  "[data-observation]",
  "[data-progress]",
  "[data-gates]",
];
const requiredClaims = [
  "31b6f49",
  "0c2b73b",
  "c9072b0",
  "951d48a",
  "5099cf3",
  "5c5ddef",
  "68bbe0a",
  "80aaaf3",
  "3cfb4de",
  "c58785c",
  "e745bc75c",
  "R33 V2 checker and full collector",
  "collector has not run",
  "construct and structurally validate its V2 observations",
  "validation command separately constructs the V2 comparison record and applies the gate",
  "fe2o3-production-build-config-v2",
  "source-isa-summary-v1",
  "12 Ferric-local Rust M1 kernel roots",
  "seven canonical source modules",
  "no prebuilt or vendor kernel dependency",
  "compiler, runtime, and KFD ownership remains in fe2o3",
  "model kernels and inference remain in Ferric",
  "Remote exact aggregate qualification",
  "zero aggregate outputs",
  "no HSACO",
  "No aggregate hardware execution",
  "Qwen serving",
  "vLLM/SGLang comparison",
  "grants no correctness or execution authority",
  "511 engine library tests",
  "84 capture tests",
  "145 doctests",
  "all-target strict clippy",
  "live hardware execution remain open",
  "smoke has not executed",
  "CURRENT=None",
  "GPU availability and baseline container access were not revalidated",
  "comparison not run",
  "no current aggregate or engineering HSACO, Qwen token, aggregate hardware execution",
  "All 33 M1 roadmap gates and all 17 assurance properties remain Open",
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
      /exact aggregate qualification (?:is )?(?:complete|qualified|green)/i,
      /(?:current|qualified|available) aggregate HSACO/i,
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
