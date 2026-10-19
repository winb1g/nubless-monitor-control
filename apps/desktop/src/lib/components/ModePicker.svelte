<script lang="ts">
  // Picks the monitor's picture mode. One component, two layouts:
  //   chips   – every mode at once, grouped by use (the main window)
  //   stepper – ‹ name ›, one mode at a time with a position counter (the tray)
  // Both go through pickPictureMode, so the AI-feature interlock still asks.
  import { ChevronLeft, ChevronRight } from "@lucide/svelte";
  import { ui, pickPictureMode } from "../store.svelte";
  import { groupModes, groupName, modeName } from "../modes";
  import type { ControlView } from "../types";

  let { pm, layout = "chips" }: { pm: ControlView; layout?: "chips" | "stepper" } = $props();

  const s = $derived(ui.state);
  const locked = $derived(!s?.demo && !pm.enabled);
  const ai = $derived(s?.features.ai_capable_modes ?? []);
  const isCurrent = (c: { label: string; raw: number }) =>
    ui.switching ? ui.switching === c.label : (s?.features.picture_mode ?? null) === c.label || pm.value === c.raw;

  // ---- stepper ------------------------------------------------------------
  // Stepping only moves the display; the mode is written once you stop for
  // 400ms, so passing over modes on the way never reaches the monitor.
  const COMMIT_AFTER = 400;
  let target = $state<number | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  const count = $derived(pm.choices.length);
  const currentIndex = $derived(Math.max(0, pm.choices.findIndex(isCurrent)));
  const shown = $derived(target ?? currentIndex);
  const shownMode = $derived(pm.choices[shown]);

  function go(dir: 1 | -1) {
    if (locked || ui.switching || count === 0) return;
    target = (((shown + dir) % count) + count) % count;
    clearTimeout(timer);
    timer = setTimeout(commit, COMMIT_AFTER);
  }
  async function commit() {
    const c = target === null ? null : pm.choices[target];
    if (c && !isCurrent(c)) await pickPictureMode(c.label);
    target = null;
  }
</script>

{#if layout === "chips"}
  <div class="groups" role="group" aria-label="Picture mode">
    {#each groupModes(pm.choices) as g (g.name)}
      <div class="group">
        <div class="eyebrow">{g.name}</div>
        <div class="chips">
          {#each g.items as c (c.raw)}
            {@const current = isCurrent(c)}
            <button class="chip" class:current disabled={locked} aria-pressed={current} onclick={() => !current && pickPictureMode(c.label)}>
              <span>{modeName(c.label)}</span>
              {#if ai.includes(c.label)}<span class="ai" title="Works with AI Scene Optimization">AI</span>{/if}
              {#if ui.switching === c.label}<span class="hatch under" aria-hidden="true"></span>{/if}
            </button>
          {/each}
        </div>
      </div>
    {/each}
  </div>
{:else if shownMode}
  <div class="stepper" role="group" aria-label="Picture mode">
    <div class="eyebrow">Picture mode · {groupName(shownMode.label)}</div>
    <div class="line">
      <button class="chev" aria-label="Previous picture mode" disabled={locked || !!ui.switching} onclick={() => go(-1)}><ChevronLeft size={18} strokeWidth={2.5} /></button>
      <span class="name" aria-live="polite">{modeName(shownMode.label)}</span>
      <button class="chev" aria-label="Next picture mode" disabled={locked || !!ui.switching} onclick={() => go(1)}><ChevronRight size={18} strokeWidth={2.5} /></button>
    </div>
    {#if ui.switching}
      <div class="hatch ticks" aria-hidden="true"></div>
    {:else}
      <div class="ticks" aria-hidden="true">
        {#each pm.choices as _, i}<span class:on={i === shown}></span>{/each}
      </div>
    {/if}
    <div class="eyebrow meta">
      <span>{String(shown + 1).padStart(2, "0")} / {String(count).padStart(2, "0")}</span>
      {#if ai.includes(shownMode.label)}<span>Works with AI</span>{/if}
    </div>
  </div>
{/if}

<style>
  .groups {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-5) var(--space-8);
  }
  .eyebrow {
    margin-bottom: var(--space-2);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .chip {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: var(--hit);
    padding: 0 16px;
    border: 2px solid var(--ink);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--ink);
    font-weight: 600;
    transition: background var(--fast) var(--ease), color var(--fast) var(--ease);
  }
  .chip:hover:not(:disabled):not(.current) {
    background: var(--wash);
  }
  .chip.current {
    background: var(--ink);
    color: var(--ground);
  }
  .chip:disabled {
    opacity: 0.45;
    border-style: dashed;
    cursor: default;
  }
  .ai {
    font-family: var(--font-mono);
    font-size: var(--fs-2xs);
    font-weight: 500;
    letter-spacing: 0.06em;
    color: var(--dim);
  }
  .current .ai {
    color: var(--sel-mark);
  }
  /* A mode being switched to: the busy bar along the chip's bottom edge. */
  .under {
    position: absolute;
    left: 0;
    right: 0;
    bottom: -9px;
    height: 4px;
  }

  .stepper .eyebrow {
    margin-bottom: 0;
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 28px;
    margin-top: var(--space-2);
  }
  /* 18px glyph in a 28px box, with 8px of invisible padding for the 44px hit, so the row is only as tall as its text. */
  .chev {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    margin: -8px;
    padding: 0;
    border: none;
    background: none;
    color: var(--ink);
    transition: background var(--fast) var(--ease);
  }
  .chev:hover:not(:disabled) {
    background: var(--wash);
  }
  .chev:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .name {
    font-weight: 800;
    font-size: var(--fs-xl);
    letter-spacing: -0.02em;
    white-space: nowrap;
  }
  .ticks {
    display: flex;
    gap: 3px;
    margin-top: var(--space-3);
  }
  .ticks span {
    flex: 1 1 0;
    height: 5px;
    background: var(--rule);
  }
  .ticks span.on {
    background: var(--ink);
  }
  .hatch.ticks {
    height: 5px;
  }
  .meta {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
  }
</style>
