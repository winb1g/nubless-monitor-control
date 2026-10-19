<script lang="ts">
  import { rise, hint } from "../motion";
  import Card from "../components/Card.svelte";
  import IconBadge from "../components/IconBadge.svelte";
  import Hint from "../components/Hint.svelte";
  import { fly } from "svelte/transition";
  import {
    Sparkles, WandSparkles, AppWindow, PictureInPicture2, RefreshCw, TriangleAlert, FlaskConical,
    User, Gamepad2, Crosshair, Ruler, Eye, Clapperboard, Image, Search, MonitorSmartphone,
  } from "@lucide/svelte";
  import { ui, toggleFeature, simulatePbp, rescan, useDemo, greeting, setControl, pickPictureMode } from "../store.svelte";
  import type { ControlView, Feature, FeatureState } from "../types";
  import MonitorArt from "../components/MonitorArt.svelte";
  import Slider from "../components/Slider.svelte";
  import Segmented from "../components/Segmented.svelte";
  import Toggle from "../components/Toggle.svelte";
  import Lock from "../components/Lock.svelte";

  const s = $derived(ui.state);
  const byId = $derived(Object.fromEntries((s?.controls ?? []).map((c) => [c.id, c])) as Record<string, ControlView>);
  const group = (g: ControlView["group"]) => (s?.controls ?? []).filter((c) => c.group === g);

  const smart: { id: Feature; key: keyof FeatureState; title: string; body: string; icon: typeof Sparkles }[] = [
    { id: "ai-scene-optimization", key: "ai_scene_optimization", title: "AI Scene Optimization", body: "Your monitor tunes the picture to what's on screen.", icon: Sparkles },
    { id: "ai-picture-mode", key: "ai_picture_mode", title: "AI Picture Mode", body: "Picks a picture mode for what you're doing.", icon: WandSparkles },
    { id: "my-display-preset", key: "my_display_preset", title: "My Display Preset", body: "Switches picture mode per app.", icon: AppWindow },
  ];

  const tileIcons: Record<string, typeof User> = {
    "Personalized Picture": User, "Gamer 1": Gamepad2, "Gamer 2": Gamepad2, FPS: Crosshair,
    Calibration1: Ruler, Calibration2: Ruler, "Color Weakness": Eye, "DCI-P3": Clapperboard,
  };

  const pbpReason = "Not available while PBP/PIP is on. Turn PBP/PIP off first.";
  const mute = $derived(byId["mute"]);
  const aiSound = $derived(byId["ai-sound"]);
  /** A picture-mode switch is in flight or its values are being re-read. */
  const loading = $derived(!!ui.switching || !!s?.refreshing);

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
  <header>
    <div>
      <p class="eyebrow">{greeting()}</p>
      <h1>Your displays</h1>
    </div>
    {#if s?.phase === "connected"}
      <div class="status" class:demo={s.demo}>
        <span class="dot"></span>{s.demo ? "Demo monitor" : "Connected"}
      </div>
    {/if}
  </header>

  <!-- Monitor strip -->
  <div class="strip">
    {#if !s}
      {#each [0, 1] as _}<div class="mon skeleton" style="height:178px"></div>{/each}
    {:else}
      {#each s.monitors as m (m.id)}
        <button class="mon" class:selected={m.selected} class:unsupported={!m.supported} in:fly={rise()}>
          <MonitorArt size={132} dim={!m.supported} />
          <span class="name">{m.name}</span>
          <span class="tag" class:ok={m.supported}>{m.demo ? "Simulated" : m.supported ? "Supported" : "Not supported yet"}</span>
        </button>
      {/each}
      <button class="mon add" onclick={rescan} disabled={ui.busy}>
        <IconBadge icon={RefreshCw} size={54} iconSize={22} tone="tint" round spin={ui.busy} />
        <span class="name">{ui.busy ? "Looking…" : "Scan again"}</span>
        <span class="tag">Find connected monitors</span>
      </button>
    {/if}
  </div>

  {#if s?.lg_switch_running}
    <div class="banner warn" in:fly={hint}>
      <TriangleAlert size={18} />
      <div><b>LG Switch is running.</b> It talks to your monitor too, so a change can occasionally need a second try. Quit LG Switch for the smoothest experience.</div>
    </div>
  {/if}
  {#if s?.baseline.error}
    <div class="banner err"><TriangleAlert size={18} /><div><b>Changes are paused.</b> nubless couldn't save your original settings ({s.baseline.error}), so it won't change anything until it can.</div></div>
  {/if}
  {#if s?.error}
    <div class="banner err"><TriangleAlert size={18} /><div>{s.error}</div></div>
  {/if}

  {#if s && s.phase === "empty"}
    <div class="empty card" in:fly={rise()}>
      <div class="float"><MonitorArt size={170} /></div>
      <h2>Let's find your {s.model}</h2>
      <p class="muted">nubless talks to your monitor over the cable you already use. If it doesn't show up:</p>
      <ol>
        <li>Connect it with DisplayPort, HDMI or USB-C, not through a dock or KVM.</li>
        <li>Make sure <b>DDC/CI</b> is on in the monitor's menu (it's on by default).</li>
        <li>Give it a few seconds after waking up, then scan again.</li>
      </ol>
      <div class="row">
        <button class="btn primary" onclick={rescan} disabled={ui.busy}><Search size={16} />{ui.busy ? "Looking…" : "Scan again"}</button>
        <button class="btn" onclick={useDemo}><MonitorSmartphone size={16} />Try the demo monitor</button>
      </div>
    </div>
  {:else if !s}
    <div class="grid">
      {#each [0, 1, 2, 3] as _}
        <div class="card"><div class="skeleton" style="height:18px;width:40%"></div><div class="skeleton" style="height:12px;width:70%;margin-top:14px"></div><div class="skeleton" style="height:34px;margin-top:18px"></div><div class="skeleton" style="height:34px;margin-top:12px"></div></div>
      {/each}
    </div>
  {:else}
    <div class="grid">
      <!-- Smart features: the interlocks live here -->
      <Card title="Smart features" hint="Only one can be on at a time">
        {#each smart as f}
          {@const pending = ui.pendingFeature?.id === f.id}
          {@const on = pending ? ui.pendingFeature!.on : s.features.state[f.key]}
          {@const lock = featureLock(f.id, s.features.state[f.key])}
          <div class="feature" class:on>
            <IconBadge icon={f.icon} tone={on ? "brand" : "soft"} />
            <div class="ftext"><b>{f.title}</b><span class="muted small">{pending ? (on ? "Turning on…" : "Turning off…") : f.body}</span></div>
            {#if lock}
              <Lock control={lock} />
            {:else}
              <Toggle checked={on} {pending} disabled={!!ui.pendingFeature && !pending} label={f.title} onchange={(v) => toggleFeature(f.id, v)} />
            {/if}
          </div>
        {/each}
        {#if s.demo}
          <div class="feature demo-row">
            <IconBadge icon={PictureInPicture2} />
            <div class="ftext"><b>PBP/PIP <span class="badge">Demo</span></b><span class="muted small">Pretend PBP/PIP is on, to see the interlocks.</span></div>
            <Toggle checked={s.features.state.pbp} label="Simulate PBP/PIP" onchange={(v) => simulatePbp(v)} />
          </div>
        {/if}
      </Card>

      <!-- Picture mode -->
      {#if byId["picture-mode"]}
        {@const pm = byId["picture-mode"]}
        <Card title="Picture mode" i={1}>
          {#snippet aside()}
            {#if !s.demo && !pm.enabled}<Lock control={pm} />{:else}<Hint><Sparkles size={12} /> works with AI features</Hint>{/if}
          {/snippet}
          <div class="tiles">
            {#each pm.choices as c (c.raw)}
              {@const Icon = tileIcons[c.label] ?? Image}
              {@const current = ui.switching ? ui.switching === c.label : (s.features.picture_mode ?? null) === c.label || pm.value === c.raw}
              <button class="tile" class:active={current} class:pending={ui.switching === c.label} disabled={!s.demo && !pm.enabled} onclick={() => !current && pickPictureMode(c.label)}>
                <IconBadge icon={Icon} size={40} iconSize={20} tone={current ? "brand" : "raised"} busy={ui.switching === c.label} />
                <span>{c.label}</span>
                {#if s.features.ai_capable_modes.includes(c.label)}<span class="ai" title="Works with AI Scene Optimization"><Sparkles size={11} /></span>{/if}
              </button>
            {/each}
          </div>
        </Card>
      {/if}

      <Card title="Picture" i={2} {loading} aside={loadingHint}>
        {#each group("picture").filter((c) => c.kind === "range") as c (c.id)}<Slider control={c} />{/each}
        {#each group("picture").filter((c) => c.kind !== "range") as c (c.id)}
          <div class="inforow"><span>{c.label}</span>{#if c.enabled}<span>{c.display}</span>{:else}<Lock control={c} />{/if}</div>
        {/each}
      </Card>

      <Card title="Gaming" i={3} {loading} aside={loadingHint}>
        {#each group("game").filter((c) => c.kind === "choice") as c (c.id)}<Segmented control={c} />{/each}
        {#each group("game").filter((c) => c.kind !== "choice") as c (c.id)}
          <div class="inforow"><span>{c.label}</span>{#if c.enabled}<span>{c.display}</span>{:else}<Lock control={c} />{/if}</div>
        {/each}
      </Card>

      <Card title="Sound" i={4} {loading}>
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
      </Card>

      <Card title="About this monitor" i={5}>
        {#each group("info") as c (c.id)}
          <div class="inforow">
            <span>{c.label}</span>
            <span class="faint">{c.display ?? "–"}{#if c.reason && c.reason !== "Information from your monitor." && c.kind !== "info"}&nbsp;<Lock control={c} />{/if}</span>
          </div>
        {/each}
      </Card>

      {#if !s.labs && s.controls.some((c) => c.unlock === "labs")}
        <Card row tint i={6}>
          <IconBadge icon={FlaskConical} />
          <div class="ftext">
            <b>{s.controls.filter((c) => c.unlock === "labs").length} more controls are almost ready</b>
            <span class="muted small">They match LG's own names and values but haven't been tested on hardware. Labs lets you try them.</span>
          </div>
          <button class="btn" onclick={() => (ui.page = "settings")}>Open Labs</button>
        </Card>
      {/if}
    </div>
  {/if}
</div>

{#snippet loadingHint()}
  {#if loading}<Hint loading>Loading {ui.switching ?? s?.features.picture_mode ?? "new"} settings…</Hint>{/if}
{/snippet}

<style>
  .page {
    max-width: 1180px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-bottom: 22px;
  }
  .eyebrow {
    color: var(--text-3);
    font-weight: 550;
    margin-bottom: 2px;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 600;
    font-size: var(--fs-md);
    color: var(--mint);
    background: var(--mint-soft);
    padding: 6px var(--space-3);
    border-radius: var(--r-full);
  }
  .status.demo {
    color: var(--accent-ink);
    background: var(--accent-soft);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
    animation: pulse 2s infinite;
  }

  .strip {
    display: flex;
    gap: var(--space-4);
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    padding: var(--space-1) 2px 18px;
    margin-bottom: var(--space-2);
  }
  .mon {
    scroll-snap-align: start;
    flex: none;
    width: 210px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: var(--space-5) 14px var(--space-4);
    border-radius: var(--r-2xl);
    border: 1px solid var(--line);
    background: var(--panel);
    box-shadow: var(--shadow);
    transition: transform var(--base) var(--spring), box-shadow var(--base);
  }
  .mon:hover {
    transform: translateY(-3px);
  }
  .mon.selected {
    border-color: transparent;
    box-shadow: 0 0 0 2px var(--accent), var(--shadow);
  }
  .mon .name {
    font-weight: 600;
    margin-top: var(--space-2);
  }
  .tag {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }
  .tag.ok {
    color: var(--mint);
    font-weight: 600;
  }
  .mon.add {
    justify-content: center;
    border-style: dashed;
    border-color: var(--line-strong);
    background: transparent;
    box-shadow: none;
    color: var(--text-2);
  }

  .banner {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    padding: var(--space-3) var(--space-4);
    border-radius: var(--r-xl);
    margin-bottom: var(--space-4);
    line-height: 1.45;
  }
  .banner.warn {
    background: var(--sun-soft);
    color: color-mix(in srgb, var(--sun) 70%, var(--text));
  }
  .banner.err {
    background: var(--coral-soft);
    color: color-mix(in srgb, var(--coral) 70%, var(--text));
  }
  .banner :global(svg) {
    flex: none;
    margin-top: 1px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: var(--space-4);
    align-items: start;
  }

  .feature {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) 10px;
    margin: 0 -10px;
    border-radius: var(--r-lg);
    transition: background var(--base) var(--ease);
  }
  .feature.on {
    background: var(--accent-soft);
  }
  .ftext {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .ftext b {
    font-weight: 600;
  }
  .demo-row {
    border-top: 1px dashed var(--line-strong);
    border-radius: 0 0 var(--r-lg) var(--r-lg);
    margin-top: 6px;
  }
  .badge {
    font-size: var(--fs-2xs);
    font-weight: 700;
    letter-spacing: 0.02em;
    padding: 1px 7px;
    margin-left: 6px;
    border-radius: var(--r-full);
    background: var(--accent-soft);
    color: var(--accent-ink);
    vertical-align: 1px;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
    margin-top: 6px;
  }
  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: 14px 6px var(--space-3);
    border-radius: var(--r-xl);
    border: 1px solid var(--line);
    background: var(--panel-2);
    font-size: var(--fs-xs);
    font-weight: 550;
    color: var(--text-2);
    text-align: center;
    line-height: 1.25;
    transition: transform var(--fast) var(--spring), box-shadow var(--fast);
  }
  .tile:hover:not(:disabled) {
    transform: translateY(-2px);
  }
  .tile:active:not(:disabled) {
    transform: scale(0.96);
  }
  .tile.active {
    color: var(--text);
    box-shadow: 0 0 0 2px var(--accent);
  }
  .tile:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .ai {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    color: var(--accent);
    opacity: 0.8;
  }

  .inforow {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 11px 0;
    border-top: 1px solid var(--line);
    font-weight: 500;
  }

  .empty {
    max-width: 620px;
    margin: 10px auto 0;
    text-align: center;
    padding: 36px var(--space-10);
  }
  .empty .float {
    display: flex;
    justify-content: center;
    margin-bottom: 18px;
    animation: float 4s ease-in-out infinite;
  }
  @keyframes float {
    50% {
      transform: translateY(-6px);
    }
  }
  .empty h2 {
    font-size: var(--fs-2xl);
    margin-bottom: 6px;
  }
  .empty ol {
    text-align: left;
    margin: 18px auto var(--space-6);
    max-width: 440px;
    padding-left: var(--space-5);
    color: var(--text-2);
    line-height: 1.7;
  }
  .row {
    display: flex;
    gap: 10px;
    justify-content: center;
  }
</style>
