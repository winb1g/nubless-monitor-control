<script lang="ts">
  // Small print beside a heading: "Only one can be on at a time". With
  // `loading` it turns ink-dark and drops in, for "Loading Gamer 2 settings…".
  import type { Snippet } from "svelte";
  import { fly } from "svelte/transition";
  import { hint } from "../motion";

  let { loading = false, children }: { loading?: boolean; children: Snippet } = $props();
</script>

<span class="hint eyebrow" class:loading in:fly={loading ? hint : { duration: 0 }}>{@render children()}</span>

<style>
  .loading {
    color: var(--ink);
  }
</style>
