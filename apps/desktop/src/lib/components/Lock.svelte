<script lang="ts">
  // Why a control can't be used right now. Controls Labs would unlock get the
  // flask; anything else that is locked gets the padlock and a dashed outline.
  // A disclosure: click or Enter opens the reason, Escape or a click elsewhere
  // closes it. The reason is also the button's accessible description, so a
  // screen reader hears it without opening anything.
  import { Lock, FlaskConical } from "@lucide/svelte";
  import { ui } from "../store.svelte";
  import type { ControlView } from "../types";

  let { control, open = $bindable(false) }: { control: Pick<ControlView, "reason" | "unlock">; open?: boolean } = $props();
  const id = `lock-${Math.random().toString(36).slice(2, 8)}`;
  let root = $state<HTMLElement>();
  let button = $state<HTMLButtonElement>();
  const info = $derived(control.reason === "Information from your monitor.");
  const labs = $derived(control.unlock === "labs");

  function away(e: Event) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
  function escape(e: KeyboardEvent) {
    if (open && e.key === "Escape") {
      open = false;
      button?.focus();
    }
  }
</script>

<svelte:window onpointerdown={away} onkeydown={escape} />

{#if control.reason && !info}
  <span class="wrap" bind:this={root}>
    <button
      bind:this={button}
      class="tag"
      class:dashed={!labs}
      onclick={() => (open = !open)}
      aria-expanded={open}
      aria-controls={id}
      aria-describedby="{id}-why"
    >
      {#if labs}<FlaskConical size={11} strokeWidth={2.25} />Labs{:else}<Lock size={11} strokeWidth={2.25} />Locked{/if}
    </button>
    <span id="{id}-why" class="sr">{control.reason}</span>
    {#if open}
      <span class="pop" {id}>
        <span aria-hidden="true">{control.reason}</span>
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
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .pop {
    position: absolute;
    right: 0;
    top: calc(100% + var(--space-2));
    z-index: 20;
    width: 260px;
    max-width: calc(100vw - 2 * var(--space-4));
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
