<script lang="ts">
  import { rise } from "../motion";
  import { fly } from "svelte/transition";
  import { Check } from "@lucide/svelte";
  import { ui } from "../store.svelte";
  import { upd, checkForUpdates, installAndRestart } from "../updates.svelte";

  $effect(() => {
    if (upd.phase === "idle") checkForUpdates();
  });

  const busy = $derived(["checking", "downloading", "installing", "restarting"].includes(upd.phase));
  const ago = $derived(upd.checkedAt ? upd.checkedAt.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" }) : null);
</script>

<div class="page">
  <div class="page-head">
    <h1>Updates</h1>
    <p class="subtitle">You're on version {ui.state?.version ?? "…"}.</p>
  </div>

  <section class="status" in:fly={rise}>
    {#key upd.phase}
      <div class="state" in:fly={rise}>
        {#if upd.phase === "checking" || upd.phase === "idle"}
          <div class="hatch wide" aria-hidden="true"></div>
          <h2>Checking for updates…</h2>
          <p class="muted">This only takes a moment.</p>
        {:else if upd.phase === "current"}
          <span class="ok" aria-hidden="true"><Check size={22} strokeWidth={3} /></span>
          <h2>You're up to date</h2>
          <p class="muted">nubless {ui.state?.version} is the latest version.</p>
        {:else if upd.phase === "unpublished"}
          <h2>No releases yet</h2>
          <p class="muted">You're running a development build. When the first release ships, it'll show up here.</p>
        {:else if upd.phase === "available"}
          <div class="eyebrow">Available</div>
          <h2>nubless {upd.version} is ready</h2>
          {#if upd.notes}<p class="notes">{upd.notes}</p>{/if}
          <button class="btn signal" onclick={installAndRestart}>Update and restart</button>
        {:else if upd.phase === "downloading"}
          <h2>Downloading {upd.version}</h2>
          <div class="bar"><span style="width:{upd.progress}%"></span></div>
          <p class="mono small">{upd.progress}%</p>
        {:else if upd.phase === "installing"}
          <div class="hatch wide" aria-hidden="true"></div>
          <h2>Installing…</h2>
          <p class="muted">Almost there.</p>
        {:else if upd.phase === "restarting"}
          <h2>Restarting nubless</h2>
          <p class="muted">See you in a second.</p>
        {:else}
          <div class="eyebrow err">Error</div>
          <h2>Couldn't check</h2>
          <p class="muted">{upd.error}</p>
        {/if}
      </div>
    {/key}
  </section>

  <div class="foot">
    <span class="eyebrow">{ago ? `Last checked at ${ago}` : ""}</span>
    <button class="link" onclick={checkForUpdates} disabled={busy}>Check again</button>
  </div>
</div>

<style>
  .page {
    max-width: 860px;
  }
  .status {
    min-height: 260px;
    padding: 28px 0 32px;
    border-top: 2px solid var(--ink);
  }
  .state {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }
  h2 {
    font-size: var(--fs-3xl);
    margin-top: var(--space-2);
  }
  .state > .eyebrow + h2 {
    margin-top: 0;
  }
  .wide {
    width: 320px;
    max-width: 100%;
    height: 10px;
  }
  .ok {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border: 2px solid var(--ink);
    background: var(--signal);
    color: var(--on-signal);
  }
  .notes {
    max-width: 520px;
    color: var(--dim);
    margin: var(--space-1) 0 var(--space-3);
  }
  .state .btn {
    margin-top: var(--space-3);
  }
  .err {
    color: var(--err);
  }
  .bar {
    width: 520px;
    max-width: 100%;
    height: 20px;
    border: 2px solid var(--ink);
    margin-top: var(--space-2);
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--ink);
    transition: width var(--fast) linear;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: var(--space-2);
    border-top: 1px solid var(--rule);
  }
</style>
