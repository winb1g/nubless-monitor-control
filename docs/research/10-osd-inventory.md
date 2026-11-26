# 10 · The on-screen menu (OSD), option by option

Recorded by hand from a real 27GM950B (firmware 3.05, 2026-10-20): every menu page, every option. This is what nubless is measured against: a control is "done" when each option below is readable and (if we ever write it) writable with the OSD agreeing.

Status key: ✔ mapped and used · ◐ read-only or partly mapped · ○ not mapped yet · ⛔ never (deny-list).

| Menu | Setting | Options in the OSD | In nubless | Next step |
|---|---|---|---|---|
| **AI Boost** | AI Upscaling | High, Low, Off | ◐ `0xA6` sub 4, read-only; LG's enum has 1 / 2 / 4 and a unit read 1 while LG Switch showed Off, so read as 1 = Off, 2 = Low, 4 = High (inferred) | Read the value in each option |
| | AI Scene Optimization | on / off; limited game modes | ✔ `0xA6` sub 2 | – |
| | AI Sound | on / off | ✔ `0xA6` sub 6 | – |
| **Game Adjust** | Game Mode (picture mode) | Personalized Picture, Gamer 1, Gamer 2, FPS, RTS, Vivid, Reader, HDR Effect, sRGB, DCI-P3, Color Weakness, Max Brightness, Calibration 1, Calibration 2 (**14**) | ◐ `0x15`: 8 of 14 (Personalized, Gamer 1/2, FPS, DCI-P3, Color Weakness, Calibration 1/2) | Read `0x15` in RTS, Vivid, Reader, HDR Effect, sRGB, Max Brightness |
| | Dual-Mode | Off (Full Wide), Off (24.5"), On (Full Wide), On (24.5") | ◐ `0xB1` reads `0x2200` = Off (Full Wide); write channel on the deny-list | Read `0xB1` in all four |
| | Adaptive Sync | Extended, Basic, Off | ✔ read · Labs write `0xF8` (3 / 2 / 0) | Test card |
| | Motion Blur Reduction | not in the first inventory: **only available while Adaptive Sync is Off** ([`../rules.md`](../rules.md)); options not recorded | ◐ `0xFA` reads 2 (declared `00 01`) and is read-only | With Adaptive Sync Off: record the options, read `0xFA` in each |
| | Black Stabilizer | slider 0–100 | ✔ read · Labs write `0xF9` (0–100) | Test card |
| | Response Time | Faster, Fast, Normal, **Off** | ✔ read · Labs write `0xF7` (1 / 2 / 3, **0 = Off**) | Test card |
| | Crosshair | red plus, green plus, red dot, green dot, off | ○ | Probe diff |
| | FPS Counter | on / off | ○ | Probe diff |
| | Game Reset | no / yes | ⛔ factory-style reset | never |
| **Picture Adjust** | Brightness | slider | ✔ `0x10` | – |
| | Contrast | slider | ✔ `0x12` | Test card |
| | Sharpness | slider | ○ (standard `0x87` is **not** in the capabilities string) | Probe diff |
| | Color Temp | Custom, Warm, Medium, Cool, Manual (0–100) | ◐ `0x69` reads 6500; `0x14` reads a non-standard value | Probe diff per option, and the Manual slider |
| | Local Dimming | High, Middle, Low, Off | ⛔ **not exposed to a computer**: no readable code tracks it, LG Switch has no control for it, and the `0x50` service channel is unsafe ([`11-local-dimming.md`](11-local-dimming.md), fw 3.05, 2026-11-26) | Menu only. Check whether each picture mode keeps its own level |
| | Picture Reset | no / yes | ⛔ | never |
| **Sound** | Sound Out | H/P out, internal speaker | ○ | Probe diff |
| | Bass Boost | on / off | ○ | Probe diff |
| | Volume | slider | ✔ read · Labs write `0x62` | Test card |
| **Input** | Input List | HDMI 1, HDMI 2, DisplayPort, USB-C | ◐ `0x60`: `0x11` HDMI 1, `0x12` HDMI 2, `0x0F` DisplayPort, `0x10` USB-C (by elimination); **blocked**: it can cut DDC/CI | Read-only checks only |
| | Aspect Ratio | Full Wide, Original, Just Scan | ◐ `0xF5` reads 1 = Full Wide; the monitor declares four values (`01 02 03 04`) | Read `0xF5` in each |
| | Auto Input Switch | on / off | ○ | Probe diff |
| **General** | Hexagon Lighting | Static 1 (purple), Static 2 (red), Static 3 (cyan), Static 4 (white), Cycling, Off | ○ (possibly not on the DDC/CI channel at all) | Probe diff |

Also declared by the monitor but not matched to any OSD setting: `0xF6` (values `00 01 02`), `0xFA` (`00 01`, read 2), `0xFD` (`00 01`), `0xFE` (`00 01 02`). They are the first candidates for Bass Boost, Auto Input Switch and similar (not Local Dimming: see [`11`](11-local-dimming.md)); the probe diffs below settle it.

**`0xAF` counts OSD changes.** Its high byte goes up by one with every change made in the menu, whatever the setting (`FF`, `00`, `01`, … wrapping), and the low byte stays `00`. Stepping Local Dimming Off → Low → Middle → High → Off → Low → Off → Low read `FF`, `00`, `01`, `02`, `03`, (`04`), `04`, `05` (fw 3.05, 2026-11-26). LG Switch names the code `MONITOR_MODEL_NAME` ([`08`](08-lg-opcode-and-enum-tables.md)). `nubless probe diff` marks it as noise.

**Local Dimming isn't in LG Switch either** (static analysis of LG Switch 7.60.0.0, 2026-11-26). No opcode, enum member, sub-command, webOS setting key or resource names it. The DDC layer (`LGProtocolEngine.dll`) only builds standard MCCS Get/Set VCP frames from host address `0x51` to `0x6E`, with no remap table, so LG Switch never uses the `0x50` service channel. Still open: only a newer LG app that lists the 27GM950B.

> [!CAUTION]
> **Don't probe LG's `0x50` service channel.** A scan that sent standard Get VCP requests from host address `0x50` (2026-11-26, Linux over HDMI, fw 3.05) never got a single VCP reply (`6E 88 02 …`). From the first code it returned a repeating stream that looks like internal memory (with the model name `27GM950B` in ASCII), from `0xCE` a short `BE <code> …` acknowledgement, from `0xF8` only `0xFF`. The screen dimmed and the monitor stopped responding until it was restarted. On that channel a "read" is not a read. The script is gone; see [`SAFETY.md`](../../SAFETY.md).

## How to map one (read-only, safe)

`nubless probe` only reads. Change **one** OSD setting between the two commands, then diff:

```
nubless probe save before.json
# change ONE setting in the OSD (for example Local Dimming: Off -> High)
nubless probe diff before.json
```

If a setting moves nothing but `0xAF`, it isn't in the codes `probe` reads by default: save with `nubless probe save --all before.json` (every code from `0x00` to `0xFF` except the factory and calibration ones; a minute or two) and diff against that.

For a setting with several options, repeat per option and write down the code, the option name and the raw value. Put the result in the PR as a test card (see [`../mapping-guide.md`](../mapping-guide.md)). Nothing is written to the monitor, so this is the lowest-risk way to move a control from ○ to ◐.
