<script lang="ts">
  import { fly } from "svelte/transition";
  import { Maximize2, Power, Search } from "@lucide/svelte";
  import { ui, rescan } from "../store.svelte";
  import { api } from "../api";
  import Slider from "../components/Slider.svelte";
  import Lock from "../components/Lock.svelte";

  const s = $derived(ui.state);
  const ctl = (id: string) => s?.controls.find((c) => c.id === id);
  const pm = $derived(ctl("picture-mode"));
  const open = async (page?: string) => (await api()).openMain(page);
  const quit = async () => (await api()).quit();
</script>

<div class="pop" in:fly={{ y: 10, duration: 220 }}>
  <header>
    <img src="/logo.svg" alt="" width="28" height="28" />
    <div class="who">
      <b>{s?.monitors.find((m) => m.selected)?.name ?? "nubless"}</b>
      <span class="small" class:ok={s?.phase === "connected"}>
        {s?.phase === "connected" ? (s.demo ? "Demo monitor" : "Connected") : s ? "No monitor found" : "Connecting…"}
      </span>
    </div>
    <button class="icon" onclick={() => open()} aria-label="Open nubless"><Maximize2 size={16} /></button>
  </header>

  {#if !s}
    <div class="skeleton" style="height:56px;margin:8px 0"></div>
    <div class="skeleton" style="height:56px;margin:8px 0"></div>
  {:else if s.phase !== "connected"}
    <div class="empty">
      <p class="muted">Your 27GM950B didn't answer.</p>
      <button class="btn primary" onclick={rescan} disabled={ui.busy}><Search size={15} />{ui.busy ? "Looking…" : "Scan again"}</button>
    </div>
  {:else}
    {#if ctl("brightness")}<Slider control={ctl("brightness")!} compact />{/if}
    {#if ctl("contrast")}<Slider control={ctl("contrast")!} compact />{/if}
    {#if pm}
      <div class="pmhead"><span>Picture mode</span>{#if !pm.enabled}<Lock control={pm} />{/if}</div>
      <div class="chips">
        {#each pm.choices.slice(0, 4) as c (c.raw)}
          <button class="chip" class:active={pm.value === c.raw} disabled={!pm.enabled}>{c.label}</button>
        {/each}
      </div>
    {/if}
  {/if}

  <footer>
    <button class="row" onclick={() => open()}><Maximize2 size={15} />Open nubless</button>
    <button class="row" onclick={quit}><Power size={15} />Quit</button>
  </footer>
</div>

<style>
  .pop {
    margin: 10px;
    padding: 16px 18px 10px;
    border-radius: 22px;
    background: var(--panel);
    border: 1px solid var(--line);
    box-shadow: 0 18px 40px -16px rgba(16, 24, 40, 0.45);
    height: calc(100vh - 20px);
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--line);
    margin-bottom: 4px;
  }
  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .who span {
    color: var(--text-3);
  }
  .who span.ok {
    color: var(--mint);
    font-weight: 600;
  }
  .icon {
    border: none;
    background: var(--panel-2);
    width: 32px;
    height: 32px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: var(--text-2);
    transition: transform var(--fast) var(--spring);
  }
  .icon:hover {
    transform: scale(1.08);
    color: var(--accent);
  }
  .pmhead {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-weight: 550;
    font-size: 13px;
    margin: 8px 0;
  }
  .chips {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .chip {
    border: 1px solid var(--line);
    background: var(--panel-2);
    border-radius: 12px;
    padding: 8px;
    font-size: 12px;
    font-weight: 550;
    color: var(--text-2);
  }
  .chip.active {
    box-shadow: 0 0 0 2px var(--accent);
    color: var(--text);
  }
  .chip:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }
  footer {
    margin-top: auto;
    border-top: 1px solid var(--line);
    padding-top: 6px;
    display: flex;
    justify-content: space-between;
  }
  .row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border: none;
    background: none;
    padding: 8px 10px;
    border-radius: 10px;
    color: var(--text-2);
    font-weight: 550;
  }
  .row:hover {
    background: var(--panel-2);
    color: var(--text);
  }
</style>
