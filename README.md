# Nubless Monitor Control

**Never touch the joystick nub under your monitor again.**

nubless is an open-source, cross-platform replacement for the on-screen menu (OSD) of the **LG UltraGear AI 27GM950B**, written in Rust. It talks to the monitor over standard DDC/CI, the same channel LG's own *LG Switch* app uses, so you can change picture settings from your desktop instead of from the nub.

Inspired by [OpenLogi](https://github.com/AprilNEA/OpenLogi) (Logitech), [MonitorControl](https://github.com/MonitorControl/MonitorControl) and [BetterDisplay](https://github.com/waydabber/BetterDisplay).

> **Status: early.** The core, safety guard and CLI work, and real-hardware reads are verified on a 27GM950B, and brightness, picture mode, AI Scene Optimization and AI Sound writes are verified on the OSD. Other writes wait for the tests in [`docs/verify-on-hardware.md`](docs/verify-on-hardware.md). Not affiliated with LG Electronics.

## Two promises

1. **nubless will never brick your display.** Factory resets, calibration/LUT channels, power, EDID and every unmapped code are on a hard-coded deny-list that no profile, flag or user input can override. Firmware is out of scope, permanently.
2. **You can never get your settings into a mess.** Sliders stay inside safe ranges, features that LG's firmware keeps apart stay apart (the same interlocks as LG Switch), and writes the monitor refuses are rolled back. Your original settings are saved the moment nubless first connects, before it can change anything, so "Reset to my original settings" always works.

How both are enforced, in code and tests: [`SAFETY.md`](SAFETY.md).

## What works today

| Control | Code | Status |
|---|---|---|
| Brightness, Contrast | `0x10`, `0x12` | read ✔ · write: safe policy (standard MCCS) |
| Black Stabilizer, Response Time, Adaptive-Sync/VRR | `0xF9`, `0xF7`, `0xF8` | read ✔ · write: `--experimental` until round-trip confirmed |
| Volume, Mute | `0x62`, `0x8D` | read ✔ · write: `--experimental` |
| Picture mode (all 8 tiles) | `0x15` | read ✔ · write ✔ (OSD-verified), through the AI interlock |
| AI Scene Optimization, AI Sound | `0xA6` sub-commands 2, 6 | read ✔ · write ✔ (OSD-verified) |
| Color temperature, RGB gains, aspect ratio, motion blur, AI Upscaling | various | read ✔ · not writable yet (encoding unconfirmed) |
| Input | `0x60` | read ✔ · **blocked**: switching input can cut the DDC link (dead end) |
| Firmware level, usage hours, Dual-Mode state | `0xC9`, `0xC0`, `0xB1` | read-only |
| Interlocks (AI features, PBP/PIP, picture modes that block AI) | — | done, from LG Switch's own logic ([`docs/rules.md`](docs/rules.md)) |

The full control map with evidence is in [`docs/research/05-screen-settings-panel.md`](docs/research/05-screen-settings-panel.md).

## Desktop app

A tray app with a full settings window, built with Tauri 2 and Svelte: your monitors in a strip, smart features with LG's interlocks, picture mode, sliders, gaming and sound controls, Labs for not-yet-verified controls, light and dark themes, and one-click updates (update, install and restart). Run it against a simulated monitor with no hardware involved:

```bash
cd apps/desktop && npm install && npx tauri dev -- -- --demo
```

The UI alone also runs in a normal browser (`npm run dev`) with a built-in preview backend, which is handy for design work.

## Try it

Needs Rust 1.80+. On Linux, load `i2c-dev` and give your user access to `/dev/i2c-*`. Enable **DDC/CI** in the monitor's menu (it is on by default).

```bash
cargo build --release
```

```bash
nubless list                       # which displays are supported (read-only)
nubless get                        # read every control
nubless set brightness 60          # change one (read back, journaled)
nubless --experimental set adaptive-sync basic   # asks "Keep this change?" or reverts in 15 s
nubless undo                       # undo the last change
nubless snapshot restore --baseline   # put back what you had before nubless
nubless controls                   # what each control is, how sure we are, and why
nubless --mock get                 # a simulated 27GM950B, no hardware needed
```

If LG Switch is running, its service also talks DDC/CI. nubless paces and retries its messages, but quitting LG Switch avoids collisions.

## Layout

| Path | What |
|---|---|
| `crates/nubless-core` | Profiles, capabilities parser, write guard, controller (pacing, read-back, rollback, journal), interlock rules, mock monitor. No hardware dependencies. |
| `crates/nubless-hw` | Real DDC/CI through [`ddc-hi`](https://crates.io/crates/ddc-hi): Windows `dxva2`, Linux `i2c-dev`, macOS. |
| `crates/nubless-cli` | The `nubless` command. |
| `apps/desktop` | The desktop app: Svelte UI (`src/`) and Tauri shell (`src-tauri/`), whose monitor thread owns the connection. |
| `docs/` | Safety rules, interlocks, hardware checklist, roadmap, and the reverse-engineering research behind all of it. |

## Contributing

The most useful help right now is **mapping**: running the read-only `vcp-diff` around one OSD setting at a time and recording the result. See [`docs/mapping-guide.md`](docs/mapping-guide.md) and [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Legal

Independent interoperability work on hardware the authors own. "LG", "LG Switch" and "UltraGear" are trademarks of LG Electronics; nubless is not affiliated with or endorsed by LG. No LG binaries, decoded string dumps or assets are included. Use at your own risk.

## License

Code: MIT. Docs: CC BY 4.0.
