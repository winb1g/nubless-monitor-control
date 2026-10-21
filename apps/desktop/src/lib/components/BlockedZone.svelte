<script lang="ts">
  // The stretch of a bar that can't be used: frozen slanted ticks (.blocked in
  // app.css) with a padlock on a plate, so the ticks never run through the icon.
  // Sits inside a positioned, outlined bar and fills it from the left up to `pct`.
  // `compact` (the tray's thin bar) keeps the ticks and drops the padlock.
  import { Lock } from "@lucide/svelte";

  let { pct, compact = false }: { pct: number; compact?: boolean } = $props();
</script>

<div class="blocked zone" style:width="{pct}%" aria-hidden="true">
  {#if !compact}<span class="plate"><Lock size={12} strokeWidth={2.5} /></span>{/if}
</div>

<style>
  .zone {
    position: absolute;
    inset: 0 auto 0 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-right: 2px solid var(--ink);
  }
  .plate {
    display: flex;
    align-items: center;
    height: 100%;
    padding: 0 var(--space-1);
    background: var(--ground);
    border-inline: 2px solid var(--ink);
  }
</style>
