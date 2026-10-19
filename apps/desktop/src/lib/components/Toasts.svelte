<script lang="ts">
  import { toastIn, toastOut, toastReflow } from "../motion";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { CircleCheck, Info, TriangleAlert, CircleX } from "@lucide/svelte";
  import { ui, dismiss } from "../store.svelte";

  let { compact = false }: { compact?: boolean } = $props();

  const icons = { success: CircleCheck, info: Info, warn: TriangleAlert, error: CircleX };
</script>

<div class="stack" class:compact aria-live="polite">
  {#each ui.toasts as t (t.id)}
    {@const Icon = icons[t.tone]}
    <div class="toast {t.tone}" animate:flip={toastReflow} in:fly={toastIn} out:fly={toastOut}>
      <Icon size={17} />
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
    gap: 10px;
    max-width: 520px;
    padding: 10px var(--space-3) 10px 14px;
    border-radius: var(--r-lg);
    background: var(--inverse);
    color: var(--inverse-text);
    box-shadow: var(--shadow-lg);
    font-weight: 500;
  }
  .success :global(svg) {
    color: var(--inverse-mint);
  }
  .info :global(svg) {
    color: var(--inverse-accent);
  }
  .warn :global(svg) {
    color: var(--inverse-sun);
  }
  .error :global(svg) {
    color: var(--inverse-coral);
  }
  .text {
    line-height: 1.35;
  }
  .undo {
    border: none;
    background: var(--inverse-fill);
    color: var(--inverse-text);
    font-weight: 600;
    padding: 5px var(--space-3);
    border-radius: 9px;
    transition: background var(--fast);
  }
  .undo:hover {
    background: var(--inverse-fill-hover);
  }
  .timer {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 2px;
    width: 100%;
    background: var(--grad);
    transform-origin: left;
    animation: shrink 5s linear forwards;
  }
  @keyframes shrink {
    to {
      transform: scaleX(0);
    }
  }
</style>
