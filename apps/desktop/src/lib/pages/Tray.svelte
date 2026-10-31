<script lang="ts">
  import { popover } from "../motion";
  import { fly } from "svelte/transition";
  import { Maximize2 } from "@lucide/svelte";
  import { ui, rescan, selectMonitor } from "../store.svelte";
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
  /** With two or more monitors, the popover can switch between them. */
  const choices = $derived(s?.monitors.filter((m) => m.supported) ?? []);
  /** The OS's own number for it ("LG ULTRAGEAR+ (2)" is 2), else its place in the list. */
  const number = (name: string, i: number) => name.match(/\((\d+)\)$/)?.[1] ?? String(i + 1);
  const status = $derived(!s ? "Looking for your monitor…" : s.phase === "connected" ? (s.demo ? "Simulated" : "Connected") : "No monitor found");
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
  {#if s && choices.length > 1}
    <div class="pick" role="group" aria-label="Monitor">
      {#each choices as m, i (m.id)}
        <button class:selected={m.selected} aria-pressed={m.selected} aria-label={m.name} title={m.port ? `${m.name} · ${m.port}` : m.name} onclick={() => selectMonitor(m.id)}>{number(m.name, i)}</button>
      {/each}
      <span class="eyebrow">{s.linked ? "All together" : "One at a time"}</span>
    </div>
  {/if}

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

  .pick {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding-top: var(--space-2);
  }
  .pick button {
    min-width: var(--hit);
    min-height: 28px;
    border: 2px solid var(--ink);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--ink);
    font-weight: 800;
    transition: background var(--fast) var(--ease);
  }
  .pick button:hover:not(.selected) {
    background: var(--wash);
  }
  .pick button.selected {
    background: var(--ink);
    color: var(--ground);
  }
  .pick .eyebrow {
    margin-left: auto;
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
