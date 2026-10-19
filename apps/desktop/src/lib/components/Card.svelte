<script lang="ts">
  // The standard card: a panel that rises into place with its siblings, with
  // an optional title row (title on the left, a hint or `aside` on the right).
  //
  //   <Card title="Sound" hint="Changes apply right away" i={2}>…</Card>
  //   <Card row i={1}><IconBadge icon={Palette} /><div>…</div><Toggle … /></Card>
  import type { Snippet } from "svelte";
  import { fly } from "svelte/transition";
  import { rise } from "../motion";
  import Hint from "./Hint.svelte";

  let {
    title,
    hint,
    aside,
    i = 0,
    row = false,
    tint = false,
    loading = false,
    children,
  }: {
    title?: string;
    /** Plain small print at the right of the title row. */
    hint?: string;
    /** Anything else for the right of the title row: a Lock, a loading Hint. */
    aside?: Snippet;
    /** Position among the cards arriving together; staggers the rise. */
    i?: number;
    /** Lay the children out in a row: icon, text, control. */
    row?: boolean;
    /** A soft accent wash, for the one card on a page that leads. */
    tint?: boolean;
    /** Values are being re-read: dim the body and run a sheen across it. */
    loading?: boolean;
    children: Snippet;
  } = $props();
</script>

<section class="card" class:row class:tint class:loading in:fly={rise(i)}>
  {#if title || hint || aside}
    <div class="title">
      {#if title}<h2>{title}</h2>{/if}
      {#if aside}{@render aside()}{:else if hint}<Hint>{hint}</Hint>{/if}
    </div>
  {/if}
  {@render children()}
</section>

<style>
  .title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }
  .tint {
    background: linear-gradient(135deg, var(--accent-soft), transparent 65%), var(--panel);
  }

  .loading {
    position: relative;
    overflow: hidden;
  }
  .loading > :global(:not(.title)) {
    pointer-events: none;
    opacity: 0.55;
    transition: opacity var(--base) var(--ease);
  }
  .loading::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(100deg, transparent 30%, color-mix(in srgb, var(--accent) 10%, transparent) 50%, transparent 70%);
    background-size: 220% 100%;
    animation: sheen 1.1s var(--ease) infinite;
  }
  @keyframes sheen {
    from {
      background-position: 120% 0;
    }
    to {
      background-position: -100% 0;
    }
  }
</style>
