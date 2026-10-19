# 09 · LG's AI channel (VCP 0xA6) and picture-mode rules

Decoded from LG Switch 7.60 (`SwitchAppModel.dll`, `ModeSwitchModule.dll`) by annotating the IL with the app's own decrypted strings, then cross-checked against a live 27GM950B with `nubless probe watch` while switching settings in LG Switch. Method as in [`06-method.md`](06-method.md); no LG code is reproduced here.

## VCP 0xA6 is a multiplexed sub-command channel

`MonitorManager.SetSoCAIModeOnOff`, `SetAISoundOnOff` and `SetSoCUpscallingAIModeValue` all write VCP `0xA6` (`AIMODE_DISPLAY_SETTINGS`) with value `(sub-command << 8) | argument`. Getters first write `(sub-command << 8) | 0xFF` (`eGET_REQUEST_VALUE`) and then read 0xA6; the reply carries the same high byte and the current value in the low byte (`HandleAIModeOpcodeReceviedResponse`).

Sub-commands and values, from LG's `AIMODE_DISPLAYSETTINGS_VALUES` enum:

| Sub | Meaning | Values |
|---:|---|---|
| 1 | supported AI option list | bitmask |
| 2 | **AI Scene Optimization** (on the monitor's SoC) | 1 off, 2 on |
| 3 | AI Picture Mode (PC side) | 1 off, 2 on |
| 4 | **AI Upscaling** | LG names 1 Low, 2 Middle, 4 High |
| 5 | scene prediction result | 1 office, 2 animation, 3 cinema, 4 game, 5 sports, 6 common |
| 6 | **AI Sound** | 1 off, 2 on |
| 20 | supported upscaling option list | bitmask |
| — | 0xFF as argument | "report the current value" |
| — | 0x01FF (511) | request the supported-option list |

Live check: when LG Switch opened, the watcher saw 0xA6 answer `0x0202` (AI Scene Optimization on, matching LG's toggle), `0x010F` (option list), `0x0601` (AI Sound off, matching) and `0x0401` (AI Upscaling = 1 while LG Switch showed *Off*: the upscaling encoding is **not** confirmed).

The 27GM950B answers 0xA6 but does not list it in its capabilities string.

## Which picture modes allow AI

`MonitorParameterConstants.PictureModeToAISupportedMap` maps LG picture modes to AI picture-mode classes. Only these modes are in it: Personalized Picture (`eSmart_Picture` 60), Custom (0), Gamer 1 (39), Gamer 2 (40), Max Brightness (61), FPS (43), RTS (44), Vivid (41). On a mode change LG Switch (`SetSoCAIModeONOFFEnabledDisabledStatusBasedOnCurrentPictureMode`) checks the current mode against that map and the monitor's own list; if it isn't AI-capable it turns **both** AI Scene Optimization and AI Picture Mode off and disables their toggles.

Confirmed on hardware: with AI Scene Optimization on, LG Switch could not switch to Calibration1. The monitor refuses, and LG Switch shows its "may not be available in certain setting modes" toast.

## Picture mode values on the wire are not LG's enum

LG Switch's tile list is keyed by `E_PICTUREMODE_NAME` and it passes that enum to its native service, which translates it into the monitor's own value before writing VCP 0x15. The translation lives in native code (`SmartMonitorControlService.exe` / `LGMonitorDDCCISDK.dll`) and was not decoded. Values observed on the wire:

| Tile | LG enum | VCP 0x15 |
|---|---:|---:|
| Personalized Picture | 60 | 65 |
| Gamer 1 | 39 | 45 |
| Gamer 2 | 40 | 46 |
| Calibration1, Calibration2, Color Weakness, DCI-P3, FPS | 18, 19, 30, 25, 43 | not observed yet |

Each picture mode keeps its own brightness, black stabilizer, colour temperature and sharpness (`0x87`, answered though undeclared): switching Personalized → Gamer 1 changed all of them.

## Brightness/contrast are never greyed out on this monitor

`IsBrightnessEnabled` / `IsContrastEnabled` / `IsSharpnessEnabled` are only toggled in `NetworkSettingsViewModel` (webOS network monitors). For DDC/CI monitors like the 27GM950B, LG Switch does not disable these sliders based on AI or picture mode.

## Other observations

- When LG Switch's Screen Settings opens, it also exercises `0xB1` (Dual-Mode: `0x0102`, `0x0011`, back to `0x2200`) and `0xDB` (picture wizard: `0x0A01`, back to `0xA100`), i.e. it uses write-then-read sub-command queries on those channels too.
- `0xF6` flickers 1 → 2 → 0 during a picture-mode change.
