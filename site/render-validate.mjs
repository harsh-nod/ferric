import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { chromium } from "playwright";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const pageUrl = pathToFileURL(join(siteRoot, "index.html")).href;
const screenshotRoot = process.env.FERRIC_SCREENSHOT_DIR;
const viewports = [
  ["desktop", 1440, 1100],
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
  "CURRENT=None",
  "No authenticated current-source Qwen, hardware, numerical, or performance run exists",
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
    requiredClaims.forEach((claim) => {
      assert(result.currentText.includes(claim), `${name}: rendered current view is missing ${claim}`);
    });
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

console.log("Validated rendered Ferric Pages at 1440px, 390px, and 320px.");
