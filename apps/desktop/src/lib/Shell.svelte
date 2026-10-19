<script lang="ts">
  import { pageIn, pageOut, fadeQuick } from "./motion";
  import { fly, fade } from "svelte/transition";
  import { ui, toast, type Page } from "./store.svelte";
  import { checkForUpdates, upd } from "./updates.svelte";
  import Logo from "./components/Logo.svelte";
  import Displays from "./pages/Displays.svelte";
  import SettingsPage from "./pages/Settings.svelte";
  import Updates from "./pages/Updates.svelte";
  import About from "./pages/About.svelte";

  const nav: { id: Page; label: string }[] = [
    { id: "displays", label: "Displays" },
    { id: "settings", label: "Settings" },
    { id: "updates", label: "Updates" },
    { id: "about", label: "About" },
  ];

  // Quietly check for updates once at start; the nav shows a signal square if one is ready.
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
  /** The LED: blinks while the monitor is simulated, solid when real, dashed when absent. */
  const status = $derived(
    !s ? null : s.phase === "connected" ? (s.demo ? { text: "Simulated", led: "blink" } : { text: "Connected", led: "" }) : s.phase === "scanning" ? { text: "Looking…", led: "off" } : { text: "No monitor", led: "off" },
  );
</script>

<div class="shell">
  <header>
    <Logo size={24} />
    <nav aria-label="Main">
      {#each nav as n (n.id)}
        <button class:active={ui.page === n.id} onclick={() => (ui.page = n.id)} aria-current={ui.page === n.id ? "page" : undefined}>
          {n.label}
          {#if n.id === "updates" && ui.updateBadge}<span class="dot" aria-label="Update available" transition:fade={fadeQuick}></span>{/if}
        </button>
      {/each}
    </nav>
    {#if status}
      <span class="status eyebrow"><span class="led {status.led}" aria-hidden="true"></span>{status.text}</span>
    {/if}
  </header>

  <main>
    {#key ui.page}
      <div class="view" in:fly={pageIn} out:fade={pageOut}>
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
    grid-template-rows: auto 1fr;
    height: 100%;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-1) var(--space-6);
    padding: var(--space-2) var(--space-10);
    border-bottom: 1px solid var(--rule);
  }
  nav {
    display: flex;
    flex-wrap: wrap;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--hit);
    padding: 0 14px;
    border: none;
    background: none;
    color: var(--ink);
    font-weight: 600;
    transition: background var(--fast) var(--ease);
  }
  nav button:hover:not(.active) {
    background: var(--wash);
  }
  nav button.active {
    background: var(--ink);
    color: var(--ground);
  }
  .dot {
    width: 8px;
    height: 8px;
    border: 2px solid var(--ink);
    background: var(--signal);
  }
  .active .dot {
    border-color: var(--ground);
  }
  .status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  main {
    overflow-y: auto;
    position: relative;
  }
  .view {
    min-height: 100%;
  }
</style>
