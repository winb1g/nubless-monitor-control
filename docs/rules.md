# Feature interlocks (LG Switch parity)

LG Switch keeps some features mutually exclusive, and its English UI resources (`bin/en/SwitchAppModel.resources.dll`) and view-model strings (`ModeSwitchModule.dll`) show which ones. nubless reproduces the behaviour in `crates/nubless-core/src/rules.rs`, using its own wording. The resource keys are listed so the evidence can be re-checked; the LG text itself is not copied here.

## Evidence

| LG resource key / symbol | What it shows |
|---|---|
| `txtMDPMsgForAIModePopupDesc` | Turning on **AI Picture Mode** disables AI Scene Optimization and My Display Preset, after a yes/no question. |
| `txtAIModeMsgForMDPPopupDesc` | Turning on **My Display Preset** disables AI Scene Optimization and AI Picture Mode, after a yes/no question. |
| `txtSoCFeatureEnableddesc`, `SoCFeatureEnabledToast`, `UpdatePCAIAndMDPValueBasedOnSOC` | Turning on **AI Scene Optimization** disables AI Picture Mode and My Display Preset, with a notice instead of a question. |
| `txtAISOCPBPdesc`, `txtPCAIPBPdesc`, `txtMyDisplayPresetPBPdesc` (+ `*PBPToast`) | While **PBP/PIP** is on, each of the three features is disabled. |
| `UpdateAISoCUpScalingOptionList: PBP mode is ON …` | **AI Upscaling** options are disabled while PBP is on. |
| `PPWHDRPopupDesc`, `txtPBPDesc` | The **Personalized Picture Wizard** needs HDR off and PBP/PIP off. |
| `ToastScreenSettingsDesc` | General: a setting may be unavailable in some modes, and the user is told which component to turn off. nubless shows this when a write is rolled back. |
| `IsBrightnessEnabled`, `IsContrastEnabled`, `IsSharpnessEnabled`, `SetSoCAIModeONOFFEnabledDisabledStatusBasedOnCurrentPictureMode` | Some sliders are greyed out in some states, and whether AI Scene Optimization can be used depends on the picture mode. **The exact conditions are not known yet** (see `verify-on-hardware.md`). |

Note: `SoC` in LG's names is the monitor's own AI chip, so **AI Scene Optimization runs on the monitor** (a monitor setting nubless can eventually control). **AI Picture Mode** is the PC-side feature that uses LG's encrypted model, and My Display Preset is PC-side per-application switching. The earlier research notes called AI Scene Optimization PC software; that was corrected here.

## Rules as implemented

| Action | Result | Asks first? |
|---|---|---|
| AI Scene Optimization → on | AI Picture Mode and My Display Preset off | No: shows a notice if anything was turned off |
| AI Picture Mode → on | AI Scene Optimization and My Display Preset off | Yes, if anything would be turned off |
| My Display Preset → on | AI Scene Optimization and AI Picture Mode off | Yes, if anything would be turned off |
| Any of the three → on while PBP/PIP is on | Refused, with "turn PBP/PIP off first" | — |
| PBP/PIP comes on | All three turn off, with a notice | — |
| Any feature → **off** | Off, immediately | **Never** (nubless rule: no dead ends) |

Open question: LG may show its question even when nothing would be turned off. nubless asks only when something would change; tracked in `verify-on-hardware.md`.

## Tests

`cargo test -p nubless-core rules` checks every combination of the five state bits: at most one smart feature is on after any action, PBP/PIP always wins, turning off never asks, and every unavailable control explains itself.
