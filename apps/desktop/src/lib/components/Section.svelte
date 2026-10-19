<script lang="ts">
  // A titled block of a page: a 2px ink rule, the title (and a hint or `aside`
  // on the right), then the content. Replaces the old card; there is no fill,
  // border or shadow around the content.
  //
  //   <Section title="Sound" hint="Changes apply right away">…</Section>
  import type { Snippet } from "svelte";
  import { fly } from "svelte/transition";
  import { rise } from "../motion";
  import Hint from "./Hint.svelte";

  let {
    title,
    hint,
    aside,
    loading = false,
    children,
  }: {
    title: string;
    /** Plain small print at the right of the title row. */
    hint?: string;
    /** Anything else for the right of the title row: a Lock, a loading Hint. */
    aside?: Snippet;
    /** Values are being re-read: dim the body and run the busy bar under the title. */
    loading?: boolean;
    children: Snippet;
  } = $props();
</script>

<section class="section" class:loading in:fly={rise}>
  <div class="title">
    <h2>{title}</h2>
    {#if aside}{@render aside()}{:else if hint}<Hint>{hint}</Hint>{/if}
  </div>
  {#if loading}<div class="hatch busy" aria-hidden="true"></div>{/if}
  <div class="body">{@render children()}</div>
</section>

<style>
  .section {
    min-width: 0;
  }
  .title {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--space-1) var(--space-3);
    padding-top: 10px;
    border-top: 2px solid var(--ink);
  }
  .busy {
    margin-top: var(--space-2);
  }
  .body {
    transition: opacity var(--base) var(--ease);
  }
  .loading .body {
    pointer-events: none;
    opacity: 0.5;
  }
</style>
