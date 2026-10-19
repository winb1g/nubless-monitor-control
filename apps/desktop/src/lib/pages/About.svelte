<script lang="ts">
  import { rise } from "../motion";
  import { fly } from "svelte/transition";
  import { ui } from "../store.svelte";
  import Logo from "../components/Logo.svelte";
  import { inTauri } from "../api";

  const REPO = "https://github.com/winb1g/nubless-monitor-control";

  async function open(url: string) {
    if (inTauri) (await import("@tauri-apps/plugin-opener")).openUrl(url);
    else window.open(url, "_blank");
  }

  const firmware = $derived(ui.state?.controls.find((c) => c.id === "firmware")?.display);
</script>

<div class="page">
  <section class="hero" in:fly={rise}>
    <Logo size={56} />
    <h1>Never touch the joystick nub again.</h1>
    <span class="eyebrow">Version {ui.state?.version ?? "…"}{firmware ? ` · Monitor firmware ${firmware}` : ""}</span>
    <div class="actions">
      <button class="btn primary" onclick={() => open(REPO)}>View on GitHub</button>
      <button class="btn" onclick={() => open(`${REPO}/issues/new`)}>Report a problem</button>
    </div>
  </section>

  <div class="grid">
    <section in:fly={rise}>
      <h2>Our promise</h2>
      <p class="muted">nubless will never brick your display, and you'll never have to reach for the monitor's menu to undo something it did. Values stay in safe ranges, features that don't mix stay apart, and your original settings are always one click away.</p>
      <button class="link" onclick={() => open(`${REPO}/blob/main/SAFETY.md`)}>Read the safety contract</button>
    </section>
    <section in:fly={rise}>
      <h2>Thanks</h2>
      <p class="muted">Talks to your monitor with ddc-hi. Built with Tauri and Svelte.</p>
    </section>
    <section in:fly={rise}>
      <h2>Open source</h2>
      <p class="muted">Free software under the GNU GPL v3. Not affiliated with LG Electronics. LG, LG Switch and UltraGear are trademarks of LG Electronics.</p>
    </section>
  </div>
</div>

<style>
  .page {
    max-width: 960px;
  }
  .hero {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
    padding: 52px 0 48px;
  }
  .hero h1 {
    max-width: 680px;
    font-size: 40px;
    margin-top: var(--space-3);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: var(--space-2);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 32px 40px;
  }
  .grid section {
    padding-top: 12px;
    border-top: 2px solid var(--ink);
  }
  .grid h2 {
    margin-bottom: 6px;
  }
  .grid p {
    margin-bottom: 6px;
  }
</style>
