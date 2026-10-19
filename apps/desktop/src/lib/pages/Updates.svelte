<script lang="ts">
  import { rise, pop } from "../motion";
  import { fly, scale } from "svelte/transition";
  import { RefreshCw, CircleCheck, Download, Rocket, CloudOff, PackageOpen } from "@lucide/svelte";
  import { ui } from "../store.svelte";
  import IconBadge from "../components/IconBadge.svelte";
  import { upd, checkForUpdates, installAndRestart } from "../updates.svelte";

  $effect(() => {
    if (upd.phase === "idle") checkForUpdates();
  });

  const busy = $derived(["checking", "downloading", "installing", "restarting"].includes(upd.phase));
  const ago = $derived(upd.checkedAt ? upd.checkedAt.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" }) : null);
</script>

<div class="page">
  <h1>Updates</h1>
  <p class="subtitle">You're on version {ui.state?.version ?? "…"}.</p>

  <section class="card status" in:fly={rise()}>
    {#key upd.phase}
      <div class="state" in:scale={pop}>
        {#if upd.phase === "checking" || upd.phase === "idle"}
          <IconBadge icon={RefreshCw} size={68} iconSize={26} tone="neutral" spin />
          <h2>Checking for updates…</h2>
          <p class="muted">This only takes a moment.</p>
        {:else if upd.phase === "current"}
          <IconBadge icon={CircleCheck} size={68} iconSize={28} tone="mint" />
          <h2>You're up to date</h2>
          <p class="muted">nubless {ui.state?.version} is the latest version.</p>
        {:else if upd.phase === "unpublished"}
          <IconBadge icon={PackageOpen} size={68} iconSize={26} tone="neutral" />
          <h2>No releases yet</h2>
          <p class="muted">You're running a development build. When the first release ships, it'll show up here.</p>
        {:else if upd.phase === "available"}
          <IconBadge icon={Download} size={68} iconSize={26} tone="brand" glow />
          <h2>nubless {upd.version} is ready</h2>
          {#if upd.notes}<p class="notes">{upd.notes}</p>{/if}
          <button class="btn primary big" onclick={installAndRestart}><Rocket size={17} />Update and restart</button>
        {:else if upd.phase === "downloading"}
          <IconBadge icon={Download} size={68} iconSize={26} tone="brand" glow />
          <h2>Downloading {upd.version}</h2>
          <div class="bar"><span style="width:{upd.progress}%"></span></div>
          <p class="muted small">{upd.progress}%</p>
        {:else if upd.phase === "installing"}
          <IconBadge icon={RefreshCw} size={68} iconSize={26} tone="brand" glow spin />
          <h2>Installing…</h2>
          <p class="muted">Almost there.</p>
        {:else if upd.phase === "restarting"}
          <IconBadge icon={Rocket} size={68} iconSize={26} tone="mint" />
          <h2>Restarting nubless</h2>
          <p class="muted">See you in a second.</p>
        {:else}
          <IconBadge icon={CloudOff} size={68} iconSize={26} tone="coral" />
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
    max-width: 700px;
  }
  .status {
    min-height: 280px;
    display: grid;
    place-items: center;
    text-align: center;
    padding: var(--space-10);
  }
  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
  }
  h2 {
    font-size: var(--fs-2xl);
    margin-top: 10px; /* breathing room under the status badge */
  }
  .notes {
    max-width: 440px;
    color: var(--text-2);
    background: var(--panel-2);
    padding: var(--space-3) 14px;
    border-radius: var(--r-lg);
    margin: var(--space-2) 0 6px;
  }
  .big {
    height: 42px;
    padding: 0 var(--space-5);
    margin-top: 10px;
  }
  .bar {
    width: 320px;
    height: 8px;
    border-radius: var(--r-full);
    background: var(--panel-2);
    overflow: hidden;
    margin-top: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--grad);
    border-radius: var(--r-full);
    transition: width var(--fast) linear;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: var(--space-3);
  }
</style>
