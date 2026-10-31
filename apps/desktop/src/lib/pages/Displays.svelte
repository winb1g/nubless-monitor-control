<script lang="ts">
  import { rise, hint } from "../motion";
  import { fly } from "svelte/transition";
  import { FlaskConical, Search } from "@lucide/svelte";
  import { ui, toggleFeature, simulatePbp, rescan, useDemo, setControl, selectMonitor, setLinked } from "../store.svelte";
  import type { ControlView, Feature, FeatureState } from "../types";
  import Section from "../components/Section.svelte";
  import Hint from "../components/Hint.svelte";
  import MonitorArt from "../components/MonitorArt.svelte";
  import ModePicker from "../components/ModePicker.svelte";
  import Slider from "../components/Slider.svelte";
  import Segmented from "../components/Segmented.svelte";
  import Toggle from "../components/Toggle.svelte";
  import Lock from "../components/Lock.svelte";

  const s = $derived(ui.state);
  const byId = $derived(Object.fromEntries((s?.controls ?? []).map((c) => [c.id, c])) as Record<string, ControlView>);
  const group = (g: ControlView["group"]) => (s?.controls ?? []).filter((c) => c.group === g);

  const smart: { id: Feature; key: keyof FeatureState; title: string; body: string }[] = [
    { id: "ai-scene-optimization", key: "ai_scene_optimization", title: "AI Scene Optimization", body: "Your monitor tunes the picture to what's on screen." },
    { id: "ai-picture-mode", key: "ai_picture_mode", title: "AI Picture Mode", body: "Picks a picture mode for what you're doing." },
    { id: "my-display-preset", key: "my_display_preset", title: "My Display Preset", body: "Switches picture mode per app." },
  ];

  const pbpReason = "Not available while PBP/PIP is on. Turn PBP/PIP off first.";
  const mute = $derived(byId["mute"]);
  const aiSound = $derived(byId["ai-sound"]);
  /** A picture-mode switch is in flight or its values are being re-read. */
  const loading = $derived(!!ui.switching || !!s?.refreshing);
  const labsCount = $derived(s?.controls.filter((c) => c.unlock === "labs").length ?? 0);
  /** Monitors nubless can control; with two or more they can be linked. */
  const supportedCount = $derived(s?.monitors.filter((m) => m.supported).length ?? 0);

  /** Why a smart feature can't be turned on right now, if it can't. */
  function featureLock(id: Feature, on: boolean): { reason: string; unlock: "labs" | null } | null {
    if (!s) return null;
    const lock = s.features.locks[id];
    if (lock) return { reason: lock, unlock: lock.includes("Labs") ? "labs" : null };
    if (on) return null;
    if (s.features.state.pbp) return { reason: pbpReason, unlock: null };
    if (s.features.state.picture_mode_blocks_ai && id !== "my-display-preset")
      return { reason: `Not available in ${s.features.picture_mode ?? "this picture mode"}. Switch to Personalized Picture, Gamer 1, Gamer 2 or FPS first.`, unlock: null };
    return null;
  }
</script>

<div class="page">
  <!-- Monitor tabs -->
  <div class="tabs">
    {#if !s}
      <div class="skeleton tab-skel"></div>
    {:else}
      {#each s.monitors as m (m.id)}
        <button
          class="mon"
          class:selected={m.selected}
          class:unsupported={!m.supported}
          class:linked={s.linked && m.supported && !m.selected}
          aria-pressed={m.selected}
          disabled={!m.supported}
          title={[m.name, m.port && `Connected to ${m.port}`, m.detail].filter(Boolean).join(" · ")}
          onclick={() => selectMonitor(m.id)}
          in:fly={rise}
        >
          <span class="name">{m.name}</span>
          <!-- The OS's name for it is the title; what it is and where it's plugged in, below. -->
          <span class="eyebrow">{m.supported ? [m.demo ? "Simulated" : s.model, m.port].filter(Boolean).join(" · ") : "Not supported yet"}</span>
        </button>
      {/each}
      {#if s.phase !== "empty"}
        <button class="btn" onclick={rescan} disabled={ui.busy}>{ui.busy ? "Looking…" : "Scan again"}</button>
      {/if}
      {#if supportedCount > 1}
        <div class="linkall" in:fly={rise}>
          <div class="ltext">
            <b>Change all together</b>
            <span class="muted small">{s.linked ? `Every change goes to all ${supportedCount} monitors` : "Changes go to the selected monitor"}</span>
          </div>
          <Toggle checked={s.linked} label="Change all monitors together" onchange={setLinked} />
        </div>
      {/if}
    {/if}
  </div>

  {#if s?.lg_switch_running}
    <div class="banner" in:fly={hint}>
      <span class="eyebrow warn">Warning</span>
      <div><b>LG Switch is running.</b> It talks to your monitor too, so a change can occasionally need a second try. Quit LG Switch for the smoothest experience.</div>
    </div>
  {/if}
  {#if s?.baseline.error}
    <div class="banner">
      <span class="eyebrow err">Error</span>
      <div><b>Changes are paused.</b> nubless couldn't save your original settings ({s.baseline.error}), so it won't change anything until it can.</div>
    </div>
  {/if}
  {#if s?.error}
    <div class="banner"><span class="eyebrow err">Error</span><div>{s.error}</div></div>
  {/if}

  {#if s && s.phase === "empty"}
    <div class="empty" in:fly={rise}>
      <div class="art"><MonitorArt /></div>
      <div class="copy">
        <div class="eyebrow">Display 01</div>
        <h1>Let's find your {s.model}</h1>
        <p class="muted">nubless talks to your monitor over the cable you already use. If it doesn't show up:</p>
        <ol>
          <li><span class="n">01</span><span>Connect it with DisplayPort, HDMI or USB-C, not through a dock or KVM.</span></li>
          <li><span class="n">02</span><span>Make sure <b>DDC/CI</b> is on in the monitor's menu (it's on by default).</span></li>
          <li><span class="n">03</span><span>Give it a few seconds after waking up, then scan again.</span></li>
        </ol>
        <div class="row">
          <button class="btn primary" onclick={rescan} disabled={ui.busy}><Search size={16} />{ui.busy ? "Looking…" : "Scan again"}</button>
          <button class="btn" onclick={useDemo}>Try the demo monitor</button>
        </div>
      </div>
    </div>
  {:else if !s}
    <div class="starting" role="status" in:fly={rise}>
      <div class="hatch wide" aria-hidden="true"></div>
      <h2>Looking for your monitor…</h2>
      <p class="muted">This can take a few seconds when nubless starts, while it checks your displays.</p>
    </div>
    <div class="cols">
      <div class="left">
        {#each [0, 1] as _}<div class="skeleton" style="height:72px"></div>{/each}
        <div class="skeleton" style="height:140px"></div>
      </div>
      <div class="right"><div class="skeleton" style="height:260px"></div></div>
    </div>
  {:else}
    <div class="cols">
      <div class="left">
        <Section title="Picture" {loading} aside={loadingHint}>
          {#each group("picture").filter((c) => c.kind === "range") as c (c.id)}<Slider control={c} fine />{/each}
          {#each group("picture").filter((c) => c.kind === "choice" && c.id !== "ai-upscaling") as c (c.id)}<Segmented control={c} />{/each}
          {#each group("picture").filter((c) => c.kind !== "range" && c.kind !== "choice" && c.id !== "color-temperature") as c (c.id)}
            <div class="inforow"><span>{c.label}</span>{#if c.enabled}<span class="mono small">{c.display}</span>{:else}<Lock control={c} />{/if}</div>
          {/each}
        </Section>

        {#if byId["picture-mode"]}
          {@const pm = byId["picture-mode"]}
          <Section title="Picture mode" {loading}>
            {#snippet aside()}
              {#if !s.demo && !pm.enabled}<Lock control={pm} />{:else}<Hint>AI = works with AI features</Hint>{/if}
            {/snippet}
            <div class="picker"><ModePicker {pm} /></div>
          </Section>
        {/if}
      </div>

      <div class="right">
        <!-- Smart features: the interlocks live here -->
        <Section title="Smart features" hint="Only one can be on at a time">
          {#each smart as f}
            {@const pending = ui.pendingFeature?.id === f.id}
            {@const on = pending ? ui.pendingFeature!.on : s.features.state[f.key]}
            {@const lock = featureLock(f.id, s.features.state[f.key])}
            <div class="feature">
              <div class="ftext"><b>{f.title}</b><span class="muted small">{pending ? (on ? "Turning on…" : "Turning off…") : f.body}</span></div>
              {#if lock}
                <Lock control={lock} />
              {:else}
                <Toggle checked={on} {pending} disabled={!!ui.pendingFeature && !pending} label={f.title} onchange={(v) => toggleFeature(f.id, v)} />
              {/if}
            </div>
          {/each}
          {#if byId["ai-upscaling"]}<div class="upscaling"><Segmented control={byId["ai-upscaling"]} /></div>{/if}
          {#if s.demo}
            <div class="feature">
              <div class="ftext"><b>PBP/PIP <span class="tag solid">Demo</span></b><span class="muted small">Pretend PBP/PIP is on, to see the interlocks.</span></div>
              <Toggle checked={s.features.state.pbp} label="Simulate PBP/PIP" onchange={(v) => simulatePbp(v)} />
            </div>
          {/if}
        </Section>
      </div>
    </div>

    <div class="three">
      <Section title="Gaming" {loading} aside={loadingHint}>
        {#each group("game").filter((c) => c.kind === "choice") as c (c.id)}<Segmented control={c} />{/each}
        {#each group("game").filter((c) => c.kind !== "choice") as c (c.id)}
          <div class="inforow"><span>{c.label}</span>{#if c.enabled}<span class="mono small">{c.display}</span>{:else}<Lock control={c} />{/if}</div>
        {/each}
      </Section>

      <Section title="Sound" {loading}>
        {#if byId["volume"]}<Slider control={byId["volume"]} />{/if}
        {#if aiSound}
          <div class="inforow">
            <span>AI Sound</span>
            {#if aiSound.enabled}
              <Toggle checked={aiSound.value === 2} label="AI Sound" onchange={(v) => setControl("ai-sound", v ? 2 : 1, { before: aiSound.value })} />
            {:else}<Lock control={aiSound} />{/if}
          </div>
        {/if}
        {#if mute}
          <div class="inforow">
            <span>Mute speakers</span>
            {#if mute.enabled}
              <Toggle checked={mute.value === 1} label="Mute" onchange={(v) => setControl("mute", v ? 1 : 2, { before: mute.value })} />
            {:else}<Lock control={mute} />{/if}
          </div>
        {/if}
      </Section>

      <Section title="About this monitor">
        {#each group("info") as c (c.id)}
          <div class="inforow">
            <span>{c.label}</span>
            <span class="mono small info">{c.display ?? "–"}{#if c.reason && c.reason !== "Information from your monitor." && c.kind !== "info"}<Lock control={c} />{/if}</span>
          </div>
        {/each}
      </Section>
    </div>

    {#if !s.labs && labsCount > 0}
      <div class="labs" in:fly={rise}>
        <FlaskConical size={28} strokeWidth={2} />
        <div class="ltext">
          <b>{labsCount} more controls are almost ready</b>
          <span class="muted">They match LG's own names and values but haven't been tested on hardware. Labs lets you try them.</span>
        </div>
        <button class="btn" onclick={() => (ui.page = "settings")}>Open Labs</button>
      </div>
    {/if}
  {/if}
</div>

{#snippet loadingHint()}
  {#if loading}<Hint loading>Loading {ui.switching ?? s?.features.picture_mode ?? "new"} settings…</Hint>{/if}
{/snippet}

<style>
  .page {
    max-width: 1180px;
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    align-items: stretch;
    gap: var(--space-2);
    padding: var(--space-6) 0 var(--space-8);
  }
  .tab-skel {
    width: 200px;
    height: 52px;
  }
  .mon {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 6px 16px;
    border: 2px solid var(--ink);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--ink);
    text-align: left;
    transition: background var(--fast) var(--ease);
  }
  .mon:hover:not(.selected) {
    background: var(--wash);
  }
  .mon.selected {
    background: var(--ink);
    color: var(--ground);
  }
  .mon.selected .eyebrow {
    color: var(--sel-mark);
  }
  .mon.unsupported {
    border-style: dashed;
    cursor: default;
  }
  .mon.linked {
    background: var(--wash);
  }
  .linkall {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-left: auto;
    padding: 6px 0 6px var(--space-4);
    border-left: 1px solid var(--rule);
  }
  .linkall .ltext {
    display: flex;
    flex-direction: column;
  }
  .name {
    font-weight: 800;
    font-size: var(--fs-lg);
  }

  .banner {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 14px;
    margin-bottom: var(--space-4);
    border: 2px solid var(--ink);
    line-height: 1.45;
  }
  .warn {
    color: var(--warn);
  }
  .err {
    color: var(--err);
  }

  .cols {
    display: flex;
    flex-wrap: wrap;
    gap: 44px 56px;
    align-items: flex-start;
  }
  .left {
    flex: 1.4 1 520px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 40px;
  }
  .right {
    flex: 1 1 340px;
    min-width: 0;
  }
  .picker {
    margin-top: var(--space-4);
  }
  .three {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 36px 48px;
    margin-top: 52px;
  }

  .feature {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    min-height: 64px;
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--rule);
  }
  .upscaling {
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--rule);
  }
  .ftext {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .ftext b {
    font-weight: 600;
  }
  .ftext .tag {
    margin-left: 6px;
    vertical-align: 2px;
  }
  .inforow {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-4);
    min-height: 52px;
    border-bottom: 1px solid var(--rule);
    font-weight: 500;
  }
  .info {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .labs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
    margin-top: 44px;
    padding-top: 14px;
    border-top: 2px solid var(--ink);
  }
  .labs :global(svg) {
    flex: none;
  }
  .ltext {
    flex: 1 1 320px;
    display: flex;
    flex-direction: column;
    max-width: 680px;
  }
  .ltext b {
    font-weight: 800;
    font-size: var(--fs-lg);
  }

  .starting {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    margin-bottom: var(--space-8);
  }
  .starting h2 {
    font-size: var(--fs-3xl);
  }
  .wide {
    width: 320px;
    max-width: 100%;
    height: 10px;
  }

  .empty {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 40px 64px;
    padding-top: var(--space-8);
  }
  .art {
    flex: 1 1 340px;
    min-width: 0;
    max-width: 480px;
  }
  .copy {
    flex: 1 1 340px;
    min-width: 0;
    max-width: 520px;
  }
  .copy h1 {
    margin: 6px 0 10px;
    font-size: 40px;
  }
  ol {
    list-style: none;
    margin: var(--space-5) 0 0;
    padding: 0;
    border-bottom: 1px solid var(--rule);
  }
  li {
    display: flex;
    gap: var(--space-4);
    padding: 12px 0;
    border-top: 1px solid var(--rule);
  }
  .n {
    flex: 0 0 28px;
    padding-top: 2px;
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    color: var(--dim);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 22px;
  }
</style>
