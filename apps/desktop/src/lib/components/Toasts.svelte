<script lang="ts">
  import { toastIn, toastOut, toastReflow } from "../motion";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { ui, dismiss } from "../store.svelte";

  let { compact = false }: { compact?: boolean } = $props();

  // Success and info are plain text; warnings and errors carry a colored label.
  const labels = { warn: "Notice", error: "Error" } as const;
</script>

<div class="stack" class:compact aria-live="polite">
  {#each ui.toasts as t (t.id)}
    <div class="toast" animate:flip={toastReflow} in:fly={toastIn} out:fly={toastOut}>
      {#if t.tone === "warn" || t.tone === "error"}<span class="label {t.tone}">{labels[t.tone]}</span>{/if}
      <span class="text">{t.text}</span>
      {#if t.undo}
        <button
          class="undo"
          onclick={() => {
            dismiss(t.id);
            t.undo?.();
          }}>Undo</button
        >
        <span class="timer"></span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    left: 50%;
    bottom: 22px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    z-index: 50;
    pointer-events: none;
  }
  .compact {
    bottom: 64px;
    width: calc(100% - 40px);
  }
  .compact .toast {
    max-width: 100%;
    font-size: var(--fs-sm);
  }
  .toast {
    pointer-events: auto;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    max-width: 520px;
    padding: 10px 14px;
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
  }
  .warn {
    color: var(--toast-warn);
  }
  .error {
    color: var(--toast-err);
  }
  .text {
    line-height: 1.35;
  }
  .undo {
    flex: none;
    min-height: 36px;
    padding: 0 8px;
    border: none;
    background: none;
    color: var(--toast-signal);
    font-weight: 800;
  }
  .undo:hover {
    text-decoration: underline;
  }
  /* How long Undo stays on offer. */
  .timer {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 2px;
    width: 100%;
    background: var(--toast-signal);
    transform-origin: left;
    animation: shrink var(--undo-window) linear forwards;
  }
  @keyframes shrink {
    to {
      transform: scaleX(0);
    }
  }
</style>
