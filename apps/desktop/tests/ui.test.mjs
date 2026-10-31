// Drives the real UI in a browser against the preview backend (the same demo
// monitor `npm run dev` shows), so the flows people rely on are checked without
// hardware: locks that say why, "Keep this change?", Undo, the AI interlock dialog.
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

const segment = (group, name) => page.getByRole("group", { name: group }).getByRole("button", { name, exact: true });
const prompt = () => page.getByRole("group", { name: "Keep this change?" });
const slider = (id) => page.locator(`input#slider-${id}`);

test("the demo monitor loads with its controls", async () => {
  await open();
  await page.getByText("27GM950B").first().waitFor();
  assert.equal(await slider("brightness").inputValue(), "80");
  assert.equal(await slider("contrast").inputValue(), "70");
});

test("hardware-confirmed controls need no switch", async () => {
  await open();
  assert.ok(await segment("Response Time", "Faster").isEnabled());
  assert.ok(await slider("black-stabilizer").isEnabled());
  assert.ok(await segment("AI Upscaling", "High").isEnabled());
  assert.ok(await segment("Color temperature", "Cool").isEnabled());
});

const row = (label) => page.locator("div.row", { hasText: label });
const lockOf = (label) => row(label).getByRole("button", { name: "Locked" });
const switchSync = async (to) => {
  await segment("Adaptive-Sync/VRR", to).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Switch" }).click();
  await page.clock.runFor(500);
};

test("Motion Blur Reduction is locked while Adaptive-Sync is on, and the lock says why", async () => {
  await open();
  assert.equal(await segment("Motion Blur Reduction", "On").getAttribute("aria-disabled"), "true");
  assert.equal(await row("Motion Blur Reduction").locator(".pop").count(), 0, "closed until asked");
  await lockOf("Motion Blur Reduction").click();
  assert.match(await row("Motion Blur Reduction").locator(".pop").innerText(), /Turn Adaptive-Sync off first/);
});

test("the reason is reachable by keyboard and by clicking the dead option", async () => {
  await open();
  const lock = lockOf("Motion Blur Reduction");
  // A screen reader hears the reason with the button, without opening anything.
  await lock.focus();
  const described = await lock.getAttribute("aria-describedby");
  assert.match(await page.locator("#" + described).innerText(), /only works while Adaptive-Sync is off/);
  await page.keyboard.press("Enter");
  const pop = row("Motion Blur Reduction").locator(".pop");
  await pop.waitFor();
  await page.keyboard.press("Escape");
  await pop.waitFor({ state: "hidden" });
  assert.ok(await lock.evaluate((el) => el === document.activeElement), "focus stays on the lock");
  // Clicking the option that can't be used explains why, instead of doing nothing.
  await segment("Motion Blur Reduction", "On").click({ force: true });
  await pop.waitFor();
  // A click elsewhere closes it.
  await page.mouse.click(4, 4);
  await pop.waitFor({ state: "hidden" });
});

test("Adaptive-Sync Off unlocks Motion Blur Reduction, and it changes without a question", async () => {
  await open();
  await switchSync("Off");
  assert.equal(await segment("Motion Blur Reduction", "On").getAttribute("aria-disabled"), "false");
  await segment("Motion Blur Reduction", "On").click();
  await page.clock.runFor(300);
  assert.equal(await segment("Motion Blur Reduction", "On").getAttribute("aria-pressed"), "true");
  assert.equal(await prompt().count(), 0, "it can't hide the picture");
});

test("turning Adaptive-Sync back on switches Motion Blur Reduction off, says so, and relocks it", async () => {
  await open();
  await switchSync("Off");
  await segment("Motion Blur Reduction", "On").click();
  await page.clock.runFor(300);
  await switchSync("Basic");
  assert.equal(await segment("Motion Blur Reduction", "On").getAttribute("aria-disabled"), "true");
  assert.equal(await segment("Motion Blur Reduction", "Off").getAttribute("aria-pressed"), "true");
  await page.getByText("Motion Blur Reduction turned off with it").waitFor();
});

test("the Adaptive-Sync warning says Motion Blur Reduction will switch off when it is on", async () => {
  await open();
  await switchSync("Off");
  await segment("Adaptive-Sync/VRR", "Basic").click();
  const dialog = page.getByRole("alertdialog");
  assert.doesNotMatch(await dialog.innerText(), /Motion Blur Reduction/, "nothing to switch off yet");
  await dialog.getByRole("button", { name: "Not now" }).click();
  await segment("Motion Blur Reduction", "On").click();
  await page.clock.runFor(300);
  await segment("Adaptive-Sync/VRR", "Basic").click();
  assert.match(await dialog.innerText(), /Motion Blur Reduction will switch off too/);
  await dialog.getByRole("button", { name: "Not now" }).click();
});

test("PBP/PIP locks AI Upscaling with a reason, and turning it off unlocks it", async () => {
  await open();
  assert.equal(await segment("AI Upscaling", "High").getAttribute("aria-disabled"), "false");
  await page.getByRole("switch", { name: "Simulate PBP/PIP" }).click();
  await page.clock.runFor(300);
  assert.equal(await segment("AI Upscaling", "High").getAttribute("aria-disabled"), "true");
  await lockOf("AI Upscaling").click();
  assert.match(await row("AI Upscaling").locator(".pop").innerText(), /not available while PBP\/PIP is on/);
  await page.getByRole("switch", { name: "Simulate PBP/PIP" }).click();
  await page.clock.runFor(300);
  assert.equal(await segment("AI Upscaling", "High").getAttribute("aria-disabled"), "false");
});

test("AI Upscaling and the color temperature presets change from the Picture section", async () => {
  await open();
  assert.equal(await segment("AI Upscaling", "Off").getAttribute("aria-pressed"), "true");
  await segment("AI Upscaling", "High").click();
  await page.clock.runFor(300);
  assert.equal(await segment("AI Upscaling", "High").getAttribute("aria-pressed"), "true");
  assert.equal(await prompt().count(), 0, "neither can hide the picture");
  assert.equal(await segment("Color temperature", "Custom").getAttribute("aria-pressed"), "true");
  await segment("Color temperature", "Warm").click();
  await page.clock.runFor(300);
  assert.equal(await segment("Color temperature", "Warm").getAttribute("aria-pressed"), "true");
  assert.equal(await prompt().count(), 0);
});

test("a risky change asks 'Keep this change?' and Keep leaves it", async () => {
  await open();
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
  await segment("Response Time", "Faster").click();
  await page.clock.runFor(300);
  await prompt().waitFor();
  await page.clock.runFor(4000);
  assert.ok(await prompt().isVisible(), "still waiting at 4.3 s");
  await page.clock.runFor(1500);
  await page.clock.runFor(300);
  await prompt().waitFor({ state: "hidden" });
  assert.equal(await segment("Response Time", "Fast").getAttribute("aria-pressed"), "true");
});

test("switching VRR sets expectations first, then offers a plain Undo, not Keep", async () => {
  await open();
  await segment("Adaptive-Sync/VRR", "Off").click();
  const dialog = page.getByRole("alertdialog");
  await dialog.waitFor();
  assert.match(await dialog.innerText(), /go black for a few seconds/);
  await dialog.getByRole("button", { name: "Not now" }).click();
  await page.clock.runFor(300);
  assert.equal(await segment("Adaptive-Sync/VRR", "Extended").getAttribute("aria-pressed"), "true");
  await segment("Adaptive-Sync/VRR", "Off").click();
  await dialog.getByRole("button", { name: "Switch" }).click();
  await page.clock.runFor(500);
  assert.equal(await segment("Adaptive-Sync/VRR", "Off").getAttribute("aria-pressed"), "true");
  assert.equal(await prompt().count(), 0, "no Keep prompt");
  await page.getByRole("button", { name: "Undo" }).click();
  await page.clock.runFor(500);
  assert.equal(await segment("Adaptive-Sync/VRR", "Extended").getAttribute("aria-pressed"), "true");
});

test("changing the same control again keeps the first 'before'", async () => {
  await open();
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

test("a slider's handle sits at its value on the monitor's own scale, even with a floor", async () => {
  await open();
  // Contrast's safe range starts at 20, but 70 is still 70% along the bar.
  const where = await page.evaluate(() => {
    const bar = document.querySelector("input#slider-contrast").parentElement.querySelector(".bar");
    const fill = bar.querySelector("i").getBoundingClientRect();
    const box = bar.getBoundingClientRect();
    return (fill.right - box.left) / box.width;
  });
  assert.ok(Math.abs(where - 0.7) < 0.03, `handle at ${where}`);
  assert.equal(await slider("contrast").inputValue(), "70");
});

test("with no monitor found, the empty state offers the demo", async () => {
  await open({ query: "?preview=empty" });
  await page.getByRole("button", { name: /demo/i }).first().waitFor();
});

test("an error stays on screen until dismissed, and the app stays usable", async () => {
  await open({ query: "?preview=fail-vrr" });
  await segment("Adaptive-Sync/VRR", "Off").click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Switch" }).click();
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

const tab = (n) => page.getByRole("button", { name: new RegExp(`^LG ULTRAGEAR\\+ \\(${n}\\)`) });

test("with three monitors, each tab selects its own monitor and values", async () => {
  await open({ query: "?monitors=3" });
  await tab(2).click();
  await page.clock.runFor(200);
  assert.equal(await tab(2).getAttribute("aria-pressed"), "true");
  assert.equal(await tab(1).getAttribute("aria-pressed"), "false");
  await slider("brightness").fill("40");
  await slider("brightness").dispatchEvent("change");
  await page.clock.runFor(1000);
  await tab(1).click();
  await page.clock.runFor(200);
  assert.equal(await slider("brightness").inputValue(), "80", "monitor 1 kept its own brightness");
  await tab(2).click();
  await page.clock.runFor(200);
  assert.equal(await slider("brightness").inputValue(), "40");
});

test("Change all together sends a change to every monitor", async () => {
  await open({ query: "?monitors=3" });
  await page.getByRole("switch", { name: "Change all monitors together" }).click();
  await page.clock.runFor(200);
  await page.getByText("Every change goes to all 3 monitors").waitFor();
  await slider("brightness").fill("30");
  await slider("brightness").dispatchEvent("change");
  await page.clock.runFor(1000);
  for (const n of [2, 3]) {
    await tab(n).click();
    await page.clock.runFor(200);
    assert.equal(await slider("brightness").inputValue(), "30", `monitor ${n}`);
  }
});

test("one monitor shows no link switch, and the tray offers a picker only with several", async () => {
  await open();
  assert.equal(await page.getByRole("switch", { name: "Change all monitors together" }).count(), 0);
  await ctx.close();
  await open({ hash: "#/tray", query: "?monitors=2", size: { width: 340, height: 560 } });
  const pick = page.getByRole("group", { name: "Monitor" });
  const second = pick.getByRole("button", { name: "LG ULTRAGEAR+ (2)" });
  await second.click();
  await page.clock.runFor(200);
  assert.equal(await second.getAttribute("aria-pressed"), "true");
});

test("each monitor shows the name the OS gives it, with its model and port", async () => {
  await open({ query: "?monitors=2" });
  await tab(2).waitFor();
  assert.match(await tab(2).innerText(), /^LG ULTRAGEAR\+ \(2\)\n.*· DP-2$/i);
  assert.match(await tab(2).getAttribute("title"), /Connected to DP-2/);
});

test("Revert after switching monitors puts back the monitor the change was made on", async () => {
  await open({ query: "?monitors=2" });
  await segment("Response Time", "Faster").click();
  await prompt().waitFor();
  await tab(2).click();
  await page.clock.runFor(500);
  assert.equal(await segment("Response Time", "Fast").getAttribute("aria-pressed"), "true", "monitor 2 never changed");
  await tab(1).click();
  await page.clock.runFor(500);
  assert.equal(await segment("Response Time", "Fast").getAttribute("aria-pressed"), "true", "monitor 1 was put back");
});
