<script lang="ts">
  // A settings row: title and description on the left, its control (the
  // children) on the right, and optional `below` content on its own line.
  //
  //   <Item title="Start at login" body="Open nubless quietly…">
  //     <Toggle … />
  //   </Item>
  import type { Snippet } from "svelte";
  import { fly } from "svelte/transition";
  import { rise } from "../motion";

  let {
    title,
    body,
    mark,
    lead = false,
    children,
    below,
  }: {
    title: string;
    /** Description under the title. A plain string, or a snippet when it needs markup. */
    body?: string | Snippet;
    /** A small glyph after the title: the Labs flask. */
    mark?: Snippet;
    /** The first row of a page gets the heavy rule. */
    lead?: boolean;
    /** The control: a Toggle, a button, a Segmented. */
    children?: Snippet;
    /** Full-width content on its own line under the row. */
    below?: Snippet;
  } = $props();
</script>

<div class="item" class:lead in:fly={rise}>
  <div class="text">
    <h2>{title}{#if mark}<span class="mark">{@render mark()}</span>{/if}</h2>
    {#if typeof body === "string"}<p class="muted">{body}</p>{:else if body}<p class="muted">{@render body()}</p>{/if}
  </div>
  {#if children}<div class="control">{@render children()}</div>{/if}
  {#if below}<div class="below">{@render below()}</div>{/if}
</div>

<style>
  .item {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3) var(--space-8);
    padding: 18px 0;
    border-top: 1px solid var(--rule);
  }
  .item:last-child {
    border-bottom: 1px solid var(--rule);
  }
  .lead {
    border-top: 2px solid var(--ink);
  }
  .text {
    flex: 1 1 380px;
    min-width: 0;
  }
  h2 {
    font-size: var(--fs-xl);
  }
  .mark {
    display: inline-flex;
    vertical-align: -3px;
    margin-left: 6px;
  }
  p {
    margin-top: var(--space-1);
    max-width: 560px;
  }
  .control {
    flex: none;
  }
  .below {
    flex: 1 1 100%;
  }
</style>
