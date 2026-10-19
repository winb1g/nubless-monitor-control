<script lang="ts">
  import { fly } from "svelte/transition";
  import { History, FlaskConical, Palette, MonitorSmartphone, ShieldCheck, Search } from "@lucide/svelte";
  import { ui, resetOriginal, setLabs, useDemo, rescan } from "../store.svelte";
  import Toggle from "../components/Toggle.svelte";
  import { theme, setTheme } from "../theme.svelte";

  const s = $derived(ui.state);
</script>

<div class="page">
  <h1>Settings</h1>
  <p class="muted sub">Make nubless yours. Every change here is safe to try.</p>

  <section class="card hero" in:fly={{ y: 12, duration: 300 }}>
    <span class="icon big"><ShieldCheck size={22} /></span>
    <div class="text">
      <h2>Your original settings</h2>
      {#if s?.baseline.saved}
        <p class="muted">Saved the first time nubless connected, before it changed anything ({s.baseline.values} settings{s.baseline.saved_in ? ` in ${s.baseline.saved_in}` : ""}). Reset puts them back{s.baseline.saved_in ? `; each picture mode keeps its own settings, so reset works while your monitor is in ${s.baseline.saved_in}` : ""}.</p>
      {:else if s?.baseline.error}
        <p class="muted">Couldn't be saved: {s.baseline.error}. nubless won't change anything until this works.</p>
      {:else}
        <p class="muted">Saved automatically when nubless first connects to your monitor.</p>
      {/if}
    </div>
    <button class="btn" onclick={resetOriginal} disabled={!s?.baseline.saved || ui.busy}>
      <History size={16} />{ui.busy ? "Resetting…" : "Reset to my original settings"}
    </button>
  </section>

  <section class="card row" in:fly={{ y: 12, duration: 300, delay: 50 }}>
    <span class="icon"><FlaskConical size={18} /></span>
    <div class="text">
      <h2>Labs</h2>
      <p class="muted">Try controls that match LG's own names and values but haven't been tested on a real monitor yet, like Response Time and Adaptive-Sync. Safe ranges and interlocks still apply.</p>
    </div>
    <Toggle checked={!!s?.labs} label="Labs" onchange={(v) => setLabs(v)} />
  </section>

  <section class="card row" in:fly={{ y: 12, duration: 300, delay: 100 }}>
    <span class="icon"><Palette size={18} /></span>
    <div class="text">
      <h2>Appearance</h2>
      <p class="muted">Follow your system or pick a side.</p>
    </div>
    <div class="seg">
      {#each ["system", "light", "dark"] as t}
        <button class:active={theme.value === t} onclick={() => setTheme(t as "system" | "light" | "dark")}>{t[0].toUpperCase() + t.slice(1)}</button>
      {/each}
    </div>
  </section>

  <section class="card row" in:fly={{ y: 12, duration: 300, delay: 150 }}>
    <span class="icon"><MonitorSmartphone size={18} /></span>
    <div class="text">
      <h2>Demo monitor</h2>
      <p class="muted">{s?.demo ? "You're using a simulated 27GM950B. Nothing is sent to real hardware." : "Explore every control on a simulated 27GM950B. Nothing is sent to real hardware."}</p>
    </div>
    {#if s?.demo}
      <button class="btn" onclick={rescan}><Search size={16} />Find my monitor</button>
    {:else}
      <button class="btn" onclick={useDemo}>Use demo</button>
    {/if}
  </section>
</div>

<style>
  .page {
    padding: 34px 40px 80px;
    max-width: 860px;
  }
  .sub {
    margin: 4px 0 24px;
  }
  .card {
    margin-bottom: 14px;
  }
  .row,
  .hero {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .hero {
    background: linear-gradient(135deg, var(--accent-soft), transparent 65%), var(--panel);
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .icon {
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    background: var(--panel-2);
    color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .icon.big {
    width: 46px;
    height: 46px;
    border-radius: 14px;
    background: var(--grad);
    color: #fff;
    box-shadow: none;
  }
  .seg {
    display: flex;
    padding: 3px;
    gap: 2px;
    border-radius: 12px;
    background: var(--panel-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .seg button {
    border: none;
    background: none;
    padding: 6px 12px;
    border-radius: 9px;
    font-weight: 550;
    color: var(--text-2);
    transition: background var(--fast), color var(--fast);
  }
  .seg button.active {
    background: var(--panel);
    color: var(--text);
    box-shadow: var(--shadow-sm), 0 0 0 1px var(--line);
  }
</style>
