<script lang="ts">
  // An outlined bar you drag, with its value as a mono readout. With `fine`,
  // − and + sit at the bar's ends: ±1 per press, hold to repeat. `compact` is
  // the tray's smaller version.
  import { Minus, Plus } from "@lucide/svelte";
  import { setControl } from "../store.svelte";
  import { ms } from "../motion";
  import type { ControlView } from "../types";
  import BlockedZone from "./BlockedZone.svelte";
  import Lock from "./Lock.svelte";

  let { control, compact = false, fine = false }: { control: ControlView; compact?: boolean; fine?: boolean } = $props();

  let local = $state<number | null>(null);
  let dragging = $state(false);
  let lastSent = 0;
  let gestureStart: number | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let settle: ReturnType<typeof setTimeout> | undefined;
  let repeat: ReturnType<typeof setInterval> | undefined;
  let holdDelay: ReturnType<typeof setTimeout> | undefined;

  const id = $derived(`slider-${control.id}`);
  const value = $derived(local ?? control.value ?? control.min);
  // A safe range that starts above 0 (Contrast: 20) is drawn on the monitor's own
  // 0-to-max scale, so the handle sits where the monitor's menu puts it. The part
  // below the floor is hatched and can't be dragged into.
  const base = $derived(control.min > 0 ? 0 : control.min);
  const span = $derived(Math.max(1, control.max - base));
  const pct = $derived(((value - base) / span) * 100);
  const floorPct = $derived(((control.min - base) / span) * 100);

  // Drags are throttled to the monitor's pace; the last value always goes out.
  function send(v: number, final: boolean) {
    clearTimeout(timer);
    const now = performance.now();
    if (final || now - lastSent > 220) {
      lastSent = now;
      setControl(control.id, v, final ? { before: gestureStart } : { quiet: true });
      if (final) gestureStart = null;
    } else {
      timer = setTimeout(() => send(v, final), 220 - (now - lastSent));
    }
  }

  // The input spans the whole scale; anything under the floor snaps back to it.
  function pinned(e: Event) {
    const input = e.target as HTMLInputElement;
    const v = Math.max(control.min, Number(input.value));
    input.value = String(v);
    return v;
  }

  function oninput(e: Event) {
    gestureStart ??= control.value;
    const v = pinned(e);
    local = v;
    send(v, false);
  }

  function onchange(e: Event) {
    const v = pinned(e);
    dragging = false;
    send(v, true);
    clearTimeout(settle);
    settle = setTimeout(() => (local = null), 400);
  }

  // Fine adjust: each press moves one step and goes out at the throttled pace.
  // The press (or a held run of them) counts as one change once it stops, so
  // Undo puts back where it started.
  function step(dir: 1 | -1) {
    gestureStart ??= control.value;
    const v = Math.min(control.max, Math.max(control.min, value + dir));
    if (v === value) return;
    local = v;
    send(v, false);
    clearTimeout(settle);
    settle = setTimeout(() => {
      send(local ?? v, true);
      settle = setTimeout(() => (local = null), 400);
    }, 500);
  }
  function hold(dir: 1 | -1) {
    step(dir);
    holdDelay = setTimeout(() => (repeat = setInterval(() => step(dir), 120)), 420);
  }
  function release() {
    clearTimeout(holdDelay);
    clearInterval(repeat);
    repeat = undefined;
  }
  // A keyboard press arrives as a click with no pointer (detail 0); pointer
  // presses are handled on pointerdown so holding repeats.
  const keyStep = (dir: 1 | -1) => (e: MouseEvent) => e.detail === 0 && step(dir);
</script>

<div class="slider" class:compact class:off={!control.enabled}>
  <div class="head">
    <label class="label" for={id}>{control.label}</label>
    {#if control.enabled}
      <span class="val" class:drag={dragging}>{control.value === null && local === null ? "–" : value}</span>
    {:else}
      <Lock {control} />
    {/if}
  </div>
  <div class="line">
    {#if fine && control.enabled}
      <button class="fine start" aria-label="Decrease {control.label}" onpointerdown={() => hold(-1)} onpointerup={release} onpointerleave={release} onclick={keyStep(-1)}>
        <Minus size={14} strokeWidth={3} />
      </button>
    {/if}
    <div class="track">
      <div class="bar" aria-hidden="true">{#if floorPct > 0}<BlockedZone pct={floorPct} {compact} />{/if}<i style:margin-left="{floorPct}%" style:width="{pct - floorPct}%"></i></div>
      <input
        {id}
        type="range"
        aria-describedby={floorPct > 0 ? `${id}-floor` : undefined}
        min={base}
        max={control.max}
        step="1"
        value={value}
        disabled={!control.enabled}
        onpointerdown={() => (dragging = true)}
        {oninput}
        {onchange}
      />
      {#if floorPct > 0}<span id="{id}-floor" class="note">Below {control.min} is blocked so the picture can't become unreadable.</span>{/if}
    </div>
    {#if fine && control.enabled}
      <button class="fine end" aria-label="Increase {control.label}" onpointerdown={() => hold(1)} onpointerup={release} onpointerleave={release} onclick={keyStep(1)}>
        <Plus size={14} strokeWidth={3} />
      </button>
    {/if}
  </div>
</div>

<style>
  .slider {
    padding: var(--space-2) 0;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--space-3);
  }
  .label {
    font-weight: 800;
    font-size: var(--fs-lg);
  }
  .compact .label {
    font-size: var(--fs-base);
  }
  .val {
    font-family: var(--font-mono);
    font-size: var(--fs-3xl);
    font-variant-numeric: tabular-nums;
    padding: 0 6px;
    margin-right: -6px;
  }
  .compact .val {
    font-size: var(--fs-2xl);
  }
  /* While you drag, the readout inverts. */
  .val.drag {
    background: var(--ink);
    color: var(--ground);
  }
  .line {
    display: flex;
    align-items: center;
    margin-top: var(--space-2);
  }
  .track {
    position: relative;
    flex: 1 1 auto;
    height: 16px;
  }
  .bar {
    height: 16px;
    border: 2px solid var(--ink);
  }
  .bar {
    position: relative;
  }
  /* Read by screen readers only. */
  .note {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--ink);
    /* Moves smoothly when something else sets the value (a hotkey); a drag follows the pointer at once. */
    transition: width var(--base) var(--ease);
  }
  .compact .bar,
  .compact .track {
    height: 14px;
  }
  input {
    position: absolute;
    left: 0;
    top: -14px;
    width: 100%;
    height: var(--hit);
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }
  .track:has(input:focus-visible) .bar {
    outline: 2px solid var(--ink);
    outline-offset: 3px;
  }
  /* −/+ are 44px to hit and look like a glyph; the negative margin keeps the row as tall as the bar. */
  .fine {
    flex: none;
    display: grid;
    place-items: center;
    width: var(--hit);
    height: var(--hit);
    padding: 0;
    border: none;
    background: none;
    color: var(--ink);
    transition: background var(--fast) var(--ease);
  }
  .fine:hover {
    background: var(--wash);
  }
  .fine.start {
    margin: -14px 0 -14px -10px;
  }
  .fine.end {
    margin: -14px -10px -14px 0;
  }
  .off {
    opacity: 0.6;
  }
  .off .bar {
    border-style: dashed;
  }
  .off .bar i {
    background: var(--rule);
  }
  .off input {
    cursor: default;
  }
</style>
