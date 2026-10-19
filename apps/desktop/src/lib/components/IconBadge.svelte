<script lang="ts">
  // An icon on a rounded tile: feature rows, picture-mode tiles, settings
  // cards, update status. Its corner radius and glow scale with `size`.
  import type { Snippet } from "svelte";
  import type { LucideIcon } from "@lucide/svelte";

  /**
   * soft: quiet panel tile (default) · raised: lifted off a panel-2 surface ·
   * tint: accent wash · brand: the gradient, for "on" and "selected" ·
   * neutral: grey, for states with nothing to say · mint / coral: good / bad news.
   */
  type Tone = "soft" | "raised" | "tint" | "brand" | "neutral" | "mint" | "coral";

  let {
    icon: Icon,
    size = 36,
    iconSize = 18,
    tone = "soft",
    round = false,
    glow = false,
    busy = false,
    spin = false,
    children,
  }: {
    icon: LucideIcon;
    size?: number;
    iconSize?: number;
    tone?: Tone;
    round?: boolean;
    /** A colored glow underneath; pair it with tone="brand". */
    glow?: boolean;
    /** Waiting on the monitor: the shared busy ring spins around the badge. */
    busy?: boolean;
    /** Spin the icon itself, for a refresh in progress. */
    spin?: boolean;
    /** Extras pinned to the badge, such as a corner marker. */
    children?: Snippet;
  } = $props();
</script>

<span
  class="badge {tone}"
  class:round
  class:glow
  class:spin
  class:busy-ring={busy}
  style:--size="{size}px"
  style:--badge-glow={size >= 56 ? "var(--glow-lg)" : "var(--glow-sm)"}
>
  <Icon size={iconSize} />
  {@render children?.()}
</span>

<style>
  .badge {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
    width: var(--size);
    height: var(--size);
    border-radius: calc(var(--size) * 0.32);
    color: var(--accent);
    --ring-inset: calc(var(--size) * -0.125);
    transition: background var(--base) var(--ease), color var(--base), box-shadow var(--base);
  }
  .soft {
    background: var(--panel-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .raised {
    background: var(--panel);
    box-shadow: var(--shadow-sm), inset 0 0 0 1px var(--line);
  }
  .tint {
    background: var(--accent-soft);
  }
  .brand {
    background: var(--grad);
    color: var(--on-accent);
  }
  .neutral {
    background: var(--panel-2);
    color: var(--text-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .mint {
    background: var(--mint-soft);
    color: var(--mint);
  }
  .coral {
    background: var(--coral-soft);
    color: var(--coral);
  }
  .round {
    border-radius: 50%;
  }
  .glow {
    box-shadow: var(--badge-glow);
  }
  .spin > :global(svg:first-child) {
    animation: spin var(--spin) linear infinite;
  }
</style>
