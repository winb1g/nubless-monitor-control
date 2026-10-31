<script lang="ts">
  // A choice among a few named values (Response Time: Normal, Fast, Faster).
  // The selected one is an ink fill; a locked control gets a dashed outline, and
  // clicking one of its options opens the reason instead of doing nothing.
  import { setControl } from "../store.svelte";
  import type { ControlView } from "../types";
  import Lock from "./Lock.svelte";

  let { control }: { control: ControlView } = $props();
  let pending = $state<number | null>(null);
  let lockOpen = $state(false);
  const why = $derived(`why-${control.id}`);
  const current = $derived(pending ?? control.value);

  async function pick(raw: number) {
    if (!control.enabled) {
      lockOpen = true;
      return;
    }
    if (raw === control.value) return;
    pending = raw;
    await setControl(control.id, raw, { before: control.value });
    pending = null;
  }
</script>

<div class="row" class:off={!control.enabled}>
  <div class="head">
    <span class="label">{control.label}</span>
    {#if !control.enabled}<Lock {control} bind:open={lockOpen} />{/if}
  </div>
  {#if !control.enabled}<span id={why} class="sr">{control.reason}</span>{/if}
  <div class="seg" role="group" aria-label={control.label} aria-describedby={control.enabled ? undefined : why}>
    {#each control.choices as c (c.raw)}
      <button class:active={c.raw === current} aria-disabled={!control.enabled} onclick={() => pick(c.raw)} aria-pressed={c.raw === current}>{c.label}</button>
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
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .seg {
    display: inline-flex;
    max-width: 100%;
    overflow-x: auto;
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
  button:hover:not([aria-disabled="true"]):not(.active) {
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
