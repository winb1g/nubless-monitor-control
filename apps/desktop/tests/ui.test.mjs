// Drives the real UI in a browser against the preview backend (the same demo
// monitor `npm run dev` shows), so the flows people rely on are checked without
// hardware: Labs gating, "Keep this change?", Undo, the AI interlock dialog.
//
//   npm run test:ui
//
// Needs a Chromium. Playwright's own works after `npx playwright-core install
// chromium`; or point CHROME_PATH at any Chrome/Chromium binary.

import { test, before, after, beforeEach, afterEach } from "node:test";
import assert from "node:assert/strict";
import { createServer } from "vite";
import { chromium } from "playwright-core";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const PORT = 1498;
const url = (hash = "", query = "") => `http://localhost:${PORT}/${query}${hash}`;

let server, browser, ctx, page, problems;

before(async () => {
  server = await createServer({ root, logLevel: "error", server: { port: PORT, strictPort: true } });
  await server.listen();
  browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || undefined,
    channel: process.env.CHROME_PATH ? undefined : "chrome",
    args: ["--no-sandbox"],
  });
});

after(async () => {
  await browser?.close();
  await server?.close();
});

beforeEach(() => {
  problems = [];
});

afterEach(async () => {
  await ctx?.close();
  assert.deepEqual(problems, [], "the page logged errors");
});

/** A fresh window onto the demo monitor, with the page's errors collected. */
async function open({ hash = "", query = "", size = { width: 1040, height: 900 }, colorScheme = "light" } = {}) {
  ctx = await browser.newContext({ viewport: size, colorScheme });
  await ctx.addInitScript(() => localStorage.setItem("nubless.theme", "light"));
  page = await ctx.newPage();
  page.on("pageerror", (e) => problems.push(String(e)));
  page.on("console", (m) => m.type() === "error" && problems.push(m.text()));
  await page.clock.install();
  await page.goto(url(hash, query));
  // The demo monitor answers after a beat.
  await page.clock.runFor(1000);
  return page;
}

const labsOn = async () => {
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: /settings/i }).click();
  await page.getByRole("switch", { name: "Labs" }).click();
  await page.clock.runFor(300);
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: /displays/i }).click();
};
const segment = (group, name) => page.getByRole("group", { name: group }).getByRole("button", { name, exact: true });
const prompt = () => page.getByRole("group", { name: "Keep this change?" });
const slider = (id) => page.locator(`input#slider-${id}`);

test("the demo monitor loads with its controls", async () => {
  await open();
  await page.getByText("27GM950B").first().waitFor();
  assert.equal(await slider("brightness").inputValue(), "80");
  assert.equal(await slider("contrast").inputValue(), "70");
});

test("Labs controls are locked until Labs is on, and the lock says why", async () => {
  await open();
  assert.ok(await segment("Response Time", "Faster").isDisabled());
  await labsOn();
  assert.ok(await segment("Response Time", "Faster").isEnabled());
  assert.ok(await slider("black-stabilizer").isEnabled());
});

test("a risky change asks 'Keep this change?' and Keep leaves it", async () => {
  await open();
  await labsOn();
  await segment("Response Time", "Faster").click();
  await page.clock.runFor(300);
  await prompt().waitFor();
  assert.match(await prompt().innerText(), /Response Time\s+Faster/);
  assert.match(await prompt().innerText(), /Reverts in \d s/);
  await prompt().getByRole("button", { name: "Keep" }).click();
  await prompt().waitFor({ state: "hidden" });
  assert.equal(await segment("Response Time", "Faster").getAttribute("aria-pressed"), "true");
  // Well past the window, it is still kept.
  await page.clock.runFor(10_000);
  assert.equal(await segment("Response Time", "Faster").getAttribute("aria-pressed"), "true");
});

test("Revert puts the old value back at once", async () => {
  await open();
  await labsOn();
  await segment("Response Time", "Faster").click();
  await page.clock.runFor(300);
  await prompt().getByRole("button", { name: "Revert" }).click();
  await page.clock.runFor(300);
  await prompt().waitFor({ state: "hidden" });
  assert.equal(await segment("Response Time", "Fast").getAttribute("aria-pressed"), "true");
  await page.getByText("Response Time put back").waitFor();
});

test("no answer in 5 seconds puts the old value back", async () => {
  await open();
  await labsOn();
  await segment("Adaptive-Sync/VRR", "Off").click();
  await page.clock.runFor(300);
  await prompt().waitFor();
  await page.clock.runFor(4000);
  assert.ok(await prompt().isVisible(), "still waiting at 4.3 s");
  await page.clock.runFor(1500);
  await page.clock.runFor(300);
  await prompt().waitFor({ state: "hidden" });
  assert.equal(await segment("Adaptive-Sync/VRR", "Extended").getAttribute("aria-pressed"), "true");
});

test("changing the same control again keeps the first 'before'", async () => {
  await open();
  await labsOn();
  await segment("Response Time", "Faster").click();
  await page.clock.runFor(300);
  await segment("Response Time", "Normal").click();
  await page.clock.runFor(300);
  await prompt().getByRole("button", { name: "Revert" }).click();
  await page.clock.runFor(300);
  // Not "Faster": the whole run of tweaks is put back as one change.
  assert.equal(await segment("Response Time", "Fast").getAttribute("aria-pressed"), "true");
});

test("a slider drag asks once, when it ends, and Revert restores where it started", async () => {
  await open();
  await labsOn();
  await slider("black-stabilizer").fill("90");
  await page.clock.runFor(1000);
  await prompt().waitFor();
  assert.match(await prompt().innerText(), /Black Stabilizer\s+90/);
  await prompt().getByRole("button", { name: "Revert" }).click();
  await page.clock.runFor(1000);
  assert.equal(await slider("black-stabilizer").inputValue(), "50");
});

test("a change that can't hide the picture gets an Undo toast, not a question", async () => {
  await open();
  await slider("brightness").fill("40");
  await page.clock.runFor(1000);
  assert.equal(await prompt().count(), 0);
  await page.getByText("Brightness updated").waitFor();
  await page.getByRole("button", { name: "Undo" }).click();
  await page.clock.runFor(1000);
  assert.equal(await slider("brightness").inputValue(), "80");
});

test("the question is a real control: named, reachable by keyboard, big enough to hit", async () => {
  await open();
  await labsOn();
  await segment("Response Time", "Faster").click();
  await page.clock.runFor(300);
  for (const name of ["Keep", "Revert"]) {
    const box = await prompt().getByRole("button", { name }).boundingBox();
    assert.ok(box.height >= 44, `${name} is ${box.height}px tall`);
  }
  await prompt().getByRole("button", { name: "Revert" }).focus();
  await page.keyboard.press("Enter");
  await page.clock.runFor(300);
  assert.equal(await segment("Response Time", "Fast").getAttribute("aria-pressed"), "true");
});

test("the question fits the tray's narrow window", async () => {
  await open({ hash: "#/tray", size: { width: 340, height: 450 } });
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
  assert.equal(overflow, false);
});

test("the prompt shows in dark mode with readable text", async () => {
  await open({ colorScheme: "dark" });
  await labsOn();
  await segment("Response Time", "Faster").click();
  await page.clock.runFor(300);
  await prompt().waitFor();
  const [fg, bg] = await prompt().evaluate((el) => {
    const s = getComputedStyle(el);
    return [s.color, s.backgroundColor];
  });
  assert.notEqual(fg, bg);
});

test("a picture mode that doesn't work with AI asks before turning AI off", async () => {
  await open();
  const ai = page.getByRole("switch", { name: /AI Scene Optimization/ });
  await ai.click();
  await page.clock.runFor(1500);
  assert.equal(await ai.getAttribute("aria-checked"), "true");

  await page.getByRole("button", { name: "Calibration 1" }).click();
  await page.clock.runFor(500);
  const dialog = page.getByRole("alertdialog");
  await dialog.waitFor();
  await dialog.getByRole("button", { name: "Not now" }).click();
  await page.clock.runFor(500);
  assert.equal(await ai.getAttribute("aria-checked"), "true", "'Not now' changes nothing");

  await page.getByRole("button", { name: "Calibration 1" }).click();
  await page.clock.runFor(500);
  await dialog.getByRole("button", { name: "Switch" }).click();
  await page.clock.runFor(1500);
  // AI is off, and in this mode its switch is replaced by a lock that says why.
  assert.equal(await ai.count(), 0);
  assert.equal(await page.getByRole("button", { name: "Calibration 1" }).getAttribute("aria-pressed"), "true");
});

test("every page opens without errors", async () => {
  await open();
  for (const name of [/settings/i, /updates/i, /about/i, /displays/i]) {
    await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name }).click();
    await page.clock.runFor(500);
  }
});

test("with no monitor found, the empty state offers the demo", async () => {
  await open({ query: "?preview=empty" });
  await page.getByRole("button", { name: /demo/i }).first().waitFor();
});

test("an error stays on screen until dismissed, and the app stays usable", async () => {
  await open({ query: "?preview=fail-vrr" });
  await labsOn();
  await segment("Adaptive-Sync/VRR", "Off").click();
  await page.clock.runFor(500);
  const error = page.getByText(/did not apply 'adaptive-sync'/);
  await error.waitFor();
  // Long enough to read it, copy it, or take a screenshot.
  await page.clock.runFor(30_000);
  assert.ok(await error.isVisible(), "the error is still there after 30 s");
  // Nothing else went dead.
  assert.ok(await slider("brightness").isEnabled());
  assert.ok(await segment("Response Time", "Fast").isEnabled());
  await page.getByRole("button", { name: "Dismiss" }).click();
  await page.clock.runFor(500);
  await error.waitFor({ state: "hidden" });
});
