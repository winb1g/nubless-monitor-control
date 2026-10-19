<script lang="ts">
  let {
    checked,
    disabled = false,
    pending = false,
    label,
    onchange,
  }: { checked: boolean; disabled?: boolean; pending?: boolean; label: string; onchange: (on: boolean) => void } = $props();
</script>

<button
  class="toggle"
  class:on={checked}
  class:pending
  role="switch"
  aria-checked={checked}
  aria-busy={pending}
  aria-label={label}
  {disabled}
  onclick={() => !pending && onchange(!checked)}
>
  <span class="knob" class:busy-ring={pending}></span>
</button>

<style>
  .toggle {
    flex: none;
    width: 46px;
    height: 28px;
    border-radius: 99px;
    border: none;
    padding: 3px;
    background: var(--line-strong);
    transition: background var(--base) var(--ease);
  }
  .toggle.on {
    background: var(--grad);
  }
  .knob {
    position: relative;
    display: block;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 6px rgba(20, 24, 40, 0.25);
    transition: transform var(--base) var(--spring), width var(--fast) var(--ease);
  }
  .toggle:active:not(:disabled):not(.pending) .knob {
    width: 27px;
  }
  .on .knob {
    transform: translateX(18px);
  }
  .on:active:not(:disabled):not(.pending) .knob {
    transform: translateX(13px);
  }
  /* Waiting on the monitor: the shared busy ring (app.css) spins inside the knob. */
  .pending {
    cursor: progress;
  }
  .knob {
    --ring-inset: 5px;
  }
  .toggle:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
