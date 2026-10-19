<script lang="ts">
  // The wordmark: "nubless" in Archivo 800 with a lime cursor block after it.
  // Everything scales from `size` (the type size in px). The block is a signal
  // fill inside an ink outline, so it holds on both grounds.
  let { size = 24 }: { size?: number } = $props();
</script>

<span class="logo" role="img" aria-label="nubless" style:--s="{size}px">
  <span class="word" aria-hidden="true">nubless</span>
  <span class="block" aria-hidden="true"></span>
</span>

<style>
  .logo {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--s) * 0.12);
    line-height: 1;
    color: var(--ink);
  }
  .word {
    font-weight: 800;
    font-size: calc(var(--s) * 1.05);
    letter-spacing: -0.03em;
  }
  .block {
    display: block;
    width: calc(var(--s) * 0.42);
    height: calc(var(--s) * 0.68);
    background: var(--signal);
    border: 2px solid currentColor;
  }
</style>
