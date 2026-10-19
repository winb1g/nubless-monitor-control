# Roadmap

Scope: the **27GM950B** only, until it fully replaces its OSD.

| Phase | Deliverable | Done when |
|---|---|---|
| 0 ✔ | Research, safety core, CLI, mock monitor, read path on real hardware | `nubless get` matches LG Switch |
| 1 | First hardware writes: brightness, contrast, volume, mute | Round-trip test cards recorded; 1,000 write/read cycles with no stuck state |
| 2 | Map the LG codes: response time, VRR, black stabilizer, picture mode, color temperature, AI Scene Optimization, AI Sound, AI Upscaling | Every option of every dropdown has a recorded value |
| 3 (in progress) | **Desktop UI** (tray app + panel in the layout of LG Switch's Screen Settings), global hotkeys, safe ranges, undo toasts, interlock prompts | Feature parity with the OSD for everyday settings |
| 4 | Software features: display orientation, per-app picture presets (My Display Preset), Dual-Mode layout and hotkey | Parity with LG Switch's Screen Settings |
| 5 | Packaging: signed Windows build, macOS app, Linux packages; udev rule for i2c access | Install in one step on each OS |

Not planned: firmware flashing, LG's encrypted AI Picture Mode model, wallpapers, video-call and calendar features.

UI stack: **Tauri 2 + Svelte 5** (decided 2026-10-18), with the signed GitHub Releases updater.