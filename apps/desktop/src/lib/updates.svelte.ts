// Update flow: check → "Update and restart" → download with progress → install → relaunch.
// Uses Tauri's signed updater against GitHub Releases (`latest.json`).

import { api, inTauri } from "./api";
import { ui } from "./store.svelte";

export type Phase = "idle" | "checking" | "current" | "unpublished" | "available" | "downloading" | "installing" | "restarting" | "error";

export const upd = $state({
  phase: "idle" as Phase,
  version: "",
  notes: "",
  progress: 0,
  error: "",
  checkedAt: null as Date | null,
  /** Check on its own at start and every few hours. On unless the user turned it off. */
  auto: true,
  autoLoaded: false,
});

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let pending: any = null;

const RECHECK_MS = 6 * 60 * 60 * 1000;
let timers: ReturnType<typeof setTimeout>[] = [];

/** Mirror "an update is waiting" into the nav dot and the tray. */
function announce(version: string | null) {
  ui.updateBadge = version !== null;
  api().then((b) => b.setUpdateAvailable(version)).catch(() => {});
}

/** Read the saved choice and (re)start the quiet background checks to match it. */
export async function loadAutoUpdate() {
  try {
    upd.auto = await (await api()).getAutoUpdate();
  } catch {
    upd.auto = true;
  }
  upd.autoLoaded = true;
  scheduleChecks();
}

export async function setAutoUpdate(on: boolean) {
  upd.auto = await (await api()).setAutoUpdate(on);
  scheduleChecks();
}

function scheduleChecks() {
  timers.forEach(clearTimeout);
  timers = [];
  if (!upd.auto) return;
  const again = () => {
    void checkForUpdates();
    timers.push(setTimeout(again, RECHECK_MS));
  };
  timers.push(setTimeout(again, 2500));
}

export async function checkForUpdates() {
  if (upd.phase === "checking" || upd.phase === "downloading" || upd.phase === "installing") return;
  upd.phase = "checking";
  upd.error = "";
  try {
    if (!inTauri) {
      await new Promise((r) => setTimeout(r, 900));
      pending = { version: "0.2.0", body: "Preview of the update flow. The tray now steps through picture modes.", preview: true };
    } else {
      const { check } = await import("@tauri-apps/plugin-updater");
      pending = await check();
    }
    upd.checkedAt = new Date();
    if (pending) {
      upd.phase = "available";
      upd.version = pending.version;
      upd.notes = pending.body ?? "";
      announce(pending.version);
    } else {
      upd.phase = "current";
      announce(null);
    }
  } catch (e) {
    upd.checkedAt = new Date();
    const msg = String(e);
    // No release published yet: GitHub answers 404 for latest.json.
    if (/404|valid release JSON|Not Found/i.test(msg)) {
      upd.phase = "unpublished";
    } else {
      upd.phase = "error";
      upd.error = "Couldn't reach GitHub. Check your connection and try again.";
    }
  }
}

export async function installAndRestart() {
  if (!pending) return;
  upd.phase = "downloading";
  upd.progress = 0;
  try {
    if (pending.preview) {
      for (let p = 0; p <= 100; p += 4) {
        upd.progress = p;
        await new Promise((r) => setTimeout(r, 40));
      }
      upd.phase = "installing";
      await new Promise((r) => setTimeout(r, 900));
      upd.phase = "restarting";
      await new Promise((r) => setTimeout(r, 1200));
      upd.phase = "current";
      return;
    }
    let total = 0;
    let got = 0;
    await pending.downloadAndInstall((ev: { event: string; data?: { contentLength?: number; chunkLength?: number } }) => {
      if (ev.event === "Started") total = ev.data?.contentLength ?? 0;
      if (ev.event === "Progress") {
        got += ev.data?.chunkLength ?? 0;
        upd.progress = total ? Math.min(100, Math.round((got / total) * 100)) : 50;
      }
      if (ev.event === "Finished") upd.phase = "installing";
    });
    upd.phase = "restarting";
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch {
    upd.phase = "error";
    upd.error = "The update didn't finish. Nothing changed on your computer. Try again.";
  }
}
