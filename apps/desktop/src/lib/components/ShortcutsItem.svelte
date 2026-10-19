<script lang="ts">
  // Global keyboard shortcuts: a master switch, and each action's keys, which
  // you change by clicking them and pressing the new combination.
  import { Keyboard, RotateCcw } from "@lucide/svelte";
  import { api } from "../api";
  import { comboFrom, keyCaps } from "../keys";
  import type { HotkeyAction, HotkeysView } from "../types";
  import Card from "./Card.svelte";
  import IconBadge from "./IconBadge.svelte";
  import Toggle from "./Toggle.svelte";

  let { i = 0 }: { i?: number } = $props();

  let view = $state<HotkeysView | null>(null);
  let recording = $state<HotkeyAction | null>(null);
  let recordError = $state<string | null>(null);

  $effect(() => {
    api().then((b) => b.getHotkeys()).then((v) => (view = v));
  });

  async function save(enabled: boolean, changes: Partial<Record<HotkeyAction, string>> = {}) {
    if (!view) return;
    const bindings = Object.fromEntries(view.bindings.map((b) => [b.action, b.keys])) as Record<HotkeyAction, string>;
    view = await (await api()).setHotkeys(enabled, { ...bindings, ...changes });
  }

  function record(action: HotkeyAction) {
    recording = recording === action ? null : action;
    recordError = null;
  }

  function onkeydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.code === "Escape" && !e.ctrlKey && !e.altKey && !e.metaKey) {
      recording = null;
      return;
    }
    const combo = comboFrom(e);
    if (!combo) return;
    if ("error" in combo) {
      recordError = combo.error;
      return;
    }
    const action = recording;
    // Refuse a clash here, so the shortcut you didn't touch keeps working.
    const taken = view!.bindings.find((b) => b.action !== action && b.keys === combo.keys);
    if (taken) {
      recordError = `${keyCaps(combo.keys).join(" ")} is already ${taken.label}.`;
      return;
    }
    recording = null;
    recordError = null;
    save(view!.enabled, { [action]: combo.keys });
  }
</script>

<svelte:window {onkeydown} />

<Card row i={i} wrap>
  <IconBadge icon={Keyboard} size={38} />
  <div class="text">
    <h2>Keyboard shortcuts</h2>
    <p class="muted">Change your monitor from anywhere, even with nubless closed to the tray. Click a shortcut to change it.</p>
  </div>
  <Toggle checked={!!view?.enabled} label="Keyboard shortcuts" onchange={(v) => save(v)} />

  {#if view}
    <ul class="bindings" class:off={!view.enabled}>
      {#each view.bindings as b (b.action)}
        <li>
          <span class="label">{b.label}</span>
          <button
            class="combo"
            class:recording={recording === b.action}
            class:error={!!b.error}
            disabled={!view.enabled}
            onclick={() => record(b.action)}
            aria-label="{b.label}: {keyCaps(b.keys).join(' ')}. Click to change."
          >
            {#if recording === b.action}
              <span class="prompt">Press new keys… <span class="muted">Esc to cancel</span></span>
            {:else}
              {#each keyCaps(b.keys) as cap}<kbd>{cap}</kbd>{/each}
            {/if}
          </button>
          {#if b.keys !== b.default_keys}
            <button class="reset" title="Back to {keyCaps(b.default_keys).join(' ')}" aria-label="Reset {b.label} to its default" disabled={!view.enabled} onclick={() => save(view!.enabled, { [b.action]: b.default_keys })}>
              <RotateCcw size={14} />
            </button>
          {/if}
          {#if recording === b.action && recordError}
            <span class="note warn">{recordError}</span>
          {:else if b.error && view.enabled}
            <span class="note warn">{b.error} Pick another.</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</Card>

<style>
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .bindings {
    flex-basis: 100%;
    list-style: none;
    margin: var(--space-2) 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    transition: opacity var(--base) var(--ease);
  }
  .bindings.off {
    opacity: 0.5;
  }
  li {
    display: grid;
    grid-template-columns: 1fr auto 28px;
    align-items: center;
    gap: var(--space-2);
  }
  .label {
    font-weight: 550;
  }
  .combo {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-height: 34px;
    padding: 4px var(--space-2);
    border: none;
    border-radius: var(--r-sm);
    background: var(--panel-2);
    box-shadow: inset 0 0 0 1px var(--line);
    transition: box-shadow var(--fast), background var(--fast);
  }
  .combo:hover:not(:disabled) {
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  .combo.recording {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }
  .combo.error {
    box-shadow: inset 0 0 0 1.5px var(--coral);
  }
  .combo:disabled {
    cursor: default;
  }
  kbd {
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: 600;
    min-width: 24px;
    padding: 2px 7px;
    text-align: center;
    border-radius: var(--r-xs);
    background: var(--panel);
    box-shadow: var(--shadow-sm), inset 0 -1px 0 var(--line);
  }
  .prompt {
    font-size: var(--fs-sm);
    color: var(--accent-ink);
    font-weight: 550;
    padding: 0 var(--space-1);
  }
  .reset {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: var(--r-sm);
    background: none;
    color: var(--text-2);
    transition: background var(--fast), color var(--fast);
  }
  .reset:hover:not(:disabled) {
    background: var(--panel-2);
    color: var(--text);
  }
  .note {
    grid-column: 1 / -1;
    justify-self: end;
    font-size: var(--fs-xs);
  }
  .warn {
    color: var(--coral);
  }
</style>
