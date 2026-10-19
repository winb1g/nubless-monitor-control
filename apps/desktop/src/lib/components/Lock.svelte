<script lang="ts">
  // Why a control can't be used yet. Labs controls get the flask; anything else
  // that is locked gets the padlock and a dashed outline. Click for the reason.
  import { Lock, FlaskConical } from "@lucide/svelte";
  import { ui } from "../store.svelte";
  import type { ControlView } from "../types";

  let { control }: { control: Pick<ControlView, "reason" | "unlock"> } = $props();
  let open = $state(false);
  const info = $derived(control.reason === "Information from your monitor.");
  const labs = $derived(control.unlock === "labs");
</script>

{#if control.reason && !info}
  <span class="wrap">
    <button
      class="tag"
      class:dashed={!labs}
      onclick={() => (open = !open)}
      onblur={() => setTimeout(() => (open = false), 150)}
      aria-label={control.reason}
      aria-expanded={open}
    >
      {#if labs}<FlaskConical size={11} strokeWidth={2.25} />Labs{:else}<Lock size={11} strokeWidth={2.25} />Locked{/if}
    </button>
    {#if open}
      <span class="pop" role="tooltip">
        {control.reason}
        {#if labs}
          <button class="link" onclick={() => (ui.page = "settings")}>Turn on Labs</button>
        {/if}
      </span>
    {/if}
  </span>
{/if}

<style>
  .wrap {
    position: relative;
  }
  .tag {
    min-height: 28px;
    cursor: pointer;
  }
  .tag:hover {
    background: var(--wash);
  }
  .pop {
    position: absolute;
    right: 0;
    top: calc(100% + var(--space-2));
    z-index: 20;
    width: 260px;
    padding: 10px var(--space-3);
    font-size: var(--fs-sm);
    line-height: 1.45;
    color: var(--ink);
    background: var(--ground);
    border: 2px solid var(--ink);
    border-radius: var(--r-md);
    animation: in var(--fast) var(--ease);
  }
  .link {
    display: block;
    min-height: 0;
    margin-top: 6px;
    font-size: var(--fs-sm);
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>
