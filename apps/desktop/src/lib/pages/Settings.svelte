<script lang="ts">
  import { FlaskConical, Search } from "@lucide/svelte";
  import { ui, resetOriginal, setLabs, useDemo, rescan, toast } from "../store.svelte";
  import { api } from "../api";
  import Item from "../components/Item.svelte";
  import ShortcutsItem from "../components/ShortcutsItem.svelte";
  import Toggle from "../components/Toggle.svelte";
  import { theme, setTheme } from "../theme.svelte";

  const s = $derived(ui.state);

  // Read from the OS each time, so it's right even if changed in system settings.
  let autostart = $state(false);
  let autostartBusy = $state(false);
  $effect(() => {
    api().then((b) => b.getAutostart()).then((on) => (autostart = on)).catch(() => {});
  });
  async function setAutostart(on: boolean) {
    autostartBusy = true;
    try {
      autostart = await (await api()).setAutostart(on);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      autostartBusy = false;
    }
  }
</script>

<div class="page">
  <div class="page-head">
    <h1>Settings</h1>
    <p class="subtitle">Make nubless yours. Every change here is safe to try.</p>
  </div>

  <div class="stack">
    <Item lead title="Your original settings" {body}>
      <button class="btn" onclick={resetOriginal} disabled={!s?.baseline.saved || ui.busy}>{ui.busy ? "Resetting…" : "Reset to my original settings"}</button>
    </Item>

    <Item title="Start at login" body="Open nubless quietly in the tray when you sign in, so your shortcuts are always ready.">
      <Toggle checked={autostart} pending={autostartBusy} label="Start at login" onchange={setAutostart} />
    </Item>

    <ShortcutsItem />

    <Item title="Labs" body="Try controls that match LG's own names and values but haven't been tested on a real monitor yet, like Response Time and Adaptive-Sync. Safe ranges and interlocks still apply.">
      {#snippet mark()}<FlaskConical size={20} strokeWidth={2} />{/snippet}
      <Toggle checked={!!s?.labs} label="Labs" onchange={(v) => setLabs(v)} />
    </Item>

    <Item title="Appearance" body="Follow your system or pick a side.">
      <div class="seg" role="group" aria-label="Appearance">
        {#each ["system", "light", "dark"] as t}
          <button class:active={theme.value === t} aria-pressed={theme.value === t} onclick={() => setTheme(t as "system" | "light" | "dark")}>{t[0].toUpperCase() + t.slice(1)}</button>
        {/each}
      </div>
    </Item>

    <Item title="Demo monitor" body={s?.demo ? "You're using a simulated 27GM950B. Nothing is sent to real hardware." : "Explore every control on a simulated 27GM950B. Nothing is sent to real hardware."}>
      {#if s?.demo}
        <button class="btn" onclick={rescan}><Search size={16} />Find my monitor</button>
      {:else}
        <button class="btn" onclick={useDemo}>Use demo</button>
      {/if}
    </Item>
  </div>
</div>

{#snippet body()}
  {#if s?.baseline.saved}
    Saved the first time nubless connected, before it changed anything ({s.baseline.values} settings{s.baseline.saved_in ? ` in ${s.baseline.saved_in}` : ""}). Reset puts them back{s.baseline.saved_in ? `; each picture mode keeps its own settings, so reset works while your monitor is in ${s.baseline.saved_in}` : ""}.
  {:else if s?.baseline.error}
    Couldn't be saved: {s.baseline.error}. nubless won't change anything until this works.
  {:else}
    Saved automatically when nubless first connects to your monitor.
  {/if}
{/snippet}

<style>
  .page {
    max-width: 920px;
  }
  .seg {
    display: inline-flex;
    border: 2px solid var(--ink);
    border-radius: var(--r-sm);
  }
  .seg button {
    min-height: 40px;
    padding: 0 16px;
    border: none;
    background: transparent;
    color: var(--ink);
    font-weight: 600;
    transition: background var(--fast) var(--ease), color var(--fast) var(--ease);
  }
  .seg button:hover:not(.active) {
    background: var(--wash);
  }
  .seg button.active {
    background: var(--ink);
    color: var(--ground);
  }
</style>
