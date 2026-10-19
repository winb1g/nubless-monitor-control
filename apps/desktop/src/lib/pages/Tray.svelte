<script lang="ts">
  import { popover } from "../motion";
  import { fly } from "svelte/transition";
  import { Maximize2 } from "@lucide/svelte";
  import { ui, rescan } from "../store.svelte";
  import { api } from "../api";
  import Slider from "../components/Slider.svelte";
  import Lock from "../components/Lock.svelte";
  import Logo from "../components/Logo.svelte";
  import ModePicker from "../components/ModePicker.svelte";

  const s = $derived(ui.state);
  const ctl = (id: string) => s?.controls.find((c) => c.id === id);
  const pm = $derived(ctl("picture-mode"));
  const loading = $derived(!!ui.switching || !!s?.refreshing);
  const open = async (page?: string) => (await api()).openMain(page);
  const quit = async () => (await api()).quit();
  const status = $derived(!s ? "Connecting…" : s.phase === "connected" ? (s.demo ? "Simulated" : "Connected") : "No monitor found");
</script>

<div class="pop" in:fly={popover}>
  <header>
    <Logo size={20} />
    <button class="icon" onclick={() => open()} aria-label="Open nubless" title="Open nubless"><Maximize2 size={18} strokeWidth={2} /></button>
  </header>
  <div class="who">
    <b>{s?.monitors.find((m) => m.selected)?.name ?? "nubless"}</b>
    <span class="eyebrow status"><i class="led" class:blink={s?.phase === "connected" && s.demo} class:off={s?.phase !== "connected"}></i>{status}</span>
  </div>

  {#if !s}
    <div class="skeleton" style="height:56px;margin:14px 0 8px"></div>
    <div class="skeleton" style="height:56px;margin:8px 0"></div>
    <div class="skeleton" style="height:96px;margin:12px 0"></div>
  {:else if s.phase !== "connected"}
    <div class="empty">
      <p class="muted">Your 27GM950B didn't answer.</p>
      <button class="btn primary" onclick={rescan} disabled={ui.busy}>{ui.busy ? "Looking…" : "Scan again"}</button>
    </div>
  {:else}
    <div class="sliders" class:loading>
      {#if ctl("brightness")}<Slider control={ctl("brightness")!} compact />{/if}
      {#if ctl("contrast")}<Slider control={ctl("contrast")!} compact />{/if}
    </div>

    {#if pm}
      <div class="modes">
        <ModePicker {pm} layout="stepper" />
        {#if !s.demo && !pm.enabled}<div class="lock"><Lock control={pm} /></div>{/if}
      </div>
    {/if}
  {/if}

  <footer>
    <button class="ft" onclick={() => open()}>Open nubless</button>
    <button class="ft" onclick={quit}>Quit</button>
  </footer>
</div>

<style>
  .pop {
    margin: 2px;
    padding: 8px var(--space-4) 0;
    border: 2px solid var(--ink);
    border-radius: var(--r-lg);
    background: var(--ground);
    height: calc(100vh - 4px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    font-size: var(--fs-md);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--rule);
  }
  .icon {
    display: grid;
    place-items: center;
    width: var(--hit);
    height: var(--hit);
    border: none;
    background: none;
    color: var(--ink);
    transition: background var(--fast) var(--ease);
  }
  .icon:hover {
    background: var(--wash);
  }
  .who {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
    padding-top: 6px;
  }
  .who b {
    font-weight: 800;
    font-size: var(--fs-base);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .status .led {
    width: 8px;
    height: 8px;
  }

  .sliders {
    padding-top: var(--space-2);
    transition: opacity var(--base) var(--ease);
  }
  .sliders :global(.slider) {
    padding: 5px 0;
  }
  .sliders.loading {
    opacity: 0.5;
    pointer-events: none;
  }

  .modes {
    margin-top: var(--space-3);
    padding-top: 10px;
    border-top: 2px solid var(--ink);
  }
  .lock {
    margin-top: var(--space-2);
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
  }
  footer {
    margin-top: auto;
    border-top: 1px solid var(--rule);
    display: flex;
    justify-content: space-between;
  }
  .ft {
    min-height: var(--hit);
    padding: 0 var(--space-2);
    border: none;
    background: none;
    font-weight: 600;
    transition: background var(--fast) var(--ease);
  }
  .ft:hover {
    background: var(--wash);
  }
</style>
