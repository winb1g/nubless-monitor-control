<script lang="ts">
  // Small print beside a heading: "Only one can be on at a time". With
  // `loading` it turns into the accent "Loading…" hint and drops in.
  import type { Snippet } from "svelte";
  import { fly } from "svelte/transition";
  import { hint } from "../motion";

  let { loading = false, children }: { loading?: boolean; children: Snippet } = $props();
</script>

<span class="hint" class:loading in:fly={loading ? hint : { duration: 0 }}>{@render children()}</span>

<style>
  .hint {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }
  .loading {
    color: var(--accent-ink);
    font-weight: 600;
  }
</style>
