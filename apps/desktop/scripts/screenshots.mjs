// Regenerates the README screenshots in docs/assets from the demo monitor
// (the browser preview backend, so no hardware is involved).
//
//   npm run screenshots
//
// Needs a Chromium. Playwright's own works after `npx playwright install
// chromium`; or point CHROME_PATH at any Chrome/Chromium binary.

import { createServer } from "vite";
import { chromium } from "playwright-core";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "../../../docs/assets");

const server = await createServer({ root: join(here, ".."), logLevel: "error", server: { port: 1499, strictPort: true } });
await server.listen();

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || undefined,
  args: ["--no-sandbox"],
});

async function shoot(file, { width, height, theme, hash = "" }) {
  const ctx = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 2, colorScheme: theme, reducedMotion: "reduce" });
  await ctx.addInitScript((t) => localStorage.setItem("nubless.theme", t), theme);
  const page = await ctx.newPage();
  await page.goto(`http://localhost:1499/${hash}`);
  await page.waitForTimeout(1800); // the demo monitor answers after a beat
  await page.screenshot({ path: join(out, file) });
  await ctx.close();
  console.log("wrote", file);
}

for (const theme of ["light", "dark"]) {
  await shoot(`window-${theme}.png`, { width: 1040, height: 990, theme });
  await shoot(`tray-${theme}.png`, { width: 340, height: 450, theme, hash: "#/tray" });
}

await browser.close();
await server.close();
