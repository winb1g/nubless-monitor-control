<script lang="ts">
  import { fly } from "svelte/transition";
  import { GitBranch, Bug, ShieldCheck, Heart, Scale } from "@lucide/svelte";
  import { ui } from "../store.svelte";
  import { inTauri } from "../api";

  const REPO = "https://github.com/winb1g/nubless-monitor-control";

  async function open(url: string) {
    if (inTauri) (await import("@tauri-apps/plugin-opener")).openUrl(url);
    else window.open(url, "_blank");
  }

  const firmware = $derived(ui.state?.controls.find((c) => c.id === "firmware")?.display);
  let taps = $state(0);
</script>

<div class="page">
  <section class="hero" in:fly={{ y: 12, duration: 320 }}>
    <button class="logo" class:wiggle={taps > 0} onclick={() => taps++} onanimationend={() => (taps = 0)} aria-label="nubless logo">
      <img src="/logo.svg" alt="" width="84" height="84" />
    </button>
    <h1>nubless</h1>
    <p class="muted">Never touch the joystick nub again.</p>
    <span class="ver">Version {ui.state?.version ?? "…"}{firmware ? ` · Monitor firmware ${firmware}` : ""}</span>
    <div class="actions">
      <button class="btn primary" onclick={() => open(REPO)}><GitBranch size={16} />View on GitHub</button>
      <button class="btn" onclick={() => open(`${REPO}/issues/new`)}><Bug size={16} />Report a problem</button>
    </div>
  </section>

  <div class="grid">
    <section class="card" in:fly={{ y: 12, duration: 320, delay: 60 }}>
      <span class="icon"><ShieldCheck size={18} /></span>
      <h2>Our promise</h2>
      <p class="muted">nubless will never brick your display, and you'll never have to reach for the monitor's menu to undo something it did. Values stay in safe ranges, features that don't mix stay apart, and your original settings are always one click away.</p>
      <button class="link" onclick={() => open(`${REPO}/blob/main/SAFETY.md`)}>Read the safety contract</button>
    </section>
    <section class="card" in:fly={{ y: 12, duration: 320, delay: 120 }}>
      <span class="icon"><Heart size={18} /></span>
      <h2>Thanks</h2>
      <p class="muted">Inspired by OpenLogi, MonitorControl and BetterDisplay. Talks to your monitor with ddc-hi. Built with Tauri and Svelte.</p>
    </section>
    <section class="card" in:fly={{ y: 12, duration: 320, delay: 180 }}>
      <span class="icon"><Scale size={18} /></span>
      <h2>Open source</h2>
      <p class="muted">MIT licensed. Not affiliated with LG Electronics. LG, LG Switch and UltraGear are trademarks of LG Electronics.</p>
    </section>
  </div>
</div>

<style>
  .page {
    padding: 34px 40px 80px;
    max-width: 960px;
  }
  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 6px;
    padding: 20px 0 30px;
  }
  .logo {
    border: none;
    background: none;
    padding: 0;
    margin-bottom: 10px;
    filter: drop-shadow(0 14px 24px rgba(108, 92, 255, 0.35));
    transition: transform var(--base) var(--spring);
  }
  .logo:hover {
    transform: translateY(-3px) rotate(-3deg);
  }
  .wiggle {
    animation: wiggle 500ms var(--spring);
  }
  @keyframes wiggle {
    25% {
      transform: rotate(-10deg) scale(1.06);
    }
    60% {
      transform: rotate(8deg) scale(1.04);
    }
  }
  h1 {
    font-size: 32px;
  }
  .ver {
    font-size: 12.5px;
    color: var(--text-3);
    background: var(--panel-2);
    padding: 4px 12px;
    border-radius: 99px;
    margin-top: 6px;
  }
  .actions {
    display: flex;
    gap: 10px;
    margin-top: 18px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 16px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .icon {
    width: 36px;
    height: 36px;
    border-radius: 11px;
    display: grid;
    place-items: center;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .link {
    align-self: flex-start;
    border: none;
    background: none;
    padding: 0;
    color: var(--accent-ink);
    font-weight: 600;
  }
</style>
