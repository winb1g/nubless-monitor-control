<script lang="ts">
  import { Lock, FlaskConical, Info } from "@lucide/svelte";
  import { ui } from "../store.svelte";
  import type { ControlView } from "../types";

  let { control }: { control: Pick<ControlView, "reason" | "unlock"> } = $props();
  let open = $state(false);
  const info = $derived(control.reason === "Information from your monitor.");
</script>

{#if control.reason && !info}
  <span class="wrap">
    <button
      class="chip"
      class:labs={control.unlock === "labs"}
      onclick={() => (open = !open)}
      onblur={() => setTimeout(() => (open = false), 150)}
      aria-label={control.reason}
    >
      {#if control.unlock === "labs"}<FlaskConical size={13} />Labs{:else}<Lock size={12} />Locked{/if}
    </button>
    {#if open}
      <span class="pop" role="tooltip">
        <Info size={14} />
        <span>
          {control.reason}
          {#if control.unlock === "labs"}
            <button class="link" onclick={() => (ui.page = "settings")}>Turn on Labs</button>
          {/if}
        </span>
      </span>
    {/if}
  </span>
{/if}

<style>
  .wrap {
    position: relative;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border: none;
    font-size: var(--fs-xs);
    font-weight: 600;
    padding: 3px 9px;
    border-radius: var(--r-full);
    background: var(--panel-2);
    color: var(--text-3);
    box-shadow: inset 0 0 0 1px var(--line);
    transition: transform var(--fast) var(--spring);
  }
  .chip:hover {
    transform: translateY(-1px);
  }
  .chip.labs {
    background: var(--accent-soft);
    color: var(--accent-ink);
    box-shadow: none;
  }
  .pop {
    position: absolute;
    right: 0;
    top: calc(100% + var(--space-2));
    z-index: 20;
    width: 260px;
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3);
    font-size: var(--fs-sm);
    line-height: 1.45;
    color: var(--text-2);
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-lg);
    animation: in var(--fast) var(--ease);
  }
  .pop :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--accent);
  }
  .link {
    display: block;
    margin-top: 6px;
    border: none;
    background: none;
    padding: 0;
    color: var(--accent-ink);
    font-weight: 600;
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }
</style>
