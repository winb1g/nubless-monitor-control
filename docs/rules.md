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
| `PictureModeToAISupportedMap`, `SetSoCAIModeONOFFEnabledDisabledStatusBasedOnCurrentPictureMode` | AI Scene Optimization and AI Picture Mode only work in Personalized Picture, Custom, Gamer 1, Gamer 2, Max Brightness, FPS, RTS and Vivid. In any other mode LG Switch turns both off and disables them; with AI on, the monitor refuses a switch to such a mode (confirmed on hardware with Calibration1). |
| `IsBrightnessEnabled`, `IsContrastEnabled`, `IsSharpnessEnabled` | Only used for webOS network monitors. On the 27GM950B, LG Switch never greys out these sliders. |

Note: `SoC` in LG's names is the monitor's own AI chip, so **AI Scene Optimization runs on the monitor** (a monitor setting nubless can eventually control). **AI Picture Mode** is the PC-side feature that uses LG's encrypted model, and My Display Preset is PC-side per-application switching. The earlier research notes called AI Scene Optimization PC software; that was corrected here.

## Rules as implemented

| Action | Result | Asks first? |
|---|---|---|
| AI Scene Optimization → on | AI Picture Mode and My Display Preset off | No: shows a notice if anything was turned off |
| AI Picture Mode → on | AI Scene Optimization and My Display Preset off | Yes, if anything would be turned off |
| My Display Preset → on | AI Scene Optimization and AI Picture Mode off | Yes, if anything would be turned off |
| Any of the three → on while PBP/PIP is on | Refused, with "turn PBP/PIP off first" | — |
| PBP/PIP comes on | All three turn off, with a notice | — |
| AI Scene Optimization or AI Picture Mode → on in a non-AI picture mode | Refused, with "switch to Personalized Picture, Gamer 1, Gamer 2 or FPS first" | — |
| Picture mode → a non-AI mode while an AI feature is on | AI features off, then the mode switches | **Yes** (LG lets the monitor refuse and shows a generic toast; nubless asks and does both steps for you) |
| Picture mode → a non-AI mode with AI off | Switches; AI toggles become unavailable | No |
| Any feature → **off** | Off, immediately | **Never** (nubless rule: no dead ends) |

Open question: LG may show its question even when nothing would be turned off. nubless asks only when something would change; tracked in `verify-on-hardware.md`.

## Observed on the monitor (not from LG Switch)

These interlocks were found by using the real monitor, not by reading LG Switch's resources, so there is no resource key to cite. They are recorded with the date, the firmware and what is still unknown. None is implemented in `rules.rs` yet.

| Observation | Seen | Implemented? |
|---|---|---|
| **Motion Blur Reduction is only available while Adaptive-Sync is Off.** Turning Adaptive-Sync **Off** makes Motion Blur Reduction available; it stays unavailable while Adaptive-Sync is **Basic** or **Extended**. | 2026-10-21, 27GM950B, firmware 3.05, found by the owner while testing | No. Motion Blur Reduction (`0xFA`) is still read-only (`Candidate`), so there is nothing to block yet. |
| **Adaptive-Sync Basic or Extended clears Motion Blur Reduction.** After On, then Basic, then back to Off, Motion Blur Reduction was Off. `0xFA` reads 1 for On and 2 for Off or hidden; `0xF4` reads 7 with Adaptive-Sync Off and 6 otherwise. | 2026-10-30, 27GM950B, firmware 3.05, read at each OSD state with `nubless probe` | No. |

What this implies once Motion Blur Reduction becomes writable (neither is built yet):
- It would be refused with "turn Adaptive-Sync off first" while Adaptive-Sync is Basic or Extended, the same shape as the PBP/PIP rows above.
- Turning Adaptive-Sync **on** (Basic or Extended) switches Motion Blur Reduction off (seen 2026-10-30), so the confirmation should say so.

Still to record on the monitor (also in [`verify-on-hardware.md`](verify-on-hardware.md)):
- Where it appears (monitor menu, LG Switch, or both) and what its options are. It was missing from the first menu inventory ([`research/10-osd-inventory.md`](research/10-osd-inventory.md)), which was taken with Adaptive-Sync on.
- Its raw values on `0xFA`. The monitor declares `00 01` but read **2** while Adaptive-Sync was Extended, so 2 is probably "unavailable" rather than a setting.
- Whether the setting you had survives a trip through Basic or Extended.

## Tests

`cargo test -p nubless-core rules` checks every combination of the six state bits: at most one smart feature is on after any action, PBP/PIP always wins, no AI feature is on in a non-AI picture mode, turning off never asks, and every unavailable control explains itself.
