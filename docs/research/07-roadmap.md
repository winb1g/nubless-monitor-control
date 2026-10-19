# 07 · Roadmap for the open-source controller

Working name: **OpenSwitch** (placeholder; avoid LG marks in the final name).

## Goals

1. Control the monitor from a small desktop UI and **global hotkeys** (brightness, contrast, volume/mute, input source, picture mode, response time, Adaptive-Sync, black stabilizer).
2. Zero dependency on LG Switch; works with LG's services stopped.
3. Tiny footprint (tray app, a few MB, no background GPU/AI work).
4. Scriptable CLI for the same operations.
5. Honest device support: a **model profile** file (JSON) per monitor listing codes, ranges, labelled values and confidence. 27GM950B first.

## Non-goals

Firmware flashing, AI Scene/Picture, wallpapers, Screen Splitter clone (maybe later), anything needing LG's encrypted assets.

## Architecture

```
┌────────────┐   ┌───────────────┐   ┌──────────────────────────┐
│ Tray UI    │   │ CLI           │   │ Global hotkeys           │
└─────┬──────┘   └──────┬────────┘   └────────────┬─────────────┘
      └─────────────────┴────────── core ─────────┘
                    model profile (JSON) ─► ddc layer
                    ddc layer: dxva2 (Windows)   [later: ddcutil/i2c (Linux)]
```

- **Core**: monitor discovery (EDID → model), profile lookup, value validation, rate-limiter/queue per monitor, retry + read-back.
- **DDC layer**: Windows `dxva2.dll` P/Invoke (`GetVCPFeatureAndVCPFeatureReply`, `SetVCPFeature`, capabilities). Works today on the 27GM950B for reads.
- **Profile** (`profiles/27GM950B.json`): `{ id, edidProductCodes, codes: { brightness: {vcp:"10", range:[0,100], confidence:"confirmed"}, responseTime: {vcp:"F5", values:{...}, confidence:"candidate"} } }`. Controls with confidence below *confirmed* are hidden or shown behind an "experimental" switch.
- **Language**: C# (.NET 8, WinUI/WPF tray, trivially P/Invokes `dxva2`) or Rust (`windows` crate, small static exe, `tauri`/`egui` UI). Pick by who will maintain it; the protocol work is identical.
- **Hotkeys**: `RegisterHotKey`; no keyboard hook, no admin rights.
- **Coexistence**: if `SmartMonitorControlService.exe` is running it may talk DDC/CI at the same time. Serialise with retry/back-off; document that LG Switch can be quit or its service stopped.

## Phases

| Phase | Deliverable | Exit criterion |
|---|---|---|
| 0 (now) | Research repo, read-only tools | `docs/04`, `docs/05` filled for standard codes |
| 1 | **Standard-code controller**: brightness, contrast, volume, mute, input, power-save with read-back; tray + hotkeys + CLI | Round-trip tested on a 27GM950B; no stuck states after 1,000 write/read cycles |
| 2 | **Private-code mapping** with `vcp-diff` (response time, Adaptive-Sync, black stabilizer, picture modes, color temperature) | Every option of every dropdown has a recorded value; profile marked *confirmed* |
| 3 | Software features: orientation, per-app presets, Dual-Mode layout/hotkeys | Parity with LG's panel for the 27GM950B |
| 4 | Other models via community profiles + a "contribute a profile" wizard (guided vcp-diff) | 3+ models |

## Safety rules (non-negotiable)

- **Never write a code that is not in a model profile marked at least *confirmed* for that action.**
- **Blocklist** `0x04`, `0x05`, `0x08` (factory restore), `0xD6` values other than on/standby, and anything not declared by the capabilities string. LG's own SDK exposes `CFactorySetting::Restore`; we do not.
- Writes only after explicit user action; no background "sync" writes; rate-limited; always read back.
- Never touch firmware. Never send raw user-typed VCP writes unless a hidden developer flag is set and a confirmation names the code and value.
- Log every write (code, old, new) locally for undo.

## Immediate next steps

1. User runs `tools/vcp-diff.ps1` around each Screen Settings control (see the test card in `05-screen-settings-panel.md`). Each pair takes about a minute.
2. Fill the mapping tables; mark confidence.
3. Prototype the Phase 1 core with brightness first: write `value`, wait, read back, write the original back.
4. Decide language/UI stack, create the app repo, move `docs/` and `tools/` under it or keep this as the research repo.
