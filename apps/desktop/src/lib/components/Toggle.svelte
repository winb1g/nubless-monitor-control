<script lang="ts">
  let {
    checked,
    disabled = false,
    label,
    onchange,
  }: { checked: boolean; disabled?: boolean; label: string; onchange: (on: boolean) => void } = $props();
</script>

<button
  class="toggle"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
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
    display: block;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 6px rgba(20, 24, 40, 0.25);
    transition: transform var(--base) var(--spring), width var(--fast) var(--ease);
  }
  .toggle:active:not(:disabled) .knob {
    width: 27px;
  }
  .on .knob {
    transform: translateX(18px);
  }
  .on:active:not(:disabled) .knob {
    transform: translateX(13px);
  }
  .toggle:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
