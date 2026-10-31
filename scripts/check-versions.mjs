// Fails if the app version differs between Cargo (Rust crates) and package.json
// (which tauri.conf.json reads). Bump both together.
// With a tag argument (`node scripts/check-versions.mjs v0.2.5`), also fails if the
// tag isn't "v" + that version, so a release can't ship installers named for an old one.
import { readFileSync } from "node:fs";

const cargo = readFileSync("Cargo.toml", "utf8").match(/\[workspace\.package\][^[]*?\nversion\s*=\s*"([^"]+)"/)?.[1];
const npm = JSON.parse(readFileSync("apps/desktop/package.json", "utf8")).version;
const tauri = JSON.parse(readFileSync("apps/desktop/src-tauri/tauri.conf.json", "utf8")).version;

const problems = [];
if (!cargo) problems.push("Cargo.toml has no [workspace.package] version");
if (cargo !== npm) problems.push(`Cargo.toml ${cargo} != apps/desktop/package.json ${npm}`);
if (tauri !== "../package.json") problems.push(`tauri.conf.json version should be "../package.json", is ${JSON.stringify(tauri)}`);
const tag = process.argv[2];
if (tag && tag !== `v${cargo}`) {
  problems.push(`tag ${tag} != v${cargo}; bump Cargo.toml and apps/desktop/package.json (and Cargo.lock) before tagging`);
}

if (problems.length) {
  console.error("Version mismatch:\n  " + problems.join("\n  "));
  process.exit(1);
}
console.log(`Versions in sync: ${cargo}`);
