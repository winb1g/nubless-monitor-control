<script lang="ts">
  import { rise, pop } from "../motion";
  import { fly, scale } from "svelte/transition";
  import { RefreshCw, CircleCheck, Download, Rocket, CloudOff, PackageOpen } from "@lucide/svelte";
  import { ui } from "../store.svelte";
  import { upd, checkForUpdates, installAndRestart } from "../updates.svelte";

  $effect(() => {
    if (upd.phase === "idle") checkForUpdates();
  });

  const busy = $derived(["checking", "downloading", "installing", "restarting"].includes(upd.phase));
  const ago = $derived(upd.checkedAt ? upd.checkedAt.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" }) : null);
</script>

<div class="page">
  <h1>Updates</h1>
  <p class="muted sub">You're on version {ui.state?.version ?? "…"}.</p>

  <section class="card status" in:fly={rise()}>
    {#key upd.phase}
      <div class="state" in:scale={pop}>
        {#if upd.phase === "checking" || upd.phase === "idle"}
          <span class="badge spin"><RefreshCw size={26} /></span>
          <h2>Checking for updates…</h2>
          <p class="muted">This only takes a moment.</p>
        {:else if upd.phase === "current"}
          <span class="badge ok"><CircleCheck size={28} /></span>
          <h2>You're up to date</h2>
          <p class="muted">nubless {ui.state?.version} is the latest version.</p>
        {:else if upd.phase === "unpublished"}
          <span class="badge"><PackageOpen size={26} /></span>
          <h2>No releases yet</h2>
          <p class="muted">You're running a development build. When the first release ships, it'll show up here.</p>
        {:else if upd.phase === "available"}
          <span class="badge new"><Download size={26} /></span>
          <h2>nubless {upd.version} is ready</h2>
          {#if upd.notes}<p class="notes">{upd.notes}</p>{/if}
          <button class="btn primary big" onclick={installAndRestart}><Rocket size={17} />Update and restart</button>
        {:else if upd.phase === "downloading"}
          <span class="badge new"><Download size={26} /></span>
          <h2>Downloading {upd.version}</h2>
          <div class="bar"><span style="width:{upd.progress}%"></span></div>
          <p class="muted small">{upd.progress}%</p>
        {:else if upd.phase === "installing"}
          <span class="badge spin new"><RefreshCw size={26} /></span>
          <h2>Installing…</h2>
          <p class="muted">Almost there.</p>
        {:else if upd.phase === "restarting"}
          <span class="badge ok"><Rocket size={26} /></span>
          <h2>Restarting nubless</h2>
          <p class="muted">See you in a second.</p>
        {:else}
          <span class="badge err"><CloudOff size={26} /></span>
          <h2>Couldn't check</h2>
          <p class="muted">{upd.error}</p>
        {/if}
      </div>
    {/key}
  </section>

  <div class="foot">
    <span class="faint small">{ago ? `Last checked at ${ago}` : ""}</span>
    <button class="btn ghost" onclick={checkForUpdates} disabled={busy}><RefreshCw size={15} />Check again</button>
  </div>
</div>

<style>
  .page {
    padding: 34px 40px 80px;
    max-width: 700px;
  }
  .sub {
    margin: 4px 0 24px;
  }
  .status {
    min-height: 280px;
    display: grid;
    place-items: center;
    text-align: center;
    padding: 40px;
  }
  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .badge {
    width: 68px;
    height: 68px;
    border-radius: 22px;
    display: grid;
    place-items: center;
    margin-bottom: 10px;
    background: var(--panel-2);
    color: var(--text-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .badge.ok {
    background: var(--mint-soft);
    color: var(--mint);
    box-shadow: none;
  }
  .badge.new {
    background: var(--grad);
    color: #fff;
    box-shadow: 0 10px 24px -10px rgba(108, 92, 255, 0.7);
  }
  .badge.err {
    background: var(--coral-soft);
    color: var(--coral);
    box-shadow: none;
  }
  .spin :global(svg) {
    animation: spin var(--spin) linear infinite;
  }
  h2 {
    font-size: 19px;
  }
  .notes {
    max-width: 440px;
    color: var(--text-2);
    background: var(--panel-2);
    padding: 12px 14px;
    border-radius: 14px;
    margin: 8px 0 6px;
  }
  .big {
    height: 42px;
    padding: 0 20px;
    margin-top: 10px;
  }
  .bar {
    width: 320px;
    height: 8px;
    border-radius: 99px;
    background: var(--panel-2);
    overflow: hidden;
    margin-top: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--grad);
    border-radius: 99px;
    transition: width 160ms linear;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 12px;
  }
</style>
