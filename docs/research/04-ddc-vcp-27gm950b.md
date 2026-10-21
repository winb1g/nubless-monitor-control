# 04 · What the 27GM950B reports over DDC/CI

Read with `tools/vcp-scan.ps1` through Windows' `dxva2.dll` (no LG software involved), monitor connected by DisplayPort. Windows shows it as *Generic PnP Monitor* / EDID name `LG ULTRAGEAR+`, product code `5CF8`.

## Capabilities string (as returned by the monitor)

```
(prot(monitor)type(lcd)model(WK95U)cmds(01 02 03 0C E3 F3)
 vcp(02 04 05 08 10 12 14(05 08 0B) 16 18 1A 52 60(11 12 0F 10) AC AE B2 B6 C0 C6 C8 C9 D6(01 04) DF
     62 8D F4 F5(01 02 03 04) F6(00 01 02) 4D 4E 4F 15(01 06 11 13 14 15 18 19 20 22 23 24 28 29 32 48)
     F7(00 01 02 03) F8(00 01) F9 E4 E5 E6 E7 E8 E9 EA EB EF FA(00 01) FD(00 01) FE(00 01 02) FF)
 mccs_ver(2.1)mswhql(1))
```

- `model(WK95U)` is a **copy-paste leftover** from another LG model (a 34WK95U); don't use this field to identify the monitor.
- `cmds(01 02 03 0C E3 F3)`: standard DDC/CI command opcodes: `01`/`02` get-VCP request/reply, `03` set-VCP, `0C` save current settings, `E3`/`F3` capabilities reply/request. Nothing unusual.
- `mccs_ver(2.1)`.

## Declared codes and a snapshot of their values

Snapshot values are one example state (2026-10-18) and **depend on the user's settings**. "Std" is the name from the public VESA MCCS table as I know it; confidence is stated, not assumed.

| VCP | Std name | cur / max in snapshot | Notes / confidence |
|---|---|---|---|
| `02` | New control value | 2 / 2 | standard, high |
| `04` | Restore factory defaults | 0 / 255 | **dangerous if written** |
| `05` | Restore factory luminance/contrast | 0 / 1 | **dangerous if written** |
| `08` | Restore factory color defaults | 0 / 255 | **dangerous if written** |
| `10` | Luminance (brightness) | **80** / 100 | **Matches LG Switch panel (Brightness 80). Confirmed.** |
| `12` | Contrast | **70** / 100 | **Matches LG Switch panel (Contrast 70). Confirmed.** |
| `14` | Select color preset (sub-values `05`,`08`,`0B`) | `0xFFDE` / 65535 | Standard values would be 6500K (`05`), 9300K (`08`), user 1 (`0B`); the read-back is not a standard id, so LG likely uses this differently. Unknown. |
| `16` `18` `1A` | Video gain R/G/B | 50 / 50 / 50, max 100 | standard, high. Probably the *Manual* color-temperature sliders. |
| `52` | Active control | 0 / 255 | standard, low relevance |
| `60` | Input source (declared values `0F`,`10`,`11`,`12`) | `0x0F` / 18 | standard: `0F` DisplayPort-1, `10` DisplayPort-2, `11` HDMI-1, `12` HDMI-2. Current value `0F` matches a DisplayPort connection. |
| `62` | Audio speaker volume | 20 / 100 | standard, high |
| `8D` | Audio mute | 2 | standard (1 muted, 2 unmuted), medium |
| `AC`, `AE` | Horizontal / vertical frequency | 4800 / 480 | read-only timing info; the values looked odd (maybe a different unit), low confidence |
| `B6` | Display technology type | 3 | standard (3 = TFT?), low |
| `C0` | Display usage time | (varies per monitor) | probably hours, medium |
| `C6` | Application enable key | 104 | standard, no use |
| `C8` | Display controller type | 21 (`0x15`) | manufacturer id of the scaler, no use |
| **`C9`** | **Display firmware level** | **`0x0305` = 3.05** | **Equals the `US` number in LG's firmware name and the first number in LG Switch's update history. Verified.** |
| `D6` | Power mode (`01` on, `04` standby/off) | 1 | standard, high. Writing `04` would turn the display off. |
| `DF` | VCP version | `0x0201` = 2.1 (first read) | **returned an error on a later read** (see "flakiness") |
| `B2` | Flat-panel sub-pixel layout | read error | not supported or flaky |
| `15` (sub-values 01 … 48) | unknown / manufacturer | 65 / 255 | unknown. The long sub-value list looks like a mode table. |
| `4D` `4E` `4F` | unknown | 32770 / 16908 / 39810 (raw) | unknown |
| `E4`–`EB`, `EF`, `F4`–`FA`, `FD`–`FF` | **manufacturer-specific range** | see below | **Almost certainly where LG's picture mode, response time, Adaptive-Sync, Black Stabilizer, AI features live.** Not yet mapped. |

### LG's names for these codes (from `T_E_OPCODE`, added after decoding LG Switch)

Re-read on 2026-10-18 with `tools/vcp-read.ps1`. "Value meaning" uses LG's own enums (`08-lg-opcode-and-enum-tables.md`).

| VCP | LG name | Reads | Value meaning | Panel match |
|---|---|---|---|---|
| `0x15` | `MANUFACTURER_PICTURE_MODE_VALUE` | 65 | `E_PICTUREMODE_NAME`? 65 = `Studio3`; likely not the direct index (UI shows *Personalized Picture*) | unclear |
| `0x4D` `0x4E` `0x4F` (`0x45`) | `GET_PICTURE_MODE_LIST` … `LIST3` | `0x8002`, `0x420C`, `0x9B82` (`0x3000`) | bitmasks of supported picture modes: bit *n* of list *k* = mode `16k+n` | **7/7 UI tiles present** (18, 19, 25, 30, 39, 40, 43) |
| `0xF4` / `0xF5` | `GET_RATIO_LIST` / `RATIO_VALUE` | 6 / 1 | list bits 1,2 = 16:9, 21:9; value 1 = 16:9 (`T_E_ASPECT_RATIO`) | not in screenshots |
| `0xF6` | `SES` | 1 | unknown | n/a |
| `0xF7` | `RESPONSE_TIME` | 2 | `0 Off, 1 HIGH, 2 MIDDLE, 3 LOW` | UI *Fast* |
| `0xF8` | `FREE_SYNC` | 3 | `0 Off, 1 On, 2 Basic, 3 Extended` | UI *Extended* ✔ |
| `0xF9` | `BLACK_STABILIZER` | 50 | 0–255 range; enum `OFF/LOW/MIDDLE/HIGH` is for other models | UI *50* ✔ |
| `0xFA` | `MOTION_BLUR` | 2 | unknown scale; declared `00 01`, so 2 may mean "unavailable": the setting only exists while Adaptive-Sync is Off ([`../rules.md`](../rules.md)) | not in screenshots (hidden while Adaptive-Sync was on) |
| `0x62` / `0x8D` | `VOLUME` / `AUDIO_MUTE` | 20 / 2 | volume 0–100; mute 1 = muted, 2 = unmuted | n/a |
| `0x69` / `0x6A` | `COLOR_TEMP_VALUE` / `COLOR_TEMP_LIST` | 6500 / `0x020F` | kelvin / bitmask over `T_E_COLOR_TEMP` (bits 0-3, 9 → Warm, Medium, Cool, Custom, Manual3) | UI *Manual* |
| `0xB1` | `MONITOR_DUAL_MODE` | `0x2200` | high byte `0x22` = `SWITCH_OFF`; sub-commands `0x01/0x02` on/off lists, `0x11/0x12` last on/off, `0x20` option, `0x21/0x22` switch on/off, `0x2F` switch | UI *Off (Full Wide)* |
| `0xE2` | `HDR_VALUE` | 1 | unknown scale | n/a |
| `0xA6` / `0xA7` | `AIMODE_DISPLAY_SETTINGS` / `AIMODE_PCPREDICTION_CLASS` | `0x0401` / 254 | unknown | AI Picture Mode? |
| `0xEC` | `SMARTRESOLUTION` | 1 | unknown | n/a |
| `0xFB` | `SCALER_ISP_AREA` | 1 (max 15) | unknown | n/a |
| `0xE4`–`0xEB` | `CALIBATION_COMMAND`, `GETGAMMA_FROM_SCALER`, `WRITE_GAMMA_LUT`, `WRITE_COLOR_MATRIX`, scaler enable/pattern, `WRITE_NEW_COLOR_MATRIX` | 0 / … | **calibration write channel: never touch** | n/a |
| `0xAF`, `0xA1`, `0xA2` | `MONITOR_MODEL_NAME`, `GET_MONITOR_ID_EXTENDED(_2)` | `0xFF00`, `0xD098`, `0x4800` | model name likely arrives through a table/multi-byte read, not a single value | n/a |
| `0x47`, `0xE0`, `0xE1`, `0xED`, `0xF0`, `0xF1`, `0xFC` | various `MANUFACTURER_*` | no answer | not supported on this monitor | n/a |

These matches are strong evidence (name **and** value agree with the UI for `0xF8`, `0xF9`, `0x4E`/`0x4F`, `0xB1`) but **no write has been performed**. Confirm each with `vcp-diff` before trusting a value table.

Manufacturer-specific codes with their declared sub-values and snapshot reading:

| VCP | Declared sub-values | cur / max |
|---|---|---|
| `F4` | none | 6 / 65535 |
| `F5` | 01 02 03 04 | 1 / 255 |
| `F6` | 00 01 02 | 1 / 255 |
| `F7` | 00 01 02 03 | 2 / 255 |
| `F8` | 00 01 | 3 / 255 |
| `F9` | none | **50** / 255 (same as the panel's Black Stabilizer 50, a *candidate*, not confirmed) |
| `E4` | none | 0 / 255 |
| `E5` | none | 0 / 179 |
| `E6` `E7` | none | 0 / 65535 |
| `E8` | none | 0 / 255 |
| `E9` | none | 4274 / 255 (cur > max, so likely a packed value) |
| `EA` `EB` | none | 0 / 255 · 0 / 1 |
| `EF` | none | 49188 / 65535 |
| `FA` | 00 01 | 2 / 255 |
| `FD` `FE` `FF` | 00 01 · 00 01 02 · none | FD error · 4 / 255 · FF error |

## Flakiness (important for any implementation)

DDC/CI is a slow I2C-like link. In testing, the same code sometimes answered and sometimes failed within minutes (`DF`, `B2`, `FD`, `FF`). A controller must:

- wait **at least 40-50 ms between messages** (the DDC/CI protocol requires inter-message delays of that order; `vcp-diff.ps1` sleeps 60 ms between retries),
- **retry** reads (3 attempts with short sleeps worked),
- treat a single failed read as "unknown", never as "value 0",
- **read back after writing** to confirm,
- serialise access (one command at a time per monitor), including against LG's own service if it is running.

## How to map a manufacturer code

```powershell
.\tools\vcp-diff.ps1 -Save before.json
# change exactly ONE control in LG Switch (or the monitor OSD)
.\tools\vcp-diff.ps1 -Compare before.json
```

Record the code, the value before/after, and the UI label in `05-screen-settings-panel.md`. Repeat for every option of every dropdown so the value table is complete.
