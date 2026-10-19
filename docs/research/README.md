# lg-monitor-re

Interoperability research on **LG Switch** (v7.60), to build an **open-source, lightweight replacement** for its *Screen Settings* panel: control an LG UltraGear monitor (tested on a **27GM950B**) from the desktop and with hotkeys, instead of the on-monitor joystick, and without running LG's ~940 MB app (the download size LG lists for the Windows installer).

> Research snapshot imported from the `lg-monitor-re` working folder on 2026-10-18. The app itself lives in the repo root; this folder is the evidence behind it.

## What's in here

| Path | What |
|---|---|
| [`00-overview.md`](00-overview.md) | Architecture of LG Switch: processes, services, pipes, who talks to the monitor |
| [`01-endpoints.md`](01-endpoints.md) | Every LG server URL the app uses (verified live) |
| [`02-local-files-and-registry.md`](02-local-files-and-registry.md) | Where the app keeps settings, history and hotkeys |
| [`03-firmware-update-flow.md`](03-firmware-update-flow.md) | How firmware is found, named, downloaded and flashed; per-model URL table |
| [`04-ddc-vcp-27gm950b.md`](04-ddc-vcp-27gm950b.md) | What the monitor reports over DDC/CI and what each code appears to do |
| [`05-screen-settings-panel.md`](05-screen-settings-panel.md) | Control-by-control map of the panel we want to reproduce, with evidence level |
| [`08-lg-opcode-and-enum-tables.md`](08-lg-opcode-and-enum-tables.md) | LG's own opcode table (211 entries) and value enums (response time, Adaptive-Sync, picture modes…) with numeric values |
| [`06-method.md`](06-method.md) | How the findings were obtained (reproducible), and what we deliberately did not do |
| [`09-ai-channel-and-picture-modes.md`](09-ai-channel-and-picture-modes.md) | LG's AI sub-command channel (VCP 0xA6), which picture modes allow AI, and picture-mode values on the wire |
| [`07-roadmap.md`](07-roadmap.md) | Proposed architecture, phases, safety rules |
| [`tools/`](tools/README.md) | Scripts to reproduce and to continue the mapping (read-only) |

Companion project: [`lg-firmware-watch`](https://lg-firmware-watch.vercel.app/) (site that tracks 27GM950B firmware).

## Ground rules

- **Read-only first.** The mapping is done by *snapshotting* the monitor's VCP values before/after you change a setting in LG's own UI (`tools/vcp-diff.ps1`). We never write to an unmapped code.
- **No LG binaries or decoded string dumps are committed** (see `.gitignore`). Findings are written in our own words; we publish facts needed for interoperability (protocol, endpoints, formats), not LG's code or assets.
- **Dangerous operations are documented, never wrapped casually.** LG's SDK includes a *factory restore* path (VCP `0x04`/`0x05`/`0x08` and a `CFactorySetting::Restore`). A control app must not expose them without an explicit confirmation.
- Firmware flashing is **out of scope** for a replacement. A bad flash can brick a monitor; use LG's tool.

## Legal note

This is independent research for interoperability with hardware the author owns. It is not affiliated with or endorsed by LG Electronics. "LG", "LG Switch", "UltraGear" are LG trademarks. Check the laws that apply to you before reverse engineering software; this repo is not legal advice.

## License

Code in this repo: MIT. Docs: CC BY 4.0.
