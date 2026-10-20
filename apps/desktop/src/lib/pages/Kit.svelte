<script lang="ts">
  // The design kit: every token and component in every state, on one page.
  // Dev only (`npm run dev`, then open /#/kit); it is not part of the built app.
  // Use it to check a change in light and dark, and with reduced motion on.
  import { Maximize2 } from "@lucide/svelte";
  import { ui, toast, ask, askToKeep } from "../store.svelte";
  import { theme, setTheme } from "../theme.svelte";
  import { upd, type Phase } from "../updates.svelte";
  import type { ControlView } from "../types";
  import Logo from "../components/Logo.svelte";
  import Section from "../components/Section.svelte";
  import Item from "../components/Item.svelte";
  import Hint from "../components/Hint.svelte";
  import Lock from "../components/Lock.svelte";
  import Toggle from "../components/Toggle.svelte";
  import Slider from "../components/Slider.svelte";
  import Segmented from "../components/Segmented.svelte";
  import ModePicker from "../components/ModePicker.svelte";
  import Updates from "./Updates.svelte";

  const s = $derived(ui.state);
  const ctl = (id: string) => s?.controls.find((c) => c.id === id);
  const locked: ControlView = {
    id: "kit-locked", label: "Volume", group: "sound", kind: "range", value: 20, display: "20", min: 0, max: 100, choices: [],
    enabled: false, reason: "Looks right, but hasn't been tested on a real monitor yet.", unlock: "labs", confirm: false,
  };
  const lockedSeg: ControlView = {
    id: "kit-seg", label: "Adaptive-Sync/VRR", group: "game", kind: "choice", value: 3, display: "Extended",
    min: 0, max: 0, choices: [{ raw: 0, label: "Off" }, { raw: 2, label: "Basic" }, { raw: 3, label: "Extended" }],
    enabled: false, reason: "Looks right, but hasn't been tested on a real monitor yet.", unlock: "labs", confirm: false,
  };
  const colors = ["ground", "wash", "ink", "dim", "rule", "signal", "ok", "warn", "err"];
  const sizes = ["2xs", "xs", "sm", "md", "base", "lg", "xl", "2xl", "3xl", "4xl"];
  const phases: Phase[] = ["checking", "current", "unpublished", "available", "downloading", "installing", "restarting", "error"];

  let sw = $state([true, false]);
  let loading = $state(false);
  let phase = $state<Phase>("available");
  $effect(() => {
    upd.phase = phase;
    upd.version = "0.2.0";
    upd.progress = 34;
    upd.error = "Couldn't reach GitHub. Check your connection and try again.";
  });
</script>

<div class="kit">
  <header>
    <Logo size={24} />
    <span class="eyebrow">Design kit · dev only</span>
    <div class="seg" role="group" aria-label="Theme">
      {#each ["system", "light", "dark"] as t}
        <button class:active={theme.value === t} aria-pressed={theme.value === t} onclick={() => setTheme(t as "system" | "light" | "dark")}>{t}</button>
      {/each}
    </div>
  </header>

  <div class="body">
    <h1>Design kit</h1>

    <Section title="Color">
      <div class="swatches">
        {#each colors as c}
          <div class="sw"><span class="chip" style:background="var(--{c})"></span><span class="mono small">--{c}</span></div>
        {/each}
      </div>
      <p class="muted small note">Signal is 1.03:1 on the light ground: a fill inside an ink outline, never text there.</p>
    </Section>

    <Section title="Type">
      <div class="types">
        {#each sizes as z}<div style:font-size="var(--fs-{z})" style:font-weight={z === "4xl" || z === "2xl" || z === "xl" ? 800 : 400}>--fs-{z} · Never touch the joystick nub</div>{/each}
        <div class="mono" style:font-size="var(--fs-3xl)">0123456789 · 80 / 100</div>
        <div class="eyebrow">Eyebrow · Picture mode · Gaming</div>
      </div>
    </Section>

    <Section title="Logo">
      <div class="row gap">
        <Logo size={16} /><Logo size={24} /><Logo size={36} /><Logo size={56} />
      </div>
    </Section>

    <Section title="Buttons, links, tags, LED">
      <div class="row gap">
        <button class="btn">Secondary</button>
        <button class="btn primary">Primary</button>
        <button class="btn signal">Update and restart</button>
        <button class="btn" disabled>Disabled</button>
        <button class="link">Read the safety contract</button>
        <button class="icon" aria-label="Open"><Maximize2 size={18} /></button>
      </div>
      <div class="row gap">
        <span class="tag">AI</span><span class="tag solid">Demo</span><span class="tag dashed">Dashed</span>
        <Lock control={{ reason: "Looks right, but hasn't been tested on a real monitor yet.", unlock: "labs" }} />
        <Lock control={{ reason: "Changing this could disconnect nubless.", unlock: null }} />
        <span class="eyebrow led-row"><i class="led blink"></i>Simulated</span>
        <span class="eyebrow led-row"><i class="led"></i>Connected</span>
        <span class="eyebrow led-row"><i class="led off"></i>No monitor</span>
      </div>
      <div class="hatch" style="max-width:320px;margin-top:16px"></div>
    </Section>

    <Section title="Switch">
      <div class="row gap">
        <Toggle checked={sw[0]} label="On" onchange={(v) => (sw[0] = v)} />
        <Toggle checked={sw[1]} label="Off" onchange={(v) => (sw[1] = v)} />
        <Toggle checked={false} pending label="Pending" onchange={() => {}} />
        <Toggle checked={false} disabled label="Disabled" onchange={() => {}} />
      </div>
    </Section>

    {#if s}
      <Section title="Sliders" {loading}>
        <div class="two">
          <div>
            {#if ctl("brightness")}<Slider control={ctl("brightness")!} fine />{/if}
            {#if ctl("contrast")}<Slider control={ctl("contrast")!} compact />{/if}
          </div>
          <div><Slider control={locked} /></div>
        </div>
        <button class="btn" onclick={() => (loading = !loading)} style="margin-top:16px">{loading ? "Stop loading" : "Show loading state"}</button>
      </Section>

      <Section title="Segmented">
        <div class="two">
          <div>{#if ctl("response-time")}<Segmented control={ctl("response-time")!} />{/if}</div>
          <div><Segmented control={lockedSeg} /></div>
        </div>
      </Section>

      {#if ctl("picture-mode")}
        <Section title="Picture mode">
          <div style="margin-top:16px"><ModePicker pm={ctl("picture-mode")!} /></div>
          <div class="stepper-box"><ModePicker pm={ctl("picture-mode")!} layout="stepper" /></div>
        </Section>
      {/if}
    {/if}

    <Section title="Section aside and hint" hint="Only one can be on at a time">
      <div class="row gap"><Hint>Plain hint</Hint><Hint loading>Loading Gamer 2 settings…</Hint></div>
    </Section>

    <div class="rows">
      <Item lead title="Settings row" body="Title and description on the left, the control on the right.">
        <Toggle checked label="Example" onchange={() => {}} />
      </Item>
      <Item title="Another row" body="Rows separate with a hairline rule."><button class="btn">Action</button></Item>
    </div>

    <Section title="Banners, toasts, dialog">
      <div class="banner"><span class="eyebrow warn">Warning</span><div><b>LG Switch is running.</b> It talks to your monitor too.</div></div>
      <div class="banner"><span class="eyebrow err">Error</span><div><b>Changes are paused.</b> nubless couldn't save your original settings.</div></div>
      <div class="row gap">
        <button class="btn" onclick={() => toast("Brightness updated", "success", () => {})}>Toast with Undo</button>
        <button class="btn" onclick={() => askToKeep("black-stabilizer", "Black Stabilizer", "70", 50)}>Keep this change?</button>
        <button class="btn" onclick={() => toast("Another smart feature is on.", "warn")}>Notice</button>
        <button class="btn" onclick={() => toast("Couldn't reach the monitor.", "error")}>Error</button>
        <button class="btn" onclick={() => ask({ title: "Turn off AI Scene Optimization?", body: "Turning on AI Picture Mode will turn off AI Scene Optimization. Continue?", confirm: "Continue", cancel: "Cancel" })}>Dialog</button>
      </div>
    </Section>

    <Section title="Updates page">
      <div class="row gap">
        {#each phases as p}<button class="btn" class:primary={phase === p} onclick={() => (phase = p)}>{p}</button>{/each}
      </div>
      <div class="frame"><Updates /></div>
    </Section>
  </div>
</div>

<style>
  .kit {
    height: 100%;
    overflow-y: auto;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3) var(--space-6);
    padding: var(--space-2) var(--space-10);
    border-bottom: 1px solid var(--rule);
    position: sticky;
    top: 0;
    background: var(--ground);
    z-index: 5;
  }
  .body {
    max-width: 1000px;
    padding: 0 var(--space-10) 80px;
    display: flex;
    flex-direction: column;
    gap: 40px;
  }
  .body h1 {
    padding-top: var(--space-8);
  }
  .seg {
    display: inline-flex;
    border: 2px solid var(--ink);
  }
  .seg button {
    min-height: 40px;
    padding: 0 14px;
    border: none;
    background: transparent;
    color: var(--ink);
    font-weight: 600;
    text-transform: capitalize;
  }
  .seg button.active {
    background: var(--ink);
    color: var(--ground);
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
    margin-top: var(--space-4);
  }
  .sw {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .chip {
    width: 92px;
    height: 44px;
    border: 1px solid var(--ink);
  }
  .note {
    margin-top: var(--space-3);
  }
  .types {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: var(--space-4);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    margin-top: var(--space-4);
  }
  .gap {
    gap: var(--space-3) var(--space-4);
  }
  .icon {
    display: grid;
    place-items: center;
    width: var(--hit);
    height: var(--hit);
    border: none;
    background: none;
    color: var(--ink);
  }
  .led-row {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .two {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: var(--space-8);
    margin-top: var(--space-4);
  }
  .stepper-box {
    max-width: 308px;
    margin-top: var(--space-6);
  }
  .banner {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 14px;
    margin-top: var(--space-3);
    border: 2px solid var(--ink);
  }
  .warn {
    color: var(--warn);
  }
  .err {
    color: var(--err);
  }
  .frame {
    margin-top: var(--space-4);
    border: 1px dashed var(--rule);
    overflow: hidden;
  }
</style>
