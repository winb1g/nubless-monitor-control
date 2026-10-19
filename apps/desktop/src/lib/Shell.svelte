<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { MonitorCog, Settings, Download, Info } from "@lucide/svelte";
  import { ui, toast, type Page } from "./store.svelte";
  import { checkForUpdates, upd } from "./updates.svelte";
  import Displays from "./pages/Displays.svelte";
  import SettingsPage from "./pages/Settings.svelte";
  import Updates from "./pages/Updates.svelte";
  import About from "./pages/About.svelte";

  const nav: { id: Page; label: string; icon: typeof Info }[] = [
    { id: "displays", label: "Displays", icon: MonitorCog },
    { id: "settings", label: "Settings", icon: Settings },
    { id: "updates", label: "Updates", icon: Download },
    { id: "about", label: "About", icon: Info },
  ];
  const index = $derived(nav.findIndex((n) => n.id === ui.page));

  // Quietly check for updates once at start; the sidebar shows a dot if one is ready.
  $effect(() => {
    if (upd.phase === "idle") setTimeout(checkForUpdates, 2500);
  });

  // First connection ever: a small moment to say the original settings are safe. Once only.
  let savedToastShown = false;
  $effect(() => {
    if (ui.state?.baseline.just_saved && !savedToastShown) {
      savedToastShown = true;
      toast("Your original settings are saved. You can always go back.", "success");
    }
  });

  const s = $derived(ui.state);
</script>

<div class="shell">
  <aside>
    <div class="brand">
      <img src="/logo.svg" alt="" width="30" height="30" />
      <span>nubless</span>
    </div>
    <nav style="--i:{index}">
      <span class="pill"></span>
      {#each nav as n (n.id)}
        <button class:active={ui.page === n.id} onclick={() => (ui.page = n.id)} aria-current={ui.page === n.id ? "page" : undefined}>
          <n.icon size={18} />
          <span>{n.label}</span>
          {#if n.id === "updates" && ui.updateBadge}<span class="dot" transition:fade></span>{/if}
        </button>
      {/each}
    </nav>
    <div class="foot">
      {#if s}
        <div class="mon" class:ok={s.phase === "connected"}>
          <span class="led"></span>
          <span>{s.phase === "connected" ? (s.demo ? "Demo monitor" : s.model) : "No monitor"}</span>
        </div>
      {/if}
    </div>
  </aside>

  <main>
    {#key ui.page}
      <div class="view" in:fly={{ y: 10, duration: 280, delay: 60 }} out:fade={{ duration: 60 }}>
        {#if ui.page === "displays"}<Displays />
        {:else if ui.page === "settings"}<SettingsPage />
        {:else if ui.page === "updates"}<Updates />
        {:else}<About />{/if}
      </div>
    {/key}
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 236px 1fr;
    height: 100%;
  }
  aside {
    display: flex;
    flex-direction: column;
    padding: 22px 14px 16px;
    background: color-mix(in srgb, var(--panel) 55%, var(--bg));
    border-right: 1px solid var(--line);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px 22px;
    font-weight: 700;
    font-size: 17px;
    letter-spacing: -0.02em;
  }
  nav {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .pill {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 42px;
    border-radius: 13px;
    background: var(--panel);
    box-shadow: var(--shadow-sm), 0 0 0 1px var(--line);
    transform: translateY(calc(var(--i) * 46px));
    transition: transform 380ms var(--spring);
  }
  nav button {
    position: relative;
    z-index: 1;
    height: 42px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 14px;
    border: none;
    background: none;
    border-radius: 13px;
    color: var(--text-2);
    font-weight: 550;
    text-align: left;
    transition: color var(--fast);
  }
  nav button:hover {
    color: var(--text);
  }
  nav button.active {
    color: var(--text);
  }
  nav button.active :global(svg) {
    color: var(--accent);
  }
  .dot {
    margin-left: auto;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent);
    animation: pulse 2s infinite;
  }
  .foot {
    margin-top: auto;
  }
  .mon {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 10px 12px;
    border-radius: 13px;
    background: var(--panel);
    box-shadow: 0 0 0 1px var(--line);
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-3);
  }
  .led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-3);
  }
  .mon.ok {
    color: var(--text-2);
  }
  .mon.ok .led {
    background: var(--mint);
    color: var(--mint);
    animation: pulse 2.4s infinite;
  }
  main {
    overflow-y: auto;
    position: relative;
  }
  .view {
    min-height: 100%;
  }
</style>
