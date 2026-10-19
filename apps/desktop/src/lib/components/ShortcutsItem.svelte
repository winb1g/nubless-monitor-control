<script lang="ts">
  // Global keyboard shortcuts: a master switch, and each action's keys, which
  // you change by clicking them and pressing the new combination.
  import { RotateCcw } from "@lucide/svelte";
  import { api } from "../api";
  import { comboFrom, keyCaps } from "../keys";
  import type { HotkeyAction, HotkeysView } from "../types";
  import Item from "./Item.svelte";
  import Toggle from "./Toggle.svelte";

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

<Item title="Keyboard shortcuts" body="Change your monitor from anywhere, even with nubless closed to the tray. Click a shortcut to change it.">
  <Toggle checked={!!view?.enabled} label="Keyboard shortcuts" onchange={(v) => save(v)} />
  {#snippet below()}
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
                <span class="prompt">Press new keys… <span class="eyebrow">Esc to cancel</span></span>
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
              <span class="note err">{recordError}</span>
            {:else if b.error && view.enabled}
              <span class="note err">{b.error} Pick another.</span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/snippet}
</Item>

<style>
  .bindings {
    list-style: none;
    margin: var(--space-1) 0 0;
    padding: 0;
    transition: opacity var(--base) var(--ease);
  }
  .bindings.off {
    opacity: 0.5;
  }
  li {
    display: grid;
    grid-template-columns: 1fr auto 44px;
    align-items: center;
    gap: var(--space-2);
    min-height: 48px;
    border-top: 1px solid var(--rule);
  }
  .label {
    font-weight: 500;
  }
  .combo {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-height: 36px;
    padding: 4px 6px;
    border: 2px solid transparent;
    background: none;
    transition: background var(--fast) var(--ease);
  }
  .combo:hover:not(:disabled) {
    background: var(--wash);
  }
  .combo.recording {
    background: var(--signal);
    color: var(--on-signal);
    border-color: var(--ink);
  }
  .combo.error kbd {
    border-color: var(--err);
    color: var(--err);
  }
  .combo:disabled {
    cursor: default;
  }
  /* A keycap: outlined mono, nothing raised. */
  kbd {
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    min-width: 28px;
    padding: 3px 8px;
    text-align: center;
    border: 2px solid var(--ink);
    border-radius: var(--r-sm);
  }
  .prompt {
    font-size: var(--fs-sm);
    font-weight: 600;
    padding: 0 var(--space-1);
  }
  .prompt .eyebrow {
    color: var(--on-signal);
    margin-left: 6px;
  }
  .reset {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border: none;
    background: none;
    color: var(--dim);
    transition: background var(--fast) var(--ease), color var(--fast) var(--ease);
  }
  .reset:hover:not(:disabled) {
    background: var(--wash);
    color: var(--ink);
  }
  .note {
    grid-column: 1 / -1;
    justify-self: end;
    padding-bottom: var(--space-2);
    font-size: var(--fs-xs);
  }
  .err {
    color: var(--err);
  }
</style>
