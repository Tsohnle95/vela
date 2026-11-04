// Run with: ./node_modules/.bin/electron scripts/mockup-mobile-moon-smoke.cjs
const assert = require("node:assert/strict");
const { app, BrowserWindow } = require("electron");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const designs = [
  "04-nocturne-tidal-city-rose-night",
  "04-nocturne-tidal-city-rose-night-workflow-rose-terminal"
];
const viewports = [
  ["mobile-320x844", 320, 844],
  ["mobile-390x844", 390, 844],
  ["mobile-600x900", 600, 900],
  ["desktop-1440x900", 1440, 900]
];
const screenshotDirectory = process.env.VELA_MOON_SCREENSHOT_DIR
  ?? fs.mkdtempSync(path.join(os.tmpdir(), "vela-mobile-moon-"));

fs.mkdirSync(screenshotDirectory, { recursive: true });

const measureLayout = `(() => {
  const rect = (element) => {
    const { x, y, right, bottom, width, height } = element.getBoundingClientRect();
    return { x, y, right, bottom, width, height };
  };
  const textRect = (element) => {
    const range = document.createRange();
    range.selectNodeContents(element);
    return rect(range);
  };
  const hero = document.querySelector(".art-hero");
  const terrain = hero.querySelector(".atmosphere-terrain");
  const facadePaths = [...terrain.querySelectorAll(":scope > path")]
    .filter((path) => /^M0 (?:790v|810v)/.test(path.getAttribute("d")));
  const distantBuildings = [...hero.querySelectorAll(".atmosphere-distance > g > path")];
  const skylinePaths = [...facadePaths, ...distantBuildings];
  return {
    viewport: { width: innerWidth, height: innerHeight },
    hero: rect(hero),
    moon: rect(hero.querySelector(".atmosphere-sky circle[fill^='url(#planet-']")),
    heading: textRect(hero.querySelector("h1")),
    subtitle: textRect(hero.querySelector("h2")),
    cta: rect(hero.querySelector(".art-button")),
    navigation: rect(document.querySelector(".explore-nav")),
    skylineTop: Math.min(...skylinePaths.map((path) => path.getBoundingClientRect().top)),
    skylinePathCount: skylinePaths.length
  };
})()`;

function overlaps(first, second) {
  return first.x < second.right && first.right > second.x
    && first.y < second.bottom && first.bottom > second.y;
}

app.whenReady().then(async () => {
  const window = new BrowserWindow({
    show: false,
    width: 1440,
    height: 900,
    webPreferences: { backgroundThrottling: false }
  });
  const failures = [];

  try {
    for (const id of designs) {
      for (const [size, width, height] of viewports) {
        window.setContentSize(width, height);
        await window.loadFile(path.join(process.cwd(), "mockup-design/iterations/explore.html"), {
          query: { design: id }
        });
        await window.webContents.executeJavaScript("document.fonts.ready.then(() => true)");
        await new Promise((resolve) => setTimeout(resolve, 150));

        const result = await window.webContents.executeJavaScript(measureLayout);
        const screenshot = path.join(screenshotDirectory, `${id}-${size}.png`);
        fs.writeFileSync(screenshot, (await window.webContents.capturePage()).toPNG());

        let pagePassed = true;
        try {
          assert.ok(result.skylinePathCount > 0, "city skyline geometry should be present");
          assert.ok(result.moon.width >= 70, "moon should remain large enough to read");
          assert.ok(result.moon.x >= result.hero.x, "moon should not be clipped at the left edge");
          assert.ok(result.moon.right <= result.hero.right, "moon should not be clipped at the right edge");
          assert.ok(result.moon.y > result.navigation.bottom, "moon should clear the navigation");
          assert.ok(result.moon.bottom < result.hero.bottom, "moon should remain inside the hero");
          assert.ok(!overlaps(result.moon, result.heading), "moon should not cover the heading");
          assert.ok(!overlaps(result.moon, result.subtitle), "moon should not cover the subtitle");
          assert.ok(!overlaps(result.moon, result.cta), "moon should not cover the CTA");

          if (size.startsWith("mobile")) {
            assert.ok(
              result.moon.bottom + 10 < result.skylineTop,
              `mobile moon should be fully above the city roofs (moon bottom ${result.moon.bottom.toFixed(1)}px, skyline ${result.skylineTop.toFixed(1)}px)`
            );
          } else {
            assert.ok(result.moon.width > 150, "desktop moon scale should remain unchanged");
          }
        } catch (error) {
          pagePassed = false;
          failures.push(`${id} at ${size}: ${error.message}`);
        }

        console.log(`${pagePassed ? "PASS" : "FAIL"} ${id} ${size}: ${JSON.stringify(result)}`);
        console.log(`Screenshot: ${screenshot}`);
      }
    }
  } finally {
    window.destroy();
    app.quit();
  }

  if (failures.length) {
    throw new Error(`Mobile moon visibility assertions failed:\n- ${failures.join("\n- ")}`);
  }

  console.log(`All moon visibility assertions passed. Screenshots: ${screenshotDirectory}`);
}).catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
