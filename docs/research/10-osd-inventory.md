# 10 · The on-screen menu (OSD), option by option

Recorded by hand from a real 27GM950B (firmware 3.05, 2026-10-20): every menu page, every option. This is what nubless is measured against: a control is "done" when each option below is readable and (if we ever write it) writable with the OSD agreeing.

Status key: ✔ mapped and used · ◐ read-only or partly mapped · ○ not mapped yet · ⛔ never (deny-list).

| Menu | Setting | Options in the OSD | In nubless | Next step |
|---|---|---|---|---|
| **AI Boost** | AI Upscaling | (options not recorded) | ◐ `0xA6` sub 4, read-only; a unit read 1 while LG Switch showed Off | Record the options; read the value in each |
| | AI Scene Optimization | on / off; limited game modes | ✔ `0xA6` sub 2 | – |
| | AI Sound | on / off | ✔ `0xA6` sub 6 | – |
| **Game Adjust** | Game Mode (picture mode) | Personalized Picture, Gamer 1, Gamer 2, FPS, RTS, Vivid, Reader, HDR Effect, sRGB, DCI-P3, Color Weakness, Max Brightness, Calibration 1, Calibration 2 (**14**) | ◐ `0x15`: 8 of 14 (Personalized, Gamer 1/2, FPS, DCI-P3, Color Weakness, Calibration 1/2) | Read `0x15` in RTS, Vivid, Reader, HDR Effect, sRGB, Max Brightness |
| | Dual-Mode | Off (Full Wide), Off (24.5"), On (Full Wide), On (24.5") | ◐ `0xB1` reads `0x2200` = Off (Full Wide); write channel on the deny-list | Read `0xB1` in all four |
| | Adaptive Sync | Extended, Basic, Off | ✔ read · Labs write `0xF8` (3 / 2 / 0) | Test card |
| | Black Stabilizer | slider (scale not recorded) | ✔ read · Labs write `0xF9`, assumed 0–100 | Record the OSD scale; test card |
| | Response Time | Faster, Fast, Normal, **Off** | ✔ read · Labs write `0xF7` (1 / 2 / 3, **0 = Off**) | Test card |
| | Crosshair | red plus, green plus, red dot, green dot, off | ○ | Probe diff |
| | FPS Counter | on / off | ○ | Probe diff |
| | Game Reset | no / yes | ⛔ factory-style reset | never |
| **Picture Adjust** | Brightness | slider | ✔ `0x10` | – |
| | Contrast | slider | ✔ `0x12` | Test card |
| | Sharpness | slider | ○ (standard `0x87` is **not** in the capabilities string) | Probe diff |
| | Color Temp | Custom, Warm, Medium, Cool, Manual (0–100) | ◐ `0x69` reads 6500; `0x14` reads a non-standard value | Probe diff per option, and the Manual slider |
| | Local Dimming | High, Middle, Low, Off | ○ | Probe diff |
| | Picture Reset | no / yes | ⛔ | never |
| **Sound** | Sound Out | H/P out, internal speaker | ○ | Probe diff |
| | Bass Boost | on / off | ○ | Probe diff |
| | Volume | slider | ✔ read · Labs write `0x62` | Test card |
| **Input** | Input List | HDMI 1, HDMI 2, DisplayPort, USB-C | ◐ `0x60`: `0x11` HDMI 1, `0x12` HDMI 2, `0x0F` DisplayPort, `0x10` USB-C (by elimination); **blocked**: it can cut DDC/CI | Read-only checks only |
| | Aspect Ratio | Full Wide, Original, Just Scan | ◐ `0xF5` reads 1 = Full Wide; the monitor declares four values (`01 02 03 04`) | Read `0xF5` in each |
| | Auto Input Switch | on / off | ○ | Probe diff |
| **General** | Hexagon Lighting | Static 1 (purple), Static 2 (red), Static 3 (cyan), Static 4 (white), Cycling, Off | ○ (possibly not on the DDC/CI channel at all) | Probe diff |

Also declared by the monitor but not matched to any OSD setting: `0xF6` (values `00 01 02`), `0xFA` (`00 01`, read 2), `0xFD` (`00 01`), `0xFE` (`00 01 02`). They are the first candidates for Local Dimming, Bass Boost, Auto Input Switch and similar; the probe diffs below settle it.

## How to map one (read-only, safe)

`nubless probe` only reads. Change **one** OSD setting between the two commands, then diff:

```
nubless probe save before.json
# change ONE setting in the OSD (for example Local Dimming: Off -> High)
nubless probe diff before.json
```

For a setting with several options, repeat per option and write down the code, the option name and the raw value. Put the result in the PR as a test card (see [`../mapping-guide.md`](../mapping-guide.md)). Nothing is written to the monitor, so this is the lowest-risk way to move a control from ○ to ◐.
