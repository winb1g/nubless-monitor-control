<script lang="ts">
  import { setControl } from "../store.svelte";
  import type { ControlView } from "../types";
  import Lock from "./Lock.svelte";

  let { control }: { control: ControlView } = $props();
  let pending = $state<number | null>(null);
  const current = $derived(pending ?? control.value);
  const index = $derived(Math.max(0, control.choices.findIndex((c) => c.raw === current)));
  const known = $derived(control.choices.some((c) => c.raw === current));

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
  <div class="seg" style="--n:{control.choices.length};--i:{index}">
    {#if known}<span class="thumb"></span>{/if}
    {#each control.choices as c (c.raw)}
      <button
        class:active={c.raw === current}
        disabled={!control.enabled}
        onclick={() => pick(c.raw)}
        aria-pressed={c.raw === current}>{c.label}</button
      >
    {/each}
  </div>
</div>

<style>
  .row {
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
  .seg {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
    padding: 4px;
    border-radius: 14px;
    background: var(--panel-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .thumb {
    position: absolute;
    top: 4px;
    bottom: 4px;
    left: 4px;
    width: calc((100% - 8px) / var(--n));
    transform: translateX(calc(100% * var(--i)));
    border-radius: 10px;
    background: var(--panel);
    box-shadow: var(--shadow-sm), 0 0 0 1px var(--line);
    transition: transform var(--slow) var(--spring);
  }
  button {
    position: relative;
    z-index: 1;
    height: 34px;
    border: none;
    background: none;
    border-radius: 10px;
    font-weight: 550;
    color: var(--text-2);
    transition: color var(--fast);
  }
  button.active {
    color: var(--text);
  }
  button:active:not(:disabled) {
    transform: scale(0.97);
  }
  .off button {
    cursor: default;
  }
  .off .seg {
    opacity: 0.55;
  }
  .off .label {
    color: var(--text-2);
  }
</style>
