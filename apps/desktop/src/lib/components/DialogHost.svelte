<script lang="ts">
  import { pop, fadeQuick } from "../motion";
  import { fade, scale } from "svelte/transition";
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
      transition:scale={pop}
      onclick={(e) => e.stopPropagation()}
      onkeydown={() => {}}
    >
      <h2 id="dlg-title">{d.title}</h2>
      <p class="muted">{d.body}</p>
      <div class="actions">
        <button class="btn" onclick={() => d.resolve(false)}>{d.cancel ?? "Cancel"}</button>
        <!-- svelte-ignore a11y_autofocus -->
        <button class="btn primary" autofocus onclick={() => d.resolve(true)}>{d.confirm}</button>
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
    background: rgba(10, 12, 20, 0.38);
    backdrop-filter: blur(6px);
  }
  .dialog {
    width: min(420px, calc(100vw - 32px));
    padding: 24px;
    border-radius: var(--r-lg);
    background: var(--panel);
    box-shadow: var(--shadow-lg);
    border: 1px solid var(--line);
  }
  h2 {
    font-size: 17px;
    margin-bottom: 8px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 22px;
  }
</style>
