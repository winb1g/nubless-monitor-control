#!/usr/bin/env node
// Keeps the desktop UI on its design tokens (docs/design-system.md).
//
// Components may not carry their own colors, shadows, gradients, fonts, radii
// or durations: those live in apps/desktop/src/app.css as tokens. This script
// also checks that every var(--x) is defined somewhere, and that the two dark
// theme blocks in app.css redefine the same tokens.
//
// A line that really needs a literal (a drawing, a one-off timer) can say why
// with a trailing `lint-tokens: allow` comment. Run it with:
//   node scripts/lint-tokens.mjs        (or `npm run lint:tokens` in apps/desktop)

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = join(root, "apps/desktop/src");
const tokensFile = join(src, "app.css");

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    return statSync(p).isDirectory() ? walk(p) : [p];
  });
}
const files = walk(src).filter((f) => /\.(svelte|css)$/.test(f));
const problems = [];
const report = (file, line, msg) => problems.push(`${relative(root, file)}:${line}  ${msg}`);

// ---- 1. literals in components ------------------------------------------------
const RULES = [
  [/(?<![&\w/])#[0-9a-fA-F]{3,8}\b/, "raw hex color: use a color token"],
  [/\b(?:rgb|rgba|hsl|hsla|color-mix)\(/, "raw color function: use a color token"],
  [/\b(?:linear|radial|conic)-gradient\(/, "gradients are not part of the look"],
  [/\b(?:box|text)-shadow\s*:|drop-shadow\(/, "shadows are not part of the look"],
  [/font-family\s*:(?!\s*var\()/, "font-family must be var(--font) or var(--font-mono)"],
  [/border-radius\s*:(?!\s*(?:var\(|0\b|inherit))/, "border-radius must be a --r-* token"],
  [/(?:transition|animation)[^;{]*?(?<![\w-])\d*\.?\d+m?s\b/, "durations must be --instant/--fast/--base/--slow (or --hatch/--blink)"],
];
for (const file of files) {
  if (file === tokensFile) continue;
  readFileSync(file, "utf8")
    .split("\n")
    .forEach((text, i) => {
      if (text.includes("lint-tokens: allow")) return;
      for (const [re, msg] of RULES) if (re.test(text)) report(file, i + 1, msg);
    });
}

// ---- 2. every var(--x) is defined ------------------------------------------------
const all = files.map((f) => [f, readFileSync(f, "utf8")]);
const defined = new Set();
for (const [, text] of all) {
  for (const m of text.matchAll(/(?:^|[\s;{"'])(?:style:)?(--[a-z0-9-]+)\s*[:=]/g)) defined.add(m[1]);
}
for (const [file, text] of all) {
  text.split("\n").forEach((line, i) => {
    for (const m of line.matchAll(/var\((--[a-z0-9-]+)/g)) {
      if (m[1].endsWith("-")) continue; // a name built in the template: var(--fs-{size})
      if (!defined.has(m[1])) report(file, i + 1, `${m[1]} is not defined (renamed or removed token?)`);
    }
  });
}

// ---- 3. both dark blocks define the same tokens ------------------------------------
const css = readFileSync(tokensFile, "utf8");
const block = (open) => {
  const start = css.indexOf(open);
  if (start < 0) return null;
  const body = css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
  return new Set([...body.matchAll(/(--[a-z0-9-]+)\s*:/g)].map((m) => m[1]));
};
const viaMedia = block(':root:not([data-theme="light"])');
const viaAttr = block(':root[data-theme="dark"]');
if (!viaMedia || !viaAttr) {
  report(tokensFile, 1, "could not find both dark theme blocks");
} else {
  for (const n of viaMedia) if (!viaAttr.has(n)) report(tokensFile, 1, `${n} is in the dark media block but not in [data-theme="dark"]`);
  for (const n of viaAttr) if (!viaMedia.has(n)) report(tokensFile, 1, `${n} is in [data-theme="dark"] but not in the dark media block`);
}

if (problems.length) {
  console.error(problems.join("\n"));
  console.error(`\n${problems.length} design token problem${problems.length === 1 ? "" : "s"}. See docs/design-system.md.`);
  process.exit(1);
}
console.log(`design tokens ok (${files.length} files)`);
