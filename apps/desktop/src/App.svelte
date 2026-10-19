<script lang="ts">
  import { onMount } from "svelte";
  import { init, toast } from "./lib/store.svelte";
  import { inTauri } from "./lib/api";
  import { applyTheme } from "./lib/theme.svelte";
  import Shell from "./lib/Shell.svelte";
  import Tray from "./lib/pages/Tray.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import DialogHost from "./lib/components/DialogHost.svelte";

  const isTray = location.hash.startsWith("#/tray");
  if (isTray) document.body.classList.add("tray");
  applyTheme();

  onMount(async () => {
    try {
      await init();
    } catch (e) {
      toast(`nubless couldn't start talking to your monitor: ${e}`, "error");
    }
    // The main window starts hidden to avoid a blank flash; show it once painted.
    if (inTauri && !isTray) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      requestAnimationFrame(() => getCurrentWindow().show());
    }
  });
</script>

{#if isTray}
  <Tray />
{:else}
  <Shell />
  <Toasts />
  <DialogHost />
{/if}
