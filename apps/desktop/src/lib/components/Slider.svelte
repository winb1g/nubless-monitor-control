<script lang="ts">
  import { setControl } from "../store.svelte";
  import { ms } from "../motion";
  import type { ControlView } from "../types";
  import Lock from "./Lock.svelte";

  let { control, compact = false }: { control: ControlView; compact?: boolean } = $props();

  let local = $state<number | null>(null);
  let dragging = $state(false);
  let bump = $state(false);
  let lastSent = 0;
  let gestureStart: number | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const value = $derived(local ?? control.value ?? control.min);
  const pct = $derived(((value - control.min) / Math.max(1, control.max - control.min)) * 100);

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

  function oninput(e: Event) {
    gestureStart ??= control.value;
    const v = Number((e.target as HTMLInputElement).value);
    if ((v === control.min || v === control.max) && v !== local) {
      bump = true;
      setTimeout(() => (bump = false), ms("fast"));
    }
    local = v;
    send(v, false);
  }

  function onchange(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    dragging = false;
    send(v, true);
    setTimeout(() => (local = null), 400);
  }
</script>

<div class="slider" class:compact class:off={!control.enabled}>
  <div class="head">
    <span class="label">{control.label}</span>
    {#if control.enabled}
      <span class="val" class:pop={dragging}>{control.value === null && local === null ? "–" : value}</span>
    {:else}
      <Lock {control} />
    {/if}
  </div>
  <div class="track" class:bump style="--p:{pct}%">
    <input
      type="range"
      min={control.min}
      max={control.max}
      step="1"
      value={value}
      disabled={!control.enabled}
      aria-label={control.label}
      onpointerdown={() => (dragging = true)}
      {oninput}
      {onchange}
    />
  </div>
</div>

<style>
  .slider {
    padding: 10px 0;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }
  .label {
    font-weight: 550;
  }
  .compact .label {
    font-size: var(--fs-md);
  }
  .val {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    min-width: 34px;
    text-align: center;
    padding: 2px 8px;
    border-radius: var(--r-xs);
    background: var(--accent-soft);
    color: var(--accent-ink);
    transition: transform var(--fast) var(--spring);
  }
  .val.pop {
    transform: scale(1.12);
  }
  .track {
    position: relative;
    height: 28px;
    display: flex;
    align-items: center;
    transition: transform var(--fast) var(--spring);
  }
  .track.bump {
    animation: bump var(--fast) var(--ease);
  }
  @keyframes bump {
    40% {
      transform: scaleX(1.012);
    }
  }
  input {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 8px;
    margin: 0;
    border-radius: var(--r-full);
    background: var(--grad-x) 0 0 / var(--p) 100% no-repeat, var(--panel-2);
    box-shadow: inset 0 0 0 1px var(--line);
    cursor: pointer;
  }
  input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--knob);
    border: none;
    box-shadow: var(--shadow-knob), 0 0 0 4px var(--accent-soft);
    transition: transform var(--fast) var(--spring), box-shadow var(--fast);
  }
  input:hover::-webkit-slider-thumb {
    transform: scale(1.1);
  }
  input:active::-webkit-slider-thumb {
    transform: scale(1.18);
    box-shadow: var(--shadow-knob), 0 0 0 7px var(--accent-soft);
  }
  .off input {
    cursor: default;
    background: var(--panel-2);
    opacity: 0.6;
  }
  .off input::-webkit-slider-thumb {
    box-shadow: var(--shadow-knob);
  }
  .off .label {
    color: var(--text-2);
  }
</style>
