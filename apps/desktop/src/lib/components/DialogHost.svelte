<script lang="ts">
  import { dialogIn, fadeQuick } from "../motion";
  import { fade, fly } from "svelte/transition";
  import { ui } from "../store.svelte";

  function onkeydown(e: KeyboardEvent) {
    if (!ui.dialog) return;
    if (e.key === "Escape") ui.dialog.resolve(false);
    if (e.key === "Enter") ui.dialog.resolve(true);
  }
</script>

<svelte:window {onkeydown} />

{#if ui.dialog}
  {@const d = ui.dialog}
  <div class="scrim" transition:fade={fadeQuick} onclick={() => d.resolve(false)} role="presentation">
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="dlg-title"
      tabindex="-1"
      in:fly={dialogIn}
      onclick={(e) => e.stopPropagation()}
      onkeydown={() => {}}
    >
      <h2 id="dlg-title">{d.title}</h2>
      <p class="muted">{d.body}</p>
      <div class="actions">
        <button class="btn" onclick={() => d.resolve(false)}>{d.cancel ?? "Cancel"}</button>
        <!-- svelte-ignore a11y_autofocus -->
        <button class={d.tone === "signal" ? "btn signal" : "btn primary"} autofocus onclick={() => d.resolve(true)}>{d.confirm}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    background: var(--scrim);
  }
  .dialog {
    width: min(440px, calc(100vw - 32px));
    padding: 18px 20px 14px;
    border-radius: var(--r-md);
    background: var(--ground);
    border: 2px solid var(--ink);
  }
  h2 {
    font-size: var(--fs-xl);
    margin-bottom: var(--space-2);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: var(--space-4);
  }
</style>
