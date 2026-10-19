<script lang="ts">
  // An ON/OFF key: the state is a word, and "on" is a signal fill. While the
  // monitor confirms a change it shows "…" with the busy bar under it.
  let {
    checked,
    disabled = false,
    pending = false,
    label,
    onchange,
  }: { checked: boolean; disabled?: boolean; pending?: boolean; label: string; onchange: (on: boolean) => void } = $props();
</script>

<span class="wrap">
  <button
    class="key"
    class:on={checked}
    class:pending
    role="switch"
    aria-checked={checked}
    aria-busy={pending}
    aria-label={label}
    {disabled}
    onclick={() => !pending && onchange(!checked)}
  >
    {pending ? "…" : checked ? "ON" : "OFF"}
  </button>
  {#if pending}<span class="hatch tick" aria-hidden="true"></span>{/if}
</span>

<style>
  .wrap {
    position: relative;
    display: inline-block;
    flex: none;
  }
  .key {
    min-width: 72px;
    height: var(--hit);
    border: 2px solid var(--ink);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--ink);
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    font-weight: 500;
    letter-spacing: 0.08em;
    transition: background var(--fast) var(--ease);
  }
  .key:hover:not(:disabled) {
    background: var(--wash);
  }
  .key.on {
    background: var(--signal);
    color: var(--on-signal);
  }
  .key.on:hover:not(:disabled) {
    background: var(--signal-hover);
  }
  .pending {
    cursor: progress;
  }
  .tick {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 3px);
    height: 3px;
  }
  .key:disabled {
    opacity: 0.45;
    border-style: dashed;
    cursor: default;
  }
  .pending:disabled {
    opacity: 1;
    border-style: solid;
  }
</style>
