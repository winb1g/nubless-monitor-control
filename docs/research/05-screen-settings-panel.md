# 05 · Screen Settings panel: control map

Target: LG Switch → **Screen Settings** ("Set up your Monitor for your work environment"). Panel contents come from screenshots of LG Switch 7.60 on a 27GM950B. This file is the working checklist for the open-source UI.

**Where the panel lives in the code:** `ModeSwitchModule.dll` → `Views/ScreenSettings.xaml`, `ViewModels.ScreenSettingsViewModel`, `Models.ScreenSettingsManager`. Its **property names are not obfuscated** (WPF bindings), so the full control list and per-model capability flags are readable (see "Capability flags" below). Requests go to the control service through `SwitchAppModel.IPCHandler`; opcode numbers and value enums are in [`08-lg-opcode-and-enum-tables.md`](08-lg-opcode-and-enum-tables.md).

**Status legend**
- **Confirmed**: live value matched the UI *and* the mapping was verified.
- **Strong**: LG's own opcode/enum name *and* the live read-back both match the UI. Still needs one write round-trip to confirm.
- **Candidate**: plausible from names or value shape, not matched to the UI yet.
- **Software**: implemented on the PC (Windows API / app logic), not a monitor setting.
- **Unknown**: no lead yet.

| # | Control (as shown) | UI range/options | Code (LG name) | Live read-back vs UI | Status |
|---|---|---|---|---|---|
| 1 | **Monitor** selector | dropdown | Enumerate physical monitors; name from EDID | n/a | Software |
| 2 | **AI Scene Optimization** | toggle | `CaptureAndPredictScene.exe` + encrypted ViT model | n/a | Software (out of scope) |
| 3 | **AI Picture Mode** | toggle | `AIMODE_DISPLAY_SETTINGS` `0xA6` / `AIMODE_PCPREDICTION_CLASS` `0xA7` (read `0x0401` / `254`); feature flag in `ai_config.json` | not compared | Candidate |
| 4 | **Picture Settings** tiles | Personalized Picture, Gamer 1, Gamer 2, Calibration1, Calibration2, Color Weakness, DCI-P3, FPS | **Supported list**: `GET_PICTURE_MODE_LIST` `0x4D`/`0x4E`/`0x4F` (+`0x45`) are 16-bit **bitmasks** over `E_PICTUREMODE_NAME` (0-15 / 16-31 / 32-47 / 48-63). **Current mode**: `PICTURE_MODE_VALUE` `0x15`. | Decoded masks contain 18 Calibration1, 19 Calibration2, 25 DCI_P3, 30 Color_Weakness, 39 Gamer1, 40 Gamer2, 43 FPS: **all 7 non-personalized tiles match.** The masks also contain Reader(1), sRGB(15), HDR_Effect(33), Vivid(41), RTS(44), Extention(47), which the UI does not show (app-side filter or different semantics). `0x15` currently reads 65 (`Studio3`?), not obviously "Personalized Picture". | **Strong** (list), Unknown (current value encoding) |
| 5 | **Brightness** | 0–100 (80) | `IMAGE_LUMINANCE` `0x10` | 80 = 80 | **Confirmed** |
| 6 | **Contrast** | 0–100 (70) | `IMAGE_CONTRAST` `0x12` | 70 = 70 | **Confirmed** |
| 7 | **Black Stabilizer** | slider (50) | `MANUFACTURER_BLACK_STABILIZER` `0xF9` | 50 = 50 | **Strong** |
| 8 | **Response Time** | dropdown (*Fast*) | `MANUFACTURER_RESPONSE_TIME` `0xF7`; LG enum `OFF=0, HIGH=1, MIDDLE=2, LOW=3` | reads 2 = `MIDDLE`, UI says *Fast* (so likely `HIGH`=Faster, `MIDDLE`=Fast, `LOW`=Normal) | **Strong** |
| 9 | **Adaptive-Sync/VRR** | dropdown (*Extended*) | `MANUFACTURER_FREE_SYNC` `0xF8`; LG enum `OFF=0, ON=1, BASIC=2, EXTENDED=3` | reads 3 = `EXTENDED`, UI says *Extended* | **Strong** |
| 10 | **Color temperature** | dropdown (*Manual*) + slider (0) | Value `COLOR_TEMP_VALUE` `0x69` (reads 6500), list `COLOR_TEMP_LIST` `0x6A` (reads `0x020F` = bits 0,1,2,3,9 = Warm, Medium, Cool, Custom, Manual3), manual RGB gains `0x16/0x18/0x1A` (50/50/50). LG enums `T_E_COLOR_TEMP`, `T_E_COLOR_TEMP_MANUAL_3`. | list bits fit the *Manual* choice; slider value 0 not matched | Candidate |
| 11 | **Display Orientation** | dropdown (*Landscape*) | Windows `ChangeDisplaySettingsEx` (`DMDO_*`); LG also keeps per-monitor values in the registry | n/a | Software |
| 12 | **Switch Dual-Mode** | *Conversion* button; shortcut (*Alt + Shift + M*); size (*Off (Full Wide)*) | Monitor side: `MONITOR_DUAL_MODE` `0xB1` with sub-commands `ON_LIST=1, OFF_LIST=2, LAST_ON=0x11, LAST_OFF=0x12, OPTION=0x20, SWITCH=0x2F, SWITCH_ON=0x21, SWITCH_OFF=0x22`. Hotkey/size in registry (`ScreenSplitter\DualModeShortcutKey`, `DualModeSizeOn_*`). | `0xB1` reads `0x2200`: high byte `0x22` = `SWITCH_OFF`, matching *Off* | Candidate (promising) |
| 13 | **My Display Preset** | toggle + *Edit* (app list) | Per-application picture-mode presets (`_AppPresetList.txt`, `PICTUREMODE_SET_LIST`); needs #4 | n/a | Software (depends on #4) |
| 14 | **AI Upscaling** | dropdown (*Off*) | VM has `AISoCUpScalingOptions`/`AISoCUpScalingOptionValue`; reply message `eSoCUpScalingList`/`eSoCUpScaling`; opcode not identified (not `0xA6`-confirmed) | none | Unknown |
| 15 | **AI Sound** | toggle (*Off*) | reply message `eAISound`; opcode not identified. Standard volume/mute are `0x62` (reads 20) and `0x8D` (reads 2 = unmuted). | none | Unknown |

Not shown in the screenshots but present in the view-model: **Sharpness** (`0x87`), **Aspect ratio** (`GET_RATIO_LIST` `0xF4` reads 6 = bits 1,2 → 16:9 and 21:9; `RATIO_VALUE` `0xF5` reads 1 = 16:9), **Motion blur** (`0xFA`, reads 2), **KVM switch**, **PBP**, **RGB color adjust** sliders, **HDR** (`HDR_VALUE` `0xE2`, reads 1).

## Capability flags (readable in `ScreenSettingsViewModel`)

The panel is **data-driven per model**: each control is shown only if the monitor says it is supported (`IsBrightnessSupported`, `IsContrastSupported`, `IsSharpnessSupported`, `IsBlackStabilizerSupported`, `IsResponseTimeSupported`, `IsFreeSyncSupported`, `IsAspectRatioSupported`, `IsColorTempManualSupported`, `IsColorRedSupported/Green/Blue`, `IsDisplayOrientationSupported`, `IsDualModeVisible`, `IsKVMSwitchVisible`, `IsSoCUpScalingSupported`, `IsSoCAIModeSupported`, `IsDisplayAIModeSupported`, `IsPicModeSettingsVisible`, `IsDisplayPresetOptionVisible`, `IsMDMonitorSupported`, `IsWebOSMoniSelected`, `IsPBPMsgVisible`). A replacement should do the same, driven by the capabilities string plus a model profile.

## Work order for the first usable release

1. **Brightness + Contrast** (confirmed): safe, standard, immediate value and covers brightness hotkeys.
2. **Volume + mute + input source** (`0x62`, `0x8D`, `0x60`): standard codes; verify by diff.
3. **Round-trip the strong candidates** (`0xF7`, `0xF8`, `0xF9`): change one in LG Switch, run `vcp-diff`, confirm the code and value table (which value is *Faster/Fast/Normal*; Adaptive-Sync option order).
4. **Picture mode** (`0x15` + lists): confirm the encoding of the current value; this unlocks "My Display Preset".
5. **Dual-Mode** (`0xB1`): understand the sub-command protocol (looks like a value that packs a sub-command in the high byte).
6. **Software features**: orientation, hotkeys, per-app presets.
7. Skip AI Scene/AI Picture Mode (encrypted model; not needed for control).

## Do-not-write list

`0x04`, `0x05`, `0x08` (factory restores), `0xD6` other than on/standby, and the **calibration/LUT group `0xE4`–`0xEB`** (`CALIBATION_COMMAND`, `WRITE_GAMMA_LUT`, `WRITE_COLOR_MATRIX`, scaler pattern/enable commands) which can corrupt the monitor's tone/color pipeline. Also `0xB1`/`0xDB` sub-commands until their protocol is understood.

## Per-control test card (copy this block per control when confirming)

```
Control:            Response Time
Date / LG Switch:   2026-10-18 / 7.60
Before (UI):        Fast       VCP snapshot: before.json
After  (UI):        Faster     VCP snapshot: after.json
vcp-diff output:    (paste)
Code:               0x__       Values: Off=_  Normal=_  Fast=_  Faster=_
Round-trip tested:  no
```
