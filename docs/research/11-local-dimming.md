# 11 · Local Dimming: why nubless can't control it

**Conclusion (2026-11-26, firmware 3.05):** the 27GM950B doesn't expose Local Dimming to a computer through any channel nubless can safely use. It stays a setting in the monitor's own menu (Picture Adjust → Local Dimming: High, Middle, Low, Off). This page records what was tried, so nobody repeats it, and what would reopen the question.

## What was tried

| # | Route | Sends anything? | Result |
|---|---|---|---|
| 1 | **Probe diff of the declared and known codes**, Off → Low → Middle → High → Off, on Windows | Standard DDC/CI read requests (host `0x51`) | Only `0xAF` changed. Its high byte goes up by one with **every** menu change, whatever the setting (`FF`, `00`, `01`, `02`, `03`, …), so it's a change counter and not the level. An early reading of it as Off/Low/Middle/High was a coincidence of the first three steps; the code that assumed it was reverted. |
| 2 | **Full scan** of every code `0x00`–`0xFF` except factory and calibration (`nubless probe save --all`, 72 codes answer), Off → High | Standard read requests (`0x51`) | Only `0xAF` changed. No readable VCP code tracks the level. |
| 3 | **MCCS "Active Control"** (`0x52`, with `0x02`): the standard's own "which setting did the user just change" | Standard read requests (`0x51`) | Neither code moved. LG doesn't implement it; the `0xAF` counter is the nearest thing. |
| 4 | **LG Switch 7.60.0.0**, static analysis | Nothing (no monitor attached) | No opcode, enum member, `0xA6` sub-command, webOS setting key or UI resource for Local Dimming. Its DDC layer only builds standard Get/Set VCP frames from `0x51`. LG Switch can't set Local Dimming, so there's nothing to trace or copy. |
| 5 | **USB control interface** (LG controls lighting over USB HID on some UltraGear models) | Nothing (device list only) | No device with LG's USB vendor ID (`043E`) on the test PC. |
| 6 | **LG's `0x50` service channel** | Requests that turned out **not** to be reads | **Stopped for good.** See below. |

## The `0x50` incident

A scan sent standard Get VCP requests from host address `0x50` instead of `0x51` (Linux, HDMI). On that channel the monitor never answered in VCP format, not even for the first code. It returned what looks like internal memory (with the model name in ASCII), then from `0xCE` a different acknowledgement pattern, then from `0xF8` only `0xFF`. **The screen dimmed and the monitor stopped responding until it was restarted.** No setting nubless can read changed.

`0x50` is LG's factory and service channel, and its command set is undocumented. The same bytes that mean "read" on `0x51` evidently mean something else there, and which request caused the dimming is unknown, so no subset of it can be called safe. A request there could, for all we know, write calibration or other non-volatile data. A passive "dump" isn't possible either: DDC/CI only answers requests, and `0x50` exists only inside the request. [`SAFETY.md`](../../SAFETY.md) now forbids sending anything from `0x50`, and the scan script was removed.

(Not to be confused with I²C **device** address `0x50`, the EDID chip the OS reads at every connect. That holds identity data and no settings.)

## What would reopen it

Only sources that need nothing sent to the monitor:

- **A newer LG app** that lists the 27GM950B and shows Local Dimming. Then its opcode tables can be read, or its traffic captured while it changes the setting.
- **LG's service documentation** for the 27GM950B, if it describes the service channel's command set. Until then, nothing is sent on `0x50`.

## A workaround to check (menu only)

If each picture mode keeps its own Local Dimming level, switching picture mode switches dimming, and nubless already does that (the picture-mode buttons and the **Next picture mode** shortcut). To check: set Gamer 1 to High and Gamer 2 to Off in the monitor's menu, then switch between them and see whether each keeps its own level. Not yet recorded.
