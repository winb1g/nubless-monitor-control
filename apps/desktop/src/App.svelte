<script lang="ts">
  import { onMount } from "svelte";
  import { init, toast } from "./lib/store.svelte";
  import { applyTheme } from "./lib/theme.svelte";
  import Shell from "./lib/Shell.svelte";
  import Tray from "./lib/pages/Tray.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import DialogHost from "./lib/components/DialogHost.svelte";

  const isTray = location.hash.startsWith("#/tray");
  // The design kit (every component, every state) is for `npm run dev` only.
  const isKit = import.meta.env.DEV && location.hash.startsWith("#/kit");
  if (isTray) document.body.classList.add("tray");
  applyTheme();

  onMount(async () => {
    try {
      await init();
    } catch (e) {
      toast(`nubless couldn't start talking to your monitor: ${e}`, "error");
    }
  });
</script>

{#if isKit}
  {#await import("./lib/pages/Kit.svelte") then kit}<kit.default />{/await}
{:else if isTray}
  <Tray />
{:else}
  <Shell />
{/if}
<Toasts compact={isTray} />
<DialogHost />
