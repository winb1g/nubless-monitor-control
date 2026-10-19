<script lang="ts">
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
    <div class="toast {t.tone}" animate:flip={{ duration: 220 }} in:fly={{ y: 16, duration: 260 }} out:fly={{ y: 8, duration: 160 }}>
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
    gap: 8px;
    z-index: 50;
    pointer-events: none;
  }
  .compact {
    bottom: 64px;
    width: calc(100% - 40px);
  }
  .compact .toast {
    max-width: 100%;
    font-size: 12.5px;
  }
  .toast {
    pointer-events: auto;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 520px;
    padding: 10px 12px 10px 14px;
    border-radius: 14px;
    background: #1b1e29;
    color: #f1f2f7;
    box-shadow: var(--shadow-lg);
    font-weight: 500;
  }
  .success :global(svg) {
    color: #3ddc97;
  }
  .info :global(svg) {
    color: #9d90ff;
  }
  .warn :global(svg) {
    color: #ffc04d;
  }
  .error :global(svg) {
    color: #ff7a66;
  }
  .text {
    line-height: 1.35;
  }
  .undo {
    border: none;
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
    font-weight: 600;
    padding: 5px 12px;
    border-radius: 9px;
    transition: background var(--fast);
  }
  .undo:hover {
    background: rgba(255, 255, 255, 0.22);
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
