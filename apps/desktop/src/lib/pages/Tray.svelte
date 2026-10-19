<script lang="ts">
  import { popover } from "../motion";
  import { fly } from "svelte/transition";
  import {
    Maximize2, Power, Search, User, Gamepad2, Crosshair, Ruler, Eye, Clapperboard, Image, Sparkles,
  } from "@lucide/svelte";
  import { ui, rescan, pickPictureMode } from "../store.svelte";
  import { api } from "../api";
  import Slider from "../components/Slider.svelte";
  import Lock from "../components/Lock.svelte";
  import IconBadge from "../components/IconBadge.svelte";
  import Hint from "../components/Hint.svelte";

  const s = $derived(ui.state);
  const ctl = (id: string) => s?.controls.find((c) => c.id === id);
  const pm = $derived(ctl("picture-mode"));
  const loading = $derived(!!ui.switching || !!s?.refreshing);
  const open = async (page?: string) => (await api()).openMain(page);
  const quit = async () => (await api()).quit();

  /** Short names so eight modes fit a 4×2 grid without scrolling. */
  const short: Record<string, string> = {
    "Personalized Picture": "Personal",
    "Gamer 1": "Gamer 1",
    "Gamer 2": "Gamer 2",
    Calibration1: "Calib. 1",
    Calibration2: "Calib. 2",
    "Color Weakness": "Color aid",
    "DCI-P3": "DCI-P3",
    FPS: "FPS",
  };
  const icons: Record<string, typeof User> = {
    "Personalized Picture": User, "Gamer 1": Gamepad2, "Gamer 2": Gamepad2, FPS: Crosshair,
    Calibration1: Ruler, Calibration2: Ruler, "Color Weakness": Eye, "DCI-P3": Clapperboard,
  };
</script>

<div class="pop" in:fly={popover}>
  <header>
    <img src="/logo.svg" alt="" width="26" height="26" />
    <div class="who">
      <b>{s?.monitors.find((m) => m.selected)?.name ?? "nubless"}</b>
      <span class:ok={s?.phase === "connected"}>
        <i class="dot"></i>{s?.phase === "connected" ? (s.demo ? "Demo monitor" : "Connected") : s ? "No monitor found" : "Connecting…"}
      </span>
    </div>
    <button class="icon" onclick={() => open()} aria-label="Open nubless" title="Open nubless"><Maximize2 size={15} /></button>
  </header>

  {#if !s}
    <div class="skeleton" style="height:48px;margin:14px 0 8px"></div>
    <div class="skeleton" style="height:48px;margin:8px 0"></div>
    <div class="skeleton" style="height:120px;margin:12px 0"></div>
  {:else if s.phase !== "connected"}
    <div class="empty">
      <p class="muted">Your 27GM950B didn't answer.</p>
      <button class="btn primary" onclick={rescan} disabled={ui.busy}><Search size={15} />{ui.busy ? "Looking…" : "Scan again"}</button>
    </div>
  {:else}
    <div class="sliders" class:loading>
      {#if ctl("brightness")}<Slider control={ctl("brightness")!} compact />{/if}
      {#if ctl("contrast")}<Slider control={ctl("contrast")!} compact />{/if}
    </div>

    {#if pm}
      <div class="pmhead">
        <span>Picture mode</span>
        {#if loading}
          <Hint loading>Loading…</Hint>
        {:else if !s.demo && !pm.enabled}
          <Lock control={pm} />
        {/if}
      </div>
      <div class="grid">
        {#each pm.choices as c (c.raw)}
          {@const Icon = icons[c.label] ?? Image}
          {@const current = ui.switching ? ui.switching === c.label : (s.features.picture_mode ?? null) === c.label || pm.value === c.raw}
          <button
            class="mode"
            class:active={current}
            class:pending={ui.switching === c.label}
            disabled={!s.demo && !pm.enabled}
            title={c.label}
            onclick={() => !current && pickPictureMode(c.label)}
          >
            <IconBadge icon={Icon} iconSize={17} tone={current ? "brand" : "soft"} glow={current} busy={ui.switching === c.label}>
              {#if s.features.ai_capable_modes.includes(c.label)}<span class="ai"><Sparkles size={9} /></span>{/if}
            </IconBadge>
            <span class="label">{short[c.label] ?? c.label}</span>
          </button>
        {/each}
      </div>
    {/if}
  {/if}

  <footer>
    <button class="row" onclick={() => open()}><Maximize2 size={14} />Open nubless</button>
    <button class="row" onclick={quit}><Power size={14} />Quit</button>
  </footer>
</div>

<style>
  .pop {
    margin: 10px;
    padding: 14px var(--space-4) var(--space-2);
    border-radius: 20px;
    background: var(--panel);
    border: 1px solid var(--line);
    box-shadow: var(--shadow-popover);
    height: calc(100vh - 20px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--line);
  }
  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .who b {
    font-size: var(--fs-base);
    font-weight: 650;
  }
  .who span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--text-3);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .who span.ok {
    color: var(--mint);
    font-weight: 600;
  }
  .icon {
    border: none;
    background: var(--panel-2);
    width: 30px;
    height: 30px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    color: var(--text-2);
    transition: transform var(--fast) var(--spring), color var(--fast);
  }
  .icon:hover {
    transform: scale(1.08);
    color: var(--accent);
  }

  .sliders {
    padding-top: 6px;
    transition: opacity var(--base) var(--ease);
  }
  .sliders :global(.slider) {
    padding: 6px 0;
  }
  .sliders :global(.head) {
    margin-bottom: 6px;
  }
  .sliders.loading {
    opacity: 0.5;
    pointer-events: none;
  }

  .pmhead {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--fs-md);
    font-weight: 600;
    margin: var(--space-3) 0 var(--space-2);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }
  .mode {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: var(--space-2) 2px 7px;
    border: none;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--text-2);
    transition: background var(--fast), transform var(--fast) var(--spring);
  }
  .mode:hover:not(:disabled) {
    background: var(--panel-2);
  }
  .mode:active:not(:disabled) {
    transform: scale(0.95);
  }
  .ai {
    position: absolute;
    top: -3px;
    right: -3px;
    width: 15px;
    height: 15px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--panel);
    color: var(--accent);
    box-shadow: 0 0 0 1px var(--line);
  }
  .label {
    font-size: var(--fs-2xs);
    font-weight: 550;
    line-height: 1.1;
    white-space: nowrap;
  }
  .mode.active .label {
    color: var(--text);
    font-weight: 650;
  }
  .mode:disabled {
    opacity: 0.55;
    cursor: default;
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
    border-top: 1px solid var(--line);
    padding-top: 6px;
    display: flex;
    justify-content: space-between;
  }
  .row {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    border: none;
    background: none;
    padding: 7px 10px;
    border-radius: var(--r-sm);
    font-size: var(--fs-md);
    color: var(--text-2);
    font-weight: 550;
  }
  .row:hover {
    background: var(--panel-2);
    color: var(--text);
  }
</style>
