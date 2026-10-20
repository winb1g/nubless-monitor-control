<script lang="ts">
  // "Keep this change?" for a change that can hide the picture or resync the
  // panel. The change has already gone out; unless it is kept before the timer
  // line runs out, it is put back (store.svelte.ts). The line is --keep-window.
  import { fly } from "svelte/transition";
  import { toastIn, toastOut } from "../motion";
  import { ui, keepChange, revertChange, KEEP_MS } from "../store.svelte";

  let now = $state(Date.now());
  let start = $state(Date.now());
  // Restart the countdown whenever the same prompt is asked again.
  $effect(() => {
    const k = ui.keep;
    if (!k) return;
    void k.run;
    start = now = Date.now();
    const tick = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(tick);
  });
  const left = $derived(Math.max(0, Math.ceil((KEEP_MS - (now - start)) / 1000)));
</script>

{#if ui.keep}
  <div class="keep" role="group" aria-label="Keep this change?" in:fly={toastIn} out:fly={toastOut}>
      <span class="label">Keep?</span>
      <span class="text">
        {ui.keep.label} <strong>{ui.keep.shown}</strong>
        <span class="clock" aria-live="polite">Reverts in {left} s</span>
      </span>
      <button class="revert" onclick={revertChange}>Revert</button>
      <button class="keepit" onclick={keepChange}>Keep</button>
      {#key ui.keep.run}<span class="timer"></span>{/key}
    </div>
{/if}

<style>
  .keep {
    pointer-events: auto;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    max-width: min(520px, 100%);
    padding: 6px 6px 8px 14px;
    background: var(--toast-bg);
    color: var(--toast-ink);
    outline: 2px solid var(--ink);
    font-weight: 600;
  }
  .label {
    flex: none;
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    font-weight: 500;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--toast-warn);
  }
  .text {
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .clock {
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }
  button {
    flex: none;
    min-height: var(--hit);
    padding: 0 14px;
    border: none;
    border-radius: var(--r-sm);
    font-weight: 800;
  }
  .revert {
    background: none;
    color: var(--toast-ink);
  }
  .revert:hover {
    text-decoration: underline;
  }
  .keepit {
    background: var(--toast-signal);
    color: var(--on-signal);
  }
  .keepit:hover {
    background: var(--signal-hover);
  }
  /* How long "Keep" stays on offer before the change is put back. */
  .timer {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 2px;
    width: 100%;
    background: var(--toast-warn);
    transform-origin: left;
    animation: shrink var(--keep-window) linear forwards;
  }
  @keyframes shrink {
    to {
      transform: scaleX(0);
    }
  }
</style>
