<script lang="ts">
  // A choice among a few named values (Response Time: Normal, Fast, Faster).
  // The selected one is an ink fill; a locked control gets a dashed outline.
  import { setControl } from "../store.svelte";
  import type { ControlView } from "../types";
  import Lock from "./Lock.svelte";

  let { control }: { control: ControlView } = $props();
  let pending = $state<number | null>(null);
  const current = $derived(pending ?? control.value);

  async function pick(raw: number) {
    if (!control.enabled || raw === control.value) return;
    pending = raw;
    await setControl(control.id, raw, { before: control.value });
    pending = null;
  }
</script>

<div class="row" class:off={!control.enabled}>
  <div class="head">
    <span class="label">{control.label}</span>
    {#if !control.enabled}<Lock {control} />{/if}
  </div>
  <div class="seg" role="group" aria-label={control.label}>
    {#each control.choices as c (c.raw)}
      <button class:active={c.raw === current} disabled={!control.enabled} onclick={() => pick(c.raw)} aria-pressed={c.raw === current}>{c.label}</button>
    {/each}
  </div>
</div>

<style>
  .row {
    padding: var(--space-2) 0;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-2);
  }
  .label {
    font-weight: 600;
  }
  .seg {
    display: inline-flex;
    border: 2px solid var(--ink);
    border-radius: var(--r-sm);
  }
  button {
    min-height: 40px;
    padding: 0 14px;
    border: none;
    background: transparent;
    color: var(--ink);
    font-weight: 600;
    transition: background var(--fast) var(--ease), color var(--fast) var(--ease);
  }
  button:hover:not(:disabled):not(.active) {
    background: var(--wash);
  }
  button.active {
    background: var(--ink);
    color: var(--ground);
  }
  .off .seg {
    opacity: 0.6;
    border-style: dashed;
  }
  .off button {
    cursor: default;
  }
</style>
